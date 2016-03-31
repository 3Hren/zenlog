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
}

#[derive(Default)]
pub struct MainRegistry {
    sources: HashMap<&'static str, Box<SourceFactory>>,
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

        MainRegistry {
            sources: sources,
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
}

pub type SourceFactory = Fn(Value, mpsc::Sender<Record>) -> Result<Box<Source>, ()> + Send + Sync;
pub type OutputFactory = Fn(Value) -> Result<Box<Output>, ()> + Send + Sync;

/// Represents the event proccessing pipeline.
struct Pipe {
    thread: Option<JoinHandle<()>>,
    sources: Vec<Box<Source>>,
}

impl Pipe {
    fn run(config: &PipeConfig, registry: &Registry) -> Result<Pipe, ()> {
        // Pipelines.
        let (tx, rx) = mpsc::channel();

        // Start Sources.
        let mut sources = Vec::new();

        for source in config.sources() {
            let ty = source.find("type").unwrap().as_string().unwrap();
            let factory = registry.source(&ty).unwrap();

            trace!("starting '{}' source with config {:#?}", ty, source);
            let source = factory(source.clone(), tx.clone()).unwrap();
            sources.push(source);
        }

        let thread = thread::spawn(move || {
            debug!("started pipeline processing thread");

            for record in rx {
                debug!("processing {:?} ...", record);
            }

            debug!("successfully stopped pipeline procesing thread");
        });

        //     // Fill.
        //     let filters = Vec::new();
        //     let outputs = Vec::new();
        //
        //     let thread = thread::spawn(move || {
        //         for record in rx {
        //             debug!("processing {:?} ...", record);
        //
        //             if record.find("message").is_none() {
        //                 warn!(target: "pipe", "dropping '{:?}': message field required", record);
        //                 continue;
        //             }
        //
        //             filters.each(|| ...);
        //             // Must consume ASAP.
        //             outputs.each().process(...);
        //             unimplemented!();
        //         }
        //     });
        //
        //     threads.push(thread);
        // TODO: Drop all sources.
        // TODO: Drop all outputs.
        // TODO: Wait for all threads are joined.

        let pipe = Pipe {
            thread: Some(thread),
            sources: sources,
        };

        Ok(pipe)
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

        info!("started {} pipelines", config.len());

        // Main control loop.
        for event in rx {
            match event {
                Control::Hup => {
                    // TODO: For each pipeline - reload().
                    unimplemented!();
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
