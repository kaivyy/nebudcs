//! Canonical TrainingSession domain model and strict lifecycle state machine.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::error::{TrainingError, TrainingResult};

/// Strict lifecycle states for a training session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionLifecycleState {
    /// Created with metadata; project and scenario not yet loaded.
    Created,
    /// Preparing plant topology, initial states, and scenario parameters.
    Preparing,
    /// Project and scenario loaded and validated; ready to commence.
    Loaded,
    /// Ready to commence training execution.
    Ready,
    /// Simulation active; trainee interacting and events being scheduled.
    Running,
    /// Simulation execution temporarily suspended by instructor.
    Paused,
    /// Simulation interrupted by unhandled process hazard, fault, or trip.
    Faulted,
    /// Training session concluded normally; ready for evaluation report generation.
    Completed,
    /// Terminated prematurely by instructor or due to fatal training safety violation.
    Aborted,
}

impl std::fmt::Display for SessionLifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "CREATED"),
            Self::Preparing => write!(f, "PREPARING"),
            Self::Loaded => write!(f, "LOADED"),
            Self::Ready => write!(f, "READY"),
            Self::Running => write!(f, "RUNNING"),
            Self::Paused => write!(f, "PAUSED"),
            Self::Faulted => write!(f, "FAULTED"),
            Self::Completed => write!(f, "COMPLETED"),
            Self::Aborted => write!(f, "ABORTED"),
        }
    }
}

/// Canonical training session representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingSession {
    /// Unique session identifier (e.g. ULID).
    pub id: String,
    /// Target project identifier.
    pub project_id: String,
    /// Target scenario identifier.
    pub scenario_id: String,
    /// Specific scenario version locked to this session.
    #[serde(default)]
    pub scenario_version: Option<String>,
    /// Human-readable title of this training session.
    pub name: String,
    /// Detailed description of session goals.
    pub description: String,
    /// Identifier or name of the instructor overseeing the session.
    pub instructor_id: String,
    /// Identifier or name of the assigned trainee / operator.
    pub trainee_id: String,
    /// Current authoritative lifecycle state.
    pub state: SessionLifecycleState,
    /// UTC timestamp of session creation (epoch milliseconds).
    pub created_at_ms: i64,
    /// UTC timestamp when session was first transitioned to RUNNING.
    pub started_at_ms: Option<i64>,
    /// UTC timestamp when session transitioned to COMPLETED or ABORTED.
    pub completed_at_ms: Option<i64>,
    /// Exact simulation time when RUNNING commenced (seconds).
    pub simulation_start_time_sec: f64,
    /// Most recently recorded simulation time (seconds).
    pub current_simulation_time_sec: f64,
    /// Optional abort reason message if session was terminated early.
    pub abort_reason: Option<String>,
    /// Serialized initial runtime snapshot or state checkpoint.
    #[serde(default)]
    pub runtime_snapshot: Option<String>,
    /// Calculated final performance score (0.0 - 100.0%).
    #[serde(default)]
    pub score: Option<f64>,
    /// Completion outcome description (e.g. "SUCCESS_WITH_RECOVERY", "FAILED_CRITICAL_TRIP").
    #[serde(default)]
    pub completion_state: Option<String>,
    /// Arbitrary session-specific metadata tags.
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl TrainingSession {
    /// Creates a new training session in the CREATED lifecycle state.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        instructor_id: impl Into<String>,
        trainee_id: impl Into<String>,
        wall_clock_ms: i64,
    ) -> Self {
        Self {
            id: id.into(),
            project_id: String::new(),
            scenario_id: String::new(),
            scenario_version: None,
            name: name.into(),
            description: description.into(),
            instructor_id: instructor_id.into(),
            trainee_id: trainee_id.into(),
            state: SessionLifecycleState::Created,
            created_at_ms: wall_clock_ms,
            started_at_ms: None,
            completed_at_ms: None,
            simulation_start_time_sec: 0.0,
            current_simulation_time_sec: 0.0,
            abort_reason: None,
            runtime_snapshot: None,
            score: None,
            completion_state: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Evaluates whether a transition from the current state to the target state is allowed.
    pub fn can_transition_to(&self, target: SessionLifecycleState) -> bool {
        match (self.state, target) {
            // Self-transitions are no-ops
            (s, t) if s == t => true,
            // Created -> Preparing | Loaded | Ready
            (SessionLifecycleState::Created, SessionLifecycleState::Preparing) => true,
            (SessionLifecycleState::Created, SessionLifecycleState::Loaded) => true,
            (SessionLifecycleState::Created, SessionLifecycleState::Ready) => true,
            // Preparing -> Ready | Loaded
            (SessionLifecycleState::Preparing, SessionLifecycleState::Ready) => true,
            (SessionLifecycleState::Preparing, SessionLifecycleState::Loaded) => true,
            // Loaded <-> Ready
            (SessionLifecycleState::Loaded, SessionLifecycleState::Ready) => true,
            (SessionLifecycleState::Ready, SessionLifecycleState::Loaded) => true,
            // Loaded | Ready -> Running
            (SessionLifecycleState::Loaded, SessionLifecycleState::Running) => true,
            (SessionLifecycleState::Ready, SessionLifecycleState::Running) => true,
            // Running <-> Paused
            (SessionLifecycleState::Running, SessionLifecycleState::Paused) => true,
            (SessionLifecycleState::Paused, SessionLifecycleState::Running) => true,
            // Running -> Faulted -> Running | Paused
            (SessionLifecycleState::Running, SessionLifecycleState::Faulted) => true,
            (SessionLifecycleState::Faulted, SessionLifecycleState::Running) => true,
            (SessionLifecycleState::Faulted, SessionLifecycleState::Paused) => true,
            // Running or Paused or Faulted -> Completed
            (SessionLifecycleState::Running, SessionLifecycleState::Completed) => true,
            (SessionLifecycleState::Paused, SessionLifecycleState::Completed) => true,
            (SessionLifecycleState::Faulted, SessionLifecycleState::Completed) => true,
            // Any non-terminal state -> Aborted
            (SessionLifecycleState::Created, SessionLifecycleState::Aborted) => true,
            (SessionLifecycleState::Preparing, SessionLifecycleState::Aborted) => true,
            (SessionLifecycleState::Loaded, SessionLifecycleState::Aborted) => true,
            (SessionLifecycleState::Ready, SessionLifecycleState::Aborted) => true,
            (SessionLifecycleState::Running, SessionLifecycleState::Aborted) => true,
            (SessionLifecycleState::Paused, SessionLifecycleState::Aborted) => true,
            (SessionLifecycleState::Faulted, SessionLifecycleState::Aborted) => true,
            // Reset transition: Loaded, Ready, Paused, Running, Faulted can reset back to Loaded or Ready
            (
                SessionLifecycleState::Running,
                SessionLifecycleState::Loaded | SessionLifecycleState::Ready,
            ) => true,
            (
                SessionLifecycleState::Paused,
                SessionLifecycleState::Loaded | SessionLifecycleState::Ready,
            ) => true,
            (
                SessionLifecycleState::Faulted,
                SessionLifecycleState::Loaded | SessionLifecycleState::Ready,
            ) => true,
            // All other transitions are forbidden
            _ => false,
        }
    }

    /// Transitions to the target state, validating lifecycle rules.
    pub fn transition_to(
        &mut self,
        target: SessionLifecycleState,
        sim_time: f64,
        wall_clock_ms: i64,
    ) -> TrainingResult<()> {
        if !self.can_transition_to(target) {
            return Err(TrainingError::InvalidStateTransition {
                from: self.state.to_string(),
                to: target.to_string(),
                reason: format!("Illegal transition from {} to {}", self.state, target),
            });
        }

        match target {
            SessionLifecycleState::Running => {
                if self.started_at_ms.is_none() {
                    self.started_at_ms = Some(wall_clock_ms);
                    self.simulation_start_time_sec = sim_time;
                }
            }
            SessionLifecycleState::Completed | SessionLifecycleState::Aborted
                if self.completed_at_ms.is_none() =>
            {
                self.completed_at_ms = Some(wall_clock_ms);
            }
            _ => {}
        }

        self.state = target;
        self.current_simulation_time_sec = sim_time;
        Ok(())
    }

    /// Associates project and scenario identifiers and transitions to LOADED / READY.
    pub fn load_project_and_scenario(
        &mut self,
        project_id: impl Into<String>,
        scenario_id: impl Into<String>,
        sim_time: f64,
        wall_clock_ms: i64,
    ) -> TrainingResult<()> {
        if self.state != SessionLifecycleState::Created
            && self.state != SessionLifecycleState::Loaded
            && self.state != SessionLifecycleState::Preparing
            && self.state != SessionLifecycleState::Ready
        {
            return Err(TrainingError::InvalidStateTransition {
                from: self.state.to_string(),
                to: SessionLifecycleState::Loaded.to_string(),
                reason: "Can only load project and scenario in CREATED, PREPARING, LOADED or READY state".to_string(),
            });
        }

        let pid = project_id.into();
        let sid = scenario_id.into();
        if pid.trim().is_empty() {
            return Err(TrainingError::ProjectNotFound(
                "Project ID cannot be empty".to_string(),
            ));
        }
        if sid.trim().is_empty() {
            return Err(TrainingError::ScenarioNotFound(
                "Scenario ID cannot be empty".to_string(),
            ));
        }

        self.project_id = pid;
        self.scenario_id = sid;
        self.state = SessionLifecycleState::Loaded;
        self.current_simulation_time_sec = sim_time;
        self.created_at_ms = wall_clock_ms;
        Ok(())
    }

    /// Transitions session through PREPARING to READY state.
    pub fn prepare_session(&mut self, sim_time: f64, wall_clock_ms: i64) -> TrainingResult<()> {
        if self.state == SessionLifecycleState::Created
            || self.state == SessionLifecycleState::Loaded
        {
            self.state = SessionLifecycleState::Preparing;
            self.current_simulation_time_sec = sim_time;
            self.state = SessionLifecycleState::Ready;
            self.created_at_ms = wall_clock_ms;
            Ok(())
        } else if self.state == SessionLifecycleState::Preparing {
            self.state = SessionLifecycleState::Ready;
            self.current_simulation_time_sec = sim_time;
            Ok(())
        } else if self.state == SessionLifecycleState::Ready {
            Ok(())
        } else {
            Err(TrainingError::InvalidStateTransition {
                from: self.state.to_string(),
                to: SessionLifecycleState::Ready.to_string(),
                reason: format!("Cannot prepare session currently in {}", self.state),
            })
        }
    }

    /// Resets the session back to LOADED baseline, clearing started/completed timestamps.
    pub fn reset_session(&mut self) -> TrainingResult<()> {
        if self.state == SessionLifecycleState::Completed
            || self.state == SessionLifecycleState::Aborted
        {
            return Err(TrainingError::InvalidStateTransition {
                from: self.state.to_string(),
                to: SessionLifecycleState::Loaded.to_string(),
                reason: "Cannot reset a terminal session (COMPLETED or ABORTED)".to_string(),
            });
        }

        self.state = SessionLifecycleState::Loaded;
        self.started_at_ms = None;
        self.completed_at_ms = None;
        self.simulation_start_time_sec = 0.0;
        self.current_simulation_time_sec = 0.0;
        self.abort_reason = None;
        Ok(())
    }

    /// Updates the current simulation time on the session.
    pub fn update_sim_time(&mut self, sim_time: f64) {
        self.current_simulation_time_sec = sim_time;
    }
}
