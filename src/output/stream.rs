use serde_json::ser::to_string;

use super::Output;
use super::super::{Record};

pub struct Stream;

impl Output for Stream {
    fn handle(record: &Record) {
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
