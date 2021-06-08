//! Genesis consensus parameters for each Zcash network.

use crate::{block, parameters::Network};

/// The previous block hash for the genesis block.
///
/// All known networks use the Bitcoin `null` value for the parent of the
/// genesis block. (In Bitcoin, `null` is `[0; 32]`.)
pub const GENESIS_PREVIOUS_BLOCK_HASH: block::Hash = block::Hash([0; 32]);

/// Returns the hash for the genesis block in `network`.
pub fn genesis_hash(network: Network) -> block::Hash {
    match network {
        // bitcoin-cli getblockhash 0
        Network::Mainnet => "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f",
        // bitcoin-cli -testnet getblockhash 0
        Network::Testnet => "000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943",
    }
    .parse()
    .expect("hard-coded hash parses")
}
