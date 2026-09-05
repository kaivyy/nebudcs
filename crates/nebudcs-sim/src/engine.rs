//! Central simulation engine orchestrating clock, double-buffered state, systems, and execution plan.
//!
//! # Tick Lifecycle Pipeline
//!
//! 1. Verify engine is running (or explicit `step()` requested).
//! 2. Calculate next timestamp: $t_{N+1} = t_N + \Delta t$.
//! 3. Prepare State[N+1] (deep copy of State[N] with updated timestamp).
//! 4. Execute registered systems in deterministic order.
//!    - Each system receives `&StateReadView` (State[N]) and `&mut StateWriteView` (State[N+1]).
//! 5. If all systems succeed: commit State[N+1] $\rightarrow$ State[N].
//! 6. Advance clock ($t_N \leftarrow t_{N+1}$, increment `tick_count`).
//! 7. If any system fails: abort tick without committing; state remains at State[N].

use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_types::{EquipmentId, TagId};

use crate::clock::{ClockState, SimulationClock};
use crate::error::SimError;
use crate::plan::ExecutionPlan;
use crate::state::{DoubleBufferedState, EquipmentState, StateBuffer, StateReadView};
use crate::system::{SimulationSystem, StepContext};

/// Central deterministic simulation engine.
pub struct SimulationEngine {
    /// Fixed-step simulation clock.
    clock: SimulationClock,
    /// Double-buffered simulation state.
    state: DoubleBufferedState,
    /// Static execution plan derived from plant topology.
    plan: ExecutionPlan,
    /// Registered simulation systems executed in deterministic order.
    systems: Vec<Box<dyn SimulationSystem>>,
}

impl SimulationEngine {
    /// Creates a new simulation engine with a given fixed timestep $\Delta t$.
    pub fn new(dt: f64) -> Result<Self, SimError> {
        let clock = SimulationClock::new(dt)?;
        Ok(Self {
            clock,
            state: DoubleBufferedState::new(),
            plan: ExecutionPlan::new(),
            systems: Vec::new(),
        })
    }

    /// Sets the static execution plan for this engine.
    pub fn set_plan(&mut self, plan: ExecutionPlan) {
        self.plan = plan;
    }

    /// Read-only reference to the current execution plan.
    #[inline]
    pub fn plan(&self) -> &ExecutionPlan {
        &self.plan
    }

    /// Registers an extensible simulation system.
    pub fn add_system<S: SimulationSystem + 'static>(&mut self, system: S) {
        self.systems.push(Box::new(system));
    }

    /// Initializes the simulation state with an initial buffer.
    pub fn initialize(&mut self, initial_state: StateBuffer) {
        self.state.initialize(initial_state);
    }

    /// Read-only access to the simulation clock.
    #[inline]
    pub fn clock(&self) -> &SimulationClock {
        &self.clock
    }

    /// Mutable access to the simulation clock (play/pause/multiplier).
    #[inline]
    pub fn clock_mut(&mut self) -> &mut SimulationClock {
        &mut self.clock
    }

    /// Read-only view of the currently committed state State[N].
    #[inline]
    pub fn state(&self) -> StateReadView<'_> {
        self.state.read_view()
    }

    /// Returns the raw committed StateBuffer.
    #[inline]
    pub fn committed_buffer(&self) -> &StateBuffer {
        self.state.current()
    }

    /// Convenience: lookup an equipment state from the committed buffer.
    #[inline]
    pub fn get_equipment(&self, id: EquipmentId) -> Option<&EquipmentState> {
        self.state.current().get_equipment(id)
    }

    /// Convenience: lookup a tag value from the committed buffer.
    #[inline]
    pub fn get_tag(&self, id: TagId) -> Option<&TagValue> {
        self.state.current().get_tag(id)
    }

    /// Convenience: lookup a tag quality from the committed buffer.
    #[inline]
    pub fn get_quality(&self, id: TagId) -> Option<TagQuality> {
        self.state.current().get_quality(id)
    }

    /// Executes exactly one simulation tick unconditionally.
    ///
    /// Even if the clock is paused, this method advances the simulation by
    /// one discrete step (single-step operation).
    pub fn step(&mut self) -> Result<(), SimError> {
        self.execute_tick()
    }

    /// Ticks the simulation if and only if the clock is in the `Running` state.
    ///
    /// If paused, returns `Ok(false)` without advancing state or time.
    /// If running, executes one tick and returns `Ok(true)`.
    pub fn tick_if_running(&mut self) -> Result<bool, SimError> {
        if self.clock.state() == ClockState::Running {
            self.execute_tick()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Core deterministic execution pipeline.
    ///
    /// Guarantees that State[N+1] is committed only after ALL systems succeed.
    /// If any system fails, State[N] remains intact.
    fn execute_tick(&mut self) -> Result<(), SimError> {
        let dt = self.clock.dt();
        let next_time = self.clock.time().advance(dt);
        let ctx = StepContext {
            dt,
            tick_count: self.clock.tick_count(),
        };

        // 1. Prepare State[N+1] as clone of State[N] with time = next_time
        self.state.prepare_next(next_time);

        // 2. Execute all registered systems against (ReadView, WriteView)
        {
            let (read_view, mut write_view) = self.state.rw_views();

            for sys in &mut self.systems {
                if let Err(e) = sys.step(ctx, &read_view, &mut write_view) {
                    return Err(SimError::SystemExecutionFailed {
                        system_name: sys.name().to_string(),
                        detail: e.to_string(),
                    });
                }
            }
        }

        // 3. Commit State[N+1] into State[N]
        self.state.commit();

        // 4. Advance clock
        self.clock.advance_tick();

        Ok(())
    }
}
