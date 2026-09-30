use indexmap::IndexMap;
use nebudcs_model::equipment::{
    Cardinality, EquipmentCategory, EquipmentDefinition, EquipmentInstance, Phase, PortDefinition,
    PortDirection, PortDomain, Position2D,
};
use nebudcs_model::project::{Area, ControlLoopInstance, InstrumentInstance, Plant, Project, Unit};
use nebudcs_model::stream::{StreamDefinition, StreamType};
use nebudcs_topology::validation::validate_project;
use nebudcs_types::{EntityId, EquipmentDefinitionId, EquipmentId, PortId, SchemaVersion};

fn make_sample_definitions() -> IndexMap<EquipmentDefinitionId, EquipmentDefinition> {
    let mut defs = IndexMap::new();

    // Pump definition: Suction (Inlet), Discharge (Outlet)
    let pump_def_id = EquipmentDefinitionId::new();
    let pump_suction = PortDefinition {
        id: PortId::new(),
        name: "Suction".to_string(),
        domain: PortDomain::Material,
        direction: PortDirection::Inlet,
        phase_compatibility: vec![Phase::Liquid],
        required: true,
        cardinality: Cardinality::ExactlyOne,
        description: "Pump inlet suction nozzle".to_string(),
    };
    let pump_discharge = PortDefinition {
        id: PortId::new(),
        name: "Discharge".to_string(),
        domain: PortDomain::Material,
        direction: PortDirection::Outlet,
        phase_compatibility: vec![Phase::Liquid],
        required: true,
        cardinality: Cardinality::OneOrMore,
        description: "Pump outlet discharge nozzle".to_string(),
    };
    let pump_def = EquipmentDefinition {
        id: pump_def_id,
        name: "CentrifugalPump".to_string(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Rotating,
        capabilities: vec![],
        parameters: IndexMap::new(),
        ports: vec![pump_suction, pump_discharge],
        tags: vec![],
        operating_envelope: Default::default(),
        alarm_definitions: vec![],
        failure_modes: vec![],
        state_machine: Default::default(),
        metadata: Default::default(),
    };
    defs.insert(pump_def_id, pump_def);

    // Valve definition: Inlet, Outlet
    let valve_def_id = EquipmentDefinitionId::new();
    let valve_inlet = PortDefinition {
        id: PortId::new(),
        name: "Inlet".to_string(),
        domain: PortDomain::Material,
        direction: PortDirection::Inlet,
        phase_compatibility: vec![Phase::Liquid, Phase::Gas],
        required: true,
        cardinality: Cardinality::ExactlyOne,
        description: "Valve inlet".to_string(),
    };
    let valve_outlet = PortDefinition {
        id: PortId::new(),
        name: "Outlet".to_string(),
        domain: PortDomain::Material,
        direction: PortDirection::Outlet,
        phase_compatibility: vec![Phase::Liquid, Phase::Gas],
        required: true,
        cardinality: Cardinality::ExactlyOne,
        description: "Valve outlet".to_string(),
    };
    let valve_def = EquipmentDefinition {
        id: valve_def_id,
        name: "ControlValve".to_string(),
        version: SchemaVersion::new(1, 0, 0),
        category: EquipmentCategory::Valve,
        capabilities: vec![],
        parameters: IndexMap::new(),
        ports: vec![valve_inlet, valve_outlet],
        tags: vec![],
        operating_envelope: Default::default(),
        alarm_definitions: vec![],
        failure_modes: vec![],
        state_machine: Default::default(),
        metadata: Default::default(),
    };
    defs.insert(valve_def_id, valve_def);

    defs
}

#[test]
fn test_validate_empty_project() {
    let project = Project::new("Empty Plant");
    let defs = make_sample_definitions();
    let summary = validate_project(&project, &defs);

    assert!(summary.is_valid);
    assert!(summary.results.iter().any(|r| r.code == "VLD-H01"));
}

#[test]
fn test_validate_valid_unit_flowsheet() {
    let defs = make_sample_definitions();
    let pump_def = defs.values().find(|d| d.name == "CentrifugalPump").unwrap();
    let valve_def = defs.values().find(|d| d.name == "ControlValve").unwrap();

    let mut project = Project::new("Refinery Loop");
    let mut plant = Plant::new("Area 100");
    let mut area = Area::new("Distillation");
    let mut unit = Unit::new("Crude Preflash");

    let p101_id = EquipmentId::new();
    let p101 = EquipmentInstance {
        id: p101_id,
        definition_id: pump_def.id,
        definition_version: pump_def.version,
        name: "P-101".to_string(),
        parameters: IndexMap::new(),
        position: Some(Position2D { x: 100.0, y: 100.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    };

    let fv101_id = EquipmentId::new();
    let fv101 = EquipmentInstance {
        id: fv101_id,
        definition_id: valve_def.id,
        definition_version: valve_def.version,
        name: "FV-101".to_string(),
        parameters: IndexMap::new(),
        position: Some(Position2D { x: 250.0, y: 100.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    };

    let stream = StreamDefinition::new(
        "S-01",
        StreamType::Material,
        p101_id,
        pump_def.ports[1].id, // Discharge (Outlet)
        fv101_id,
        valve_def.ports[0].id, // Inlet
    );

    unit.equipment.push(p101);
    unit.equipment.push(fv101);
    unit.streams.push(stream);

    area.units.push(unit);
    plant.areas.push(area);
    project.plants.push(plant);

    let summary = validate_project(&project, &defs);
    assert_eq!(
        summary.error_count, 0,
        "Expected 0 errors: {:?}",
        summary.results
    );
}

#[test]
fn test_validate_unknown_equipment_definition() {
    let defs = make_sample_definitions();
    let mut project = Project::new("Invalid Def Project");
    let mut plant = Plant::new("Plant");
    let mut area = Area::new("Area");
    let mut unit = Unit::new("Unit");

    let fake_def_id = EquipmentDefinitionId::new();
    let eq = EquipmentInstance {
        id: EquipmentId::new(),
        definition_id: fake_def_id,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "K-999".to_string(),
        parameters: IndexMap::new(),
        position: None,
        display_width: None,
        display_height: None,
        rotation: None,
    };
    unit.equipment.push(eq);
    area.units.push(unit);
    plant.areas.push(area);
    project.plants.push(plant);

    let summary = validate_project(&project, &defs);
    assert!(!summary.is_valid);
    assert!(summary.results.iter().any(|r| r.code == "VLD-EQ02"));
}

#[test]
fn test_validate_incompatible_port_directions() {
    let defs = make_sample_definitions();
    let pump_def = defs.values().find(|d| d.name == "CentrifugalPump").unwrap();

    let mut project = Project::new("Invalid Ports Project");
    let mut plant = Plant::new("Plant");
    let mut area = Area::new("Area");
    let mut unit = Unit::new("Unit");

    let p1 = EquipmentId::new();
    let p2 = EquipmentId::new();

    unit.equipment.push(EquipmentInstance {
        id: p1,
        definition_id: pump_def.id,
        definition_version: pump_def.version,
        name: "P-1".to_string(),
        parameters: IndexMap::new(),
        position: None,
        display_width: None,
        display_height: None,
        rotation: None,
    });
    unit.equipment.push(EquipmentInstance {
        id: p2,
        definition_id: pump_def.id,
        definition_version: pump_def.version,
        name: "P-2".to_string(),
        parameters: IndexMap::new(),
        position: None,
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Attempt to connect Inlet (Suction) to Inlet (Suction)
    let bad_stream = StreamDefinition::new(
        "BAD-S1",
        StreamType::Material,
        p1,
        pump_def.ports[0].id, // Inlet
        p2,
        pump_def.ports[0].id, // Inlet
    );
    unit.streams.push(bad_stream);

    area.units.push(unit);
    plant.areas.push(area);
    project.plants.push(plant);

    let summary = validate_project(&project, &defs);
    assert!(!summary.is_valid);
    assert!(summary.results.iter().any(|r| r.code == "VLD-PRT01"));
}

#[test]
fn test_validate_instrument_and_control_sanity() {
    let defs = make_sample_definitions();
    let mut project = Project::new("Control Test");
    let mut plant = Plant::new("Plant");
    let mut area = Area::new("Area");
    let mut unit = Unit::new("Unit");

    // Bad instrument: min >= max
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "TI-999".to_string(),
        equipment_id: None,
        port_id: None,
        measurement_kind: "Temperature".to_string(),
        range_min: 100.0,
        range_max: 50.0, // Invalid!
        unit: "degC".to_string(),
        position: None,
    });

    // Bad control loop: negative Ti
    unit.control_loops.push(ControlLoopInstance {
        id: EntityId::new(),
        name: "TIC-999".to_string(),
        pv_tag: "TI-999".to_string(),
        sp: 75.0,
        out_tag: "TV-999.POS".to_string(),
        kp: 1.5,
        ti: -10.0, // Invalid!
        td: 0.0,
        reverse_acting: false,
        mode: "AUTO".to_string(),
        output_min: 0.0,
        output_max: 100.0,
        position: None,
    });

    area.units.push(unit);
    plant.areas.push(area);
    project.plants.push(plant);

    let summary = validate_project(&project, &defs);
    assert!(!summary.is_valid);
    assert!(summary.results.iter().any(|r| r.code == "VLD-INST02"));
    assert!(summary.results.iter().any(|r| r.code == "VLD-CTL04"));
}

#[test]
fn test_validate_displays_and_navigation_links() {
    let defs = make_sample_definitions();
    let mut project = Project::new("Test Plant Displays");

    let mut plant = Plant::new("Plant 1");
    let mut area = Area::new("Area 1");
    let mut unit = Unit::new("Unit 1");

    let pump_def = defs.values().next().unwrap();
    let p101_id = EquipmentId::new();
    unit.equipment.push(EquipmentInstance {
        id: p101_id,
        definition_id: pump_def.id,
        definition_version: pump_def.version,
        name: "P-101".to_string(),
        parameters: IndexMap::new(),
        position: Some(Position2D { x: 100.0, y: 100.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });
    area.units.push(unit);
    plant.areas.push(area);
    project.plants.push(plant);

    // 1. Add valid display
    project.displays.push(nebudcs_model::project::ProjectDisplay {
        id: "disp-overview".to_string(),
        name: "Plant Overview".to_string(),
        display_type: "plant_overview".to_string(),
        area_id: None,
        system_id: None,
        subsystem_id: None,
        unit_id: None,
        equipment_ids: vec![p101_id.to_string()],
        instrument_ids: Vec::new(),
        stream_ids: Vec::new(),
        viewport_x: None,
        viewport_y: None,
        viewport_width: None,
        viewport_height: None,
        viewport_zoom: None,
        is_default: true,
        order: Some(1),
        description: None,
        placements: Vec::new(),
    });

    // 2. Add navigation link targeting non-existent display
    project.navigation_links.push(nebudcs_model::project::ProjectNavigationLink {
        id: "nav-broken".to_string(),
        source_display_id: "disp-overview".to_string(),
        source_element_id: Some(p101_id.to_string()),
        target_display_id: "disp-nonexistent".to_string(),
        target_element_id: None,
        label: "Jump to Nonexistent".to_string(),
        behavior: "focus_and_center".to_string(),
        description: None,
    });

    let summary = validate_project(&project, &defs);
    assert!(!summary.is_valid);
    assert!(summary.results.iter().any(|r| r.code == "VLD-NAV01"));

    // Fix navigation link target
    project.displays.push(nebudcs_model::project::ProjectDisplay {
        id: "disp-nonexistent".to_string(),
        name: "Secondary Display".to_string(),
        display_type: "unit_process_graphic".to_string(),
        area_id: None,
        system_id: None,
        subsystem_id: None,
        unit_id: None,
        equipment_ids: Vec::new(),
        instrument_ids: Vec::new(),
        stream_ids: Vec::new(),
        viewport_x: None,
        viewport_y: None,
        viewport_width: None,
        viewport_height: None,
        viewport_zoom: None,
        is_default: false,
        order: Some(2),
        description: None,
        placements: Vec::new(),
    });

    let fixed_summary = validate_project(&project, &defs);
    assert!(fixed_summary.is_valid);
    assert!(!fixed_summary.results.iter().any(|r| r.code == "VLD-NAV01"));
}

#[test]
fn test_validate_display_duplicate_id_and_placement_warnings() {
    let defs = make_sample_definitions();
    let mut project = Project::new("Test Plant Displays Duplicate Check");

    let mut plant = Plant::new("Plant 1");
    let mut area = Area::new("Area 1");
    let mut unit = Unit::new("Unit 1");

    let pump_def = defs.values().next().unwrap();
    let p101_id = EquipmentId::new();
    unit.equipment.push(EquipmentInstance {
        id: p101_id,
        definition_id: pump_def.id,
        definition_version: pump_def.version,
        name: "P-101".to_string(),
        parameters: IndexMap::new(),
        position: Some(Position2D { x: 100.0, y: 100.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });
    area.units.push(unit);
    plant.areas.push(area);
    project.plants.push(plant);

    // Add display with valid placement
    project.displays.push(nebudcs_model::project::ProjectDisplay {
        id: "disp-main".to_string(),
        name: "Main Display".to_string(),
        display_type: "plant_overview".to_string(),
        area_id: None,
        system_id: None,
        subsystem_id: None,
        unit_id: None,
        equipment_ids: vec![p101_id.to_string()],
        instrument_ids: Vec::new(),
        stream_ids: Vec::new(),
        viewport_x: None,
        viewport_y: None,
        viewport_width: None,
        viewport_height: None,
        viewport_zoom: None,
        is_default: true,
        order: Some(1),
        description: None,
        placements: vec![nebudcs_model::project::DisplayElementPlacement {
            equipment_id: p101_id.to_string(),
            x: 100.0,
            y: 200.0,
            width: Some(120.0),
            height: Some(80.0),
            rotation: Some(0.0),
        }],
    });

    // 1. Add a second display with duplicate ID -> triggers VLD-DSP02 error
    project.displays.push(nebudcs_model::project::ProjectDisplay {
        id: "disp-main".to_string(),
        name: "Duplicate Main Display".to_string(),
        display_type: "system_overview".to_string(),
        area_id: None,
        system_id: None,
        subsystem_id: None,
        unit_id: None,
        equipment_ids: Vec::new(),
        instrument_ids: Vec::new(),
        stream_ids: Vec::new(),
        viewport_x: None,
        viewport_y: None,
        viewport_width: None,
        viewport_height: None,
        viewport_zoom: None,
        is_default: false,
        order: Some(2),
        description: None,
        placements: vec![nebudcs_model::project::DisplayElementPlacement {
            equipment_id: "eq-nonexistent".to_string(),
            x: 300.0,
            y: 400.0,
            width: None,
            height: None,
            rotation: None,
        }],
    });

    let summary = validate_project(&project, &defs);
    assert!(!summary.is_valid);
    assert!(summary.results.iter().any(|r| r.code == "VLD-DSP02" && r.severity == nebudcs_model::validation::ValidationSeverity::Error));
    assert!(summary.results.iter().any(|r| r.code == "VLD-DSP04" && r.severity == nebudcs_model::validation::ValidationSeverity::Warning));
}
