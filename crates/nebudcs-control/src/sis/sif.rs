//! Canonical Safety Instrumented Function (SIF) data model, lifecycle, and final elements.
//!
//! Standards Reference: IEC 61511-1 Clause 11 / ISA-84.
//! Note: Virtual simulation model for engineering training and verification.

use crate::actuator::{ActuatorCommand, CommandPriority};
use crate::runtime::IoImageTable;
use crate::sis::voting::{
    evaluate_voting, DegradationPolicy, SensorChannel, SifVotingResult, TripDirection,
    VotingArchitecture,
};
use nebudcs_model::tag::TagQuality;
use serde::{Deserialize, Serialize};

/// Operational lifecycle state of a Safety Instrumented Function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SifState {
    /// Armed: Monitoring inputs, all process variables within normal limits.
    #[default]
    Armed,
    /// DelayTiming: Setpoint exceeded, transient trip delay timer active.
    DelayTiming,
    /// Tripped: Safety limit breached, trip latched, final elements commanded to safe state.
    Tripped,
    /// Bypassed: Administratively bypassed or inhibited for maintenance/startup.
    Bypassed,
    /// Degraded: Operating normally but with one or more diagnosed sensor faults.
    Degraded,
    /// InProofTest: Function is undergoing simulated proof testing.
    InProofTest,
}

/// Target safe state for a final control element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SifSafeState {
    /// Fail closed (0.0% position) - e.g. feed isolation valve, fuel shutoff.
    ValveClosed,
    /// Fail open (100.0% position) - e.g. emergency depressuring, flare blowdown, cooling jacket.
    ValveOpen,
    /// Motor / Driver de-energized (0.0% speed/power) - e.g. feed pump trip.
    DriverTrip,
    /// Custom target position percentage (0.0% - 100.0%).
    CustomPosition(f64),
}

impl SifSafeState {
    /// Numeric target position (0.0 to 100.0%) commanded to actuator arbiter.
    pub fn target_position(&self) -> f64 {
        match self {
            Self::ValveClosed => 0.0,
            Self::ValveOpen => 100.0,
            Self::DriverTrip => 0.0,
            Self::CustomPosition(pos) => pos.clamp(0.0, 100.0),
        }
    }
}

/// Safety action classification for documentation and cause/effect reporting.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SifActionType {
    /// Immediate feed or reactant isolation.
    FeedIsolation,
    /// Emergency blowdown or depressuring to flare.
    EmergencyDepressuring,
    /// Rotating equipment driver trip (pump/compressor/turbine).
    DriverTrip,
    /// Heating energy cutoff (fired heater / electrical).
    HeatingCutoff,
    /// Emergency cooling or quenching injection.
    EmergencyCooling,
    /// General safety interlock action.
    GeneralSafetyTrip,
}

/// Configuration of a final element governed by an SIF.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SifFinalElement {
    /// Equipment identifier (e.g. "XV-101", "P-101").
    pub target_equipment: String,
    /// Controlled actuator variable (typically "opening" for valves, "speed" for pumps).
    #[serde(default = "default_actuator_var")]
    pub actuator_var: String,
    /// Demanded safe state when SIF trips.
    pub safe_state: SifSafeState,
    /// Final-element design philosophy: true = de-energize to trip (fail-safe), false = energize to trip.
    #[serde(default = "default_true")]
    pub de_energize_to_trip: bool,
    /// High-level safety action type.
    #[serde(default = "default_action_type")]
    pub action_type: SifActionType,
    /// Feedback monitoring tag (e.g. "ZSO-101" limit switch or position feedback).
    #[serde(default)]
    pub feedback_tag: Option<String>,
}

fn default_actuator_var() -> String {
    "opening".to_string()
}

fn default_true() -> bool {
    true
}

fn default_action_type() -> SifActionType {
    SifActionType::GeneralSafetyTrip
}

impl SifFinalElement {
    pub fn new(
        target_equipment: impl Into<String>,
        safe_state: SifSafeState,
        action_type: SifActionType,
    ) -> Self {
        Self {
            target_equipment: target_equipment.into(),
            actuator_var: "opening".to_string(),
            safe_state,
            de_energize_to_trip: true,
            action_type,
            feedback_tag: None,
        }
    }

    pub fn with_actuator_var(mut self, var: impl Into<String>) -> Self {
        self.actuator_var = var.into();
        self
    }

    pub fn with_feedback(mut self, tag: impl Into<String>) -> Self {
        self.feedback_tag = Some(tag.into());
        self
    }
}

/// Administrative bypass / maintenance inhibit tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SifBypass {
    /// Category of bypass (Maintenance, Startup, Calibration).
    pub bypass_type: String,
    /// Operator / Engineer identity authorizing bypass.
    pub authorized_by: String,
    /// Mandatory justification reason for audit trail.
    pub reason: String,
    /// Simulation time when bypass was initiated.
    pub start_sim_time: f64,
    /// Optional expiration time; if elapsed, bypass automatically cancels.
    pub duration_seconds: Option<f64>,
    /// Whether bypass is currently active.
    pub active: bool,
}

/// Live diagnostics telemetry for a Safety Instrumented Function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SifDiagnostics {
    pub total_sensors: usize,
    pub healthy_sensors: usize,
    pub faulted_sensors: usize,
    pub final_elements_count: usize,
    pub trip_count: u64,
    pub last_trip_sim_time: Option<f64>,
    pub last_trip_cause: Option<String>,
    pub last_reset_sim_time: Option<f64>,
    pub voting_scheme: String,
    pub delay_elapsed_seconds: f64,
}

/// Static engineering specification of a Safety Instrumented Function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SifConfig {
    /// Unique SIF identifier (e.g. "SIF-101-TAHH").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Engineering description.
    #[serde(default)]
    pub description: String,
    /// Associated hazard scenario reference (e.g. "HAZ-REACTOR-RUNAWAY").
    #[serde(default)]
    pub hazard_id: Option<String>,
    #[serde(default)]
    pub hazard_scenario: Option<String>,
    /// Engineering target SIL classification (metadata/simulation reference, NOT a certified claim).
    #[serde(default)]
    pub target_sil: Option<String>,
    /// Trip initiation direction (High / Low).
    #[serde(default)]
    pub direction: TripDirection,
    /// Safety trip threshold setpoint.
    pub threshold: f64,
    /// Margin below/above threshold required before reset is allowed.
    #[serde(default)]
    pub reset_deadband: f64,
    /// On-delay trip filter time in seconds.
    #[serde(default)]
    pub trip_delay_seconds: f64,
    /// Sensor voting scheme (1oo1, 1oo2, 2oo2, 2oo3, 2oo4).
    #[serde(default)]
    pub architecture: VotingArchitecture,
    /// Voting degradation policy on sensor channel fault.
    #[serde(default)]
    pub degradation_policy: DegradationPolicy,
    /// Bound sensor signal tags (1 to 4 tags depending on architecture).
    #[serde(default)]
    pub sensor_tags: Vec<String>,
    /// Commanded final control elements.
    #[serde(default)]
    pub final_elements: Vec<SifFinalElement>,
    /// Whether the trip latches and requires explicit manual operator reset.
    #[serde(default = "default_true")]
    pub latched: bool,
    /// Optional permissive tags that must be TRUE (1.0) before reset is permitted.
    #[serde(default)]
    pub reset_permissives: Vec<String>,
    /// Estimated probability of failure on demand (PFDavg) metadata for training calculations.
    #[serde(default)]
    pub pfd_avg_estimate: Option<f64>,
}

/// Runtime Safety Instrumented Function entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyInstrumentedFunction {
    config: SifConfig,
    state: SifState,
    bypass: Option<SifBypass>,
    delay_timer: f64,
    trip_latched: bool,
    diagnostics: SifDiagnostics,
    last_voting_result: Option<SifVotingResult>,
}

impl SafetyInstrumentedFunction {
    /// Creates a new SIF from static configuration in the Armed state.
    pub fn new(config: SifConfig) -> Self {
        let total = config.sensor_tags.len();
        let diag = SifDiagnostics {
            total_sensors: total,
            healthy_sensors: total,
            faulted_sensors: 0,
            final_elements_count: config.final_elements.len(),
            trip_count: 0,
            last_trip_sim_time: None,
            last_trip_cause: None,
            last_reset_sim_time: None,
            voting_scheme: format!("{:?}", config.architecture),
            delay_elapsed_seconds: 0.0,
        };
        Self {
            config,
            state: SifState::Armed,
            bypass: None,
            delay_timer: 0.0,
            trip_latched: false,
            diagnostics: diag,
            last_voting_result: None,
        }
    }

    pub fn id(&self) -> &str {
        &self.config.id
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn config(&self) -> &SifConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut SifConfig {
        &mut self.config
    }

    pub fn state(&self) -> SifState {
        self.state
    }

    pub fn is_tripped(&self) -> bool {
        self.state == SifState::Tripped
    }

    pub fn is_bypassed(&self) -> bool {
        self.state == SifState::Bypassed
    }

    pub fn diagnostics(&self) -> &SifDiagnostics {
        &self.diagnostics
    }

    pub fn last_voting_result(&self) -> Option<&SifVotingResult> {
        self.last_voting_result.as_ref()
    }

    pub fn bypass_info(&self) -> Option<&SifBypass> {
        self.bypass.as_ref()
    }

    /// Sets or clears an administrative bypass.
    pub fn set_bypass(&mut self, bypass: Option<SifBypass>) {
        if let Some(b) = bypass {
            if b.active {
                self.bypass = Some(b);
                self.state = SifState::Bypassed;
            } else {
                self.bypass = None;
                self.state = if self.trip_latched {
                    SifState::Tripped
                } else {
                    SifState::Armed
                };
            }
        } else {
            self.bypass = None;
            self.state = if self.trip_latched {
                SifState::Tripped
            } else {
                SifState::Armed
            };
        }
    }

    /// Attempts to manually reset a tripped SIF.
    ///
    /// Requires:
    /// 1. SIF is Tripped or Latched
    /// 2. Process condition has cleared outside reset deadband
    /// 3. All reset permissives are satisfied
    pub fn request_reset(&mut self, sim_time: f64, io_image: &IoImageTable) -> Result<(), String> {
        if !self.trip_latched && self.state != SifState::Tripped {
            return Ok(()); // Already normal
        }

        // 1. Check if initiating condition has cleared
        let sensors = self.read_sensor_channels(io_image);
        let cleared = match self.config.direction {
            TripDirection::High => {
                let clear_thresh = self.config.threshold - self.config.reset_deadband;
                sensors
                    .iter()
                    .all(|s| !s.is_healthy() || s.value < clear_thresh)
            }
            TripDirection::Low => {
                let clear_thresh = self.config.threshold + self.config.reset_deadband;
                sensors
                    .iter()
                    .all(|s| !s.is_healthy() || s.value > clear_thresh)
            }
        };

        if !cleared {
            return Err(format!(
                "SIF '{}' reset rejected: initiating process condition has not cleared within deadband",
                self.config.id
            ));
        }

        // 2. Check all configured reset permissives
        for perm in &self.config.reset_permissives {
            let p_val = io_image.read_input_value(perm).unwrap_or(0.0);
            if p_val <= 0.0 {
                return Err(format!(
                    "SIF '{}' reset rejected: reset permissive '{}' not satisfied",
                    self.config.id, perm
                ));
            }
        }

        // 3. Clear trip latch
        self.trip_latched = false;
        self.delay_timer = 0.0;
        self.diagnostics.last_reset_sim_time = Some(sim_time);
        self.state = if self.bypass.is_some() {
            SifState::Bypassed
        } else {
            SifState::Armed
        };

        Ok(())
    }

    /// Reads configured sensor channels from the controller I/O table.
    fn read_sensor_channels(&self, io_image: &IoImageTable) -> Vec<SensorChannel> {
        let mut channels = Vec::new();
        for tag in &self.config.sensor_tags {
            if let Some(sig) = io_image.read_input(tag) {
                let mut ch = SensorChannel::new(tag.clone(), tag.clone(), sig.value);
                ch = ch.with_quality(sig.quality);
                channels.push(ch);
            } else {
                // Unmapped or missing tag -> treat as Bad quality
                let ch =
                    SensorChannel::new(tag.clone(), tag.clone(), 0.0).with_quality(TagQuality::Bad);
                channels.push(ch);
            }
        }
        channels
    }

    /// Deterministic 50 Hz simulation tick execution of the SIF.
    ///
    /// Generates priority ActuatorCommands when in Tripped state.
    pub fn step(
        &mut self,
        dt: f64,
        sim_time: f64,
        io_image: &IoImageTable,
        actuator_commands: &mut Vec<(String, ActuatorCommand)>,
    ) {
        // 1. Check bypass expiration
        if let Some(ref b) = self.bypass {
            if let Some(dur) = b.duration_seconds {
                if sim_time - b.start_sim_time >= dur {
                    self.bypass = None;
                    self.state = if self.trip_latched {
                        SifState::Tripped
                    } else {
                        SifState::Armed
                    };
                }
            }
        }

        // 2. Read sensors and evaluate voting logic
        let channels = self.read_sensor_channels(io_image);
        let voting_res = evaluate_voting(
            self.config.architecture,
            self.config.degradation_policy,
            self.config.direction,
            self.config.threshold,
            &channels,
        );

        self.diagnostics.total_sensors = channels.len();
        self.diagnostics.healthy_sensors = voting_res.healthy_channels;
        self.diagnostics.faulted_sensors = voting_res.faulted_channels;
        self.diagnostics.voting_scheme = voting_res.effective_scheme.clone();
        let trip_demanded = voting_res.trip_demanded;
        self.last_voting_result = Some(voting_res);

        // 3. Evaluate SIF operational state transitions
        if self.bypass.is_some() {
            self.state = SifState::Bypassed;
            self.delay_timer = 0.0;
        } else if self.trip_latched {
            self.state = SifState::Tripped;
        } else if trip_demanded {
            if self.config.trip_delay_seconds > 0.0 {
                self.delay_timer += dt;
                self.diagnostics.delay_elapsed_seconds = self.delay_timer;
                if self.delay_timer >= self.config.trip_delay_seconds {
                    self.trip_latched = self.config.latched;
                    self.state = SifState::Tripped;
                    self.diagnostics.trip_count += 1;
                    self.diagnostics.last_trip_sim_time = Some(sim_time);
                    self.diagnostics.last_trip_cause = Some(format!(
                        "Voting condition met: {} (threshold: {:.2})",
                        self.diagnostics.voting_scheme, self.config.threshold
                    ));
                } else {
                    self.state = SifState::DelayTiming;
                }
            } else {
                self.trip_latched = self.config.latched;
                self.state = SifState::Tripped;
                self.diagnostics.trip_count += 1;
                self.diagnostics.last_trip_sim_time = Some(sim_time);
                self.diagnostics.last_trip_cause = Some(format!(
                    "Voting condition met: {} (threshold: {:.2})",
                    self.diagnostics.voting_scheme, self.config.threshold
                ));
            }
        } else {
            self.delay_timer = 0.0;
            self.diagnostics.delay_elapsed_seconds = 0.0;
            if self.diagnostics.faulted_sensors > 0 {
                self.state = SifState::Degraded;
            } else {
                self.state = SifState::Armed;
            }
        }

        // 4. Assert final element safe state commands if Tripped
        if self.state == SifState::Tripped {
            for fe in &self.config.final_elements {
                let target_pos = fe.safe_state.target_position();
                let cmd = ActuatorCommand::new(CommandPriority::Esd, target_pos)
                    .with_source(format!("SIS:{}", self.config.id));
                actuator_commands.push((fe.target_equipment.clone(), cmd));
            }
        }
    }
}
