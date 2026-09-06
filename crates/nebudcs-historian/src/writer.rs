//! Dedicated background persistence worker thread with batching and transaction management.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};
use std::time::{Duration, Instant};

use crate::buffer::{HistorianReceiver, HistorianRecord};
use crate::error::{HistorianError, HistorianResult};
use crate::health::SharedHealthTracker;
use crate::point::HistorianPointConfig;
use crate::run::HistorianRunMeta;
use crate::sqlite::SqliteStorage;

/// Default configuration parameters for the persistence worker.
#[derive(Debug, Clone)]
pub struct WriterConfig {
    /// Maximum number of records to accumulate before forcing a commit.
    pub max_batch_size: usize,
    /// Maximum wait time before flushing an incomplete batch.
    pub flush_timeout: Duration,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 5_000,
            flush_timeout: Duration::from_millis(250),
        }
    }
}

/// Handle to the background persistence worker thread.
pub struct HistorianWriterHandle {
    join_handle: Option<JoinHandle<HistorianResult<usize>>>,
    shutdown_flag: Arc<AtomicBool>,
}

impl HistorianWriterHandle {
    /// Signals the background thread to shut down and waits for final flush completion.
    pub fn join(mut self) -> HistorianResult<usize> {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.join_handle.take() {
            handle
                .join()
                .map_err(|_| HistorianError::WorkerError("Writer thread panicked".to_string()))?
        } else {
            Ok(0)
        }
    }
}

impl Drop for HistorianWriterHandle {
    fn drop(&mut self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Spawns the dedicated persistence worker thread on a filesystem database path.
pub fn spawn_writer_thread<P: AsRef<Path>>(
    db_path: P,
    run_meta: HistorianRunMeta,
    points: Vec<HistorianPointConfig>,
    receiver: HistorianReceiver,
    config: WriterConfig,
) -> HistorianResult<HistorianWriterHandle> {
    let path_buf: PathBuf = db_path.as_ref().to_path_buf();
    let mut storage = SqliteStorage::open(&path_buf)?;

    // Register run and points upfront
    storage.insert_run(&run_meta)?;
    storage.insert_points(&run_meta.run_id, &points)?;

    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_clone = Arc::clone(&shutdown_flag);
    let run_id = run_meta.run_id.clone();
    let health = Arc::clone(receiver.health_tracker());

    let join_handle = Builder::new()
        .name("nebudcs-historian-writer".to_string())
        .spawn(move || run_worker_loop(storage, run_id, receiver, health, config, shutdown_clone))
        .map_err(|e| HistorianError::WorkerError(format!("Failed to spawn writer thread: {e}")))?;

    Ok(HistorianWriterHandle {
        join_handle: Some(join_handle),
        shutdown_flag,
    })
}

/// Spawns a writer thread using an in-memory database (for testing).
pub fn spawn_in_memory_writer_thread(
    mut storage: SqliteStorage,
    run_meta: HistorianRunMeta,
    points: Vec<HistorianPointConfig>,
    receiver: HistorianReceiver,
    config: WriterConfig,
) -> HistorianResult<HistorianWriterHandle> {
    storage.insert_run(&run_meta)?;
    storage.insert_points(&run_meta.run_id, &points)?;

    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_clone = Arc::clone(&shutdown_flag);
    let run_id = run_meta.run_id.clone();
    let health = Arc::clone(receiver.health_tracker());

    let join_handle = Builder::new()
        .name("nebudcs-historian-writer-mem".to_string())
        .spawn(move || run_worker_loop(storage, run_id, receiver, health, config, shutdown_clone))
        .map_err(|e| HistorianError::WorkerError(format!("Failed to spawn writer thread: {e}")))?;

    Ok(HistorianWriterHandle {
        join_handle: Some(join_handle),
        shutdown_flag,
    })
}

/// Internal loop executed on the dedicated persistence thread.
fn run_worker_loop(
    mut storage: SqliteStorage,
    run_id: String,
    receiver: HistorianReceiver,
    health: SharedHealthTracker,
    config: WriterConfig,
    shutdown_flag: Arc<AtomicBool>,
) -> HistorianResult<usize> {
    let mut total_written = 0;
    let mut batch: Vec<HistorianRecord> = Vec::with_capacity(config.max_batch_size);

    loop {
        // 1. Check if shutdown requested
        let should_shutdown = shutdown_flag.load(Ordering::Relaxed);

        // 2. Receive first record (blocking with timeout)
        match receiver.recv_timeout(config.flush_timeout) {
            Ok(rec) => {
                batch.push(rec);

                // Drain as many items as available up to batch size
                while batch.len() < config.max_batch_size {
                    match receiver.try_recv() {
                        Ok(item) => batch.push(item),
                        Err(_) => break,
                    }
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                // Timeout elapsed; flush any pending records in batch
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                // Sender dropped: drain all remaining records in channel
                while let Ok(item) = receiver.try_recv() {
                    batch.push(item);
                }
                if !batch.is_empty() {
                    let count = flush_batch(&mut storage, &run_id, &batch, &health)?;
                    total_written += count;
                }
                break;
            }
        }

        // 3. Flush batch if non-empty
        if !batch.is_empty() {
            let count = flush_batch(&mut storage, &run_id, &batch, &health)?;
            total_written += count;
            batch.clear();
        }

        // 4. Terminate if shutdown requested and queue is completely drained
        if should_shutdown && receiver.is_empty() {
            break;
        }
    }

    Ok(total_written)
}

/// Flushes an accumulated batch to SQLite within an atomic transaction.
fn flush_batch(
    storage: &mut SqliteStorage,
    run_id: &str,
    batch: &[HistorianRecord],
    health: &SharedHealthTracker,
) -> HistorianResult<usize> {
    let start_time = Instant::now();

    match storage.write_batch(run_id, batch) {
        Ok(count) => {
            let duration_us = start_time.elapsed().as_micros() as u64;
            // Update last flush sim time from latest sample if present
            let latest_sim_time = batch.iter().rev().find_map(|r| match r {
                HistorianRecord::Sample(s) => Some(s.sim_time),
                HistorianRecord::Event(e) => Some(e.sim_time),
                _ => None,
            });

            if let Some(t) = latest_sim_time {
                health.record_flush_success(t, duration_us);
            }
            Ok(count)
        }
        Err(e) => {
            health.record_sqlite_error();
            Err(e)
        }
    }
}
