//! Structured error types for training sessions, evaluations, and replays.

use thiserror::Error;

/// Error conditions encountered during training session operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum TrainingError {
    /// Invalid lifecycle state transition attempt.
    #[error("Invalid session lifecycle transition from {from} to {to}: {reason}")]
    InvalidStateTransition {
        from: String,
        to: String,
        reason: String,
    },

    /// Training session not found.
    #[error("Training session not found: {0}")]
    SessionNotFound(String),

    /// Referenced scenario not found.
    #[error("Scenario not found: {0}")]
    ScenarioNotFound(String),

    /// Referenced project not found.
    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    /// Trainee action validation failed.
    #[error("Trainee action rejected: {0}")]
    ActionRejected(String),

    /// Evaluation failure or malformed criteria.
    #[error("Evaluation error: {0}")]
    Evaluation(String),

    /// Replay failure or missing historical data.
    #[error("Replay error: {0}")]
    Replay(String),

    /// Serialization / deserialization failure.
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Convenience result type for training operations.
pub type TrainingResult<T> = Result<T, TrainingError>;
