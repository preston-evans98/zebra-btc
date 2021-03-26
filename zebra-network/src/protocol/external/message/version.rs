use super::super::types::*;
use super::{Nonce, ProtocolVersion};
use bitcoin_serde_derive::{BtcDeserialize, BtcSerialize};
use chrono::{DateTime, TimeZone, Utc};
use std::net;
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use zebra_chain::{BitcoinDeserialize, BitcoinSerialize, SerializationError};

use zebra_chain::block;

// #[derive(Deserializable, Serializable, Debug, Clone)]
/// A `version` message.
///
/// Note that although this is called `version` in Bitcoin, its role is really
/// analogous to a `ClientHello` message in TLS, used to begin a handshake, and
/// is distinct from a simple version number.
///
/// [Bitcoin reference](https://en.bitcoin.it/wiki/Protocol_documentation#version)
#[derive(Clone, PartialEq, Eq, Debug, BtcSerialize, BtcDeserialize)]
pub struct Version {
    /// The network version number supported by the sender.
    pub version: ProtocolVersion,

    /// The network services advertised by the sender.
    pub services: PeerServices,

    /// The time when the version message was sent.
    pub timestamp: DateTime<Utc>,

    /// The network address of the node receiving this message, and its
    /// advertised network services.
    ///
    /// Q: how does the handshake know the remote peer's services already?
    pub address_recv: (PeerServices, net::SocketAddr),

    /// The network address of the node sending this message, and its
    /// advertised network services.
    pub address_from: (PeerServices, net::SocketAddr),

    /// Node random nonce, randomly generated every time a version
    /// packet is sent. This nonce is used to detect connections
    /// to self.
    pub nonce: Nonce,

    /// The user agent advertised by the sender.
    pub user_agent: String,

    /// The last block received by the emitting node.
    pub best_block: block::Height,

    /// Whether the remote peer should announce relayed
    /// transactions or not, see [BIP 0037](https://github.com/bitcoin/bips/blob/master/bip-0037.mediawiki)
    pub relay: bool,
}

impl Version {
    pub fn new(
        version: ProtocolVersion,
        their_ip: SocketAddr,
        their_services: PeerServices, // Should be PeerServices::NODE_NETWORK when connecting outbound
        our_services: PeerServices,
        our_ip: SocketAddr,
        nonce: Nonce,
        user_agent: String,
        best_block: block::Height,
        relay: bool,
    ) -> Version {
        // To try to stay within the range where bitcoin core will ignore our clock skew,
        // truncate the timestamp to the nearest 5 minutes.
        let now = Utc::now().timestamp();
        let timestamp = Utc.timestamp(now - now.rem_euclid(5 * 60), 0);
        Version {
            version,
            services: our_services,
            timestamp: timestamp,
            address_recv: (their_services, their_ip),
            address_from: (our_services, our_ip),
            nonce,
            user_agent,
            best_block,
            relay,
        }
    }
    // pub fn protocol_version(&self) -> ProtocolVersion {
    //     self.protocol_version
    // }
    // pub fn services(&self) -> Services {
    //     self.services
    // }
    // pub fn timestamp(&self) -> u64 {
    //     self.timestamp
    // }
    // pub fn receiver_services(&self) -> Services {
    //     self.receiver_services
    // }
    // pub fn receiver(&self) -> &SocketAddr {
    //     &self.receiver
    // }
    // pub fn transmitter_services(&self) -> Services {
    //     self.transmitter_services
    // }
    // pub fn transmitter_ip(&self) -> &SocketAddr {
    //     &self.transmitter_ip
    // }
    // pub fn nonce(&self) -> Nonce {
    //     self.nonce
    // }
    // pub fn user_agent(&self) -> &String {
    //     &self.user_agent
    // }
    // pub fn best_block(&self) -> u32 {
    //     self.best_block
    // }
    // pub fn relay(&self) -> bool {
    //     self.relay
    // }
}

// impl super::Payload for Version {
//     fn serialized_size(&self) -> usize {
//         85 + CompactInt::size(self.user_agent.len()) + self.user_agent.len()
//     }
//     fn to_bytes(&self) -> Result<Vec<u8>, std::io::Error> {
//         let mut target = Vec::with_capacity(self.serialized_size());
//         self.serialize(&mut target)?;
//         Ok(target)
//     }
// }

// #[test]
// fn serial_size() {
//     use super::Payload;

//     let msg = Version::new(
//         ([192, 168, 0, 1], 8333).into(),
//         2371,
//         ([192, 168, 0, 2], 8333).into(),
//         0x2329381,
//         &config::Config::mainnet(),
//     );
//     let serial = msg.to_bytes().expect("Serializing into vec shouldn't fail");
//     assert_eq!(serial.len(), msg.serialized_size());
//     assert_eq!(serial.len(), serial.capacity())
// }
