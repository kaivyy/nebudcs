//! Canonical industrial data types, timestamps, quality flags, and tag models.
//!
//! Provides the foundational data contracts for all external industrial protocols
//! (Modbus TCP, OPC UA, MQTT, Sparkplug B) across ISA-95 Levels 0 through 4.

use serde::{Deserialize, Serialize};

/// Strongly typed industrial value supporting integer, floating point, boolean, string,
/// enumerated, and byte-array payloads. Not reduced to raw f64.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum IndustrialDataValue {
    Boolean(bool),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Int64(i64),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
    String(String),
    Enumeration { name: String, ordinal: i32 },
    ByteArray(Vec<u8>),
}

impl IndustrialDataValue {
    /// Extracts the numeric value as an `f64` for analog processing if applicable.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
            Self::Int16(v) => Some(*v as f64),
            Self::UInt16(v) => Some(*v as f64),
            Self::Int32(v) => Some(*v as f64),
            Self::UInt32(v) => Some(*v as f64),
            Self::Int64(v) => Some(*v as f64),
            Self::UInt64(v) => Some(*v as f64),
            Self::Float32(v) => Some(*v as f64),
            Self::Float64(v) => Some(*v),
            Self::Enumeration { ordinal, .. } => Some(*ordinal as f64),
            Self::String(s) => s.parse::<f64>().ok(),
            Self::ByteArray(_) => None,
        }
    }

    /// Extracts as a boolean flag.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            Self::Int16(v) => Some(*v != 0),
            Self::UInt16(v) => Some(*v != 0),
            Self::Int32(v) => Some(*v != 0),
            Self::UInt32(v) => Some(*v != 0),
            Self::Int64(v) => Some(*v != 0),
            Self::UInt64(v) => Some(*v != 0),
            Self::Float32(v) => Some(v.abs() > 1e-6),
            Self::Float64(v) => Some(v.abs() > 1e-6),
            Self::Enumeration { ordinal, .. } => Some(*ordinal != 0),
            Self::String(s) => {
                let lower = s.to_lowercase();
                if lower == "true" || lower == "1" || lower == "yes" || lower == "on" {
                    Some(true)
                } else if lower == "false" || lower == "0" || lower == "no" || lower == "off" {
                    Some(false)
                } else {
                    None
                }
            }
            Self::ByteArray(_) => None,
        }
    }

    /// Extracts as an integer `i64`.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Boolean(b) => Some(if *b { 1 } else { 0 }),
            Self::Int16(v) => Some(*v as i64),
            Self::UInt16(v) => Some(*v as i64),
            Self::Int32(v) => Some(*v as i64),
            Self::UInt32(v) => Some(*v as i64),
            Self::Int64(v) => Some(*v),
            Self::UInt64(v) => Some(*v as i64),
            Self::Float32(v) => Some(*v as i64),
            Self::Float64(v) => Some(*v as i64),
            Self::Enumeration { ordinal, .. } => Some(*ordinal as i64),
            Self::String(s) => s.parse::<i64>().ok(),
            Self::ByteArray(_) => None,
        }
    }

    /// Extracts as a display string.
    pub fn as_string(&self) -> String {
        match self {
            Self::Boolean(b) => b.to_string(),
            Self::Int16(v) => v.to_string(),
            Self::UInt16(v) => v.to_string(),
            Self::Int32(v) => v.to_string(),
            Self::UInt32(v) => v.to_string(),
            Self::Int64(v) => v.to_string(),
            Self::UInt64(v) => v.to_string(),
            Self::Float32(v) => format!("{:.4}", v),
            Self::Float64(v) => format!("{:.4}", v),
            Self::String(s) => s.clone(),
            Self::Enumeration { name, ordinal } => format!("{}({})", name, ordinal),
            Self::ByteArray(b) => format!("<{} bytes>", b.len()),
        }
    }

    /// Returns corresponding data type enumeration.
    pub fn datatype(&self) -> IndustrialDataType {
        match self {
            Self::Boolean(_) => IndustrialDataType::Boolean,
            Self::Int16(_) => IndustrialDataType::Int16,
            Self::UInt16(_) => IndustrialDataType::UInt16,
            Self::Int32(_) => IndustrialDataType::Int32,
            Self::UInt32(_) => IndustrialDataType::UInt32,
            Self::Int64(_) => IndustrialDataType::Int64,
            Self::UInt64(_) => IndustrialDataType::UInt64,
            Self::Float32(_) => IndustrialDataType::Float32,
            Self::Float64(_) => IndustrialDataType::Float64,
            Self::String(_) => IndustrialDataType::String,
            Self::Enumeration { .. } => IndustrialDataType::Enumeration,
            Self::ByteArray(_) => IndustrialDataType::ByteArray,
        }
    }
}

/// Data type classification for schema and gateway configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IndustrialDataType {
    Boolean,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Float32,
    Float64,
    String,
    Enumeration,
    ByteArray,
}

/// Canonical industrial data quality model.
///
/// Preserves semantic fidelity across protocol adapters (OPC UA StatusCodes, Modbus exceptions,
/// MQTT communication dropouts) and guarantees that errors are never mapped to Good.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IndustrialQuality {
    /// Valid, reliable process data within normal sensor operating range.
    Good,
    /// Value may be accurate but confidence is impaired (e.g. initial value, low accuracy mode).
    Uncertain,
    /// Value cannot be trusted due to fatal failure.
    Bad,
    /// Value has not been refreshed within the configured stale timeout window.
    Stale,
    /// Network connection lost, packet timed out, or TCP socket broken.
    CommunicationFailure,
    /// Value exceeds transmitter physical or electrical measurement limits (e.g. 3.6 mA or 21.0 mA).
    OutOfRange,
    /// Misconfigured address, data type mismatch, or invalid scaling denominator.
    ConfigurationError,
    /// Hardware sensor open-circuit, short-circuit, or internal self-test failure.
    SensorFailure,
    /// Operator manual override or engineering bypass active.
    Substituted,
    /// Generated by simulation engine or test loopback generator.
    Simulated,
}

impl IndustrialQuality {
    /// Returns `true` if the signal is valid and safe for automatic control loops.
    #[inline]
    pub fn is_good(&self) -> bool {
        matches!(self, Self::Good | Self::Simulated)
    }

    /// Returns `true` if the quality represents an active fault or invalid data.
    #[inline]
    pub fn is_bad(&self) -> bool {
        !self.is_good() && !matches!(self, Self::Uncertain | Self::Substituted)
    }
}

/// Structured timestamp differentiating physical source origin, protocol server transit,
/// and local gateway ingestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IndustrialTimestamp {
    /// Timestamp assigned at source transmitter or field RTU (UTC epoch milliseconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_timestamp_ms: Option<u64>,
    /// Timestamp assigned by intermediate OPC UA server or broker (UTC epoch milliseconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_timestamp_ms: Option<u64>,
    /// Timestamp when NebuDCS protocol adapter ingested and validated the message.
    pub ingestion_timestamp_ms: u64,
}

impl IndustrialTimestamp {
    /// Creates a timestamp recorded at local arrival.
    pub fn now(source_ts: Option<u64>, server_ts: Option<u64>) -> Self {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            source_timestamp_ms: source_ts,
            server_timestamp_ms: server_ts,
            ingestion_timestamp_ms: now_ms,
        }
    }

    /// Checks whether the sample has exceeded the configured stale timeout window.
    pub fn is_stale(&self, timeout_ms: u64, current_wall_clock_ms: u64) -> bool {
        if timeout_ms == 0 {
            return false;
        }
        let ref_ts = self
            .source_timestamp_ms
            .unwrap_or(self.ingestion_timestamp_ms);
        current_wall_clock_ms.saturating_sub(ref_ts) > timeout_ms
    }

    /// Detects out-of-order sample reception based on previous source timestamp.
    pub fn is_out_of_order(&self, previous_source_ts: Option<u64>) -> bool {
        match (self.source_timestamp_ms, previous_source_ts) {
            (Some(curr), Some(prev)) => curr < prev,
            _ => false,
        }
    }
}

/// Access permission policy for mapped tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccessLevel {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

/// Originating source subsystem classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DataSourceKind {
    Simulated,
    Internal,
    OpcUa,
    Modbus,
    Mqtt,
    Sparkplug,
}

/// Provenance metadata tracking origin connection, physical device address, and conduit.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TagProvenance {
    pub connection_id: String,
    pub remote_address: String,
    pub protocol: String,
    pub zone_id: String,
}

/// Fully qualified canonical industrial tag value record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndustrialTagValue {
    /// Canonical tag ID (e.g. "PT-101.PV", "FV-102.CMD").
    pub canonical_id: String,
    /// Physical or protocol origin.
    pub source: DataSourceKind,
    /// Strongly typed value payload.
    pub value: IndustrialDataValue,
    /// Associated data type classification.
    pub datatype: IndustrialDataType,
    /// Engineering units (e.g. "bar", "degC", "kg/h", "%").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engineering_unit: Option<String>,
    /// Signal quality status.
    pub quality: IndustrialQuality,
    /// Detailed timestamp tracking.
    pub timestamp: IndustrialTimestamp,
    /// Expected or negotiated update rate in milliseconds.
    pub update_rate_ms: u32,
    /// Read/Write access level.
    pub access: AccessLevel,
    /// Tracking provenance back to external protocol endpoint.
    pub provenance: TagProvenance,
    /// Associated ISA-18.2 alarm state if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alarm_state: Option<String>,
    /// Diagnostic or error text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic_state: Option<String>,
}
