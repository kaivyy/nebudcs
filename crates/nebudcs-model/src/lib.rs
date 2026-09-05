//! Project hierarchy DTOs, EquipmentDefinition schema, and domain model for NebuDCS.
//!
//! # Crate dependencies
//!
//! - `nebudcs-types` — IDs, time, errors
//! - `nebudcs-units` — physical quantities and display units
//!
//! This crate contains **no behaviour** — it is pure data/schema.  The
//! simulation engine (`nebudcs-sim`, Phase 3) holds runtime state.

pub mod equipment;
pub mod project;
pub mod stream;
pub mod tag;
pub mod validation;

// Commonly used re-exports.
pub use equipment::{
    AlarmDefinition, AlarmPriority, AlarmType, Capability, EquipmentCategory, EquipmentDefinition,
    EquipmentInstance, EquipmentMetadata, FailureBehavior, FailureModeDefinition,
    OperatingEnvelope, ParameterDefinition, Phase, PortDefinition, PortDirection, PortDomain,
    Position2D, TagDefinition, TagSource, UtilityKind,
};
pub use project::{Area, Plant, Project, ProjectMetadata, Unit};
pub use stream::{PortRef, StreamComposition, StreamDefinition, StreamType};
pub use tag::{TagQuality, TagSnapshot, TagValue};
pub use validation::{ValidationResult, ValidationSeverity};
