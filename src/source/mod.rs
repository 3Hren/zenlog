mod random;

pub use self::random::Random;

pub trait Source: Send {
    /// Returns type as a string that is used mainly for concrete factory identification.
    fn ty() -> &'static str where Self: Sized;
}
