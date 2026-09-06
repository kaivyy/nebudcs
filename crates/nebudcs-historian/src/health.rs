use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use nebudcs_types::SimulationTime;
use serde::{Deserialize, Serialize};

/// Snapshot of historian health and operational status.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HistorianHealth {
    /// True if the worker is actively writing and queue capacity is normal.
    pub is_healthy: bool,
    /// Number of samples currently waiting in the in-memory queue.
    pub queued_samples: usize,
    /// Maximum capacity of the bounded queue.
    pub queue_capacity: usize,
    /// Total count of samples dropped due to buffer exhaustion.
    pub dropped_samples_total: u64,
    /// Simulation time of the most recent successful SQLite flush.
    pub last_flush_sim_time: SimulationTime,
    /// Time taken to execute the most recent SQLite batch commit in microseconds.
    pub last_flush_duration_us: u64,
    /// Number of SQLite transaction errors encountered.
    pub sqlite_error_count: u64,
}

/// Thread-safe lock-free atomic tracker for historian health.
#[derive(Debug)]
pub struct HistorianHealthTracker {
    queue_capacity: usize,
    dropped_samples: AtomicU64,
    last_flush_sim_time_bits: AtomicU64,
    last_flush_duration_us: AtomicU64,
    sqlite_errors: AtomicU64,
    is_healthy: AtomicBool,
}

impl HistorianHealthTracker {
    /// Creates a new health tracker for a given queue capacity.
    pub fn new(queue_capacity: usize) -> Self {
        Self {
            queue_capacity,
            dropped_samples: AtomicU64::new(0),
            last_flush_sim_time_bits: AtomicU64::new(0.0f64.to_bits()),
            last_flush_duration_us: AtomicU64::new(0),
            sqlite_errors: AtomicU64::new(0),
            is_healthy: AtomicBool::new(true),
        }
    }

    /// Increments the count of dropped samples by 1.
    #[inline]
    pub fn record_drop(&self) {
        self.dropped_samples.fetch_add(1, Ordering::Relaxed);
        self.is_healthy.store(false, Ordering::Relaxed);
    }

    /// Increments SQLite error counter.
    #[inline]
    pub fn record_sqlite_error(&self) {
        self.sqlite_errors.fetch_add(1, Ordering::Relaxed);
        self.is_healthy.store(false, Ordering::Relaxed);
    }

    /// Records a successful batch flush.
    #[inline]
    pub fn record_flush_success(&self, sim_time: SimulationTime, duration_us: u64) {
        self.last_flush_sim_time_bits
            .store(sim_time.seconds().to_bits(), Ordering::Relaxed);
        self.last_flush_duration_us
            .store(duration_us, Ordering::Relaxed);
        self.is_healthy.store(true, Ordering::Relaxed);
    }

    /// Returns a health snapshot given the current queue depth.
    pub fn snapshot(&self, current_queued: usize) -> HistorianHealth {
        let sim_time_f64 = f64::from_bits(self.last_flush_sim_time_bits.load(Ordering::Relaxed));
        let sim_time = SimulationTime::try_new(sim_time_f64).unwrap_or(SimulationTime::ZERO);

        HistorianHealth {
            is_healthy: self.is_healthy.load(Ordering::Relaxed),
            queued_samples: current_queued,
            queue_capacity: self.queue_capacity,
            dropped_samples_total: self.dropped_samples.load(Ordering::Relaxed),
            last_flush_sim_time: sim_time,
            last_flush_duration_us: self.last_flush_duration_us.load(Ordering::Relaxed),
            sqlite_error_count: self.sqlite_errors.load(Ordering::Relaxed),
        }
    }
}

/// Shared handle to the health tracker.
pub type SharedHealthTracker = Arc<HistorianHealthTracker>;
