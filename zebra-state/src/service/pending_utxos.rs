use std::collections::HashMap;
use std::future::Future;

use tokio::sync::broadcast;

use zebra_chain::transparent;

use crate::{BoxError, Response, Utxo};

#[derive(Debug, Default)]
pub struct PendingUtxos(HashMap<transparent::OutPoint, broadcast::Sender<Utxo>>);

impl PendingUtxos {
    /// Returns a future that will resolve to the `transparent::Output` pointed
    /// to by the given `transparent::OutPoint` when it is available.
    pub fn queue(
        &mut self,
        outpoint: transparent::OutPoint,
    ) -> impl Future<Output = Result<Response, BoxError>> {
        let mut receiver = self
            .0
            .entry(outpoint)
            .or_insert_with(|| {
                let (sender, _) = broadcast::channel(1);
                sender
            })
            .subscribe();

        async move {
            receiver
                .recv()
                .await
                .map(Response::Utxo)
                .map_err(BoxError::from)
        }
    }

    /// Notify all requests waiting for the [`Utxo`] pointed to by the given
    /// [`transparent::OutPoint`] that the [`Utxo`] has arrived.
    pub fn respond(&mut self, outpoint: &transparent::OutPoint, utxo: Utxo) {
        if let Some(sender) = self.0.remove(outpoint) {
            // Adding the outpoint as a field lets us crossreference
            // with the trace of the verification that made the request.
            tracing::trace!(?outpoint, "found pending UTXO");
            let _ = sender.send(utxo);
        }
    }

    /// Check the list of pending UTXO requests against the supplied UTXO index.
    pub fn check_against(&mut self, utxos: &HashMap<transparent::OutPoint, Utxo>) {
        for (outpoint, utxo) in utxos.iter() {
            if let Some(sender) = self.0.remove(outpoint) {
                tracing::trace!(?outpoint, "found pending UTXO");
                let _ = sender.send(utxo.clone());
            }
        }
    }

    /// Scan the set of waiting utxo requests for channels where all receivers
    /// have been dropped and remove the corresponding sender.
    pub fn prune(&mut self) {
        self.0.retain(|_, chan| chan.receiver_count() > 0);
    }

    /// Returns the number of utxos that are being waited on.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}
