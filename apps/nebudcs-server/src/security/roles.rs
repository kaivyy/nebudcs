//! Server-side Role-Based Access Control (RBAC), constant-time token comparison,
//! and path traversal defense.

use crate::error::{ServerError, ServerResult};
use std::fmt;

/// Roles defining permission levels across NebuDCS server operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UserRole {
    /// Lowest privilege: Operator controls only (SP, Mode, Out, Ack, Reset Trip).
    /// Strictly prohibited from administrative, topology, scenario editing, and fault actions.
    Trainee,
    /// Standard DCS operator: Full operator console control, alarm shelving.
    Operator,
    /// Training instructor: Can author scenarios, orchestrate sessions, and inject faults.
    Instructor,
    /// Process/control engineer: Can design topology, configure equipment, and tune PID loops.
    Engineer,
    /// Full platform administrator: Backup, restore, migration, shutdown, user management.
    Admin,
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Trainee => write!(f, "Trainee"),
            Self::Operator => write!(f, "Operator"),
            Self::Instructor => write!(f, "Instructor"),
            Self::Engineer => write!(f, "Engineer"),
            Self::Admin => write!(f, "Admin"),
        }
    }
}

impl UserRole {
    /// Parses role string case-insensitively.
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.trim().to_uppercase().as_str() {
            "TRAINEE" => Some(Self::Trainee),
            "OPERATOR" | "OPER" => Some(Self::Operator),
            "INSTRUCTOR" | "INST" => Some(Self::Instructor),
            "ENGINEER" | "ENG" => Some(Self::Engineer),
            "ADMIN" | "ADMINISTRATOR" | "ROOT" => Some(Self::Admin),
            _ => None,
        }
    }

    /// Whether this role can perform platform administration (backup, restore, migration, server settings).
    pub fn can_administer(&self) -> bool {
        matches!(self, Self::Admin)
    }

    /// Whether this role can edit engineering topology and custom equipment models.
    pub fn can_modify_topology(&self) -> bool {
        matches!(self, Self::Admin | Self::Engineer)
    }

    /// Whether this role can launch/control training curricula and inject dynamic process faults.
    pub fn can_instruct(&self) -> bool {
        matches!(self, Self::Admin | Self::Instructor)
    }

    /// Whether this role can start, pause, step, or reset the core simulation clock.
    pub fn can_control_simulation(&self) -> bool {
        matches!(self, Self::Admin | Self::Engineer | Self::Instructor)
    }

    /// Whether this role can operate standard DCS controls (SP, Mode, Manual Output, Ack Alarm).
    pub fn can_operate_dcs(&self) -> bool {
        // All roles, including Trainee, are permitted to operate DCS console
        true
    }

    /// Whether this role can run scientific validation benchmarks.
    pub fn can_run_validation(&self) -> bool {
        matches!(self, Self::Admin | Self::Engineer | Self::Instructor | Self::Operator)
    }

    /// Whether this role can register custom benchmark cases.
    pub fn can_register_benchmarks(&self) -> bool {
        matches!(self, Self::Admin | Self::Engineer | Self::Instructor)
    }
}

/// Constant-time byte slice comparison to eliminate timing side-channel attacks on secret tokens.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Sanitizes an identifier parameter (project ID, scenario ID, template ID, filename)
/// against directory traversal attacks (`..`, `/`, `\`, null bytes, control characters).
pub fn sanitize_identifier(id: &str) -> ServerResult<String> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return Err(ServerError::InvalidRequest(
            "Identifier cannot be empty".to_string(),
        ));
    }
    if trimmed.len() > 128 {
        return Err(ServerError::InvalidRequest(
            "Identifier exceeds maximum length of 128 characters".to_string(),
        ));
    }

    // Check for traversal characters and null bytes
    if trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains('\0')
    {
        return Err(ServerError::Forbidden(format!(
            "Path traversal attack detected in identifier: '{id}'"
        )));
    }

    // Ensure identifier contains only safe alphanumeric, dash, underscore, and dot characters
    for c in trimmed.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.' {
            return Err(ServerError::InvalidRequest(format!(
                "Identifier contains invalid character '{c}'. Only alphanumeric, '-', '_', and '.' are allowed."
            )));
        }
    }

    Ok(trimmed.to_string())
}
