#[macro_use] extern crate log;
extern crate chan_signal as signal;

extern crate zenlog;

use std::sync::atomic::Ordering;

use signal::Signal;

use zenlog::{Registry, Runtime, RuntimeConfig};
use zenlog::logging::{AsLogLevel, AsUsize};

fn main() {
    let filename = ".zenlog.json";

    // List of signals we want to listen.
    // - INT and TERM - for graceful termination.
    // - HUP - for graceful runtime reloading without restarting.
    // - USR1 - for file output reloading.
    // - USR2 - for runtime logging system reloading.
    //
    // We definetely should listen signals here, not elsewhere in the library, because they are
    // tricky and fucking dangerous. First of all, all threads except this one should block ALL
    // signals, otherwise we can blow up, resulting in sudden catch one of non blocked signal,
    // which probably just kills the application.
    // Also there shouldn't be any other signal handlers installed. This is the law, asshole!
    let sigset = [Signal::INT, Signal::TERM, Signal::HUP, Signal::USR1, Signal::USR2];
    let listener = signal::notify(&sigset);

    let cfg = RuntimeConfig::from(filename)
        .expect("failed to read configuration file");

    let severity = zenlog::logging::init(cfg.severity())
        .expect("failed to initialize the logging system");

    let registry = Registry::new();

    info!("starting Zenlog");
    info!("special signal handlers are set for {:?} signals", sigset);

    let mut runtime = Some(Runtime::new(cfg.pipelines(), &registry)
        .expect("failed to create runtime"));

    for signal in listener {
        info!("caught {:?} signal", signal);
        match signal {
            Signal::HUP => {
                match RuntimeConfig::from(filename) {
                    Ok(cfg) => {
                        runtime = match Runtime::new(cfg.pipelines(), &registry) {
                            Ok(runtime) => Some(runtime),
                            Err(err) => {
                                error!("failed to create runtime: {:?}", err);
                                continue;
                            }
                        };
                    }
                    Err(err) => {
                        error!("failed to read {}: {:?}", filename, err);
                    }
                }
            }
            Signal::USR1 => {
                // Always valid.
                runtime.as_mut().unwrap().hup();
            }
            Signal::USR2 => {
                match RuntimeConfig::from(filename) {
                    Ok(cfg) => {
                        match cfg.severity().as_level() {
                            Some(lvl) => {
                                severity.store(lvl.as_usize(), Ordering::SeqCst);
                                info!("severity level is now {}", cfg.severity());
                            }
                            None => {
                                warn!("failed to reinitialize the logging system");
                            }
                        }
                    }
                    Err(err) => {
                        error!("failed to read {}: {:?}", filename, err);
                    }
                }
            }
            signal => {
                info!("caught {:?} signal, shutting down", signal);
                break;
            }
        }
    }

    runtime.unwrap();
    info!("Zenlog has been successfully stopped");
}
