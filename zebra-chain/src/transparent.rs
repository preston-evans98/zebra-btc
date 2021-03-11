//! Transparent-related (Bitcoin-inherited) functionality.
#![allow(clippy::unit_arg)]

mod address;
mod keys;
mod script;
mod serialize;

pub use address::Address;
pub use script::Script;

use crate::{
    cached::Cached, compactint::CompactInt, BitcoinDeserialize, BitcoinSerialize,
    SerializationError,
};
use bitcoin_serde_derive::{BtcDeserialize, BtcSerialize};

#[cfg(any(test, feature = "proptest-impl"))]
use proptest_derive::Arbitrary;

#[cfg(any(test, feature = "proptest-impl"))]
mod arbitrary;
#[cfg(test)]
mod prop;

use crate::{
    amount::{Amount, NonNegative},
    block, transaction,
};

/// Arbitrary data inserted by miners into a coinbase transaction.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, BtcDeserialize, BtcSerialize)]
pub struct CoinbaseData(
    /// Invariant: this vec must be less than 100 bytes.
    /// We enforce this by only constructing CoinbaseData fields by
    /// parsing blocks with 100-byte data fields. When we implement block
    /// creation, we should provide a constructor for the (non-blockheight) coinbase data field
    /// that restricts it to 95 = 100 -1 -4 bytes (safe for any block height up
    /// to 500_000_000).
    pub(super) Vec<u8>,
);

impl AsRef<[u8]> for CoinbaseData {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl CoinbaseData {
    pub fn serialized_size(&self) -> usize {
        CompactInt::size(self.0.len()) + self.0.len()
    }
}

impl std::fmt::Debug for CoinbaseData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let escaped = String::from_utf8(
            self.0
                .iter()
                .cloned()
                .flat_map(std::ascii::escape_default)
                .collect(),
        )
        .expect("ascii::escape_default produces utf8");
        f.debug_tuple("CoinbaseData").field(&escaped).finish()
    }
}

/// OutPoint
///
/// A particular transaction output reference.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash, BtcSerialize)]
#[cfg_attr(any(test, feature = "proptest-impl"), derive(Arbitrary))]
pub struct OutPoint {
    /// References the transaction that contains the UTXO being spent.
    pub hash: transaction::Hash,

    /// Identifies which UTXO from that transaction is referenced; the
    /// first output is 0, etc.
    pub index: u32,
}

impl OutPoint {
    #[inline]
    pub const fn len() -> usize {
        36
    }
}
/// A transparent input to a transaction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Input {
    /// A reference to an output of a previous transaction.
    PrevOut {
        /// The previous output transaction reference.
        outpoint: OutPoint,
        /// The script that authorizes spending `outpoint`.
        unlock_script: Script,
        /// The sequence number for the output.
        sequence: u32,
    },
    /// New coins created by the block reward.
    Coinbase {
        /// The height of this block, which can be computed from the coinbase data after BIP 34 activation.
        /// The `Cached<block::height>` is wrapped in an explicit option to make it clear that not all blocks have a height encoded
        height: Option<Cached<block::Height>>,
        /// Free data inserted by miners.  
        /// Includes the block height post BIP 34
        //// Note that Block number 227,835 (timestamp 2013-03-24 15:49:13 GMT) was the last version 1 block.
        data: CoinbaseData,
        /// The sequence number for the output.
        sequence: u32,
    },
}

impl Input {
    pub fn len(&self) -> usize {
        match *self {
            Input::PrevOut {
                outpoint,
                ref unlock_script,
                sequence,
            } => OutPoint::len() + unlock_script.serialized_size() + 4,
            Input::Coinbase {
                height,
                ref data,
                sequence,
            } => data.serialized_size() + 4,
        }
    }
}

/// A transparent output from a transaction.
///
/// The most fundamental building block of a transaction is a
/// transaction output -- the ZEC you own in your "wallet" is in
/// fact a subset of unspent transaction outputs (or "UTXO"s) of the
/// global UTXO set.
///
/// UTXOs are indivisible, discrete units of value which can only be
/// consumed in their entirety. Thus, if I want to send you 1 ZEC and
/// I only own one UTXO worth 2 ZEC, I would construct a transaction
/// that spends my UTXO and sends 1 ZEC to you and 1 ZEC back to me
/// (just like receiving change).
#[derive(
    Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash, BtcDeserialize, BtcSerialize,
)]
#[cfg_attr(any(test, feature = "proptest-impl"), derive(Arbitrary))]
pub struct Output {
    /// Transaction value.
    // At https://en.bitcoin.it/wiki/Protocol_documentation#tx, this is an i64.
    pub value: Amount<NonNegative>,

    /// The lock script defines the conditions under which this output can be spent.
    pub lock_script: Script,
}

impl Output {
    /// Returns the serialized length (in bytes) of this Output
    pub fn len(&self) -> usize {
        8 + self.lock_script.serialized_size()
    }
}
