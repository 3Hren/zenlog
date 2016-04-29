use std::sync::Arc;

use serde_json::Value;

use super::{Output, OutputFrom};
use super::super::{Record};

pub struct Null;

impl Output for Null {
    fn ty() -> &'static str where Self: Sized {
        "nil"
    }

    fn handle(&mut self, _record: &Arc<Record>) {}
}

quick_error! {
    #[derive(Debug)]
    pub enum Error {}
}

impl OutputFrom for Null {
    type Error = Error;
    type Config = Value;

    fn from(_config: Value) -> Result<Null, Error> {
        Ok(Null)
    }
}
