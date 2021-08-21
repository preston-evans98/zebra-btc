//! Transparent Address types.

use std::{fmt, io};

use ripemd160::{Digest, Ripemd160};
use secp256k1::PublicKey;
use sha2::{Digest as sha2Digest, Sha256};

#[cfg(test)]
use proptest::{arbitrary::Arbitrary, collection::vec, prelude::*};

use crate::{
    parameters::Network,
    serialization::{BitcoinDeserialize, BitcoinSerialize, SerializationError},
};

use super::Script;

/// Magic numbers used to identify what networks the Addresses
/// are associated with.
mod magics {
    pub mod p2sh {
        pub const MAINNET: [u8; 1] = [0x05];
        pub const TESTNET: [u8; 1] = [0xc4];
    }

    pub mod p2pkh {
        pub const MAINNET: [u8; 1] = [0x00];
        pub const TESTNET: [u8; 1] = [0x6f];
    }
}

/// Bitcoin Addresses
///
/// In Bitcoin a single byte is preprended to the hash to specify
/// the address type. The result is then hashed with sha256d, the first four bytes
/// of the output are appended as a checksum, and the result is Base58Check encoded
///
/// https://en.bitcoin.it/wiki/Base58Check_encoding
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Address {
    /// P2SH (Pay to Script Hash) addresses
    PayToScriptHash {
        /// Production, test, or other network
        network: Network,
        /// 20 bytes specifying a script hash.
        script_hash: [u8; 20],
    },
    /// P2PKH (Pay to Public Key Hash) addresses
    PayToPublicKeyHash {
        /// Production, test, or other network
        network: Network,
        /// 20 bytes specifying a public key hash, which is a RIPEMD-160
        /// hash of a SHA-256 hash of a compressed ECDSA key encoding.
        pub_key_hash: [u8; 20],
    },
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut debug_struct = f.debug_struct("TransparentAddress");

        match self {
            Address::PayToScriptHash {
                network,
                script_hash,
            } => debug_struct
                .field("network", network)
                .field("script_hash", &hex::encode(script_hash))
                .finish(),
            Address::PayToPublicKeyHash {
                network,
                pub_key_hash,
            } => debug_struct
                .field("network", network)
                .field("pub_key_hash", &hex::encode(pub_key_hash))
                .finish(),
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut bytes = io::Cursor::new(Vec::new());
        let _ = self.bitcoin_serialize(&mut bytes);

        f.write_str(&bs58::encode(bytes.get_ref()).with_check().into_string())
    }
}

impl std::str::FromStr for Address {
    type Err = SerializationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let result = &bs58::decode(s).with_check(None).into_vec();

        match result {
            Ok(bytes) => Self::bitcoin_deserialize(&bytes[..]),
            Err(_) => Err(SerializationError::Parse("t-addr decoding error")),
        }
    }
}

impl BitcoinSerialize for Address {
    fn bitcoin_serialize<W: io::Write>(&self, mut writer: W) -> Result<(), io::Error> {
        match self {
            Address::PayToScriptHash {
                network,
                script_hash,
            } => {
                // Dev network doesn't have a recommendation so we
                // default to testnet bytes if it's not mainnet.
                match *network {
                    Network::Mainnet => writer.write_all(&magics::p2sh::MAINNET[..])?,
                    _ => writer.write_all(&magics::p2sh::TESTNET[..])?,
                }
                writer.write_all(script_hash)?
            }
            Address::PayToPublicKeyHash {
                network,
                pub_key_hash,
            } => {
                // Dev network doesn't have a recommendation so we
                // default to testnet bytes if it's not mainnet.
                match *network {
                    Network::Mainnet => writer.write_all(&magics::p2pkh::MAINNET[..])?,
                    _ => writer.write_all(&magics::p2pkh::TESTNET[..])?,
                }
                writer.write_all(pub_key_hash)?
            }
        }

        Ok(())
    }
}

impl BitcoinDeserialize for Address {
    fn bitcoin_deserialize<R: io::Read>(mut reader: R) -> Result<Self, SerializationError> {
        let mut version_bytes = [0; 1];
        reader.read_exact(&mut version_bytes)?;

        let mut hash_bytes = [0; 20];
        reader.read_exact(&mut hash_bytes)?;

        match version_bytes {
            magics::p2sh::MAINNET => Ok(Address::PayToScriptHash {
                network: Network::Mainnet,
                script_hash: hash_bytes,
            }),
            magics::p2sh::TESTNET => Ok(Address::PayToScriptHash {
                network: Network::Testnet,
                script_hash: hash_bytes,
            }),
            magics::p2pkh::MAINNET => Ok(Address::PayToPublicKeyHash {
                network: Network::Mainnet,
                pub_key_hash: hash_bytes,
            }),
            magics::p2pkh::TESTNET => Ok(Address::PayToPublicKeyHash {
                network: Network::Testnet,
                pub_key_hash: hash_bytes,
            }),
            _ => Err(SerializationError::Parse("bad addr version/type")),
        }
    }
}

trait ToAddressWithNetwork {
    /// Convert `self` to an `Address`, given the current `network`.
    fn to_address(&self, network: Network) -> Address;
    fn to_address_uncompressed(&self, network: Network) -> Address;
}

impl ToAddressWithNetwork for Script {
    fn to_address(&self, network: Network) -> Address {
        Address::PayToScriptHash {
            network,
            script_hash: Address::hash_payload(&self.0[..]),
        }
    }
    fn to_address_uncompressed(&self, network: Network) -> Address {
        Address::PayToScriptHash {
            network,
            script_hash: Address::hash_payload(&self.0[..]),
        }
    }
}

impl ToAddressWithNetwork for PublicKey {
    fn to_address(&self, network: Network) -> Address {
        Address::PayToPublicKeyHash {
            network,
            pub_key_hash: Address::hash_payload(&self.serialize()[..]),
        }
    }

    fn to_address_uncompressed(&self, network: Network) -> Address {
        Address::PayToPublicKeyHash {
            network,
            pub_key_hash: Address::hash_payload(&self.serialize_uncompressed()[..]),
        }
    }
}

impl Address {
    /// A hash of a transparent address payload, as used in
    /// transparent pay-to-script-hash and pay-to-publickey-hash
    /// addresses.
    ///
    /// The resulting hash in both of these cases is always exactly 20
    /// bytes.
    /// https://en.bitcoin.it/Base58Check_encoding#Encoding_a_Bitcoin_address
    fn hash_payload(bytes: &[u8]) -> [u8; 20] {
        let sha_hash = Sha256::digest(bytes);
        let ripe_hash = Ripemd160::digest(&sha_hash);
        let mut payload = [0u8; 20];
        payload[..].copy_from_slice(&ripe_hash[..]);
        payload
    }
}

#[cfg(test)]
impl Address {
    fn p2pkh_strategy() -> impl Strategy<Value = Self> {
        (any::<Network>(), vec(any::<u8>(), 20))
            .prop_map(|(network, payload_bytes)| {
                let mut bytes = [0; 20];
                bytes.copy_from_slice(payload_bytes.as_slice());
                Self::PayToPublicKeyHash {
                    network,
                    pub_key_hash: bytes,
                }
            })
            .boxed()
    }

    fn p2sh_strategy() -> impl Strategy<Value = Self> {
        (any::<Network>(), vec(any::<u8>(), 20))
            .prop_map(|(network, payload_bytes)| {
                let mut bytes = [0; 20];
                bytes.copy_from_slice(payload_bytes.as_slice());
                Self::PayToScriptHash {
                    network,
                    script_hash: bytes,
                }
            })
            .boxed()
    }
}

#[cfg(test)]
impl Arbitrary for Address {
    type Parameters = ();

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        prop_oneof![Self::p2pkh_strategy(), Self::p2sh_strategy(),].boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

#[cfg(test)]
mod tests {

    use secp256k1::PublicKey;

    use super::*;

    #[test]
    fn pubkey_mainnet_uncompressed() {
        zebra_test::init();

        let pub_key = PublicKey::from_slice(&[
            4, 80, 134, 58, 214, 74, 135, 174, 138, 47, 232, 60, 26, 241, 168, 64, 60, 181, 63, 83,
            228, 134, 216, 81, 29, 173, 138, 4, 136, 126, 91, 35, 82, 44, 212, 112, 36, 52, 83,
            162, 153, 250, 158, 119, 35, 119, 22, 16, 58, 188, 17, 161, 223, 56, 133, 94, 214, 242,
            238, 24, 126, 156, 88, 43, 166,
        ])
        .expect("A PublicKey from slice");

        let t_addr = pub_key.to_address_uncompressed(Network::Mainnet);

        assert_eq!(format!("{}", t_addr), "16UwLL9Risc3QfPqBUvKofHmBQ7wMtjvM");
    }

    #[test]
    fn pubkey_testnet_uncompressed() {
        zebra_test::init();

        let pub_key = PublicKey::from_slice(&[
            4, 45, 115, 49, 52, 94, 13, 166, 171, 17, 37, 235, 57, 72, 138, 84, 42, 153, 35, 243,
            28, 88, 92, 32, 17, 77, 33, 26, 159, 107, 201, 243, 191, 85, 209, 216, 67, 203, 124,
            241, 211, 107, 50, 209, 203, 0, 210, 241, 64, 239, 2, 142, 114, 106, 25, 167, 102, 246,
            202, 124, 239, 123, 149, 101, 131,
        ])
        .expect("A PublicKey from slice");

        let t_addr = pub_key.to_address_uncompressed(Network::Testnet);

        assert_eq!(format!("{}", t_addr), "myfp2YcyYjksxmdfA74yEuBmaUgt9xWCot");
    }

    #[test]
    fn empty_script_mainnet() {
        zebra_test::init();

        let script = Script(vec![0; 20]);

        let t_addr = script.to_address(Network::Mainnet);

        assert_eq!(format!("{}", t_addr), "3FDDHcFYKGeWDagq4rXnDSrmzbXjDhP97D");
    }

    #[test]
    fn empty_script_testnet() {
        zebra_test::init();

        let script = Script(vec![0; 20]);

        let t_addr = script.to_address(Network::Testnet);

        assert_eq!(format!("{}", t_addr), "2N6mRMMBZvj9rRNKNjz9eqPr3CwjtzggLTT");
    }

    #[test]
    fn from_string() {
        zebra_test::init();

        let t_addr: Address = "3Q7achm1qfMPzMiKQYafAPcPhn3hvcBaRL".parse().unwrap();

        assert_eq!(format!("{}", t_addr), "3Q7achm1qfMPzMiKQYafAPcPhn3hvcBaRL");
    }

    #[test]
    fn debug() {
        zebra_test::init();

        let addr: Address = "3Q7achm1qfMPzMiKQYafAPcPhn3hvcBaRL".parse().unwrap();

        assert_eq!(
            format!("{:?}", addr),
            "TransparentAddress { network: Mainnet, script_hash: \"f5f80a632ec39691cd1a9a268ff854c210773bfd\" }"
        );
    }
}

#[cfg(test)]
proptest! {

    #[test]
    fn transparent_address_roundtrip(taddr in any::<Address>()) {
        zebra_test::init();

        let mut data = Vec::new();

        taddr.bitcoin_serialize(&mut data).expect("t-addr should serialize");

        let taddr2 = Address::bitcoin_deserialize(&data[..]).expect("randomized t-addr should deserialize");

        prop_assert_eq![taddr, taddr2];
    }
}
