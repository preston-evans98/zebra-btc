use std::sync::Arc;

use crate::cached::Cached;
use crate::work::difficulty::CompactDifficulty;

use super::*;

use crate::LedgerState;
use chrono::{TimeZone, Utc};
use proptest::{
    arbitrary::{any, Arbitrary},
    prelude::*,
};

impl Arbitrary for Block {
    type Parameters = LedgerState;

    fn arbitrary_with(ledger_state: Self::Parameters) -> Self::Strategy {
        let transactions_strategy = Transaction::vec_strategy(ledger_state, 2);

        (any::<Header>(), transactions_strategy)
            .prop_map(|(header, transactions)| Self {
                header,
                transactions,
            })
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl Block {
    /// Returns a strategy for creating Vecs of blocks with increasing height of
    /// the given length.
    pub fn partial_chain_strategy(
        init: LedgerState,
        count: usize,
    ) -> BoxedStrategy<Vec<Arc<Self>>> {
        let mut current = init;
        let mut vec = Vec::with_capacity(count);
        for _ in 0..count {
            vec.push(Block::arbitrary_with(current).prop_map(Arc::new));
            current.tip_height.0 += 1;
        }

        vec.boxed()
    }
}

impl Arbitrary for Header {
    type Parameters = ();

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        (
            // version is interpreted as i32 in the spec, so we are limited to i32::MAX here
            (4u32..(i32::MAX as u32)),
            any::<Hash>(),
            any::<merkle::Root>(),
            // time is interpreted as u32 in the spec, but rust timestamps are i64
            (0i64..(u32::MAX as i64)),
            any::<CompactDifficulty>(),
            (0u32..(u32::MAX)),
        )
            .prop_map(
                |(
                    version,
                    previous_block_hash,
                    merkle_root,
                    timestamp,
                    difficulty_threshold,
                    nonce,
                )| {
                    Header::new(
                        version,
                        previous_block_hash,
                        merkle_root,
                        Utc.timestamp(timestamp, 0),
                        difficulty_threshold,
                        nonce,
                    )
                },
            )
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}
