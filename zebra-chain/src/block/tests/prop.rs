use std::env;
use std::io::ErrorKind;

use proptest::{arbitrary::any, prelude::*, test_runner::Config};
use zebra_test::prelude::*;

use crate::serialization::{BitcoinDeserializeInto, BitcoinSerialize, SerializationError};
use crate::{block, parameters::Network, LedgerState};

use super::super::{serialize::MAX_BLOCK_BYTES, *};

proptest! {
    #[test]
    fn block_hash_roundtrip(hash in any::<Hash>()) {
        zebra_test::init();

        let bytes = hash.bitcoin_serialize_to_vec()?;
        let other_hash: Hash = bytes.bitcoin_deserialize_into()?;

        prop_assert_eq![hash, other_hash];
    }

    #[test]
    fn block_hash_display_fromstr_roundtrip(hash in any::<Hash>()) {
        zebra_test::init();

        let display = format!("{}", hash);
        let parsed = display.parse::<Hash>().expect("hash should parse");
        prop_assert_eq!(hash, parsed);
    }

}
proptest! {
    #![proptest_config(Config::with_cases(1000))]

    #[test]
    fn blockheader_roundtrip(header in any::<Header>()) {
        zebra_test::init();

        let bytes = header.bitcoin_serialize_to_vec().unwrap();
        let other_header = bytes.bitcoin_deserialize_into()?;

        prop_assert_eq![header, other_header];
    }
}

proptest! {
    // The block roundtrip test can be really slow, so we use fewer cases by
    // default. Set the PROPTEST_CASES env var to override this default.
    #![proptest_config(Config::with_cases(env::var("PROPTEST_CASES")
                                          .ok()
                                           .and_then(|v| v.parse().ok())
                                          .unwrap_or(16)))]

    #[test]
    fn block_roundtrip(block in any::<Block>(), _network in any::<Network>()) {
        zebra_test::init();
        let bytes = block.bitcoin_serialize_to_vec()?;
        let bytes = &mut bytes.as_slice();

        // Check the block size limit
        if bytes.len() <= MAX_BLOCK_BYTES as _ {
            // Check deserialization
            let other_block: Block = bytes.bitcoin_deserialize_into()?;

            prop_assert_eq![block, other_block];
        } else {
            let serialization_err = bytes.bitcoin_deserialize_into::<Block>()
                .expect_err("blocks larger than the maximum size should fail");
            match serialization_err {
                SerializationError::Io(io_err) => {
                    prop_assert_eq![io_err.kind(), ErrorKind::UnexpectedEof];
                }
                _ => {
                    prop_assert!(false,
                                 "blocks larger than the maximum size should fail with an io::Error");
                }
            }
        }
    }
}

#[test]
fn blocks_have_coinbase() -> Result<()> {
    zebra_test::init();

    let strategy = any::<block::Height>()
        .prop_map(|tip_height| LedgerState {
            tip_height,
            is_coinbase: true,
            network: Network::Mainnet,
        })
        .prop_flat_map(Block::arbitrary_with);

    proptest!(|(blk in strategy)| {
        // let has_coinbase = block.coinbase_height().is_some();
        let has_coinbase = blk.transactions[0].is_coinbase();
        prop_assert!(has_coinbase);
    });

    Ok(())
}
