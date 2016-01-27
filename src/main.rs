#[macro_use] extern crate log;

extern crate chan_signal as signals;

extern crate zenlog;

use std::sync::mpsc;

use signals::Signal;

use zenlog::{Config, Control, Runtime};

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

    let mut runtime = Runtime::from(config.clone())
        .expect("failed to initialize Zenlog");

    info!("starting Zenlog");

    // Control channel.
    let (mut tx, rx) = mpsc::channel();

    let mut thread = runtime.run(rx);

    for signal in listener {
        match signal {
            Signal::HUP => {
                info!("caught HUP signal");
                // TODO: Reread config

                // Create new runtime, then swap.
                runtime = Runtime::from(config.clone()).unwrap();
                tx.send(Control::Shutdown).unwrap();
                thread.join().unwrap();

                let (xx, rx) = mpsc::channel();
                tx = xx;
                thread = runtime.run(rx);
            }
            signal => {
                info!("caught {:?} signal, shutting down", signal);
                break;
            }
        }
    }

    tx.send(Control::Shutdown)
        .expect("failed to emit shutdown signal");

    thread.join()
        .expect("failed to gracefully shut down the runtime");

    info!("Zenlog has been successfully stopped");
}
