use std::error::Error;
use std::sync::Arc;
use std::sync::mpsc::Sender;

use super::{Config, Record};

pub use self::stdin::StdinSource;
pub use self::udp::UdpSource;

mod stdin;
mod udp;

pub trait Source: Send {}

pub trait SourceFactory {
    /// The reason of run failure.
    type Error: Into<Box<Error>>;

    /// Returns type as a string that is used mainly for concrete component identification.
    fn ty() -> &'static str
        where Self: Sized;

    /// Constructs and immediately run a new source by configuring it with the given config.
    fn run(cfg: &Config, tx: Sender<Arc<Record>>) -> Result<Box<Source>, Self::Error>
        where Self: Sized;
}
