use std::sync::Arc;

use block::Height;
use chrono::{TimeZone, Utc};
use proptest::{arbitrary::any, collection::vec, prelude::*};

use crate::{block, parameters::NetworkUpgrade, serialization::SmallUnixTime, transparent};
use crate::{cached::Cached, LedgerState};

use super::{LockTime, Memo, Transaction};

impl Transaction {
    /// Generate a proptest strategy for V1 Transactions
    pub fn v1_strategy(ledger_state: LedgerState) -> BoxedStrategy<Self> {
        (
            transparent::Input::vec_strategy(ledger_state, 10),
            vec(any::<transparent::Output>(), 0..10),
            any::<LockTime>(),
        )
            .prop_map(|(inputs, outputs, locktime)| Transaction {
                version: 1,
                inputs,
                outputs,
                locktime,
                hash: Cached::new(),
            })
            .boxed()
    }

    // /// Generate a proptest strategy for V2 Transactions
    // pub fn v2_strategy(ledger_state: LedgerState) -> BoxedStrategy<Self> {
    //     (
    //         transparent::Input::vec_strategy(ledger_state, 10),
    //         vec(any::<transparent::Output>(), 0..10),
    //         any::<LockTime>(),
    //     )
    //         .prop_map(|(inputs, outputs, lock_time)| Transaction::V2 {
    //             inputs,
    //             outputs,
    //             lock_time,
    //             hash: Cached::new(),
    //         })
    //         .boxed()
    // }

    /// Proptest Strategy for creating a Vector of transactions where the first
    /// transaction is always the only coinbase transaction
    pub fn vec_strategy(
        mut ledger_state: LedgerState,
        len: usize,
    ) -> BoxedStrategy<Vec<Arc<Self>>> {
        let coinbase = Transaction::arbitrary_with(ledger_state).prop_map(Arc::new);
        ledger_state.is_coinbase = false;
        let remainder = vec(
            Transaction::arbitrary_with(ledger_state).prop_map(Arc::new),
            len,
        );

        (coinbase, remainder)
            .prop_map(|(first, mut remainder)| {
                remainder.insert(0, first);
                remainder
            })
            .boxed()
    }
}

impl Arbitrary for Memo {
    type Parameters = ();

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (vec(any::<u8>(), 512))
            .prop_map(|v| {
                let mut bytes = [0; 512];
                bytes.copy_from_slice(v.as_slice());
                Memo(Box::new(bytes))
            })
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl Arbitrary for LockTime {
    type Parameters = ();

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        prop_oneof![
            (block::Height::MIN.0..=block::Height::MAX.0)
                .prop_map(|n| LockTime::Height(block::Height(n))),
            (LockTime::MIN_TIMESTAMP..=LockTime::MAX_TIMESTAMP)
                .prop_map(|n| { LockTime::Time(SmallUnixTime(Utc.timestamp(n as i64, 0))) })
        ]
        .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl Arbitrary for Transaction {
    type Parameters = LedgerState;

    fn arbitrary_with(ledger_state: Self::Parameters) -> Self::Strategy {
        let LedgerState {
            tip_height,
            network,
            ..
        } = ledger_state;

        let height = Height(tip_height.0 + 1);
        let network_upgrade = NetworkUpgrade::current(network, height);
        Self::v1_strategy(ledger_state)
        // match network_upgrade {
        //     NetworkUpgrade::Genesis | NetworkUpgrade::BeforeOverwinter => {
        //         Self::v1_strategy(ledger_state)
        //     } // NetworkUpgrade::Overwinter => Self::v2_strategy(ledger_state),
        //       // NetworkUpgrade::Sapling => Self::v3_strategy(ledger_state),
        //       // NetworkUpgrade::Blossom | NetworkUpgrade::Heartwood | NetworkUpgrade::Canopy => {
        //       //     Self::v4_strategy(ledger_state)
        //       // }
        // }
    }

    type Strategy = BoxedStrategy<Self>;
}
