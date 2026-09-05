//! Strongly-typed simulation error definitions.

use thiserror::Error;

use nebudcs_types::{EquipmentId, NebuError, TagId};

/// Errors encountered during simulation setup, execution, state access, or clock control.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum SimError {
    #[error("invalid timestep {dt}: must be finite and strictly positive (> 0.0)")]
    InvalidTimestep { dt: f64 },

    #[error("invalid speed multiplier {multiplier}: must be finite and strictly positive (> 0.0)")]
    InvalidSpeedMultiplier { multiplier: f64 },

    #[error("equipment state for {equipment_id} not found in runtime state")]
    EquipmentNotFound { equipment_id: EquipmentId },

    #[error("tag {tag_id} not found in runtime state")]
    TagNotFound { tag_id: TagId },

    #[error("illegal state access: cannot read uninitialized buffer")]
    UninitializedBuffer,

    #[error("simulation lifecycle error: {reason}")]
    LifecycleError { reason: String },

    #[error("execution plan error: {reason}")]
    ExecutionPlanError { reason: String },

    #[error("system execution failed for system '{system_name}': {detail}")]
    SystemExecutionFailed { system_name: String, detail: String },
}

impl From<SimError> for NebuError {
    fn from(err: SimError) -> Self {
        NebuError::InvalidState {
            operation: "simulation".to_string(),
            detail: err.to_string(),
        }
    }
}
