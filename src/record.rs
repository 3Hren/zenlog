pub trait Value {
    fn is_none(&self) -> bool;
    fn as_bool(&self) -> Option<bool>;
    fn as_i64(&self) -> Option<i64>;
    fn as_u64(&self) -> Option<u64>;
    fn as_f64(&self) -> Option<f64>;
    fn as_str(&self) -> Option<&str>;

    fn iter(&self) -> Option<Box<Iterator<Item=&Value>>>;
    fn iter_map(&self) -> Option<Box<Iterator<Item=(&str, &Value)>>>;
}

///
pub trait Record {
    /// Returns an event message.
    ///
    /// Every logging event must contain some message.
    fn message(&self) -> &str;

    ///
    /// We save your CPU time, freeing from formatting timestamps into strings.
    fn timestamp(&self) -> Option<i64>;

    fn iter(&self) -> Box<Iterator<Item=(&str, &Value)>>;
}
