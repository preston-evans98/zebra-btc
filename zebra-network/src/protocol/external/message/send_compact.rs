use bitcoin_serde_derive::{BtcDeserialize, BtcSerialize};
use zebra_chain::{BitcoinDeserialize, BitcoinSerialize, SerializationError};

#[derive(Debug, Clone, PartialEq, Eq, BtcDeserialize, BtcSerialize)]
pub struct SendCompact {
    pub announce: bool,
    pub version: u64,
}

impl SendCompact {
    pub const fn serialized_size(&self) -> usize {
        9
    }
}

#[test]
fn serial_size() {
    let msg = SendCompact {
        announce: true,
        version: 32381,
    };
    let serial = msg
        .bitcoin_serialize_to_vec()
        .expect("Serializing into vec shouldn't fail");
    assert_eq!(serial.len(), msg.serialized_size());
    assert_eq!(serial.len(), serial.capacity())
}
