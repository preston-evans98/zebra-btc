#![allow(clippy::unit_arg)]
use crate::{
    compactint::CompactInt,
    serialization::{BitcoinDeserialize, BitcoinSerialize, SerializationError},
};
use std::{
    fmt,
    io::{self, Read},
};

/// An encoding of a Bitcoin script.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
#[cfg_attr(
    any(test, feature = "proptest-impl"),
    derive(proptest_derive::Arbitrary)
)]
pub struct Script(pub Vec<u8>);

impl Script {
    pub fn serialized_size(&self) -> usize {
        CompactInt::size(self.0.len()) + self.0.len()
    }
}

impl fmt::Debug for Script {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Script")
            .field(&hex::encode(&self.0))
            .finish()
    }
}

impl BitcoinSerialize for Script {
    fn bitcoin_serialize<W: io::Write>(&self, mut writer: W) -> Result<(), io::Error> {
        CompactInt::from(self.0.len()).bitcoin_serialize(&mut writer)?;
        writer.write_all(&self.0[..])?;
        Ok(())
    }
}

impl BitcoinDeserialize for Script {
    fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<Self, SerializationError> {
        // XXX what is the max length of a script?
        let len = CompactInt::bitcoin_deserialize(&mut reader)?.value();
        let mut bytes = Vec::new();
        reader.take(len).read_to_end(&mut bytes)?;
        Ok(Script(bytes))
    }
}

#[cfg(test)]
mod proptests {
    use std::io::Cursor;

    use proptest::prelude::*;

    use super::*;
    use crate::serialization::{BitcoinDeserialize, BitcoinSerialize};

    proptest! {
        #[test]
        fn script_roundtrip(script in any::<Script>()) {
            zebra_test::init();

            let mut bytes = Cursor::new(Vec::new());
            script.bitcoin_serialize(&mut bytes)?;

            bytes.set_position(0);
            let other_script = Script::bitcoin_deserialize(&mut bytes)?;

            prop_assert_eq![script, other_script];
        }
    }
}
