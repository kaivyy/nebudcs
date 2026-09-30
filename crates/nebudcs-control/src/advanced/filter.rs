//! Signal conditioning and dynamic compensation filters for advanced process control.
//!
//! Provides:
//! - [`FirstOrderFilter`]: First-order exponential lag filter (noise suppression / damping).
//! - [`LeadLagCompensator`]: Dynamic lead-lag compensator for feedforward disturbance matching.
//! - [`DeadTimeBuffer`]: Ring buffer for transport delay simulation.

use serde::{Deserialize, Serialize};

/// First-order lag (low-pass) filter.
///
/// Continuous transfer function: $G(s) = \frac{1}{1 + T_f s}$.
/// Discrete recurrence: $y[k] = y[k-1] + \alpha (u[k] - y[k-1])$,
/// where $\alpha = \frac{\Delta t}{\Delta t + T_f}$.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirstOrderFilter {
    /// Filter time constant $T_f$ in seconds.
    pub time_constant: f64,
    /// Current filtered output value $y[k-1]$.
    output: f64,
    /// Whether the filter has processed its first sample.
    initialized: bool,
}

impl FirstOrderFilter {
    /// Creates a new first-order filter with the given time constant in seconds.
    pub fn new(time_constant: f64) -> Self {
        Self {
            time_constant: time_constant.max(0.0),
            output: 0.0,
            initialized: false,
        }
    }

    /// Initializes or pre-seeds the filter at a specific steady-state value.
    pub fn initialize(&mut self, initial_val: f64) {
        self.output = initial_val;
        self.initialized = true;
    }

    /// Current filtered output.
    pub fn output(&self) -> f64 {
        self.output
    }

    /// Resets filter state.
    pub fn reset(&mut self) {
        self.output = 0.0;
        self.initialized = false;
    }

    /// Steps the filter forward by timestep $\Delta t$ with input sample $u$.
    pub fn step(&mut self, dt: f64, u: f64) -> f64 {
        if !u.is_finite() || dt <= 0.0 {
            return self.output;
        }

        if !self.initialized {
            self.output = u;
            self.initialized = true;
            return self.output;
        }

        if self.time_constant <= 1e-9 {
            self.output = u;
            return self.output;
        }

        let alpha = dt / (dt + self.time_constant);
        self.output += alpha * (u - self.output);
        self.output
    }
}

/// Dynamic lead-lag compensator.
///
/// Continuous transfer function:
/// $$G(s) = \frac{1 + T_{lead} s}{1 + T_{lag} s}$$
///
/// Extensively utilized in feed-forward disturbance rejection to synchronize the
/// timing and shape of disturbance corrections with the process response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeadLagCompensator {
    /// Lead time constant $T_{lead}$ in seconds.
    pub t_lead: f64,
    /// Lag time constant $T_{lag}$ in seconds.
    pub t_lag: f64,
    /// Overall static gain $K$.
    pub gain: f64,
    /// Internal filter state.
    state: f64,
    /// Current output.
    output: f64,
    initialized: bool,
}

impl LeadLagCompensator {
    /// Creates a new lead-lag compensator with $T_{lead}$, $T_{lag}$, and unit gain.
    pub fn new(t_lead: f64, t_lag: f64) -> Self {
        Self::with_gain(t_lead, t_lag, 1.0)
    }

    /// Creates a lead-lag compensator with explicit gain $K$.
    pub fn with_gain(t_lead: f64, t_lag: f64, gain: f64) -> Self {
        Self {
            t_lead: t_lead.max(0.0),
            t_lag: t_lag.max(0.0),
            gain,
            state: 0.0,
            output: 0.0,
            initialized: false,
        }
    }

    /// Initializes the compensator at a known steady-state input value.
    pub fn initialize(&mut self, u_initial: f64) {
        self.state = u_initial;
        self.output = self.gain * u_initial;
        self.initialized = true;
    }

    /// Current output value.
    pub fn output(&self) -> f64 {
        self.output
    }

    /// Resets internal state.
    pub fn reset(&mut self) {
        self.state = 0.0;
        self.output = 0.0;
        self.initialized = false;
    }

    /// Executes one discrete calculation step.
    pub fn step(&mut self, dt: f64, u: f64) -> f64 {
        if !u.is_finite() || dt <= 0.0 {
            return self.output;
        }

        if !self.initialized {
            self.initialize(u);
            return self.output;
        }

        // If T_lag is negligible, output follows scaled input directly
        if self.t_lag <= 1e-9 {
            self.output = self.gain * u;
            return self.output;
        }

        let alpha = dt / (dt + self.t_lag);
        let prev_state = self.state;
        self.state += alpha * (u - prev_state);

        // State-space output equation:
        // y = K * [ state + (T_lead / T_lag) * (u - state) ]
        let lead_ratio = self.t_lead / self.t_lag;
        let y_raw = self.state + lead_ratio * (u - self.state);

        self.output = self.gain * y_raw;
        self.output
    }
}

/// Discrete transport delay (dead-time) buffer.
///
/// Implements $y(t) = u(t - \theta)$ using a preallocated ring buffer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeadTimeBuffer {
    /// Dead-time duration $\theta$ in seconds.
    pub dead_time: f64,
    /// Fixed sampling timestep $\Delta t$ in seconds.
    pub dt: f64,
    /// Ring buffer storing past input samples.
    buffer: Vec<f64>,
    /// Write pointer.
    head: usize,
    /// Current output.
    output: f64,
}

impl DeadTimeBuffer {
    /// Creates a dead-time buffer with duration $\theta$ and timestep $\Delta t$.
    pub fn new(dead_time: f64, dt: f64, initial_val: f64) -> Self {
        let dt_clamped = dt.max(0.001);
        let steps = ((dead_time.max(0.0) / dt_clamped).round() as usize).max(1);
        let buffer = vec![initial_val; steps];
        Self {
            dead_time: dead_time.max(0.0),
            dt: dt_clamped,
            buffer,
            head: 0,
            output: initial_val,
        }
    }

    /// Resets buffer with a specific value.
    pub fn reset(&mut self, val: f64) {
        self.buffer.fill(val);
        self.head = 0;
        self.output = val;
    }

    /// Current output.
    pub fn output(&self) -> f64 {
        self.output
    }

    /// Steps the dead-time buffer with sample $u$.
    pub fn step(&mut self, u: f64) -> f64 {
        if !u.is_finite() || self.buffer.is_empty() {
            return self.output;
        }

        // Read oldest sample
        self.output = self.buffer[self.head];
        // Overwrite with new sample
        self.buffer[self.head] = u;
        // Advance ring pointer
        self.head = (self.head + 1) % self.buffer.len();

        self.output
    }
}
