//! The primary implementation of the `zebra_state::Service` built upon sled

use std::{collections::HashMap, sync::Arc};

use tracing::trace;
use zebra_chain::transparent;
use zebra_chain::{
    block::{self, Block},
    parameters::{Network, GENESIS_PREVIOUS_BLOCK_HASH},
};

use crate::{BoxError, Config, HashOrHeight, QueuedBlock};
use sled_format::{SledDeserialize, SledSerialize, FromSled};

mod sled_format;

/// The finalized part of the chain state, stored in sled.
///
/// This structure has two categories of methods:
///
/// - *synchronous* methods that perform writes to the sled state;
/// - *asynchronous* methods that perform reads.
///
/// For more on this distinction, see RFC5. The synchronous methods are
/// implemented as ordinary methods on the [`FinalizedState`]. The asynchronous
/// methods are not implemented using `async fn`, but using normal methods that
/// return `impl Future<Output = ...>`. This allows them to move data (e.g.,
/// clones of handles for [`sled::Tree`]s) into the futures they return.
///
/// This means that the returned futures have a `'static` lifetime and don't
/// borrow any resources from the [`FinalizedState`], and the actual database work is
/// performed asynchronously when the returned future is polled, not while it is
/// created.  This is analogous to the way [`tower::Service::call`] works.
pub struct FinalizedState {
    /// Queued blocks that arrived out of order, indexed by their parent block hash.
    queued_by_prev_hash: HashMap<block::Hash, QueuedBlock>,

    hash_by_height: sled::Tree,
    height_by_hash: sled::Tree,
    block_by_height: sled::Tree,
    tx_by_hash: sled::Tree,
    utxo_by_outpoint: sled::Tree,
    sprout_nullifiers: sled::Tree,
    sapling_nullifiers: sled::Tree,
    // sprout_anchors: sled::Tree,
    // sapling_anchors: sled::Tree,
}

impl FinalizedState {
    pub fn new(config: &Config, network: Network) -> Self {
        let db = config.sled_config(network).open().unwrap();

        Self {
            queued_by_prev_hash: HashMap::new(),
            hash_by_height: db.open_tree(b"hash_by_height").unwrap(),
            height_by_hash: db.open_tree(b"height_by_hash").unwrap(),
            block_by_height: db.open_tree(b"block_by_height").unwrap(),
            tx_by_hash: db.open_tree(b"tx_by_hash").unwrap(),
            utxo_by_outpoint: db.open_tree(b"utxo_by_outpoint").unwrap(),
            sprout_nullifiers: db.open_tree(b"sprout_nullifiers").unwrap(),
            sapling_nullifiers: db.open_tree(b"sapling_nullifiers").unwrap(),
        }
    }

    /// Queue a finalized block to be committed to the state.
    ///
    /// After queueing a finalized block, this method checks whether the newly
    /// queued block (and any of its descendants) can be committed to the state.
    pub fn queue_and_commit_finalized_blocks(&mut self, queued_block: QueuedBlock) {
        let prev_hash = queued_block.block.header.previous_block_hash;
        self.queued_by_prev_hash.insert(prev_hash, queued_block);

        while let Some(queued_block) = self.queued_by_prev_hash.remove(&self.finalized_tip_hash()) {
            let height = queued_block
                .block
                .coinbase_height()
                .expect("valid blocks must have a height");
            self.commit_finalized(queued_block);
            metrics::counter!("state.committed.block.count", 1);
            metrics::gauge!("state.committed.block.height", height.0 as _);
        }

        metrics::gauge!(
            "state.queued.block.count",
            self.queued_by_prev_hash.len() as _
        );
    }

    /// Returns the hash of the current finalized tip block.
    pub fn finalized_tip_hash(&self) -> block::Hash {
        self.tip()
            .expect("inability to look up tip is unrecoverable")
            .map(|(_, hash)| hash)
            // if the state is empty, return the genesis previous block hash
            .unwrap_or(GENESIS_PREVIOUS_BLOCK_HASH)
    }

    /// Returns the height of the current finalized tip block.
    pub fn finalized_tip_height(&self) -> Option<block::Height> {
        self.tip()
            .expect("inability to look up tip is unrecoverable")
            .map(|(height, _)| height)
    }

    /// Immediately commit `block` to the finalized state.
    pub fn commit_finalized_direct(&mut self, block: Arc<Block>) -> Result<block::Hash, BoxError> {
        let height = block
            .coinbase_height()
            .expect("finalized blocks are valid and have a coinbase height");
        let hash = block.hash();

        trace!(?height, "Finalized block");

        // TODO: check highest entry of hash_by_height as in RFC

        self.hash_by_height.zs_insert(height, hash)?;
        self.height_by_hash.zs_insert(hash, height)?;
        self.block_by_height.zs_insert(height, &*block)?;

        for transaction in block.transactions.iter() {
            let transaction_hash = transaction.hash();
            self.tx_by_hash.zs_insert(transaction_hash, transaction)?;

            for (index, output) in transaction.outputs().iter().enumerate() {
                let outpoint = transparent::OutPoint {
                    hash: transaction_hash,
                    index: index as _,
                };

                self.utxo_by_outpoint.zs_insert(outpoint, output)?;
            }

            for sprout_nullifier in transaction.sprout_nullifiers() {
                self.sprout_nullifiers.zs_insert(sprout_nullifier, ())?;
            }

            for sapling_nullifier in transaction.sapling_nullifiers() {
                self.sapling_nullifiers.zs_insert(sapling_nullifier, ())?;
            }
        }

        Ok(hash)
    }

    /// Commit a finalized block to the state.
    ///
    /// It's the caller's responsibility to ensure that blocks are committed in
    /// order. This function is called by [`queue`], which ensures order.
    /// It is intentionally not exposed as part of the public API of the
    /// [`FinalizedState`].
    fn commit_finalized(&mut self, queued_block: QueuedBlock) {
        let QueuedBlock { block, rsp_tx } = queued_block;
        let result = self.commit_finalized_direct(block);
        let _ = rsp_tx.send(result.map_err(Into::into));
    }

    // TODO: this impl works only during checkpointing, it needs to be rewritten
    pub fn block_locator(&self) -> Result<Vec<block::Hash>, BoxError> {
        let (tip_height, _) = match self.tip()? {
            Some(height) => height,
            None => return Ok(Vec::new()),
        };

        let heights = crate::util::block_locator_heights(tip_height);
        let mut hashes = Vec::with_capacity(heights.len());

        for height in heights {
            if let Some(hash) = self.hash_by_height.zs_get(&height)? {
                hashes.push(hash);
            }
        }

        Ok(hashes)
    }

    pub fn tip(&self) -> Result<Option<(block::Height, block::Hash)>, BoxError> {
        if let Some((height_bytes, hash_bytes)) = self.hash_by_height.iter().rev().next().transpose()? {
            let height = block::Height::from_ivec(height_bytes)?;
            let hash = block::Hash::from_ivec(hash_bytes)?;

            Ok(Some((height, hash)))
        } else {
            Ok(None)
        }
    }

    pub fn depth(&self, hash: block::Hash) -> Result<Option<u32>, BoxError> {
        let height: block::Height = match self.height_by_hash.zs_get(&hash)? {
            Some(height) => height,
            None => return Ok(None),
        };

        let (tip_height, _) = self.tip()?.expect("tip must exist");

        Ok(Some(tip_height.0 - height.0))
    }

    pub fn block(&self, hash_or_height: HashOrHeight) -> Result<Option<Arc<Block>>, BoxError> {
        let height = match hash_or_height {
            HashOrHeight::Height(height) => height,
            HashOrHeight::Hash(hash) => match self.height_by_hash.zs_get(&hash)? {
                Some(height) => height,
                None => return Ok(None),
            },
        };

        Ok(self.block_by_height.zs_get(&height)?)
    }

    /// Returns the `transparent::Output` pointed to by the given
    /// `transparent::OutPoint` if it is present.
    pub fn utxo(
        &self,
        outpoint: &transparent::OutPoint,
    ) -> Result<Option<transparent::Output>, BoxError> {
        self.utxo_by_outpoint.zs_get(outpoint)
    }
}
