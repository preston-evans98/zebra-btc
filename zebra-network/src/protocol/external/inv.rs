//! Inventory items for the Bitcoin protocol.

// XXX the exact optimal arrangement of all of these parts is a little unclear
// until we have more pieces in place the optimal global arrangement of items is
// a little unclear.

use std::io::{Read, Write};

use zebra_chain::{
    block,
    serialization::{BitcoinDeserialize, BitcoinSerialize, SerializationError},
    transaction,
};

/// An inventory hash which refers to some advertised or requested data.
///
/// Bitcoin calls this an "inventory vector" but it is just a typed hash, not a
/// container, so we do not use that term to avoid confusion with `Vec<T>`.
///
/// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#Inventory_Vectors)
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum InventoryHash {
    /// An error.
    ///
    /// The Bitcoin wiki just says "Any data of with this number may be ignored",
    /// so we don't include a typed hash.
    Error,
    /// A hash of a transaction.
    Tx(transaction::Hash),
    /// A hash of a block.
    Block(block::Hash),
    /// A hash of a filtered block.
    ///
    /// The Bitcoin wiki says: Hash of a block header, but only to be used in
    /// getdata message. Indicates the reply should be a merkleblock message
    /// rather than a block message; this only works if a bloom filter has been
    /// set.
    FilteredBlock(block::Hash),
}

impl From<transaction::Hash> for InventoryHash {
    fn from(tx: transaction::Hash) -> InventoryHash {
        InventoryHash::Tx(tx)
    }
}

impl From<block::Hash> for InventoryHash {
    fn from(hash: block::Hash) -> InventoryHash {
        // Auto-convert to Block rather than FilteredBlock because filtered
        // blocks aren't useful for Zcash.
        InventoryHash::Block(hash)
    }
}

impl BitcoinSerialize for InventoryHash {
    fn bitcoin_serialize<W: Write>(&self, mut writer: W) -> Result<(), std::io::Error> {
        let (code, bytes) = match *self {
            InventoryHash::Error => (0, [0; 32]),
            InventoryHash::Tx(hash) => (1, hash.0),
            InventoryHash::Block(hash) => (2, hash.0),
            InventoryHash::FilteredBlock(hash) => (3, hash.0),
        };
        code.bitcoin_serialize(&mut writer)?;
        bytes.bitcoin_serialize(&mut writer)?;
        Ok(())
    }
}

// TODO: Implement segwit
impl BitcoinDeserialize for InventoryHash {
    fn bitcoin_deserialize<R: Read>(mut reader: R) -> Result<Self, SerializationError> {
        // let code = reader.read_u32::<LittleEndian>()?;
        let code = u32::bitcoin_deserialize(&mut reader)?;
        let bytes = <[u8; 32]>::bitcoin_deserialize(&mut reader)?;
        match code {
            0 => Ok(InventoryHash::Error),
            1 => Ok(InventoryHash::Tx(transaction::Hash(bytes))),
            2 => Ok(InventoryHash::Block(block::Hash(bytes))),
            3 => Ok(InventoryHash::FilteredBlock(block::Hash(bytes))),
            // (0x01000000 as u32 & 1) => Ok(InventoryHash::WitnessTx(transaction::Hash(bytes))),
            // 0x01000000 as u32 & 2 => Ok(InventoryHash::WitnessBlock(block::Hash(bytes))),
            // 0x01000000 as u32 & 3 => Ok(InventoryHash::WitnessFilteredBlock(block::Hash(bytes))),
            _ => Err(SerializationError::Parse("invalid inventory code")),
        }
    }
}
