mod random;
mod tcp;

pub use self::random::Random;
pub use self::tcp::TcpSource;

pub trait Source: Send {
    /// Returns type as a string that is used mainly for concrete factory identification.
    fn ty() -> &'static str where Self: Sized;
}
