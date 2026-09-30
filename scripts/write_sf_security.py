store_forward_content = r"""//! Store-and-Forward buffering for non-realtime industrial telemetry and historian spooling.
//!
//! Provides bounded in-memory and disk-spooled ring buffers that store historical
//! records during upstream network interruptions, draining in strict FIFO order
//! upon reconnection with duplicate detection and timestamp fidelity.

use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

use super::types::IndustrialTagValue;
use crate::error::ProtocolError;

/// Buffer overflow mitigation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OverflowPolicy {
    /// Discards oldest uncommitted record to preserve most recent telemetry.
    #[default]
    DropOldest,
    /// Discards incoming samples when buffer is full, prioritizing older un-replayed records.
    DropNewest,
}

/// High-throughput bounded Store-and-Forward queue.
#[derive(Debug, Clone)]
pub struct StoreAndForwardQueue {
    capacity: usize,
    buffer: VecDeque<IndustrialTagValue>,
    overflow_policy: OverflowPolicy,
    dropped_count: u64,
    spooled_count: u64,
    replayed_count: u64,
    seen_keys: HashSet<(String, u64)>,
}

impl StoreAndForwardQueue {
    pub fn new(capacity: usize, overflow_policy: OverflowPolicy) -> Self {
        Self {
            capacity: capacity.max(100),
            buffer: VecDeque::with_capacity(capacity.min(10_000)),
            overflow_policy,
            dropped_count: 0,
            spooled_count: 0,
            replayed_count: 0,
            seen_keys: HashSet::new(),
        }
    }

    /// Enqueues a sample during network disconnection, honoring buffer capacity.
    pub fn enqueue(&mut self, sample: IndustrialTagValue) -> Result<(), ProtocolError> {
        let key = (
            sample.canonical_id.clone(),
            sample.timestamp.source_timestamp_ms.unwrap_or(sample.timestamp.ingestion_timestamp_ms),
        );

        // Deduplication
        if self.seen_keys.contains(&key) {
            return Ok(());
        }

        if self.buffer.len() >= self.capacity {
            self.dropped_count += 1;
            match self.overflow_policy {
                OverflowPolicy::DropOldest => {
                    if let Some(dropped) = self.buffer.pop_front() {
                        let d_key = (
                            dropped.canonical_id,
                            dropped.timestamp.source_timestamp_ms.unwrap_or(dropped.timestamp.ingestion_timestamp_ms),
                        );
                        self.seen_keys.remove(&d_key);
                    }
                }
                OverflowPolicy::DropNewest => {
                    return Err(ProtocolError::InvalidPayload("Store-and-forward queue full; new sample dropped".into()));
                }
            }
        }

        self.seen_keys.insert(key);
        self.buffer.push_back(sample);
        self.spooled_count += 1;
        Ok(())
    }

    /// Drains a batch of queued samples in strict FIFO order for upstream transmission.
    pub fn drain_batch(&mut self, max_batch_size: usize) -> Vec<IndustrialTagValue> {
        let batch_size = max_batch_size.min(self.buffer.len());
        let mut batch = Vec::with_capacity(batch_size);

        for _ in 0..batch_size {
            if let Some(sample) = self.buffer.pop_front() {
                let key = (
                    sample.canonical_id.clone(),
                    sample.timestamp.source_timestamp_ms.unwrap_or(sample.timestamp.ingestion_timestamp_ms),
                );
                self.seen_keys.remove(&key);
                self.replayed_count += 1;
                batch.push(sample);
            }
        }

        batch
    }

    /// Current number of spooled items awaiting replay.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn dropped_count(&self) -> u64 {
        self.dropped_count
    }

    pub fn spooled_count(&self) -> u64 {
        self.spooled_count
    }

    pub fn replayed_count(&self) -> u64 {
        self.replayed_count
    }
}
"""

with open("crates/nebudcs-protocol/src/industrial/store_forward.rs", "w") as f:
    f.write(store_forward_content)
print("Wrote store_forward.rs successfully")

security_content = r"""//! ISA-95 / IEC 62443 industrial cybersecurity architecture, zones, conduits, and SSRF guards.

use serde::{Deserialize, Serialize};
use std::net::IpAddr;

use super::adapter::ProtocolType;
use crate::error::ProtocolError;

/// ISA-95 hierarchical process automation level classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Isa95Level {
    /// Level 0: Physical Process (Piping, Tanks, Reacting Medium).
    Level0Process,
    /// Level 1: Basic Sensing & Manipulation (Transmitters, Actuators, Smart Instruments).
    Level1Devices,
    /// Level 2: Realtime Control & Automation (Virtual DCS / PLC controllers, Safety Instrumented Systems).
    Level2Control,
    /// Level 3: Operations & Supervisory (DCS Console, SCADA, Historian, Batch Execution, OTS).
    Level3Operations,
    /// Level 4: Enterprise Business Planning & Logistics (ERP, MES, Cloud Analytics).
    Level4Enterprise,
}

/// Industrial cybersecurity zone definition conforming conceptually to IEC 62443.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndustrialZoneDefinition {
    pub id: String,
    pub name: String,
    pub level: Isa95Level,
    pub permitted_protocols: Vec<ProtocolType>,
    pub trust_boundary: bool,
    pub security_policy: String,
}

impl IndustrialZoneDefinition {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        level: Isa95Level,
        permitted_protocols: Vec<ProtocolType>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            level,
            permitted_protocols,
            trust_boundary: true,
            security_policy: "DefaultRestricted".into(),
        }
    }
}

/// Conduit representing an authenticated and encrypted logical path between two zones.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndustrialConduit {
    pub id: String,
    pub name: String,
    pub source_zone_id: String,
    pub destination_zone_id: String,
    pub protocol: ProtocolType,
    pub encrypted: bool,
}

/// Server-Side Request Forgery (SSRF) defense and endpoint safety validator.
///
/// Prevents trainees or compromised client requests from using connection-test APIs
/// as arbitrary port scanners or reaching sensitive cloud metadata services.
#[derive(Debug, Clone)]
pub struct SsrfGuard {
    /// If true, permits 127.0.0.1 and localhost for local test suites and loopback adapters.
    pub allow_loopback: bool,
}

impl Default for SsrfGuard {
    fn default() -> Self {
        Self {
            allow_loopback: true, // Configurable; enabled in development/test environment
        }
    }
}

impl SsrfGuard {
    pub fn new(allow_loopback: bool) -> Self {
        Self { allow_loopback }
    }

    /// Validates an endpoint string before attempting TCP socket creation.
    pub fn validate_endpoint(&self, endpoint: &str) -> Result<(), ProtocolError> {
        let trimmed = endpoint.trim();
        if trimmed.is_empty() {
            return Err(ProtocolError::InvalidPayload("Endpoint cannot be empty".into()));
        }

        // Strip URI scheme if present (e.g. "tcp://", "opc.tcp://", "mqtt://")
        let host_port = if let Some(idx) = trimmed.find("://") {
            &trimmed[idx + 3..]
        } else {
            trimmed
        };

        let parts: Vec<&str> = host_port.split(':').collect();
        let host = parts[0];

        // 1. Cloud metadata service block (AWS / GCP / Azure 169.254.169.254)
        if host == "169.254.169.254" || host.starts_with("169.254.") {
            return Err(ProtocolError::SecurityError(
                "Access to link-local cloud metadata endpoints is strictly forbidden (SSRF defense)".into(),
            ));
        }

        // 2. Local loopback check
        if !self.allow_loopback {
            if host == "localhost" || host == "127.0.0.1" || host == "::1" || host.starts_with("127.") {
                return Err(ProtocolError::SecurityError(
                    "Access to loopback interface is restricted in production mode".into(),
                ));
            }
        }

        // 3. Port check if specified
        if parts.len() > 1 {
            let port_str = parts[1].split('/').next().unwrap_or(parts[1]);
            if let Ok(port) = port_str.parse::<u16>() {
                // Deny well-known restricted system ports (e.g. SSH 22, Telnet 23, SMTP 25, DNS 53)
                if port == 22 || port == 23 || port == 25 || port == 53 {
                    return Err(ProtocolError::SecurityError(format!(
                        "Target port {} is blacklisted for industrial protocol communication",
                        port
                    )));
                }
            }
        }

        Ok(())
    }

    /// Sanitizes sensitive fields in connection configurations before export.
    pub fn redact_password(&self, secret: &str) -> String {
        if secret.is_empty() {
            String::new()
        } else {
            "***REDACTED***".to_string()
        }
    }
}
"""

with open("crates/nebudcs-protocol/src/industrial/security.rs", "w") as f:
    f.write(security_content)
print("Wrote security.rs successfully")
