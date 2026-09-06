//! Strongly typed error types for NebuDCS Historian.

use thiserror::Error;

use crate::point::HistorianPointId;
use nebudcs_types::TagId;

/// Result type for historian operations.
pub type HistorianResult<T> = Result<T, HistorianError>;

/// Errors that can occur within the historian subsystem.
#[derive(Debug, Error)]
pub enum HistorianError {
    /// SQLite database engine failure.
    #[error("SQLite database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Historian point is not registered.
    #[error("Historian point not found: {point_id:?}")]
    PointNotFound { point_id: HistorianPointId },

    /// Historian point already exists with given ID.
    #[error("Historian point already registered: {point_id:?}")]
    PointAlreadyRegistered { point_id: HistorianPointId },

    /// Tag is already bound to a historian point.
    #[error("Tag {tag_id} is already bound to historian point")]
    TagAlreadyBound { tag_id: TagId },

    /// Invalid time range specified in query.
    #[error("Invalid time range: start ({start:?}) must be <= end ({end:?})")]
    InvalidTimeRange { start: f64, end: f64 },

    /// Query returned no results or invalid run ID.
    #[error("Run not found: {run_id}")]
    RunNotFound { run_id: String },

    /// Persistence worker thread encountered a fatal panic or channel error.
    #[error("Historian worker error: {0}")]
    WorkerError(String),

    /// Security validation error (e.g. path traversal).
    #[error("Security validation failed: {0}")]
    SecurityError(String),

    /// IO error during database path operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error.
    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
