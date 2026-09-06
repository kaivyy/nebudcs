//! Error types for the NebuDCS scenario engine.

use thiserror::Error;

/// Specific error conditions encountered during scenario parsing, validation, and execution.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ScenarioError {
    /// Scenario definition failed semantic or numeric validation.
    #[error("Scenario validation failed: {0}")]
    Validation(String),

    /// Target entity (tag, equipment, loop) was not found in runtime stores.
    #[error("Target not found: {0}")]
    TargetNotFound(String),

    /// Operation is invalid for the current scenario lifecycle state.
    #[error("Invalid scenario state: {0}")]
    InvalidState(String),

    /// Simulation engine error encountered during scenario execution.
    #[error("Simulation execution error: {0}")]
    Execution(String),

    /// Serialization or deserialization failure.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Incompatible schema version.
    #[error("Incompatible scenario schema version: expected {expected}, got {actual}")]
    IncompatibleVersion { expected: String, actual: String },
}

/// Convenience result type for scenario operations.
pub type ScenarioResult<T> = Result<T, ScenarioError>;
