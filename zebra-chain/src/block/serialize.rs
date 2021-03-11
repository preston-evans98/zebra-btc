// use std::{convert::TryInto, io};

// use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
// use chrono::{TimeZone, Utc};

// use crate::{
//     serialization::{
//         BitcoinDeserialize, BitcoinDeserializeInto, BitcoinSerialize, SerializationError,
//     },
//     work::difficulty::CompactDifficulty,
// };

// use super::{merkle, Block, CountedHeader, Hash, Header};

/// The maximum size of a Bitcoin block, in bytes.
///
/// After SegWit, this includes the size of the Witness structure
pub const MAX_BLOCK_BYTES: u64 = 4_000_000;

// impl ZcashSerialize for Header {
//     fn zcash_serialize<W: io::Write>(&self, mut writer: W) -> Result<(), io::Error> {
//         writer.write_u32::<LittleEndian>(self.version)?;
//         self.previous_block_hash.zcash_serialize(&mut writer)?;
//         writer.write_all(&self.merkle_root.0[..])?;
//         writer.write_all(&self.root_bytes[..])?;
//         // this is a truncating cast, rather than a saturating cast
//         // but u32 times are valid until 2106, and our block verification time
//         // checks should detect any truncation.
//         writer.write_u32::<LittleEndian>(self.time.timestamp() as u32)?;
//         writer.write_u32::<LittleEndian>(self.difficulty_threshold.0)?;
//         writer.write_all(&self.nonce[..])?;
//         self.solution.zcash_serialize(&mut writer)?;
//         Ok(())
//     }
// }

// impl BitcoinDeserialize for Header {
//     fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<Self, SerializationError> {

//         let (version, future_version_flag) = {
//             const LOW_31_BITS: u32 = (1 << 31) - 1;
//             let raw_version = reader.read_u32::<LittleEndian>()?;
//             (raw_version & LOW_31_BITS, raw_version >> 31 != 0)
//         };

//         if future_version_flag {
//             return Err(SerializationError::Parse(
//                 "high bit was set in version field",
//             ));
//         }
//         if version < 4 {
//             return Err(SerializationError::Parse("version must be at least 4"));
//         }

//         Ok(Header {
//             version,
//             previous_block_hash: Hash::zcash_deserialize(&mut reader)?,
//             merkle_root: merkle::Root(reader.read_32_bytes()?),
//             root_bytes: reader.read_32_bytes()?,
//             // This can't panic, because all u32 values are valid `Utc.timestamp`s
//             time: Utc.timestamp(reader.read_u32::<LittleEndian>()? as i64, 0),
//             difficulty_threshold: CompactDifficulty(reader.read_u32::<LittleEndian>()?),
//             nonce: reader.read_32_bytes()?,
//             solution: equihash::Solution::zcash_deserialize(reader)?,
//         })
//     }

//     fn bitcoin_deserialize<R: io::Read>(reader: R) -> Result<Self>
//     where
//         Self: Sized {
//         todo!()
//     }
// }

// impl ZcashSerialize for CountedHeader {
//     fn zcash_serialize<W: io::Write>(&self, mut writer: W) -> Result<(), io::Error> {
//         self.header.zcash_serialize(&mut writer)?;
//         writer.write_compactsize(self.transaction_count as u64)?;
//         Ok(())
//     }
// }

// impl ZcashDeserialize for CountedHeader {
//     fn zcash_deserialize<R: io::Read>(mut reader: R) -> Result<Self, SerializationError> {
//         Ok(CountedHeader {
//             header: (&mut reader).zcash_deserialize_into()?,
//             transaction_count: reader.read_compactsize()?.try_into().unwrap(),
//         })
//     }
// }

// impl ZcashSerialize for Block {
//     fn zcash_serialize<W: io::Write>(&self, mut writer: W) -> Result<(), io::Error> {
//         // All block structs are validated when they are parsed.
//         // So we don't need to check MAX_BLOCK_BYTES here, until
//         // we start generating our own blocks (see #483).
//         self.header.zcash_serialize(&mut writer)?;
//         self.transactions.zcash_serialize(&mut writer)?;
//         Ok(())
//     }
// }

// impl ZcashDeserialize for Block {
//     fn zcash_deserialize<R: io::Read>(reader: R) -> Result<Self, SerializationError> {
//         // If the limit is reached, we'll get an UnexpectedEof error
//         let limited_reader = &mut reader.take(MAX_BLOCK_BYTES);
//         Ok(Block {
//             header: limited_reader.zcash_deserialize_into()?,
//             transactions: limited_reader.zcash_deserialize_into()?,
//         })
//     }
// }
