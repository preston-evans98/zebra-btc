use super::CompactInt;
use byteorder::{LittleEndian, ReadBytesExt};
use chrono::{DateTime, TimeZone, Utc};
use std::io;
use std::net::SocketAddr;

use super::SerializationError;

type Result<R> = std::result::Result<R, SerializationError>;

pub trait BitcoinDeserialize {
    fn bitcoin_deserialize<R: io::Read>(reader: R) -> Result<Self>
    where
        Self: Sized;
}

/// Helper for deserializing more succinctly via type inference
pub trait BitcoinDeserializeInto {
    /// Deserialize based on type inference
    fn bitcoin_deserialize_into<T>(self) -> Result<T>
    where
        T: BitcoinDeserialize;
}

impl<R: io::Read> BitcoinDeserializeInto for R {
    fn bitcoin_deserialize_into<T>(self) -> Result<T>
    where
        T: BitcoinDeserialize,
    {
        T::bitcoin_deserialize(self)
    }
}

impl BitcoinDeserialize for bool {
    fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<bool> {
        let value = reader.read_u8()?;
        match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(SerializationError::Parse("Invalid bool")),
        }
    }
}
impl BitcoinDeserialize for u8 {
    fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<u8> {
        Ok(reader.read_u8()?)
    }
}

impl BitcoinDeserialize for u16 {
    fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<u16> {
        Ok(reader.read_u16::<LittleEndian>()?)
    }
}

impl BitcoinDeserialize for u32 {
    fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<u32> {
        Ok(reader.read_u32::<LittleEndian>()?)
    }
}

impl BitcoinDeserialize for u64 {
    fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<u64> {
        Ok(reader.read_u64::<LittleEndian>()?)
    }
}

impl BitcoinDeserialize for i32 {
    fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<i32> {
        Ok(reader.read_i32::<LittleEndian>()?)
    }
}

impl BitcoinDeserialize for i64 {
    fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<i64> {
        Ok(reader.read_i64::<LittleEndian>()?)
    }
}

impl BitcoinDeserialize for DateTime<Utc> {
    fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<DateTime<Utc>> {
        Ok(Utc.timestamp(reader.read_u32::<LittleEndian>()? as i64, 0))
    }
}

impl<T> BitcoinDeserialize for Vec<T>
where
    T: BitcoinDeserialize,
{
    fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<Vec<T>> {
        let len = CompactInt::bitcoin_deserialize(&mut reader)?.value() as usize;
        // Limit preallocation to about 1000 items since blind preallocation is a DOS vulnerability
        // TODO: Replace with SafeAllocate when specialization stabilizes
        let blind_alloc_limit = 1024;
        let mut result: Vec<T> = Vec::with_capacity(std::cmp::min(len, blind_alloc_limit));
        for _ in 0..len {
            result.push(T::bitcoin_deserialize(&mut reader)?);
        }
        Ok(result)
    }
}

impl BitcoinDeserialize for String {
    fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<String> {
        let len = CompactInt::bitcoin_deserialize(&mut reader)?.value() as usize;
        // Limit preallocation to about 1000 items since blind preallocation is a DOS vulnerability
        // TODO: Replace with SafeAllocate when specialization stabilizes
        let blind_alloc_limit = 1024;
        let mut buf = vec![0; std::cmp::min(len, blind_alloc_limit)];
        reader.read_exact(&mut buf)?;
        String::from_utf8(buf).map_err(|_| SerializationError::Parse("invalid utf-8"))
    }
}

// TODO: test
impl BitcoinDeserialize for SocketAddr {
    fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<SocketAddr> {
        Ok(SocketAddr::from((
            <[u8; 16]>::bitcoin_deserialize(&mut reader)?,
            reader.read_u16::<LittleEndian>()?,
            // u16::read(&mut reader)?,
        )))
    }
}

impl<T: Sized + BitcoinDeserialize> BitcoinDeserialize for Option<T> {
    fn bitcoin_deserialize<R: io::Read>(reader: R) -> Result<Option<T>> {
        Ok(Some(T::bitcoin_deserialize(reader)?))
    }
}

// TODO: Replace when const generics stabilize
macro_rules! impl_deserializable_byte_array {
    ($size:expr) => {
        impl BitcoinDeserialize for [u8; $size] {
            fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<[u8; $size]> {
                let mut result = [0u8; $size];
                reader.read_exact(&mut result);
                Ok(result)
            }
        }
    };
}

impl_deserializable_byte_array!(4);
impl_deserializable_byte_array!(16);
impl_deserializable_byte_array!(32);
