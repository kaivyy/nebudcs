//! NebuDCS Phase 15: Training Studio domain subsystem.
//!
//! Subsystem architecture:
//! - [`session`]: Canonical TrainingSession model with strict lifecycle state machine.
//! - [`action`]: Structured evidence capture of operator / trainee interventions.
//! - [`evaluation`]: Deterministic, explainable grading engine consuming session evidence.
//! - [`replay`]: Read-only historical playback and state reconstruction frames.
//! - [`error`]: Structured error types for all training domain boundaries.

pub mod action;
pub mod error;
pub mod evaluation;
pub mod replay;
pub mod session;

pub use action::{
    ActionStatus, OperatorErrorClassification, TraineeActionRecord, TraineeActionType,
};
pub use error::{TrainingError, TrainingResult};
pub use evaluation::{
    evaluate_session, CriterionKind, CriterionResult, CriterionSeverity, EvaluationCriterion,
    EvaluationReport, SafetyTripPolicy, TrainingDebrief,
};
pub use replay::{ReplayFrame, ReplayMarker, ReplayMarkerKind, ReplaySessionData};
pub use session::{SessionLifecycleState, TrainingSession};
