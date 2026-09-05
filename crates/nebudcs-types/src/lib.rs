//! Core strongly-typed primitives for NebuDCS.
//!
//! This crate has **zero** workspace crate dependencies.  It provides the
//! foundational types that every other NebuDCS crate builds upon:
//!
//! - [`ids`] — strongly-typed domain identifiers.
//! - [`time`] — deterministic [`SimulationTime`].
//! - [`version`] — [`SchemaVersion`] for project-file compatibility.
//! - [`error`] — [`NebuError`] and [`NebuResult`].
//!
//! # Architectural invariant
//!
//! This crate must remain dependency-free with respect to other NebuDCS
//! crates.  Adding a workspace dependency here creates a potential for
//! circular dependencies and must be rejected.

pub mod error;
pub mod ids;
pub mod time;
pub mod version;

// Re-export the most commonly used items at the crate root for ergonomics.
pub use error::{NebuError, NebuResult};
pub use ids::{
    AlarmDefinitionId, AreaId, EntityId, EquipmentDefinitionId, EquipmentId, FailureModeId,
    PipeSegmentId, PlantId, PortId, ProjectId, SignalConnectionId, StreamId, TagId, UnitId,
};
pub use time::SimulationTime;
pub use version::SchemaVersion;
