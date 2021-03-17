//! Consensus-critical serialization.
//!
//! This module contains four traits: `ZcashSerialize` and `ZcashDebitcoin_serialize`,
//! analogs of the Serde `Serialize` and `Debitcoin_serialize` traits but intended for
//! consensus-critical Zcash serialization formats, and `WriteZcashExt` and
//! `ReadZcashExt`, extension traits for `io::Read` and `io::Write` with utility functions
//! for reading and writing data (e.g., the Bitcoin variable-integer format).

mod deserialize;
use std::convert::TryFrom;
mod error;
// mod read_zcash;
// mod write_zcash;
// mod zcash_debitcoin_serialize;
// mod zcash_bitcoin_serialize;

// pub(crate) mod serde_helpers;

pub mod sha256d;

use chrono::{DateTime, Utc};
pub use deserialize::{BitcoinDeserialize, BitcoinDeserializeInto};
pub use error::SerializationError;
// pub use read_zcash::ReadZcashExt;
// pub use write_zcash::WriteZcashExt;
// pub use zcash_debitcoin_serialize::{ZcashDebitcoin_serialize, ZcashDebitcoin_serializeInto};
// pub use zcash_bitcoin_serialize::ZcashSerialize;

// #[cfg(test)]
// mod proptests;

use crate::compactint::CompactInt;
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use std::net::IpAddr;

pub trait BitcoinSerialize {
    // fn bitcoin_serialize(&self, target: &mut Vec<u8>);
    fn bitcoin_serialize<W: std::io::Write>(&self, target: W) -> Result<(), std::io::Error>;

    fn bitcoin_serialize_to_vec(&self) -> Result<Vec<u8>, std::io::Error> {
        let mut data = Vec::new();
        self.bitcoin_serialize(&mut data)?;
        Ok(data)
    }
}
impl BitcoinSerialize for bool {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        target.write_all(&[*self as u8])
    }
}

impl BitcoinSerialize for &u8 {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        target.write_all(&[*(*self) as u8])
    }
}

impl BitcoinSerialize for &char {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        target.write_all(&[*(*self) as u8])
    }
}

impl BitcoinSerialize for u16 {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        target.write_u16::<LittleEndian>(*self)
    }
}

impl BitcoinSerialize for u32 {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        target.write_u32::<LittleEndian>(*self)
    }
}
impl BitcoinSerialize for u64 {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        target.write_u64::<LittleEndian>(*self)
    }
}
impl BitcoinSerialize for i32 {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        target.write_i32::<LittleEndian>(*self)
    }
}
impl BitcoinSerialize for i64 {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        target.write_i64::<LittleEndian>(*self)
    }
}

impl BitcoinSerialize for DateTime<Utc> {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        let time_u32 = match u32::try_from(self.timestamp()) {
            Ok(t) => t,
            Err(e) => std::u32::MAX,
        };
        target.write_u32::<LittleEndian>(time_u32)
    }
}

// TODO: Uncomment when specialization stabilizes
// impl BitcoinSerialize for u8 {
//     fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
//     where
//         W: std::io::Write,
//     {
//         target.write_all(&[*self])
//     }
// }

impl BitcoinSerialize for std::net::Ipv6Addr {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        target.write_all(&self.octets())
    }
}

impl BitcoinSerialize for std::net::IpAddr {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        match self {
            IpAddr::V4(addr) => addr.to_ipv6_mapped().bitcoin_serialize(&mut target),
            IpAddr::V6(addr) => addr.bitcoin_serialize(&mut target),
        }
    }
}

impl BitcoinSerialize for &std::net::SocketAddr {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        self.ip().bitcoin_serialize(&mut target)?;
        target.write_u16::<BigEndian>(self.port())
    }
}

impl BitcoinSerialize for std::net::SocketAddr {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        self.ip().bitcoin_serialize(&mut target)?;
        target.write_u16::<BigEndian>(self.port())
    }
}

impl BitcoinSerialize for &[u8] {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        target.write_all(self)
    }
}

impl BitcoinSerialize for [u8; 4] {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        target.write_all(self)
    }
}

impl BitcoinSerialize for [u8; 12] {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        target.write_all(self)
    }
}

impl BitcoinSerialize for [u8; 32] {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        target.write_all(self)
    }
}

impl BitcoinSerialize for Vec<u8> {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        CompactInt::from(self.len()).bitcoin_serialize(&mut target)?;
        target.write_all(self)?;
        Ok(())
    }
}

impl<T> BitcoinSerialize for Vec<T>
where
    T: BitcoinSerialize,
{
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        CompactInt::from(self.len()).bitcoin_serialize(&mut target)?;
        for item in self.iter() {
            item.bitcoin_serialize(&mut target)?
        }
        Ok(())
    }
}

impl<T, U> BitcoinSerialize for (T, U)
where
    T: BitcoinSerialize,
    U: BitcoinSerialize,
{
    fn bitcoin_serialize<W>(&self, mut target: W) -> std::result::Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        self.0.bitcoin_serialize(&mut target)?;
        self.1.bitcoin_serialize(&mut target)?;
        Ok(())
    }
}
impl BitcoinSerialize for String {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        CompactInt::from(self.len()).bitcoin_serialize(&mut target)?;
        self.as_bytes().bitcoin_serialize(&mut target)?;
        Ok(())
    }
}

impl<T: BitcoinSerialize> BitcoinSerialize for Option<T> {
    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        match self {
            Some(contents) => return contents.bitcoin_serialize(&mut target),
            None => Ok(()),
        }
    }
}
