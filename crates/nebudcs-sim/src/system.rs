//! Extensible simulation system trait.
//!
//! # System Invariant
//!
//! Systems participate in simulation ticks by reading exclusively from [`StateReadView`] (State[N])
//! and writing outputs exclusively to [`StateWriteView`] (State[N+1]).
//!
//! This ensures no system can observe uncommitted or partially updated state during tick execution.

use crate::error::SimError;
use crate::state::{StateReadView, StateWriteView};

/// Context passed to a simulation system during tick execution.
#[derive(Debug, Clone, Copy)]
pub struct StepContext {
    /// Fixed timestep $\Delta t$ for this tick in seconds.
    pub dt: f64,
    /// Total ticks executed prior to this tick.
    pub tick_count: u64,
}

/// A generic, extensible simulation system.
///
/// Implementations calculate subsystem behavior (e.g. equipment models, hydraulics,
/// signal propagation) by reading State[N] and writing State[N+1].
pub trait SimulationSystem: Send {
    /// Human-readable identifier for diagnostics.
    fn name(&self) -> &str;

    /// Executes system logic for one simulation step.
    ///
    /// Must read from `read_state` and write strictly to `write_state`.
    fn step(
        &mut self,
        ctx: StepContext,
        read_state: &StateReadView<'_>,
        write_state: &mut StateWriteView<'_>,
    ) -> Result<(), SimError>;
}
