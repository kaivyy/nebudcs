//! Cluster, High Availability (HA) Lifecycle, Split-Brain Protection, and State Replication Manager.
//!
//! Standards Reference: High-Availability Distributed Systems, IEC 62443 Security Boundaries,
//! Leader-Follower Consensus with Epoch Fencing.

use nebudcs_protocol::{
    ClusterNodeDto, ClusterTopologyDto, FailoverRequestDto, FailureInjectionRequestDto,
    FailureInjectionStatusDto, HaHeartbeatMessage, HaState, NodeRole, PromotionResultDto,
    ReplicationSnapshotDto,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Computes a hex-encoded SHA-256 digest.
fn compute_sha256(data: &[u8]) -> String {
    crate::backup::sha256::sha256_hex(data)
}

/// Core High Availability & Distributed Cluster Manager.
#[derive(Debug)]
pub struct ClusterManager {
    cluster_id: String,
    local_node_id: String,
    local_node_name: String,
    local_role: NodeRole,
    local_ha_state: HaState,
    leader_id: Option<String>,
    leader_epoch: AtomicU64,
    lease_remaining_secs: f64,
    lease_duration_secs: f64,
    nodes: BTreeMap<String, ClusterNodeDto>,
    heartbeat_seq: AtomicU64,
    split_brain_fencing_active: bool,
    failure_injection: Option<FailureInjectionStatusDto>,
    last_replication_snapshot: Option<ReplicationSnapshotDto>,
    simulated_replication_lag_ms: f64,
}

impl ClusterManager {
    /// Creates a new ClusterManager initialized in Primary/Standalone mode.
    pub fn new(
        cluster_id: impl Into<String>,
        local_node_id: impl Into<String>,
        local_node_name: impl Into<String>,
        role: NodeRole,
        initial_ha_state: HaState,
    ) -> Self {
        let cluster_id = cluster_id.into();
        let local_node_id = local_node_id.into();
        let local_node_name = local_node_name.into();

        let mut mgr = Self {
            cluster_id: cluster_id.clone(),
            local_node_id: local_node_id.clone(),
            local_node_name: local_node_name.clone(),
            local_role: role,
            local_ha_state: initial_ha_state,
            leader_id: if initial_ha_state.is_active_primary() {
                Some(local_node_id.clone())
            } else {
                None
            },
            leader_epoch: AtomicU64::new(1),
            lease_remaining_secs: 5.0,
            lease_duration_secs: 5.0,
            nodes: BTreeMap::new(),
            heartbeat_seq: AtomicU64::new(0),
            split_brain_fencing_active: false,
            failure_injection: None,
            last_replication_snapshot: None,
            simulated_replication_lag_ms: 12.5,
        };

        // Register default cluster topology nodes
        mgr.register_default_cluster_topology();
        mgr
    }

    /// Registers standard industrial DCS cluster topology nodes.
    fn register_default_cluster_topology(&mut self) {
        let local_id = self.local_node_id.clone();
        let is_primary = self.local_ha_state.is_active_primary();

        // 1. Local node (Primary / Control Simulation)
        self.nodes.insert(
            local_id.clone(),
            ClusterNodeDto {
                node_id: local_id.clone(),
                name: self.local_node_name.clone(),
                role: self.local_role,
                ha_state: self.local_ha_state,
                endpoint_url: "http://127.0.0.1:8088".to_string(),
                is_local: true,
                epoch: self.leader_epoch.load(Ordering::SeqCst),
                last_heartbeat_secs_ago: 0.0,
                health_status: if is_primary { "HEALTHY_PRIMARY".to_string() } else { "HEALTHY".to_string() },
                replication_lag_ms: 0.0,
            },
        );

        // 2. Secondary Standby Node
        self.nodes.insert(
            "node-standby-02".to_string(),
            ClusterNodeDto {
                node_id: "node-standby-02".to_string(),
                name: "Secondary Standby Replica".to_string(),
                role: NodeRole::Standby,
                ha_state: HaState::Synchronized,
                endpoint_url: "http://127.0.0.1:8089".to_string(),
                is_local: false,
                epoch: self.leader_epoch.load(Ordering::SeqCst),
                last_heartbeat_secs_ago: 0.25,
                health_status: "SYNCHRONIZED_STANDBY".to_string(),
                replication_lag_ms: self.simulated_replication_lag_ms,
            },
        );

        // 3. Historian Node
        self.nodes.insert(
            "node-historian-01".to_string(),
            ClusterNodeDto {
                node_id: "node-historian-01".to_string(),
                name: "Plant Historian Service".to_string(),
                role: NodeRole::Historian,
                ha_state: HaState::Synchronized,
                endpoint_url: "http://127.0.0.1:8090".to_string(),
                is_local: false,
                epoch: self.leader_epoch.load(Ordering::SeqCst),
                last_heartbeat_secs_ago: 0.50,
                health_status: "HEALTHY".to_string(),
                replication_lag_ms: 5.0,
            },
        );

        // 4. Industrial Protocol Gateway Node
        self.nodes.insert(
            "node-comm-01".to_string(),
            ClusterNodeDto {
                node_id: "node-comm-01".to_string(),
                name: "Fieldbus Protocol Gateway (Modbus/OPC-UA)".to_string(),
                role: NodeRole::Communication,
                ha_state: HaState::Synchronized,
                endpoint_url: "http://127.0.0.1:8091".to_string(),
                is_local: false,
                epoch: self.leader_epoch.load(Ordering::SeqCst),
                last_heartbeat_secs_ago: 0.35,
                health_status: "HEALTHY".to_string(),
                replication_lag_ms: 8.0,
            },
        );

        // 5. Web & API Edge Gateway
        self.nodes.insert(
            "node-gateway-01".to_string(),
            ClusterNodeDto {
                node_id: "node-gateway-01".to_string(),
                name: "Edge API & WebSocket Gateway".to_string(),
                role: NodeRole::WebGateway,
                ha_state: HaState::Synchronized,
                endpoint_url: "http://127.0.0.1:8080".to_string(),
                is_local: false,
                epoch: self.leader_epoch.load(Ordering::SeqCst),
                last_heartbeat_secs_ago: 0.10,
                health_status: "HEALTHY".to_string(),
                replication_lag_ms: 2.0,
            },
        );
    }

    /// Returns the current comprehensive cluster topology and HA consensus state.
    pub fn get_topology(&self) -> ClusterTopologyDto {
        let nodes_vec: Vec<ClusterNodeDto> = self.nodes.values().cloned().collect();
        let total_nodes = nodes_vec.len();
        let active_nodes = nodes_vec
            .iter()
            .filter(|n| n.ha_state != HaState::Failed && n.last_heartbeat_secs_ago < 10.0)
            .count();

        ClusterTopologyDto {
            cluster_id: self.cluster_id.clone(),
            current_leader_id: self.leader_id.clone(),
            leader_epoch: self.leader_epoch.load(Ordering::SeqCst),
            lease_remaining_secs: self.lease_remaining_secs,
            local_node_id: self.local_node_id.clone(),
            local_ha_state: self.local_ha_state,
            nodes: nodes_vec,
            split_brain_fencing_active: self.split_brain_fencing_active,
            total_nodes_count: total_nodes,
            active_nodes_count: active_nodes,
            replication_health: if self.split_brain_fencing_active {
                "FENCED_SPLIT_BRAIN_PROTECTED".to_string()
            } else if self.local_ha_state == HaState::Degraded {
                "DEGRADED".to_string()
            } else {
                "OPTIMAL_SYNCHRONIZED".to_string()
            },
        }
    }

    /// Emits a local heartbeat to be broadcast across the cluster.
    pub fn generate_heartbeat(&self, sim_time_secs: f64, sim_tick: u64) -> HaHeartbeatMessage {
        let seq = self.heartbeat_seq.fetch_add(1, Ordering::SeqCst) + 1;
        HaHeartbeatMessage {
            sender_node_id: self.local_node_id.clone(),
            sender_role: self.local_role,
            sender_ha_state: self.local_ha_state,
            epoch: self.leader_epoch.load(Ordering::SeqCst),
            sequence: seq,
            sim_time_secs,
            sim_tick,
            timestamp_iso: format!("t_{:.3}s", sim_time_secs),
        }
    }

    /// Processes an inbound heartbeat from a peer node with split-brain protection and epoch fencing.
    pub fn handle_heartbeat(&mut self, hb: HaHeartbeatMessage) -> Result<(), String> {
        let local_epoch = self.leader_epoch.load(Ordering::SeqCst);

        // 1. Split-Brain Protection: Check for conflicting Primaries
        if hb.sender_ha_state.is_active_primary() && self.local_ha_state.is_active_primary() {
            if hb.epoch > local_epoch {
                // Inbound primary has higher authority / newer epoch.
                // Local primary must immediately fence itself and demote to STANDBY.
                self.local_ha_state = HaState::Standby;
                self.leader_id = Some(hb.sender_node_id.clone());
                self.leader_epoch.store(hb.epoch, Ordering::SeqCst);
                self.split_brain_fencing_active = true;

                if let Some(local_node) = self.nodes.get_mut(&self.local_node_id) {
                    local_node.ha_state = HaState::Standby;
                    local_node.epoch = hb.epoch;
                    local_node.health_status = "FENCED_DEMOTED_TO_STANDBY".to_string();
                }
            } else if hb.epoch < local_epoch {
                // Stale primary detected: reject heartbeat and fence old node
                self.split_brain_fencing_active = true;
                return Err(format!(
                    "Split-brain detected: Rejected stale heartbeat from node '{}' with epoch {} < local epoch {}",
                    hb.sender_node_id, hb.epoch, local_epoch
                ));
            } else if hb.sender_node_id != self.local_node_id {
                // Same epoch but different node claiming Primary: tie-break by deterministic lexical node ID
                if hb.sender_node_id < self.local_node_id {
                    self.local_ha_state = HaState::Standby;
                    self.leader_id = Some(hb.sender_node_id.clone());
                    self.split_brain_fencing_active = true;
                } else {
                    // We win tie-break; peer must demote
                    self.split_brain_fencing_active = true;
                }
            }
        }

        // 2. Update peer entry in topology map
        if let Some(node) = self.nodes.get_mut(&hb.sender_node_id) {
            node.ha_state = hb.sender_ha_state;
            node.epoch = hb.epoch;
            node.last_heartbeat_secs_ago = 0.0;
            node.health_status = if hb.sender_ha_state == HaState::Failed {
                "FAILED".to_string()
            } else {
                "HEALTHY".to_string()
            };
        } else {
            // Dynamically register previously unseen node
            self.nodes.insert(
                hb.sender_node_id.clone(),
                ClusterNodeDto {
                    node_id: hb.sender_node_id.clone(),
                    name: format!("Node {}", hb.sender_node_id),
                    role: hb.sender_role,
                    ha_state: hb.sender_ha_state,
                    endpoint_url: "http://dynamic-peer".to_string(),
                    is_local: false,
                    epoch: hb.epoch,
                    last_heartbeat_secs_ago: 0.0,
                    health_status: "HEALTHY".to_string(),
                    replication_lag_ms: 10.0,
                },
            );
        }

        Ok(())
    }

    /// Periodic tick for lease management, peer heartbeat aging, and failure injection countdown.
    pub fn process_tick(&mut self, dt_secs: f64) {
        // 1. Advance peer heartbeat age
        for (id, node) in self.nodes.iter_mut() {
            if id != &self.local_node_id {
                node.last_heartbeat_secs_ago += dt_secs;
                if node.last_heartbeat_secs_ago > 5.0 && node.ha_state != HaState::Failed {
                    node.health_status = "HEARTBEAT_TIMEOUT_UNREACHABLE".to_string();
                }
            }
        }

        // 2. Leader lease countdown if primary
        if self.local_ha_state.is_active_primary() {
            self.lease_remaining_secs = (self.lease_remaining_secs - dt_secs).max(0.0);
            if self.lease_remaining_secs <= 0.0 {
                // Lease expired: refresh if healthy
                self.lease_remaining_secs = self.lease_duration_secs;
            }
        }

        // 3. Handle active failure injection countdown
        if let Some(ref mut fi) = self.failure_injection {
            fi.remaining_secs -= dt_secs;
            if fi.remaining_secs <= 0.0 {
                self.failure_injection = None;
                self.simulated_replication_lag_ms = 12.5;
            }
        }
    }

    /// Triggers controlled failover or manual node promotion.
    pub fn request_failover(&mut self, req: FailoverRequestDto) -> Result<PromotionResultDto, String> {
        let prev_leader = self.leader_id.clone();
        let new_epoch = self.leader_epoch.fetch_add(1, Ordering::SeqCst) + 1;

        if req.target_leader_id == self.local_node_id {
            // Promoting local node
            self.local_ha_state = HaState::Primary;
            self.leader_id = Some(self.local_node_id.clone());
            self.lease_remaining_secs = self.lease_duration_secs;
            self.split_brain_fencing_active = false;

            if let Some(node) = self.nodes.get_mut(&self.local_node_id) {
                node.ha_state = HaState::Primary;
                node.epoch = new_epoch;
                node.health_status = "HEALTHY_PROMOTED_PRIMARY".to_string();
            }

            // Mark previous leader as STANDBY
            if let Some(ref old_id) = prev_leader {
                if old_id != &self.local_node_id {
                    if let Some(old_node) = self.nodes.get_mut(old_id) {
                        old_node.ha_state = HaState::Standby;
                    }
                }
            }

            Ok(PromotionResultDto {
                success: true,
                promoted_node_id: self.local_node_id.clone(),
                new_epoch,
                previous_leader_id: prev_leader,
                message: format!(
                    "Node '{}' successfully promoted to PRIMARY at epoch {new_epoch}. Reason: {}",
                    self.local_node_id, req.reason
                ),
            })
        } else {
            // Promoting a peer node (demoting local if currently primary)
            if self.local_ha_state.is_active_primary() {
                self.local_ha_state = HaState::Standby;
                if let Some(local_node) = self.nodes.get_mut(&self.local_node_id) {
                    local_node.ha_state = HaState::Standby;
                    local_node.health_status = "STANDBY_DEMOTED".to_string();
                }
            }

            self.leader_id = Some(req.target_leader_id.clone());
            if let Some(target_node) = self.nodes.get_mut(&req.target_leader_id) {
                target_node.ha_state = HaState::Primary;
                target_node.epoch = new_epoch;
                target_node.health_status = "HEALTHY_PRIMARY".to_string();
            }

            Ok(PromotionResultDto {
                success: true,
                promoted_node_id: req.target_leader_id.clone(),
                new_epoch,
                previous_leader_id: prev_leader,
                message: format!(
                    "Target node '{}' promoted to PRIMARY at epoch {new_epoch}. Reason: {}",
                    req.target_leader_id, req.reason
                ),
            })
        }
    }

    /// Creates an authoritative replication snapshot with cryptographic SHA-256 verification.
    pub fn create_replication_snapshot(
        &mut self,
        sim_time_secs: f64,
        sim_tick: u64,
        tag_count: usize,
        active_alarms_count: usize,
        sis_tripped: bool,
        historian_seq: u64,
    ) -> ReplicationSnapshotDto {
        let epoch = self.leader_epoch.load(Ordering::SeqCst);
        let seq = self.heartbeat_seq.load(Ordering::SeqCst);

        let payload = serde_json::json!({
            "epoch": epoch,
            "seq": seq,
            "sim_time": sim_time_secs,
            "sim_tick": sim_tick,
            "tag_count": tag_count,
            "alarms_count": active_alarms_count,
            "sis_tripped": sis_tripped,
            "historian_seq": historian_seq,
            "project_version": "1.0.0",
        });

        let payload_str = serde_json::to_string(&payload).unwrap_or_default();
        let checksum = compute_sha256(payload_str.as_bytes());

        let snap = ReplicationSnapshotDto {
            epoch,
            sequence: seq,
            sim_time_secs,
            sim_tick,
            sim_state: "RUNNING".to_string(),
            project_version: "1.0.0".to_string(),
            tag_count,
            active_alarms_count,
            sis_tripped,
            historian_seq,
            checksum_sha256: checksum,
            payload_json: payload_str,
        };

        self.last_replication_snapshot = Some(snap.clone());
        snap
    }

    /// Ingests and verifies a replication snapshot on a standby node.
    pub fn apply_replication_snapshot(&mut self, snap: ReplicationSnapshotDto) -> Result<(), String> {
        let calc_hash = compute_sha256(snap.payload_json.as_bytes());
        if calc_hash != snap.checksum_sha256 {
            return Err(format!(
                "Replication snapshot integrity error: computed hash {calc_hash} != manifest {}",
                snap.checksum_sha256
            ));
        }

        let local_epoch = self.leader_epoch.load(Ordering::SeqCst);
        if snap.epoch < local_epoch {
            return Err(format!(
                "Rejected replication snapshot from stale epoch {} < local epoch {}",
                snap.epoch, local_epoch
            ));
        }

        self.leader_epoch.store(snap.epoch, Ordering::SeqCst);
        self.last_replication_snapshot = Some(snap);
        if self.local_ha_state == HaState::Synchronizing {
            self.local_ha_state = HaState::Synchronized;
        }

        Ok(())
    }

    /// Injects controlled infrastructure failure for HA verification.
    pub fn inject_failure(&mut self, req: FailureInjectionRequestDto) -> Result<FailureInjectionStatusDto, String> {
        let f_type = req.failure_type.to_lowercase();
        let duration = if req.duration_secs <= 0.0 { 10.0 } else { req.duration_secs };

        let msg = match f_type.as_str() {
            "delayheartbeat" | "delayed_heartbeat" => {
                if let Some(target) = req.target_node_id.as_ref() {
                    if let Some(node) = self.nodes.get_mut(target) {
                        node.last_heartbeat_secs_ago = 6.0; // trigger timeout
                    }
                }
                "Injected delayed heartbeat timeout".to_string()
            }
            "replicationlag" | "replication_lag" => {
                let lag = req.induced_lag_ms.unwrap_or(2500) as f64;
                self.simulated_replication_lag_ms = lag;
                if let Some(target) = req.target_node_id.as_ref() {
                    if let Some(node) = self.nodes.get_mut(target) {
                        node.replication_lag_ms = lag;
                    }
                }
                format!("Injected {lag:.0}ms replication lag")
            }
            "networkpartition" | "network_partition" => {
                self.split_brain_fencing_active = true;
                self.local_ha_state = HaState::Degraded;
                "Injected network partition and fencing".to_string()
            }
            "crashprocess" | "node_failure" => {
                if let Some(target) = req.target_node_id.as_ref() {
                    if let Some(node) = self.nodes.get_mut(target) {
                        node.ha_state = HaState::Failed;
                        node.health_status = "INJECTED_NODE_CRASH".to_string();
                    }
                }
                "Injected simulated node crash".to_string()
            }
            _ => return Err(format!("Unknown failure injection type: '{}'", req.failure_type)),
        };

        let status = FailureInjectionStatusDto {
            active: true,
            failure_type: Some(req.failure_type),
            target_node_id: req.target_node_id,
            remaining_secs: duration,
            message: msg,
        };

        self.failure_injection = Some(status.clone());
        Ok(status)
    }

    /// Clears any active failure injection and restores normal operation.
    pub fn clear_failure_injection(&mut self) -> FailureInjectionStatusDto {
        self.failure_injection = None;
        self.simulated_replication_lag_ms = 12.5;
        self.split_brain_fencing_active = false;
        if self.local_ha_state == HaState::Degraded {
            self.local_ha_state = HaState::Primary;
        }

        // Restore node health statuses
        for node in self.nodes.values_mut() {
            if node.ha_state == HaState::Failed {
                node.ha_state = HaState::Synchronized;
                node.health_status = "RECOVERED_SYNCHRONIZED".to_string();
            }
            node.replication_lag_ms = 12.5;
            node.last_heartbeat_secs_ago = 0.1;
        }

        FailureInjectionStatusDto {
            active: false,
            failure_type: None,
            target_node_id: None,
            remaining_secs: 0.0,
            message: "All failure injections cleared; normal cluster operation restored".to_string(),
        }
    }

    /// Returns the current failure injection status.
    pub fn failure_injection_status(&self) -> FailureInjectionStatusDto {
        self.failure_injection.clone().unwrap_or(FailureInjectionStatusDto {
            active: false,
            failure_type: None,
            target_node_id: None,
            remaining_secs: 0.0,
            message: "No active failure injection".to_string(),
        })
    }

    pub fn local_node_id(&self) -> &str {
        &self.local_node_id
    }

    pub fn local_ha_state(&self) -> HaState {
        self.local_ha_state
    }

    pub fn leader_epoch(&self) -> u64 {
        self.leader_epoch.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_initial_topology() {
        let mgr = ClusterManager::new(
            "test-cluster",
            "node-sim-01",
            "Sim Primary",
            NodeRole::ControlSimulation,
            HaState::Primary,
        );

        let topo = mgr.get_topology();
        assert_eq!(topo.cluster_id, "test-cluster");
        assert_eq!(topo.current_leader_id, Some("node-sim-01".to_string()));
        assert_eq!(topo.leader_epoch, 1);
        assert_eq!(topo.local_ha_state, HaState::Primary);
        assert_eq!(topo.nodes.len(), 5);
        assert!(!topo.split_brain_fencing_active);
    }

    #[test]
    fn test_heartbeat_exchange_and_peer_update() {
        let mut mgr = ClusterManager::new(
            "test-cluster",
            "node-sim-01",
            "Sim Primary",
            NodeRole::ControlSimulation,
            HaState::Primary,
        );

        let hb = HaHeartbeatMessage {
            sender_node_id: "node-standby-02".to_string(),
            sender_role: NodeRole::Standby,
            sender_ha_state: HaState::Synchronized,
            epoch: 1,
            sequence: 42,
            sim_time_secs: 10.0,
            sim_tick: 200,
            timestamp_iso: "t_10.000s".to_string(),
        };

        assert!(mgr.handle_heartbeat(hb).is_ok());
        let topo = mgr.get_topology();
        let standby = topo.nodes.iter().find(|n| n.node_id == "node-standby-02").unwrap();
        assert_eq!(standby.ha_state, HaState::Synchronized);
        assert_eq!(standby.last_heartbeat_secs_ago, 0.0);
    }

    #[test]
    fn test_split_brain_fencing_higher_epoch() {
        let mut mgr = ClusterManager::new(
            "test-cluster",
            "node-sim-01",
            "Sim Primary",
            NodeRole::ControlSimulation,
            HaState::Primary,
        );

        // Another primary arrives with epoch 2 > local epoch 1
        let conflicting_hb = HaHeartbeatMessage {
            sender_node_id: "node-standby-02".to_string(),
            sender_role: NodeRole::ControlSimulation,
            sender_ha_state: HaState::Primary,
            epoch: 2,
            sequence: 1,
            sim_time_secs: 15.0,
            sim_tick: 300,
            timestamp_iso: "t_15.000s".to_string(),
        };

        assert!(mgr.handle_heartbeat(conflicting_hb).is_ok());
        assert_eq!(mgr.local_ha_state(), HaState::Standby);
        assert!(mgr.get_topology().split_brain_fencing_active);
        assert_eq!(mgr.leader_epoch(), 2);
    }

    #[test]
    fn test_split_brain_rejection_stale_epoch() {
        let mut mgr = ClusterManager::new(
            "test-cluster",
            "node-sim-01",
            "Sim Primary",
            NodeRole::ControlSimulation,
            HaState::Primary,
        );
        mgr.leader_epoch.store(5, Ordering::SeqCst);

        // Stale heartbeat with epoch 3 < 5
        let stale_hb = HaHeartbeatMessage {
            sender_node_id: "node-old-primary".to_string(),
            sender_role: NodeRole::ControlSimulation,
            sender_ha_state: HaState::Primary,
            epoch: 3,
            sequence: 10,
            sim_time_secs: 5.0,
            sim_tick: 100,
            timestamp_iso: "t_5.000s".to_string(),
        };

        let res = mgr.handle_heartbeat(stale_hb);
        assert!(res.is_err());
        assert!(mgr.get_topology().split_brain_fencing_active);
    }

    #[test]
    fn test_controlled_promotion_and_failover() {
        let mut mgr = ClusterManager::new(
            "test-cluster",
            "node-standby-02",
            "Standby",
            NodeRole::Standby,
            HaState::Standby,
        );

        let req = FailoverRequestDto {
            target_leader_id: "node-standby-02".to_string(),
            target_epoch: 2,
            force: false,
            reason: "Simulated Primary Unreachable".to_string(),
        };

        let res = mgr.request_failover(req).unwrap();
        assert!(res.success);
        assert_eq!(res.promoted_node_id, "node-standby-02");
        assert_eq!(res.new_epoch, 2);
        assert_eq!(mgr.local_ha_state(), HaState::Primary);
    }

    #[test]
    fn test_replication_snapshot_create_and_apply() {
        let mut primary = ClusterManager::new(
            "test-cluster",
            "node-01",
            "Primary",
            NodeRole::ControlSimulation,
            HaState::Primary,
        );
        let mut standby = ClusterManager::new(
            "test-cluster",
            "node-02",
            "Standby",
            NodeRole::Standby,
            HaState::Synchronizing,
        );

        let snap = primary.create_replication_snapshot(12.5, 250, 150, 2, false, 999);
        assert_eq!(snap.epoch, 1);
        assert_eq!(snap.tag_count, 150);

        // Standby applies snapshot successfully
        assert!(standby.apply_replication_snapshot(snap).is_ok());
        assert_eq!(standby.local_ha_state(), HaState::Synchronized);
    }

    #[test]
    fn test_failure_injection_and_clear() {
        let mut mgr = ClusterManager::new(
            "test-cluster",
            "node-sim-01",
            "Sim Primary",
            NodeRole::ControlSimulation,
            HaState::Primary,
        );

        let fi_req = FailureInjectionRequestDto {
            failure_type: "NetworkPartition".to_string(),
            target_node_id: None,
            duration_secs: 15.0,
            induced_lag_ms: None,
        };

        let fi = mgr.inject_failure(fi_req).unwrap();
        assert!(fi.active);
        assert_eq!(mgr.local_ha_state(), HaState::Degraded);
        assert!(mgr.get_topology().split_brain_fencing_active);

        let cleared = mgr.clear_failure_injection();
        assert!(!cleared.active);
        assert_eq!(mgr.local_ha_state(), HaState::Primary);
        assert!(!mgr.get_topology().split_brain_fencing_active);
    }
}
