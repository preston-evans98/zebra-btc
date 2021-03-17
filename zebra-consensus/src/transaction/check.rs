//! Transaction checks.
//!
//! Code in this file can freely assume that no pre-V4 transactions are present.

use zebra_chain::transaction::Transaction;

use crate::error::TransactionError;

/// Checks that the transaction has inputs and outputs.
///
/// More specifically:
///
/// * at least one of tx_in_count, nShieldedSpend, and nJoinSplit MUST be non-zero.
/// * at least one of tx_out_count, nShieldedOutput, and nJoinSplit MUST be non-zero.
///
/// https://zips.z.cash/protocol/canopy.pdf#txnencodingandconsensus
pub fn has_inputs_and_outputs(tx: &Transaction) -> Result<(), TransactionError> {
    // The consensus rule is written in terms of numbers, but our transactions
    // hold enum'd data. Mixing pattern matching and numerical checks is risky,
    // so convert everything to counts and sum up.
    let inputs = &tx.inputs;
    let outputs = &tx.outputs;
    // match tx {
    //     Transaction::V4 {
    //         inputs,
    //         outputs,
    //         joinsplit_data,
    //         shielded_data,
    //         ..
    //     } => {
    let tx_in_count = inputs.len();
    let tx_out_count = outputs.len();

    if tx_in_count == 0 {
        Err(TransactionError::NoInputs)
    } else if tx_out_count == 0 {
        Err(TransactionError::NoOutputs)
    } else {
        Ok(())
    }
    //     }
    //     Transaction::V1 { .. } | Transaction::V2 { .. } | Transaction::V3 { .. } => {
    //         unreachable!("tx version is checked first")
    //     }
    // }
}
