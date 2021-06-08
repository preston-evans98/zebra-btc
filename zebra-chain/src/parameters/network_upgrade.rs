//! Network upgrade consensus parameters for Zcash.

use NetworkUpgrade::*;

use crate::block;
use crate::parameters::{Network, Network::*};

use std::collections::{BTreeMap, HashMap};
use std::ops::Bound::*;

use chrono::{DateTime, Duration, Utc};

/// A Zcash network upgrade.
///
/// Network upgrades can change the Zcash network protocol or consensus rules in
/// incompatible ways.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum NetworkUpgrade {
    /// The Bitcoin protocol at the Genesis block, before any softforks had been implemented.
    Genesis,
    /// [`BIP 34`](https://github.com/bitcoin/bips/blob/master/bip-0034.mediawiki):
    /// The rule that requires blocks to contain their height (number) in the coinbase input,
    /// and the introduction of version 2 blocks has been implemented since core **v0.7.0**.
    /// The rule took effect for version 2 blocks as of *block 224413* (March 5th 2013),
    /// and version 1 blocks are no longer allowed since *block 227931* (March 25th 2013)
    ///([PR #1526](https://github.com/bitcoin/bitcoin/pull/1526)).
    BIP34,
    /// [`BIP 66`](https://github.com/bitcoin/bips/blob/master/bip-0066.mediawiki):
    /// The strict DER rules and associated version 3 blocks have been implemented since core **v0.10.0**
    ///([PR #5713](https://github.com/bitcoin/bitcoin/pull/5713)).
    BIP66,
    /// [`BIP 65`](https://github.com/bitcoin/bips/blob/master/bip-0065.mediawiki):
    /// The CHECKLOCKTIMEVERIFY softfork was merged in bitcoin core **v0.12.0**
    /// ([PR #6351](https://github.com/bitcoin/bitcoin/pull/6351)), and backported
    // to **v0.11.2** and **v0.10.4**.
    /// Mempool-only CLTV was added in [PR #6124](https://github.com/bitcoin/bitcoin/pull/6124).
    BIP65,
    /// [`BIP 112`](https://github.com/bitcoin/bips/blob/master/bip-0112.mediawiki):
    /// The CHECKSEQUENCEVERIFY opcode has been implemented since **v0.12.1**
    /// ([PR #7524](https://github.com/bitcoin/bitcoin/pull/7524)), and has been *buried* since core **v0.19.0**
    /// ([PR #16060](https://github.com/bitcoin/bitcoin/pull/16060)).
    CSV,
    /// The Segregated Witness Network Upgrade creates a new data structure "the witness structure"
    /// which is commmitted to in the coinbase transaction and contains only the witnesses (signatures, scripts, etc.) for
    /// transactions. SegWit fixes transaction malleability and yields a defacto block size increase.
    /// SegWit includes [`BIP 141`](https://github.com/bitcoin/bips/blob/master/bip-0141.mediawiki),
    /// [`BIP 143`](https://github.com/bitcoin/bips/blob/master/bip-0143.mediawiki)
    /// [`BIP 144`](https://github.com/bitcoin/bips/blob/master/bip-0144.mediawiki) and
    /// [`BIP 145`](https://github.com/bitcoin/bips/blob/master/bip-0145.mediawiki)
    SegWit,
}

/// Mainnet network upgrade activation heights.
///
/// This is actually a bijective map, but it is const, so we use a vector, and
/// do the uniqueness check in the unit tests.
pub(crate) const MAINNET_ACTIVATION_HEIGHTS: &[(block::Height, NetworkUpgrade)] = &[
    (block::Height(0), Genesis),
    (block::Height(227931), BIP34), // 0x000000000000024b89b42a942fe0d9fea3bb44ab7bd1b19115dd6a759c0808b8;
    (block::Height(363725), BIP66), // 00000000000000000379eaa19dce8c9b722d46ae6a57c2f1a988119488b50931
    (block::Height(388381), BIP65), // 000000000000000004c2b624ed5d7756c508d90fd0da2c7c679febfa6c4735f0
    (block::Height(419328), CSV), // 000000000000000004a1b34462cb8aeebd5799177f7a29cf28f2d1961716b5b5
    (block::Height(481824), SegWit), // 0000000000000000001c8018d9cb3b742ef25114f27563e3fc4a1902167f9893
];

/// Testnet network upgrade activation heights.
///
/// This is actually a bijective map, but it is const, so we use a vector, and
/// do the uniqueness check in the unit tests.
pub(crate) const TESTNET_ACTIVATION_HEIGHTS: &[(block::Height, NetworkUpgrade)] = &[
    (block::Height(0), Genesis),
    (block::Height(21111), BIP34), // 0x0000000023b3a96d3484e5abb3755c413e7d41500f8e2a5c3f0dd01299cd8ef8
    (block::Height(330776), BIP66), // 000000002104c8c45e99a8853285a3b592602a3ccde2b832481da85e9e4ba182
    (block::Height(581885), BIP65), // 00000000007f6655f22f98e72ed80d8b06dc761d5da09df0fa1dc4be4f861eb6
    (block::Height(770112), CSV), // 00000000025e930139bac5c6c31a403776da130831ab85be56578f3fa75369bb
    (block::Height(834624), SegWit), // 00000000002b980fcd729daaa248fd9316a5200e9b367f4ff2c42453e84201ca
];

/// The Consensus Branch Id, used to bind transactions and blocks to a
/// particular network upgrade.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct ConsensusBranchId(u32);

impl From<ConsensusBranchId> for u32 {
    fn from(branch: ConsensusBranchId) -> u32 {
        branch.0
    }
}

impl From<u32> for ConsensusBranchId {
    fn from(branch: u32) -> ConsensusBranchId {
        ConsensusBranchId(branch)
    }
}

// /// Network Upgrade Consensus Branch Ids.
// ///
// /// Branch ids are the same for mainnet and testnet. If there is a testnet
// /// rollback after a bug, the branch id changes.
// ///
// /// Branch ids were introduced in the Overwinter upgrade, so there are no
// /// Genesis or BeforeOverwinter branch ids.
// ///
// /// This is actually a bijective map, but it is const, so we use a vector, and
// /// do the uniqueness check in the unit tests.
// pub(crate) const CONSENSUS_BRANCH_IDS: &[(NetworkUpgrade, ConsensusBranchId)] = &[
//     (Overwinter, ConsensusBranchId(0x5ba81b19)),
//     (Sapling, ConsensusBranchId(0x76b809bb)),
//     (Blossom, ConsensusBranchId(0x2bb40e60)),
//     (Heartwood, ConsensusBranchId(0xf5b9230b)),
//     (Canopy, ConsensusBranchId(0xe9ff75a6)),
// ];

/// The target block spacing
const POW_TARGET_SPACING: i64 = 10 * 60;

/// The number of blocks per averaging window for difficulty threshold arithmetic mean calculations.
pub const POW_AVERAGING_WINDOW: usize = 2016;

/// The multiplier used to derive the testnet minimum difficulty block time gap
/// threshold.
const TESTNET_MINIMUM_DIFFICULTY_GAP_MULTIPLIER: i32 = 2;

impl NetworkUpgrade {
    /// Returns a BTreeMap of activation heights and network upgrades for
    /// `network`.
    ///
    /// If the activation height of a future upgrade is not known, that
    /// network upgrade does not appear in the list.
    ///
    /// This is actually a bijective map.
    pub(crate) fn activation_list(network: Network) -> BTreeMap<block::Height, NetworkUpgrade> {
        match network {
            Mainnet => MAINNET_ACTIVATION_HEIGHTS,
            Testnet => TESTNET_ACTIVATION_HEIGHTS,
        }
        .iter()
        .cloned()
        .collect()
    }

    pub fn branch_id(&self) -> u32 {
        eprintln!("Error: called NetworkUpgrade::branch_id() Branch Ids were introduced in ZIP 200 and do not exist in Bitcoin.");
        // FIXME
        0
    }

    /// Returns the current network upgrade for `network` and `height`.
    pub fn current(network: Network, height: block::Height) -> NetworkUpgrade {
        NetworkUpgrade::activation_list(network)
            .range(..=height)
            .map(|(_, nu)| *nu)
            .next_back()
            .expect("every height has a current network upgrade")
    }

    /// Returns the next network upgrade for `network` and `height`.
    ///
    /// Returns None if the name of the next upgrade has not been decided yet.
    pub fn next(network: Network, height: block::Height) -> Option<NetworkUpgrade> {
        NetworkUpgrade::activation_list(network)
            .range((Excluded(height), Unbounded))
            .map(|(_, nu)| *nu)
            .next()
    }

    /// Returns the activation height for this network upgrade on `network`.
    ///
    /// Returns None if this network upgrade is a future upgrade, and its
    /// activation height has not been set yet.
    pub fn activation_height(&self, network: Network) -> Option<block::Height> {
        NetworkUpgrade::activation_list(network)
            .iter()
            .filter(|(_, nu)| nu == &self)
            .map(|(height, _)| *height)
            .next()
    }

    /// Returns the target block spacing for the network upgrade.
    pub fn target_spacing(&self) -> Duration {
        match self {
            Genesis | BIP34 | BIP66 | BIP65 | CSV | SegWit => Duration::seconds(POW_TARGET_SPACING),
        }
    }

    /// Returns the target block spacing for `network` and `height`.
    ///
    /// See [`target_spacing()`] for details.
    pub fn target_spacing_for_height(network: Network, height: block::Height) -> Duration {
        NetworkUpgrade::current(network, height).target_spacing()
    }

    /// Returns the minimum difficulty block spacing for `network` and `height`.
    /// Returns `None` if the testnet minimum difficulty consensus rule is not active.
    ///
    /// Based on https://zips.z.cash/zip-0208#minimum-difficulty-blocks-on-the-test-network
    ///
    /// `zcashd` requires a gap that's strictly greater than 6 times the target
    /// threshold, but ZIP-205 and ZIP-208 are ambiguous. See bug #1276.
    pub fn minimum_difficulty_spacing_for_height(
        network: Network,
        height: block::Height,
    ) -> Option<Duration> {
        match (network, height) {
            (Network::Mainnet, _) => None,
            (Network::Testnet, _) => {
                let network_upgrade = NetworkUpgrade::current(network, height);
                Some(network_upgrade.target_spacing() * TESTNET_MINIMUM_DIFFICULTY_GAP_MULTIPLIER)
            }
        }
    }

    /// Returns true if the gap between `block_time` and `previous_block_time` is
    /// greater than the Testnet minimum difficulty time gap. This time gap
    /// depends on the `network` and `block_height`.
    ///
    /// Returns false on Mainnet, when `block_height` is less than the minimum
    /// difficulty start height, and when the time gap is too small.
    ///
    /// `block_time` can be less than, equal to, or greater than
    /// `previous_block_time`, because block times are provided by miners.
    ///
    /// Implements the Testnet minimum difficulty adjustment from ZIPs 205 and 208.
    ///
    /// Spec Note: Some parts of ZIPs 205 and 208 previously specified an incorrect
    /// check for the time gap. This function implements the correct "greater than"
    /// check.
    pub fn is_testnet_min_difficulty_block(
        network: Network,
        block_height: block::Height,
        block_time: DateTime<Utc>,
        previous_block_time: DateTime<Utc>,
    ) -> bool {
        let block_time_gap = block_time - previous_block_time;
        if let Some(min_difficulty_gap) =
            NetworkUpgrade::minimum_difficulty_spacing_for_height(network, block_height)
        {
            block_time_gap > min_difficulty_gap
        } else {
            false
        }
    }

    /// Returns the averaging window timespan for the network upgrade.
    ///
    /// `AveragingWindowTimespan` from the Zcash specification.
    pub fn averaging_window_timespan(&self) -> Duration {
        self.target_spacing() * (POW_AVERAGING_WINDOW as _)
    }

    /// Returns the averaging window timespan for `network` and `height`.
    ///
    /// See [`averaging_window_timespan()`] for details.
    pub fn averaging_window_timespan_for_height(
        network: Network,
        height: block::Height,
    ) -> Duration {
        NetworkUpgrade::current(network, height).averaging_window_timespan()
    }

    /// Returns true if the maximum block time rule is active for `network` and `height`.
    ///
    /// Always returns true
    pub fn is_max_block_time_enforced(network: Network, height: block::Height) -> bool {
        match network {
            Network::Mainnet => true,
            Network::Testnet => true,
        }
    }
}

// impl ConsensusBranchId {
//     /// Returns the current consensus branch id for `network` and `height`.
//     ///
//     /// Returns None if the network has no branch id at this height.
//     pub fn current(network: Network, height: block::Height) -> Option<ConsensusBranchId> {
//         NetworkUpgrade::current(network, height).branch_id()
//     }
// }
