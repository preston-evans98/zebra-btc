//! Definitions of network messages.

use std::error::Error;
use std::{fmt, sync::Arc};

use zebra_chain::{
    block::{self, Block},
    transaction::Transaction,
};

use super::inv::InventoryHash;
use super::types::*;
use crate::meta_addr::MetaAddr;

mod version;
pub use version::Version;

mod get_blocks;
pub use get_blocks::GetBlocks;

mod get_headers;
pub use get_headers::GetHeaders;

mod merkle_block;
pub use merkle_block::MerkleBlock;

mod compact_block;
pub use compact_block::CompactBlock;

mod get_block_txn;
pub use get_block_txn::GetBlockTxn;

mod block_txn;
pub use block_txn::BlockTxn;

mod send_compact;
pub use send_compact::SendCompact;

use super::Command;

pub trait Payload {
    fn serialized_size(&self) -> usize;
    fn to_bytes(&self) -> Result<Vec<u8>, std::io::Error>;
}

/// A Bitcoin network message.
///
/// The Zcash network protocol is mostly inherited from Bitcoin, and a list of
/// Bitcoin network messages can be found [on the Bitcoin
/// wiki][btc_wiki_protocol].
///
/// That page describes the wire format of the messages, while this enum stores
/// an internal representation. The internal representation is unlinked from the
/// wire format, and the translation between the two happens only during
/// serialization and deserialization. For instance, Bitcoin identifies messages
/// by a 12-byte ascii command string; we consider this a serialization detail
/// and use the enum discriminant instead. (As a side benefit, this also means
/// that we have a clearly-defined validation boundary for network messages
/// during serialization).
///
/// [btc_wiki_protocol]: https://en.bitcoin.it/wiki/Protocol_documentation
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Message {
    /// A `version` message.
    ///
    /// Note that although this is called `version` in Bitcoin, its role is really
    /// analogous to a `ClientHello` message in TLS, used to begin a handshake, and
    /// is distinct from a simple version number.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#version)
    Version(Version),

    /// A `verack` message.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#verack)
    Verack,

    /// A `ping` message.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#ping)
    Ping(
        /// A nonce unique to this [`Ping`] message.
        Nonce,
    ),

    /// A `pong` message.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#pong)
    Pong(
        /// The nonce from the [`Ping`] message this was in response to.
        Nonce,
    ),

    /// A `reject` message.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#reject)
    Reject {
        /// Type of message rejected.
        // It's unclear if this is strictly limited to message command
        // codes, so leaving it a String.
        message: String,

        /// RejectReason code relating to rejected message.
        ccode: RejectReason,

        /// Human-readable version of rejection reason.
        reason: String,

        /// Optional extra data provided for some errors.
        // Currently, all errors which provide this field fill it with
        // the TXID or block header hash of the object being rejected,
        // so the field is 32 bytes.
        //
        // Q: can we tell Rust that this field is optional? Or just
        // default its value to an empty array, I guess.
        data: Option<[u8; 32]>,
    },

    /// A `getaddr` message.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#getaddr)
    GetAddr,

    /// An `addr` message.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#addr)
    Addr(Vec<MetaAddr>),

    /// A `getblocks` message.
    ///
    /// `known_blocks` is a series of known block hashes spaced out along the
    /// peer's best chain. The remote peer uses them to compute the intersection
    /// of its best chain and determine the blocks following the intersection
    /// point.
    ///
    /// The peer responds with an `inv` packet with the hashes of subsequent blocks.
    /// If supplied, the `stop` parameter specifies the last header to request.
    /// Otherwise, an inv packet with the maximum number (500) are sent.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#getheaders)
    GetBlocks(GetBlocks),

    /// An `inv` message.
    ///
    /// Allows a node to advertise its knowledge of one or more
    /// objects. It can be received unsolicited, or in reply to
    /// `getblocks`.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#inv)
    Inv(Vec<InventoryHash>),

    /// A `getheaders` message.
    ///
    /// `known_blocks` is a series of known block hashes spaced out along the
    /// peer's best chain. The remote peer uses them to compute the intersection
    /// of its best chain and determine the blocks following the intersection
    /// point.
    ///
    /// The peer responds with an `headers` packet with the headers of subsequent blocks.
    /// If supplied, the `stop` parameter specifies the last header to request.
    /// Otherwise, the maximum number of block headers (160) are sent.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#getheaders)
    GetHeaders(
        /// Hashes of known blocks, ordered from highest height to lowest height.
        GetHeaders, // known_blocks: Vec<block::Hash>,
                    // /// Optionally, the last header to request.
                    // stop: Option<block::Hash>
    ),

    /// A `headers` message.
    ///
    /// Returns block headers in response to a getheaders packet.
    ///
    /// Each block header is accompanied by a transaction count.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#headers)
    Headers(Vec<block::CountedHeader>),

    /// A `getdata` message.
    ///
    /// `getdata` is used in response to `inv`, to retrieve the
    /// content of a specific object, and is usually sent after
    /// receiving an `inv` packet, after filtering known elements.
    ///
    /// `zcashd` returns requested items in a single batch of messages.
    /// Missing blocks are silently skipped. Missing transaction hashes are
    /// included in a single `NotFound` message following the transactions.
    /// Other item or non-item messages can come before or after the batch.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#getdata)
    /// [zcashd code](https://github.com/zcash/zcash/blob/e7b425298f6d9a54810cb7183f00be547e4d9415/src/main.cpp#L5523)
    GetData(Vec<InventoryHash>),

    /// A `block` message.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#block)
    Block(Arc<Block>),

    /// A `tx` message.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#tx)
    Tx(Arc<Transaction>),

    /// A `notfound` message.
    ///
    /// When a peer requests a list of transaction hashes, `zcashd` returns:
    ///   - a batch of messages containing found transactions, then
    ///   - a `NotFound` message containing a list of transaction hashes that
    ///      aren't available in its mempool or state.
    ///
    /// But when a peer requests blocks or headers, any missing items are
    /// silently skipped, without any `NotFound` messages.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#notfound)
    /// [zcashd code](https://github.com/zcash/zcash/blob/e7b425298f6d9a54810cb7183f00be547e4d9415/src/main.cpp#L5632)
    // See note above on `Inventory`.
    NotFound(Vec<InventoryHash>),

    /// A `mempool` message.
    ///
    /// This was defined in [BIP35], which is included in Zcash.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#mempool)
    /// [BIP35]: https://github.com/bitcoin/bips/blob/master/bip-0035.mediawiki
    Mempool,

    /// A `filterload` message.
    ///
    /// This was defined in [BIP37], which is included in Zcash.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#filterload.2C_filteradd.2C_filterclear.2C_merkleblock)
    /// [BIP37]: https://github.com/bitcoin/bips/blob/master/bip-0037.mediawiki
    FilterLoad {
        /// The filter itself is simply a bit field of arbitrary
        /// byte-aligned size. The maximum size is 36,000 bytes.
        filter: Filter,

        /// The number of hash functions to use in this filter. The
        /// maximum value allowed in this field is 50.
        hash_functions_count: u32,

        /// A random value to add to the seed value in the hash
        /// function used by the bloom filter.
        tweak: Tweak,

        /// A set of flags that control how matched items are added to the filter.
        flags: u8,
    },

    /// A `filteradd` message.
    ///
    /// This was defined in [BIP37], which is included in Zcash.
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#filterload.2C_filteradd.2C_filterclear.2C_merkleblock)
    /// [BIP37]: https://github.com/bitcoin/bips/blob/master/bip-0037.mediawiki
    FilterAdd {
        /// The data element to add to the current filter.
        // The data field must be smaller than or equal to 520 bytes
        // in size (the maximum size of any potentially matched
        // object).
        //
        // A Vec instead of [u8; 520] because of needed traits.
        data: Vec<u8>,
    },

    /// A `filterclear` message.
    ///
    /// This was defined in [BIP37]
    ///
    /// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#filterload.2C_filteradd.2C_filterclear.2C_merkleblock)
    /// [BIP37]: https://github.com/bitcoin/bips/blob/master/bip-0037.mediawiki
    FilterClear,

    /// A `MerkleBlock` message.
    ///
    /// A reply to a `getdata`` message which requested a block using the inventory type MSG_MERKLEBLOCK.
    /// It is only part of the reply: if any matching transactions are found, they will be sent separately as `tx` messages.
    /// [Bitccoin Reference](https://developer.bitcoin.org/reference/p2p_networking.html#merkleblock)
    MerkleBlock(MerkleBlock),

    /// A `compactblock` message.
    ///
    /// The “cmpctblock” message is a reply to a “getdata” message which requested a block using the inventory type “MSG_CMPCT_BLOCK”.
    /// Note that “cmpctblock” is sent only if the requested block is recent enough.
    /// [BIP 157](https://github.com/bitcoin/bips/blob/master/bip-0152.mediawiki)
    CompactBlock(CompactBlock),

    /// A `getblocktxn` message
    ///
    /// Upon receipt of a properly-formatted “getblocktxn” message,
    /// nodes which recently provided the sender of such a message a “cmpctblock” message
    /// for the block hash identified - [Bitcoin Reference](https://developer.bitcoin.org/reference/p2p_networking.html#getblocktxn)
    /// Specified in [BIP 152](https://github.com/bitcoin/bips/blob/master/bip-0152.mediawiki)
    GetBlockTxn(GetBlockTxn),

    /// A `blocktxn` essage
    ///
    /// Contains transactions from a cmpctblck message which the requester did not have in their mempool.
    BlockTxn(BlockTxn),

    /// A `sendcompact` message
    ///
    /// Requests that the receiver send cmpctblcks in future.
    /// [Bitcoin Reference](https://developer.bitcoin.org/reference/p2p_networking.html#sendcmpct)
    /// Defined in [BIP 152](https://github.com/bitcoin/bips/blob/master/bip-0152.mediawiki)
    SendCompact(SendCompact),

    /// A `Feefilter` message
    ///
    /// The “feefilter” message is a request to the receiving peer to not relay any transaction inv messages to
    /// the sending peer where the fee rate for the transaction is below the fee rate specified in the feefilter message.
    /// Defined in [BIP 133](https://github.com/bitcoin/bips/blob/master/bip-0133.mediawiki)
    FeeFilter(u64),

    /// A `sendheaders` message
    ///
    /// The “sendheaders” message tells the receiving peer to send new block
    /// announcements using a “headers” message rather than an “inv” message.
    SendHeaders,
}

impl<E> From<E> for Message
where
    E: Error,
{
    fn from(e: E) -> Self {
        Message::Reject {
            message: e.to_string(),

            // The generic case, impls for specific error types should
            // use specific varieties of `RejectReason`.
            ccode: RejectReason::Other,

            reason: e.source().unwrap().to_string(),

            // Allow this to be overridden but not populated by default, methinks.
            data: None,
        }
    }
}

/// Reject Reason CCodes
///
/// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#reject)
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum RejectReason {
    Malformed = 0x01,
    Invalid = 0x10,
    Obsolete = 0x11,
    Duplicate = 0x12,
    Nonstandard = 0x40,
    Dust = 0x41,
    InsufficientFee = 0x42,
    Checkpoint = 0x43,
    Other = 0x50,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Message::Version { .. } => "version",
            Message::Verack => "verack",
            Message::Ping(_) => "ping",
            Message::Pong(_) => "pong",
            Message::Reject { .. } => "reject",
            Message::GetAddr => "getaddr",
            Message::Addr(_) => "addr",
            Message::GetBlocks { .. } => "getblocks",
            Message::Inv(_) => "inv",
            Message::GetHeaders { .. } => "getheaders",
            Message::Headers(_) => "headers",
            Message::GetData(_) => "getdata",
            Message::Block(_) => "block",
            Message::Tx(_) => "tx",
            Message::NotFound(_) => "notfound",
            Message::Mempool => "mempool",
            Message::FilterLoad { .. } => "filterload",
            Message::FilterAdd { .. } => "filteradd",
            Message::FilterClear => "filterclear",
            Message::MerkleBlock(_) => "merkleblock",
            Message::CompactBlock(_) => "cmpctblck",
            Message::GetBlockTxn(_) => "getblocktxn",
            Message::BlockTxn(_) => "blocktxn",
            Message::SendCompact(_) => "sendcmpct",
            Message::FeeFilter(_) => "feefilter",
            Message::SendHeaders => "sendheaders",
        })
    }
}

impl Message {
    pub fn command(&self) -> Command {
        match self {
            Message::Addr { .. } => Command::Addr,
            Message::BlockTxn { .. } => Command::BlockTxn,
            Message::Block { .. } => Command::Block,
            Message::CompactBlock { .. } => Command::CmpctBlock,
            Message::FeeFilter { .. } => Command::FeeFilter,
            Message::FilterAdd { .. } => Command::FilterAdd,
            Message::FilterClear {} => Command::FilterClear,
            Message::FilterLoad { .. } => Command::FilterLoad,
            Message::GetAddr {} => Command::GetAddr,
            Message::GetBlockTxn { .. } => Command::GetBlockTxn,
            Message::GetBlocks { .. } => Command::GetBlocks,
            Message::GetData { .. } => Command::GetData,
            Message::GetHeaders { .. } => Command::GetHeaders,
            Message::Headers { .. } => Command::Headers,
            Message::Inv { .. } => Command::Inv,
            Message::Mempool {} => Command::MemPool,
            Message::MerkleBlock { .. } => Command::MerkleBlock,
            Message::NotFound { .. } => Command::MemPool,
            Message::Ping { .. } => Command::Ping,
            Message::Pong { .. } => Command::Pong,
            Message::Reject { .. } => Command::Reject,
            Message::SendCompact { .. } => Command::SendCmpct,
            Message::SendHeaders {} => Command::SendHeaders,
            Message::Tx { .. } => Command::Tx,
            Message::Verack {} => Command::Verack,
            Message::Version { .. } => Command::Version,
        }
    }
}
