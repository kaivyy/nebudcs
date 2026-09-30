//! Comprehensive project and topology validation engine for NebuDCS.
//!
//! Evaluates design-time model correctness across:
//! - Project structure & hierarchy
//! - Equipment template existence & parameter sanity
//! - Stream source/sink existence and port compatibility
//! - Cardinality & unconnected required port warnings
//! - Topology graph consistency and recycle loop detection
//! - Instrument configuration & ranges
//! - Control loop tuning & limits
//! - Alarm thresholds & deadbands
//! - Safety & process interlock configurations

use std::collections::BTreeMap;
use std::str::FromStr;

use indexmap::IndexMap;
use nebudcs_model::equipment::{Cardinality, EquipmentDefinition, EquipmentInstance};
use nebudcs_model::project::{
    AlarmInstance, ControlLoopInstance, InstrumentInstance, InterlockInstance, Project,
};
use nebudcs_model::stream::StreamDefinition;
use nebudcs_model::validation::{ValidationResult, ValidationSeverity};
use nebudcs_types::{EntityId, EquipmentDefinitionId, EquipmentId, PortId};

use crate::graph::TopologyGraph;
use crate::port::validate_port_compatibility;

fn to_entity_id<T: std::fmt::Display>(val: T) -> Option<EntityId> {
    val.to_string().parse::<EntityId>().ok()
}

/// Summary result of validating an entire project.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProjectValidationSummary {
    pub is_valid: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub review_count: usize,
    pub results: Vec<ValidationResult>,
}

impl ProjectValidationSummary {
    /// Constructs a summary from a collection of validation results.
    pub fn from_results(results: Vec<ValidationResult>) -> Self {
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut info_count = 0;
        let mut review_count = 0;

        for r in &results {
            match r.severity {
                ValidationSeverity::Error => error_count += 1,
                ValidationSeverity::Warning => warning_count += 1,
                ValidationSeverity::Info => info_count += 1,
                ValidationSeverity::EngineeringReview => review_count += 1,
            }
        }

        Self {
            is_valid: error_count == 0,
            error_count,
            warning_count,
            info_count,
            review_count,
            results,
        }
    }
}

/// Validates an entire project against a registered library of equipment definitions.
pub fn validate_project(
    project: &Project,
    defs: &IndexMap<EquipmentDefinitionId, EquipmentDefinition>,
) -> ProjectValidationSummary {
    let mut results = Vec::new();

    // 1. Hierarchy validation
    if project.plants.is_empty() {
        results.push(ValidationResult::info(
            "VLD-H01",
            "Project contains no plants",
            to_entity_id(project.id),
        ));
        return ProjectValidationSummary::from_results(results);
    }

    let mut total_units = 0;
    for plant in &project.plants {
        for area in &plant.areas {
            for unit in &area.units {
                total_units += 1;
                validate_unit_contents(
                    &unit.equipment,
                    &unit.streams,
                    &unit.instruments,
                    &unit.control_loops,
                    &unit.alarms,
                    &unit.interlocks,
                    defs,
                    &mut results,
                );
            }
        }
    }

    if total_units == 0 {
        results.push(ValidationResult::info(
            "VLD-H02",
            "Project contains no process units",
            to_entity_id(project.id),
        ));
    }

    validate_displays_and_navigation(project, &mut results);

    ProjectValidationSummary::from_results(results)
}

/// Validates displays, operational structure, and cross-display navigation links.
pub fn validate_displays_and_navigation(
    project: &Project,
    results: &mut Vec<ValidationResult>,
) {
    let mut known_equipment = std::collections::HashSet::new();
    for plant in &project.plants {
        for area in &plant.areas {
            for unit in &area.units {
                for eq in &unit.equipment {
                    known_equipment.insert(eq.id.to_string());
                    known_equipment.insert(eq.name.clone());
                }
            }
        }
    }
    if let Some(ref ps) = project.plant_structure {
        for area in &ps.areas {
            for sys in &area.systems {
                for sub in &sys.subsystems {
                    for u in &sub.units {
                        for eq_id in &u.equipment_ids {
                            known_equipment.insert(eq_id.clone());
                        }
                    }
                }
            }
        }
    }

    let mut known_displays = std::collections::HashSet::new();
    for disp in &project.displays {
        if disp.id.trim().is_empty() {
            results.push(ValidationResult::error(
                "VLD-DSP01",
                "Display identifier cannot be empty",
                None,
            ));
        } else if !known_displays.insert(disp.id.clone()) {
            results.push(ValidationResult::error(
                "VLD-DSP02",
                format!("Duplicate display identifier '{}'", disp.id),
                None,
            ));
        }

        // Validate equipment membership references
        for eq_id in &disp.equipment_ids {
            let parsed_id = nebudcs_types::EquipmentId::from_str(eq_id)
                .map(|id| id.to_string())
                .unwrap_or_default();
            if !known_equipment.is_empty()
                && !known_equipment.contains(eq_id)
                && !known_equipment.contains(&parsed_id)
            {
                results.push(ValidationResult::warning(
                    "VLD-DSP03",
                    format!("Display '{}' references non-existent equipment ID '{}'", disp.name, eq_id),
                    None,
                ));
            }
        }

        // Validate display-specific equipment placement references
        for pl in &disp.placements {
            let parsed_id = nebudcs_types::EquipmentId::from_str(&pl.equipment_id)
                .map(|id| id.to_string())
                .unwrap_or_default();
            if !known_equipment.is_empty()
                && !known_equipment.contains(&pl.equipment_id)
                && !known_equipment.contains(&parsed_id)
            {
                results.push(ValidationResult::warning(
                    "VLD-DSP04",
                    format!("Display '{}' placement references non-existent equipment ID '{}'", disp.name, pl.equipment_id),
                    None,
                ));
            }
        }
    }

    // Also collect displays from plant_structure if present
    if let Some(ref ps) = project.plant_structure {
        for disp in &ps.displays {
            known_displays.insert(disp.id.clone());
        }
    }

    // Validate navigation links
    for link in &project.navigation_links {
        if !known_displays.is_empty() && !known_displays.contains(&link.target_display_id) {
            results.push(ValidationResult::error(
                "VLD-NAV01",
                format!(
                    "Navigation link '{}' references non-existent target display '{}'",
                    link.label, link.target_display_id
                ),
                None,
            ));
        }
        if !known_displays.is_empty() && !known_displays.contains(&link.source_display_id) {
            results.push(ValidationResult::warning(
                "VLD-NAV02",
                format!(
                    "Navigation link '{}' references non-existent source display '{}'",
                    link.label, link.source_display_id
                ),
                None,
            ));
        }
        if let Some(ref elem_id) = link.target_element_id {
            let parsed_id = nebudcs_types::EquipmentId::from_str(elem_id)
                .map(|id| id.to_string())
                .unwrap_or_default();
            if !known_equipment.is_empty()
                && !known_equipment.contains(elem_id)
                && !known_equipment.contains(&parsed_id)
            {
                results.push(ValidationResult::warning(
                    "VLD-NAV03",
                    format!(
                        "Navigation link '{}' references non-existent target element '{}'",
                        link.label, elem_id
                    ),
                    None,
                ));
            }
        }
    }
}

/// Validates equipment, streams, instruments, and control systems within a single unit.
#[allow(clippy::too_many_arguments)]
pub fn validate_unit_contents(
    equipment: &[EquipmentInstance],
    streams: &[StreamDefinition],
    instruments: &[InstrumentInstance],
    control_loops: &[ControlLoopInstance],
    alarms: &[AlarmInstance],
    interlocks: &[InterlockInstance],
    defs: &IndexMap<EquipmentDefinitionId, EquipmentDefinition>,
    results: &mut Vec<ValidationResult>,
) {
    let mut equipment_map: BTreeMap<
        EquipmentId,
        (&EquipmentInstance, Option<&EquipmentDefinition>),
    > = BTreeMap::new();
    let mut port_connection_counts: BTreeMap<(EquipmentId, PortId), usize> = BTreeMap::new();

    // 1. Equipment Validation
    for eq in equipment {
        if eq.name.trim().is_empty() {
            results.push(ValidationResult::error(
                "VLD-EQ01",
                "Equipment name cannot be empty",
                to_entity_id(eq.id),
            ));
        }

        let def_opt = defs.get(&eq.definition_id);
        if let Some(def) = def_opt {
            equipment_map.insert(eq.id, (eq, Some(def)));

            // Parameter finite checks
            for (param_name, val) in &eq.parameters {
                if !val.is_finite() {
                    results.push(ValidationResult::error(
                        "VLD-EQ03",
                        format!("Parameter '{param_name}' has non-finite value ({val})"),
                        to_entity_id(eq.id),
                    ));
                }
            }
        } else {
            equipment_map.insert(eq.id, (eq, None));
            results.push(ValidationResult::error(
                "VLD-EQ02",
                format!(
                    "Equipment '{}' references unknown template definition '{}'",
                    eq.name, eq.definition_id
                ),
                to_entity_id(eq.id),
            ));
        }
    }

    // 2. Stream Validation & Port Compatibility
    for stream in streams {
        if stream.name.trim().is_empty() {
            results.push(ValidationResult::warning(
                "VLD-STR01",
                "Stream name is empty",
                to_entity_id(stream.id),
            ));
        }

        let source_entry = equipment_map.get(&stream.source.equipment_id);
        let sink_entry = equipment_map.get(&stream.sink.equipment_id);

        if source_entry.is_none() {
            results.push(ValidationResult::error(
                "VLD-STR02",
                format!(
                    "Stream '{}' references nonexistent source equipment '{}'",
                    stream.name, stream.source.equipment_id
                ),
                to_entity_id(stream.id),
            ));
            continue;
        }

        if sink_entry.is_none() {
            results.push(ValidationResult::error(
                "VLD-STR03",
                format!(
                    "Stream '{}' references nonexistent sink equipment '{}'",
                    stream.name, stream.sink.equipment_id
                ),
                to_entity_id(stream.id),
            ));
            continue;
        }

        let (source_eq, source_def_opt) = source_entry.unwrap();
        let (sink_eq, sink_def_opt) = sink_entry.unwrap();

        if let (Some(s_def), Some(d_def)) = (source_def_opt, sink_def_opt) {
            let s_port_opt = s_def.ports.iter().find(|p| {
                p.id == stream.source.port_id || p.name == stream.source.port_id.to_string()
            });
            let d_port_opt = d_def
                .ports
                .iter()
                .find(|p| p.id == stream.sink.port_id || p.name == stream.sink.port_id.to_string());

            match (s_port_opt, d_port_opt) {
                (Some(s_port), Some(d_port)) => {
                    let s_count = *port_connection_counts
                        .get(&(source_eq.id, s_port.id))
                        .unwrap_or(&0);
                    let d_count = *port_connection_counts
                        .get(&(sink_eq.id, d_port.id))
                        .unwrap_or(&0);

                    if let Err(err) = validate_port_compatibility(
                        source_eq.id,
                        s_port,
                        s_count,
                        sink_eq.id,
                        d_port,
                        d_count,
                        Some(&stream.stream_type),
                    ) {
                        results.push(ValidationResult::error(
                            "VLD-PRT01",
                            format!(
                                "Incompatible stream connection '{}' between '{}.{}' and '{}.{}': {err}",
                                stream.name,
                                source_eq.name,
                                s_port.name,
                                sink_eq.name,
                                d_port.name
                            ),
                            to_entity_id(stream.id),
                        ));
                    }

                    *port_connection_counts
                        .entry((source_eq.id, s_port.id))
                        .or_default() += 1;
                    *port_connection_counts
                        .entry((sink_eq.id, d_port.id))
                        .or_default() += 1;
                }
                (None, _) => {
                    results.push(ValidationResult::error(
                        "VLD-PRT02",
                        format!(
                            "Source port '{}' not found on equipment '{}'",
                            stream.source.port_id, source_eq.name
                        ),
                        to_entity_id(stream.id),
                    ));
                }
                (_, None) => {
                    results.push(ValidationResult::error(
                        "VLD-PRT03",
                        format!(
                            "Sink port '{}' not found on equipment '{}'",
                            stream.sink.port_id, sink_eq.name
                        ),
                        to_entity_id(stream.id),
                    ));
                }
            }
        }

        // Composition check if material
        if let Some(comp) = &stream.composition {
            if !comp.is_empty() {
                let sum: f64 = comp.components.values().sum();
                if (sum - 1.0).abs() > 0.01 {
                    results.push(ValidationResult::warning(
                        "VLD-STR04",
                        format!(
                            "Stream '{}' composition mole fractions sum to {:.4} (expected 1.0)",
                            stream.name, sum
                        ),
                        to_entity_id(stream.id),
                    ));
                }
            }
        }
    }

    // 3. Port Cardinality & Unconnected Port Checks
    for (eq_id, (eq, def_opt)) in &equipment_map {
        if let Some(def) = def_opt {
            let mut eq_conns = 0;
            for port in &def.ports {
                let count = *port_connection_counts.get(&(*eq_id, port.id)).unwrap_or(&0);
                eq_conns += count;

                match port.cardinality {
                    Cardinality::ExactlyOne if count == 0 => {
                        results.push(ValidationResult::warning(
                            "VLD-PRT04",
                            format!(
                                "Required port '{}.{}' ({:?}) is not connected",
                                eq.name, port.name, port.direction
                            ),
                            to_entity_id(eq.id),
                        ));
                    }
                    Cardinality::OneOrMore if count == 0 => {
                        results.push(ValidationResult::warning(
                            "VLD-PRT05",
                            format!(
                                "Required port '{}.{}' expects at least one connection",
                                eq.name, port.name
                            ),
                            to_entity_id(eq.id),
                        ));
                    }
                    Cardinality::ZeroOrOne if count > 1 => {
                        results.push(ValidationResult::error(
                            "VLD-PRT06",
                            format!(
                                "Port '{}.{}' permits at most 1 connection, found {count}",
                                eq.name, port.name
                            ),
                            to_entity_id(eq.id),
                        ));
                    }
                    Cardinality::ExactlyOne if count > 1 => {
                        results.push(ValidationResult::error(
                            "VLD-PRT07",
                            format!(
                                "Port '{}.{}' permits exactly 1 connection, found {count}",
                                eq.name, port.name
                            ),
                            to_entity_id(eq.id),
                        ));
                    }
                    _ => {}
                }
            }

            if eq_conns == 0 && !def.ports.is_empty() {
                results.push(ValidationResult::info(
                    "VLD-EQ04",
                    format!(
                        "Equipment '{}' is isolated (has no connected streams)",
                        eq.name
                    ),
                    to_entity_id(eq.id),
                ));
            }
        }
    }

    // 4. Topology Consistency & Recycle Loops
    let mut topo = TopologyGraph::new();
    for (eq_id, (_, def_opt)) in &equipment_map {
        if let Some(def) = def_opt {
            topo.register_definition((*def).clone());
            let _ = topo.add_equipment(*eq_id, def.id);
        }
    }
    for stream in streams {
        let _ = topo.add_stream(stream.clone());
    }

    let scc_res = topo.analyze_scc();
    for scc in scc_res.components {
        if scc.len() > 1 {
            let names: Vec<String> = scc
                .iter()
                .filter_map(|id| equipment_map.get(id).map(|(eq, _)| eq.name.clone()))
                .collect();
            results.push(ValidationResult::info(
                "VLD-TOP01",
                format!(
                    "Recycle loop detected involving: {} (will converge iteratively)",
                    names.join(" ↔ ")
                ),
                None,
            ));
        }
    }

    // 5. Instruments Validation
    for inst in instruments {
        if inst.tag.trim().is_empty() {
            results.push(ValidationResult::error(
                "VLD-INST01",
                "Instrument tag cannot be empty",
                Some(inst.id),
            ));
        }
        if inst.range_min >= inst.range_max {
            results.push(ValidationResult::error(
                "VLD-INST02",
                format!(
                    "Instrument '{}' invalid measurement range: min ({}) >= max ({})",
                    inst.tag, inst.range_min, inst.range_max
                ),
                Some(inst.id),
            ));
        }
        if let Some(eq_id) = inst.equipment_id {
            if !equipment_map.contains_key(&eq_id) {
                results.push(ValidationResult::error(
                    "VLD-INST03",
                    format!(
                        "Instrument '{}' references nonexistent equipment '{eq_id}'",
                        inst.tag
                    ),
                    Some(inst.id),
                ));
            }
        }
    }

    // 6. Control Loops Validation
    for loop_cfg in control_loops {
        if loop_cfg.name.trim().is_empty() {
            results.push(ValidationResult::error(
                "VLD-CTL01",
                "Control loop name cannot be empty",
                Some(loop_cfg.id),
            ));
        }
        if loop_cfg.pv_tag.trim().is_empty() {
            results.push(ValidationResult::error(
                "VLD-CTL02",
                format!("Control loop '{}' PV tag cannot be empty", loop_cfg.name),
                Some(loop_cfg.id),
            ));
        }
        if loop_cfg.out_tag.trim().is_empty() {
            results.push(ValidationResult::error(
                "VLD-CTL03",
                format!("Control loop '{}' OUT tag cannot be empty", loop_cfg.name),
                Some(loop_cfg.id),
            ));
        }
        if loop_cfg.kp < 0.0 || loop_cfg.ti < 0.0 || loop_cfg.td < 0.0 {
            results.push(ValidationResult::error(
                "VLD-CTL04",
                format!(
                    "Control loop '{}' PID tuning constants must be non-negative (Kp={}, Ti={}, Td={})",
                    loop_cfg.name, loop_cfg.kp, loop_cfg.ti, loop_cfg.td
                ),
                Some(loop_cfg.id),
            ));
        }
        if loop_cfg.output_min >= loop_cfg.output_max {
            results.push(ValidationResult::error(
                "VLD-CTL05",
                format!(
                    "Control loop '{}' invalid output limits: min ({}) >= max ({})",
                    loop_cfg.name, loop_cfg.output_min, loop_cfg.output_max
                ),
                Some(loop_cfg.id),
            ));
        }
    }

    // 7. Alarms Validation
    for alm in alarms {
        if alm.name.trim().is_empty() || alm.pv_tag.trim().is_empty() {
            results.push(ValidationResult::error(
                "VLD-ALM01",
                "Alarm name and PV tag cannot be empty",
                Some(alm.id),
            ));
        }
        if alm.deadband < 0.0 {
            results.push(ValidationResult::error(
                "VLD-ALM02",
                format!("Alarm '{}' deadband must be non-negative", alm.name),
                Some(alm.id),
            ));
        }
    }

    // 8. Interlocks Validation
    for ilock in interlocks {
        if ilock.name.trim().is_empty() || ilock.source_tag.trim().is_empty() {
            results.push(ValidationResult::error(
                "VLD-ILK01",
                "Interlock name and source tag cannot be empty",
                Some(ilock.id),
            ));
        }
        if !ilock.threshold.is_finite() {
            results.push(ValidationResult::error(
                "VLD-ILK02",
                format!("Interlock '{}' threshold must be finite", ilock.name),
                Some(ilock.id),
            ));
        }
    }
}
