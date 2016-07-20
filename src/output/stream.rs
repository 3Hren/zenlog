use std::sync::Arc;

use serde_json::to_string;

use {Config, Record};
use output::{Output, OutputFactory};

/// Output that prints all records to the Standard Output.
///
/// # Warning
///
/// Quite slow. Use only for debugging purposes.
pub struct Stream;

impl Output for Stream {
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

impl OutputFactory for Stream {
    type Error = &'static str;

    fn ty() -> &'static str {
        "stream"
    }

    #[allow(unused_variables)]
    fn from(cfg: &Config) -> Result<Box<Output>, Self::Error> {
        Ok(Box::new(Stream))
    }
}
