//! Top-level error type for NebuDCS.
//!
//! [`NebuError`] is the single error type exported by `nebudcs-types`.
//! Every variant carries a meaningful message.  Higher-level crates may
//! define their own error types that convert *into* `NebuError` or wrap it.
//!
//! # Design
//!
//! - No generic "Other" catch-all variants.
//! - Every variant is named for its domain.
//! - `thiserror` generates `Display` and `std::error::Error` impls.

use thiserror::Error;

/// The canonical result type for the NebuDCS type layer.
pub type NebuResult<T> = Result<T, NebuError>;

/// All errors that can originate from the `nebudcs-types` layer.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum NebuError {
    /// A [`SimulationTime`](crate::time::SimulationTime) was constructed with
    /// an invalid (negative or NaN) value.
    #[error("invalid simulation time {value}: {reason}")]
    InvalidSimulationTime { value: f64, reason: String },

    /// A [`SchemaVersion`](crate::version::SchemaVersion) string could not be
    /// parsed.
    #[error("invalid schema version '{input}': expected 'MAJOR.MINOR.PATCH'")]
    InvalidSchemaVersion { input: String },

    /// A required field was absent in a serialised document.
    #[error("missing required field '{field}' in '{context}'")]
    MissingField { field: String, context: String },

    /// A value was outside the valid range for a quantity.
    #[error("value {value} is out of range [{min}, {max}] for '{field}'")]
    OutOfRange {
        field: String,
        value: f64,
        min: f64,
        max: f64,
    },

    /// Serialization/deserialization failed.
    #[error("serialisation error in '{context}': {detail}")]
    Serialization { context: String, detail: String },

    /// An ID reference could not be resolved.
    #[error("unresolved reference: '{id_type}' with id '{id_value}' was not found")]
    UnresolvedReference { id_type: String, id_value: String },

    /// A topology connection was semantically invalid (e.g., incompatible
    /// port domains, mismatched phases).
    #[error("invalid connection: {detail}")]
    InvalidConnection { detail: String },

    /// A project validation rule produced an error-severity finding.
    #[error("validation error [{code}]: {message}")]
    ValidationError { code: String, message: String },

    /// An operation was attempted in an invalid state.
    #[error("invalid state for operation '{operation}': {detail}")]
    InvalidState { operation: String, detail: String },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    /// Every NebuError variant must produce a non-empty Display string.
    #[test]
    fn test_error_display_non_empty() {
        let variants: Vec<NebuError> = vec![
            NebuError::InvalidSimulationTime {
                value: -1.0,
                reason: "negative".to_owned(),
            },
            NebuError::InvalidSchemaVersion {
                input: "bad".to_owned(),
            },
            NebuError::MissingField {
                field: "name".to_owned(),
                context: "EquipmentDefinition".to_owned(),
            },
            NebuError::OutOfRange {
                field: "rated_flow".to_owned(),
                value: -5.0,
                min: 0.0,
                max: 100.0,
            },
            NebuError::Serialization {
                context: "pump.toml".to_owned(),
                detail: "unexpected token".to_owned(),
            },
            NebuError::UnresolvedReference {
                id_type: "EquipmentDefinitionId".to_owned(),
                id_value: "01ABC".to_owned(),
            },
            NebuError::InvalidConnection {
                detail: "material port cannot connect to electrical port".to_owned(),
            },
            NebuError::ValidationError {
                code: "VLD-001".to_owned(),
                message: "vessel has no relief protection".to_owned(),
            },
            NebuError::InvalidState {
                operation: "start".to_owned(),
                detail: "equipment is tripped".to_owned(),
            },
        ];

        for variant in &variants {
            let display = variant.to_string();
            assert!(
                !display.is_empty(),
                "NebuError::{variant:?} Display is empty"
            );
            // Every message must be human-readable (>5 chars is a loose proxy).
            assert!(
                display.len() > 5,
                "NebuError display too short: '{display}'"
            );
        }
    }

    #[test]
    fn test_error_is_clone_and_eq() {
        let e = NebuError::MissingField {
            field: "id".to_owned(),
            context: "Project".to_owned(),
        };
        let cloned = e.clone();
        assert_eq!(e, cloned);
    }

    #[test]
    fn test_nebu_result_ok() {
        let r: NebuResult<u32> = Ok(42);
        assert!(matches!(r, Ok(42)));
    }

    #[test]
    fn test_nebu_result_err() {
        let r: NebuResult<u32> = Err(NebuError::InvalidState {
            operation: "tick".to_owned(),
            detail: "not initialised".to_owned(),
        });
        assert!(r.is_err());
    }
}
