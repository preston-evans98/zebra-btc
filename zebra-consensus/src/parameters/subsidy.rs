//! Constants for Block Subsidy, Funding Streams, and Founders’ Reward

use zebra_chain::{amount::COIN, block::Height};

/// The largest block subsidy, used before the first halving. 50 BTC
pub const MAX_BLOCK_SUBSIDY: u64 = (50 * COIN) as u64;

pub const HALVING_INTERVAL: Height = Height(210000);
