//! Vendor-Neutral DCS Control Module Architecture.
//!
//! Conforms to ISA-95 Level 2 regulatory and supervisory control module standards.
//! Encapsulates Process Variable (PV), Setpoint (SP), Output Demand (OUT),
//! operating modes (MAN, AUTO, CAS, RCAS), bumpless transfer, permissive interlocks,
//! and fail-safe output clamping.

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::error::ControlError;
use crate::pid::{PidConfig, PidController, PidMode};
use crate::runtime::io_image::IoImageTable;
use crate::runtime::program::ControllerProgram;
use nebudcs_model::tag::TagQuality;

/// Operating modes for a DCS Control Module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ControlModuleMode {
    /// Manual mode: Operator commands output directly.
    Manual,
    /// Automatic mode: Algorithm regulates PV to local setpoint.
    Auto,
    /// Cascade mode: Setpoint is provided by a master cascade controller.
    Cascade,
    /// Remote Cascade mode: Setpoint is provided by a supervisory computer.
    RemoteCascade,
}

impl ControlModuleMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "MAN",
            Self::Auto => "AUTO",
            Self::Cascade => "CAS",
            Self::RemoteCascade => "RCAS",
        }
    }
}

/// Control strategy implemented by the module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleStrategy {
    Pid,
    Cascade,
    Ratio,
    Override,
    ManualOnly,
}

/// Current status and telemetry for a DCS Control Module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlModuleStatus {
    pub id: String,
    pub name: String,
    pub mode: ControlModuleMode,
    pub sp: f64,
    pub pv: f64,
    pub out: f64,
    pub pv_quality: TagQuality,
    pub is_interlocked: bool,
    pub permissives_ok: bool,
    pub tracking_active: bool,
    pub in_alarm: bool,
    pub execution_count: u64,
}

/// A vendor-neutral DCS Control Module.
#[derive(Debug, Clone)]
pub struct DcsControlModule {
    id: String,
    name: String,
    description: String,
    mode: ControlModuleMode,
    strategy: ModuleStrategy,
    execution_rate_seconds: f64,
    accumulated_dt: f64,

    // Signals
    pv_key: String,
    sp: f64,
    remote_sp: Option<f64>,
    out_key: String,
    out_manual: f64,
    current_out: f64,
    current_pv: f64,
    pv_quality: TagQuality,

    // Limits
    output_min: f64,
    output_max: f64,

    // Algorithm
    pid: PidController,

    // Safety and permissives
    permissives: Vec<String>,
    interlocks: Vec<String>,
    safe_position: f64,
    is_interlocked: bool,
    permissives_ok: bool,

    // Tracking
    tracking_active: bool,
    tracking_value: f64,

    execution_count: u64,
}

impl DcsControlModule {
    /// Creates a new DCS Control Module.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        pv_key: impl Into<String>,
        out_key: impl Into<String>,
        initial_sp: f64,
        pid_config: PidConfig,
    ) -> Self {
        let out_min = pid_config.out_min;
        let out_max = pid_config.out_max;
        let mut pid = PidController::new(pid_config);
        pid.set_mode(PidMode::Auto);
        pid.set_sp(initial_sp);
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            mode: ControlModuleMode::Auto,
            strategy: ModuleStrategy::Pid,
            execution_rate_seconds: 0.05,
            accumulated_dt: 0.0,
            pv_key: pv_key.into(),
            sp: initial_sp,
            remote_sp: None,
            out_key: out_key.into(),
            out_manual: 0.0,
            current_out: 0.0,
            current_pv: 0.0,
            pv_quality: TagQuality::Good,
            output_min: out_min,
            output_max: out_max,
            pid,
            permissives: Vec::new(),
            interlocks: Vec::new(),
            safe_position: 0.0,
            is_interlocked: false,
            permissives_ok: true,
            tracking_active: false,
            tracking_value: 0.0,
            execution_count: 0,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn mode(&self) -> ControlModuleMode {
        self.mode
    }

    pub fn strategy(&self) -> &ModuleStrategy {
        &self.strategy
    }

    pub fn execution_rate_seconds(&self) -> f64 {
        self.execution_rate_seconds
    }

    pub fn with_execution_rate(mut self, rate_seconds: f64) -> Self {
        self.execution_rate_seconds = rate_seconds.max(0.001);
        self
    }

    pub fn pv_key(&self) -> &str {
        &self.pv_key
    }

    pub fn out_key(&self) -> &str {
        &self.out_key
    }

    pub fn sp(&self) -> f64 {
        self.sp
    }

    pub fn set_sp(&mut self, sp: f64) {
        self.sp = sp;
        self.pid.set_sp(sp);
    }

    pub fn set_remote_sp(&mut self, sp: Option<f64>) {
        self.remote_sp = sp;
        if self.mode == ControlModuleMode::Cascade {
            if let Some(s) = sp {
                self.pid.set_sp(s);
            }
        }
    }

    pub fn out_manual(&self) -> f64 {
        self.out_manual
    }

    pub fn set_manual_out(&mut self, val: f64) {
        self.out_manual = val.clamp(self.output_min, self.output_max);
        self.pid.set_manual_output(self.out_manual);
        if self.mode == ControlModuleMode::Manual {
            self.current_out = self.out_manual;
        }
    }

    pub fn current_pv(&self) -> f64 {
        self.current_pv
    }

    pub fn current_out(&self) -> f64 {
        self.current_out
    }

    pub fn pv_quality(&self) -> TagQuality {
        self.pv_quality
    }

    pub fn is_interlocked(&self) -> bool {
        self.is_interlocked
    }

    pub fn permissives_ok(&self) -> bool {
        self.permissives_ok
    }

    pub fn permissives(&self) -> &[String] {
        &self.permissives
    }

    pub fn interlocks(&self) -> &[String] {
        &self.interlocks
    }

    pub fn add_permissive(&mut self, signal_key: impl Into<String>) {
        self.permissives.push(signal_key.into());
    }

    pub fn add_interlock(&mut self, signal_key: impl Into<String>) {
        self.interlocks.push(signal_key.into());
    }

    pub fn set_safe_position(&mut self, pos: f64) {
        self.safe_position = pos;
    }

    pub fn safe_position(&self) -> f64 {
        self.safe_position
    }

    pub fn pid(&self) -> &PidController {
        &self.pid
    }

    pub fn pid_mut(&mut self) -> &mut PidController {
        &mut self.pid
    }

    /// Changes module operating mode with bumpless transfer.
    pub fn set_mode(&mut self, new_mode: ControlModuleMode) -> Result<(), ControlError> {
        if self.mode == new_mode {
            return Ok(());
        }

        // Permissive check when entering Auto or Cascade
        if (new_mode == ControlModuleMode::Auto || new_mode == ControlModuleMode::Cascade)
            && !self.permissives_ok
        {
            return Err(ControlError::InvalidParameter(format!(
                "Module '{}' cannot enter {} mode: Permissives not satisfied",
                self.name,
                new_mode.as_str()
            )));
        }

        match (self.mode, new_mode) {
            (ControlModuleMode::Manual, ControlModuleMode::Auto) => {
                // Bumpless transfer: Seed PID manual output before switching mode
                self.pid.set_manual_output(self.out_manual);
                self.pid.set_mode(PidMode::Auto);
                if !self.pid.config().sp_tracking_in_manual {
                    self.pid.set_sp(self.sp);
                } else {
                    self.sp = self.pid.sp();
                }
                self.mode = ControlModuleMode::Auto;
            }
            (ControlModuleMode::Manual, ControlModuleMode::Cascade) => {
                self.pid.set_manual_output(self.out_manual);
                self.pid.set_mode(PidMode::Cascade);
                if let Some(rsp) = self.remote_sp {
                    self.pid.set_sp(rsp);
                } else {
                    self.pid.set_sp(self.sp);
                }
                self.mode = ControlModuleMode::Cascade;
            }
            (_, ControlModuleMode::Manual) => {
                // Track current output into manual hold
                self.out_manual = self.current_out;
                self.pid.set_manual_output(self.out_manual);
                self.pid.set_mode(PidMode::Manual);
                self.mode = ControlModuleMode::Manual;
            }
            (_, ControlModuleMode::Auto) => {
                self.pid.set_sp(self.sp);
                self.pid.set_mode(PidMode::Auto);
                self.mode = ControlModuleMode::Auto;
            }
            (_, ControlModuleMode::Cascade) => {
                if let Some(rsp) = self.remote_sp {
                    self.pid.set_sp(rsp);
                }
                self.pid.set_mode(PidMode::Cascade);
                self.mode = ControlModuleMode::Cascade;
            }
            (_, ControlModuleMode::RemoteCascade) => {
                self.mode = ControlModuleMode::RemoteCascade;
            }
        }
        Ok(())
    }

    /// Sets external tracking mode (e.g. from actuator feedback or safety override).
    pub fn set_tracking(&mut self, active: bool, track_value: f64) {
        self.tracking_active = active;
        self.tracking_value = track_value;
        if active {
            self.pid.enter_tracking(track_value);
        } else if self.mode != ControlModuleMode::Manual {
            self.pid.exit_tracking();
        }
    }

    /// Obtains current module status summary.
    pub fn status(&self) -> ControlModuleStatus {
        ControlModuleStatus {
            id: self.id.clone(),
            name: self.name.clone(),
            mode: self.mode,
            sp: if self.mode == ControlModuleMode::Cascade {
                self.remote_sp.unwrap_or(self.sp)
            } else {
                self.sp
            },
            pv: self.current_pv,
            out: self.current_out,
            pv_quality: self.pv_quality,
            is_interlocked: self.is_interlocked,
            permissives_ok: self.permissives_ok,
            tracking_active: self.tracking_active,
            in_alarm: self.is_interlocked
                || !self.permissives_ok
                || self.pv_quality != TagQuality::Good,
            execution_count: self.execution_count,
        }
    }

    /// Executes one control calculation cycle.
    pub fn step(
        &mut self,
        dt: f64,
        sim_time: f64,
        io_image: &mut IoImageTable,
    ) -> Result<(), ControlError> {
        self.accumulated_dt += dt;
        if self.accumulated_dt < (self.execution_rate_seconds - 1e-9) {
            return Ok(());
        }
        let cycle_dt = self.accumulated_dt;
        self.accumulated_dt = 0.0;
        self.execution_count += 1;

        // 1. Input Acquisition (%I)
        if let Some(sig) = io_image.read_input(&self.pv_key) {
            self.current_pv = sig.value;
            self.pv_quality = sig.quality;
        } else {
            self.pv_quality = TagQuality::Bad;
        }

        self.pid.set_quality(self.pv_quality);

        // 2. Permissive Evaluation
        let mut perm_ok = true;
        for key in &self.permissives {
            let p_val = io_image.read_input_value(key).unwrap_or(0.0);
            if p_val <= 0.0 {
                perm_ok = false;
                break;
            }
        }
        self.permissives_ok = perm_ok;

        // If in Auto/Cas and permissives fail, force fallback to Manual
        if !perm_ok
            && (self.mode == ControlModuleMode::Auto || self.mode == ControlModuleMode::Cascade)
        {
            self.out_manual = self.current_out;
            self.pid.set_manual_output(self.out_manual);
            self.mode = ControlModuleMode::Manual;
            self.pid.set_mode(PidMode::Manual);
        }

        // 3. Interlock Evaluation
        let mut interlocked = false;
        for key in &self.interlocks {
            let i_val = io_image.read_input_value(key).unwrap_or(0.0);
            if i_val > 0.0 {
                interlocked = true;
                break;
            }
        }
        self.is_interlocked = interlocked;

        // 4. Algorithm Execution
        let out_demand = if interlocked {
            // Safety Interlock clamps output directly
            self.safe_position
        } else if self.tracking_active {
            self.tracking_value
        } else {
            match self.mode {
                ControlModuleMode::Manual => {
                    self.pid.set_manual_output(self.out_manual);
                    let _ = self.pid.step(cycle_dt, self.current_pv);
                    self.out_manual
                }
                ControlModuleMode::Auto => {
                    self.pid.set_sp(self.sp);
                    self.pid.step(cycle_dt, self.current_pv)
                }
                ControlModuleMode::Cascade => {
                    let target_sp = self.remote_sp.unwrap_or(self.sp);
                    self.pid.set_sp(target_sp);
                    self.pid.step(cycle_dt, self.current_pv)
                }
                ControlModuleMode::RemoteCascade => {
                    let target_sp = self.remote_sp.unwrap_or(self.sp);
                    self.pid.set_sp(target_sp);
                    self.pid.step(cycle_dt, self.current_pv)
                }
            }
        };

        self.current_out = out_demand.clamp(self.output_min, self.output_max);

        // 5. Output Commit (%Q)
        let out_quality = if interlocked {
            TagQuality::Good // Explicit fail-safe command
        } else if self.pv_quality == TagQuality::Bad {
            TagQuality::Uncertain
        } else {
            TagQuality::Good
        };

        io_image.write_output(&self.out_key, self.current_out, out_quality, sim_time);

        Ok(())
    }

    /// Resets the module to its initial state.
    pub fn reset(&mut self) {
        self.pid.reset();
        self.accumulated_dt = 0.0;
        self.current_out = 0.0;
        self.current_pv = 0.0;
        self.is_interlocked = false;
        self.permissives_ok = true;
        self.tracking_active = false;
    }
}

impl ControllerProgram for DcsControlModule {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(
        &mut self,
        dt: f64,
        sim_time: f64,
        io_image: &mut IoImageTable,
    ) -> Result<(), ControlError> {
        self.step(dt, sim_time, io_image)
    }

    fn reset(&mut self) {
        self.reset();
    }

    fn clone_box(&self) -> Box<dyn ControllerProgram> {
        Box::new(self.clone())
    }
}
