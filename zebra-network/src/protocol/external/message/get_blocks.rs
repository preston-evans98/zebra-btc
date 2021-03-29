use zebra_chain::{
    block, compactint::CompactInt, BitcoinDeserialize, BitcoinSerialize, SerializationError,
};
// use tracing::warn;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GetBlocks {
    /// A list of the sender's bets known block hashes, ordered from newest to oldest
    pub block_header_hashes: Vec<block::Hash>,
    /// The stop hash. Set to None if the observed stop hash is [0u8;32],
    /// which signifies that the sender wants as many hashes as we can send (up to 500)
    pub stop_hash: Option<block::Hash>,
}

impl BitcoinDeserialize for GetBlocks {
    fn bitcoin_deserialize<R: std::io::Read>(mut reader: R) -> Result<Self, SerializationError>
    where
        Self: Sized,
    {
        let block_header_hashes = Vec::bitcoin_deserialize(&mut reader)?;
        let raw_stop_hash = block::Hash::bitcoin_deserialize(&mut reader)?;
        let stop_hash = if raw_stop_hash.0 == [0u8; 32] {
            None
        } else {
            Some(raw_stop_hash)
        };
        Ok(GetBlocks {
            block_header_hashes,
            stop_hash,
        })
    }
}

impl BitcoinSerialize for GetBlocks {
    fn bitcoin_serialize<W: std::io::Write>(&self, mut target: W) -> Result<(), std::io::Error> {
        self.block_header_hashes.bitcoin_serialize(&mut target)?;
        match self.stop_hash {
            Some(hash) => hash.bitcoin_serialize(&mut target),
            None => [0u8; 32].bitcoin_serialize(&mut target),
        }
    }
}

impl GetBlocks {
    // pub fn new(block_hashes: Vec<BlockHash>, inv_message: bool, config: &Config) -> GetBlocks {
    //     let message = GetBlocks {
    //         protocol_version: config.get_protocol_version(),
    //         block_header_hashes: block_hashes,
    //         stop_hash: BlockHash::from([0u8; 32]),
    //     };
    //     if !inv_message {
    //         //The header hash of the last header hash being requested; set to all zeroes to request an “inv” message
    //         //with all subsequent header hashes (a maximum of 500 will be sent as a reply to this message;
    //         //if you need more than 500, you will need to send another “getblocks” message with a higher-height
    //         //header hash as the first entry in block header hash field).
    //         match message.block_header_hashes.last() {
    //             Some(_) => {} // message.stop_hash = *hash.clone(),
    //             None => {
    //                 warn!("GetBlocks: stop hash was empty");
    //             }
    //         }
    //     }
    //     message
    // }

    // pub fn new(payload: Payload::GetBlocksPayload,config: &Config) -> GetBlocks {
    //     let mut message = GetBlocks {
    //         protocol_version: config.get_protocol_version(),
    //         hash_count: CompactInt::from(payload.block_hashes.len()),
    //         block_header_hashes: Vec.new(),
    //         stop_hash: Vec.new(),
    //     };
    //     for hash in payload.block_hashes.iter() {
    //         message.block_header_hashes.append(hash)
    //     }
    //     if *inv_message {
    //         message.body.append(u256::new());
    //     } else {
    //         match payload.block_hashes.last() {
    //             Some(hash) => message.body.append(hash),
    //             None => {
    //                 warn!("GetBlocks: stop hash was empty");
    //                 message.body.append(u256::new());
    //             }
    //         }
    //     }
    //     //msg.create_header_for_body(Command::GetBlocks, config.magic());
    //     return message;
    //}
}

impl GetBlocks {
    fn serialized_size(&self) -> usize {
        //protocol version, block header hashes, and stop_hash
        4 + CompactInt::size(self.block_header_hashes.len())
            + (self.block_header_hashes.len() * 32)
            + 32
    }
}

#[test]
fn serial_size() {
    let int1 = block::Hash::from_bytes_exact([0u8; 32]);
    let int2 = block::Hash::from_bytes_exact([1u8; 32]);
    let int3 = block::Hash::from_bytes_exact([3u8; 32]);
    let msg = GetBlocks {
        block_header_hashes: Vec::from([int1, int2, int3]),
        stop_hash: Some(block::Hash::from_bytes_exact([0u8; 32])),
    };
    let serial = msg
        .bitcoin_serialize_to_vec()
        .expect("Serializing into vec shouldn't fail");
    assert_eq!(serial.len(), msg.serialized_size());
    assert_eq!(serial.len(), serial.capacity())
}
