//! Deterministic simulation clock, double-buffered state, and execution pipeline for NebuDCS.
//!
//! # Responsibilities
//!
//! - [`SimulationClock`]: Fixed $\Delta t$, deterministic tick counting, pause/play, speed multiplier pacing.
//! - [`DoubleBufferedState`]: Strict double buffering where systems read State[N] and write State[N+1].
//! - [`ExecutionPlan`]: Topologically ordered acyclic and cyclic blocks derived from [`nebudcs_topology`].
//! - [`SimulationSystem`]: Extensible trait for simulation systems reading State[N] and writing State[N+1].
//! - [`SimulationEngine`]: High-level orchestrator running the commit pipeline.

pub mod clock;
pub mod engine;
pub mod error;
pub mod plan;
pub mod state;
pub mod system;

pub use clock::{ClockState, SimulationClock};
pub use engine::SimulationEngine;
pub use error::SimError;
pub use plan::{ExecutionBlock, ExecutionPlan};
pub use state::{DoubleBufferedState, EquipmentState, StateBuffer, StateReadView, StateWriteView};
pub use system::{SimulationSystem, StepContext};
