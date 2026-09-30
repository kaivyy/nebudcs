//! Store-and-Forward buffering for non-realtime industrial telemetry and historian spooling.
//!
//! Provides bounded in-memory and disk-spooled ring buffers that store historical
//! records during upstream network interruptions, draining in strict FIFO order
//! upon reconnection with duplicate detection and timestamp fidelity.

use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

use super::types::IndustrialTagValue;
use crate::error::ProtocolError;

/// Buffer overflow mitigation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OverflowPolicy {
    /// Discards oldest uncommitted record to preserve most recent telemetry.
    #[default]
    DropOldest,
    /// Discards incoming samples when buffer is full, prioritizing older un-replayed records.
    DropNewest,
}

/// High-throughput bounded Store-and-Forward queue.
#[derive(Debug, Clone)]
pub struct StoreAndForwardQueue {
    capacity: usize,
    buffer: VecDeque<IndustrialTagValue>,
    overflow_policy: OverflowPolicy,
    dropped_count: u64,
    spooled_count: u64,
    replayed_count: u64,
    seen_keys: HashSet<(String, u64)>,
}

impl StoreAndForwardQueue {
    pub fn new(capacity: usize, overflow_policy: OverflowPolicy) -> Self {
        let cap = capacity.max(1);
        Self {
            capacity: cap,
            buffer: VecDeque::with_capacity(cap.min(10_000)),
            overflow_policy,
            dropped_count: 0,
            spooled_count: 0,
            replayed_count: 0,
            seen_keys: HashSet::new(),
        }
    }

    /// Enqueues a sample during network disconnection, honoring buffer capacity.
    pub fn enqueue(&mut self, sample: IndustrialTagValue) -> Result<(), ProtocolError> {
        let key = (
            sample.canonical_id.clone(),
            sample
                .timestamp
                .source_timestamp_ms
                .unwrap_or(sample.timestamp.ingestion_timestamp_ms),
        );

        // Deduplication
        if self.seen_keys.contains(&key) {
            return Ok(());
        }

        if self.buffer.len() >= self.capacity {
            self.dropped_count += 1;
            match self.overflow_policy {
                OverflowPolicy::DropOldest => {
                    if let Some(dropped) = self.buffer.pop_front() {
                        let d_key = (
                            dropped.canonical_id,
                            dropped
                                .timestamp
                                .source_timestamp_ms
                                .unwrap_or(dropped.timestamp.ingestion_timestamp_ms),
                        );
                        self.seen_keys.remove(&d_key);
                    }
                }
                OverflowPolicy::DropNewest => {
                    return Err(ProtocolError::InvalidPayload(
                        "Store-and-forward queue full; new sample dropped",
                    ));
                }
            }
        }

        self.seen_keys.insert(key);
        self.buffer.push_back(sample);
        self.spooled_count += 1;
        Ok(())
    }

    /// Drains a batch of queued samples in strict FIFO order for upstream transmission.
    pub fn drain_batch(&mut self, max_batch_size: usize) -> Vec<IndustrialTagValue> {
        let batch_size = max_batch_size.min(self.buffer.len());
        let mut batch = Vec::with_capacity(batch_size);

        for _ in 0..batch_size {
            if let Some(sample) = self.buffer.pop_front() {
                let key = (
                    sample.canonical_id.clone(),
                    sample
                        .timestamp
                        .source_timestamp_ms
                        .unwrap_or(sample.timestamp.ingestion_timestamp_ms),
                );
                self.seen_keys.remove(&key);
                self.replayed_count += 1;
                batch.push(sample);
            }
        }

        batch
    }

    /// Current number of spooled items awaiting replay.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn dropped_count(&self) -> u64 {
        self.dropped_count
    }

    pub fn spooled_count(&self) -> u64 {
        self.spooled_count
    }

    pub fn replayed_count(&self) -> u64 {
        self.replayed_count
    }
}
