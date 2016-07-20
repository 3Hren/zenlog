use std::error::Error;
use std::io::{stdin, BufReader, Read};
use std::sync::mpsc::Sender;
use std::thread;

use serde_json::StreamDeserializer;

use {Config, Record};
use source::{Source, SourceFactory};

pub struct StdinSource;

impl Source for StdinSource {}

impl SourceFactory for StdinSource {
    type Error = Box<Error>;

    fn ty() -> &'static str {
        "stdin"
    }

    #[allow(unused_variables)]
    fn run(cfg: &Config, tx: Sender<Record>) -> Result<Box<Source>, Box<Error>> {
        thread::spawn(move || {
            let rd = stdin();
            let rd = rd.lock();
            let rd = BufReader::new(rd);
            for record in StreamDeserializer::new(rd.bytes()) {
                match record {
                    Ok(record) => {
                        tx.send(record).expect("pipeline must outlive all attached inputs");
                    }
                    Err(err) => {
                        warn!("unable to decode payload - {}", err);
                        break;
                    }
                }
            }
        });

        Ok(Box::new(StdinSource))
    }
}
