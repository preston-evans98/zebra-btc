//! Generate blockchain testing constructions
use chrono::{DateTime, NaiveDateTime, Utc};
use std::{iter::FromIterator, sync::Arc};
use transparent::CoinbaseData;

use crate::{
    serialization::{BitcoinDeserialize, BitcoinSerialize},
    transaction::{LockTime, Transaction},
    transparent,
};

use super::super::{serialize::MAX_BLOCK_BYTES, Block, Header};

/// Generate a block header
pub fn block_header() -> Header {
    Header::bitcoin_deserialize(&zebra_test::vectors::DUMMY_HEADER[..]).unwrap()
}

/// Generate a block with multiple transactions just below limit
pub fn large_multi_transaction_block() -> Block {
    multi_transaction_block(false)
}

/// Generate a block with one transaction and multiple inputs just below limit
pub fn large_single_transaction_block() -> Block {
    single_transaction_block(false)
}

/// Generate a block with multiple transactions just above limit
pub fn oversized_multi_transaction_block() -> Block {
    multi_transaction_block(true)
}

/// Generate a block with one transaction and multiple inputs just above limit
pub fn oversized_single_transaction_block() -> Block {
    single_transaction_block(true)
}

// Implementation of block generation with multiple transactions
fn multi_transaction_block(oversized: bool) -> Block {
    // A dummy transaction
    let tx = Transaction::bitcoin_deserialize(&zebra_test::vectors::DUMMY_TX1[..]).unwrap();
    let coinbase_input = transparent::Input::Coinbase {
        height: None,
        data: CoinbaseData(Vec::new()),
        sequence: 0,
    };
    let output =
        transparent::Output::bitcoin_deserialize(&zebra_test::vectors::DUMMY_OUTPUT1[..]).unwrap();
    let coinbase = Transaction::new(
        1,
        vec![coinbase_input],
        vec![output],
        LockTime::Height(crate::block::Height(0)),
    );

    // A block header
    let mut header = block_header();

    // Serialize header
    let mut data_header = Vec::new();
    header
        .bitcoin_serialize(&mut data_header)
        .expect("Block header should serialize");

    // Calculate the number of transactions we need
    let mut max_transactions_in_block =
        (MAX_BLOCK_BYTES as usize - data_header.len() - coinbase.len())
            / zebra_test::vectors::DUMMY_TX1[..].len();
    if oversized {
        max_transactions_in_block += 1;
    }

    // Create transactions to be just below or just above the limit
    let transactions = std::iter::once(Arc::new(coinbase))
        .chain(std::iter::repeat(Arc::new(tx)))
        .take(max_transactions_in_block + 1)
        .collect::<Vec<_>>();

    header.merkle_root =
        crate::block::merkle::Root::from_iter(transactions.iter().map(|tx| tx.hash()));

    // Add the transactions into a block
    Block {
        header,
        transactions,
    }
}

// Implementation of block generation with one transaction and multiple inputs
fn single_transaction_block(oversized: bool) -> Block {
    // Dummy input and output
    let coinbase = transparent::Input::Coinbase {
        height: None,
        data: CoinbaseData(Vec::new()),
        sequence: 0,
    };
    let input =
        transparent::Input::bitcoin_deserialize(&zebra_test::vectors::DUMMY_INPUT1[..]).unwrap();
    let output =
        transparent::Output::bitcoin_deserialize(&zebra_test::vectors::DUMMY_OUTPUT1[..]).unwrap();

    // A block header
    let mut header = block_header();

    // Serialize header
    let mut data_header = Vec::new();
    header
        .bitcoin_serialize(&mut data_header)
        .expect("Block header should serialize");

    // Serialize a LockTime
    let lock_time = LockTime::Time(DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(61, 0),
        Utc,
    ));
    let mut data_locktime = Vec::new();
    lock_time
        .bitcoin_serialize(&mut data_locktime)
        .expect("LockTime should serialize");

    // Calculate the number of inputs we need
    // let mut max_inputs_in_tx = (MAX_BLOCK_BYTES as usize
    //     - data_header.len()
    //     - zebra_test::vectors::DUMMY_OUTPUT1[..].len()
    //     - data_locktime.len())
    //     / (zebra_test::vectors::DUMMY_INPUT1[..].len());
    let mut max_outputs_in_tx =
        (MAX_BLOCK_BYTES as usize - data_header.len() - coinbase.len() - data_locktime.len())
            / zebra_test::vectors::DUMMY_OUTPUT1[..].len();

    // The above calculation is wrong somehow. This makes it correct. TODO: Troubleshoot
    max_outputs_in_tx -= 5;

    if oversized {
        max_outputs_in_tx += 1;
    }

    // Create inputs to be just below the limit
    let inputs = vec![coinbase];
    // let inputs = std::iter::once(coinbase)
    //     .chain(std::iter::repeat(input))
    //     .take(max_inputs_in_tx)
    //     .collect::<Vec<_>>();

    let outputs = std::iter::repeat(output).take(max_outputs_in_tx).collect();

    // Create a big transaction
    let big_transaction = Transaction::new(1, inputs, outputs, lock_time);

    // Put the big transaction into a block
    let transactions = vec![Arc::new(big_transaction)];
    header.merkle_root =
        crate::block::merkle::Root::from_iter(transactions.iter().map(|tx| tx.hash()));
    Block {
        header,
        transactions,
    }
}
