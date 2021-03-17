use std::{fmt, io};

use crate::serialization::{sha256d, BitcoinDeserialize, BitcoinSerialize, SerializationError};
use bitcoin_serde_derive::BtcSerialize;
#[cfg(any(test, feature = "proptest-impl"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

use super::Header;

/// A hash of a block, used to identify blocks and link blocks into a chain. ⛓️
///
/// Technically, this is the (SHA256d) hash of a block *header*, but since the
/// block header includes the Merkle root of the transaction Merkle tree, it
/// binds the entire contents of the block and is used to identify entire blocks.
///
/// Note: Zebra displays transaction and block hashes in big-endian byte-order,
/// following the u256 convention set by Bitcoin and zcashd.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, BtcSerialize)]
#[cfg_attr(any(test, feature = "proptest-impl"), derive(Arbitrary))]
pub struct Hash(pub [u8; 32]);

impl Hash {
    pub fn from_bytes_exact(bytes: [u8; 32]) -> Hash {
        Hash(bytes)
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut reversed_bytes = self.0;
        reversed_bytes.reverse();
        f.write_str(&hex::encode(&reversed_bytes))
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut reversed_bytes = self.0;
        reversed_bytes.reverse();
        f.debug_tuple("block::Hash")
            .field(&hex::encode(&reversed_bytes))
            .finish()
    }
}

impl BitcoinDeserialize for Hash {
    fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<Self, SerializationError>
    where
        Self: Sized,
    {
        Ok(Hash(<[u8; 32]>::bitcoin_deserialize(&mut reader)?))
    }
}

impl<'a> From<&'a Header> for Hash {
    fn from(block_header: &'a Header) -> Self {
        let mut hash_writer = sha256d::Writer::default();
        block_header
            .bitcoin_serialize(&mut hash_writer)
            .expect("Sha256dWriter is infallible");
        Self(hash_writer.finish())
    }
}
impl std::str::FromStr for Hash {
    type Err = SerializationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut bytes = [0; 32];
        if hex::decode_to_slice(s, &mut bytes[..]).is_err() {
            Err(SerializationError::Parse("hex decoding error"))
        } else {
            bytes.reverse();
            Ok(Hash(bytes))
        }
    }
}
