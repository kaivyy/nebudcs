//! Signal types, quality definitions, and fault models for the I/O HAL.

use nebudcs_model::tag::TagQuality;
use serde::{Deserialize, Serialize};

/// Data quality classification for I/O signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoSignalQuality {
    /// Normal valid measurement within operating limits.
    Good,
    /// Degraded confidence, under-range/over-range warnings, or manual override.
    Uncertain,
    /// Wire-break, short-circuit, communication loss, or invalid hardware state.
    Bad,
    /// Hardware channel explicitly disabled by operator or engineer.
    Disabled,
    /// Process variable exceeded physical transducer limits.
    OutOfRange,
    /// Sensor hardware element fault (diaphragm rupture, open RTD).
    SensorFault,
    /// Communication failure on chassis backplane or remote I/O bus.
    CommunicationFault,
    /// Invalid scaling or address configuration.
    ConfigurationFault,
}

impl IoSignalQuality {
    /// Whether the signal can be safely consumed by normal regulatory control loops.
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Good | Self::Uncertain)
    }

    /// Whether this indicates a hard failure condition.
    pub fn is_bad(&self) -> bool {
        !self.is_usable()
    }
}

impl From<IoSignalQuality> for TagQuality {
    fn from(q: IoSignalQuality) -> Self {
        match q {
            IoSignalQuality::Good => TagQuality::Good,
            IoSignalQuality::Uncertain => TagQuality::Uncertain,
            IoSignalQuality::Bad
            | IoSignalQuality::Disabled
            | IoSignalQuality::OutOfRange
            | IoSignalQuality::SensorFault
            | IoSignalQuality::CommunicationFault
            | IoSignalQuality::ConfigurationFault => TagQuality::Bad,
        }
    }
}

impl std::fmt::Display for IoSignalQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Standard industrial analog electrical signaling ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AnalogSignalRange {
    /// 4–20 mA DC current loop (NAMUR NE43 standard: 3.8-20.5mA normal).
    #[default]
    Current4_20mA,
    /// 0–20 mA DC current loop.
    Current0_20mA,
    /// 0–10 V DC voltage signaling.
    Voltage0_10V,
    /// -10 V to +10 V DC bipolar voltage.
    VoltageMinus10_10V,
    /// Custom raw ADC/DAC counts.
    CustomCounts { raw_min: i32, raw_max: i32 },
}

impl AnalogSignalRange {
    /// Default raw integer count span for 16-bit converters.
    pub fn default_raw_bounds(&self) -> (i32, i32) {
        match self {
            // For 4-20mA on a standard 16-bit 0-20mA ADC:
            // 0 mA = 0 counts, 4 mA = 6554 counts, 20 mA = 32767 counts
            Self::Current4_20mA => (6554, 32767),
            Self::Current0_20mA => (0, 32767),
            Self::Voltage0_10V => (0, 32767),
            Self::VoltageMinus10_10V => (-32768, 32767),
            Self::CustomCounts { raw_min, raw_max } => (*raw_min, *raw_max),
        }
    }
}

/// Mathematical transduction curve applied during signal conditioning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConversionCurve {
    /// Linear scaling: $EU = LRV + \frac{raw - raw_{min}}{raw_{max} - raw_{min}} (URV - LRV)$.
    #[default]
    Linear,
    /// Square root extraction (e.g. for DP flow measurements): $EU = LRV + \sqrt{fraction} (URV - LRV)$.
    SquareRoot,
    /// Polynomial calibration: $EU = c_0 + c_1 \cdot x + c_2 \cdot x^2 + c_3 \cdot x^3$.
    Polynomial([f64; 4]),
}

/// Fault classification for analog channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalogFaultState {
    /// Normal operating condition.
    Normal,
    /// NAMUR NE43 under-range condition (3.6 mA <= I < 3.8 mA).
    UnderRange,
    /// NAMUR NE43 over-range condition (20.5 mA < I <= 21.0 mA).
    OverRange,
    /// NAMUR NE43 wire break / open circuit (I < 3.6 mA).
    WireBreak,
    /// NAMUR NE43 short-circuit / high fault (I > 21.0 mA).
    ShortCircuit,
    /// Sensor output frozen.
    Frozen,
    /// Calibration drift error.
    CalibrationDrift,
}

/// Fault classification for digital channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DigitalFaultState {
    /// Normal operating condition.
    Normal,
    /// Signal stuck at TRUE regardless of field contact.
    StuckTrue,
    /// Signal stuck at FALSE regardless of field contact.
    StuckFalse,
    /// Inverted electrical contact.
    Inverted,
    /// Contact chatter / bouncing.
    Bouncing,
}

/// Fallback behavior when an analog output channel loses controller communications or faults.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutputFallbackBehavior {
    /// Fail Open (sends 100% or URV).
    FailOpen,
    /// Fail Closed (sends 0% or LRV).
    #[default]
    FailClosed,
    /// Hold Last Value prior to failure.
    HoldLast,
    /// Drive output to explicit configured safe value.
    ForcedValue(f64),
    /// Zero raw output (0 mA / 0 counts).
    Zero,
}

/// Fallback behavior for discrete digital outputs on communication or controller failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DigitalFallbackBehavior {
    /// De-energize output relay / open contact (FALSE).
    #[default]
    FailOpen,
    /// Energize output relay / close contact (TRUE).
    FailClosed,
    /// Hold last commanded state.
    HoldLast,
}

/// Operational status of an I/O channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChannelStatus {
    /// Channel is active and scanning.
    #[default]
    Enabled,
    /// Channel is administratively disabled.
    Disabled,
    /// Channel has detected a hardware or wire fault.
    Faulted,
}
