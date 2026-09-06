//! Bounded lock-free ring buffer channel decoupling simulation from persistence.

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use nebudcs_types::SimulationTime;
use std::sync::Arc;

use crate::event::HistoricalEvent;
use crate::health::{HistorianHealth, HistorianHealthTracker, SharedHealthTracker};
use crate::sample::HistoricalSample;

/// Default bounded queue capacity ($100,000$ entries $\approx 3.2\text{ MB}$ RAM).
pub const DEFAULT_QUEUE_CAPACITY: usize = 100_000;

/// Atomic record passed across the non-blocking channel to the background writer.
#[derive(Debug, Clone, PartialEq)]
pub enum HistorianRecord {
    /// Numerical or discrete timeseries point.
    Sample(HistoricalSample),
    /// Discrete DCS event (alarm, trip, mode change, intervention).
    Event(HistoricalEvent),
    /// Explicit data loss marker emitted after buffer saturation.
    DataLossGap {
        sim_time: SimulationTime,
        seq: u64,
        dropped_count: u64,
    },
    /// Flush synchronization barrier (used during shutdown or testing).
    FlushSync,
}

/// Error returned when non-blocking send fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistorianSendError {
    /// Queue is currently at capacity.
    Full,
    /// Persistence worker thread has disconnected.
    Disconnected,
}

/// Producer handle held by the simulation engine or sampling system.
///
/// All send operations are strictly non-blocking.
#[derive(Debug, Clone)]
pub struct HistorianSender {
    sender: Sender<HistorianRecord>,
    health: SharedHealthTracker,
    capacity: usize,
}

impl HistorianSender {
    /// Attempts to enqueue a record without blocking.
    ///
    /// If the queue is at capacity:
    /// - Drops the record immediately.
    /// - Increments the dropped sample counter.
    /// - Returns `Err(HistorianSendError::Full)`.
    ///
    /// The simulation hot loop never blocks or panics on backpressure.
    #[inline]
    pub fn try_send(&self, record: HistorianRecord) -> Result<(), HistorianSendError> {
        match self.sender.try_send(record) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                self.health.record_drop();
                Err(HistorianSendError::Full)
            }
            Err(TrySendError::Disconnected(_)) => Err(HistorianSendError::Disconnected),
        }
    }

    /// Number of items currently queued in the channel.
    #[inline]
    pub fn len(&self) -> usize {
        self.sender.len()
    }

    /// Returns `true` if the channel is currently empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.sender.is_empty()
    }

    /// Total capacity of this bounded channel.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Current operational health and queue metrics.
    pub fn health(&self) -> HistorianHealth {
        self.health.snapshot(self.sender.len())
    }

    /// Internal access to health tracker.
    pub fn health_tracker(&self) -> &SharedHealthTracker {
        &self.health
    }
}

/// Consumer handle held exclusively by the background persistence worker thread.
#[derive(Debug)]
pub struct HistorianReceiver {
    receiver: Receiver<HistorianRecord>,
    health: SharedHealthTracker,
    capacity: usize,
}

impl HistorianReceiver {
    /// Receives a record, blocking with a timeout.
    pub fn recv_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<HistorianRecord, crossbeam_channel::RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }

    /// Attempts to receive a record without blocking.
    pub fn try_recv(&self) -> Result<HistorianRecord, crossbeam_channel::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Number of items currently queued.
    #[inline]
    pub fn len(&self) -> usize {
        self.receiver.len()
    }

    /// Returns `true` if empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.receiver.is_empty()
    }

    /// Internal access to health tracker.
    pub fn health_tracker(&self) -> &SharedHealthTracker {
        &self.health
    }

    /// Total capacity.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

/// Creates a new bounded historian channel and atomic health tracker.
pub fn create_historian_channel(capacity: usize) -> (HistorianSender, HistorianReceiver) {
    let cap = if capacity == 0 {
        DEFAULT_QUEUE_CAPACITY
    } else {
        capacity
    };
    let (tx, rx) = bounded(cap);
    let health = Arc::new(HistorianHealthTracker::new(cap));

    (
        HistorianSender {
            sender: tx,
            health: Arc::clone(&health),
            capacity: cap,
        },
        HistorianReceiver {
            receiver: rx,
            health,
            capacity: cap,
        },
    )
}
