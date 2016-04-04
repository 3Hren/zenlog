#![feature(plugin)]
#![plugin(clippy)]
#![plugin(serde_macros)]
// #![warn(clippy_pedantic)]
#![feature(box_syntax)]
#![feature(custom_derive)]
#![feature(question_mark)]

#[macro_use] extern crate log;
#[macro_use] extern crate quick_error;
extern crate rand;

extern crate chrono;
extern crate mio;
extern crate serde;
extern crate serde_json;
extern crate serde_yaml;
extern crate term;

extern crate yaml_rust as yaml;

use std::collections::HashMap;
use std::thread::{self, JoinHandle};
use std::sync::{mpsc, Arc};

pub use serde_json::Value;

pub mod logging;

mod config;
mod output;
mod source;

use output::Output;
use source::Source;

pub use config::Config;
use config::PipeConfig;

pub type Record = Value;

enum Control {
    Hup,
    Shutdown,
}

pub trait Registry: Send +  Sync {
    fn source(&self, ty: &str) -> Option<&SourceFactory>;
    fn output(&self, ty: &str) -> Option<&OutputFactory>;
}

#[derive(Default)]
pub struct MainRegistry {
    sources: HashMap<&'static str, Box<SourceFactory>>,
    outputs: HashMap<&'static str, Box<OutputFactory>>,
}

impl MainRegistry {
    pub fn new() -> MainRegistry {
        info!("registering components");
        let mut sources: HashMap<&'static str, Box<SourceFactory>> = HashMap::new();

        sources.insert("random",
            box |config, tx| {
                let config = serde_json::value::from_value(config).unwrap();
                source::Random::run(config, tx).map(|v| Box::new(v) as Box<Source>)
            }
        );
        debug!("registered Random component in 'source' category");

        sources.insert("tcp",
            box |config, tx| {
                let config = serde_json::value::from_value(config).unwrap();
                source::TcpSource::run(config, tx).map(|v| Box::new(v) as Box<Source>)
            }
        );
        debug!("registered TCP component in 'source' category");

        let mut outputs: HashMap<&'static str, Box<OutputFactory>> = HashMap::new();

        outputs.insert("stream", box |_| Ok(box output::Stream));
        debug!("registered Stream component in 'output' category");

        outputs.insert("file", box |_| Ok(box output::FileOutput::new("/tmp/zenlog.log")));
        debug!("registered File component in 'output' category");

        MainRegistry {
            sources: sources,
            outputs: outputs,
        }
    }

    /// Registers a source with the factory.
    fn add_source<T: Source + Sized>(&mut self, factory: Box<SourceFactory>) {
        self.sources.insert(T::ty(), factory);
    }
}

impl Registry for MainRegistry {
    fn source(&self, ty: &str) -> Option<&SourceFactory> {
        self.sources.get(ty).map(|val| &**val)
    }

    fn output(&self, ty: &str) -> Option<&OutputFactory> {
        self.outputs.get(ty).map(|val| &**val)
    }
}

pub type SourceFactory = Fn(Value, mpsc::Sender<Record>) -> Result<Box<Source>, ()> + Send + Sync;
pub type OutputFactory = Fn(Value) -> Result<Box<Output>, ()> + Send + Sync;

/// Represents the event proccessing pipeline.
///
/// The control flow on destruction is:
///  1. Drop pipe.
///  2. Drop all sources.
///  3. Tx is dropped -> Rx is exhaused -> Control thread is stopping.
///  4. Drop filters.
///  5. Drop outputs.
struct Pipe {
    thread: Option<JoinHandle<()>>,
    sources: Vec<Box<Source>>,
    hups: Vec<mpsc::Sender<()>>,
}

impl Pipe {
    fn run(config: &PipeConfig, registry: &Registry) -> Result<Pipe, ()> {
        // Pipelines.
        let (tx, rx) = mpsc::channel();

        // Start Sources.
        let mut sources = Vec::new();

        for config in config.sources() {
            let ty = match config.find("type") {
                Some(&Value::String(ref ty)) => ty,
                Some(..) | None => {
                    error!("config {:?} is malformed: required field 'type' is missing or have non-string value", config);
                    // TODO: return Err(MissingType(config));
                    return Err(());
                }
            };

            // TODO: let factory = registry.source(&ty).map_err(UnregisteredFactory(ty.clone()))?;
            let factory = match registry.source(&ty) {
                Some(factory) => factory,
                None => {
                    error!("failed to create source {}: factory is not registered", ty);
                    // TODO: return Err(UnregisteredFactory(ty));
                    return Err(());
                }
            };

            trace!("starting '{}' source with config {:#?}", ty, config);
            let source = factory(config.clone(), tx.clone()).unwrap();
            sources.push(source);
        }

        let mut outputs = Vec::new();
        for config in config.outputs() {
            let ty = match config.find("type") {
                Some(&Value::String(ref ty)) => ty,
                Some(..) | None => {
                    error!("config {:?} is malformed: required field 'type' is missing or have non-string value", config);
                    // TODO: return Err(MissingType(config));
                    return Err(());
                }
            };

            // TODO: let factory = registry.output(&ty).map_err(UnregisteredFactory(ty.clone()))?;
            let factory = match registry.output(&ty) {
                Some(factory) => factory,
                None => {
                    error!("failed to create output {}: factory is not registered", ty);
                    // TODO: return Err(UnregisteredFactory(ty));
                    return Err(());
                }
            };

            trace!("created '{}' output with config {:#?}", ty, config);
            let output = factory(config.clone()).unwrap();
            outputs.push(output);
        }

        // Collect all hup channels.
        let hups: Vec<mpsc::Sender<()>> = outputs.iter()
            .filter_map(|output| output.hup())
            .collect();

        let thread = thread::spawn(move || {
            debug!("started pipeline processing thread");

            for record in rx {
                debug!("processing {:?} ...", record);

                if record.find("message").is_none() {
                    error!("drop '{:?}': message field required", record);
                    continue;
                }

                if record.find("timestamp").is_none() {
                    // TODO: Add (which format?).
                }

                // TODO: Filter.

                let record = Arc::new(record);

                for output in &mut outputs {
                    output.handle(&record);
                }
            }

            debug!("successfully stopped pipeline procesing thread");
        });

        let pipe = Pipe {
            thread: Some(thread),
            sources: sources,
            hups: hups,
        };

        Ok(pipe)
    }

    fn hup(&mut self) {
        for hup in &self.hups {
            if let Err(err) = hup.send(()) {
                error!("failed to send hup event to one of the receivers: {:?}", err);
            }
        }
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        self.sources.clear();

        if let Err(err) = self.thread.take().unwrap().join() {
            error!("failed to gracefully shut down the runtime: {:?}", err);
        }
    }
}

pub struct Runtime {
    tx: mpsc::Sender<Control>,
    thread: Option<JoinHandle<()>>,
}

impl Runtime {
    /// Constructs Zenlog Runtime by constructing and starting all pipelines listed in the given
    /// config.
    // TODO: Move to From trait maybe.
    // TODO: Consider scoped thread API to avoid ARCs.
    pub fn from(config: Vec<PipeConfig>, registry: Arc<Registry>) -> Runtime {
        trace!("initializing the runtime: {:#?}", config);

        let (tx, rx) = mpsc::channel();

        let thread = thread::spawn(move || Runtime::run(&config, registry, rx));

        Runtime {
            tx: tx,
            thread: Some(thread),
        }
    }

    pub fn hup(&mut self) {
        if let Err(err) = self.tx.send(Control::Hup) {
            error!("failed to send hup signal to the runtime: {}", err);
        }
    }

    /// Blocks the current thread for running Zenlog Runtime.
    fn run(config: &[PipeConfig], registry: Arc<Registry>, rx: mpsc::Receiver<Control>) {
        let mut pipelines = Vec::new();

        for c in config {
            pipelines.push(Pipe::run(c, &*registry).unwrap());
        }

        info!("started {} pipeline(s)", config.len());

        // Main control loop.
        for event in rx {
            match event {
                Control::Hup => {
                    debug!("reloading each pipeline");

                    for pipeline in &mut pipelines {
                        pipeline.hup();
                    }
                }
                Control::Shutdown => {
                    debug!("received shutdown event");
                    break;
                }
            }
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        if let Err(err) = self.tx.send(Control::Shutdown) {
            error!("failed to send shutdown signal to the runtime: {}", err);
        }

        if let Err(err) = self.thread.take().unwrap().join() {
            error!("failed to gracefully shut down the runtime: {:?}", err);
        }
    }
}
