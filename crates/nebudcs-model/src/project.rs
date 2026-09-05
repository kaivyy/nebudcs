//! Project hierarchy data transfer objects.
//!
//! `Project` is the root of the NebuDCS plant model.  Everything in a
//! simulation lives under a project.
//!
//! # Hierarchy
//!
//! ```text
//! Project
//! └── Plant (1..n)
//!     └── Area (1..n)
//!         └── Unit (1..n)
//!             ├── EquipmentInstance (0..n)
//!             └── StreamDefinition (0..n)
//! ```

use serde::{Deserialize, Serialize};

use nebudcs_types::{AreaId, PlantId, ProjectId, SchemaVersion, UnitId};

use crate::equipment::EquipmentInstance;
use crate::stream::StreamDefinition;

/// Top-level project metadata.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Free-text description.
    pub description: String,
    /// Author / organisation.
    pub author: String,
    /// Date created (ISO 8601 string — not parsed for Phase 1).
    pub created: String,
    /// Date last modified.
    pub modified: String,
}

/// A process unit within an area — the finest subdivision of the plant
/// hierarchy before individual equipment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unit {
    pub id: UnitId,
    pub name: String,
    /// Equipment instances that belong to this unit.
    #[serde(default)]
    pub equipment: Vec<EquipmentInstance>,
    /// Stream connections within (and entering/leaving) this unit.
    #[serde(default)]
    pub streams: Vec<StreamDefinition>,
}

impl Unit {
    /// Create an empty unit with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: UnitId::new(),
            name: name.into(),
            equipment: Vec::new(),
            streams: Vec::new(),
        }
    }
}

/// A plant area — groups related process units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Area {
    pub id: AreaId,
    pub name: String,
    #[serde(default)]
    pub units: Vec<Unit>,
}

impl Area {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: AreaId::new(),
            name: name.into(),
            units: Vec::new(),
        }
    }
}

/// A plant — a collection of areas sharing a common site or purpose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plant {
    pub id: PlantId,
    pub name: String,
    #[serde(default)]
    pub areas: Vec<Area>,
}

impl Plant {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: PlantId::new(),
            name: name.into(),
            areas: Vec::new(),
        }
    }
}

/// The root of the NebuDCS domain model.
///
/// A `Project` is a complete, self-contained simulation project.  It carries
/// its schema version so that migrations can be applied on load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    /// Schema version of the persisted file format.
    pub schema_version: SchemaVersion,
    #[serde(default)]
    pub plants: Vec<Plant>,
    #[serde(default)]
    pub metadata: ProjectMetadata,
}

impl Project {
    /// Create a new empty project.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ProjectId::new(),
            name: name.into(),
            schema_version: SchemaVersion::CURRENT,
            plants: Vec::new(),
            metadata: ProjectMetadata::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn build_project() -> Project {
        let mut project = Project::new("Test Refinery");
        let mut plant = Plant::new("CDU Plant");
        let mut area = Area::new("Distillation Area");
        let unit = Unit::new("NHT Unit");
        area.units.push(unit);
        plant.areas.push(area);
        project.plants.push(plant);
        project
    }

    #[test]
    fn test_project_hierarchy() {
        let project = build_project();
        assert_eq!(project.name, "Test Refinery");
        assert_eq!(project.plants.len(), 1);
        assert_eq!(project.plants[0].areas.len(), 1);
        assert_eq!(project.plants[0].areas[0].units.len(), 1);
    }

    #[test]
    fn test_project_serde_json() {
        let project = build_project();
        let json = serde_json::to_string_pretty(&project).expect("serialize");
        let decoded: Project = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(project.name, decoded.name);
        assert_eq!(project.schema_version, decoded.schema_version);
        assert_eq!(project.plants.len(), decoded.plants.len());
    }

    #[test]
    fn test_project_schema_version_is_current() {
        let project = Project::new("test");
        assert_eq!(project.schema_version, SchemaVersion::CURRENT);
    }
}
