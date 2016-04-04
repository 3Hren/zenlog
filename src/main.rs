#[macro_use] extern crate log;

extern crate chan_signal as signals;

extern crate zenlog;

use std::sync::Arc;

use signals::Signal;

use zenlog::{Config, MainRegistry, Runtime};

fn main() {
    let filename = ".zenlog.yml";

    // List of signals we want to listen.
    // - INT and TERM - for graceful termination.
    // - HUP - for graceful runtime reloading without restarting.
    // - ALRM - (yeah, alarm, what the fuck?!) for file output reloading.
    //
    // We definetely should listen signals here, not elsewhere in the library, because they are
    // tricky and fucking dangerous. First of all, all threads except this one should block ALL
    // signals, otherwise we can blow up, resulting in sudden catch one of non blocked signal,
    // which probably just kills the application.
    // Also there shouldn't be any other signal handlers installed. This is the law, asshole!
    let sigset  = [Signal::INT, Signal::TERM, Signal::HUP, Signal::ALRM];
    let listener = signals::notify(&sigset);

    let config = Config::from(filename)
        .expect("failed to read configuration file");

    zenlog::logging::reset(zenlog::logging::from_usize(config.severity()))
        .expect("failed to initialize logging system");

    let registry = Arc::new(MainRegistry::new());

    info!("starting Zenlog");
    info!("special signal handlers are set for {:?} signals", sigset);

    let mut runtime = Some(Runtime::from(config.pipeline().clone(), registry.clone()));

    for signal in listener {
        info!("caught {:?} signal", signal);
        match signal {
            Signal::HUP => {
                // TODO: Reread config
                runtime = Some(Runtime::from(config.pipeline().clone(), registry.clone()));
            }
            Signal::ALRM => {
                // Always valid.
                runtime.as_mut().unwrap().hup();
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
