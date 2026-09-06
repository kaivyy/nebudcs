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
}
