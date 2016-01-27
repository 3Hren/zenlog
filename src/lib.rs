#![feature(plugin)]
#![plugin(clippy)]
#![plugin(serde_macros)]
// #![warn(clippy_pedantic)]
#![feature(box_syntax)]
#![feature(custom_derive)]
#![feature(question_mark)]

#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;
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
use std::sync::mpsc;

pub use serde_json::Value;

pub mod logging;

mod config;
// TODO: Enable: mod output;
mod source;

use source::Source;

pub use config::Config;
use config::PipeConfig;

// TODO: Use DI instead.
lazy_static! {
    static ref REGISTRY: Registry = {
        info!("registering components");
        let mut r = Registry::default();

        r.sources.insert("random",
            box |config, tx| {
                let config = serde_json::value::from_value(config).unwrap();
                source::Random::run(config, tx).map(|v| Box::new(v) as Box<Source>)
            }
        );
        debug!(r#"registered "random" component in "source" category"#);

        r
    };
}

pub type Record = Value;

pub enum Control {
    Hup,
    Shutdown,
}

type SourceFactory = Fn(Value, mpsc::Sender<Record>) -> Result<Box<Source>, ()> + Send + Sync;

#[derive(Default)]
struct Registry {
    sources: HashMap<&'static str, Box<SourceFactory>>,
}

impl Registry {
    /// Registers a source with the factory.
    fn add_source<T: Source + Sized>(&mut self, factory: Box<SourceFactory>) {
        self.sources.insert(T::ty(), factory);
    }
}

/// Represents the event proccessing pipeline.
struct Pipe {
    sources: Vec<Box<Source>>,
}

impl Pipe {
    fn run(config: &PipeConfig) -> Result<Pipe, ()> {
        // Pipelines.
        let (tx, rx) = mpsc::channel();

        // Start Sources.
        let mut sources = Vec::new();
        for source in config.sources() {
            let factory = REGISTRY.sources.get(&source.find("type").unwrap().as_string().unwrap()).unwrap();
            let source = factory(source.clone(), tx.clone()).unwrap();
            sources.push(source);
        }

        let pipe = Pipe {
            sources: sources,
        };

        Ok(pipe)
    }
}

pub struct Runtime {
    config: Vec<PipeConfig>,
}

impl Runtime {
    // Config -> Factories.
    // TODO: Move to From trait.
    pub fn from(config: Config) -> Result<Runtime, ()> {
        logging::reset(logging::from_usize(config.severity()))
            .expect("failed to initialize logging system");

        trace!("initialized the runtime: {:#?}", config);

        let runtime = Runtime {
            config: config.pipeline().clone(),
        };

        Ok(runtime)
    }

    pub fn run(self, rx: mpsc::Receiver<Control>) -> JoinHandle<()> {
        // Spawn control thread, which starts all sources and outputs and listen for events.
        let runtime = self;
        thread::spawn(move || {
            let mut pipelines = Vec::new();

            for config in &runtime.config {
                pipelines.push(Pipe::run(config).unwrap());
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
            }

            info!("started {} pipelines", runtime.config.len());

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

            // TODO: Drop all sources.
            // TODO: Drop all outputs.
            // TODO: Wait for all threads are joined.
        })
    }
}
