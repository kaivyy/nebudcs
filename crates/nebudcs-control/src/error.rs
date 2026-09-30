//! Error types for the NebuDCS control and automation engine.

use nebudcs_types::{EquipmentId, TagId};
use thiserror::Error;

/// Error conditions arising during control, instrumentation, or actuator operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ControlError {
    /// An invalid configuration was provided (e.g. URV == LRV or negative time constant).
    #[error("invalid configuration for {component}: {detail}")]
    InvalidConfiguration { component: String, detail: String },

    /// Tag with the specified ID was not found in runtime state.
    #[error("tag not found: {tag_id}")]
    TagNotFound { tag_id: TagId },

    /// Equipment with the specified ID was not found in runtime state.
    #[error("equipment not found: {equipment_id}")]
    EquipmentNotFound { equipment_id: EquipmentId },

    /// An invalid mode transition was requested.
    #[error("invalid mode transition from {from} to {to}: {reason}")]
    InvalidModeTransition {
        from: String,
        to: String,
        reason: String,
    },

    /// Actuator mechanical or dynamic error.
    #[error("actuator error for {actuator_id}: {detail}")]
    ActuatorError { actuator_id: String, detail: String },

    /// Protective safety interlock or ESD trip occurred.
    #[error("protection tripped: {reason}")]
    ProtectionTripped { reason: String },

    /// An invalid controller lifecycle transition was attempted.
    #[error("invalid controller lifecycle transition from {from} to {to}: {reason}")]
    InvalidLifecycleTransition {
        from: String,
        to: String,
        reason: String,
    },

    /// Controller task execution failed.
    #[error("task execution error for {task_id}: {detail}")]
    TaskExecutionError { task_id: String, detail: String },

    /// Controller watchdog timeout exceeded.
    #[error("controller watchdog timeout for {controller_id}: elapsed {elapsed_seconds:.3}s > limit {limit_seconds:.3}s")]
    WatchdogTimeout {
        controller_id: String,
        elapsed_seconds: f64,
        limit_seconds: f64,
    },

    /// Controller is in a faulted state.
    #[error("controller {controller_id} is faulted ({code}): {detail}")]
    ControllerFaulted {
        controller_id: String,
        code: String,
        detail: String,
    },

    /// Error within the I/O Hardware Abstraction Layer.
    #[error("HAL error: {0}")]
    Hal(#[from] crate::hal::HalError),

    /// Invalid parameter or configuration argument.
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    /// IEC 61131-3 compilation or execution error.
    #[error("IEC 61131-3 logic error: {0}")]
    IecError(String),
}
