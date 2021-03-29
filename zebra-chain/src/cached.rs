use crate::{BitcoinDeserialize, BitcoinSerialize, SerializationError};

/// A Cached type is an option that is never serialized.
///
/// It can be added to any struct without risking a consensus break.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cached<T: Clone + Copy>(Option<T>);

impl<T: Clone + Copy> Cached<T> {
    pub fn new() -> Cached<T> {
        Cached(None)
    }
    pub fn from(val: T) -> Cached<T> {
        Cached(Some(val))
    }
    pub fn value(&self) -> Option<T> {
        self.0
    }
    pub fn mut_value(&mut self) -> &mut Option<T> {
        &mut self.0
    }
    pub fn has_value(&self) -> bool {
        self.0.is_some()
    }
    // pub fn set_value(&mut self, new: T) {
    //     self.0 = new
    // }
}

impl<T: Clone + Copy> std::fmt::Debug for Cached<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)?;
        Ok(())
    }
}
impl<T: Clone + Copy> BitcoinSerialize for Cached<T> {
    fn bitcoin_serialize<W>(&self, _: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        Ok(())
    }
}
impl<T: Clone + Copy> BitcoinDeserialize for Cached<T> {
    fn bitcoin_deserialize<R: std::io::Read>(
        _: R,
    ) -> std::result::Result<Self, SerializationError> {
        Ok(Cached(None))
    }
}
