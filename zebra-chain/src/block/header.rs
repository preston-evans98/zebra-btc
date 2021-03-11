use std::io::Write;

use bytes::Buf;
use chrono::{DateTime, Duration, Utc};
use thiserror::Error;

use crate::{serialization::sha256d, BitcoinDeserialize, BitcoinSerialize, SerializationError};
use bitcoin_serde_derive::{BtcDeserialize, BtcSerialize};

use crate::{cached::Cached, work::difficulty::CompactDifficulty};

use super::{merkle, Hash, Height};

/// A block header, containing metadata about a block.
///
/// How are blocks chained together? They are chained together via the
/// backwards reference (previous header hash) present in the block
/// header. Each block points backwards to its parent, all the way
/// back to the genesis block (the first block in the blockchain).
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, BtcSerialize, BtcDeserialize,
)]
pub struct Header {
    /// The block's version field.
    pub version: u32,

    /// The hash of the previous block, used to create a chain of blocks back to
    /// the genesis block.
    ///
    /// This ensures no previous block can be changed without also changing this
    /// block’s header.
    pub previous_block_hash: Hash,

    /// The root of the Bitcoin-inherited transaction Merkle tree, binding the
    /// block header to the transactions in the block.
    ///
    /// Note that because of a flaw in Bitcoin's design, the `merkle_root` does
    /// not always precisely bind the contents of the block (CVE-2012-2459). It
    /// is sometimes possible for an attacker to create multiple distinct sets of
    /// transactions with the same Merkle root, although only one set will be
    /// valid.
    pub merkle_root: merkle::Root,

    /// The block timestamp is a Unix epoch time (UTC) when the miner
    /// started hashing the header (according to the miner).
    pub time: DateTime<Utc>,

    /// An encoded version of the target threshold this block’s header
    /// hash must be less than or equal to, in the same nBits format
    /// used by Bitcoin.
    ///
    /// For a block at block height `height`, bits MUST be equal to
    /// `ThresholdBits(height)`.
    ///
    /// [Bitcoin-nBits](https://bitcoin.org/en/developer-reference#target-nbits)
    pub difficulty_threshold: CompactDifficulty,

    /// An arbitrary field that miners can change to modify the header
    /// hash in order to produce a hash less than or equal to the
    /// target threshold.
    pub nonce: u32,

    /// For efficiency, we cache the hash of this block after computing it the first time
    /// This field is not part of the consensus serialization
    hash: Cached<Hash>,

    /// The BIP-34 block height of this block
    /// This field is not part of the consensus serialization
    reported_height: Cached<usize>,
}

/// TODO: Use this error as the source for zebra_consensus::error::BlockError::Time,
/// and make `BlockError::Time` add additional context.
/// See https://github.com/ZcashFoundation/zebra/issues/1021 for more details.
#[derive(Error, Debug)]
pub enum BlockTimeError {
    #[error("invalid time {0:?} in block header {1:?} {2:?}: block time is more than 2 hours in the future ({3:?}). Hint: check your machine's date, time, and time zone.")]
    InvalidBlockTime(
        DateTime<Utc>,
        crate::block::Height,
        crate::block::Hash,
        DateTime<Utc>,
    ),
}

impl Header {
    // Returns length of serialized header in bytes
    pub const fn len() -> usize {
        80
    }
    pub fn new(
        version: u32,
        previous_block_hash: Hash,
        merkle_root: merkle::Root,
        time: DateTime<Utc>,
        difficulty_threshold: CompactDifficulty,
        nonce: u32,
    ) -> Header {
        Header {
            version,
            previous_block_hash,
            merkle_root,
            time,
            difficulty_threshold,
            nonce,
            hash: Cached::new(),
            reported_height: Cached::new(),
        }
    }
    /// TODO: Inline this function into zebra_consensus::block::check::time_is_valid_at.
    /// See https://github.com/ZcashFoundation/zebra/issues/1021 for more details.
    pub fn time_is_valid_at(
        &self,
        now: DateTime<Utc>,
        height: &Height,
        hash: &Hash,
    ) -> Result<(), BlockTimeError> {
        let two_hours_in_the_future = now
            .checked_add_signed(Duration::hours(2))
            .expect("calculating 2 hours in the future does not overflow");
        if self.time <= two_hours_in_the_future {
            Ok(())
        } else {
            Err(BlockTimeError::InvalidBlockTime(
                self.time,
                *height,
                *hash,
                two_hours_in_the_future,
            ))?
        }
    }

    pub fn bitcoin_serialize_to_vec(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Header::len());
        self.bitcoin_serialize(&mut out);
        out
    }
    /// Deserializes a block header from a Buf object, allowing more efficient block hash calculation than vanilla deserialize
    pub fn deserialize_from_buf<B: Buf>(mut src: B) -> Result<Self, SerializationError> {
        if src.remaining() < Header::len() {
            return Err(SerializationError::Parse(
                "Not enough bytes in block header",
            ));
        }
        // Hash the next 80 bytes (the header). This avoids needing to reserialize and hash later.
        // Note: this op is zero-copy if the underlying is a Bytes or BytesMut object
        let mut src = src.copy_to_bytes(80);
        let mut hash_writer = sha256d::Writer::default();
        hash_writer.write_all(&src[..]);
        let hash_bytes = hash_writer.finish();
        let own_hash = Hash::from_bytes(hash_bytes);

        // Deserialize the ehader
        let mut src = std::io::Cursor::new(src);
        Ok(Header {
            version: u32::bitcoin_deserialize(&mut src)?,
            previous_block_hash: Hash::bitcoin_deserialize(&mut src)?,
            merkle_root: merkle::Root::bitcoin_deserialize(&mut src)?,
            time: <DateTime<Utc>>::bitcoin_deserialize(&mut src)?,
            difficulty_threshold: CompactDifficulty::bitcoin_deserialize(&mut src)?,
            nonce: u32::bitcoin_deserialize(&mut src)?,
            hash: Cached::from(own_hash),
            reported_height: Cached::new(),
        })
    }
}

/// A header with a count of the number of transactions in its block.
///
/// This structure is used in the Bitcoin network protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountedHeader {
    pub header: Header,
    pub transaction_count: usize,
}
