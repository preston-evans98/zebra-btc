use bitcoin_serde_derive::{BtcDeserialize, BtcSerialize};
use zebra_chain::{
    block, compactint::CompactInt, BitcoinDeserialize, BitcoinSerialize, SerializationError,
};

#[derive(BtcSerialize, BtcDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct GetBlockTxn {
    pub block_hash: block::Hash,
    pub indexes: Vec<CompactInt>,
}

impl GetBlockTxn {
    fn serialized_size(&self) -> usize {
        let mut len = 32 + CompactInt::size(self.indexes.len());
        for index in self.indexes.iter() {
            len += CompactInt::size(index.value() as usize);
        }
        len
    }
}

#[test]
fn serial_size() {
    let int1 = CompactInt::from(567892322);
    let int2 = CompactInt::from(7892322);
    let int3 = CompactInt::from(0);
    let msg = GetBlockTxn {
        block_hash: block::Hash::from_bytes_exact([242u8; 32]),
        indexes: Vec::from([int1, int2, int3]),
    };
    let serial = msg
        .bitcoin_serialize_to_vec()
        .expect("Serializing into vec shouldn't fail");
    assert_eq!(serial.len(), msg.serialized_size());
    assert_eq!(serial.len(), serial.capacity())
}
