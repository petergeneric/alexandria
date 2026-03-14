//! Bounded crossbeam channel decoupling ingestion from indexing.
//!
//! Provides backpressure: if the queue is full, new snapshots are dropped with a warning.

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};

use crate::ingest::PageSnapshot;

pub struct IngestQueue {
    sender: Sender<PageSnapshot>,
    receiver: Receiver<PageSnapshot>,
}

impl IngestQueue {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = bounded(capacity);
        Self { sender, receiver }
    }

    pub fn sender(&self) -> &Sender<PageSnapshot> {
        &self.sender
    }

    pub fn receiver(&self) -> &Receiver<PageSnapshot> {
        &self.receiver
    }

    /// Try to enqueue a snapshot without blocking.
    /// Returns false if the queue is full.
    pub fn try_send(&self, snapshot: PageSnapshot) -> bool {
        match self.sender.try_send(snapshot) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                tracing::warn!("ingest queue full, dropping snapshot");
                false
            }
            Err(TrySendError::Disconnected(_)) => {
                tracing::error!("ingest queue disconnected");
                false
            }
        }
    }
}
