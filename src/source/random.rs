use std::collections::BTreeMap;
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;

use rand;
use rand::distributions::{Range, Sample};

use mio;
use mio::{EventLoop, Handler};

use serde_json::Value;

use super::{Source, SourceFrom};
use super::super::{Record};

fn rand_string(len: u16) -> String {
    (0..len).map(|_| (0x20u8 + (rand::random::<f32>() * 96.0) as u8) as char).collect()
}

struct SleepHandler {
    rate: f64,
    range: Range<u16>,
    random: rand::ThreadRng,
    tx: mpsc::Sender<Record>,
}

impl SleepHandler {
    pub fn new(rate: f64, range: Range<u16>, tx: mpsc::Sender<Record>) -> SleepHandler {
        SleepHandler {
            rate: rate,
            range: range,
            random: rand::thread_rng(),
            tx: tx,
        }
    }
}

impl Handler for SleepHandler {
    type Timeout = u64;
    type Message = ();

    fn timeout(&mut self, ev: &mut EventLoop<SleepHandler>, id: u64) {
        let message = rand_string(self.range.sample(&mut self.random));

        let mut map = BTreeMap::new();
        map.insert("message".to_owned(), Value::String(message));

        match self.tx.send(Value::Object(map)) {
            Ok(()) => {}
            Err(err) => {
                error!("failed to send a record: {}", err);
                return ev.shutdown();
            }
        }

        // TODO: This is a single-shot timer, replace with periodic one.
        if ev.timeout_ms(id, (1000.0 / self.rate) as u64).is_err() {
            error!("failed to schedule a timeout");
            ev.shutdown();
        }
    }

    fn notify(&mut self, ev: &mut EventLoop<SleepHandler>, _: ()) {
        ev.shutdown();
    }
}

/// Configuration.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Config {
    /// Rate in number of events per second.
    rate: f64,
    /// Minimum and maximum length range of a generated string.
    range: (u16, u16),
}

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        InvalidRange(range: (u16, u16)) {}
    }
}

pub struct Random {
    /// Termination channel.
    terminator: mio::Sender<()>,
    /// Rate controller thread.
    thread: Option<JoinHandle<()>>,
}

impl Drop for Random {
    fn drop(&mut self) {
        if let Err(err) = self.terminator.send(()) {
            error!("failed to send termination event: {}", err);
        }

        // This should never fail, because we own thread variable and noone joins it elsewhere.
        self.thread.take().unwrap().join().unwrap();
    }
}

impl Source for Random {
    fn ty() -> &'static str {
        "random"
    }
}

impl SourceFrom for Random {
    type Config = Config;

    fn run(config: Config, tx: mpsc::Sender<Record>) -> Result<Random, Box<::std::error::Error>> {
        let (min, max) = config.range;

        if min > max {
            return Err(box Error::InvalidRange((min, max)));
        }

        let rate = config.rate;
        let range = Range::new(min, max + 1);

        let mut ev = EventLoop::new().unwrap();
        let terminator = ev.channel();

        let thread = thread::spawn(move || {
            ev.timeout_ms(0, 0).unwrap();
            ev.run(&mut SleepHandler::new(rate, range, tx)).unwrap();
        });

        let source = Random {
            terminator: terminator,
            thread: Some(thread),
        };

        Ok(source)
    }
}

#[cfg(test)]
mod test {

use std::sync::mpsc;

use super::{Config, Random};

#[test]
fn ty() {
    assert_eq!("random", Random::ty());
}

#[test]
fn generates_random_string() {
    let config = Config {
        rate: 1.0,
        range: (8, 8),
    };
    let (tx, rx) = mpsc::channel();
    let source = Random::run(config, tx);

    let record = rx.recv().unwrap();

    assert!(record.is_object());
    assert!(record.find("message").is_some());
    assert!(record.find("message").unwrap().is_string());
    assert_eq!(8, record.find("message").unwrap().as_string().unwrap().len());

    drop(source);
}

}  // mod test
