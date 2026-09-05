//! Project validation results.
//!
//! Validation is performed at design time (project load, save, and explicit
//! "validate" actions) — not inside the simulation tick loop.
//!
//! Results are collected in a `Vec<ValidationResult>` and presented to the
//! engineer.  An `Error`-severity result must be resolved before simulation
//! can begin.

use serde::{Deserialize, Serialize};

use nebudcs_types::EntityId;

/// The severity level of a validation finding.
///
/// Ordered from most severe to least.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationSeverity {
    /// Simulation cannot proceed until this is resolved.
    Error = 3,
    /// Simulation can proceed but a potential problem was identified.
    Warning = 2,
    /// Informational note for the engineer.
    Info = 1,
    /// Requires engineering review but is not necessarily wrong.
    EngineeringReview = 0,
}

/// A single validation finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationResult {
    pub severity: ValidationSeverity,
    /// A machine-readable code for filtering (e.g., `"VLD-001"`).
    pub code: String,
    /// A human-readable explanation.
    pub message: String,
    /// The entity this finding relates to, if applicable.
    pub entity: Option<EntityId>,
}

impl ValidationResult {
    pub fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        entity: Option<EntityId>,
    ) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            code: code.into(),
            message: message.into(),
            entity,
        }
    }

    pub fn warning(
        code: impl Into<String>,
        message: impl Into<String>,
        entity: Option<EntityId>,
    ) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            code: code.into(),
            message: message.into(),
            entity,
        }
    }

    pub fn info(
        code: impl Into<String>,
        message: impl Into<String>,
        entity: Option<EntityId>,
    ) -> Self {
        Self {
            severity: ValidationSeverity::Info,
            code: code.into(),
            message: message.into(),
            entity,
        }
    }

    /// Returns `true` if this finding is blocking (severity == Error).
    pub fn is_blocking(&self) -> bool {
        self.severity == ValidationSeverity::Error
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_severity_ordering() {
        // Error > Warning > Info > EngineeringReview
        assert!(ValidationSeverity::Error > ValidationSeverity::Warning);
        assert!(ValidationSeverity::Warning > ValidationSeverity::Info);
        assert!(ValidationSeverity::Info > ValidationSeverity::EngineeringReview);
    }

    #[test]
    fn test_validation_error_is_blocking() {
        let r = ValidationResult::error("VLD-001", "missing relief valve", None);
        assert!(r.is_blocking());
    }

    #[test]
    fn test_validation_warning_not_blocking() {
        let r = ValidationResult::warning("VLD-002", "efficiency curve not specified", None);
        assert!(!r.is_blocking());
    }

    #[test]
    fn test_validation_result_serde() {
        let r = ValidationResult::error("VLD-003", "unconnected required port", None);
        let json = serde_json::to_string(&r).expect("serialize");
        let decoded: ValidationResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r.code, decoded.code);
        assert_eq!(r.severity, decoded.severity);
    }

    #[test]
    fn test_validation_result_with_entity() {
        let entity = EntityId::new();
        let r = ValidationResult::error("VLD-004", "port disconnected", Some(entity));
        assert_eq!(r.entity, Some(entity));
    }
}
