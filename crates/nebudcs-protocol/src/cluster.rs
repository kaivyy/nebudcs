//! Enterprise High Availability, Distributed Runtime, State Replication & Observability DTOs.
//!
//! Standards Reference: IEC 62443, High-Availability Systems Engineering, Distributed Consensus & Fencing.

use serde::{Deserialize, Serialize};

/// Role assumed by a node within a distributed NebuDCS cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    /// Core simulation & control execution authority.
    ControlSimulation,
    /// Hot or warm standby replica ready for promotion.
    Standby,
    /// Dedicated time-series historian & event logging node.
    Historian,
    /// Industrial protocol gateway (Modbus, OPC UA, MQTT, DNP3).
    Communication,
    /// Edge/Web API, WebSocket termination, and UI gateway.
    WebGateway,
}

impl NodeRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ControlSimulation => "ControlSimulation",
            Self::Standby => "Standby",
            Self::Historian => "Historian",
            Self::Communication => "Communication",
            Self::WebGateway => "WebGateway",
        }
    }
}

/// Comprehensive High Availability lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HaState {
    /// Single-node execution without replication.
    Standalone,
    /// Node is initializing runtime environment.
    Starting,
    /// Active primary authority executing simulation and control ticks.
    Primary,
    /// Secondary replica mirroring state from primary.
    Standby,
    /// Replica is performing full snapshot resynchronization.
    Synchronizing,
    /// Replica is synchronized and ready for immediate hot failover.
    Synchronized,
    /// Heartbeat or peer loss detected; operation continued with warning.
    Degraded,
    /// Primary failure confirmed; promoting standby to primary.
    Failover,
    /// Node encountered unrecoverable fault or has been fenced.
    Failed,
    /// Recovering from fault; requesting state catch-up.
    Recovering,
    /// Graceful cluster shutdown in progress.
    ShuttingDown,
}

impl HaState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Standalone => "STANDALONE",
            Self::Starting => "STARTING",
            Self::Primary => "PRIMARY",
            Self::Standby => "STANDBY",
            Self::Synchronizing => "SYNCHRONIZING",
            Self::Synchronized => "SYNCHRONIZED",
            Self::Degraded => "DEGRADED",
            Self::Failover => "FAILOVER",
            Self::Failed => "FAILED",
            Self::Recovering => "RECOVERING",
            Self::ShuttingDown => "SHUTTING_DOWN",
        }
    }

    pub fn is_active_primary(&self) -> bool {
        matches!(self, Self::Primary | Self::Standalone)
    }

    pub fn can_accept_commands(&self) -> bool {
        matches!(self, Self::Primary | Self::Standalone)
    }
}

/// Description of a cluster member node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterNodeDto {
    pub node_id: String,
    pub name: String,
    pub role: NodeRole,
    pub ha_state: HaState,
    pub endpoint_url: String,
    pub is_local: bool,
    pub epoch: u64,
    pub last_heartbeat_secs_ago: f64,
    pub health_status: String,
    pub replication_lag_ms: f64,
}

/// Complete cluster topology and consensus status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterTopologyDto {
    pub cluster_id: String,
    pub current_leader_id: Option<String>,
    pub leader_epoch: u64,
    pub lease_remaining_secs: f64,
    pub local_node_id: String,
    pub local_ha_state: HaState,
    pub nodes: Vec<ClusterNodeDto>,
    pub split_brain_fencing_active: bool,
    pub total_nodes_count: usize,
    pub active_nodes_count: usize,
    pub replication_health: String,
}

/// Heartbeat message broadcasted between cluster nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HaHeartbeatMessage {
    pub sender_node_id: String,
    pub sender_role: NodeRole,
    pub sender_ha_state: HaState,
    pub epoch: u64,
    pub sequence: u64,
    pub sim_time_secs: f64,
    pub sim_tick: u64,
    pub timestamp_iso: String,
}

/// Authoritative replicated state packet streamed from primary to standby.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplicationSnapshotDto {
    pub epoch: u64,
    pub sequence: u64,
    pub sim_time_secs: f64,
    pub sim_tick: u64,
    pub sim_state: String,
    pub project_version: String,
    pub tag_count: usize,
    pub active_alarms_count: usize,
    pub sis_tripped: bool,
    pub historian_seq: u64,
    pub checksum_sha256: String,
    pub payload_json: String,
}

/// Explicit request to trigger controlled failover or manual promotion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailoverRequestDto {
    pub target_leader_id: String,
    pub target_epoch: u64,
    pub force: bool,
    pub reason: String,
}

/// Result of a promotion or demotion operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionResultDto {
    pub success: bool,
    pub promoted_node_id: String,
    pub new_epoch: u64,
    pub previous_leader_id: Option<String>,
    pub message: String,
}

/// Controlled infrastructure failure injection request for HA testing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailureInjectionRequestDto {
    pub failure_type: String,
    pub target_node_id: Option<String>,
    pub duration_secs: f64,
    pub induced_lag_ms: Option<u64>,
}

/// Status of an active or cleared failure injection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailureInjectionStatusDto {
    pub active: bool,
    pub failure_type: Option<String>,
    pub target_node_id: Option<String>,
    pub remaining_secs: f64,
    pub message: String,
}

/// Unified, cross-cutting audit event across all platform operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEventDto {
    pub event_id: String,
    pub timestamp_iso: String,
    pub sim_time_secs: f64,
    pub actor: String,
    pub role: String,
    pub source: String,
    pub target: String,
    pub action: String,
    pub prev_value: Option<String>,
    pub new_value: Option<String>,
    pub status: String,
    pub reason: Option<String>,
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub previous_hash: Option<String>,
    #[serde(default)]
    pub event_hash: Option<String>,
}

/// Unified telemetry and observability across all NebuDCS subsystems.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubsystemObservabilityDto {
    // System
    pub system_cpu_pct: f64,
    pub system_memory_rss_bytes: u64,
    pub system_uptime_secs: f64,
    // Simulation
    pub sim_freq_hz: f64,
    pub sim_dt: f64,
    pub sim_tick: u64,
    pub sim_convergence_failures: u64,
    pub sim_is_deterministic: bool,
    // Control
    pub control_tasks_active: usize,
    pub control_overruns_count: u64,
    pub control_faults_active: usize,
    // I/O
    pub io_channels_total: usize,
    pub io_bad_quality_count: usize,
    pub io_stale_data_count: usize,
    // Protocols
    pub protocol_connections_active: usize,
    pub protocol_packet_rate: f64,
    pub protocol_error_count: u64,
    // SIS
    pub sis_active_trips: usize,
    pub sis_bypasses_active: usize,
    pub sis_proof_test_active: bool,
    // Historian
    pub historian_write_rate_sps: f64,
    pub historian_queue_depth: usize,
    pub historian_total_samples: u64,
    // OTS
    pub ots_active_sessions: usize,
    pub ots_scenarios_running: usize,
    // Validation
    pub validation_cases_passed: usize,
    pub validation_cases_total: usize,
    pub validation_coverage_pct: f64,
    // Cluster & HA
    pub cluster_ha_state: HaState,
    pub cluster_leader_id: Option<String>,
    pub cluster_epoch: u64,
    pub cluster_replication_lag_ms: f64,
}

/// Coherent cross-domain diagnostic item linking directly to affected objects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticItemDto {
    pub id: String,
    pub domain: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub entity_id: Option<String>,
    pub entity_type: Option<String>,
    pub timestamp_iso: String,
    pub suggested_action: String,
}
