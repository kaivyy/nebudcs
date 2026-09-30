//! Protocol adapter trait, connection lifecycle states, and diagnostic tracking.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use super::types::{IndustrialDataValue, IndustrialTagValue};
use crate::error::ProtocolError;

/// Supported industrial protocol types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProtocolType {
    ModbusTcp,
    ModbusSecurity,
    OpcUa,
    Mqtt,
    SparkplugB,
}

impl ProtocolType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ModbusTcp => "Modbus TCP",
            Self::ModbusSecurity => "Modbus Security (TLS)",
            Self::OpcUa => "OPC UA (IEC 62541)",
            Self::Mqtt => "MQTT",
            Self::SparkplugB => "Sparkplug B",
        }
    }
}

/// Dynamic connection lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Degraded,
    Reconnecting,
    Faulted,
    Stopping,
}

/// Real-time health metrics for an active protocol connection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionHealth {
    pub state: ConnectionState,
    pub is_healthy: bool,
    pub consecutive_errors: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub last_heartbeat_ms: u64,
}

/// High-resolution diagnostic counters and performance telemetry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolDiagnostics {
    pub connection_id: String,
    pub protocol: ProtocolType,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub read_operations: u64,
    pub write_operations: u64,
    pub error_count: u64,
    pub reconnect_count: u32,
    pub last_latency_ms: f64,
    pub avg_latency_ms: f64,
    pub max_latency_ms: f64,
    pub active_subscriptions: usize,
    pub stale_tags_count: usize,
    pub bad_tags_count: usize,
    pub queue_depth: usize,
    pub last_successful_comm_ms: u64,
}

/// Thread-safe atomic diagnostic counters used by protocol adapters during high-frequency execution.
#[derive(Debug, Default)]
pub struct AtomicDiagnostics {
    pub messages_sent: AtomicU64,
    pub messages_received: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub read_operations: AtomicU64,
    pub write_operations: AtomicU64,
    pub error_count: AtomicU64,
    pub reconnect_count: AtomicU32,
    pub last_successful_comm_ms: AtomicU64,
}

impl AtomicDiagnostics {
    pub fn snapshot(&self, connection_id: &str, protocol: ProtocolType) -> ProtocolDiagnostics {
        ProtocolDiagnostics {
            connection_id: connection_id.to_string(),
            protocol,
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            read_operations: self.read_operations.load(Ordering::Relaxed),
            write_operations: self.write_operations.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
            reconnect_count: self.reconnect_count.load(Ordering::Relaxed),
            last_latency_ms: 0.0,
            avg_latency_ms: 0.0,
            max_latency_ms: 0.0,
            active_subscriptions: 0,
            stale_tags_count: 0,
            bad_tags_count: 0,
            queue_depth: 0,
            last_successful_comm_ms: self.last_successful_comm_ms.load(Ordering::Relaxed),
        }
    }
}

/// Security credentials and TLS profile for a connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConnectionSecurityConfig {
    /// Enable TLS transport encryption.
    pub use_tls: bool,
    /// Client certificate identity / alias.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_certificate_id: Option<String>,
    /// Trusted CA certificate identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_certificate_id: Option<String>,
    /// Authentication username if required.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// Masked or referenced password credential ID (never plaintext).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_ref: Option<String>,
    /// Enforce server hostname certificate verification.
    pub verify_hostname: bool,
}

/// Declarative configuration defining an external industrial protocol connection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub protocol: ProtocolType,
    /// Target network address (e.g. "192.168.1.100:502" or "opc.tcp://10.0.1.50:4840").
    pub endpoint: String,
    /// Active state enable flag.
    pub enabled: bool,
    /// Connection and read/write timeout in milliseconds.
    pub timeout_ms: u64,
    /// Base retry delay on connection loss in milliseconds.
    pub reconnect_delay_ms: u64,
    /// Maximum reconnection attempts before entering Faulted state (0 = infinite).
    pub max_reconnect_attempts: u32,
    /// Keepalive heartbeat interval in milliseconds.
    pub keepalive_interval_ms: u64,
    /// Security profile.
    pub security: ConnectionSecurityConfig,
    /// Associated ISA-95 zone identifier.
    pub zone_id: String,
}

impl ConnectionConfig {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        protocol: ProtocolType,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            protocol,
            endpoint: endpoint.into(),
            enabled: true,
            timeout_ms: 3000,
            reconnect_delay_ms: 2000,
            max_reconnect_attempts: 10,
            keepalive_interval_ms: 5000,
            security: ConnectionSecurityConfig::default(),
            zone_id: "ControlZone".to_string(),
        }
    }
}

/// Common asynchronous adapter interface for industrial protocol drivers.
///
/// Protocol adapters run in dedicated background tasks and communicate with the simulation
/// host through non-blocking bounded channels, strictly decoupled from the deterministic
/// simulation tick hot path.
pub trait ProtocolAdapter: Send + Sync {
    /// Connects to remote endpoint and completes handshake.
    fn connect(&mut self) -> Result<(), ProtocolError>;

    /// Gracefully closes connection.
    fn disconnect(&mut self) -> Result<(), ProtocolError>;

    /// Reads a value synchronously from the remote device.
    fn read(&mut self, address: &str) -> Result<IndustrialTagValue, ProtocolError>;

    /// Writes a value to the remote device.
    fn write(&mut self, address: &str, value: &IndustrialDataValue) -> Result<(), ProtocolError>;

    /// Current connection health.
    fn health(&self) -> ConnectionHealth;

    /// Current diagnostic counters.
    fn diagnostics(&self) -> ProtocolDiagnostics;

    /// Terminal shutdown.
    fn shutdown(&mut self) -> Result<(), ProtocolError>;
}
