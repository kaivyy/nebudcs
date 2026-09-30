//! NebuDCS Scenario & Fault Injection Subsystem (Phase 9).
//!
//! # Architecture & Responsibilities
//!
//! - **Scenario Definition**: Declarative, versioned specification of simulations, initial states, timelines, and objectives.
//! - **Deterministic Scheduling**: Events scheduled strictly by canonical [`SimulationTime`](nebudcs_types::SimulationTime).
//! - **Generic Fault Injection**: Non-refinery-specific failure models (sensor bias, drift, freeze, loss of signal, noise, actuator stuck/trip, equipment trip).
//! - **Deterministic Disturbances**: Steps, ramps, pulses, sinusoids, and seeded bounded pseudo-random noise.
//! - **Assessment Hooks**: Objective tracking, time-in-range evaluation, and automated scoring.
//! - **Historian Integration**: Non-blocking asynchronous event journal recording.

pub mod disturbance;
pub mod engine;
pub mod error;
pub mod fault;
pub mod model;
pub mod objective;
pub mod random;
pub mod timeline;
pub mod training;
pub mod validation;

pub use disturbance::{ActiveDisturbance, DisturbanceManager};
pub use engine::{ScenarioEngine, ScenarioStatus};
pub use error::{ScenarioError, ScenarioResult};
pub use fault::{ActiveFault, ActiveFaultSummary, FaultManager};
pub use model::{
    ComparisonOp, ControllerInitialState, DisturbanceDefinition, DisturbanceType,
    EquipmentInitialState, EventTrigger, FaultDefinition, FaultType, InitialConditions,
    ObjectiveCriteria, ObjectiveStatus, OperatorAction, ScenarioAction, ScenarioDefinition,
    ScenarioEvent, ScenarioObjective, ScenarioState, TagInitialValue, SCENARIO_SCHEMA_VERSION,
};
pub use objective::{ObjectiveEvaluator, ObjectiveTrackerItem};
pub use random::DeterministicPrng;
pub use timeline::TimelineScheduler;
pub use training::*;
pub use validation::{validate_fault, validate_scenario};
