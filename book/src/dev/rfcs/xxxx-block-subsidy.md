- Feature Name: Block subsidy
- Start Date: 2020-10-05
- Design PR: [ZcashFoundation/zebra#0000](https://github.com/ZcashFoundation/zebra/pull/0000)
- Zebra Issue: [ZcashFoundation/zebra#0000](https://github.com/ZcashFoundation/zebra/issues/0000)

# Summary
[summary]: #summary


Zebra manages [semantic verification](https://github.com/ZcashFoundation/zebra/blob/main/book/src/dev/rfcs/0002-parallel-verification.md#definitions) in the `zebra-consensus` crate, this is done for all incoming blocks. Inside each block the coinbase transaction is special, it holds the subsidy rewards that are paid to different participants(miners, founders, funding stream receivers). This RFC describes how to implement the needed calculations and verification for block subsidy and miner fees.

# Motivation
[motivation]: #motivation

All incoming blocks must be validated with the protocol specifications, there is no way to avoid this logic to be present in any Zcash protocol compatible implementation such as Zebra.

Block subsidy and miner fees are part of the protocol, the semantic verification of them apply to the coinbase transaction and they must be verified in Zebra.

# Definitions
[definitions]: #definitions

- **founders reward**: The portion of the block reward that goes into a pre defined founder address. 
- **funding streams**:  The portion of the block reward that goes into a pre defined funding stream address.
- **miner subsidy**: The portion of the block reward that goes into the miner of the block. 
- **block subsidy**: Miner subsidy plus founders reward or funding stream if any of the 2 are active.
- **coinbase transaction**: The first transaction in a block; this is the transaction that handles block subsidy.
- **network upgrade**: An intentional consensus rule change undertaken by the community in order to improve the network.
- **miner fees**: The sum of the extra [transparent value pool](#transparent-value-pool-calculation) amounts, for all the transactions in a block.

### Transparent value pool calculation

There is a "transparent value pool" inside each transaction, containing "funds in motion" (distinct from the actual transparent value pool of UTXOs, being "funds at rest"). Balance of that pool works as follows:

Transparent:

- `tx_in` adds to the pool.
- `tx_out` subtracts from the pool.

https://zips.z.cash/protocol/canopy.pdf#txnencodingandconsensus

Sprout:

- `vpub_new` values from `vJoinSplit` of the transaction add to the pool.
- `vpub_old` values from `vJoinSplit` of the transaction subtract from the pool.

https://zips.z.cash/protocol/canopy.pdf#joinsplitencodingandconsensus

Sapling:

- `valueBalance` adds to the pool. It itself is equal to `sum(SaplingSpends) - sum(SaplingOutputs)`, so it has the same functional effect as for transparent and Sprout.

https://zips.z.cash/protocol/canopy.pdf#joinsplitencodingandconsensus

The balance rule is that this pool must have non-negative value, and its net value is the miner fee for the transaction.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In Zebra the consensus related code lives in the `zebra-consensus` crate. The block subsidy checks are implemented in `zebra-consensus`, in the following module:

- `zebra-consensus/src/block/subsidy/subsidy.rs` 

Inside `zebra-consensus/src/block/subsidy/` the following submodules will be created:

- `general.rs`: General block reward functions and utilities.
- `founders_reward.rs`: Specific functions related to funders reward.
- `funding_streams.rs`: Specific functions for funding streams.

In addition to calculations the block subsidy requires constants defined in the protocol. The implementation will also create additional constants, all of them will live at:

- `zebra-consensus/src/parameters/subsidy.rs`

Checking functions for blocks are implemented at `zebra-consensus/src/block/check.rs` and they are called at `zebra-consensus/src/block.rs`. This follows the already existing structure for block validation in Zebra.

It is important to note that all blocks before Sapling are verified by the CheckpointVerifier, so they are not considered in this design. The following table will show what periods are included and what is not in this proposal:

| Height                                 | Miner    | Founder reward | Funding streams | Shielded Coinbase | Target Spacing |
|----------------------------------------|----------|----------------|-----------------|-------------------|----------------|             
| ~Genesis~                              | ~**0%**~ | ~**0%**~       | ~**0%**~        | ~**No**~          | ~**None**~     |
| ~(Genesis + 1)..Sapling~               | ~80%~    | ~20%~          | ~0%~            | ~No~              | ~150 seconds~  |
| Sapling..Blossom                       | 80%      | 20%            | 0%              | No                | 150 seconds    |
| Blossom..Heartwood                     | 80%      | 20%            | 0%              | No                | 75 seconds     |
| Heartwood..Canopy                      | 80%      | 20%            | 0%              | Yes               | 75 seconds     |
| Canopy..Second Halving                 | 80%      | 0%             | 20%             | Yes               | 75 seconds     |
| Second Halving..                       | 100%     | 0%             | 0%              | Yes               | 75 seconds     |

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Given the module structure proposed above all the block subsidy calculations from the protocol must be implemented. The final goal is to do checks for each incoming block and make sure they pass the consensus rules.

To do the calculations and checks the following constants, types and functions need to be introduced:

## Constants

- `SLOW_START_INTERVAL: Height`
- `SLOW_START_SHIFT: Height`
- `MAX_BLOCK_SUBSIDY: u64`
- `PRE_BLOSSOM_POW_TARGET_SPACING: Duration`
- `POST_BLOSSOM_POW_TARGET_SPACING: Duration`
- `BLOSSOM_POW_TARGET_SPACING_RATIO: u64`
- `PRE_BLOSSOM_HALVING_INTERVAL: Height`
- `POST_BLOSSOM_HALVING_INTERVAL: Height`
- `FOUNDERS_FRACTION_DIVISOR: u64`
- `FOUNDER_ADDRESS_CHANGE_INTERVAL: u64`
- `FOUNDER_ADDRESSES_MAINNET: [&'static str; 48]`
- `FOUNDER_ADDRESSES_TESTNET: [&'static str; 48]`

### Funding streams parameter constants

The design suggests to implement the parameters needed for funding streams as:

```
/// The funding stream receiver categories
#[allow(missing_docs)]
pub enum FundingStreamReceiver {
    ElectricCoinCompany,
    ZcashFoundation,
    MajorGrants,
}

/// The numerator for each funding stream receiving category 
/// as described in [protocol specification §7.9.1][7.9.1].
/// 
/// [7.9.1]: https://zips.z.cash/protocol/protocol.pdf#zip214fundingstreams
const FUNDING_STREAM_RECEIVER_NUMERATORS: &[(FundingStreamReceiver, u64)] = &[
    (FundingStreamReceiver::ElectricCoinCompany, 7),
    (FundingStreamReceiver::ZcashFoundation, 5),
    (FundingStreamReceiver::MajorGrants, 8),
];

/// Denominator as described in [protocol specification §7.9.1][7.9.1].
/// 
/// [7.9.1]: https://zips.z.cash/protocol/protocol.pdf#zip214fundingstreams
pub const FUNDING_STREAM_RECEIVER_DENOMINATOR: u64 = 100;

#[allow(missing_docs)]
pub enum FundingStreamRange {
    StartHeight,
    EndHeight,
}

/// Start and end Heights for funding streams 
/// as described in [protocol specification §7.9.1][7.9.1].
/// 
/// [7.9.1]: https://zips.z.cash/protocol/protocol.pdf#zip214fundingstreams
const FUNDING_STREAM_HEIGHT_RANGES: &[(Network, Range<_>)] = &[
    (Network::Mainnet, Height(1_046_400)..Height(2_726_400)),
    (Network::Testnet, Height(1_028_500)..Height(2_796_000)),
];
```

Each `FundingStreamReceiver` for the mainnet must have 48 payment addresses associated with and 51 for the testnet. 

https://zips.z.cash/zip-0214#mainnet-recipient-addresses

The following constants are needed:

- `FUNDING_STREAM_ECC_ADDRESSES_MAINNET: [&'static str; 48]`
- `FUNDING_STREAM_ZF_ADDRESSES_MAINNET: [&'static str; 48]`
- `FUNDING_STREAM_MG_ADDRESSES_MAINNET: [&'static str; 48]`
- `FUNDING_STREAM_ECC_ADDRESSES_TESTNET: [&'static str; 51]`
- `FUNDING_STREAM_ZF_ADDRESSES_TESTNET: [&'static str; 51]`
- `FUNDING_STREAM_MG_ADDRESSES_TESTNET: [&'static str; 51]`

## General subsidy

The block subsidy and miner reward among other utility functions are inside the general subsidy category.

https://zips.z.cash/protocol/canopy.pdf#subsidyconcepts

https://zips.z.cash/protocol/canopy.pdf#subsidies

- `block_subsidy(Height, Network) -> Result<Amount<NonNegative>, Error>` - Total block subsidy.
- `miner_subsidy(Height, Network) -> Result<Amount<NonNegative>, Error>` - Miner portion.
- `miner_fees(&Block) -> Result<Amount<NonNegative>, Error>` - Total block fees calculated as [Transparent value pool calculation](#transparent-value-pool-calculation) that belongs to the miner.
- `coinbase_sum_outputs(&Transaction) -> Result<Amount<NonNegative>, Error>` - Sum of all output values in the coinbase transaction.
- `find_output_with_amount(&Transaction, Amount<NonNegative>) -> Vec<transparent::Output>` - Outputs where value equal to Amount.
- `shielded_coinbase(Height, Network, &Transaction) -> Result<(), Error>` - Validate shielded coinbase rules.

## Founders reward

Only functions specific to calculation of founders reward.

https://zips.z.cash/protocol/canopy.pdf#foundersreward

- `founders_reward(Height, Network) -> Result<Amount<NonNegative>, Error>` - Founders reward portion for this block.
- `founders_reward_address(Height, Network) -> Result<PayToScriptHash, Error>` - Address of the receiver founder at this block. All specified founders reward addresses are transparent `zebra_chain::transparent:Address::PayToScriptHash` addresses. (Even after the shielded coinbase changes in ZIP-213, introduced in Heartwood.)

## Funding streams

Only functions specific to the calculation of funding streams.

https://zips.z.cash/protocol/canopy.pdf#fundingstreams

https://zips.z.cash/zip-0207

https://zips.z.cash/zip-0214

- `funding_stream(height, newtork) -> Result<Amount<NonNegative>, Error>` - Funding stream portion for this block.
- `funding_stream_address(height, network) -> Result<FundingStreamAddress, Error>` - Address of the funding stream receiver at this block. The funding streams addresses can be transparent `zebra_chain::transparent:Address::PayToScriptHash` or `zebra_chain::sapling:Address` addresses.

## Consensus rules

`zebra-consensus/src/block.rs` will call the following function implemented inside `zebra-consensus/src/block/check.rs`:

`subsidy_is_valid(Network, &Block) -> Result<(), BlockError>`

`subsidy_is_valid()` will call individual functions(also implemented in `zebra-consensus/src/block/check.rs`) to verify the following consensus rules:

### 1 - Founders reward:

*[Pre-Canopy] A coinbase transaction at `height` **MUST** include at least one output that pays exactly `FoundersReward(height)` zatoshi with a standard P2SH script of the form `OP_HASH160 FounderRedeemScriptHash (height) OP_EQUAL` as its scriptPubKey.* https://zips.z.cash/protocol/protocol.pdf#foundersreward

We make use of the founders reward functions here. We get the amount of the reward with `founders_reward(height, network)` and the address for the reward at height with `founders_reward_address(height, network)`. Next we get a list of outputs that match the amount with the utility function `find_output_with_amount()`. Finally with this list, we check if any of the output scripts addresses matches our computed address.

### 2 - Funding stream:

*[Canopy onward]  The coinbase transaction at `height` **MUST**  contain at least one output per funding stream `fs` active at
height, that pays `fs.Value(height)` zatoshi in the prescribed way to the stream’s recipient address represented by `fs.AddressList` of `fs.AddressIndex(height)`.* https://zips.z.cash/protocol/protocol.pdf#fundingstreams

We make use of the funding streams functions here, similar to founders reward . We get the amount of the reward using `funding_stream(height, network)` and then the address with `funding_stream_address(height, network)`. Next we get a list of outputs that match the amount with the utility function `find_output_with_amount()`. Finally with this list, we check if any of the output scripts matches the address we have computed.

### 3 - Miner subsidy:

*The total amount of transparent outputs from a coinbase transaction, minus the amount of the `valueBalance` field if present, **MUST NOT** be greater than the amount of miner subsidy plus the total amount of transaction fees paid by transactions in this block.* https://zips.z.cash/protocol/canopy.pdf#txnencodingandconsensus

So the rule is the following before Canopy:

`coinbase_sum_outputs() <= miner_subsidy() + founders_reward() + transaction_fees()` 

and after Canopy:

`coinbase_sum_outputs() <= miner_subsidy() + funding_stream() + transaction_fees()`

### 4 - Shielded coinbase:

*[Pre-Heartwood] A coinbase transaction MUST NOT have any JoinSplit descriptions, Spend descriptions, or Output descriptions.*

*[Heartwood onward] A coinbase transaction MUST NOT have any JoinSplit descriptions or Spend descriptions.*

*[Heartwood onward] The consensus rules applied to `valueBalance`, `vShieldedOutput`, and `bindingSig` in non-coinbase transactions MUST also be applied to coinbase transactions.*

https://zips.z.cash/zip-0213#specification

This rules are implemented inside `shielded_coinbase()`.

### Validation errors

A `SubsidyError` type will be created to handle validation of the above consensus rules:

```
pub enum SubsidyError {
    #[error("not a coinbase transaction")]
    NoCoinbase,

    #[error("founders reward amount not found")]
    FoundersRewardAmountNotFound,
    
    #[error("founders reward address not found")]
    FoundersRewardAddressNotFound,
    
    #[error("founding stream amount not found")]
    FundingStreamAmountNotFound,
    
    #[error("founding stream address not found")]
    FundingStreamAddressNotFound,
    
    #[error("the sum of outputs is greater than calculated subsidies and transaction fees.")]
    MinerSubsidyRuleBroken,
    
    #[error("invalid shielded descriptions found.")]
    ShieldedDescriptionsInvalid,

    #[error("broken rule in shielded transaction inside coinbase.")]
    ShieldedRuleBroken,
}
```

## Test Plan

For each network(Mainnet, Testnet), calculation of subsidy amounts need a `Height` as input and will output different amounts according to it.

- Test all subsidy amount functions(`block_subsidy()`, `miner_subsidy()`, `founders_reward()` and `funding_stream()`) against all network upgrade events of each network and make sure they return the expected amounts according to known outputs.

For each network, the address of the reward receiver on each block will depend on the `Height`.

- Test all subsidy address functions(`founders_reward_address()` and `funding_stream_address()`) against all network upgrade events of each network and make sure they return the expected addresses.

Validation tests will test the consensus rules using real blocks from `zebra-test` crate. For both networks, blocks for all network upgrades were added to the crate in [#1096](https://github.com/ZcashFoundation/zebra/pull/1096). Blocks containing shielded coinbase were also introduced at [#1116](https://github.com/ZcashFoundation/zebra/pull/1116)

- Test validation functions(`subsidy_is_valid()`) against all the blocks zebra haves available in the test vectors collection for both networks(`zebra_test::vectors::MAINNET_BLOCKS`  and `zebra_test::vectors::TESTNET_BLOCKS`), all blocks should pass validation.

The validation errors at `SubsidyError` must be tested at least once.

- Create tests to trigger each error from `SubsidyError`.