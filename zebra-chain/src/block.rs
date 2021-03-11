//! Blocks and block-related structures (heights, headers, etc.)
#![allow(clippy::unit_arg)]

mod hash;
mod header;
mod height;
mod serialize;

pub mod merkle;

#[cfg(any(test, feature = "proptest-impl"))]
mod arbitrary;
#[cfg(test)]
mod tests;

use std::{fmt, io::Read, iter::FromIterator, sync::Arc};

use crate::compactint::CompactInt;
use crate::{BitcoinDeserialize, BitcoinSerialize, SerializationError};
use bitcoin_serde_derive::{BtcDeserialize, BtcSerialize};
use bytes::{Buf, BytesMut};
pub use hash::Hash;
pub use header::BlockTimeError;
pub use header::{CountedHeader, Header};
pub use height::Height;

use serde::{Deserialize, Serialize};

use crate::{fmt::DisplayToDebug, transaction::Transaction, transparent};

use self::serialize::MAX_BLOCK_BYTES;

/// A Bitcoin block, containing a header and a list of transactions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, BtcSerialize, BtcDeserialize)]
pub struct Block {
    /// The block header, containing block metadata.
    pub header: Header,
    /// The block transactions.
    pub transactions: Vec<std::sync::Arc<Transaction>>,
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fmter = f.debug_struct("Block");
        if let Some(height) = self.coinbase_height() {
            fmter.field("height", &height);
        }

        fmter.field("hash", &DisplayToDebug(self.hash())).finish()
    }
}

impl Block {
    /// Return the block height reported in the coinbase transaction, if any.
    /// Invariant: This method assumes that the height is pre-cached by some early construction/deserialization function.
    /// TODO: this invariant is not upheld by the implementation of bitcoin_deserialize for transparent::Input
    /// Instead, it needs to be delegated to a higher-level function which is aware of the status of BIP34 activation.
    pub fn coinbase_height(&self) -> Option<Height> {
        self.transactions
            .get(0)
            .and_then(|tx| tx.inputs.get(0))
            .and_then(|input| match input {
                transparent::Input::Coinbase { ref height, .. } => match height {
                    Some(cached_height) => cached_height.value(),
                    None => None,
                },
                _ => None,
            })
    }

    /// Compute the hash of this block.
    pub fn hash(&self) -> Hash {
        Hash::from(self)
    }

    /// TODO: re-implement structural validation
    ///  
    /// Deserializes a block. When validatoin is enabled, attempts to make structurally invalid blocks unrepresentable by enforcing that...
    /// 1. The block contains exactly one Coinbase transaction, and it's in the first position.
    /// 1. The block does not contain duplicate transactions
    /// 1. The transactions merkle-ize to the root in the block header
    pub fn deserialize_from_buf(mut src: &mut BytesMut) -> Result<Self, SerializationError> {
        let header = Header::deserialize_from_buf(src.split_to(80))?;

        let mut src = src.reader().take(MAX_BLOCK_BYTES);
        let tx_count = {
            let tx_count = CompactInt::bitcoin_deserialize(&mut src)?;
            tx_count.value()
        };

        // Reject empty blocks
        if tx_count == 0 {
            return Err(SerializationError::Parse("Block contains no transactions"));
        }

        // Deserialize and structurally validate Coinbase
        let first_tx = <Arc<Transaction>>::bitcoin_deserialize(&mut src)?;
        if !first_tx.is_coinbase() {
            return Err(SerializationError::Parse(
                "Block did not contain Coinbase in first position",
            ));
        }
        // TODO: Parse block height
        if header.version >= 2 {}
        // Sanity check number of transactions to prevent DOS attacks
        if tx_count > MAX_BLOCK_BYTES / (36 * 4) {
            return Err(SerializationError::Parse(
                "Block contained too many transactions for each to have at least one input",
            ));
        }
        let mut transactions = Vec::with_capacity(tx_count as usize);
        transactions.push(first_tx);

        // Parse and validate remaining transactions
        for _ in 1..tx_count {
            let next = <Arc<Transaction>>::bitcoin_deserialize(&mut src)?;
            if next.is_coinbase() {
                return Err(SerializationError::Parse("Block contained second Coinbase"));
            }
            transactions.push(next);
        }
        let actual_merkle_root = merkle::Root::from_iter(transactions.iter().map(|tx| tx.hash()));
        if !(actual_merkle_root == header.merkle_root) {
            return Err(SerializationError::Parse("Invalid Merkle Root"));
        }
        Ok(Block {
            header,
            transactions,
        })
    }
}

impl<'a> From<&'a Block> for Hash {
    fn from(block: &'a Block) -> Hash {
        (&block.header).into()
    }
}
