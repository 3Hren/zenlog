#[macro_use] extern crate log;

extern crate chan_signal as signals;

extern crate zenlog;

use signals::Signal;

use zenlog::{Config, Runtime};

fn main() {
    let filename = ".zenlog.yml";

    // List of signals we want to listen.
    // - INT and TERM - for graceful termination.
    // - HUP - for graceful runtime reloading without restarting.
    //
    // We definetely should listen signals here, not elsewhere in the library, because they are
    // tricky and fucking dangerous. First of all, all threads except this one should block ALL
    // signals, otherwise we can blow up, resulting in sudden catch one of non blocked signal,
    // which probably just kills the application.
    // Also there shouldn't be any other signal handlers installed. This is the law, asshole!
    let listener = signals::notify(&[Signal::INT, Signal::TERM, Signal::HUP]);

    let config = Config::from(filename)
        .expect("failed to read configuration file");

    zenlog::logging::reset(zenlog::logging::from_usize(config.severity()))
        .expect("failed to initialize logging system");

    info!("starting Zenlog");

    let mut runtime = Some(Runtime::from(config.pipeline().clone()));

    for signal in listener {
        match signal {
            Signal::HUP => {
                info!("caught HUP signal");
                // TODO: Reread config
                runtime = Some(Runtime::from(config.pipeline().clone()));
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
