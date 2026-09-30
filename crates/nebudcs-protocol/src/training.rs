//! Client-server protocol DTOs for Phase 15: Training Studio, Instructor Station, Evaluation & Replay.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Lightweight summary of a training session for listing tables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingSessionSummaryDto {
    pub id: String,
    pub name: String,
    pub project_id: String,
    pub scenario_id: String,
    pub instructor_id: String,
    pub trainee_id: String,
    pub state: String,
    pub current_sim_time_sec: f64,
    pub created_at_ms: i64,
}

/// Cognitive and operational error classification DTO for trainee interventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperatorErrorClassificationDto {
    Correct,
    Incorrect,
    Forbidden,
    Late,
    Missing,
    PartiallyCorrect,
    Inefficient,
}

/// Safety trip policy defining how an SIF trip impacts the trainee's assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SafetyTripPolicyDto {
    Expected,
    Avoidable,
    Acceptable,
    Failure,
}

/// Structured training debrief DTO summarizing performance, safety adherence, and recommendations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingDebriefDto {
    pub process_performance: String,
    pub operator_performance: String,
    pub safety_assessment: String,
    pub outcome_verdict: String,
    pub recommendations: Vec<String>,
}

/// Declarative branching path descriptor for scenario orchestration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioBranchDto {
    pub branch_id: String,
    pub name: String,
    pub description: String,
    pub triggered: bool,
    pub condition_summary: String,
}

/// Detailed descriptor of a training session including timeline timestamps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingSessionDetailDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub project_id: String,
    pub scenario_id: String,
    pub instructor_id: String,
    pub trainee_id: String,
    pub state: String,
    pub created_at_ms: i64,
    pub started_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    pub simulation_start_time_sec: f64,
    pub current_simulation_time_sec: f64,
    pub abort_reason: Option<String>,
    #[serde(default)]
    pub scenario_version: Option<String>,
    #[serde(default)]
    pub runtime_snapshot: Option<String>,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub completion_state: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

/// Request payload to instantiate a new training session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateTrainingSessionRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub scenario_id: Option<String>,
    pub instructor_id: String,
    pub trainee_id: String,
}

/// Request payload to associate and load project and scenario models into a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadSessionProjectRequest {
    pub project_id: String,
    pub scenario_id: String,
}

/// Request payload to terminate a session early.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbortSessionRequest {
    pub reason: String,
}

/// Structured evidence record of a trainee intervention.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraineeActionDto {
    pub action_id: String,
    pub session_id: String,
    pub sim_time_sec: f64,
    pub wall_clock_ms: i64,
    pub actor: String,
    pub target_entity: String,
    pub action_type: String,
    pub requested_value: Option<f64>,
    pub status: String,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default)]
    pub error_classification: Option<OperatorErrorClassificationDto>,
}

/// Submission payload for a trainee operator action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordTraineeActionRequest {
    pub session_id: String,
    pub actor: String,
    pub target_entity: String,
    pub action_type: String,
    #[serde(default)]
    pub requested_value: Option<f64>,
    #[serde(default)]
    pub command_payload: Option<serde_json::Value>,
}

/// Individual criterion evaluation result with explainable evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriterionResultDto {
    pub criterion_id: String,
    pub title: String,
    pub passed: bool,
    pub score_awarded: u32,
    pub score_possible: u32,
    pub penalty_deducted: u32,
    pub explanation: String,
    #[serde(default)]
    pub evidence: BTreeMap<String, String>,
    #[serde(default)]
    pub error_classification: Option<OperatorErrorClassificationDto>,
    #[serde(default)]
    pub safety_trip_assessment: Option<String>,
}

/// Comprehensive, explainable assessment report for a completed session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationReportDto {
    pub session_id: String,
    pub scenario_id: String,
    pub trainee_id: String,
    pub total_score: u32,
    pub max_possible_score: u32,
    pub percentage: f64,
    pub passed: bool,
    pub results: Vec<CriterionResultDto>,
    pub evaluated_at_ms: i64,
    #[serde(default)]
    pub debrief: Option<TrainingDebriefDto>,
}

/// Chronological marker positioned along historical playback timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayMarkerDto {
    pub marker_id: String,
    pub sim_time_sec: f64,
    pub kind: String,
    pub source_id: String,
    pub label: String,
    pub details: String,
    pub severity: u8,
}

/// Point-in-time state reconstruction frame for presentation on replay console.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayFrameDto {
    pub sim_time_sec: f64,
    pub tags: BTreeMap<String, f64>,
    pub active_alarms: Vec<String>,
    pub active_trips: Vec<String>,
    pub recent_actions: Vec<TraineeActionDto>,
}

/// High-level summary of replay dataset for scrubber bounds and marker track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplaySessionSummaryDto {
    pub session_id: String,
    pub project_id: String,
    pub scenario_id: String,
    pub start_sim_time_sec: f64,
    pub end_sim_time_sec: f64,
    pub duration_sec: f64,
    pub markers: Vec<ReplayMarkerDto>,
}

/// Summary descriptor of a declarative scenario for scenario catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioSummaryDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub compatible_project_id: Option<String>,
    pub duration_limit_sec: Option<f64>,
    pub event_count: usize,
    pub objective_count: usize,
}

/// Comprehensive declarative scenario representation for the Scenario Editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioAuthoringDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub compatible_project_id: Option<String>,
    pub duration_limit_sec: Option<f64>,
    pub random_seed: u64,
    pub initial_conditions_json: String,
    pub timeline_json: String,
    pub objectives_json: String,
    pub criteria_json: String,
}

/// Semantic validation outcome for an authored scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioValidationResultDto {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}
