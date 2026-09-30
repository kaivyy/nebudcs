//! Concrete channel types for Analog and Digital I/O.

use nebudcs_types::ChannelId;
use serde::{Deserialize, Serialize};

use crate::hal::address::IoChannelAddress;
use crate::hal::scaling::AnalogScalingConfig;
use crate::hal::signal::{
    AnalogFaultState, ChannelStatus, DigitalFallbackBehavior, DigitalFaultState, IoSignalQuality,
    OutputFallbackBehavior,
};

/// Analog Input Channel representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalogInputChannel {
    pub id: ChannelId,
    pub channel_number: u8,
    pub address: IoChannelAddress,
    pub name: String,
    pub raw_counts: i32,
    pub engineering_value: f64,
    pub scaling: AnalogScalingConfig,
    pub quality: IoSignalQuality,
    pub fault_state: AnalogFaultState,
    pub forced_fault: Option<AnalogFaultState>,
    pub forced_value: Option<f64>,
    pub status: ChannelStatus,
    pub last_updated_sim: f64,
}

impl AnalogInputChannel {
    /// Creates a new Analog Input Channel.
    pub fn new(
        channel_number: u8,
        address: IoChannelAddress,
        name: impl Into<String>,
        scaling: AnalogScalingConfig,
    ) -> Self {
        let (raw_min, _) = scaling.signal_range.default_raw_bounds();
        Self {
            id: ChannelId::new(),
            channel_number,
            address,
            name: name.into(),
            raw_counts: raw_min,
            engineering_value: scaling.lrv,
            scaling,
            quality: IoSignalQuality::Good,
            fault_state: AnalogFaultState::Normal,
            forced_fault: None,
            forced_value: None,
            status: ChannelStatus::Enabled,
            last_updated_sim: 0.0,
        }
    }

    /// Samples a physical process value, applying scaling, calibration, and NAMUR NE43 checks.
    pub fn sample(&mut self, field_eu: f64, sim_time: f64) {
        self.last_updated_sim = sim_time;

        if self.status == ChannelStatus::Disabled {
            self.quality = IoSignalQuality::Disabled;
            return;
        }

        // Check if a forced fault injection is active
        if let Some(fault) = self.forced_fault {
            self.fault_state = fault;
            match fault {
                AnalogFaultState::WireBreak => {
                    self.raw_counts = self.scaling.raw_min - 2000;
                    self.engineering_value = self.scaling.lrv;
                    self.quality = IoSignalQuality::Bad;
                    return;
                }
                AnalogFaultState::ShortCircuit => {
                    self.raw_counts = self.scaling.raw_max + 3000;
                    self.engineering_value = self.scaling.urv;
                    self.quality = IoSignalQuality::Bad;
                    return;
                }
                AnalogFaultState::Frozen => {
                    self.quality = IoSignalQuality::Uncertain;
                    return;
                }
                AnalogFaultState::UnderRange => {
                    self.raw_counts = self.scaling.raw_min - 300;
                    self.quality = IoSignalQuality::Uncertain;
                }
                AnalogFaultState::OverRange => {
                    self.raw_counts = self.scaling.raw_max + 300;
                    self.quality = IoSignalQuality::Uncertain;
                }
                _ => {}
            }
        }

        // Check if an explicit forced value override is active
        if let Some(forced_val) = self.forced_value {
            self.engineering_value = forced_val;
            self.quality = IoSignalQuality::Uncertain;
            return;
        }

        // Normal physical sampling: convert field EU to raw converter counts
        if let Ok((raw, _)) = self.scaling.eu_to_raw(field_eu) {
            self.raw_counts = raw;
        }

        // Transduce raw counts back to conditioned EU with NAMUR NE43 check
        if let Ok((eu, q, f)) = self.scaling.raw_to_eu(self.raw_counts) {
            self.engineering_value = eu;
            self.quality = q;
            self.fault_state = f;
        }
    }
}

/// Analog Output Channel representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalogOutputChannel {
    pub id: ChannelId,
    pub channel_number: u8,
    pub address: IoChannelAddress,
    pub name: String,
    pub command_eu: f64,
    pub actual_eu: f64,
    pub raw_counts: i32,
    pub scaling: AnalogScalingConfig,
    pub fallback_behavior: OutputFallbackBehavior,
    pub quality: IoSignalQuality,
    pub status: ChannelStatus,
    pub last_updated_sim: f64,
}

impl AnalogOutputChannel {
    /// Creates a new Analog Output Channel.
    pub fn new(
        channel_number: u8,
        address: IoChannelAddress,
        name: impl Into<String>,
        scaling: AnalogScalingConfig,
        fallback_behavior: OutputFallbackBehavior,
    ) -> Self {
        let (raw_min, _) = scaling.signal_range.default_raw_bounds();
        Self {
            id: ChannelId::new(),
            channel_number,
            address,
            name: name.into(),
            command_eu: scaling.lrv,
            actual_eu: scaling.lrv,
            raw_counts: raw_min,
            scaling,
            fallback_behavior,
            quality: IoSignalQuality::Good,
            status: ChannelStatus::Enabled,
            last_updated_sim: 0.0,
        }
    }

    /// Commands an output engineering value from a virtual controller (%Q).
    pub fn write_command(&mut self, cmd_eu: f64, sim_time: f64) {
        self.last_updated_sim = sim_time;
        self.command_eu = cmd_eu;

        if self.status == ChannelStatus::Disabled {
            self.apply_fallback(sim_time);
            return;
        }

        if let Ok((raw, q)) = self.scaling.eu_to_raw(cmd_eu) {
            self.raw_counts = raw;
            self.actual_eu = cmd_eu;
            self.quality = q;
        }
    }

    /// Applies configured fallback behavior when controller is faulted, stopped, or communication is lost.
    pub fn apply_fallback(&mut self, sim_time: f64) {
        self.last_updated_sim = sim_time;
        match self.fallback_behavior {
            OutputFallbackBehavior::FailOpen => {
                self.actual_eu = self.scaling.urv;
                if let Ok((raw, _)) = self.scaling.eu_to_raw(self.scaling.urv) {
                    self.raw_counts = raw;
                }
            }
            OutputFallbackBehavior::FailClosed => {
                self.actual_eu = self.scaling.lrv;
                if let Ok((raw, _)) = self.scaling.eu_to_raw(self.scaling.lrv) {
                    self.raw_counts = raw;
                }
            }
            OutputFallbackBehavior::HoldLast => {
                // Keep actual_eu and raw_counts as-is
            }
            OutputFallbackBehavior::ForcedValue(v) => {
                self.actual_eu = v;
                if let Ok((raw, _)) = self.scaling.eu_to_raw(v) {
                    self.raw_counts = raw;
                }
            }
            OutputFallbackBehavior::Zero => {
                self.actual_eu = 0.0;
                self.raw_counts = 0;
            }
        }
        self.quality = IoSignalQuality::Uncertain;
    }
}

/// Digital Input Channel representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DigitalInputChannel {
    pub id: ChannelId,
    pub channel_number: u8,
    pub address: IoChannelAddress,
    pub name: String,
    pub raw_state: bool,
    pub state: bool,
    pub invert: bool,
    pub debounce_time_sec: f64,
    pub last_transition_sim: f64,
    pub pending_raw_state: bool,
    pub forced_fault: Option<DigitalFaultState>,
    pub quality: IoSignalQuality,
    pub status: ChannelStatus,
    pub last_updated_sim: f64,
}

impl DigitalInputChannel {
    /// Creates a new Digital Input Channel.
    pub fn new(
        channel_number: u8,
        address: IoChannelAddress,
        name: impl Into<String>,
        invert: bool,
    ) -> Self {
        Self {
            id: ChannelId::new(),
            channel_number,
            address,
            name: name.into(),
            raw_state: false,
            state: invert,
            invert,
            debounce_time_sec: 0.02, // 20ms industrial contact debounce
            last_transition_sim: 0.0,
            pending_raw_state: false,
            forced_fault: None,
            quality: IoSignalQuality::Good,
            status: ChannelStatus::Enabled,
            last_updated_sim: 0.0,
        }
    }

    /// Samples field contact state with debounce filter and polarity inversion.
    pub fn sample(&mut self, field_contact: bool, sim_time: f64) {
        self.last_updated_sim = sim_time;

        if self.status == ChannelStatus::Disabled {
            self.quality = IoSignalQuality::Disabled;
            return;
        }

        // Apply forced fault injection if present
        if let Some(fault) = self.forced_fault {
            match fault {
                DigitalFaultState::StuckTrue => {
                    self.raw_state = true;
                    self.state = true;
                    self.quality = IoSignalQuality::Uncertain;
                    return;
                }
                DigitalFaultState::StuckFalse => {
                    self.raw_state = false;
                    self.state = false;
                    self.quality = IoSignalQuality::Uncertain;
                    return;
                }
                DigitalFaultState::Inverted => {
                    self.raw_state = field_contact;
                    self.state = !field_contact;
                    self.quality = IoSignalQuality::Good;
                    return;
                }
                _ => {}
            }
        }

        // Debounce logic
        if field_contact != self.raw_state {
            if field_contact != self.pending_raw_state {
                self.pending_raw_state = field_contact;
                self.last_transition_sim = sim_time;
            } else if (sim_time - self.last_transition_sim) >= self.debounce_time_sec {
                self.raw_state = field_contact;
                self.state = if self.invert {
                    !field_contact
                } else {
                    field_contact
                };
            }
        } else {
            self.pending_raw_state = field_contact;
            self.state = if self.invert {
                !field_contact
            } else {
                field_contact
            };
        }

        self.quality = IoSignalQuality::Good;
    }
}

/// Digital Output Channel representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DigitalOutputChannel {
    pub id: ChannelId,
    pub channel_number: u8,
    pub address: IoChannelAddress,
    pub name: String,
    pub commanded_state: bool,
    pub applied_state: bool,
    pub fallback_behavior: DigitalFallbackBehavior,
    pub forced_fault: Option<DigitalFaultState>,
    pub quality: IoSignalQuality,
    pub status: ChannelStatus,
    pub last_updated_sim: f64,
}

impl DigitalOutputChannel {
    /// Creates a new Digital Output Channel.
    pub fn new(
        channel_number: u8,
        address: IoChannelAddress,
        name: impl Into<String>,
        fallback_behavior: DigitalFallbackBehavior,
    ) -> Self {
        Self {
            id: ChannelId::new(),
            channel_number,
            address,
            name: name.into(),
            commanded_state: false,
            applied_state: false,
            fallback_behavior,
            forced_fault: None,
            quality: IoSignalQuality::Good,
            status: ChannelStatus::Enabled,
            last_updated_sim: 0.0,
        }
    }

    /// Commands a discrete state from a virtual controller (%Q).
    pub fn write_command(&mut self, cmd: bool, sim_time: f64) {
        self.last_updated_sim = sim_time;
        self.commanded_state = cmd;

        if self.status == ChannelStatus::Disabled {
            self.apply_fallback(sim_time);
            return;
        }

        if let Some(fault) = self.forced_fault {
            match fault {
                DigitalFaultState::StuckTrue => {
                    self.applied_state = true;
                    self.quality = IoSignalQuality::Uncertain;
                    return;
                }
                DigitalFaultState::StuckFalse => {
                    self.applied_state = false;
                    self.quality = IoSignalQuality::Uncertain;
                    return;
                }
                _ => {}
            }
        }

        self.applied_state = cmd;
        self.quality = IoSignalQuality::Good;
    }

    /// Applies discrete fallback behavior upon communication or controller failure.
    pub fn apply_fallback(&mut self, sim_time: f64) {
        self.last_updated_sim = sim_time;
        match self.fallback_behavior {
            DigitalFallbackBehavior::FailOpen => self.applied_state = false,
            DigitalFallbackBehavior::FailClosed => self.applied_state = true,
            DigitalFallbackBehavior::HoldLast => {}
        }
        self.quality = IoSignalQuality::Uncertain;
    }
}

/// Generalized polymorphic wrapper for an I/O channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum IoChannel {
    Ai(AnalogInputChannel),
    Ao(AnalogOutputChannel),
    Di(DigitalInputChannel),
    Do(DigitalOutputChannel),
}

impl IoChannel {
    /// Channel unique identifier.
    pub fn id(&self) -> ChannelId {
        match self {
            Self::Ai(c) => c.id,
            Self::Ao(c) => c.id,
            Self::Di(c) => c.id,
            Self::Do(c) => c.id,
        }
    }

    /// Structured hardware address.
    pub fn address(&self) -> IoChannelAddress {
        match self {
            Self::Ai(c) => c.address,
            Self::Ao(c) => c.address,
            Self::Di(c) => c.address,
            Self::Do(c) => c.address,
        }
    }

    /// 1-based channel index within module.
    pub fn channel_number(&self) -> u8 {
        match self {
            Self::Ai(c) => c.channel_number,
            Self::Ao(c) => c.channel_number,
            Self::Di(c) => c.channel_number,
            Self::Do(c) => c.channel_number,
        }
    }

    /// User-configured channel tag or name.
    pub fn name(&self) -> &str {
        match self {
            Self::Ai(c) => &c.name,
            Self::Ao(c) => &c.name,
            Self::Di(c) => &c.name,
            Self::Do(c) => &c.name,
        }
    }

    /// Operational channel status.
    pub fn status(&self) -> ChannelStatus {
        match self {
            Self::Ai(c) => c.status,
            Self::Ao(c) => c.status,
            Self::Di(c) => c.status,
            Self::Do(c) => c.status,
        }
    }

    /// Signal data quality.
    pub fn quality(&self) -> IoSignalQuality {
        match self {
            Self::Ai(c) => c.quality,
            Self::Ao(c) => c.quality,
            Self::Di(c) => c.quality,
            Self::Do(c) => c.quality,
        }
    }

    /// Simulation timestamp of last update.
    pub fn last_updated_sim(&self) -> f64 {
        match self {
            Self::Ai(c) => c.last_updated_sim,
            Self::Ao(c) => c.last_updated_sim,
            Self::Di(c) => c.last_updated_sim,
            Self::Do(c) => c.last_updated_sim,
        }
    }

    /// Value coerced to floating point representation for generic routing.
    pub fn value_as_float(&self) -> f64 {
        match self {
            Self::Ai(c) => c.engineering_value,
            Self::Ao(c) => c.actual_eu,
            Self::Di(c) => {
                if c.state {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Do(c) => {
                if c.applied_state {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    /// Value coerced to boolean representation for discrete logic.
    pub fn value_as_bool(&self) -> bool {
        match self {
            Self::Ai(c) => c.engineering_value >= 0.5,
            Self::Ao(c) => c.actual_eu >= 0.5,
            Self::Di(c) => c.state,
            Self::Do(c) => c.applied_state,
        }
    }
}
