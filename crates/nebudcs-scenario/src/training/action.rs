//! Structured trainee operator action tracking and evidence recording.

use serde::{Deserialize, Serialize};

/// Cognitive and operational error classification (Hollnagel CREAM / Rasmussen SRK).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperatorErrorClassification {
    /// Correct action executed in proper sequence within permissible operational band.
    Correct,
    /// Incorrect action or parameter value applied.
    Incorrect,
    /// Prohibited action violating safety interlocks, bypasses, or operating envelope.
    Forbidden,
    /// Correct action taken but exceeded permissible response time window.
    Late,
    /// Required intervention omitted or missed entirely.
    Missing,
    /// Partially correct intervention that failed to complete recovery.
    PartiallyCorrect,
    /// Legitimate action taken that was sub-optimal or introduced excessive oscillation.
    Inefficient,
}

/// Action type dispatched by a trainee operator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TraineeActionType {
    /// Controller setpoint change (PV units).
    SetSetpoint { loop_name: String, sp: f64 },
    /// Controller operational mode toggle ("MAN", "AUTO", "CAS", "RCAS").
    SetMode { loop_name: String, mode: String },
    /// Manual controller output adjustment (0.0 to 100.0%).
    SetManualOutput { loop_name: String, output: f64 },
    /// Direct tag write command.
    WriteTag { tag: String, value: f64 },
    /// ANSI/ISA-18.2 alarm state acknowledgement.
    AcknowledgeAlarm { alarm_name: String },
    /// Temporary alarm shelving with duration limit.
    ShelveAlarm {
        alarm_name: String,
        duration_sec: f64,
    },
    /// Protective trip manual reset command.
    ResetProtection,
    /// Safety Instrumented Function (SIF) dedicated manual reset command.
    ResetSif { sif_id: String },
    /// Direct equipment command ("start", "stop", etc.).
    EquipmentCommand {
        equipment_name: String,
        command: String,
    },
    /// Manual field valve position command (0.0 - 100.0%).
    FieldValveCommand { valve_id: String, position: f64 },
    /// Procedure or startup step acknowledgement.
    ProcedureStepAck { step_id: String },
    /// Immediate plant emergency stop (ESD) activation.
    EmergencyStop,
}

impl TraineeActionType {
    /// Returns the target entity identifier (tag, controller loop, equipment, or alarm name).
    pub fn target_entity(&self) -> &str {
        match self {
            Self::SetSetpoint { loop_name, .. } => loop_name,
            Self::SetMode { loop_name, .. } => loop_name,
            Self::SetManualOutput { loop_name, .. } => loop_name,
            Self::WriteTag { tag, .. } => tag,
            Self::AcknowledgeAlarm { alarm_name } => alarm_name,
            Self::ShelveAlarm { alarm_name, .. } => alarm_name,
            Self::ResetProtection => "PROTECTION_SYSTEM",
            Self::ResetSif { sif_id } => sif_id,
            Self::EquipmentCommand { equipment_name, .. } => equipment_name,
            Self::FieldValveCommand { valve_id, .. } => valve_id,
            Self::ProcedureStepAck { step_id } => step_id,
            Self::EmergencyStop => "ESD_SYSTEM",
        }
    }

    /// Returns the discriminator string for action type categorization.
    pub fn action_type_name(&self) -> &'static str {
        match self {
            Self::SetSetpoint { .. } => "SET_SETPOINT",
            Self::SetMode { .. } => "SET_MODE",
            Self::SetManualOutput { .. } => "SET_MANUAL_OUTPUT",
            Self::WriteTag { .. } => "WRITE_TAG",
            Self::AcknowledgeAlarm { .. } => "ACK_ALARM",
            Self::ShelveAlarm { .. } => "SHELVE_ALARM",
            Self::ResetProtection => "RESET_PROTECTION",
            Self::ResetSif { .. } => "RESET_SIF",
            Self::EquipmentCommand { .. } => "EQUIPMENT_COMMAND",
            Self::FieldValveCommand { .. } => "FIELD_VALVE_COMMAND",
            Self::ProcedureStepAck { .. } => "PROCEDURE_STEP_ACK",
            Self::EmergencyStop => "EMERGENCY_STOP",
        }
    }

    /// Returns associated numeric value if applicable.
    pub fn numeric_value(&self) -> Option<f64> {
        match self {
            Self::SetSetpoint { sp, .. } => Some(*sp),
            Self::SetManualOutput { output, .. } => Some(*output),
            Self::WriteTag { value, .. } => Some(*value),
            Self::FieldValveCommand { position, .. } => Some(*position),
            _ => None,
        }
    }
}

/// Execution outcome status of a trainee action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ActionStatus {
    /// Action verified and committed by authoritative simulation engine.
    Accepted,
    /// Action rejected at validation tier before reaching simulation core.
    Rejected { reason: String },
    /// Action execution attempted but simulation engine returned failure.
    Failed { error: String },
}

/// Comprehensive, immutable structured evidence record for an operator action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraineeActionRecord {
    /// Unique identifier for this discrete action event.
    pub action_id: String,
    /// Associated training session identifier.
    pub session_id: String,
    /// Exact simulation timestamp when action was submitted (seconds).
    pub sim_time_sec: f64,
    /// Wall-clock epoch timestamp (milliseconds).
    pub wall_clock_ms: i64,
    /// Trainee or operator identifier / username.
    pub actor: String,
    /// Target entity identifier.
    pub target_entity: String,
    /// Categorical action type string.
    pub action_type: String,
    /// Optional requested float value.
    pub requested_value: Option<f64>,
    /// Full strongly typed action payload.
    pub action: TraineeActionType,
    /// Command execution outcome.
    pub status: ActionStatus,
    /// Cognitive and operational error classification.
    #[serde(default)]
    pub error_classification: Option<OperatorErrorClassification>,
}

impl TraineeActionRecord {
    /// Creates a new accepted action evidence record.
    pub fn accepted(
        action_id: impl Into<String>,
        session_id: impl Into<String>,
        sim_time_sec: f64,
        wall_clock_ms: i64,
        actor: impl Into<String>,
        action: TraineeActionType,
    ) -> Self {
        let target_entity = action.target_entity().to_string();
        let action_type = action.action_type_name().to_string();
        let requested_value = action.numeric_value();
        Self {
            action_id: action_id.into(),
            session_id: session_id.into(),
            sim_time_sec,
            wall_clock_ms,
            actor: actor.into(),
            target_entity,
            action_type,
            requested_value,
            action,
            status: ActionStatus::Accepted,
            error_classification: Some(OperatorErrorClassification::Correct),
        }
    }

    /// Creates a new rejected action evidence record.
    pub fn rejected(
        action_id: impl Into<String>,
        session_id: impl Into<String>,
        sim_time_sec: f64,
        wall_clock_ms: i64,
        actor: impl Into<String>,
        action: TraineeActionType,
        reason: impl Into<String>,
    ) -> Self {
        let target_entity = action.target_entity().to_string();
        let action_type = action.action_type_name().to_string();
        let requested_value = action.numeric_value();
        Self {
            action_id: action_id.into(),
            session_id: session_id.into(),
            sim_time_sec,
            wall_clock_ms,
            actor: actor.into(),
            target_entity,
            action_type,
            requested_value,
            action,
            status: ActionStatus::Rejected {
                reason: reason.into(),
            },
            error_classification: Some(OperatorErrorClassification::Forbidden),
        }
    }
}
