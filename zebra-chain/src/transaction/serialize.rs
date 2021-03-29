//! Contains impls of `ZcashSerialize`, `ZcashDeserialize` for all of the
//! transaction types, so that all of the serialization logic is in one place.

use std::{io, sync::Arc};

use crate::serialization::{BitcoinDeserialize, BitcoinSerialize, SerializationError};

/// The maximum size of a transaction (excluding the witness) in bytes.
pub const MAX_TX_SIZE: u64 = 1_000_000;

use super::*;
use crate::transparent;

/// Deserializes a transaction, calculating and caching its TxID.
impl BitcoinDeserialize for Transaction {
    fn bitcoin_deserialize<R: std::io::Read>(src: R) -> Result<Self, SerializationError> {
        // Put a sanity limit of 1 MB (a whole block) on the size of transaction to protect against DOS attacks
        let mut src = src.take(MAX_TX_SIZE);
        // Deserialize each field
        let version = i32::bitcoin_deserialize(&mut src)?;
        let inputs = <Vec<transparent::Input>>::bitcoin_deserialize(&mut src)?;
        let outputs = <Vec<transparent::Output>>::bitcoin_deserialize(&mut src)?;
        let locktime = LockTime::bitcoin_deserialize(&mut src)?;
        let hash = Cached::new();
        let mut tx = Transaction {
            version,
            inputs,
            outputs,
            locktime,
            hash,
        };
        // Calculate and cache the TxID.
        let own_hash = tx.hash();
        tx.hash = Cached::from(own_hash);
        Ok(tx)
    }
}

impl<T> BitcoinDeserialize for Arc<T>
where
    T: BitcoinDeserialize,
{
    fn bitcoin_deserialize<R: io::Read>(reader: R) -> Result<Self, SerializationError> {
        Ok(Arc::new(T::bitcoin_deserialize(reader)?))
    }
}

impl<T> BitcoinSerialize for Arc<T>
where
    T: BitcoinSerialize,
{
    fn bitcoin_serialize<W: io::Write>(&self, writer: W) -> Result<(), io::Error> {
        T::bitcoin_serialize(self, writer)
    }
}
