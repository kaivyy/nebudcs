//! Error types for the NebuDCS alarm engine.

use thiserror::Error;

/// Error conditions arising during alarm processing.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AlarmError {
    /// Alarm with given name not found.
    #[error("alarm not found: {alarm_name}")]
    AlarmNotFound { alarm_name: String },

    /// Invalid configuration for alarm block.
    #[error("invalid alarm configuration for {alarm_name}: {detail}")]
    InvalidConfiguration { alarm_name: String, detail: String },
}
