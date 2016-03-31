//! The core idea is to process records as fast as possible.
//!
//! Some outputs can handle that way, i.e null output or stdout. But some of them are blocking, i.e
//! file or elasticsearch, so using a separate thread is required to avoid entire pipeling locking.
//!
//! We can: handle each output in separate thread and communicate with them via channels, but that
//! would mean unnecessary intermediate queue for some outputs - this is bad.
//!
//! The result: each output manages its blocking mode itself.

mod stream;

pub use self::stream::Stream;

use super::{Record};

pub trait Output: Send {
    fn handle(&mut self, record: &Record);
}
