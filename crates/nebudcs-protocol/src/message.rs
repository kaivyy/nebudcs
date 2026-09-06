//! WebSocket protocol message envelopes for client-server communication.

use serde::{Deserialize, Serialize};

use crate::command::RuntimeCommand;
use crate::dto::{
    AlarmSnapshotDto, DeltaUpdateDto, HistoricalEventDto, ScenarioStatusDto, StateSnapshotDto,
};
use crate::error::ProtocolError;

/// Client-announced features and buffer constraints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientCapabilities {
    pub supports_binary: bool,
    pub max_batch_size: usize,
}

impl Default for ClientCapabilities {
    fn default() -> Self {
        Self {
            supports_binary: false,
            max_batch_size: 1000,
        }
    }
}

/// Inbound messages sent from WebSocket clients to the NebuDCS server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Initial session handshake announcing protocol version and capabilities.
    Handshake {
        protocol_version: String,
        client_id: String,
        capabilities: ClientCapabilities,
    },

    /// Subscribes to real-time telemetry for specified tag names.
    Subscribe {
        #[serde(default)]
        tags: Vec<String>,
        #[serde(default)]
        all_tags: bool,
        #[serde(default)]
        include_alarms: bool,
    },

    /// Unsubscribes from specific tags.
    Unsubscribe { tags: Vec<String> },

    /// Requests a coherent full state snapshot.
    RequestSnapshot,

    /// Dispatches a typed domain command to the simulation engine.
    Command {
        command_id: String,
        command: RuntimeCommand,
    },

    /// Heartbeat ping from client.
    Ping { nonce: u64 },
}

/// Outbound messages sent from the NebuDCS server to WebSocket clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Handshake acknowledgment confirming session established.
    HandshakeAck {
        protocol_version: String,
        server_version: String,
        session_id: String,
        model_version: String,
    },

    /// Full state snapshot.
    Snapshot(StateSnapshotDto),

    /// Incremental high-frequency delta update.
    Delta(DeltaUpdateDto),

    /// Alarm state update broadcast.
    AlarmUpdate(Vec<AlarmSnapshotDto>),

    /// Discrete operational or safety event.
    Event(HistoricalEventDto),

    /// Command acceptance or execution acknowledgment.
    CommandAck {
        command_id: String,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error_code: Option<String>,
    },

    /// Heartbeat pong response.
    Pong { nonce: u64 },

    /// Protocol or validation error envelope.
    Error(ProtocolError),

    /// Scenario status update broadcast.
    ScenarioUpdate(ScenarioStatusDto),
}
