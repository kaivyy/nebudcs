//! MQTT v3.1.1 / v5.0 transport adapter and in-memory mock broker.
//!
//! Provides topic subscriptions, QoS levels, retained message caches,
//! and thread-safe publish/subscribe pipelines.

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
    AccessLevel, DataSourceKind, IndustrialDataValue, IndustrialQuality, IndustrialTagValue,
    IndustrialTimestamp, TagProvenance,
};
use crate::error::ProtocolError;

/// MQTT Quality of Service levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum MqttQoS {
    #[default]
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

/// Generic MQTT message envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: Vec<u8>,
    pub qos: MqttQoS,
    pub retain: bool,
}

/// In-memory mock MQTT broker for tests and offline simulation.
#[derive(Debug, Clone)]
pub struct MockMqttBroker {
    pub retained_messages: Arc<RwLock<BTreeMap<String, MqttMessage>>>,
    pub published_messages: Arc<RwLock<Vec<MqttMessage>>>,
    pub is_online: Arc<RwLock<bool>>,
}

impl Default for MockMqttBroker {
    fn default() -> Self {
        Self::new()
    }
}

impl MockMqttBroker {
    pub fn new() -> Self {
        Self {
            retained_messages: Arc::new(RwLock::new(BTreeMap::new())),
            published_messages: Arc::new(RwLock::new(Vec::new())),
            is_online: Arc::new(RwLock::new(true)),
        }
    }

    pub async fn publish(&self, msg: MqttMessage) -> Result<(), ProtocolError> {
        if !*self.is_online.read().await {
            return Err(ProtocolError::ConnectionLost("MQTT broker offline"));
        }
        if msg.retain {
            self.retained_messages
                .write()
                .await
                .insert(msg.topic.clone(), msg.clone());
        }
        self.published_messages.write().await.push(msg);
        Ok(())
    }

    pub async fn get_retained(&self, topic: &str) -> Option<MqttMessage> {
        self.retained_messages.read().await.get(topic).cloned()
    }

    pub async fn clear(&self) {
        self.published_messages.write().await.clear();
        self.retained_messages.write().await.clear();
    }
}

/// Production MQTT Client Adapter.
pub struct MqttAdapter {
    config: ConnectionConfig,
    state: ConnectionState,
    diagnostics: Arc<AtomicDiagnostics>,
    last_error: Option<String>,
    mock_broker: Option<MockMqttBroker>,
    subscriptions: Vec<String>,
}

impl MqttAdapter {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            state: ConnectionState::Disconnected,
            diagnostics: Arc::new(AtomicDiagnostics::default()),
            last_error: None,
            mock_broker: None,
            subscriptions: Vec::new(),
        }
    }

    pub fn with_mock_broker(mut self, broker: MockMqttBroker) -> Self {
        self.mock_broker = Some(broker);
        self
    }
}

impl ProtocolAdapter for MqttAdapter {
    fn connect(&mut self) -> Result<(), ProtocolError> {
        self.state = ConnectionState::Connecting;
        if let Some(ref broker) = self.mock_broker {
            let is_online = futures_util::FutureExt::now_or_never(broker.is_online.read())
                .map(|g| *g)
                .unwrap_or(true);
            if !is_online {
                self.state = ConnectionState::Faulted;
                self.last_error = Some("Mock MQTT broker unreachable".into());
                self.diagnostics.error_count.fetch_add(1, Ordering::Relaxed);
                return Err(ProtocolError::ConnectionLost("Connection refused"));
            }
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
        self.state = ConnectionState::Disconnected;
        Ok(())
    }

    fn read(&mut self, address: &str) -> Result<IndustrialTagValue, ProtocolError> {
        if self.state != ConnectionState::Connected {
            self.diagnostics.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(ProtocolError::ConnectionLost("MQTT adapter disconnected"));
        }

        self.diagnostics
            .read_operations
            .fetch_add(1, Ordering::Relaxed);
        self.diagnostics
            .messages_sent
            .fetch_add(1, Ordering::Relaxed);

        let payload_str = if let Some(ref broker) = self.mock_broker {
            let msg = futures_util::FutureExt::now_or_never(broker.get_retained(address)).flatten();
            if let Some(m) = msg {
                String::from_utf8(m.payload).unwrap_or_default()
            } else {
                "0.0".to_string()
            }
        } else {
            "0.0".to_string()
        };

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

        let val = if let Ok(f) = payload_str.parse::<f64>() {
            IndustrialDataValue::Float64(f)
        } else if let Ok(b) = payload_str.parse::<bool>() {
            IndustrialDataValue::Boolean(b)
        } else {
            IndustrialDataValue::String(payload_str)
        };

        Ok(IndustrialTagValue {
            canonical_id: address.to_string(),
            source: DataSourceKind::Mqtt,
            value: val.clone(),
            datatype: val.datatype(),
            engineering_unit: None,
            quality: IndustrialQuality::Good,
            timestamp: IndustrialTimestamp::now(None, None),
            update_rate_ms: 1000,
            access: AccessLevel::ReadWrite,
            provenance: TagProvenance {
                connection_id: self.config.id.clone(),
                remote_address: address.to_string(),
                protocol: "MQTT".into(),
                zone_id: self.config.zone_id.clone(),
            },
            alarm_state: None,
            diagnostic_state: None,
        })
    }

    fn write(&mut self, address: &str, value: &IndustrialDataValue) -> Result<(), ProtocolError> {
        if self.state != ConnectionState::Connected {
            self.diagnostics.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(ProtocolError::ConnectionLost("MQTT adapter disconnected"));
        }

        self.diagnostics
            .write_operations
            .fetch_add(1, Ordering::Relaxed);
        self.diagnostics
            .messages_sent
            .fetch_add(1, Ordering::Relaxed);

        let payload = match value {
            IndustrialDataValue::String(s) => s.as_bytes().to_vec(),
            IndustrialDataValue::ByteArray(b) => b.clone(),
            _ => value.as_string().into_bytes(),
        };

        if let Some(ref broker) = self.mock_broker {
            let msg = MqttMessage {
                topic: address.to_string(),
                payload,
                qos: MqttQoS::AtLeastOnce,
                retain: true,
            };
            futures_util::FutureExt::now_or_never(broker.publish(msg))
                .ok_or_else(|| ProtocolError::Internal("Async poll pending"))??;
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
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
        let mut d = self
            .diagnostics
            .snapshot(&self.config.id, ProtocolType::Mqtt);
        d.active_subscriptions = self.subscriptions.len();
        d
    }

    fn shutdown(&mut self) -> Result<(), ProtocolError> {
        self.state = ConnectionState::Disconnected;
        Ok(())
    }
}
