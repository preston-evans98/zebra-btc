//! Transactions and transaction-related structures.

use crate::BitcoinSerialize;
use crate::{cached::Cached, compactint::CompactInt};
use bitcoin_serde_derive::BtcSerialize;
use serde::{Deserialize, Serialize};

mod hash;
mod lock_time;
mod memo;
mod serialize;

#[cfg(any(test, feature = "proptest-impl"))]
mod arbitrary;
#[cfg(test)]
mod tests;

pub use hash::Hash;
pub use lock_time::LockTime;
pub use memo::Memo;

use crate::transparent;

/// A Bitcoin transaction. Note that this implementation doesn't yet support SegWit.
///
/// A transaction is an encoded data structure that facilitates the transfer of
/// value between two public key addresses. Everything is
/// designed to ensure that transactions can created, propagated on the network,
/// validated, and finally added to the global ledger of transactions (the
/// blockchain).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BtcSerialize)]
pub struct Transaction {
    /// The transaction version. Versions greater than 1 mean that BIP 68 applies
    pub version: i32,
    /// The transaction Inputs
    pub inputs: Vec<transparent::Input>,
    /// The transaction Outputs
    pub outputs: Vec<transparent::Output>,
    /// The transaction LockTime
    pub locktime: LockTime,
    hash: Cached<Hash>,
}

impl Transaction {
    pub fn new(
        version: i32,
        inputs: Vec<transparent::Input>,
        outputs: Vec<transparent::Output>,
        locktime: LockTime,
    ) -> Transaction {
        Transaction {
            version,
            inputs,
            outputs,
            locktime,
            hash: Cached::new(),
        }
    }
    /// Get the hash of this transaction.
    pub fn hash(&self) -> Hash {
        // If we have a cached version, just return that
        if let Some(hash) = self.hash.value() {
            return hash;
        }
        // Otherwise, serialize the tx to calculate and return the hash
        Hash::from(self)
    }

    pub fn contains_coinbase_input(&self) -> bool {
        self.inputs
            .iter()
            .any(|input| matches!(input, transparent::Input::Coinbase { .. }))
    }

    /// Returns `true` if this transaction is a coinbase transaction.
    pub fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1
            && matches!(
                self.inputs.get(0),
                Some(transparent::Input::Coinbase { .. })
            )
    }
    /// Returns the serialized length (in bytes) of a transaction.
    ///
    /// Note that this implementation is not BIPs 141/144 compliant since we haven't yet implemented SegWit
    pub fn len(&self) -> usize {
        let mut size = 0;
        size += 4 + CompactInt::size(self.inputs.len());
        for input in self.inputs.iter() {
            size += input.len();
        }
        size += CompactInt::size(self.outputs.len());
        for output in self.outputs.iter() {
            size += output.len();
        }
        size + 4
    }
}
