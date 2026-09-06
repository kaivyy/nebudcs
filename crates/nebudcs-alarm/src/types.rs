//! Alarm lifecycle states, types, and severities conforming to ANSI/ISA-18.2.

use serde::{Deserialize, Serialize};

/// Type/category of process alarm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlarmType {
    /// High process limit.
    Hi,
    /// High-High emergency limit.
    HiHi,
    /// Low process limit.
    Lo,
    /// Low-Low emergency limit.
    LoLo,
    /// Deviation from setpoint.
    Deviation,
    /// Rate of change exceeded.
    RateOfChange,
    /// Instrument/sensor bad quality.
    BadQuality,
}

/// Alarm criticality severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AlarmSeverity {
    /// Informational / diagnostic.
    Diagnostic = 1,
    /// Low severity / advisory.
    Advisory = 2,
    /// Moderate severity / warning.
    Warning = 3,
    /// Critical emergency / ESD risk.
    Critical = 4,
}

/// ANSI/ISA-18.2 compliant 5-state alarm lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlarmState {
    /// Normal operating condition; within limits.
    Normal,
    /// Limit violated; alarm active and unacknowledged by operator.
    ActiveUnack,
    /// Limit violated; acknowledged by operator.
    ActiveAck,
    /// Process condition returned within deadband, but operator has not acknowledged.
    Cleared,
    /// Temporarily suppressed/shelved by authorized operator.
    Shelved,
}
