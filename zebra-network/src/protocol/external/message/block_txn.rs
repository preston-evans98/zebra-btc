use bitcoin_serde_derive::{BtcDeserialize, BtcSerialize};
use zebra_chain::{
    block, compactint::CompactInt, transaction::Transaction, BitcoinDeserialize, BitcoinSerialize,
    SerializationError,
};
#[derive(Debug, Clone, PartialEq, Eq, BtcDeserialize, BtcSerialize)]
pub struct BlockTxn {
    pub block_hash: block::Hash,
    pub txs: Vec<Transaction>,
}

impl BlockTxn {
    pub fn serialized_size(&self) -> usize {
        let mut size = 32;
        size += CompactInt::size(self.txs.len());
        for transaction in self.txs.iter() {
            size += transaction.len();
        }
        size
    }
}

#[cfg(test)]
mod tests {
    use super::BlockTxn;
    use zebra_chain::{block, BitcoinSerialize};

    #[test]
    fn serial_size_empty() {
        let txs = Vec::with_capacity(2);
        let msg = BlockTxn {
            block_hash: block::Hash::from_bytes_exact([1u8; 32]),
            txs,
        };
        let serial = msg
            .bitcoin_serialize_to_vec()
            .expect("Serializing into vec shouldn't fail");
        assert_eq!(serial.len(), msg.serialized_size());
        assert_eq!(serial.len(), serial.capacity())
    }
    // FIXME: Set up with proptest
    // #[test]
    // fn serial_size_full() {
    //     let msg = BlockTxn {
    //         block_hash: block::Hash::from_bytes_exact([1u8; 32]),
    //         txs: Transaction::_test_txs(),
    //     };
    //     let serial = msg
    //         .bitcoin_serialize_to_vec()
    //         .expect("Serializing into vec shouldn't fail");
    //     assert_eq!(serial.len(), msg.serialized_size());
    //     assert_eq!(serial.len(), serial.capacity())
    // }
}
