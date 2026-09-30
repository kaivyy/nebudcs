use serde::{Deserialize, Serialize};

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
            return Err(ProtocolError::InvalidPayload("Endpoint cannot be empty"));
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
                "Access to link-local cloud metadata endpoints is strictly forbidden (SSRF defense)",
            ));
        }

        // 2. Local loopback check
        if !self.allow_loopback
            && (host == "localhost"
                || host == "127.0.0.1"
                || host == "::1"
                || host.starts_with("127."))
        {
            return Err(ProtocolError::SecurityError(
                "Access to loopback interface is restricted in production mode",
            ));
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

    /// Redacts embedded username/password credentials from connection URIs.
    pub fn sanitize_credentials(uri: &str) -> String {
        if let Some(at_idx) = uri.find('@') {
            if let Some(colon_idx) = uri[..at_idx].rfind(':') {
                if let Some(scheme_idx) = uri.find("://") {
                    if colon_idx > scheme_idx + 2 {
                        let mut result = String::new();
                        result.push_str(&uri[..colon_idx + 1]);
                        result.push_str("***REDACTED***");
                        result.push_str(&uri[at_idx..]);
                        return result;
                    }
                }
            }
        }
        uri.to_string()
    }
}
