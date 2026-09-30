//! Error types for the I/O Hardware Abstraction Layer (HAL).

use thiserror::Error;

/// Error conditions arising during I/O hardware abstraction operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum HalError {
    /// An invalid channel address string or component was encountered.
    #[error("invalid channel address format '{address}': {detail}")]
    InvalidAddress { address: String, detail: String },

    /// An address conflict was detected (duplicate address in same I/O system).
    #[error("duplicate channel address '{address}'")]
    DuplicateAddress { address: String },

    /// A duplicate channel ID was registered.
    #[error("duplicate channel ID '{id}'")]
    DuplicateChannelId { id: String },

    /// A module slot is already occupied in the target chassis.
    #[error("module slot {slot} is already occupied in chassis {chassis}")]
    ModuleSlotOccupied { chassis: u8, slot: u8 },

    /// Channel index exceeds the capacity of the module type.
    #[error("channel number {channel} is out of bounds for module type {module_type} (max {max_channels})")]
    ChannelOutOfBounds {
        module_type: String,
        channel: u8,
        max_channels: u8,
    },

    /// Invalid signal scaling parameters (e.g. raw_max == raw_min).
    #[error("invalid scaling configuration: {detail}")]
    InvalidScaling { detail: String },

    /// Zero span error where URV equals LRV.
    #[error("zero span in scaling: URV ({urv}) must not equal LRV ({lrv})")]
    ZeroSpan { lrv: f64, urv: f64 },

    /// Conflicting output ownership detected across controllers.
    #[error("conflicting output ownership for channel '{address}': already owned by controller {existing_controller}, attempted mapping by {new_controller}")]
    ConflictingOutputOwnership {
        address: String,
        existing_controller: String,
        new_controller: String,
    },

    /// Target rack was not found.
    #[error("rack {rack} not found in I/O system")]
    RackNotFound { rack: u8 },

    /// Target chassis was not found.
    #[error("chassis {chassis} not found in rack {rack}")]
    ChassisNotFound { rack: u8, chassis: u8 },

    /// Target module was not found.
    #[error("module at slot {slot} not found in chassis {chassis}")]
    ModuleNotFound { chassis: u8, slot: u8 },

    /// Target channel was not found.
    #[error("channel '{address}' not found in I/O system")]
    ChannelNotFound { address: String },

    /// Hardware failure state (rack unpowered, bus failure, ADC trip).
    #[error("hardware faulted for channel '{address}': {detail}")]
    HardwareFaulted { address: String, detail: String },

    /// Incompatible signal kind for operation (e.g. AI mapped to output).
    #[error(
        "incompatible signal type for channel '{address}': expected {expected}, found {found}"
    )]
    IncompatibleSignalType {
        address: String,
        expected: String,
        found: String,
    },

    /// Target controller reference was not found in control system.
    #[error("controller '{controller_id}' not found in control system")]
    ControllerNotFound { controller_id: String },

    /// General I/O system validation error.
    #[error("I/O system validation failed: {0}")]
    ValidationFailed(String),
}
