//! Industrial Intrusion Detection & Anomaly Monitoring System (IDS).
//!
//! Standards Reference: IEC 62443-3-3 System Security Requirements,
//! NIST SP 800-82 ICS Incident Response, ISA-99 Anomaly Detection.

use nebudcs_protocol::SecurityIncidentDto;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_INCIDENTS_CAPACITY: usize = 1000;

/// Intrusion Detection System recording security anomalies and computing threat levels.
#[derive(Debug)]
pub struct IntrusionDetectionSystem {
    incidents: VecDeque<SecurityIncidentDto>,
    counter: AtomicU64,
}

impl Default for IntrusionDetectionSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl IntrusionDetectionSystem {
    pub fn new() -> Self {
        let mut ids = Self {
            incidents: VecDeque::with_capacity(MAX_INCIDENTS_CAPACITY),
            counter: AtomicU64::new(0),
        };

        // Seed with standard operational security baseline initialization
        ids.record_incident(
            "INFO",
            "BASELINE_INITIALIZED",
            "127.0.0.1",
            "System",
            "IEC 62443 Security Perimeter & Intrusion Detection System activated",
            "POLICY_ENFORCED",
        );

        ids
    }

    /// Records a security incident into the audit ring buffer.
    pub fn record_incident(
        &mut self,
        severity: &str,
        category: &str,
        source_ip: &str,
        actor: &str,
        description: &str,
        mitigation: &str,
    ) -> SecurityIncidentDto {
        let count = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
        let incident_id = format!("SEC-INC-{count:05}");
        let timestamp_iso = chrono_now_iso();

        let incident = SecurityIncidentDto {
            incident_id,
            timestamp_iso,
            severity: severity.to_string(),
            category: category.to_string(),
            source_ip: source_ip.to_string(),
            actor: actor.to_string(),
            description: description.to_string(),
            mitigation_action: mitigation.to_string(),
            status: "ACTIVE".to_string(),
        };

        if self.incidents.len() >= MAX_INCIDENTS_CAPACITY {
            self.incidents.pop_front();
        }
        self.incidents.push_back(incident.clone());
        incident
    }

    /// Detects and logs unauthorized privilege escalation attempt.
    pub fn detect_privilege_escalation(
        &mut self,
        actor: &str,
        role: &str,
        attempted_action: &str,
    ) {
        let desc = format!(
            "Role privilege violation: user '{actor}' with role '{role}' attempted unauthorized action '{attempted_action}'"
        );
        self.record_incident(
            "HIGH",
            "ROLE_ESCALATION",
            "0.0.0.0",
            actor,
            &desc,
            "ACTION_DENIED_AND_LOGGED",
        );
    }

    /// Detects directory traversal or path manipulation attack.
    pub fn detect_path_traversal(&mut self, actor: &str, raw_path: &str) {
        let desc = format!(
            "Path traversal attack detected: actor '{actor}' supplied malicious path pattern '{raw_path}'"
        );
        self.record_incident(
            "CRITICAL",
            "PATH_TRAVERSAL",
            "0.0.0.0",
            actor,
            &desc,
            "REQUEST_DROPPED_AND_QUARANTINED",
        );
    }

    /// Detects rate limit / potential denial-of-service attack.
    pub fn detect_rate_limit_violation(&mut self, ip: &str, req_count: usize) {
        let desc = format!(
            "Rate limit threshold exceeded: {req_count} requests in time window from IP '{ip}'"
        );
        self.record_incident(
            "MEDIUM",
            "RATE_LIMIT_EXCEEDED",
            ip,
            "Unknown",
            &desc,
            "HTTP_429_THROTTLED",
        );
    }

    /// Detects conduit access rule violation between Purdue zones.
    pub fn detect_conduit_violation(&mut self, conduit_id: &str, reason: &str) {
        let desc = format!("Conduit security policy violation on '{conduit_id}': {reason}");
        self.record_incident(
            "HIGH",
            "CONDUIT_VIOLATION",
            "InternalBus",
            "NetworkLayer",
            &desc,
            "PACKET_DROPPED",
        );
    }

    /// Computes the overall industrial threat level based on active incidents.
    pub fn compute_threat_level(&self) -> String {
        let active_critical = self
            .incidents
            .iter()
            .filter(|i| i.status == "ACTIVE" && i.severity == "CRITICAL")
            .count();

        let active_high = self
            .incidents
            .iter()
            .filter(|i| i.status == "ACTIVE" && i.severity == "HIGH")
            .count();

        let active_total = self
            .incidents
            .iter()
            .filter(|i| i.status == "ACTIVE")
            .count();

        if active_critical > 0 {
            "CRITICAL".to_string()
        } else if active_high >= 2 {
            "HIGH".to_string()
        } else if active_total > 4 {
            "ELEVATED".to_string()
        } else {
            "LOW".to_string()
        }
    }

    /// Retrieves all incidents sorted reverse-chronologically.
    pub fn get_incidents(&self, limit: usize) -> Vec<SecurityIncidentDto> {
        let cap = if limit == 0 || limit > 500 { 100 } else { limit };
        self.incidents.iter().rev().take(cap).cloned().collect()
    }

    pub fn total_incidents_count(&self) -> usize {
        self.incidents.len()
    }

    pub fn active_incidents_count(&self) -> usize {
        self.incidents.iter().filter(|i| i.status == "ACTIVE").count()
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
