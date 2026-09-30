//! Sparkplug B industrial protocol engine.
//!
//! Implements Eclipse Sparkplug B specification (spBv1.0), topic structures,
//! metric definitions, and lifecycle birth/death state management.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::adapter::{
    AtomicDiagnostics, ConnectionConfig, ConnectionHealth, ConnectionState, ProtocolAdapter,
    ProtocolDiagnostics, ProtocolType,
};
pub use super::mqtt::{MockMqttBroker, MqttMessage, MqttQoS};
use super::types::{
    AccessLevel, DataSourceKind, IndustrialDataType, IndustrialDataValue, IndustrialQuality,
    IndustrialTagValue, IndustrialTimestamp, TagProvenance,
};
use crate::error::ProtocolError;

/// Sparkplug B message types conforming to the standard namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SparkplugMessageType {
    /// Node Birth (advises broker of edge node metrics and status).
    NBIRTH,
    /// Node Death (LWT payload indicating abrupt node failure).
    NDEATH,
    /// Device Birth (advises broker of connected device metrics).
    DBIRTH,
    /// Device Death.
    DDEATH,
    /// Node Data (telemetry update from edge node).
    NDATA,
    /// Device Data (telemetry update from physical device).
    DDATA,
    /// Node Command.
    NCMD,
    /// Device Command.
    DCMD,
}

impl SparkplugMessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NBIRTH => "NBIRTH",
            Self::NDEATH => "NDEATH",
            Self::DBIRTH => "DBIRTH",
            Self::DDEATH => "DDEATH",
            Self::NDATA => "NDATA",
            Self::DDATA => "DDATA",
            Self::NCMD => "NCMD",
            Self::DCMD => "DCMD",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "NBIRTH" => Some(Self::NBIRTH),
            "NDEATH" => Some(Self::NDEATH),
            "DBIRTH" => Some(Self::DBIRTH),
            "DDEATH" => Some(Self::DDEATH),
            "NDATA" => Some(Self::NDATA),
            "DDATA" => Some(Self::DDATA),
            "NCMD" => Some(Self::NCMD),
            "DCMD" => Some(Self::DCMD),
            _ => None,
        }
    }
}

/// Canonical Sparkplug metric representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SparkplugMetric {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<u64>,
    pub timestamp: u64,
    pub datatype: IndustrialDataType,
    pub value: IndustrialDataValue,
    pub is_historical: bool,
    pub is_transient: bool,
}

/// Sparkplug B payload model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SparkplugPayload {
    pub timestamp: u64,
    pub metrics: Vec<SparkplugMetric>,
    pub seq: u64,
}

/// Fully qualified Sparkplug B topic parser and builder.
/// Format: spBv1.0/<group_id>/<message_type>/<edge_node_id>/[<device_id>]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SparkplugTopic {
    pub group_id: String,
    pub message_type: SparkplugMessageType,
    pub edge_node_id: String,
    pub device_id: Option<String>,
}

impl SparkplugTopic {
    pub fn build(&self) -> String {
        if let Some(ref dev) = self.device_id {
            format!(
                "spBv1.0/{}/{}/{}/{}",
                self.group_id,
                self.message_type.as_str(),
                self.edge_node_id,
                dev
            )
        } else {
            format!(
                "spBv1.0/{}/{}/{}",
                self.group_id,
                self.message_type.as_str(),
                self.edge_node_id
            )
        }
    }

    pub fn parse(topic: &str) -> Result<Self, ProtocolError> {
        let parts: Vec<&str> = topic.split('/').collect();
        if parts.len() < 4 || parts[0] != "spBv1.0" {
            return Err(ProtocolError::InvalidPayload(format!(
                "Invalid Sparkplug B topic format: {}",
                topic
            )));
        }
        let group_id = parts[1].to_string();
        let msg_type = SparkplugMessageType::parse(parts[2]).ok_or_else(|| {
            ProtocolError::InvalidPayload(format!("Unknown Sparkplug message type: {}", parts[2]))
        })?;
        let edge_node_id = parts[3].to_string();
        let device_id = if parts.len() >= 5 {
            Some(parts[4].to_string())
        } else {
            None
        };

        Ok(Self {
            group_id,
            message_type: msg_type,
            edge_node_id,
            device_id,
        })
    }
}

/// Edge Node managing birth/death lifecycle and sequencing for industrial telemetry.
pub struct SparkplugEdgeNode {
    pub group_id: String,
    pub node_id: String,
    pub seq: AtomicU8,
    pub metrics: Arc<RwLock<BTreeMap<String, SparkplugMetric>>>,
    pub is_born: Arc<RwLock<bool>>,
}

impl SparkplugEdgeNode {
    pub fn new(group_id: impl Into<String>, node_id: impl Into<String>) -> Self {
        Self {
            group_id: group_id.into(),
            node_id: node_id.into(),
            seq: AtomicU8::new(0),
            metrics: Arc::new(RwLock::new(BTreeMap::new())),
            is_born: Arc::new(RwLock::new(false)),
        }
    }

    pub fn next_seq(&self) -> u8 {
        let curr = self.seq.load(Ordering::Relaxed);
        let next = if curr == 255 { 0 } else { curr + 1 };
        self.seq.store(next, Ordering::Relaxed);
        curr
    }

    /// Generates standard NBIRTH payload declaring node availability.
    pub async fn generate_birth_payload(&self) -> SparkplugPayload {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let lock = self.metrics.read().await;
        let metrics: Vec<SparkplugMetric> = lock.values().cloned().collect();

        self.seq.store(0, Ordering::Relaxed);
        *self.is_born.write().await = true;

        SparkplugPayload {
            timestamp: now_ms,
            metrics,
            seq: 0,
        }
    }

    /// Generates NDEATH payload indicating node termination.
    pub fn generate_death_payload(&self) -> SparkplugPayload {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        SparkplugPayload {
            timestamp: now_ms,
            metrics: Vec::new(),
            seq: 0,
        }
    }
}

/// Production Sparkplug B Client Adapter.
pub struct SparkplugAdapter {
    config: ConnectionConfig,
    state: ConnectionState,
    diagnostics: Arc<AtomicDiagnostics>,
    last_error: Option<String>,
    mock_broker: Option<MockMqttBroker>,
    edge_node: Arc<SparkplugEdgeNode>,
}

impl SparkplugAdapter {
    pub fn new(config: ConnectionConfig, group_id: &str, node_id: &str) -> Self {
        Self {
            config,
            state: ConnectionState::Disconnected,
            diagnostics: Arc::new(AtomicDiagnostics::default()),
            last_error: None,
            mock_broker: None,
            edge_node: Arc::new(SparkplugEdgeNode::new(group_id, node_id)),
        }
    }

    pub fn with_mock_broker(mut self, broker: MockMqttBroker) -> Self {
        self.mock_broker = Some(broker);
        self
    }
}

impl ProtocolAdapter for SparkplugAdapter {
    fn connect(&mut self) -> Result<(), ProtocolError> {
        self.state = ConnectionState::Connecting;
        if let Some(ref broker) = self.mock_broker {
            let is_online = futures_util::FutureExt::now_or_never(broker.is_online.read())
                .map(|g| *g)
                .unwrap_or(true);
            if !is_online {
                self.state = ConnectionState::Faulted;
                self.last_error = Some("Mock broker unreachable".into());
                self.diagnostics.error_count.fetch_add(1, Ordering::Relaxed);
                return Err(ProtocolError::ConnectionLost("Connection refused"));
            }

            // Publish NBIRTH
            let birth_topic = SparkplugTopic {
                group_id: self.edge_node.group_id.clone(),
                message_type: SparkplugMessageType::NBIRTH,
                edge_node_id: self.edge_node.node_id.clone(),
                device_id: None,
            };

            let payload =
                futures_util::FutureExt::now_or_never(self.edge_node.generate_birth_payload())
                    .ok_or_else(|| ProtocolError::Internal("Async birth generation pending"))?;
            let serialized = serde_json::to_vec(&payload)
                .map_err(|e| ProtocolError::InvalidPayload(e.to_string()))?;

            let msg = MqttMessage {
                topic: birth_topic.build(),
                payload: serialized,
                qos: MqttQoS::AtLeastOnce,
                retain: false,
            };
            let _ = futures_util::FutureExt::now_or_never(broker.publish(msg));
        }

        self.state = ConnectionState::Connected;
        self.diagnostics
            .reconnect_count
            .fetch_add(1, Ordering::Relaxed);
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.diagnostics
            .last_successful_comm_ms
            .store(now_ms, Ordering::Relaxed);
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), ProtocolError> {
        if let Some(ref broker) = self.mock_broker {
            // Publish NDEATH
            let death_topic = SparkplugTopic {
                group_id: self.edge_node.group_id.clone(),
                message_type: SparkplugMessageType::NDEATH,
                edge_node_id: self.edge_node.node_id.clone(),
                device_id: None,
            };
            let payload = self.edge_node.generate_death_payload();
            let serialized = serde_json::to_vec(&payload).unwrap_or_default();
            let msg = MqttMessage {
                topic: death_topic.build(),
                payload: serialized,
                qos: MqttQoS::AtLeastOnce,
                retain: false,
            };
            let _ = futures_util::FutureExt::now_or_never(broker.publish(msg));
        }
        self.state = ConnectionState::Disconnected;
        Ok(())
    }

    fn read(&mut self, address: &str) -> Result<IndustrialTagValue, ProtocolError> {
        if self.state != ConnectionState::Connected {
            self.diagnostics.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(ProtocolError::ConnectionLost(
                "Sparkplug adapter disconnected",
            ));
        }

        self.diagnostics
            .read_operations
            .fetch_add(1, Ordering::Relaxed);
        self.diagnostics
            .messages_sent
            .fetch_add(1, Ordering::Relaxed);

        let lock = futures_util::FutureExt::now_or_never(self.edge_node.metrics.read())
            .ok_or_else(|| ProtocolError::Internal("Async poll pending"))?;

        let metric = lock
            .get(address)
            .cloned()
            .unwrap_or_else(|| SparkplugMetric {
                name: address.to_string(),
                alias: None,
                timestamp: 0,
                datatype: IndustrialDataType::Float64,
                value: IndustrialDataValue::Float64(0.0),
                is_historical: false,
                is_transient: false,
            });

        self.diagnostics
            .messages_received
            .fetch_add(1, Ordering::Relaxed);
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.diagnostics
            .last_successful_comm_ms
            .store(now_ms, Ordering::Relaxed);

        Ok(IndustrialTagValue {
            canonical_id: address.to_string(),
            source: DataSourceKind::Sparkplug,
            value: metric.value.clone(),
            datatype: metric.datatype,
            engineering_unit: None,
            quality: IndustrialQuality::Good,
            timestamp: IndustrialTimestamp {
                source_timestamp_ms: Some(metric.timestamp),
                server_timestamp_ms: Some(now_ms),
                ingestion_timestamp_ms: now_ms,
            },
            update_rate_ms: 500,
            access: AccessLevel::ReadWrite,
            provenance: TagProvenance {
                connection_id: self.config.id.clone(),
                remote_address: address.to_string(),
                protocol: "SPARKPLUG_B".into(),
                zone_id: self.config.zone_id.clone(),
            },
            alarm_state: None,
            diagnostic_state: None,
        })
    }

    fn write(&mut self, address: &str, value: &IndustrialDataValue) -> Result<(), ProtocolError> {
        if self.state != ConnectionState::Connected {
            self.diagnostics.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(ProtocolError::ConnectionLost(
                "Sparkplug adapter disconnected",
            ));
        }

        self.diagnostics
            .write_operations
            .fetch_add(1, Ordering::Relaxed);
        self.diagnostics
            .messages_sent
            .fetch_add(1, Ordering::Relaxed);

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let metric = SparkplugMetric {
            name: address.to_string(),
            alias: None,
            timestamp: now_ms,
            datatype: value.datatype(),
            value: value.clone(),
            is_historical: false,
            is_transient: false,
        };

        let mut lock = futures_util::FutureExt::now_or_never(self.edge_node.metrics.write())
            .ok_or_else(|| ProtocolError::Internal("Async poll pending"))?;
        lock.insert(address.to_string(), metric.clone());

        if let Some(ref broker) = self.mock_broker {
            let data_topic = SparkplugTopic {
                group_id: self.edge_node.group_id.clone(),
                message_type: SparkplugMessageType::NDATA,
                edge_node_id: self.edge_node.node_id.clone(),
                device_id: None,
            };
            let payload = SparkplugPayload {
                timestamp: now_ms,
                metrics: vec![metric],
                seq: self.edge_node.next_seq() as u64,
            };
            let serialized = serde_json::to_vec(&payload).unwrap_or_default();
            let msg = MqttMessage {
                topic: data_topic.build(),
                payload: serialized,
                qos: MqttQoS::AtLeastOnce,
                retain: false,
            };
            let _ = futures_util::FutureExt::now_or_never(broker.publish(msg));
        }

        self.diagnostics
            .last_successful_comm_ms
            .store(now_ms, Ordering::Relaxed);
        Ok(())
    }

    fn health(&self) -> ConnectionHealth {
        ConnectionHealth {
            state: self.state,
            is_healthy: self.state == ConnectionState::Connected,
            consecutive_errors: self.diagnostics.error_count.load(Ordering::Relaxed) as u32,
            last_error: self.last_error.clone(),
            last_heartbeat_ms: self
                .diagnostics
                .last_successful_comm_ms
                .load(Ordering::Relaxed),
        }
    }

    fn diagnostics(&self) -> ProtocolDiagnostics {
        self.diagnostics
            .snapshot(&self.config.id, ProtocolType::SparkplugB)
    }

    fn shutdown(&mut self) -> Result<(), ProtocolError> {
        self.disconnect()
    }
}
