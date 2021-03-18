use std::convert::TryInto;

// use super::PrefilledTransaction;
// use bytes::Buf;
// use serde_derive::{Deserializable, Serializable};
// use shared::BlockHeader;
// use shared::CompactInt;
// use shared::Serializable;
use bitcoin_serde_derive::{BtcDeserialize, BtcSerialize};
use zebra_chain::{
    block, compactint::CompactInt, transaction::Transaction, BitcoinDeserialize, BitcoinSerialize,
    SerializationError,
};

#[derive(BtcSerialize, BtcDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct PrefilledTransaction {
    pub index: CompactInt,
    pub tx: Transaction,
}

impl PrefilledTransaction {
    /// Returns the serialized length of a PrefilledTx
    pub fn len(&self) -> usize {
        self.tx.len() + CompactInt::size(self.index.value().try_into().unwrap())
    }
}
#[derive(BtcSerialize, BtcDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct CompactBlock {
    pub header: block::Header,
    pub nonce: u64,
    pub short_ids: Vec<u64>,
    pub prefilled_txns: Vec<PrefilledTransaction>,
}

impl CompactBlock {
    fn serialized_size(&self) -> usize {
        let mut len = block::Header::len()
            + 8
            + CompactInt::size(self.short_ids.len())
            + 8 * self.short_ids.len()
            + CompactInt::size(self.prefilled_txns.len());
        for txn in self.prefilled_txns.iter() {
            len += txn.len();
        }
        len
    }
}

// FIXME: swap to proptest

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn serial_size() {
//         let txs = PrefilledTransaction::_test_txs();
//         let header = BlockHeader::_test_header();

//         let msg = super::CompactBlock {
//             header,
//             nonce: 1928712,
//             short_ids: Vec::from([8219u64; 7]),
//             prefilled_txns: txs,
//         };
//         let serial = msg.to_bytes().expect("Serializing into vec shouldn't fail");
//         assert_eq!(serial.len(), msg.serialized_size());
//         assert_eq!(serial.len(), serial.capacity())
//     }
// }
