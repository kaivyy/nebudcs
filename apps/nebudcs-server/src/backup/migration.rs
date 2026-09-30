//! Forward-compatible schema migration engine for persisted projects, scenarios, and metadata.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::persistence::DiskStore;

/// Comprehensive report produced following a migration scan or execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationReportDto {
    pub entities_scanned: usize,
    pub entities_migrated: usize,
    pub details: Vec<String>,
}

/// Migrates a raw JSON project representation to the current schema specification if necessary.
/// Returns `true` if any schema migration was performed.
pub fn migrate_project_json(val: &mut Value) -> bool {
    let mut modified = false;

    if let Some(map) = val.as_object_mut() {
        // 1. Ensure schema_version is present and updated
        if let Some(schema_ver) = map.get_mut("schema_version") {
            if schema_ver == "0.1.0" {
                *schema_ver = Value::String("1.0.0".to_string());
                modified = true;
            }
        } else {
            map.insert(
                "schema_version".to_string(),
                Value::String("1.0.0".to_string()),
            );
            modified = true;
        }

        // 2. Ensure plants -> areas -> units -> equipment definitions have definition_version
        if let Some(plants) = map.get_mut("plants").and_then(|p| p.as_array_mut()) {
            for plant in plants {
                if let Some(areas) = plant.get_mut("areas").and_then(|a| a.as_array_mut()) {
                    for area in areas {
                        if let Some(units) = area.get_mut("units").and_then(|u| u.as_array_mut()) {
                            for unit in units {
                                if let Some(eqs) =
                                    unit.get_mut("equipment").and_then(|e| e.as_array_mut())
                                {
                                    for eq in eqs {
                                        if let Some(eq_map) = eq.as_object_mut() {
                                            if !eq_map.contains_key("definition_version") {
                                                eq_map.insert(
                                                    "definition_version".to_string(),
                                                    serde_json::json!({
                                                        "major": 1,
                                                        "minor": 0,
                                                        "patch": 0
                                                    }),
                                                );
                                                modified = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    modified
}

/// Migrates a raw JSON scenario representation if necessary.
pub fn migrate_scenario_json(val: &mut Value) -> bool {
    let mut modified = false;

    if let Some(map) = val.as_object_mut() {
        // Ensure version field exists
        if !map.contains_key("version") {
            map.insert("version".to_string(), Value::String("1.0.0".to_string()));
            modified = true;
        }
    }

    modified
}

/// Runs schema migration across all persisted entities in the DiskStore.
pub fn run_migration(disk_store: &DiskStore) -> Result<MigrationReportDto, String> {
    let mut entities_scanned = 0;
    let mut entities_migrated = 0;
    let mut details = Vec::new();

    // 1. Scan and migrate projects
    for (id, mut val) in disk_store.load_all_in_category::<Value>("projects") {
        entities_scanned += 1;
        if migrate_project_json(&mut val) {
            disk_store.save_json("projects", &id, &val)?;
            entities_migrated += 1;
            details.push(format!("Migrated project '{id}' to schema version 1.0.0"));
        }
    }

    // 2. Scan and migrate scenarios
    for (id, mut val) in disk_store.load_all_in_category::<Value>("scenarios") {
        entities_scanned += 1;
        if migrate_scenario_json(&mut val) {
            disk_store.save_json("scenarios", &id, &val)?;
            entities_migrated += 1;
            details.push(format!("Migrated scenario '{id}' to schema version 1.0.0"));
        }
    }

    Ok(MigrationReportDto {
        entities_scanned,
        entities_migrated,
        details,
    })
}
