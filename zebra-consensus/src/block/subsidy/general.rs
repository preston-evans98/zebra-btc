//! Block and Miner subsidies, halvings and target spacing modifiers. - [§7.7][7.7]
//!
//! [7.7]: https://zips.z.cash/protocol/protocol.pdf#subsidies

use std::convert::TryFrom;

use zebra_chain::{
    amount::{Amount, Error, NonNegative},
    block::Height,
    parameters::Network,
    transaction::Transaction,
    transparent,
};

use crate::parameters::subsidy::*;

/// The `BlockSubsidy(height)`
///
/// In Bitcoin, the subsidy starts at 50 BTC and halves every 210_000 blocks
/// until the 64th halving. Afterwards 64 halvings, the subsidy is 0.
pub fn block_subsidy(height: Height, network: Network) -> Result<Amount<NonNegative>, Error> {
    let halvings = height.0 / HALVING_INTERVAL.0;
    if halvings >= 64 {
        return Amount::try_from(0);
    }
    let subsidy = MAX_BLOCK_SUBSIDY >> (halvings as u64);
    Amount::try_from(subsidy)
}

/// Returns a list of outputs in `Transaction`, which have a value equal to `Amount`.
pub fn find_output_with_amount(
    transaction: &Transaction,
    amount: Amount<NonNegative>,
) -> Vec<transparent::Output> {
    // TODO: shielded coinbase - Heartwood
    transaction
        .outputs
        .iter()
        .filter(|o| o.value == amount)
        .cloned()
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;
    use color_eyre::Report;

    #[test]
    fn block_subsidy_test() -> Result<(), Report> {
        zebra_test::init();

        block_subsidy_for_network(Network::Mainnet)?;
        block_subsidy_for_network(Network::Testnet)?;

        Ok(())
    }

    fn block_subsidy_for_network(network: Network) -> Result<(), Report> {
        // The initial block subsidy is 50 BTC
        // TODO: troubleshoot `overflowing_literal`; this should be an i64 but is an i32 for some reason
        // assert_eq!(
        //     Amount::try_from(5_000_000_000),
        //     block_subsidy(Height(0), network)
        // );
        // // After the first halving, it's 25 BTC
        // assert_eq!(
        //     Amount::try_from(2_500_000_000),
        //     block_subsidy(HALVING_INTERVAL, network)
        // );
        // After the second halving, it's 12.5 BTC
        assert_eq!(
            Amount::try_from(1_250_000_000),
            block_subsidy(Height(HALVING_INTERVAL.0 * 2), network)
        );

        // After the 15th halving, it's 76_293 satoshis
        assert_eq!(
            Amount::try_from(76_293),
            block_subsidy(Height(HALVING_INTERVAL.0 * 15), network)
        );

        // After the 32nd halving, the block subsidy is 1 satoshi
        // Check that the block subsidy is calculated correctly at the limit
        assert_eq!(
            Amount::try_from(1),
            block_subsidy(Height(HALVING_INTERVAL.0 * 32), network)
        );

        // After the 33rd halving, there is no block subsidy
        // Check that there are no errors
        assert_eq!(
            Amount::try_from(0),
            block_subsidy(Height(HALVING_INTERVAL.0 * 33), network)
        );

        assert_eq!(
            Amount::try_from(0),
            block_subsidy(Height(HALVING_INTERVAL.0 * 63), network)
        );

        assert_eq!(
            Amount::try_from(0),
            block_subsidy(Height(HALVING_INTERVAL.0 * 64), network)
        );

        Ok(())
    }
}
