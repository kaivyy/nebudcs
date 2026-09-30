//! IEC 62443 Industrial Cybersecurity & Purdue Enterprise Reference Architecture (PERA) DTOs.
//!
//! Standards Reference:
//! - IEC 62443-3-2: Security Risk Assessment and System Design (Zones & Conduits).
//! - IEC 62443-3-3: System Security Requirements and Security Levels (SL-1 to SL-4).
//! - IEC 62443-4-2: Technical Security Requirements for IACS Components.
//! - NIST SP 800-82: Guide to Industrial Control Systems (ICS) Security.

use serde::{Deserialize, Serialize};

/// Target and achieved security levels according to IEC 62443-3-3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecurityLevel {
    /// SL-1: Protection against casual or coincidental violation.
    Sl1Coincidental,
    /// SL-2: Protection against intentional violation using simple means with low resources.
    Sl2SimpleMeans,
    /// SL-3: Protection against intentional violation using sophisticated means with moderate resources.
    Sl3Sophisticated,
    /// SL-4: Protection against intentional violation using sophisticated means with extended resources (Nation-State).
    Sl4Extended,
}

impl Default for SecurityLevel {
    fn default() -> Self {
        Self::Sl3Sophisticated
    }
}

/// Purdue Model Zone representation (Level 0 through Level 5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurdueZoneDto {
    pub zone_id: String,
    pub name: String,
    pub level_name: String,
    pub purdue_level: u8,
    pub target_security_level: String,
    pub achieved_security_level: String,
    pub assets_count: usize,
    pub status: String,
    pub trust_boundary: bool,
    pub description: String,
    pub assigned_roles: Vec<String>,
}

/// Industrial Conduit connecting two zones with protocol inspection and firewall policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurdueConduitDto {
    pub conduit_id: String,
    pub name: String,
    pub source_zone_id: String,
    pub destination_zone_id: String,
    pub allowed_protocols: Vec<String>,
    pub security_level_required: String,
    pub is_encrypted: bool,
    pub packets_inspected: u64,
    pub violations_blocked: u64,
    pub status: String,
    pub policy_description: String,
}

/// Security incident logged by the Intrusion Detection & Anomaly System.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityIncidentDto {
    pub incident_id: String,
    pub timestamp_iso: String,
    pub severity: String,
    pub category: String,
    pub source_ip: String,
    pub actor: String,
    pub description: String,
    pub mitigation_action: String,
    pub status: String,
}

/// Cryptographic verification result for tamper-evident audit logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditVerificationResultDto {
    pub is_valid: bool,
    pub total_events_checked: usize,
    pub root_hash: String,
    pub latest_hash: String,
    pub tampering_detected: bool,
    pub first_invalid_event_id: Option<String>,
    pub verification_timestamp_iso: String,
    pub verification_status: String,
}

/// Overall system security posture and IEC 62443 compliance metrics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityStatusDto {
    pub system_security_level: String,
    pub active_threat_level: String,
    pub total_incidents: usize,
    pub active_incidents: usize,
    pub audit_chain_valid: bool,
    pub audit_events_count: usize,
    pub root_hash: String,
    pub mtls_enabled: bool,
    pub rbac_enforced: bool,
    pub zones_count: usize,
    pub conduits_count: usize,
    pub last_audit_verification_iso: Option<String>,
}

/// Request to update or configure a conduit security policy rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateConduitPolicyRequestDto {
    pub conduit_id: String,
    pub allowed_protocols: Vec<String>,
    pub is_encrypted: bool,
    pub status: String,
}
