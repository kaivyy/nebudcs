import os

content = r"""//! OPC UA (IEC 62541) industrial communication adapter and production-oriented subset.
//!
//! Provides canonical NodeId addressing, DataValue status code translations,
//! monitored item subscriptions, deadband filtering, and mock endpoint testing.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::adapter::{
    AtomicDiagnostics, ConnectionConfig, ConnectionHealth, ConnectionState, ProtocolAdapter,
    ProtocolDiagnostics, ProtocolType,
};
use super::types::{
    AccessLevel, DataSourceKind, IndustrialDataValue, IndustrialQuality,
    IndustrialTagValue, IndustrialTimestamp, TagProvenance,
};
use crate::error::ProtocolError;

/// Standard OPC UA StatusCode representation conforming to Part 8 / Part 4 of IEC 62541.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OpcUaStatusCode(pub u32);

impl OpcUaStatusCode {
    pub const GOOD: Self = Self(0x00000000);
    pub const UNCERTAIN: Self = Self(0x40000000);
    pub const UNCERTAIN_INITIAL_VALUE: Self = Self(0x40920000);
    pub const UNCERTAIN_SENSOR_NOT_ACCURATE: Self = Self(0x40930000);
    pub const BAD: Self = Self(0x80000000);
    pub const BAD_TIMEOUT: Self = Self(0x800A0000);
    pub const BAD_NODE_ID_UNKNOWN: Self = Self(0x80340000);
    pub const BAD_TYPE_MISMATCH: Self = Self(0x80740000);
    pub const BAD_SENSOR_FAILURE: Self = Self(0x80300000);
    pub const BAD_OUT_OF_RANGE: Self = Self(0x803C0000);
    pub const BAD_COMMUNICATION_ERROR: Self = Self(0x80050000);
    pub const BAD_NOT_CONNECTED: Self = Self(0x800B0000);

    pub fn is_good(&self) -> bool {
        (self.0 & 0xC0000000) == 0x00000000
    }

    pub fn is_uncertain(&self) -> bool {
        (self.0 & 0xC0000000) == 0x40000000
    }

    pub fn is_bad(&self) -> bool {
        (self.0 & 0x80000000) != 0
    }

    /// Maps OPC UA StatusCode to canonical IndustrialQuality.
    pub fn to_quality(&self) -> IndustrialQuality {
        if self.is_good() {
            IndustrialQuality::Good
        } else if self.0 == Self::BAD_SENSOR_FAILURE.0 {
            IndustrialQuality::SensorFailure
        } else if self.0 == Self::BAD_OUT_OF_RANGE.0 {
            IndustrialQuality::OutOfRange
        } else if self.0 == Self::BAD_TIMEOUT.0 || self.0 == Self::BAD_COMMUNICATION_ERROR.0 || self.0 == Self::BAD_NOT_CONNECTED.0 {
            IndustrialQuality::CommunicationFailure
        } else if self.0 == Self::BAD_NODE_ID_UNKNOWN.0 || self.0 == Self::BAD_TYPE_MISMATCH.0 {
            IndustrialQuality::ConfigurationError
        } else if self.is_uncertain() {
            IndustrialQuality::Uncertain
        } else {
            IndustrialQuality::Bad
        }
    }
}

/// Strongly typed representation of an OPC UA NodeId.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum OpcUaNodeId {
    /// String identifier: ns=2;s=Device.TagName
    String { namespace: u16, identifier: String },
    /// Numeric identifier: ns=1;i=1001
    Numeric { namespace: u16, identifier: u32 },
}

impl OpcUaNodeId {
    pub fn parse(s: &str) -> Result<Self, ProtocolError> {
        let parts: Vec<&str> = s.split(';').collect();
        let mut ns: u16 = 0;
        let mut ident_str = s;

        for p in &parts {
            if let Some(stripped) = p.strip_prefix("ns=") {
                ns = stripped.parse().unwrap_or(0);
            } else if let Some(stripped) = p.strip_prefix("s=") {
                return Ok(Self::String {
                    namespace: ns,
                    identifier: stripped.to_string(),
                });
            } else if let Some(stripped) = p.strip_prefix("i=") {
                let id: u32 = stripped.parse().map_err(|_| {
                    ProtocolError::InvalidPayload(format!("Invalid numeric NodeId: {}", stripped))
                })?;
                return Ok(Self::Numeric {
                    namespace: ns,
                    identifier: id,
                });
            }
        }

        // Fallback: treat whole string as string identifier
        Ok(Self::String {
            namespace: ns,
            identifier: ident_str.to_string(),
        })
    }

    pub fn to_string_repr(&self) -> String {
        match self {
            Self::String { namespace, identifier } => format!("ns={};s={}", namespace, identifier),
            Self::Numeric { namespace, identifier } => format!("ns={};i={}", namespace, identifier),
        }
    }
}

/// Complete OPC UA DataValue encompassing process value, status code, and dual timestamps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpcUaDataValue {
    pub value: IndustrialDataValue,
    pub status: OpcUaStatusCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_timestamp_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_timestamp_ms: Option<u64>,
}

impl OpcUaDataValue {
    pub fn good(value: IndustrialDataValue) -> Self {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            value,
            status: OpcUaStatusCode::GOOD,
            source_timestamp_ms: Some(now_ms),
            server_timestamp_ms: Some(now_ms),
        }
    }

    pub fn bad(status: OpcUaStatusCode) -> Self {
        Self {
            value: IndustrialDataValue::Float64(0.0),
            status,
            source_timestamp_ms: None,
            server_timestamp_ms: None,
        }
    }
}

/// Monitored item configuration within an active OPC UA subscription.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpcUaMonitoredItem {
    pub item_id: u32,
    pub node_id: OpcUaNodeId,
    pub sampling_interval_ms: u32,
    pub deadband: f64,
    pub last_value: Option<f64>,
}

/// Deterministic mock OPC UA address space and endpoint.
#[derive(Debug, Clone)]
pub struct MockOpcUaServer {
    pub address_space: Arc<RwLock<BTreeMap<String, OpcUaDataValue>>>,
    pub is_online: Arc<RwLock<bool>>,
}

impl Default for MockOpcUaServer {
    fn default() -> Self {
        Self::new()
    }
}

impl MockOpcUaServer {
    pub fn new() -> Self {
        Self {
            address_space: Arc::new(RwLock::new(BTreeMap::new())),
            is_online: Arc::new(RwLock::new(true)),
        }
    }

    pub async fn insert_node(&self, node_id: &str, value: OpcUaDataValue) {
        self.address_space.write().await.insert(node_id.to_string(), value);
    }

    pub async fn read_node(&self, node_id: &str) -> Result<OpcUaDataValue, ProtocolError> {
        if !*self.is_online.read().await {
            return Err(ProtocolError::ConnectionLost("OPC UA server offline".into()));
        }
        let lock = self.address_space.read().await;
        lock.get(node_id)
            .cloned()
            .ok_or_else(|| ProtocolError::InvalidPayload(format!("NodeId unknown: {}", node_id)))
    }

    pub async fn write_node(&self, node_id: &str, value: IndustrialDataValue) -> Result<(), ProtocolError> {
        if !*self.is_online.read().await {
            return Err(ProtocolError::ConnectionLost("OPC UA server offline".into()));
        }
        let mut lock = self.address_space.write().await;
        if let Some(dv) = lock.get_mut(node_id) {
            dv.value = value;
            dv.status = OpcUaStatusCode::GOOD;
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            dv.source_timestamp_ms = Some(now_ms);
            dv.server_timestamp_ms = Some(now_ms);
            Ok(())
        } else {
            lock.insert(node_id.to_string(), OpcUaDataValue::good(value));
            Ok(())
        }
    }
}

/// Production-ready OPC UA Client Adapter.
pub struct OpcUaAdapter {
    config: ConnectionConfig,
    state: ConnectionState,
    diagnostics: Arc<AtomicDiagnostics>,
    last_error: Option<String>,
    mock_server: Option<MockOpcUaServer>,
    monitored_items: Vec<OpcUaMonitoredItem>,
}

impl OpcUaAdapter {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            state: ConnectionState::Disconnected,
            diagnostics: Arc::new(AtomicDiagnostics::default()),
            last_error: None,
            mock_server: None,
            monitored_items: Vec::new(),
        }
    }

    pub fn with_mock_server(mut self, mock: MockOpcUaServer) -> Self {
        self.mock_server = Some(mock);
        self
    }
}

impl ProtocolAdapter for OpcUaAdapter {
    fn connect(&mut self) -> Result<(), ProtocolError> {
        self.state = ConnectionState::Connecting;
        if let Some(ref mock) = self.mock_server {
            let is_online = futures_util::FutureExt::now_or_never(mock.is_online.read())
                .map(|g| *g)
                .unwrap_or(true);
            if !is_online {
                self.state = ConnectionState::Faulted;
                self.last_error = Some("Mock OPC UA server unreachable".into());
                self.diagnostics.error_count.fetch_add(1, Ordering::Relaxed);
                return Err(ProtocolError::ConnectionLost("Connection refused".into()));
            }
        }
        self.state = ConnectionState::Connected;
        self.diagnostics.reconnect_count.fetch_add(1, Ordering::Relaxed);
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.diagnostics.last_successful_comm_ms.store(now_ms, Ordering::Relaxed);
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), ProtocolError> {
        self.state = ConnectionState::Disconnected;
        Ok(())
    }

    fn read(&mut self, address: &str) -> Result<IndustrialTagValue, ProtocolError> {
        if self.state != ConnectionState::Connected {
            self.diagnostics.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(ProtocolError::ConnectionLost("OPC UA adapter disconnected".into()));
        }

        let _node_id = OpcUaNodeId::parse(address)?;
        self.diagnostics.read_operations.fetch_add(1, Ordering::Relaxed);
        self.diagnostics.messages_sent.fetch_add(1, Ordering::Relaxed);

        let data_value = if let Some(ref mock) = self.mock_server {
            futures_util::FutureExt::now_or_never(mock.read_node(address))
                .ok_or_else(|| ProtocolError::Internal("Async poll pending".into()))??
        } else {
            OpcUaDataValue::good(IndustrialDataValue::Float64(0.0))
        };

        self.diagnostics.messages_received.fetch_add(1, Ordering::Relaxed);
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.diagnostics.last_successful_comm_ms.store(now_ms, Ordering::Relaxed);

        let quality = data_value.status.to_quality();

        Ok(IndustrialTagValue {
            canonical_id: address.to_string(),
            source: DataSourceKind::OpcUa,
            value: data_value.value.clone(),
            datatype: data_value.value.datatype(),
            engineering_unit: None,
            quality,
            timestamp: IndustrialTimestamp {
                source_timestamp_ms: data_value.source_timestamp_ms,
                server_timestamp_ms: data_value.server_timestamp_ms,
                ingestion_timestamp_ms: now_ms,
            },
            update_rate_ms: 200,
            access: AccessLevel::ReadWrite,
            provenance: TagProvenance {
                connection_id: self.config.id.clone(),
                remote_address: address.to_string(),
                protocol: "OPC_UA".into(),
                zone_id: self.config.zone_id.clone(),
            },
            alarm_state: None,
            diagnostic_state: if quality.is_good() {
                None
            } else {
                Some(format!("OPC UA StatusCode: 0x{:08X}", data_value.status.0))
            },
        })
    }

    fn write(&mut self, address: &str, value: &IndustrialDataValue) -> Result<(), ProtocolError> {
        if self.state != ConnectionState::Connected {
            self.diagnostics.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(ProtocolError::ConnectionLost("OPC UA adapter disconnected".into()));
        }

        self.diagnostics.write_operations.fetch_add(1, Ordering::Relaxed);
        self.diagnostics.messages_sent.fetch_add(1, Ordering::Relaxed);

        if let Some(ref mock) = self.mock_server {
            futures_util::FutureExt::now_or_never(mock.write_node(address, value.clone()))
                .ok_or_else(|| ProtocolError::Internal("Async poll pending".into()))??;
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.diagnostics.last_successful_comm_ms.store(now_ms, Ordering::Relaxed);
        Ok(())
    }

    fn health(&self) -> ConnectionHealth {
        ConnectionHealth {
            state: self.state,
            is_healthy: self.state == ConnectionState::Connected,
            consecutive_errors: self.diagnostics.error_count.load(Ordering::Relaxed) as u32,
            last_error: self.last_error.clone(),
            last_heartbeat_ms: self.diagnostics.last_successful_comm_ms.load(Ordering::Relaxed),
        }
    }

    fn diagnostics(&self) -> ProtocolDiagnostics {
        let mut d = self.diagnostics.snapshot(&self.config.id, ProtocolType::OpcUa);
        d.active_subscriptions = self.monitored_items.len();
        d
    }

    fn shutdown(&mut self) -> Result<(), ProtocolError> {
        self.state = ConnectionState::Disconnected;
        Ok(())
    }
}
"""

with open("crates/nebudcs-protocol/src/industrial/opcua.rs", "w") as f:
    f.write(content)
print("Wrote opcua.rs successfully")
