//! Strongly-typed domain identifiers for NebuDCS.
//!
//! # Design
//!
//! Every entity in NebuDCS has a distinct identifier type so that the
//! compiler prevents accidental confusion between, e.g., a [`TagId`] and an
//! [`EquipmentId`].  All identifiers wrap an inner [`ulid::Ulid`] which
//! is:
//!
//! - 128-bit, globally unique without coordination.
//! - Lexicographically sortable by creation time.
//! - Compact when serialized (26-character Crockford Base-32 string).
//!
//! # Invariant
//!
//! Two distinct newtype wrappers around `Ulid` are **not** interchangeable.
//! There is intentionally **no** `From<ProjectId> for PlantId` or similar
//! conversion.  This is enforced by the type system, not by documentation.

use serde::{Deserialize, Serialize};
use ulid::Ulid;

// ---------------------------------------------------------------------------
// Internal macro — generates a typed ID newtype.
// Each newtype is:
//   - A transparent wrapper over `Ulid`.
//   - Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize.
//   - Able to generate a new random value via `::new()`.
//   - Able to expose its inner Ulid via `.inner()`.
//   - Printable (Display delegates to Ulid's Display).
// ---------------------------------------------------------------------------
macro_rules! typed_id {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Ulid);

        impl $name {
            /// Generate a new, globally unique ID.
            #[must_use]
            pub fn new() -> Self {
                Self(Ulid::new())
            }

            /// Access the underlying [`Ulid`].
            #[must_use]
            #[inline]
            pub fn inner(self) -> Ulid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = ulid::DecodeError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                s.parse::<Ulid>().map($name)
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Domain ID types
// ---------------------------------------------------------------------------

/// The generic underlying identity carrier — used when a generic ID is
/// needed (e.g., `EntityRef` in validation results).  Domain-specific code
/// should always prefer the concrete typed IDs below.
///
/// `EntityId` is **not** interchangeable with the specific domain IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityId(Ulid);

impl EntityId {
    /// Generate a new unique entity ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Access the underlying [`Ulid`].
    #[must_use]
    #[inline]
    pub fn inner(self) -> Ulid {
        self.0
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

typed_id!(
    /// Identifies a top-level [`Project`](crate) — the root of the plant hierarchy.
    ProjectId
);

typed_id!(
    /// Identifies a [`Plant`](crate) within a project.
    PlantId
);

typed_id!(
    /// Identifies an [`Area`](crate) within a plant.
    AreaId
);

typed_id!(
    /// Identifies a process [`Unit`](crate) within an area.
    UnitId
);

typed_id!(
    /// Identifies a piece of process equipment.
    ///
    /// Equipment instances carry this ID; equipment *definitions* carry an
    /// [`EquipmentDefinitionId`].
    EquipmentId
);

typed_id!(
    /// Identifies an [`EquipmentDefinition`](crate) — the versioned, immutable
    /// template from which equipment instances are created.
    EquipmentDefinitionId
);

typed_id!(
    /// Identifies a process stream (material or utility).
    StreamId
);

typed_id!(
    /// Identifies a tag — a named, typed process variable.
    TagId
);

typed_id!(
    /// Identifies a port on an equipment definition.
    PortId
);

typed_id!(
    /// Identifies an alarm definition within an equipment definition.
    AlarmDefinitionId
);

typed_id!(
    /// Identifies a failure mode definition within an equipment definition.
    FailureModeId
);

typed_id!(
    /// Identifies a physical pipe or pipe segment in the piping topology.
    PipeSegmentId
);

typed_id!(
    /// Identifies an instrument signal connection.
    SignalConnectionId
);

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_ids_are_unique() {
        // Two freshly generated IDs must never collide.
        let a = EntityId::new();
        let b = EntityId::new();
        assert_ne!(a, b, "EntityId collision detected — ULID broken?");
    }

    #[test]
    fn test_project_ids_are_unique() {
        let a = ProjectId::new();
        let b = ProjectId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn test_tag_id_unique() {
        let a = TagId::new();
        let b = TagId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn test_pipe_segment_id_unique() {
        let a = PipeSegmentId::new();
        let b = PipeSegmentId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn test_signal_connection_id_unique() {
        let a = SignalConnectionId::new();
        let b = SignalConnectionId::new();
        assert_ne!(a, b);
    }

    /// Verify that EquipmentId and StreamId serialise to different JSON strings
    /// (they do — different ULIDs) and that the type system separates them.
    /// A compile-time check would be ideal; this test documents the intent.
    #[test]
    fn test_equipment_id_not_assignable_to_stream_id() {
        // This is a TYPE-LEVEL invariant: the following must NOT compile:
        //   let eid: EquipmentId = EquipmentId::new();
        //   let sid: StreamId = eid;  // compile error expected
        //
        // We cannot express a "must not compile" assertion inside #[test],
        // but we can verify the types have different type names via std::any.
        let eq_type = std::any::type_name::<EquipmentId>();
        let st_type = std::any::type_name::<StreamId>();
        assert_ne!(
            eq_type, st_type,
            "EquipmentId and StreamId should be distinct types"
        );

        // Also verify Tag ≠ Equipment ≠ Port ≠ Stream ≠ PipeSegment ≠ SignalConnection.
        let tag_type = std::any::type_name::<TagId>();
        let port_type = std::any::type_name::<PortId>();
        let pipe_type = std::any::type_name::<PipeSegmentId>();
        let sig_type = std::any::type_name::<SignalConnectionId>();
        let names = [eq_type, st_type, tag_type, port_type, pipe_type, sig_type];
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), names.len(), "ID types must all be distinct");
    }

    #[test]
    fn test_id_display_is_non_empty() {
        let id = TagId::new();
        let s = id.to_string();
        assert!(!s.is_empty(), "TagId Display must not be empty");
        assert_eq!(s.len(), 26, "ULID string should be 26 characters");
    }

    #[test]
    fn test_id_serde_round_trip() {
        let original = EquipmentId::new();
        let json = serde_json::to_string(&original).expect("serialize");
        let decoded: EquipmentId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_entity_id_from_str() {
        let id = TagId::new();
        let s = id.to_string();
        let parsed: TagId = s.parse().expect("parse ULID string");
        assert_eq!(id, parsed);
    }
}
