use std::sync::Arc;

use serde_json::ser::to_string;

use super::Output;
use super::super::{Record};

/// Output that prints all records to the Standard Output.
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
