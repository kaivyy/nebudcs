//! Controller Program and Routine Abstraction.
//!
//! Provides the execution unit for control logic (PID loops, interlocks, sequences,
//! and future IEC 61131-3 logic routines).

use std::fmt::Debug;
use std::sync::Arc;

use nebudcs_model::tag::TagQuality;
use nebudcs_types::EquipmentId;

use crate::error::ControlError;
use crate::pid::{PidConfig, PidController};
use crate::protection::{ProtectionBlock, ProtectionConfig, ProtectionState, TripDirection};
use crate::runtime::io_image::IoImageTable;

/// Trait implemented by any logic routine or program executable within a Controller Task.
pub trait ControllerProgram: Send + Sync + Debug {
    /// Returns the unique program or routine name.
    fn name(&self) -> &str;

    /// Executes the program logic for the current task scan cycle.
    fn execute(
        &mut self,
        dt: f64,
        sim_time: f64,
        io_image: &mut IoImageTable,
    ) -> Result<(), ControlError>;

    /// Resets the internal program state.
    fn reset(&mut self);

    /// Clones the program into a boxed trait object.
    fn clone_box(&self) -> Box<dyn ControllerProgram>;
}

impl Clone for Box<dyn ControllerProgram> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// A regulatory PID control program reading from the Input Image Table and
/// writing to the Output Image Table.
#[derive(Debug, Clone)]
pub struct PidLoopProgram {
    name: String,
    pv_key: String,
    out_key: String,
    target_actuator: Option<EquipmentId>,
    controller: PidController,
}

impl PidLoopProgram {
    /// Creates a new PID program with the given name, I/O bindings, and PID configuration.
    pub fn new(
        name: impl Into<String>,
        pv_key: impl Into<String>,
        out_key: impl Into<String>,
        target_actuator: Option<EquipmentId>,
        config: PidConfig,
    ) -> Self {
        Self {
            name: name.into(),
            pv_key: pv_key.into(),
            out_key: out_key.into(),
            target_actuator,
            controller: PidController::new(config),
        }
    }

    /// Access the underlying PID controller immutably.
    pub fn controller(&self) -> &PidController {
        &self.controller
    }

    /// Access the underlying PID controller mutably.
    pub fn controller_mut(&mut self) -> &mut PidController {
        &mut self.controller
    }

    /// Target actuator driven by this PID loop, if any.
    pub fn target_actuator(&self) -> Option<EquipmentId> {
        self.target_actuator
    }

    /// Input key in the Input Image Table.
    pub fn pv_key(&self) -> &str {
        &self.pv_key
    }

    /// Output key in the Output Image Table.
    pub fn out_key(&self) -> &str {
        &self.out_key
    }
}

impl ControllerProgram for PidLoopProgram {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(
        &mut self,
        dt: f64,
        sim_time: f64,
        io_image: &mut IoImageTable,
    ) -> Result<(), ControlError> {
        let (pv_val, pv_qual) = match io_image.read_input(&self.pv_key) {
            Some(sig) => (sig.value, sig.quality),
            None => (0.0, TagQuality::Bad),
        };

        self.controller.set_quality(pv_qual);
        let out_val = self.controller.step(dt, pv_val);

        io_image.write_output(&self.out_key, out_val, TagQuality::Good, sim_time);
        Ok(())
    }

    fn reset(&mut self) {
        self.controller.reset();
    }

    fn clone_box(&self) -> Box<dyn ControllerProgram> {
        Box::new(self.clone())
    }
}

/// An automated safety/process interlock program evaluating trip thresholds.
#[derive(Debug, Clone)]
pub struct InterlockProgram {
    name: String,
    source_key: String,
    out_key: String,
    target_actuator: Option<EquipmentId>,
    safe_position: f64,
    block: ProtectionBlock,
    reset_requested: bool,
}

impl InterlockProgram {
    /// Creates a new interlock program.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        source_key: impl Into<String>,
        out_key: impl Into<String>,
        target_actuator: Option<EquipmentId>,
        safe_position: f64,
        threshold: f64,
        reset_deadband: f64,
        direction: TripDirection,
    ) -> Self {
        let name_str = name.into();
        let config = ProtectionConfig {
            name: name_str.clone(),
            threshold,
            reset_deadband,
            direction,
        };
        Self {
            name: name_str,
            source_key: source_key.into(),
            out_key: out_key.into(),
            target_actuator,
            safe_position,
            block: ProtectionBlock::new(config),
            reset_requested: false,
        }
    }

    /// Request a reset of a tripped interlock.
    pub fn request_reset(&mut self) {
        self.reset_requested = true;
    }

    /// Returns whether this interlock is currently tripped.
    pub fn is_tripped(&self) -> bool {
        self.block.is_tripped()
    }

    /// Target actuator affected by this interlock.
    pub fn target_actuator(&self) -> Option<EquipmentId> {
        self.target_actuator
    }

    /// Commanded safe position when tripped.
    pub fn safe_position(&self) -> f64 {
        self.safe_position
    }
}

impl ControllerProgram for InterlockProgram {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(
        &mut self,
        _dt: f64,
        sim_time: f64,
        io_image: &mut IoImageTable,
    ) -> Result<(), ControlError> {
        let val = io_image.read_input_value(&self.source_key).unwrap_or(0.0);
        let reset_cmd = self.reset_requested;
        self.reset_requested = false;

        let tripped = self.block.step(val, reset_cmd);
        io_image.write_output(
            &self.out_key,
            if tripped { 1.0 } else { 0.0 },
            TagQuality::Good,
            sim_time,
        );
        Ok(())
    }

    fn reset(&mut self) {
        self.block.set_state(ProtectionState::Armed);
        self.reset_requested = false;
    }

    fn clone_box(&self) -> Box<dyn ControllerProgram> {
        Box::new(self.clone())
    }
}

/// Type alias for custom closure routines.
type RoutineFn = Arc<dyn Fn(f64, f64, &mut IoImageTable) -> Result<(), ControlError> + Send + Sync>;

/// Custom routine program executing an injected closure or function.
#[derive(Clone)]
pub struct CustomRoutineProgram {
    name: String,
    routine: RoutineFn,
}

/// Type alias for CustomRoutineProgram.
pub type RoutineProgram = CustomRoutineProgram;

impl Debug for CustomRoutineProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomRoutineProgram")
            .field("name", &self.name)
            .finish()
    }
}

impl CustomRoutineProgram {
    /// Creates a custom routine program from a closure.
    pub fn new<F>(name: impl Into<String>, routine: F) -> Self
    where
        F: Fn(f64, f64, &mut IoImageTable) -> Result<(), ControlError> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            routine: Arc::new(routine),
        }
    }
}

impl ControllerProgram for CustomRoutineProgram {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(
        &mut self,
        dt: f64,
        sim_time: f64,
        io_image: &mut IoImageTable,
    ) -> Result<(), ControlError> {
        (self.routine)(dt, sim_time, io_image)
    }

    fn reset(&mut self) {}

    fn clone_box(&self) -> Box<dyn ControllerProgram> {
        Box::new(self.clone())
    }
}
