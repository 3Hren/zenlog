//! The core idea is to process records as fast as possible.
//!
//! Some outputs can handle that way, i.e null output or stdout. But some of them are blocking, i.e
//! file or elasticsearch, so using a separate thread is required to avoid entire pipeling locking.
//!
//! We can: handle each output in separate thread and communicate with them via channels, but that
//! would mean unnecessary intermediate queue for some outputs - this is bad.
//!
//! The result: each output manages its blocking mode itself.

mod file;
mod stream;

pub use self::file::FileOutput;
pub use self::stream::Stream;

use std::sync::Arc;
use std::sync::mpsc::Sender;

use super::{Record};

pub trait Output: Send {
    fn handle(&mut self, record: &Arc<Record>);

    /// Creates an optional sender, which should be triggered when it's time to reload the output.
    ///
    /// For example it's useful for integration with logrotate, which sends HUP or USR1 signal when
    /// it's time to reopen rotated files.
    ///
    /// Default implementation always returns None.
    fn hup(&self) -> Option<Sender<()>> {
        None
    }
}
