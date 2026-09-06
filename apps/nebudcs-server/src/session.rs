//! WebSocket client session management, subscription tracking, and message routing.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, mpsc, RwLock};
use ulid::Ulid;

use nebudcs_protocol::*;

use crate::host::HostHandle;

struct ClientGuard(HostHandle);

impl Drop for ClientGuard {
    fn drop(&mut self) {
        self.0.dec_clients();
    }
}

/// Handles the full lifecycle of a connected WebSocket client session.
pub async fn handle_ws_session(socket: WebSocket, host: HostHandle) {
    let _guard = ClientGuard(host.clone());
    host.inc_clients();

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 1. Handshake Phase with 5s timeout
    let handshake_timeout = Duration::from_secs(5);
    let first_msg = match tokio::time::timeout(handshake_timeout, ws_receiver.next()).await {
        Ok(Some(Ok(msg))) => msg,
        _ => return,
    };

    let handshake_text = match first_msg {
        Message::Text(text) => text,
        _ => {
            let err = ServerMessage::Error(ProtocolError::new(
                ProtocolErrorCode::InvalidMessage,
                "First message must be a JSON Handshake",
            ));
            let _ = ws_sender
                .send(Message::Text(serde_json::to_string(&err).unwrap().into()))
                .await;
            return;
        }
    };

    let client_msg: ClientMessage = match serde_json::from_str(handshake_text.as_str()) {
        Ok(msg) => msg,
        Err(e) => {
            let err = ServerMessage::Error(ProtocolError::new(
                ProtocolErrorCode::InvalidMessage,
                format!("Malformed handshake: {e}"),
            ));
            let _ = ws_sender
                .send(Message::Text(serde_json::to_string(&err).unwrap().into()))
                .await;
            return;
        }
    };

    let client_id = match client_msg {
        ClientMessage::Handshake {
            client_id,
            protocol_version,
            capabilities: _,
        } => {
            // Version negotiation
            if !is_compatible_protocol_version(&protocol_version) {
                let err = ServerMessage::Error(ProtocolError::new(
                    ProtocolErrorCode::ProtocolVersionMismatch,
                    format!(
                        "Server protocol version is {}, client provided incompatible version {}",
                        PROTOCOL_VERSION, protocol_version
                    ),
                ));
                let _ = ws_sender
                    .send(Message::Text(serde_json::to_string(&err).unwrap().into()))
                    .await;
                return;
            }

            client_id
        }
        _ => {
            let err = ServerMessage::Error(ProtocolError::new(
                ProtocolErrorCode::InvalidMessage,
                "First message must be a Handshake",
            ));
            let _ = ws_sender
                .send(Message::Text(serde_json::to_string(&err).unwrap().into()))
                .await;
            return;
        }
    };

    let session_id = Ulid::new().to_string();
    let subscriptions = Arc::new(RwLock::new(HashSet::<String>::new()));

    // Send HandshakeAck
    let ack = ServerMessage::HandshakeAck {
        protocol_version: PROTOCOL_VERSION.to_string(),
        server_version: env!("CARGO_PKG_VERSION").to_string(),
        session_id: session_id.clone(),
        model_version: "1.0.0".to_string(),
    };
    if ws_sender
        .send(Message::Text(serde_json::to_string(&ack).unwrap().into()))
        .await
        .is_err()
    {
        return;
    }

    // Send initial snapshot
    let snapshot = host.get_snapshot().await;
    let snap_msg = ServerMessage::Snapshot(snapshot);
    if ws_sender
        .send(Message::Text(
            serde_json::to_string(&snap_msg).unwrap().into(),
        ))
        .await
        .is_err()
    {
        return;
    }

    // 2. Established Session: Channel Setup
    let (outbound_tx, mut outbound_rx) =
        mpsc::channel::<ServerMessage>(host.config().client_queue_capacity);

    // Outbound writer task
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = outbound_rx.recv().await {
            match serde_json::to_string(&msg) {
                Ok(text) => {
                    if ws_sender.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to serialize server message: {e}");
                }
            }
        }
    });

    // Inbound listener task
    let inbound_host = host.clone();
    let inbound_subs = Arc::clone(&subscriptions);
    let inbound_outbound_tx = outbound_tx.clone();
    let max_ws_message_size = host.config().max_ws_message_size;

    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = ws_receiver.next().await {
            let msg = match result {
                Ok(m) => m,
                Err(e) => {
                    tracing::debug!("WebSocket read error: {e}");
                    break;
                }
            };

            match msg {
                Message::Text(text) => {
                    if text.len() > max_ws_message_size {
                        let _ = inbound_outbound_tx
                            .send(ServerMessage::Error(ProtocolError::new(
                                ProtocolErrorCode::PayloadTooLarge,
                                "Message exceeds allowed size",
                            )))
                            .await;
                        break;
                    }

                    match serde_json::from_str::<ClientMessage>(text.as_str()) {
                        Ok(client_msg) => match client_msg {
                            ClientMessage::Subscribe {
                                tags,
                                all_tags,
                                include_alarms: _,
                            } => {
                                let mut lock = inbound_subs.write().await;
                                if all_tags {
                                    lock.insert("*".to_string());
                                }
                                for tag in tags {
                                    lock.insert(tag);
                                }
                            }
                            ClientMessage::Unsubscribe { tags } => {
                                let mut lock = inbound_subs.write().await;
                                for tag in tags {
                                    lock.remove(&tag);
                                }
                            }
                            ClientMessage::RequestSnapshot => {
                                let snapshot = inbound_host.get_snapshot().await;
                                let _ = inbound_outbound_tx
                                    .send(ServerMessage::Snapshot(snapshot))
                                    .await;
                            }
                            ClientMessage::Ping { nonce } => {
                                let _ = inbound_outbound_tx
                                    .send(ServerMessage::Pong { nonce })
                                    .await;
                            }
                            ClientMessage::Command {
                                command_id,
                                command,
                            } => match inbound_host.dispatch_command(command).await {
                                Ok(_) => {
                                    let _ = inbound_outbound_tx
                                        .send(ServerMessage::CommandAck {
                                            command_id,
                                            success: true,
                                            message: None,
                                            error_code: None,
                                        })
                                        .await;
                                }
                                Err(e) => {
                                    let _ = inbound_outbound_tx
                                        .send(ServerMessage::CommandAck {
                                            command_id,
                                            success: false,
                                            message: Some(e.to_string()),
                                            error_code: Some("COMMAND_REJECTED".to_string()),
                                        })
                                        .await;
                                }
                            },
                            ClientMessage::Handshake { .. } => {
                                let _ = inbound_outbound_tx
                                    .send(ServerMessage::Error(ProtocolError::new(
                                        ProtocolErrorCode::InvalidMessage,
                                        "Handshake already completed",
                                    )))
                                    .await;
                            }
                        },
                        Err(e) => {
                            let _ = inbound_outbound_tx
                                .send(ServerMessage::Error(ProtocolError::new(
                                    ProtocolErrorCode::InvalidMessage,
                                    format!("Malformed message: {e}"),
                                )))
                                .await;
                        }
                    }
                }
                Message::Ping(_) => {
                    // Handled at transport level
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Broadcast forwarder task
    let mut broadcast_rx = host.subscribe_broadcast();
    let bcast_subs = Arc::clone(&subscriptions);
    let bcast_outbound_tx = outbound_tx.clone();
    let bcast_host = host.clone();

    let mut broadcast_task = tokio::spawn(async move {
        loop {
            match broadcast_rx.recv().await {
                Ok(msg) => match msg {
                    ServerMessage::Delta(delta) => {
                        let subs = bcast_subs.read().await;
                        // If no specific subscriptions or wildcard, send all
                        let filtered_delta = if subs.is_empty() || subs.contains("*") {
                            Some(delta)
                        } else {
                            let filtered_tags: Vec<TagValueSampleDto> = delta
                                .changed_tags
                                .into_iter()
                                .filter(|tag| subs.contains(&tag.tag))
                                .collect();

                            if filtered_tags.is_empty() {
                                None
                            } else {
                                Some(DeltaUpdateDto {
                                    seq: delta.seq,
                                    sim_time_sec: delta.sim_time_sec,
                                    tick_count: delta.tick_count,
                                    changed_tags: filtered_tags,
                                })
                            }
                        };

                        if let Some(d) = filtered_delta {
                            if bcast_outbound_tx.try_send(ServerMessage::Delta(d)).is_err() {
                                tracing::warn!("Client outbound queue full; dropping delta frame");
                            }
                        }
                    }
                    ServerMessage::AlarmUpdate(alarms) => {
                        let subs = bcast_subs.read().await;
                        if subs.is_empty() || subs.contains("*") {
                            let _ = bcast_outbound_tx.try_send(ServerMessage::AlarmUpdate(alarms));
                        } else {
                            let filtered: Vec<AlarmSnapshotDto> = alarms
                                .into_iter()
                                .filter(|a| subs.contains(&a.alarm_name))
                                .collect();
                            if !filtered.is_empty() {
                                let _ = bcast_outbound_tx
                                    .send(ServerMessage::AlarmUpdate(filtered))
                                    .await;
                            }
                        }
                    }
                    ServerMessage::Event(ev) => {
                        let _ = bcast_outbound_tx.try_send(ServerMessage::Event(ev));
                    }
                    ServerMessage::ScenarioUpdate(sc) => {
                        let _ = bcast_outbound_tx.try_send(ServerMessage::ScenarioUpdate(sc));
                    }
                    _ => {}
                },
                Err(broadcast::error::RecvError::Lagged(missed)) => {
                    tracing::warn!(
                        "Client lagged behind broadcast by {missed} frames; triggering resync snapshot"
                    );
                    let snapshot = bcast_host.get_snapshot().await;
                    let _ = bcast_outbound_tx.try_send(ServerMessage::Snapshot(snapshot));
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });

    // Wait until any task completes
    tokio::select! {
        _ = &mut send_task => {},
        _ = &mut recv_task => {},
        _ = &mut broadcast_task => {},
    }

    send_task.abort();
    recv_task.abort();
    broadcast_task.abort();
    tracing::debug!("WebSocket session ended for client: {client_id}");
}
