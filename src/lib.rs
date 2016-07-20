#[macro_use]
extern crate log;
extern crate libc;
extern crate chrono;
extern crate mio;
extern crate serde_json;
extern crate termion;

use std::collections::HashMap;
use std::error::Error;
use std::thread::{self, JoinHandle};
use std::sync::{mpsc, Arc};
use std::sync::mpsc::Sender;

use serde_json::Value;

mod config;
mod output;
mod source;

pub mod logging;

use output::{Output, OutputFactory};
use source::{Source, SourceFactory};

use config::PipeConfig;
pub use config::RuntimeConfig;

pub type Record = Value;
pub type Config = Value;

enum Control {
    Hup,
    Shutdown,
}

type FnSourceFactory = Fn(&Config, Sender<Record>) -> Result<Box<Source>, Box<Error>>;
type FnOutputFactory = Fn(&Config) -> Result<Box<Output>, Box<Error>>;

#[derive(Default)]
pub struct Registry {
    sources: HashMap<&'static str, Box<FnSourceFactory>>,
    outputs: HashMap<&'static str, Box<FnOutputFactory>>,
}

impl Registry {
    pub fn new() -> Registry {
        info!("registering components");

        let mut registry = Registry::default();
        registry.add_source::<source::StdinSource>();
        registry.add_source::<source::UdpSource>();

        registry.add_output::<output::Stream>();

        registry
    }

    /// Registers a source with the factory.
    fn add_source<T: SourceFactory + 'static>(&mut self) {
        self.sources.insert(T::ty(),
            Box::new(|cfg, tx| {
                T::run(cfg, tx)
                    .map_err(Into::into)
            })
        );

        debug!("registered {} component in 'source' category", T::ty());
    }

    fn add_output<T: OutputFactory + 'static>(&mut self) {
        self.outputs.insert(T::ty(),
            Box::new(|cfg| {
                T::from(cfg)
                    .map_err(Into::into)
            })
        );

        debug!("registered {} component in 'output' category", T::ty());
    }

    fn source(&self, cfg: &Config, tx: Sender<Record>) -> Result<Box<Source>, Box<Error>> {
        Registry::ty(cfg)
            .map_err(Into::into)
            .and_then(|ty| self.sources.get(ty)
                .ok_or("source not found".into()))
            .and_then(|factory| factory(cfg, tx))
    }

    fn output(&self, cfg: &Config) -> Result<Box<Output>, Box<Error>> {
        Registry::ty(cfg)
            .map_err(Into::into)
            .and_then(|ty| self.outputs.get(ty)
                .ok_or("output not found".into()))
            .and_then(|factory| factory(cfg))
    }

    fn ty(cfg: &Config) -> Result<&str, &str> {
        cfg.find("type")
            .ok_or("field 'type' is required")
            .and_then(|ty| ty.as_string()
                .ok_or("field 'type' must be a string"))
    }
}

/// Event proccessing pipeline.
///
/// # Note:
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
    hups: Vec<Sender<()>>,
}

impl Pipe {
    fn run(cfg: &PipeConfig, registry: &Registry) -> Result<Pipe, Box<Error>> {
        // Pipelines.
        let (tx, rx) = mpsc::channel();

        // Start Sources.
        let mut sources = Vec::new();

        for cfg in cfg.sources() {
            trace!("starting source with config {:#?}", cfg);

            let source = try!(registry.source(cfg, tx.clone()));
            sources.push(source);
        }

        let mut outputs = Vec::new();

        for cfg in cfg.outputs() {
            trace!("constructing output with config {:#?}", cfg);

            let output = try!(registry.output(cfg));
            outputs.push(output);
        }

        // Collect all hup channels.
        let hups = outputs.iter()
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
    tx: Sender<Control>,
    thread: Option<JoinHandle<()>>,
}

impl Runtime {
    /// Constructs Zenlog Runtime by constructing and starting all pipelines listed in the given
    /// config.
    pub fn new(config: &[PipeConfig], registry: &Registry) -> Result<Runtime, Box<Error>> {
        trace!("initializing the runtime: {:#?}", config);

        let (tx, rx) = mpsc::channel();

        let thread = try!(Runtime::init(&config, registry, rx));

        let runtime = Runtime {
            tx: tx,
            thread: Some(thread),
        };

        Ok(runtime)
    }

    fn init(config: &[PipeConfig], registry: &Registry, rx: mpsc::Receiver<Control>) ->
        Result<JoinHandle<()>, Box<Error>>
    {
        let mut pipelines = Vec::new();

        for c in config {
            pipelines.push(try!(Pipe::run(c, registry)));
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

        if let Err(err) = self.thread.take().expect("thread must exist").join() {
            error!("failed to gracefully shut down the runtime: {:?}", err);
        }
    }
}
