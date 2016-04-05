mod random;
mod tcp;

pub use self::random::Random;
pub use self::tcp::TcpSource;

use std::sync::mpsc;

use serde::Deserialize;

use super::{Record};

pub trait Source: Send {
    /// Returns type as a string that is used mainly for concrete factory identification.
    fn ty() -> &'static str where Self: Sized;
}

pub trait SourceFrom: Source + Sized {
    /// Represents a source's deserializable config.
    type Config: Deserialize;

    /// Constructs and immediately run the source by configuring it with the given config.
    fn run(config: Self::Config, tx: mpsc::Sender<Record>) -> Result<Self, ()>;
}
