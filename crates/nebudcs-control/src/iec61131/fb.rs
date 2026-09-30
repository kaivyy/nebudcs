//! Standard IEC 61131-3 Function Blocks and Custom FB Instances.
//!
//! Implements industrial timers (TON, TOF, TP), counters (CTU, CTD, CTUD),
//! bistables (SR, RS), edge triggers (R_TRIG, F_TRIG), and an adapter
//! for the existing production `PidController`.

use std::collections::BTreeMap;
use std::fmt::Debug;

use crate::advanced::{
    AntiSurgeAlarmState, AntiSurgeController, FirstOrderFilter, HighSelector, LeadLagCompensator,
    LowSelector, RatioController, SplitRangeController, ValveCharacteristic, ValveCharacterizer,
};
use crate::error::ControlError;
use crate::iec61131::types::IecValue;
use crate::pid::{PidConfig, PidController, PidMode};

/// Trait implemented by any stateful function block instance.
pub trait FunctionBlockInstance: Send + Sync + Debug {
    /// Returns the instance identifier.
    fn name(&self) -> &str;

    /// Returns the function block type name (e.g. "TON", "CTU", "PID").
    fn fb_type(&self) -> &str;

    /// Executes one scan cycle of the function block using deterministic simulation time.
    fn step(
        &mut self,
        dt: f64,
        sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError>;

    /// Reads a specific output parameter from the function block.
    fn get_output(&self, param: &str) -> Option<IecValue>;

    /// Resets the internal state of the function block.
    fn reset(&mut self);

    /// Clones the function block into a boxed trait object.
    fn clone_box(&self) -> Box<dyn FunctionBlockInstance>;
}

impl Clone for Box<dyn FunctionBlockInstance> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

// ---------------------------------------------------------------------------
// Timers: TON, TOF, TP (Using Deterministic Simulation Clock)
// ---------------------------------------------------------------------------

/// Standard TON (Timer On-Delay) function block.
#[derive(Debug, Clone)]
pub struct TonBlock {
    name: String,
    elapsed_time: f64,
    output_q: bool,
}

impl TonBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            elapsed_time: 0.0,
            output_q: false,
        }
    }
}

impl FunctionBlockInstance for TonBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "TON"
    }

    fn step(
        &mut self,
        dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        let in_val = inputs
            .get("IN")
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);
        let pt = inputs
            .get("PT")
            .map(|v| v.as_f64().unwrap_or(0.0))
            .unwrap_or(0.0);

        if in_val {
            self.elapsed_time += dt;
            if self.elapsed_time >= pt {
                self.output_q = true;
                self.elapsed_time = pt;
            } else {
                self.output_q = false;
            }
        } else {
            self.elapsed_time = 0.0;
            self.output_q = false;
        }

        let mut out = BTreeMap::new();
        out.insert("Q".to_string(), IecValue::Bool(self.output_q));
        out.insert("ET".to_string(), IecValue::Time(self.elapsed_time));
        Ok(out)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "Q" => Some(IecValue::Bool(self.output_q)),
            "ET" => Some(IecValue::Time(self.elapsed_time)),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.elapsed_time = 0.0;
        self.output_q = false;
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

/// Standard TOF (Timer Off-Delay) function block.
#[derive(Debug, Clone)]
pub struct TofBlock {
    name: String,
    elapsed_time: f64,
    output_q: bool,
}

impl TofBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            elapsed_time: 0.0,
            output_q: false,
        }
    }
}

impl FunctionBlockInstance for TofBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "TOF"
    }

    fn step(
        &mut self,
        dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        let in_val = inputs
            .get("IN")
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);
        let pt = inputs
            .get("PT")
            .map(|v| v.as_f64().unwrap_or(0.0))
            .unwrap_or(0.0);

        if in_val {
            self.output_q = true;
            self.elapsed_time = 0.0;
        } else if self.output_q {
            self.elapsed_time += dt;
            if self.elapsed_time >= pt {
                self.output_q = false;
                self.elapsed_time = pt;
            }
        } else {
            self.elapsed_time = pt;
        }

        let mut out = BTreeMap::new();
        out.insert("Q".to_string(), IecValue::Bool(self.output_q));
        out.insert("ET".to_string(), IecValue::Time(self.elapsed_time));
        Ok(out)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "Q" => Some(IecValue::Bool(self.output_q)),
            "ET" => Some(IecValue::Time(self.elapsed_time)),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.elapsed_time = 0.0;
        self.output_q = false;
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

/// Standard TP (Pulse Timer) function block.
#[derive(Debug, Clone)]
pub struct TpBlock {
    name: String,
    elapsed_time: f64,
    output_q: bool,
    running: bool,
}

impl TpBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            elapsed_time: 0.0,
            output_q: false,
            running: false,
        }
    }
}

impl FunctionBlockInstance for TpBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "TP"
    }

    fn step(
        &mut self,
        dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        let in_val = inputs
            .get("IN")
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);
        let pt = inputs
            .get("PT")
            .map(|v| v.as_f64().unwrap_or(0.0))
            .unwrap_or(0.0);

        if !self.running {
            if in_val {
                self.running = true;
                self.output_q = true;
                self.elapsed_time = 0.0;
            }
        } else {
            self.elapsed_time += dt;
            if self.elapsed_time >= pt {
                self.output_q = false;
                self.elapsed_time = pt;
                if !in_val {
                    self.running = false;
                    self.elapsed_time = 0.0;
                }
            } else {
                self.output_q = true;
            }
        }

        let mut out = BTreeMap::new();
        out.insert("Q".to_string(), IecValue::Bool(self.output_q));
        out.insert("ET".to_string(), IecValue::Time(self.elapsed_time));
        Ok(out)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "Q" => Some(IecValue::Bool(self.output_q)),
            "ET" => Some(IecValue::Time(self.elapsed_time)),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.elapsed_time = 0.0;
        self.output_q = false;
        self.running = false;
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

// ---------------------------------------------------------------------------
// Counters: CTU, CTD, CTUD
// ---------------------------------------------------------------------------

/// Standard CTU (Up-Counter) function block.
#[derive(Debug, Clone)]
pub struct CtuBlock {
    name: String,
    cv: i32,
    prev_cu: bool,
    output_q: bool,
}

impl CtuBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cv: 0,
            prev_cu: false,
            output_q: false,
        }
    }
}

impl FunctionBlockInstance for CtuBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "CTU"
    }

    fn step(
        &mut self,
        _dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        let cu = inputs
            .get("CU")
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);
        let reset = inputs
            .get("RESET")
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);
        let pv = inputs
            .get("PV")
            .map(|v| v.as_i64().unwrap_or(0) as i32)
            .unwrap_or(0);

        if reset {
            self.cv = 0;
        } else if cu && !self.prev_cu && self.cv < i32::MAX {
            self.cv += 1;
        }
        self.prev_cu = cu;
        self.output_q = self.cv >= pv;

        let mut out = BTreeMap::new();
        out.insert("Q".to_string(), IecValue::Bool(self.output_q));
        out.insert("CV".to_string(), IecValue::Dint(self.cv));
        Ok(out)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "Q" => Some(IecValue::Bool(self.output_q)),
            "CV" => Some(IecValue::Dint(self.cv)),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.cv = 0;
        self.prev_cu = false;
        self.output_q = false;
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

/// Standard CTD (Down-Counter) function block.
#[derive(Debug, Clone)]
pub struct CtdBlock {
    name: String,
    cv: i32,
    prev_cd: bool,
    output_q: bool,
}

impl CtdBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cv: 0,
            prev_cd: false,
            output_q: false,
        }
    }
}

impl FunctionBlockInstance for CtdBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "CTD"
    }

    fn step(
        &mut self,
        _dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        let cd = inputs
            .get("CD")
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);
        let load = inputs
            .get("LOAD")
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);
        let pv = inputs
            .get("PV")
            .map(|v| v.as_i64().unwrap_or(0) as i32)
            .unwrap_or(0);

        if load {
            self.cv = pv;
        } else if cd && !self.prev_cd && self.cv > i32::MIN {
            self.cv -= 1;
        }
        self.prev_cd = cd;
        self.output_q = self.cv <= 0;

        let mut out = BTreeMap::new();
        out.insert("Q".to_string(), IecValue::Bool(self.output_q));
        out.insert("CV".to_string(), IecValue::Dint(self.cv));
        Ok(out)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "Q" => Some(IecValue::Bool(self.output_q)),
            "CV" => Some(IecValue::Dint(self.cv)),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.cv = 0;
        self.prev_cd = false;
        self.output_q = false;
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

// ---------------------------------------------------------------------------
// Edge Triggers: R_TRIG, F_TRIG
// ---------------------------------------------------------------------------

/// Standard R_TRIG (Rising Edge Detector) function block.
#[derive(Debug, Clone)]
pub struct RTrigBlock {
    name: String,
    prev_clk: bool,
    output_q: bool,
}

impl RTrigBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            prev_clk: false,
            output_q: false,
        }
    }
}

impl FunctionBlockInstance for RTrigBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "R_TRIG"
    }

    fn step(
        &mut self,
        _dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        let clk = inputs
            .get("CLK")
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);
        self.output_q = clk && !self.prev_clk;
        self.prev_clk = clk;

        let mut out = BTreeMap::new();
        out.insert("Q".to_string(), IecValue::Bool(self.output_q));
        Ok(out)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "Q" => Some(IecValue::Bool(self.output_q)),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.prev_clk = false;
        self.output_q = false;
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

/// Standard F_TRIG (Falling Edge Detector) function block.
#[derive(Debug, Clone)]
pub struct FTrigBlock {
    name: String,
    prev_clk: bool,
    output_q: bool,
}

impl FTrigBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            prev_clk: true,
            output_q: false,
        }
    }
}

impl FunctionBlockInstance for FTrigBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "F_TRIG"
    }

    fn step(
        &mut self,
        _dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        let clk = inputs
            .get("CLK")
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);
        self.output_q = !clk && self.prev_clk;
        self.prev_clk = clk;

        let mut out = BTreeMap::new();
        out.insert("Q".to_string(), IecValue::Bool(self.output_q));
        Ok(out)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "Q" => Some(IecValue::Bool(self.output_q)),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.prev_clk = true;
        self.output_q = false;
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

// ---------------------------------------------------------------------------
// Bistable Latches: SR, RS
// ---------------------------------------------------------------------------

/// Standard SR (Set-Dominant Bistable Latch).
#[derive(Debug, Clone)]
pub struct SrBlock {
    name: String,
    q1: bool,
}

impl SrBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            q1: false,
        }
    }
}

impl FunctionBlockInstance for SrBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "SR"
    }

    fn step(
        &mut self,
        _dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        let s1 = inputs
            .get("S1")
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);
        let r = inputs
            .get("R")
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);

        // Set dominant: Q1 = S1 OR (NOT R AND Q1)
        self.q1 = s1 || (!r && self.q1);

        let mut out = BTreeMap::new();
        out.insert("Q1".to_string(), IecValue::Bool(self.q1));
        Ok(out)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "Q1" => Some(IecValue::Bool(self.q1)),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.q1 = false;
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

/// Standard RS (Reset-Dominant Bistable Latch).
#[derive(Debug, Clone)]
pub struct RsBlock {
    name: String,
    q1: bool,
}

impl RsBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            q1: false,
        }
    }
}

impl FunctionBlockInstance for RsBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "RS"
    }

    fn step(
        &mut self,
        _dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        let s = inputs
            .get("S")
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);
        let r1 = inputs
            .get("R1")
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);

        // Reset dominant: Q1 = NOT R1 AND (S OR Q1)
        self.q1 = !r1 && (s || self.q1);

        let mut out = BTreeMap::new();
        out.insert("Q1".to_string(), IecValue::Bool(self.q1));
        Ok(out)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "Q1" => Some(IecValue::Bool(self.q1)),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.q1 = false;
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

// ---------------------------------------------------------------------------
// Existing Production PID Adapter: PID_IEC
// ---------------------------------------------------------------------------

/// IEC 61131-3 Function Block adapter directly wrapping the authoritative
/// production `PidController` without duplication.
#[derive(Debug, Clone)]
pub struct PidIecBlock {
    name: String,
    controller: PidController,
}

impl PidIecBlock {
    /// Creates a new PID IEC block using standard PID configuration.
    pub fn new(name: impl Into<String>, config: PidConfig) -> Self {
        Self {
            name: name.into(),
            controller: PidController::new(config),
        }
    }

    /// Access the underlying controller immutably.
    pub fn controller(&self) -> &PidController {
        &self.controller
    }

    /// Access the underlying controller mutably.
    pub fn controller_mut(&mut self) -> &mut PidController {
        &mut self.controller
    }
}

impl FunctionBlockInstance for PidIecBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "PID"
    }

    fn step(
        &mut self,
        dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        // Read PV (Process Variable)
        let pv_val = if let Some(pv) = inputs.get("PV") {
            pv.as_f64()?
        } else {
            self.controller.pv()
        };

        // Read Mode (AUTO / MANUAL) first so bumpless transfer does not overwrite SP
        if let Some(auto_val) = inputs.get("AUTO") {
            if auto_val.as_bool()? {
                self.controller.set_mode(PidMode::Auto);
            } else {
                self.controller.set_mode(PidMode::Manual);
            }
        }

        // Read SP (Setpoint)
        if let Some(sp) = inputs.get("SP") {
            self.controller.set_sp(sp.as_f64()?);
        }

        // Read Manual output override
        if let Some(man) = inputs.get("MAN") {
            if self.controller.mode() == PidMode::Manual {
                self.controller.set_manual_output(man.as_f64()?);
            }
        }

        // Step authoritative PID algorithm
        let out_val = self.controller.step(dt, pv_val);
        let error = self.controller.sp() - self.controller.pv();

        let mut out = BTreeMap::new();
        out.insert("OUT".to_string(), IecValue::LReal(out_val));
        out.insert("ERROR".to_string(), IecValue::LReal(error));
        out.insert("PV".to_string(), IecValue::LReal(self.controller.pv()));
        out.insert("SP".to_string(), IecValue::LReal(self.controller.sp()));
        Ok(out)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "OUT" => Some(IecValue::LReal(self.controller.output())),
            "ERROR" => Some(IecValue::LReal(self.controller.sp() - self.controller.pv())),
            "PV" => Some(IecValue::LReal(self.controller.pv())),
            "SP" => Some(IecValue::LReal(self.controller.sp())),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.controller.reset();
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

// ---------------------------------------------------------------------------
// Stage 5: Advanced Control Function Blocks
// ---------------------------------------------------------------------------

/// Dynamic Lead-Lag compensator function block (LEAD_LAG).
#[derive(Debug, Clone)]
pub struct LeadLagBlock {
    name: String,
    compensator: LeadLagCompensator,
}

impl LeadLagBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            compensator: LeadLagCompensator::new(1.0, 1.0),
        }
    }
}

impl FunctionBlockInstance for LeadLagBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "LEAD_LAG"
    }

    fn step(
        &mut self,
        dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        if let Some(t_lead) = inputs.get("T_LEAD") {
            self.compensator.t_lead = t_lead.as_f64()?.max(0.0);
        }
        if let Some(t_lag) = inputs.get("T_LAG") {
            self.compensator.t_lag = t_lag.as_f64()?.max(0.0);
        }
        if let Some(gain) = inputs.get("GAIN") {
            self.compensator.gain = gain.as_f64()?;
        }

        let in_val = if let Some(in_expr) = inputs.get("IN") {
            in_expr.as_f64()?
        } else {
            self.compensator.output()
        };

        let out = self.compensator.step(dt, in_val);
        let mut res = BTreeMap::new();
        res.insert("OUT".to_string(), IecValue::LReal(out));
        Ok(res)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "OUT" => Some(IecValue::LReal(self.compensator.output())),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.compensator.reset();
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

/// First-order exponential lag filter function block (FILTER_1ST).
#[derive(Debug, Clone)]
pub struct FilterFirstOrderBlock {
    name: String,
    filter: FirstOrderFilter,
}

impl FilterFirstOrderBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            filter: FirstOrderFilter::new(1.0),
        }
    }
}

impl FunctionBlockInstance for FilterFirstOrderBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "FILTER_1ST"
    }

    fn step(
        &mut self,
        dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        if let Some(tc) = inputs.get("TIME_CONSTANT") {
            self.filter.time_constant = tc.as_f64()?.max(0.0);
        }

        let in_val = if let Some(in_expr) = inputs.get("IN") {
            in_expr.as_f64()?
        } else {
            self.filter.output()
        };

        let out = self.filter.step(dt, in_val);
        let mut res = BTreeMap::new();
        res.insert("OUT".to_string(), IecValue::LReal(out));
        Ok(res)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "OUT" => Some(IecValue::LReal(self.filter.output())),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.filter.reset();
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

/// Stream ratio calculation function block (RATIO).
#[derive(Debug, Clone)]
pub struct RatioBlock {
    name: String,
    controller: RatioController,
}

impl RatioBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            controller: RatioController::new(1.0),
        }
    }
}

impl FunctionBlockInstance for RatioBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "RATIO"
    }

    fn step(
        &mut self,
        _dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        if let Some(r) = inputs.get("RATIO") {
            self.controller.set_ratio(r.as_f64()?);
        }
        if let Some(b) = inputs.get("BIAS") {
            self.controller.set_bias(b.as_f64()?);
        }

        let wild_val = if let Some(w) = inputs.get("WILD") {
            w.as_f64()?
        } else {
            0.0
        };

        let sp = self.controller.calculate(wild_val);
        let mut res = BTreeMap::new();
        res.insert("SP".to_string(), IecValue::LReal(sp));
        res.insert(
            "ACTUAL_RATIO".to_string(),
            IecValue::LReal(self.controller.actual_ratio()),
        );
        Ok(res)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "SP" => Some(IecValue::LReal(self.controller.output_sp())),
            "ACTUAL_RATIO" => Some(IecValue::LReal(self.controller.actual_ratio())),
            _ => None,
        }
    }

    fn reset(&mut self) {}

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

/// High signal selector function block (SEL_HIGH).
#[derive(Debug, Clone)]
pub struct HighSelectorBlock {
    name: String,
    output: f64,
}

impl HighSelectorBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: 0.0,
        }
    }
}

impl FunctionBlockInstance for HighSelectorBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "SEL_HIGH"
    }

    fn step(
        &mut self,
        _dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        let mut vals = Vec::new();
        for (k, v) in inputs {
            if k.starts_with("IN") {
                vals.push(v.as_f64()?);
            }
        }
        let sel = HighSelector;
        self.output = sel.select(&vals);

        let mut res = BTreeMap::new();
        res.insert("OUT".to_string(), IecValue::LReal(self.output));
        Ok(res)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "OUT" => Some(IecValue::LReal(self.output)),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.output = 0.0;
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

/// Low signal selector function block (SEL_LOW).
#[derive(Debug, Clone)]
pub struct LowSelectorBlock {
    name: String,
    output: f64,
}

impl LowSelectorBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: 0.0,
        }
    }
}

impl FunctionBlockInstance for LowSelectorBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "SEL_LOW"
    }

    fn step(
        &mut self,
        _dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        let mut vals = Vec::new();
        for (k, v) in inputs {
            if k.starts_with("IN") {
                vals.push(v.as_f64()?);
            }
        }
        let sel = LowSelector;
        self.output = sel.select(&vals);

        let mut res = BTreeMap::new();
        res.insert("OUT".to_string(), IecValue::LReal(self.output));
        Ok(res)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "OUT" => Some(IecValue::LReal(self.output)),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.output = 0.0;
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

/// Split-range actuator distribution function block (SPLIT_RANGE).
#[derive(Debug, Clone)]
pub struct SplitRangeBlock {
    name: String,
    controller: SplitRangeController,
    out_a: f64,
    out_b: f64,
}

impl SplitRangeBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            controller: SplitRangeController::standard_two_valve("A", "B"),
            out_a: 0.0,
            out_b: 0.0,
        }
    }
}

impl FunctionBlockInstance for SplitRangeBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "SPLIT_RANGE"
    }

    fn step(
        &mut self,
        _dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        let master = if let Some(in_val) = inputs.get("IN") {
            in_val.as_f64()?
        } else {
            0.0
        };

        let splits = self.controller.evaluate_all(master);
        self.out_a = splits.first().map(|(_, v)| *v).unwrap_or(0.0);
        self.out_b = splits.get(1).map(|(_, v)| *v).unwrap_or(0.0);

        let mut res = BTreeMap::new();
        res.insert("OUT_A".to_string(), IecValue::LReal(self.out_a));
        res.insert("OUT_B".to_string(), IecValue::LReal(self.out_b));
        Ok(res)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "OUT_A" => Some(IecValue::LReal(self.out_a)),
            "OUT_B" => Some(IecValue::LReal(self.out_b)),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.out_a = 0.0;
        self.out_b = 0.0;
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

/// Control valve trim characterizer function block (VALVE_CHAR).
#[derive(Debug, Clone)]
pub struct ValveCharBlock {
    name: String,
    characterizer: ValveCharacterizer,
    output: f64,
}

impl ValveCharBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            characterizer: ValveCharacterizer::new(ValveCharacteristic::EqualPercentage {
                rangeability: 50.0,
            }),
            output: 0.0,
        }
    }
}

impl FunctionBlockInstance for ValveCharBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "VALVE_CHAR"
    }

    fn step(
        &mut self,
        _dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        if let Some(char_type) = inputs.get("CHAR_TYPE") {
            let t = char_type.as_i64()?;
            let r = if let Some(r_val) = inputs.get("RANGEABILITY") {
                r_val.as_f64()?.max(2.0)
            } else {
                50.0
            };
            self.characterizer.characteristic = match t {
                0 => ValveCharacteristic::Linear,
                1 => ValveCharacteristic::EqualPercentage { rangeability: r },
                2 => ValveCharacteristic::QuickOpening,
                _ => ValveCharacteristic::Linear,
            };
        }

        let in_lift = if let Some(in_val) = inputs.get("IN") {
            in_val.as_f64()?
        } else {
            0.0
        };

        // Normalize if 0-100%, or keep if 0-1
        let fraction = if in_lift > 1.0 {
            in_lift / 100.0
        } else {
            in_lift
        };
        self.output = self.characterizer.evaluate(fraction) * 100.0;

        let mut res = BTreeMap::new();
        res.insert("OUT".to_string(), IecValue::LReal(self.output));
        Ok(res)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "OUT" => Some(IecValue::LReal(self.output)),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.output = 0.0;
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}

/// Compressor anti-surge control function block (ANTI_SURGE).
#[derive(Debug, Clone)]
pub struct AntiSurgeBlock {
    name: String,
    controller: AntiSurgeController,
}

impl AntiSurgeBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            controller: AntiSurgeController::new(0.35, 0.5),
        }
    }
}

impl FunctionBlockInstance for AntiSurgeBlock {
    fn name(&self) -> &str {
        &self.name
    }

    fn fb_type(&self) -> &str {
        "ANTI_SURGE"
    }

    fn step(
        &mut self,
        dt: f64,
        _sim_time: f64,
        inputs: &BTreeMap<String, IecValue>,
    ) -> Result<BTreeMap<String, IecValue>, ControlError> {
        if let Some(m) = inputs.get("MARGIN") {
            self.controller.safety_margin = m.as_f64()?.clamp(0.01, 0.5);
        }

        let flow = inputs
            .get("FLOW")
            .map(|v| v.as_f64())
            .transpose()?
            .unwrap_or(0.0);
        let p_suc = inputs
            .get("P_SUCTION")
            .map(|v| v.as_f64())
            .transpose()?
            .unwrap_or(1.0);
        let p_dis = inputs
            .get("P_DISCHARGE")
            .map(|v| v.as_f64())
            .transpose()?
            .unwrap_or(2.0);

        let valve_cmd = self.controller.step(dt, flow, p_suc, p_dis);
        let margin = self.controller.surge_margin();
        let alarm = self.controller.alarm_state();

        let mut res = BTreeMap::new();
        res.insert("VALVE_OUT".to_string(), IecValue::LReal(valve_cmd));
        res.insert("SURGE_MARGIN".to_string(), IecValue::LReal(margin));
        res.insert(
            "IS_WARNING".to_string(),
            IecValue::Bool(alarm == AntiSurgeAlarmState::MarginWarning),
        );
        res.insert(
            "IS_CRITICAL".to_string(),
            IecValue::Bool(alarm == AntiSurgeAlarmState::SurgeCritical),
        );
        Ok(res)
    }

    fn get_output(&self, param: &str) -> Option<IecValue> {
        match param.to_ascii_uppercase().as_str() {
            "VALVE_OUT" => Some(IecValue::LReal(self.controller.valve_output())),
            "SURGE_MARGIN" => Some(IecValue::LReal(self.controller.surge_margin())),
            "IS_WARNING" => Some(IecValue::Bool(
                self.controller.alarm_state() == AntiSurgeAlarmState::MarginWarning,
            )),
            "IS_CRITICAL" => Some(IecValue::Bool(
                self.controller.alarm_state() == AntiSurgeAlarmState::SurgeCritical,
            )),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.controller.reset();
    }

    fn clone_box(&self) -> Box<dyn FunctionBlockInstance> {
        Box::new(self.clone())
    }
}
