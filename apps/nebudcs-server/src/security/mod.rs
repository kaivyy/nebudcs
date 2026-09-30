//! Security hardening subsystem: RBAC, constant-time verification, rate limiting, and audit logging.

pub mod ids;
pub mod manager;
pub mod purdue;
pub mod rate_limiter;
pub mod roles;

pub use ids::IntrusionDetectionSystem;
pub use manager::CyberSecurityManager;
pub use purdue::PurdueSecurityEngine;
pub use rate_limiter::RateLimiter;
pub use roles::{constant_time_eq, sanitize_identifier, UserRole};

/// Structured security audit event logger.
pub fn audit_log(event_type: &str, actor: &str, action: &str, details: &str, permitted: bool) {
    if permitted {
        tracing::info!(
            target: "security_audit",
            event = event_type,
            actor = actor,
            action = action,
            details = details,
            permitted = true,
            "Security Event: {action} permitted for {actor}"
        );
    } else {
        tracing::warn!(
            target: "security_audit",
            event = event_type,
            actor = actor,
            action = action,
            details = details,
            permitted = false,
            "SECURITY ALERT: {action} DENIED for {actor} ({details})"
        );
    }
}
