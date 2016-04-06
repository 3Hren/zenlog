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
use std::error::Error;
use std::thread::{self, JoinHandle};
use std::sync::{mpsc, Arc};

pub use serde_json::Value;

pub mod logging;

mod config;
mod output;
mod source;

use output::Output;
use source::{Source, SourceFrom};

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

        let mut registry = MainRegistry::default();
        registry.add_source::<source::Random>();
        registry.add_source::<source::TcpSource>();

        registry.outputs.insert("stream", box |_| Ok(box output::Stream));
        debug!("registered Stream component in 'output' category");

        registry.outputs.insert("file", box |config| {
            Ok(box output::FileOutput::new(config.find("path").unwrap().as_string().unwrap()))
        });
        debug!("registered File component in 'output' category");

        registry
    }

    /// Registers a source with the factory.
    fn add_source<T: SourceFrom + 'static>(&mut self) {
        self.sources.insert(T::ty(),
            box |config, tx| {
                let config = serde_json::value::from_value(config)?;
                T::run(config, tx)
                    .map(|v| box v as Box<Source>)
                    .map_err(|e| box e as Box<Error>)
            }
        );

        debug!("registered {} component in 'source' category", T::ty());
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

pub type SourceFactory = Fn(Value, mpsc::Sender<Record>) -> Result<Box<Source>, Box<Error>> + Send + Sync;
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
            let output = factory(config.clone())?;
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

                // TODO: Maybe add this as a filter?
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
    pub fn from(config: Vec<PipeConfig>, registry: &Registry) -> Result<Runtime, ()> {
        trace!("initializing the runtime: {:#?}", config);

        let (tx, rx) = mpsc::channel();

        let thread = Runtime::init(&config, registry, rx)?;

        let runtime = Runtime {
            tx: tx,
            thread: Some(thread),
        };

        Ok(runtime)
    }

    fn init(config: &[PipeConfig], registry: &Registry, rx: mpsc::Receiver<Control>) ->
        Result<JoinHandle<()>, ()>
    {
        let mut pipelines = Vec::new();

        for c in config {
            pipelines.push(Pipe::run(c, registry)?);
        }

        info!("started {} pipeline(s)", config.len());

        let thread = thread::spawn(move || Runtime::run(pipelines, rx));

        Ok(thread)
    }

    /// Blocks the current thread for running Zenlog Runtime.
    fn run(pipelines: Vec<Pipe>, rx: mpsc::Receiver<Control>) {
        let mut pipelines = pipelines;

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

    pub fn hup(&mut self) {
        if let Err(err) = self.tx.send(Control::Hup) {
            error!("failed to send hup signal to the runtime: {}", err);
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
