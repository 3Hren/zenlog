use std::sync::Arc;

use serde_json::Value;
use serde_json::ser::to_string;

use super::{Output, OutputFrom};
use super::super::{Record};

/// Output that prints all records to the Standard Output.
///
/// Quite slow. Use only for debugging purposes.
pub struct Stream;

impl Output for Stream {
    fn ty() -> &'static str where Self: Sized {
        "stream"
    }

    fn handle(&mut self, record: &Arc<Record>) {
        match to_string(&record) {
            Ok(buf) => {
                println!("{}", buf);
            }
            Err(err) => {
                error!("failed to stringify the record: {}", err);
            }
        }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum Error {}
}

impl OutputFrom for Stream {
    type Error = Error;
    type Config = Value;

    fn from(_config: Value) -> Result<Stream, Error> {
        Ok(Stream)
    }
}
