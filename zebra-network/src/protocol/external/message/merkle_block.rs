use bitcoin_serde_derive::{BtcDeserialize, BtcSerialize};
use zebra_chain::{
    block, compactint::CompactInt, transaction, BitcoinDeserialize, BitcoinSerialize,
    SerializationError,
};

#[derive(BtcDeserialize, BtcSerialize, Debug, Clone, PartialEq, Eq)]
pub struct MerkleBlock {
    pub block_header: block::Header,
    pub transaction_count: u32,
    pub hashes: Vec<transaction::Hash>,
    pub flags: Vec<u8>,
}
impl MerkleBlock {
    fn serialized_size(&self) -> usize {
        block::Header::len()
            + 4
            + CompactInt::size(self.hashes.len())
            + (self.hashes.len() * 32)
            + CompactInt::size(self.flags.len())
            + self.flags.len()
    }
}

// #[test]
// FIXME: make Zebra compatible
// fn serial_size() {
//     use super::Payload;
//     let int1 = transaction::Hash::from_bytes_exact([0u8; 32]);
//     let int2 = transaction::Hash::from_bytes_exact([1u8; 32]);
//     let int3 = transaction::Hash::from_bytes_exact([3u8; 32]);
//     let block_header = block::Header::_test_header();

//     let msg = MerkleBlock {
//         block_header,
//         transaction_count: 113,
//         hashes: vec![int1, int2, int3],
//         flags: Vec::from([232u8, 11]),
//     };
//     let serial = msg.to_bytes().expect("Serializing into vec shouldn't fail");
//     assert_eq!(serial.len(), msg.serialized_size());
//     assert_eq!(serial.len(), serial.capacity())
// }
