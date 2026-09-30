//! Scenario authoring and catalog management subsystem.

use nebudcs_protocol::{
    ProjectDetailDto, ScenarioAuthoringDto, ScenarioSummaryDto, ScenarioValidationResultDto,
};
use nebudcs_scenario::model::{
    InitialConditions, ScenarioDefinition, ScenarioEvent, ScenarioObjective,
    SCENARIO_SCHEMA_VERSION,
};
use nebudcs_scenario::training::evaluation::EvaluationCriterion;
use std::collections::BTreeMap;

/// Authoritative scenario manager holding declarative scenarios for training and assessment.
#[derive(Debug, Clone)]
pub struct ScenarioManager {
    scenarios: BTreeMap<String, ScenarioAuthoringDto>,
    disk_store: Option<crate::persistence::DiskStore>,
}

impl Default for ScenarioManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ScenarioManager {
    /// Creates a new ScenarioManager pre-seeded with generic operational upset scenarios.
    pub fn new() -> Self {
        let mut mgr = Self {
            scenarios: BTreeMap::new(),
            disk_store: None,
        };
        mgr.seed_default_scenarios();
        mgr
    }

    /// Attaches an atomic DiskStore, loading existing scenarios from disk and saving seeded defaults if empty.
    pub fn with_disk_store(mut self, disk_store: crate::persistence::DiskStore) -> Self {
        let on_disk = disk_store.load_all_in_category::<ScenarioAuthoringDto>("scenarios");
        if !on_disk.is_empty() {
            for (id, scn) in on_disk {
                self.scenarios.insert(id, scn);
            }
        } else {
            for (id, scn) in &self.scenarios {
                let _ = disk_store.save_json("scenarios", id, scn);
            }
        }
        self.disk_store = Some(disk_store);
        self
    }

    /// Reloads all scenarios from attached DiskStore.
    pub fn reload_from_disk(&mut self) -> Result<usize, String> {
        if let Some(ref store) = self.disk_store {
            let loaded = store.load_all_in_category::<ScenarioAuthoringDto>("scenarios");
            let count = loaded.len();
            for (id, scn) in loaded {
                self.scenarios.insert(id, scn);
            }
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Reference to raw scenarios map.
    pub fn raw_scenarios(&self) -> &BTreeMap<String, ScenarioAuthoringDto> {
        &self.scenarios
    }

    /// Pre-seeds standard generic scenarios for training and automated testing.
    fn seed_default_scenarios(&mut self) {
        // 1. Generic Feed Loss & Level Recovery Scenario
        let feed_loss_id = "scn-generic-feed-loss";
        let feed_loss_init = InitialConditions::default();
        let feed_loss_timeline: Vec<ScenarioEvent> = Vec::new();
        let feed_loss_objectives: Vec<ScenarioObjective> = Vec::new();

        let feed_loss_criteria = vec![
            EvaluationCriterion {
                criterion_id: "crit-feed-alarm-ack".to_string(),
                title: "Prompt Alarm Acknowledgment".to_string(),
                description: "Operator must acknowledge low inventory or feed flow alarm within 30 seconds of activation.".to_string(),
                kind: nebudcs_scenario::training::evaluation::CriterionKind::AlarmResponseTime {
                    alarm_name: "ALM-LVL-LO".to_string(),
                    max_allowed_response_sec: 30.0,
                },
                required: true,
                weight: 30,
                penalty: 15,
                severity: nebudcs_scenario::training::evaluation::CriterionSeverity::Standard,
            },
            EvaluationCriterion {
                criterion_id: "crit-feed-mode-ctrl".to_string(),
                title: "Switch Controller to Manual or Adjust Valve".to_string(),
                description: "Operator must intervene on level controller or feed valve to arrest vessel drainage.".to_string(),
                kind: nebudcs_scenario::training::evaluation::CriterionKind::RequiredAction {
                    target_entity: "LIC-101".to_string(),
                    action_type: "SET_MODE".to_string(),
                    expected_value: None,
                    tolerance: None,
                    within_sec: Some(60.0),
                },
                required: false,
                weight: 30,
                penalty: 0,
                severity: nebudcs_scenario::training::evaluation::CriterionSeverity::Standard,
            },
            EvaluationCriterion {
                criterion_id: "crit-no-low-trip".to_string(),
                title: "Prevent Protective Low-Level Trip".to_string(),
                description: "Vessel inventory must be protected to avoid automatic low-level emergency trip.".to_string(),
                kind: nebudcs_scenario::training::evaluation::CriterionKind::NoTripOccurred,
                required: true,
                weight: 40,
                penalty: 40,
                severity: nebudcs_scenario::training::evaluation::CriterionSeverity::Critical,
            },
        ];

        let feed_loss_dto = ScenarioAuthoringDto {
            id: feed_loss_id.to_string(),
            name: "Generic Feed Flow Disturbance & Level Recovery".to_string(),
            description: "Upstream supply disruption causes rapid inventory depletion in the separation vessel. Trainee must acknowledge alarm and stabilize level before low-level trip occurs.".to_string(),
            author: "Chief Training Instructor".to_string(),
            version: "1.0.0".to_string(),
            compatible_project_id: None,
            duration_limit_sec: Some(300.0),
            random_seed: 0x42_1337_0001,
            initial_conditions_json: serde_json::to_string_pretty(&feed_loss_init).unwrap_or_default(),
            timeline_json: serde_json::to_string_pretty(&feed_loss_timeline).unwrap_or_default(),
            objectives_json: serde_json::to_string_pretty(&feed_loss_objectives).unwrap_or_default(),
            criteria_json: serde_json::to_string_pretty(&feed_loss_criteria).unwrap_or_default(),
        };
        self.scenarios
            .insert(feed_loss_id.to_string(), feed_loss_dto);

        // 2. Generic Exotherm / Cooling Upset Scenario
        let cooling_upset_id = "scn-reactor-cooling-upset";
        let cooling_criteria = vec![
            EvaluationCriterion {
                criterion_id: "crit-temp-alarm-ack".to_string(),
                title: "Acknowledge High Temperature Alarm".to_string(),
                description: "Operator must acknowledge high temperature alarm within 25 seconds."
                    .to_string(),
                kind: nebudcs_scenario::training::evaluation::CriterionKind::AlarmResponseTime {
                    alarm_name: "ALM-TEMP-HI".to_string(),
                    max_allowed_response_sec: 25.0,
                },
                required: true,
                weight: 35,
                penalty: 20,
                severity: nebudcs_scenario::training::evaluation::CriterionSeverity::Standard,
            },
            EvaluationCriterion {
                criterion_id: "crit-forbid-bypass".to_string(),
                title: "Do Not Bypass Safety Interlocks".to_string(),
                description:
                    "Safety interlock bypass during runaway condition is strictly forbidden."
                        .to_string(),
                kind: nebudcs_scenario::training::evaluation::CriterionKind::ForbiddenAction {
                    target_entity: "ESD-RUNAWAY".to_string(),
                    action_type: "BYPASS_INTERLOCK".to_string(),
                },
                required: true,
                weight: 35,
                penalty: 50,
                severity: nebudcs_scenario::training::evaluation::CriterionSeverity::Fatal,
            },
            EvaluationCriterion {
                criterion_id: "crit-no-thermal-trip".to_string(),
                title: "Avert High Temperature Trip".to_string(),
                description: "Process temperature must not breach safety trip threshold."
                    .to_string(),
                kind: nebudcs_scenario::training::evaluation::CriterionKind::NoTripOccurred,
                required: true,
                weight: 30,
                penalty: 30,
                severity: nebudcs_scenario::training::evaluation::CriterionSeverity::Critical,
            },
        ];

        let cooling_dto = ScenarioAuthoringDto {
            id: cooling_upset_id.to_string(),
            name: "Generic Cooling Loss & Exotherm Mitigation".to_string(),
            description: "Cooling media supply loss induces a thermal excursion. Trainee must respond to temperature warning and throttle energy source before high temperature trip triggers.".to_string(),
            author: "Process Safety Instructor".to_string(),
            version: "1.0.0".to_string(),
            compatible_project_id: None,
            duration_limit_sec: Some(240.0),
            random_seed: 0x42_1337_0002,
            initial_conditions_json: serde_json::to_string_pretty(&InitialConditions::default()).unwrap_or_default(),
            timeline_json: serde_json::to_string_pretty(&Vec::<ScenarioEvent>::new()).unwrap_or_default(),
            objectives_json: serde_json::to_string_pretty(&Vec::<ScenarioObjective>::new()).unwrap_or_default(),
            criteria_json: serde_json::to_string_pretty(&cooling_criteria).unwrap_or_default(),
        };
        self.scenarios
            .insert(cooling_upset_id.to_string(), cooling_dto);
    }

    /// Lists all authored scenarios as lightweight summaries.
    pub fn list_scenarios(&self) -> Vec<ScenarioSummaryDto> {
        self.scenarios
            .values()
            .map(|s| {
                let timeline: Vec<ScenarioEvent> =
                    serde_json::from_str(&s.timeline_json).unwrap_or_default();
                let objectives: Vec<ScenarioObjective> =
                    serde_json::from_str(&s.objectives_json).unwrap_or_default();

                ScenarioSummaryDto {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    description: s.description.clone(),
                    author: s.author.clone(),
                    version: s.version.clone(),
                    compatible_project_id: s.compatible_project_id.clone(),
                    duration_limit_sec: s.duration_limit_sec,
                    event_count: timeline.len(),
                    objective_count: objectives.len(),
                }
            })
            .collect()
    }

    /// Retrieves full authoring DTO for a specific scenario by ID.
    pub fn get_scenario(&self, id: &str) -> Option<ScenarioAuthoringDto> {
        self.scenarios.get(id).cloned()
    }

    /// Saves or updates a scenario authoring definition.
    pub fn save_scenario(
        &mut self,
        dto: ScenarioAuthoringDto,
    ) -> Result<ScenarioAuthoringDto, String> {
        if dto.id.trim().is_empty() {
            return Err("Scenario ID cannot be empty".to_string());
        }
        if dto.name.trim().is_empty() {
            return Err("Scenario name cannot be empty".to_string());
        }

        // Validate JSON fields syntax
        serde_json::from_str::<InitialConditions>(&dto.initial_conditions_json)
            .map_err(|e| format!("Invalid initial_conditions_json: {e}"))?;
        serde_json::from_str::<Vec<ScenarioEvent>>(&dto.timeline_json)
            .map_err(|e| format!("Invalid timeline_json: {e}"))?;
        serde_json::from_str::<Vec<ScenarioObjective>>(&dto.objectives_json)
            .map_err(|e| format!("Invalid objectives_json: {e}"))?;
        serde_json::from_str::<Vec<EvaluationCriterion>>(&dto.criteria_json)
            .map_err(|e| format!("Invalid criteria_json: {e}"))?;

        if let Some(ref store) = self.disk_store {
            if let Err(e) = store.save_json("scenarios", &dto.id, &dto) {
                tracing::warn!("Failed to persist scenario '{}' to disk: {e}", dto.id);
            }
        }

        self.scenarios.insert(dto.id.clone(), dto.clone());
        Ok(dto)
    }

    /// Duplicates an existing scenario under a new ID and title.
    pub fn duplicate_scenario(
        &mut self,
        source_id: &str,
        new_id: &str,
        new_name: &str,
    ) -> Result<ScenarioAuthoringDto, String> {
        let source = self
            .get_scenario(source_id)
            .ok_or_else(|| format!("Source scenario '{source_id}' not found"))?;

        if new_id.trim().is_empty() {
            return Err("New scenario ID cannot be empty".to_string());
        }
        if self.scenarios.contains_key(new_id) {
            return Err(format!("Scenario with ID '{new_id}' already exists"));
        }

        let mut duplicated = source;
        duplicated.id = new_id.to_string();
        duplicated.name = if new_name.trim().is_empty() {
            format!("{} (Copy)", duplicated.name)
        } else {
            new_name.to_string()
        };

        if let Some(ref store) = self.disk_store {
            let _ = store.save_json("scenarios", &duplicated.id, &duplicated);
        }

        self.scenarios
            .insert(new_id.to_string(), duplicated.clone());
        Ok(duplicated)
    }

    /// Deletes a scenario from the registry.
    pub fn delete_scenario(&mut self, id: &str) -> Result<(), String> {
        self.scenarios
            .remove(id)
            .ok_or_else(|| format!("Scenario '{id}' not found"))?;

        if let Some(ref store) = self.disk_store {
            let _ = store.delete_json("scenarios", id);
        }

        Ok(())
    }

    /// Validates an authored scenario against a target project model.
    pub fn validate_scenario_against_project(
        &self,
        scenario_id: &str,
        project: &ProjectDetailDto,
    ) -> ScenarioValidationResultDto {
        let scn = match self.get_scenario(scenario_id) {
            Some(s) => s,
            None => {
                return ScenarioValidationResultDto {
                    valid: false,
                    errors: vec![format!("Scenario '{scenario_id}' does not exist")],
                    warnings: Vec::new(),
                }
            }
        };

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 1. Compatibility check
        if let Some(compat) = &scn.compatible_project_id {
            if compat != &project.id {
                warnings.push(format!(
                    "Scenario designates compatible project '{}', but target project is '{}'",
                    compat, project.id
                ));
            }
        }

        // Build project entity lookup sets
        let equipment_names: std::collections::BTreeSet<&str> =
            project.equipment.iter().map(|e| e.name.as_str()).collect();
        let loop_names: std::collections::BTreeSet<&str> = project
            .control_loops
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        let alarm_names: std::collections::BTreeSet<&str> =
            project.alarms.iter().map(|a| a.name.as_str()).collect();
        let instrument_tags: std::collections::BTreeSet<&str> =
            project.instruments.iter().map(|i| i.tag.as_str()).collect();

        // 2. Validate initial conditions
        if let Ok(init) = serde_json::from_str::<InitialConditions>(&scn.initial_conditions_json) {
            for tag_init in &init.tags {
                if !instrument_tags.contains(tag_init.tag.as_str()) {
                    warnings.push(format!(
                        "Initial condition tag '{}' not found in target project instruments",
                        tag_init.tag
                    ));
                }
            }
            for ctrl_init in &init.controllers {
                if !loop_names.contains(ctrl_init.loop_name.as_str()) {
                    errors.push(format!(
                        "Initial condition controller '{}' does not exist in target project",
                        ctrl_init.loop_name
                    ));
                }
            }
            for eq_init in &init.equipment {
                if !equipment_names.contains(eq_init.equipment_name.as_str()) {
                    errors.push(format!(
                        "Initial condition equipment '{}' does not exist in target project",
                        eq_init.equipment_name
                    ));
                }
            }
        } else {
            errors.push("Failed to parse initial_conditions_json".to_string());
        }

        // 3. Validate criteria entities
        if let Ok(criteria) = serde_json::from_str::<Vec<EvaluationCriterion>>(&scn.criteria_json) {
            for crit in &criteria {
                match &crit.kind {
                    nebudcs_scenario::training::evaluation::CriterionKind::AlarmResponseTime { alarm_name, .. } => {
                        if !alarm_names.contains(alarm_name.as_str()) {
                            warnings.push(format!(
                                "Criterion '{}' references alarm '{}' which is not declared in target project",
                                crit.criterion_id, alarm_name
                            ));
                        }
                    }
                    nebudcs_scenario::training::evaluation::CriterionKind::CorrectControlMode { loop_name, .. }
                    | nebudcs_scenario::training::evaluation::CriterionKind::CorrectSetpoint { loop_name, .. } => {
                        if !loop_names.contains(loop_name.as_str()) {
                            errors.push(format!(
                                "Criterion '{}' references control loop '{}' which does not exist in target project",
                                crit.criterion_id, loop_name
                            ));
                        }
                    }
                    nebudcs_scenario::training::evaluation::CriterionKind::CorrectEquipmentState { equipment_name, .. }
                        if !equipment_names.contains(equipment_name.as_str()) =>
                    {
                        errors.push(format!(
                            "Criterion '{}' references equipment '{}' which does not exist in target project",
                            crit.criterion_id, equipment_name
                        ));
                    }
                    _ => {}
                }
            }
        } else {
            errors.push("Failed to parse criteria_json".to_string());
        }

        ScenarioValidationResultDto {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Converts an authored scenario DTO into an executable runtime ScenarioDefinition.
    pub fn to_runtime_definition(&self, scenario_id: &str) -> Option<ScenarioDefinition> {
        let scn = self.get_scenario(scenario_id)?;
        let initial_conditions: InitialConditions =
            serde_json::from_str(&scn.initial_conditions_json).unwrap_or_default();
        let timeline: Vec<ScenarioEvent> =
            serde_json::from_str(&scn.timeline_json).unwrap_or_default();
        let objectives: Vec<ScenarioObjective> =
            serde_json::from_str(&scn.objectives_json).unwrap_or_default();

        Some(ScenarioDefinition {
            id: scn.id,
            name: scn.name,
            description: scn.description,
            author: scn.author,
            schema_version: SCENARIO_SCHEMA_VERSION,
            duration_limit_sec: scn.duration_limit_sec,
            random_seed: scn.random_seed,
            initial_conditions,
            timeline,
            objectives,
            branches: Vec::new(),
        })
    }
}
