use super::ProtocolVersion;
use zebra_chain::{
    block, compactint::CompactInt, BitcoinDeserialize, BitcoinSerialize, SerializationError,
};
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct GetHeaders {
    /// A list of the sender's bets known block hashes, ordered from newest to oldest
    pub block_header_hashes: Vec<block::Hash>,
    /// The stop hash. Set to None if the observed stop hash is [0u8;32],
    /// which signifies that the sender wants as many hashes as we can send (up to 2000)
    pub stop_hash: Option<block::Hash>,
}

impl BitcoinDeserialize for GetHeaders {
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
        Ok(GetHeaders {
            block_header_hashes,
            stop_hash,
        })
    }
}

impl BitcoinSerialize for GetHeaders {
    fn bitcoin_serialize<W: std::io::Write>(&self, mut target: W) -> Result<(), std::io::Error> {
        self.block_header_hashes.bitcoin_serialize(&mut target)?;
        match self.stop_hash {
            Some(hash) => hash.bitcoin_serialize(&mut target),
            None => [0u8; 32].bitcoin_serialize(&mut target),
        }
    }
}

impl GetHeaders {
    fn serialized_size(&self) -> usize {
        4 + CompactInt::size(self.block_header_hashes.len())
            + (self.block_header_hashes.len() * 32)
            + 32 //protocol version, block header hashes, and stop_hash
    }
}

#[test]
fn serial_size() {
    let int1 = block::Hash::from_bytes_exact([0u8; 32]);
    let int2 = block::Hash::from_bytes_exact([1u8; 32]);
    let int3 = block::Hash::from_bytes_exact([3u8; 32]);
    let msg = GetHeaders {
        block_header_hashes: Vec::from([int1, int2, int3]),
        stop_hash: Some(block::Hash::from_bytes_exact([0u8; 32])),
    };
    let serial = msg
        .bitcoin_serialize_to_vec()
        .expect("Serializing into vec shouldn't fail");
    assert_eq!(serial.len(), msg.serialized_size());
    assert_eq!(serial.len(), serial.capacity())
}
