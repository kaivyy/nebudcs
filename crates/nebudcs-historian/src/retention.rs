//! Retention policies and chunked database pruning.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::HistorianResult;

/// Configuration defining how long historical timeseries data is retained.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "mode")]
pub enum RetentionPolicy {
    /// Keeps historical records for the last $T_{window}$ simulation seconds.
    SlidingSimTime {
        /// Retention window length in simulation seconds.
        window_sim_seconds: f64,
    },
    /// Triggers cleanup when total database file exceeds size threshold.
    MaxDatabaseBytes {
        /// Maximum allowed file size in bytes.
        max_bytes: u64,
    },
    /// Retains all samples unconditionally (e.g. for training scenarios).
    #[default]
    RetainEntireRun,
}

/// Prunes historical samples older than `cutoff_sim_time` in bounded chunks.
///
/// Executes bounded deletion batches to avoid long SQLite exclusive locks.
pub fn prune_older_than(
    conn: &Connection,
    run_id: &str,
    cutoff_sim_time: f64,
    chunk_limit: usize,
) -> HistorianResult<usize> {
    if cutoff_sim_time <= 0.0 || chunk_limit == 0 {
        return Ok(0);
    }

    let mut total_deleted = 0;

    // 1. Prune analog history in bounded chunks using composite primary key selection
    loop {
        let deleted = conn.execute(
            "DELETE FROM history_analog 
             WHERE (run_id, point_id, sim_time, seq) IN (
                 SELECT run_id, point_id, sim_time, seq 
                 FROM history_analog 
                 WHERE run_id = ?1 AND sim_time < ?2 
                 LIMIT ?3
             );",
            params![run_id, cutoff_sim_time, chunk_limit],
        )?;

        total_deleted += deleted;
        if deleted < chunk_limit {
            break;
        }
    }

    // 2. Prune discrete history in bounded chunks
    loop {
        let deleted = conn.execute(
            "DELETE FROM history_discrete 
             WHERE (run_id, point_id, sim_time, seq) IN (
                 SELECT run_id, point_id, sim_time, seq 
                 FROM history_discrete 
                 WHERE run_id = ?1 AND sim_time < ?2 
                 LIMIT ?3
             );",
            params![run_id, cutoff_sim_time, chunk_limit],
        )?;

        total_deleted += deleted;
        if deleted < chunk_limit {
            break;
        }
    }

    Ok(total_deleted)
}

/// Prunes discrete events older than `cutoff_sim_time` in bounded chunks.
pub fn prune_events_older_than(
    conn: &Connection,
    run_id: &str,
    cutoff_sim_time: f64,
    chunk_limit: usize,
) -> HistorianResult<usize> {
    if cutoff_sim_time <= 0.0 || chunk_limit == 0 {
        return Ok(0);
    }

    let mut total_deleted = 0;

    loop {
        let deleted = conn.execute(
            "DELETE FROM history_events 
             WHERE (run_id, sim_time, seq) IN (
                 SELECT run_id, sim_time, seq 
                 FROM history_events 
                 WHERE run_id = ?1 AND sim_time < ?2 
                 LIMIT ?3
             );",
            params![run_id, cutoff_sim_time, chunk_limit],
        )?;

        total_deleted += deleted;
        if deleted < chunk_limit {
            break;
        }
    }

    Ok(total_deleted)
}
