//! Strongly typed runtime commands and control actions.

use serde::{Deserialize, Serialize};

/// Simulation engine clock and execution commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum SimControlCommand {
    /// Resumes normal continuous simulation ticking.
    Play,
    /// Pauses simulation ticking.
    Pause,
    /// Advances simulation by exactly one fixed discrete step $\Delta t$.
    Step,
    /// Updates simulation clock execution pacing multiplier (e.g. 1.0, 2.0, 5.0).
    SetSpeedMultiplier { multiplier: f64 },
    /// Resets simulation state to initial project baseline.
    Reset,
}

/// PID controller operating modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PidModeDto {
    Manual,
    Auto,
    Cascade,
    RemoteCascade,
}

/// Transport-level tag value representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TagValueDto {
    Float(f64),
    Integer(i64),
    Bool(bool),
}

impl TagValueDto {
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Self::Float(v) => Some(v),
            Self::Integer(i) => Some(i as f64),
            Self::Bool(b) => Some(if b { 1.0 } else { 0.0 }),
        }
    }
}

/// Authoritative domain commands executed at discrete simulation tick boundaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeCommand {
    /// Controls simulation clock lifecycle and rate.
    SimControl(SimControlCommand),

    /// Writes a value directly to an input or manual override tag.
    WriteTag { tag: String, value: TagValueDto },

    /// Modifies the active setpoint (SP) of a closed-loop controller.
    SetSetpoint { loop_name: String, sp: f64 },

    /// Changes the operating mode (MAN/AUTO/CAS) of a PID controller.
    SetMode { loop_name: String, mode: PidModeDto },

    /// Modifies the manual output demand (OUT) when controller is in Manual mode.
    SetManualOutput { loop_name: String, output: f64 },

    /// Operator acknowledges an active unacknowledged alarm per ANSI/ISA-18.2.
    AcknowledgeAlarm { alarm_name: String },

    /// Temporarily shelves an alarm for the specified duration.
    ShelveAlarm {
        alarm_name: String,
        duration_seconds: f64,
    },

    /// Resets a latched protective trip or interlock after condition has cleared.
    ResetProtection { block_name: String },

    /// Scenario and fault injection controls.
    Scenario(ScenarioCommand),

    /// Custom equipment runtime controls.
    Equipment(EquipmentCommand),
}

/// Custom equipment runtime commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum EquipmentCommand {
    Define {
        definition_json: String,
    },
    Instantiate {
        definition_id: String,
        name: String,
        #[serde(default)]
        parameter_overrides: std::collections::BTreeMap<String, f64>,
        #[serde(default)]
        instance_id: Option<String>,
    },
    Start {
        instance_id: String,
    },
    Stop {
        instance_id: String,
    },
    ResetTrip {
        instance_id: String,
    },
    SetVariable {
        instance_id: String,
        var_name: String,
        value: f64,
    },
}

/// Scenario execution commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ScenarioCommand {
    Start,
    Pause,
    Resume,
    Abort {
        #[serde(default)]
        reason: String,
    },
    Reset,
    Load {
        scenario_json: String,
    },
    InjectFault {
        fault_json: String,
    },
    ClearFault {
        fault_id: String,
    },
}
