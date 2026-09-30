//! Turbomachinery compressor anti-surge control block.
//!
//! Protects dynamic compressors (centrifugal and axial) from aerodynamic surge by
//! evaluating the active operating point relative to the compressor's Surge Limit Line (SLL)
//! and modulating a fast-acting hot gas bypass or recycle valve.
//!
//! Features:
//! - Surge Control Line (SCL) with configurable safety margin $d_{margin}$ (e.g. 10%).
//! - Real-time pressure ratio calculation $r_p = P_{discharge} / P_{suction}$.
//! - Proportional-Integral recycle valve demand calculation.
//! - Derivative rate-of-change trip kick for rapid flow decays.
//! - Alarm and interlock diagnostics (Advisory, Warning, Trip).

use serde::{Deserialize, Serialize};

/// Anti-surge operating status alarm level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AntiSurgeAlarmState {
    /// Safe operation well to the right of the Surge Control Line.
    Normal,
    /// Operating within the safety margin band between SCL and SLL.
    MarginWarning,
    /// Immediate threat: operating point at or below the Surge Limit Line.
    SurgeCritical,
}

/// Dynamic anti-surge controller block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AntiSurgeController {
    /// Linear slope of Surge Limit Line: $Q_{sll}(r_p) = a \cdot r_p + b$.
    pub sll_slope_a: f64,
    /// Intercept of Surge Limit Line: $Q_{sll}(r_p) = a \cdot r_p + b$.
    pub sll_intercept_b: f64,
    /// Safety margin fraction (typically $0.08\text{--}0.15$, default 0.10 for 10%).
    pub safety_margin: f64,
    /// Controller proportional gain $K_p$ for recycle valve opening.
    pub kp: f64,
    /// Controller integral time $T_i$ in seconds.
    pub ti: f64,
    /// Derivative kick gain for rapid rate-of-decay response.
    pub derivative_kick_gain: f64,
    /// Flow decay rate threshold (negative $\Delta Q / \Delta t$) triggering anticipatory kick.
    pub kick_threshold_rate: f64,

    // Runtime state
    integral: f64,
    prev_flow: f64,
    output_valve: f64,
    alarm_state: AntiSurgeAlarmState,
    surge_margin_fraction: f64,
}

impl AntiSurgeController {
    /// Creates an anti-surge controller with linear SLL parameters $a$ and $b$,
    /// standard 10% safety margin, and standard anti-surge gains.
    pub fn new(sll_slope_a: f64, sll_intercept_b: f64) -> Self {
        Self {
            sll_slope_a,
            sll_intercept_b,
            safety_margin: 0.10,
            kp: 2.0,
            ti: 1.0,
            derivative_kick_gain: 1.5,
            kick_threshold_rate: 2.0,
            integral: 0.0,
            prev_flow: 0.0,
            output_valve: 0.0,
            alarm_state: AntiSurgeAlarmState::Normal,
            surge_margin_fraction: 1.0,
        }
    }

    /// Calculates the minimum flow at the Surge Limit Line for pressure ratio $r_p$.
    pub fn calculate_sll_flow(&self, pressure_ratio: f64) -> f64 {
        let rp = pressure_ratio.max(1.0);
        (self.sll_slope_a * rp + self.sll_intercept_b).max(0.01)
    }

    /// Calculates the flow at the Surge Control Line for pressure ratio $r_p$.
    pub fn calculate_scl_flow(&self, pressure_ratio: f64) -> f64 {
        let sll = self.calculate_sll_flow(pressure_ratio);
        sll * (1.0 + self.safety_margin)
    }

    /// Current surge margin fraction: $(Q - Q_{sll}) / Q_{sll}$.
    pub fn surge_margin(&self) -> f64 {
        self.surge_margin_fraction
    }

    /// Current alarm status.
    pub fn alarm_state(&self) -> AntiSurgeAlarmState {
        self.alarm_state
    }

    /// Current commanded recycle valve opening ($0\text{--}100\%$).
    pub fn valve_output(&self) -> f64 {
        self.output_valve
    }

    /// Resets internal controller memory.
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.output_valve = 0.0;
        self.alarm_state = AntiSurgeAlarmState::Normal;
        self.surge_margin_fraction = 1.0;
    }

    /// Steps the anti-surge controller with live measurement samples.
    ///
    /// - `dt`: simulation timestep in seconds.
    /// - `flow`: measured compressor suction or discharge flow.
    /// - `p_suction`: measured suction pressure (absolute, e.g. bar).
    /// - `p_discharge`: measured discharge pressure (absolute, e.g. bar).
    ///
    /// Returns the commanded recycle valve opening ($0.0\text{--}100.0\%$).
    pub fn step(&mut self, dt: f64, flow: f64, p_suction: f64, p_discharge: f64) -> f64 {
        if !flow.is_finite() || p_suction <= 0.0 || p_discharge <= 0.0 || dt <= 0.0 {
            return self.output_valve;
        }

        let p_suc = p_suction.max(0.01);
        let p_dis = p_discharge.max(p_suc);
        let rp = p_dis / p_suc;

        let q_sll = self.calculate_sll_flow(rp);
        let q_scl = self.calculate_scl_flow(rp);

        // Surge margin fraction relative to surge limit line
        self.surge_margin_fraction = (flow - q_sll) / q_sll;

        // Alarm classification
        if flow <= q_sll {
            self.alarm_state = AntiSurgeAlarmState::SurgeCritical;
        } else if flow < q_scl {
            self.alarm_state = AntiSurgeAlarmState::MarginWarning;
        } else {
            self.alarm_state = AntiSurgeAlarmState::Normal;
        }

        // Error: how far flow is below the Surge Control Line
        // If flow < q_scl, err > 0 => open recycle valve
        let err = q_scl - flow;

        // Anticipatory derivative kick on fast flow drops
        let d_flow_dt = if dt > 0.0 {
            (flow - self.prev_flow) / dt
        } else {
            0.0
        };
        let mut kick = 0.0;
        if -d_flow_dt > self.kick_threshold_rate {
            // Flow dropping rapidly toward surge line!
            kick = self.derivative_kick_gain * (-d_flow_dt - self.kick_threshold_rate);
        }

        if err > 0.0 {
            // Operating inside the surge control region: calculate PI opening command
            let p_term = self.kp * err;

            if self.ti > 0.0 {
                self.integral += (self.kp * dt / self.ti) * err;
                self.integral = self.integral.clamp(0.0, 100.0);
            }

            let raw_out = p_term + self.integral + kick;
            self.output_valve = raw_out.clamp(0.0, 100.0);
        } else {
            // Operating in safe zone: slowly discharge integral to close valve smoothly
            if self.ti > 0.0 && self.integral > 0.0 {
                self.integral =
                    (self.integral - (self.kp * dt / (2.0 * self.ti)) * err.abs()).max(0.0);
            }
            // If kick is active, allow it to hold valve open briefly
            self.output_valve = (self.integral + kick).clamp(0.0, 100.0);
        }

        self.prev_flow = flow;
        self.output_valve
    }
}
