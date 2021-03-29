//! A Tokio codec mapping byte streams to Bitcoin message streams.

use std::fmt;
use std::{
    io::{Cursor, Read, Write},
    sync::Arc,
};

use crate::meta_addr::MetaAddr;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

use zebra_chain::{
    block,
    parameters::Network,
    serialization::{sha256d, BitcoinDeserialize, BitcoinSerialize, SerializationError as Error},
    transaction::Transaction,
};

use crate::constants;

use super::{
    command::Command,
    inv::InventoryHash,
    message::{
        BlockTxn, CompactBlock, GetBlockTxn, GetBlocks, GetHeaders, MerkleBlock, Message,
        RejectReason, SendCompact, Version,
    },
    types::*,
};

/// The length of a Bitcoin message header.
const HEADER_LEN: usize = 24usize;

/// Maximum size of a protocol message body.
const MAX_PROTOCOL_MESSAGE_LEN: usize = 2 * 1024 * 1024;

/// A codec which produces Bitcoin messages from byte streams and vice versa.
pub struct Codec {
    builder: Builder,
    state: DecodeState,
}

/// A builder for specifying [`Codec`] options.
pub struct Builder {
    /// The network magic to use in encoding.
    network: Network,
    /// The protocol version to speak when encoding/decoding.
    version: ProtocolVersion,
    /// The maximum allowable message length.
    max_len: usize,
    /// An optional label to use for reporting metrics.
    metrics_label: Option<String>,
}

impl Codec {
    /// Return a builder for constructing a [`Codec`].
    pub fn builder() -> Builder {
        Builder {
            network: Network::Mainnet,
            version: constants::CURRENT_VERSION,
            max_len: MAX_PROTOCOL_MESSAGE_LEN,
            metrics_label: None,
        }
    }

    /// Reconfigure the version used by the codec, e.g., after completing a handshake.
    pub fn reconfigure_version(&mut self, version: ProtocolVersion) {
        self.builder.version = version;
    }
}

impl Builder {
    /// Finalize the builder and return a [`Codec`].
    pub fn finish(self) -> Codec {
        Codec {
            builder: self,
            state: DecodeState::Head,
        }
    }

    /// Configure the codec for the given [`Network`].
    pub fn for_network(mut self, network: Network) -> Self {
        self.network = network;
        self
    }

    /// Configure the codec for the given [`Version`].
    #[allow(dead_code)]
    pub fn for_version(mut self, version: ProtocolVersion) -> Self {
        self.version = version;
        self
    }

    /// Configure the codec's maximum accepted payload size, in bytes.
    #[allow(dead_code)]
    pub fn with_max_body_len(mut self, len: usize) -> Self {
        self.max_len = len;
        self
    }

    /// Configure the codec for the given peer address.
    pub fn with_metrics_label(mut self, metrics_label: String) -> Self {
        self.metrics_label = Some(metrics_label);
        self
    }
}

// ======== Encoding =========

impl Encoder<Message> for Codec {
    type Error = Error;

    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        use Error::Parse;
        // XXX(HACK): this is inefficient and does an extra allocation.
        // instead, we should have a size estimator for the message, reserve
        // that much space, write the header (with zeroed checksum), then the body,
        // then write the computed checksum in-place.  for now, just do an extra alloc.

        let mut body = Vec::new();
        self.write_body(&item, &mut body)?;

        if body.len() > self.builder.max_len {
            return Err(Parse("body length exceeded maximum size"));
        }

        if let Some(label) = self.builder.metrics_label.clone() {
            metrics::counter!("bytes.written", (body.len() + HEADER_LEN) as u64, "addr" =>  label);
        }

        let command = item.command();
        let command = command.bytes();
        trace!(?item, len = body.len());

        // XXX this should write directly into the buffer,
        // but leave it for now until we fix the issue above.
        let mut header = [0u8; HEADER_LEN];
        let mut header_writer = Cursor::new(&mut header[..]);
        header_writer.write_all(&Magic::from(self.builder.network).0[..])?;
        header_writer.write_all(command)?;
        header_writer.write_u32::<LittleEndian>(body.len() as u32)?;
        header_writer.write_all(&sha256d::Checksum::from(&body[..]).0)?;

        dst.reserve(HEADER_LEN + body.len());
        dst.extend_from_slice(&header);
        dst.extend_from_slice(&body);

        Ok(())
    }
}

impl Codec {
    /// Write the body of the message into the given writer. This allows writing
    /// the message body prior to writing the header, so that the header can
    /// contain a checksum of the message body.
    fn write_body<W: Write>(&self, msg: &Message, mut writer: W) -> Result<(), Error> {
        match msg {
            Message::Version(inner) => {
                inner.bitcoin_serialize(&mut writer)?;
            }
            Message::Verack => { /* Empty payload -- no-op */ }
            Message::Ping(nonce) => {
                nonce.bitcoin_serialize(&mut writer)?;
            }
            Message::Pong(nonce) => {
                nonce.bitcoin_serialize(&mut writer)?;
            }
            Message::Reject {
                message,
                ccode,
                reason: _,
                data,
            } => {
                // todo!()
                message.bitcoin_serialize(&mut writer)?;
                writer.write_u8(*ccode as u8)?;
                message.bitcoin_serialize(&mut writer)?;
                writer.write_all(&data.unwrap())?;
            }
            Message::Addr(addrs) => addrs.bitcoin_serialize(&mut writer)?,
            Message::GetAddr => { /* Empty payload -- no-op */ }
            Message::Block(block) => block.bitcoin_serialize(&mut writer)?,
            Message::GetBlocks(get_blocks) => {
                self.builder.version.bitcoin_serialize(&mut writer)?;
                get_blocks.bitcoin_serialize(&mut writer)?
            }
            Message::GetHeaders(get_headers) => {
                self.builder.version.bitcoin_serialize(&mut writer)?;
                get_headers.bitcoin_serialize(&mut writer)?
            }
            Message::Headers(headers) => headers.bitcoin_serialize(&mut writer)?,
            Message::Inv(hashes) => hashes.bitcoin_serialize(&mut writer)?,
            Message::GetData(hashes) => hashes.bitcoin_serialize(&mut writer)?,
            Message::NotFound(hashes) => hashes.bitcoin_serialize(&mut writer)?,
            Message::Tx(transaction) => transaction.bitcoin_serialize(&mut writer)?,
            Message::Mempool => { /* Empty payload -- no-op */ }
            Message::FilterLoad {
                filter,
                hash_functions_count,
                tweak,
                flags,
            } => {
                writer.write_all(&filter.0)?;
                writer.write_u32::<LittleEndian>(*hash_functions_count)?;
                writer.write_u32::<LittleEndian>(tweak.0)?;
                writer.write_u8(*flags)?;
            }
            Message::FilterAdd { data } => writer.write_all(data)?,
            Message::FilterClear => { /* Empty payload -- no-op */ }
            Message::MerkleBlock(inner) => inner.bitcoin_serialize(&mut writer)?,
            Message::CompactBlock(inner) => inner.bitcoin_serialize(&mut writer)?,
            Message::GetBlockTxn(inner) => inner.bitcoin_serialize(&mut writer)?,
            Message::BlockTxn(inner) => inner.bitcoin_serialize(&mut writer)?,
            Message::SendCompact(inner) => inner.bitcoin_serialize(&mut writer)?,
            Message::FeeFilter(inner) => inner.bitcoin_serialize(&mut writer)?,
            Message::SendHeaders => { /* Empty payload -- no-op */ }
        }
        Ok(())
    }
}

// ======== Decoding =========

enum DecodeState {
    Head,
    Body {
        body_len: usize,
        command: Command,
        checksum: sha256d::Checksum,
    },
}

impl fmt::Debug for DecodeState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecodeState::Head => write!(f, "DecodeState::Head"),
            DecodeState::Body {
                body_len,
                command,
                checksum,
            } => f
                .debug_struct("DecodeState::Body")
                .field("body_len", &body_len)
                .field("command", &command)
                .field("checksum", &checksum)
                .finish(),
        }
    }
}

impl Decoder for Codec {
    type Item = Message;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        use Error::Parse;
        match self.state {
            DecodeState::Head => {
                // First check that the src buffer contains an entire header.
                if src.len() < HEADER_LEN {
                    trace!(?self.state, "src buffer does not have an entire header, waiting");
                    // Signal that decoding requires more data.
                    return Ok(None);
                }

                // Now that we know that src contains a header, split off the header section.
                let header = src.split_to(HEADER_LEN);

                // Create a cursor over the header and parse its fields.
                let mut header_reader = Cursor::new(&header);
                let magic = Magic(<[u8; 4]>::bitcoin_deserialize(&mut header_reader)?);
                let command = Command::bitcoin_deserialize(&mut header_reader)?;
                let body_len = header_reader.read_u32::<LittleEndian>()? as usize;
                let checksum =
                    sha256d::Checksum(<[u8; 4]>::bitcoin_deserialize(&mut header_reader)?);
                trace!(
                    ?self.state,
                    ?magic,
                    command = ?command,
                    // command = %String::from_utf8(
                    //     command.iter()
                    //         .cloned()
                    //         .flat_map(std::ascii::escape_default)
                    //         .collect()
                    // ).unwrap(),
                    body_len,
                    ?checksum,
                    "read header from src buffer"
                );

                if magic != Magic::from(self.builder.network) {
                    return Err(Parse("supplied magic did not meet expectations"));
                }
                if body_len > self.builder.max_len {
                    return Err(Parse("body length exceeded maximum size"));
                }

                if let Some(label) = self.builder.metrics_label.clone() {
                    metrics::counter!("bytes.read", (body_len + HEADER_LEN) as u64, "addr" =>  label);
                }

                // Reserve buffer space for the expected body and the following header.
                src.reserve(body_len + HEADER_LEN);

                self.state = DecodeState::Body {
                    body_len,
                    command,
                    checksum,
                };

                // Now that the state is updated, recurse to attempt body decoding.
                self.decode(src)
            }
            DecodeState::Body {
                body_len,
                command,
                checksum,
            } => {
                if src.len() < body_len {
                    // Need to wait for the full body
                    trace!(?self.state, len = src.len(), "src buffer does not have an entire body, waiting");
                    return Ok(None);
                }

                // Now that we know we have the full body, split off the body,
                // and reset the decoder state for the next message. Otherwise
                // we will attempt to read the next header as the current body.
                let body = src.split_to(body_len);
                self.state = DecodeState::Head;

                if checksum != sha256d::Checksum::from(&body[..]) {
                    return Err(Parse(
                        "supplied message checksum does not match computed checksum",
                    ));
                }

                let mut body_reader = Cursor::new(&body);
                // Convention: deserialize the message directly (using `bitcoin_deserialize()`) unless
                // it requires context from the codec. In that case, use the codec's self.read_* method.
                let msg = match command {
                    Command::Addr => {
                        Message::Addr(Vec::<MetaAddr>::bitcoin_deserialize(&mut body_reader)?)
                    }
                    Command::Version => {
                        Message::Version(Version::bitcoin_deserialize(&mut body_reader)?)
                    }
                    Command::Verack => Message::Verack,
                    Command::GetBlocks => self.read_getblocks(&mut body_reader)?,
                    Command::GetData => Message::GetData(
                        <Vec<InventoryHash>>::bitcoin_deserialize(&mut body_reader)?,
                    ),
                    Command::Block => {
                        Message::Block(<Arc<block::Block>>::bitcoin_deserialize(&mut body_reader)?)
                    }

                    Command::GetHeaders => self.read_getheaders(&mut body_reader)?,
                    Command::Headers => Message::Headers(
                        <Vec<block::CountedHeader>>::bitcoin_deserialize(&mut body_reader)?,
                    ),
                    Command::Inv => {
                        Message::Inv(<Vec<InventoryHash>>::bitcoin_deserialize(&mut body_reader)?)
                    }
                    Command::MemPool => Message::Mempool,
                    Command::MerkleBlock => {
                        Message::MerkleBlock(MerkleBlock::bitcoin_deserialize(&mut body_reader)?)
                    }
                    Command::CmpctBlock => {
                        Message::CompactBlock(CompactBlock::bitcoin_deserialize(&mut body_reader)?)
                    }
                    Command::GetBlockTxn => {
                        Message::GetBlockTxn(GetBlockTxn::bitcoin_deserialize(&mut body_reader)?)
                    }
                    Command::BlockTxn => {
                        Message::BlockTxn(BlockTxn::bitcoin_deserialize(&mut body_reader)?)
                    }
                    Command::SendCmpct => {
                        Message::SendCompact(SendCompact::bitcoin_deserialize(&mut body_reader)?)
                    }
                    Command::NotFound => Message::NotFound(
                        <Vec<InventoryHash>>::bitcoin_deserialize(&mut body_reader)?,
                    ),
                    Command::Tx => {
                        Message::Tx(<Arc<Transaction>>::bitcoin_deserialize(&mut body_reader)?)
                    }
                    Command::Alert => {
                        // TODO: Verify that no additional cleanup is required.
                        self.state = DecodeState::Head;
                        debug!("Received Alert message! Alert is insecure and deprecated");
                        return Ok(None);
                    }
                    Command::FeeFilter => {
                        Message::FeeFilter(u64::bitcoin_deserialize(&mut body_reader)?)
                    }
                    Command::FilterAdd => self.read_filteradd(&mut body_reader)?,
                    Command::FilterClear => Message::FilterClear,
                    Command::FilterLoad => self.read_filterload(&mut body_reader, body_len)?,
                    Command::GetAddr => Message::GetAddr,
                    Command::Ping => Message::Ping(Nonce::bitcoin_deserialize(&mut body_reader)?),
                    Command::Pong => Message::Pong(Nonce::bitcoin_deserialize(&mut body_reader)?),
                    Command::Reject => self.read_reject(&mut body_reader)?,
                    Command::SendHeaders => Message::SendHeaders,
                };
                trace!("finished message decoding");
                Ok(Some(msg))
            }
        }
    }
}

impl Codec {
    fn read_reject<R: Read>(&self, mut reader: R) -> Result<Message, Error> {
        Ok(Message::Reject {
            message: String::bitcoin_deserialize(&mut reader)?,
            ccode: match reader.read_u8()? {
                0x01 => RejectReason::Malformed,
                0x10 => RejectReason::Invalid,
                0x11 => RejectReason::Obsolete,
                0x12 => RejectReason::Duplicate,
                0x40 => RejectReason::Nonstandard,
                0x41 => RejectReason::Dust,
                0x42 => RejectReason::InsufficientFee,
                0x43 => RejectReason::Checkpoint,
                0x50 => RejectReason::Other,
                _ => return Err(Error::Parse("invalid RejectReason value in ccode field")),
            },
            reason: String::bitcoin_deserialize(&mut reader)?,
            // Sometimes there's data, sometimes there isn't. There's no length
            // field, this is just implicitly encoded by the body_len.
            // Apparently all existing implementations only supply 32 bytes of
            // data (hash identifying the rejected object) or none (and we model
            // the Reject message that way), so instead of passing in the
            // body_len separately and calculating remaining bytes, just try to
            // read 32 bytes and ignore any failures.
            data: <[u8; 32]>::bitcoin_deserialize(&mut reader).ok(),
        })
    }

    fn read_getblocks<R: Read>(&self, mut reader: R) -> Result<Message, Error> {
        let received_version = ProtocolVersion::bitcoin_deserialize(&mut reader)?;
        let get_blocks = GetBlocks::bitcoin_deserialize(&mut reader)?;
        if self.builder.version == received_version {
            Ok(Message::GetBlocks(get_blocks))
        } else {
            Err(Error::Parse("getblocks version did not match negotiation"))
        }
    }

    fn read_getheaders<R: Read>(&self, mut reader: R) -> Result<Message, Error> {
        let received_version = ProtocolVersion::bitcoin_deserialize(&mut reader)?;
        let get_headers = GetHeaders::bitcoin_deserialize(&mut reader)?;
        if self.builder.version == received_version {
            Ok(Message::GetHeaders(get_headers))
        } else {
            Err(Error::Parse("getheaders version did not match negotiation"))
        }
    }

    fn read_filterload<R: Read>(&self, mut reader: R, body_len: usize) -> Result<Message, Error> {
        if !(FILTERLOAD_REMAINDER_LENGTH <= body_len
            && body_len <= FILTERLOAD_REMAINDER_LENGTH + MAX_FILTER_LENGTH)
        {
            return Err(Error::Parse("Invalid filterload message body length."));
        }

        const MAX_FILTER_LENGTH: usize = 36000;
        const FILTERLOAD_REMAINDER_LENGTH: usize = 4 + 4 + 1;

        let filter_length: usize = body_len - FILTERLOAD_REMAINDER_LENGTH;

        let mut filter_bytes = vec![0; filter_length];
        reader.read_exact(&mut filter_bytes)?;

        Ok(Message::FilterLoad {
            filter: Filter(filter_bytes),
            hash_functions_count: reader.read_u32::<LittleEndian>()?,
            tweak: Tweak(reader.read_u32::<LittleEndian>()?),
            flags: reader.read_u8()?,
        })
    }

    fn read_filteradd<R: Read>(&self, reader: R) -> Result<Message, Error> {
        let mut bytes = Vec::new();

        // Maximum size of data is 520 bytes.
        reader.take(520).read_exact(&mut bytes)?;

        Ok(Message::FilterAdd { data: bytes })
    }
}

// XXX replace these interior unit tests with exterior integration tests + proptest
#[cfg(test)]
mod tests {
    use super::*;
    use futures::prelude::*;
    use tokio::runtime::Runtime;

    #[test]
    fn version_message_round_trip() {
        zebra_test::init();
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};
        let services = PeerServices::NODE_NETWORK;

        let rt = Runtime::new().unwrap();

        let v = Message::Version(Version::new(
            crate::constants::CURRENT_VERSION,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 6)), 8333),
            services,
            services,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 6)), 8333),
            Nonce(0x9082_4908_8927_9238),
            "Beaver".to_owned(),
            block::Height(540_000),
            true,
        ));

        use tokio_util::codec::{FramedRead, FramedWrite};
        let v_bytes = rt.block_on(async {
            let mut bytes = Vec::new();
            {
                let mut fw = FramedWrite::new(&mut bytes, Codec::builder().finish());
                fw.send(v.clone())
                    .await
                    .expect("message should be serialized");
            }
            bytes
        });

        let v_parsed = rt.block_on(async {
            let mut fr = FramedRead::new(Cursor::new(&v_bytes), Codec::builder().finish());
            fr.next()
                .await
                .expect("a next message should be available")
                .expect("that message should deserialize")
        });

        assert_eq!(v, v_parsed);
    }

    #[test]
    fn filterload_message_round_trip() {
        zebra_test::init();

        let rt = Runtime::new().unwrap();

        let v = Message::FilterLoad {
            filter: Filter(vec![0; 35999]),
            hash_functions_count: 0,
            tweak: Tweak(0),
            flags: 0,
        };

        use tokio_util::codec::{FramedRead, FramedWrite};
        let v_bytes = rt.block_on(async {
            let mut bytes = Vec::new();
            {
                let mut fw = FramedWrite::new(&mut bytes, Codec::builder().finish());
                fw.send(v.clone())
                    .await
                    .expect("message should be serialized");
            }
            bytes
        });

        let v_parsed = rt.block_on(async {
            let mut fr = FramedRead::new(Cursor::new(&v_bytes), Codec::builder().finish());
            fr.next()
                .await
                .expect("a next message should be available")
                .expect("that message should deserialize")
        });

        assert_eq!(v, v_parsed);
    }

    #[test]
    fn filterload_message_too_large_round_trip() {
        zebra_test::init();

        let rt = Runtime::new().unwrap();

        let v = Message::FilterLoad {
            filter: Filter(vec![0; 40000]),
            hash_functions_count: 0,
            tweak: Tweak(0),
            flags: 0,
        };

        use tokio_util::codec::{FramedRead, FramedWrite};
        let v_bytes = rt.block_on(async {
            let mut bytes = Vec::new();
            {
                let mut fw = FramedWrite::new(&mut bytes, Codec::builder().finish());
                fw.send(v.clone())
                    .await
                    .expect("message should be serialized");
            }
            bytes
        });

        rt.block_on(async {
            let mut fr = FramedRead::new(Cursor::new(&v_bytes), Codec::builder().finish());
            fr.next()
                .await
                .expect("a next message should be available")
                .expect_err("that message should not deserialize")
        });
    }

    #[test]
    fn max_msg_size_round_trip() {
        use std::sync::Arc;
        use zebra_chain::serialization::BitcoinDeserializeInto;
        zebra_test::init();

        let rt = Runtime::new().unwrap();

        // make tests with a Tx message
        let tx = zebra_test::vectors::DUMMY_TX1
            .bitcoin_deserialize_into()
            .unwrap();
        let msg = Message::Tx(Arc::new(tx));

        use tokio_util::codec::{FramedRead, FramedWrite};

        // i know the above msg has a body of 85 bytes
        let size = 85;

        // reducing the max size to body size - 1
        rt.block_on(async {
            let mut bytes = Vec::new();
            {
                let mut fw = FramedWrite::new(
                    &mut bytes,
                    Codec::builder().with_max_body_len(size - 1).finish(),
                );
                fw.send(msg.clone()).await.expect_err(
                    "message should not encode as it is bigger than the max allowed value",
                );
            }
        });

        // send again with the msg body size as max size
        let msg_bytes = rt.block_on(async {
            let mut bytes = Vec::new();
            {
                let mut fw = FramedWrite::new(
                    &mut bytes,
                    Codec::builder().with_max_body_len(size).finish(),
                );
                fw.send(msg.clone())
                    .await
                    .expect("message should encode with the msg body size as max allowed value");
            }
            bytes
        });

        // receive with a reduced max size
        rt.block_on(async {
            let mut fr = FramedRead::new(
                Cursor::new(&msg_bytes),
                Codec::builder().with_max_body_len(size - 1).finish(),
            );
            fr.next()
                .await
                .expect("a next message should be available")
                .expect_err("message should not decode as it is bigger than the max allowed value")
        });

        // receive again with the tx size as max size
        rt.block_on(async {
            let mut fr = FramedRead::new(
                Cursor::new(&msg_bytes),
                Codec::builder().with_max_body_len(size).finish(),
            );
            fr.next()
                .await
                .expect("a next message should be available")
                .expect("message should decode with the msg body size as max allowed value")
        });
    }
}
