//! Unified Platform Audit Logging Engine.
//!
//! Standards Reference: IEC 62443 Security Logging, FDA 21 CFR Part 11 Audit Trail,
//! ISA-18.2 Operator Action Tracking.

use nebudcs_protocol::{AuditEventDto, AuditVerificationResultDto};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_AUDIT_LOG_CAPACITY: usize = 5000;
const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn compute_event_hash(
    prev_hash: &str,
    event_id: &str,
    timestamp: &str,
    actor: &str,
    role: &str,
    source: &str,
    target: &str,
    action: &str,
    status: &str,
    prev_val: Option<&str>,
    new_val: Option<&str>,
) -> String {
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        prev_hash,
        event_id,
        timestamp,
        actor,
        role,
        source,
        target,
        action,
        status,
        prev_val.unwrap_or(""),
        new_val.unwrap_or("")
    );
    crate::backup::sha256::sha256_hex(payload.as_bytes())
}

/// Summary statistics for platform audit trail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditSummaryDto {
    pub total_events_recorded: u64,
    pub current_buffer_size: usize,
    pub unique_actors_count: usize,
    pub unique_actions_count: usize,
    pub latest_event_timestamp: Option<String>,
}

/// Unified Audit Manager managing tamper-evident operational logs with cryptographic hash chaining.
#[derive(Debug)]
pub struct AuditManager {
    buffer: VecDeque<AuditEventDto>,
    total_counter: AtomicU64,
    latest_hash: String,
}

impl Default for AuditManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            buffer: VecDeque::with_capacity(MAX_AUDIT_LOG_CAPACITY),
            total_counter: AtomicU64::new(0),
            latest_hash: GENESIS_HASH.to_string(),
        };
        // Seed with initial system startup event
        mgr.record(
            "SystemStartup",
            "System",
            "Admin",
            "NebuDCS-Server",
            "ClusterCore",
            "INITIALIZE",
            None,
            Some("STAGE_15_IEC62443_SECURE".to_string()),
            "SUCCESS",
            Some("Authoritative server instance initialized with IEC 62443 security and tamper-evident audit logging".to_string()),
            None,
            0.0,
        );
        mgr
    }

    pub fn get_latest_hash(&self) -> String {
        self.latest_hash.clone()
    }

    /// Records an audit event into the ring buffer with SHA-256 hash chaining.
    #[allow(clippy::too_many_arguments)]
    pub fn record(
        &mut self,
        event_id_prefix: &str,
        actor: &str,
        role: &str,
        source: &str,
        target: &str,
        action: &str,
        prev_val: Option<String>,
        new_val: Option<String>,
        status: &str,
        reason: Option<String>,
        correlation_id: Option<String>,
        sim_time_secs: f64,
    ) {
        let count = self.total_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let event_id = format!("{event_id_prefix}-{count:06}");
        let timestamp_iso = chrono_now_iso();

        let prev_hash = self.latest_hash.clone();
        let ev_hash = compute_event_hash(
            &prev_hash,
            &event_id,
            &timestamp_iso,
            actor,
            role,
            source,
            target,
            action,
            status,
            prev_val.as_deref(),
            new_val.as_deref(),
        );
        self.latest_hash = ev_hash.clone();

        let event = AuditEventDto {
            event_id,
            timestamp_iso,
            sim_time_secs,
            actor: actor.to_string(),
            role: role.to_string(),
            source: source.to_string(),
            target: target.to_string(),
            action: action.to_string(),
            prev_value: prev_val,
            new_value: new_val,
            status: status.to_string(),
            reason,
            correlation_id,
            previous_hash: Some(prev_hash),
            event_hash: Some(ev_hash),
        };

        if self.buffer.len() >= MAX_AUDIT_LOG_CAPACITY {
            self.buffer.pop_front();
        }
        self.buffer.push_back(event);
    }

    /// Appends a pre-formed AuditEventDto with SHA-256 hash chaining.
    pub fn record_event(&mut self, mut event: AuditEventDto) {
        let count = self.total_counter.fetch_add(1, Ordering::SeqCst) + 1;
        if event.event_id.is_empty() {
            event.event_id = format!("AUDIT-{count:06}");
        }

        let prev_hash = self.latest_hash.clone();
        let ev_hash = compute_event_hash(
            &prev_hash,
            &event.event_id,
            &event.timestamp_iso,
            &event.actor,
            &event.role,
            &event.source,
            &event.target,
            &event.action,
            &event.status,
            event.prev_value.as_deref(),
            event.new_value.as_deref(),
        );
        self.latest_hash = ev_hash.clone();
        event.previous_hash = Some(prev_hash);
        event.event_hash = Some(ev_hash);

        if self.buffer.len() >= MAX_AUDIT_LOG_CAPACITY {
            self.buffer.pop_front();
        }
        self.buffer.push_back(event);
    }

    /// Injects a raw unverified event into the buffer, simulating direct database corruption
    /// or unauthorized external modification.
    pub fn inject_raw_event_unverified(&mut self, event: AuditEventDto) {
        if self.buffer.len() >= MAX_AUDIT_LOG_CAPACITY {
            self.buffer.pop_front();
        }
        self.buffer.push_back(event);
    }

    /// Tamper with the last recorded event in the buffer without recalculating its hash.
    /// Used for cryptographic tamper-detection testing and forensics verification.
    pub fn tamper_last_event_for_test(&mut self, new_action: &str) {
        if let Some(last) = self.buffer.back_mut() {
            last.action = new_action.to_string();
        }
    }

    /// Verifies the unbroken cryptographic integrity of the audit log hash chain.
    pub fn verify_integrity(&self) -> AuditVerificationResultDto {
        let mut is_valid = true;
        let mut tampering_detected = false;
        let mut first_invalid_id = None;

        if self.buffer.is_empty() {
            return AuditVerificationResultDto {
                is_valid: true,
                total_events_checked: 0,
                root_hash: GENESIS_HASH.to_string(),
                latest_hash: self.latest_hash.clone(),
                tampering_detected: false,
                first_invalid_event_id: None,
                verification_timestamp_iso: chrono_now_iso(),
                verification_status: "CRYPTOGRAPHICALLY_VERIFIED (EMPTY)".to_string(),
            };
        }

        let root_hash = self
            .buffer
            .front()
            .and_then(|e| e.previous_hash.clone())
            .unwrap_or_else(|| GENESIS_HASH.to_string());
        let mut current_expected_prev = root_hash.clone();

        for ev in &self.buffer {
            if let Some(prev) = &ev.previous_hash {
                if prev != &current_expected_prev {
                    is_valid = false;
                    tampering_detected = true;
                    first_invalid_id = Some(ev.event_id.clone());
                    break;
                }
            } else {
                is_valid = false;
                tampering_detected = true;
                first_invalid_id = Some(ev.event_id.clone());
                break;
            }

            let calculated_hash = compute_event_hash(
                &current_expected_prev,
                &ev.event_id,
                &ev.timestamp_iso,
                &ev.actor,
                &ev.role,
                &ev.source,
                &ev.target,
                &ev.action,
                &ev.status,
                ev.prev_value.as_deref(),
                ev.new_value.as_deref(),
            );

            if let Some(ev_h) = &ev.event_hash {
                if ev_h != &calculated_hash {
                    is_valid = false;
                    tampering_detected = true;
                    first_invalid_id = Some(ev.event_id.clone());
                    break;
                }
            } else {
                is_valid = false;
                tampering_detected = true;
                first_invalid_id = Some(ev.event_id.clone());
                break;
            }

            current_expected_prev = calculated_hash;
        }

        let status_str = if is_valid {
            "CRYPTOGRAPHICALLY_VERIFIED (UNBROKEN_CHAIN)".to_string()
        } else {
            "INTEGRITY_COMPROMISED (HASH_MISMATCH)".to_string()
        };

        AuditVerificationResultDto {
            is_valid,
            total_events_checked: self.buffer.len(),
            root_hash,
            latest_hash: self.latest_hash.clone(),
            tampering_detected,
            first_invalid_event_id: first_invalid_id,
            verification_timestamp_iso: chrono_now_iso(),
            verification_status: status_str,
        }
    }

    /// Queries audit events with flexible filtering and reverse-chronological ordering.
    pub fn query(
        &self,
        actor: Option<&str>,
        role: Option<&str>,
        action: Option<&str>,
        source: Option<&str>,
        limit: usize,
    ) -> Vec<AuditEventDto> {
        let max_limit = if limit == 0 || limit > 500 { 100 } else { limit };

        self.buffer
            .iter()
            .rev()
            .filter(|ev| {
                if let Some(act) = actor {
                    if !ev.actor.eq_ignore_ascii_case(act) {
                        return false;
                    }
                }
                if let Some(r) = role {
                    if !ev.role.eq_ignore_ascii_case(r) {
                        return false;
                    }
                }
                if let Some(ac) = action {
                    if !ev.action.eq_ignore_ascii_case(ac) {
                        return false;
                    }
                }
                if let Some(src) = source {
                    if !ev.source.eq_ignore_ascii_case(src) {
                        return false;
                    }
                }
                true
            })
            .take(max_limit)
            .cloned()
            .collect()
    }

    /// Produces an audit trail summary.
    pub fn get_summary(&self) -> AuditSummaryDto {
        let total = self.total_counter.load(Ordering::SeqCst);
        let current_buf = self.buffer.len();

        let mut actors = std::collections::HashSet::new();
        let mut actions = std::collections::HashSet::new();
        for ev in &self.buffer {
            actors.insert(&ev.actor);
            actions.insert(&ev.action);
        }

        let latest_ts = self.buffer.back().map(|e| e.timestamp_iso.clone());

        AuditSummaryDto {
            total_events_recorded: total,
            current_buffer_size: current_buf,
            unique_actors_count: actors.len(),
            unique_actions_count: actions.len(),
            latest_event_timestamp: latest_ts,
        }
    }
}

fn chrono_now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    format!("2026-09-08T{h:02}:{m:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_logging_and_query() {
        let mut audit = AuditManager::new();

        audit.record(
            "SP-CHG",
            "operator_01",
            "Operator",
            "DCS_Faceplate",
            "FIC-101",
            "SETPOINT_CHANGE",
            Some("50.0".to_string()),
            Some("65.0".to_string()),
            "SUCCESS",
            Some("Ramping feed rate".to_string()),
            Some("tx-1234".to_string()),
            42.0,
        );

        audit.record(
            "SIS-TRIP",
            "SafetyRuntime",
            "System",
            "SIF-201",
            "ESD_VALVE_XV01",
            "EMERGENCY_SHUTDOWN",
            Some("ARMED".to_string()),
            Some("TRIPPED".to_string()),
            "SUCCESS",
            Some("High high pressure interlock triggered".to_string()),
            None,
            45.5,
        );

        let summary = audit.get_summary();
        assert_eq!(summary.total_events_recorded, 3); // 1 startup + 2 records

        let operator_events = audit.query(Some("operator_01"), None, None, None, 10);
        assert_eq!(operator_events.len(), 1);
        assert_eq!(operator_events[0].action, "SETPOINT_CHANGE");

        let sis_events = audit.query(None, None, Some("EMERGENCY_SHUTDOWN"), None, 10);
        assert_eq!(sis_events.len(), 1);
        assert_eq!(sis_events[0].target, "ESD_VALVE_XV01");
    }
}
