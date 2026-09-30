//! Authoritative Naphtha Hydrotreater (NHT) Reference Plant Seed.
//!
//! Benchmark plant model based on HPCL Visakh Naphtha Hydrotreater Operating Manual
//! and EIL engineering design standards.

use indexmap::IndexMap;
use nebudcs_model::equipment::{AlarmPriority, AlarmType, EquipmentInstance, Position2D};
use nebudcs_model::project::{
    AlarmInstance, Area, BoundaryConditionInstance, ControlLoopInstance, DisplayElementPlacement,
    InitialConditionInstance, InstrumentInstance, InterlockInstance, Plant, Project,
    ProjectDisplay, ProjectMetadata, ProjectNavigationLink, ProjectPlantArea, ProjectPlantStructure,
    ProjectPlantSubsystem, ProjectPlantSystem, ProjectPlantUnit, Unit,
};
use nebudcs_model::stream::{StreamDefinition, StreamType};
use nebudcs_types::{
    EntityId, EquipmentDefinitionId, EquipmentId, PortId, ProjectId, SchemaVersion,
};

pub const PROJ_NHT_REFERENCE: &str = "proj-domain-nht-reference";

/// Builds the authoritative Naphtha Hydrotreater reference plant.
pub fn build_nht_reference_project() -> Project {
    let project_id = PROJ_NHT_REFERENCE
        .parse::<ProjectId>()
        .unwrap_or_else(|_| ProjectId::new());

    let mut project = Project::new("Naphtha Hydrotreater (NHT) Reference Plant");
    project.id = project_id;
    project.metadata = ProjectMetadata {
        description: "Industrial 30,033 BPSD Naphtha Hydrotreater reference plant reconstruction based on HPCL Visakh NHT Operating Manual and EIL engineering standards. Features 7-shell Combined Feed Exchanger train (72-E-01A..G), Charge Heater (72-F-01), Fixed-Bed Catalytic Reactor (72-R-01), Recycle Gas Compressor (72-K-01A/B), Wash Water Injection System, Stripper (72-C-01), and Naphtha Splitter (72-C-02) with full multi-display operational hierarchy and cross-display navigation.".to_string(),
        author: "NebuDCS Engineering Reference Library (HPCL/EIL Benchmark)".to_string(),
        created: "2026-09-09T00:00:00Z".to_string(),
        modified: "2026-09-09T00:00:00Z".to_string(),
    };

    let mut plant = Plant::new("Refinery Complex Area 72");
    let mut area = Area::new("Hydroprocessing Section");
    let mut unit = Unit::new("Unit 720 Naphtha Hydrotreater");

    // Standard definition identifiers
    let tank_def = "standard.tank.atmospheric".parse::<EquipmentDefinitionId>().unwrap();
    let pump_def = "standard.pump.centrifugal".parse::<EquipmentDefinitionId>().unwrap();
    let valve_def = "standard.valve.control_globe".parse::<EquipmentDefinitionId>().unwrap();
    let hx_def = "standard.heat_exchanger.shell_and_tube".parse::<EquipmentDefinitionId>().unwrap();
    let heater_def = "standard.heater.fired_process".parse::<EquipmentDefinitionId>().unwrap();
    let vessel_def = "standard.vessel.vertical_separator".parse::<EquipmentDefinitionId>().unwrap();
    let comp_def = "standard.compressor.centrifugal".parse::<EquipmentDefinitionId>().unwrap();
    let mixer_def = "standard.mixer.static_inline".parse::<EquipmentDefinitionId>().unwrap();
    let splitter_def = "standard.splitter.flow_splitter".parse::<EquipmentDefinitionId>().unwrap();
    let reactor_def = "standard.reactor.fixed_bed".parse::<EquipmentDefinitionId>().unwrap();

    // Standard port identifiers
    let inlet_port = "inlet".parse::<PortId>().unwrap();
    let outlet_port = "outlet".parse::<PortId>().unwrap();
    let suction_port = "suction".parse::<PortId>().unwrap();
    let discharge_port = "discharge".parse::<PortId>().unwrap();
    let tube_in_port = "tube_in".parse::<PortId>().unwrap();
    let tube_out_port = "tube_out".parse::<PortId>().unwrap();
    let shell_in_port = "shell_in".parse::<PortId>().unwrap();
    let shell_out_port = "shell_out".parse::<PortId>().unwrap();
    let process_in_port = "process_in".parse::<PortId>().unwrap();
    let process_out_port = "process_out".parse::<PortId>().unwrap();
    let feed_port = "feed".parse::<PortId>().unwrap();
    let effluent_port = "effluent".parse::<PortId>().unwrap();
    let gas_out_port = "gas_out".parse::<PortId>().unwrap();
    let liquid_out_port = "liquid_out".parse::<PortId>().unwrap();
    let primary_in_port = "primary_in".parse::<PortId>().unwrap();
    let secondary_in_port = "secondary_in".parse::<PortId>().unwrap();
    let mixture_out_port = "mixture_out".parse::<PortId>().unwrap();
    let feed_in_port = "feed_in".parse::<PortId>().unwrap();
    let branch_a_port = "branch_a".parse::<PortId>().unwrap();
    let branch_b_port = "branch_b".parse::<PortId>().unwrap();

    // Equipment IDs
    let tk101_id = EquipmentId::new();
    let v101_id = EquipmentId::new();
    let p101a_id = EquipmentId::new();
    let p101b_id = EquipmentId::new();
    let fv101_id = EquipmentId::new();
    let x101_id = EquipmentId::new();
    let mix101_id = EquipmentId::new();

    let e101a_id = EquipmentId::new();
    let e101b_id = EquipmentId::new();
    let e101c_id = EquipmentId::new();
    let e101d_id = EquipmentId::new();
    let e101e_id = EquipmentId::new();
    let e101f_id = EquipmentId::new();
    let e101g_id = EquipmentId::new();
    let f101_id = EquipmentId::new();
    let r101_id = EquipmentId::new();

    let mix102_id = EquipmentId::new();
    let a101_id = EquipmentId::new();
    let e101trim_id = EquipmentId::new();
    let v102_id = EquipmentId::new();
    let v103_id = EquipmentId::new();
    let k101a_id = EquipmentId::new();
    let split101_id = EquipmentId::new();
    let pv102_id = EquipmentId::new();

    let v104_id = EquipmentId::new();
    let p102a_id = EquipmentId::new();
    let fv103_id = EquipmentId::new();
    let lv101_id = EquipmentId::new();

    let e102a_id = EquipmentId::new();
    let e102b_id = EquipmentId::new();
    let c101_id = EquipmentId::new();
    let e103_id = EquipmentId::new();
    let v105_id = EquipmentId::new();
    let p104a_id = EquipmentId::new();
    let p103a_id = EquipmentId::new();

    let c102_id = EquipmentId::new();
    let e104_id = EquipmentId::new();
    let v106_id = EquipmentId::new();
    let p105a_id = EquipmentId::new();
    let e105_id = EquipmentId::new();
    let e106_id = EquipmentId::new();

    // 1. Equipment Instances with HPCL / EIL Reference Parameters
    // Feed Tank TK-101
    let mut p = IndexMap::new();
    p.insert("capacity_m3".to_string(), 10000.0);
    p.insert("level".to_string(), 72.5);
    unit.equipment.push(EquipmentInstance {
        id: tk101_id,
        definition_id: tank_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "TK-101".to_string(),
        parameters: p,
        position: Some(Position2D { x: 80.0, y: 360.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Surge Drum V-101
    let mut p = IndexMap::new();
    p.insert("volume_m3".to_string(), 45.0);
    p.insert("operating_pressure".to_string(), 350000.0);
    p.insert("level".to_string(), 50.0);
    unit.equipment.push(EquipmentInstance {
        id: v101_id,
        definition_id: vessel_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "V-101".to_string(),
        parameters: p,
        position: Some(Position2D { x: 200.0, y: 360.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Charge Pumps P-101A/B (199 m3/h = 30,033 BPSD, HPCL Visakh Manual)
    let mut p = IndexMap::new();
    p.insert("rated_flow".to_string(), 199.0);
    p.insert("rated_head".to_string(), 2800000.0);
    p.insert("speed_rpm".to_string(), 2980.0);
    unit.equipment.push(EquipmentInstance {
        id: p101a_id,
        definition_id: pump_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "P-101A".to_string(),
        parameters: p,
        position: Some(Position2D { x: 320.0, y: 360.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    let mut p = IndexMap::new();
    p.insert("rated_flow".to_string(), 199.0);
    p.insert("rated_head".to_string(), 2800000.0);
    p.insert("speed_rpm".to_string(), 0.0);
    unit.equipment.push(EquipmentInstance {
        id: p101b_id,
        definition_id: pump_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "P-101B".to_string(),
        parameters: p,
        position: Some(Position2D { x: 320.0, y: 480.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Feed Valve FV-101
    let mut p = IndexMap::new();
    p.insert("cv".to_string(), 250.0);
    p.insert("travel".to_string(), 0.65);
    unit.equipment.push(EquipmentInstance {
        id: fv101_id,
        definition_id: valve_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "FV-101".to_string(),
        parameters: p,
        position: Some(Position2D { x: 440.0, y: 360.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Feed Filter X-101
    let mut p = IndexMap::new();
    p.insert("volume_m3".to_string(), 8.5);
    p.insert("operating_pressure".to_string(), 2700000.0);
    unit.equipment.push(EquipmentInstance {
        id: x101_id,
        definition_id: vessel_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "X-101".to_string(),
        parameters: p,
        position: Some(Position2D { x: 560.0, y: 360.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Mix Tee MIX-101 (Naphtha + H2 Treat Gas)
    let mut p = IndexMap::new();
    p.insert("pressure_loss_factor".to_string(), 15000.0);
    unit.equipment.push(EquipmentInstance {
        id: mix101_id,
        definition_id: mixer_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "MIX-101".to_string(),
        parameters: p,
        position: Some(Position2D { x: 680.0, y: 360.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // 7 Combined Feed Exchangers E-101A through E-101G (Cold 79 -> 287 C, Hot 343 -> 128 C, HPCL Visakh Manual)
    let cfe_names = ["E-101A", "E-101B", "E-101C", "E-101D", "E-101E", "E-101F", "E-101G"];
    let cfe_ids = [e101a_id, e101b_id, e101c_id, e101d_id, e101e_id, e101f_id, e101g_id];
    for (i, (&eid, name)) in cfe_ids.iter().zip(cfe_names.iter()).enumerate() {
        let mut p = IndexMap::new();
        p.insert("area_m2".to_string(), 420.0);
        p.insert("overall_u".to_string(), 380.0);
        p.insert("design_duty".to_string(), 3200000.0);
        unit.equipment.push(EquipmentInstance {
            id: eid,
            definition_id: hx_def,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: name.to_string(),
            parameters: p,
            position: Some(Position2D { x: 800.0 + (i as f64) * 110.0, y: 360.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });
    }

    // Charge Heater F-101 (Raises 287 C to 332 C, 4.8 MW Duty)
    let mut p = IndexMap::new();
    p.insert("duty_w".to_string(), 4800000.0);
    p.insert("efficiency".to_string(), 0.88);
    unit.equipment.push(EquipmentInstance {
        id: f101_id,
        definition_id: heater_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "F-101".to_string(),
        parameters: p,
        position: Some(Position2D { x: 1600.0, y: 360.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Fixed-Bed Catalytic Reactor R-101 (Inlet 332 C, 24.9 kg/cm2g; Outlet 343 C, 24.4 kg/cm2g; Catalyst 19.9 m3, 15,503 kg, LHSV 10 h-1)
    let mut p = IndexMap::new();
    p.insert("catalyst_volume_m3".to_string(), 19.9);
    p.insert("catalyst_mass_kg".to_string(), 15503.0);
    p.insert("lhsv".to_string(), 10.0);
    p.insert("operating_pressure".to_string(), 2442000.0);
    p.insert("operating_temp".to_string(), 332.0);
    p.insert("max_bed_delta_t".to_string(), 10.0);
    unit.equipment.push(EquipmentInstance {
        id: r101_id,
        definition_id: reactor_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "R-101".to_string(),
        parameters: p,
        position: Some(Position2D { x: 1740.0, y: 360.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Wash Water Injection Tee MIX-102
    let mut p = IndexMap::new();
    p.insert("pressure_loss_factor".to_string(), 5000.0);
    unit.equipment.push(EquipmentInstance {
        id: mix102_id,
        definition_id: mixer_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "MIX-102".to_string(),
        parameters: p,
        position: Some(Position2D { x: 800.0, y: 600.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Effluent Air Cooler A-101 (Condenser, cools to 55 C)
    let mut p = IndexMap::new();
    p.insert("area_m2".to_string(), 1250.0);
    p.insert("overall_u".to_string(), 420.0);
    p.insert("design_duty".to_string(), 5800000.0);
    unit.equipment.push(EquipmentInstance {
        id: a101_id,
        definition_id: hx_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "A-101".to_string(),
        parameters: p,
        position: Some(Position2D { x: 940.0, y: 600.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Trim Cooler E-101_TRIM (Cools 55 to 45 C)
    let mut p = IndexMap::new();
    p.insert("area_m2".to_string(), 180.0);
    p.insert("overall_u".to_string(), 550.0);
    p.insert("design_duty".to_string(), 900000.0);
    unit.equipment.push(EquipmentInstance {
        id: e101trim_id,
        definition_id: hx_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "E-101_TRIM".to_string(),
        parameters: p,
        position: Some(Position2D { x: 1080.0, y: 600.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // High Pressure Separator V-102 (21.1 kg/cm2g = 2.07 MPa, 45 C, HPCL Visakh Manual)
    let mut p = IndexMap::new();
    p.insert("volume_m3".to_string(), 42.0);
    p.insert("operating_pressure".to_string(), 2069000.0);
    p.insert("level".to_string(), 0.50);
    unit.equipment.push(EquipmentInstance {
        id: v102_id,
        definition_id: vessel_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "V-102".to_string(),
        parameters: p,
        position: Some(Position2D { x: 1220.0, y: 600.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Recycle Gas Knock-Out Drum V-103
    let mut p = IndexMap::new();
    p.insert("volume_m3".to_string(), 22.0);
    p.insert("operating_pressure".to_string(), 2060000.0);
    p.insert("level".to_string(), 0.30);
    unit.equipment.push(EquipmentInstance {
        id: v103_id,
        definition_id: vessel_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "V-103".to_string(),
        parameters: p,
        position: Some(Position2D { x: 1220.0, y: 760.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Recycle Gas Compressor K-101A (Suction 21.1 kg/cm2g, 55 C; Discharge 36.3 kg/cm2g, 108 C; Flow 39,302 Nm3/h; MW 4.75, HPCL Visakh Manual)
    let mut p = IndexMap::new();
    p.insert("rated_flow".to_string(), 14.5);
    p.insert("rated_mass_flow".to_string(), 14.5);
    p.insert("speed_rpm".to_string(), 10450.0);
    p.insert("rated_power".to_string(), 680000.0);
    unit.equipment.push(EquipmentInstance {
        id: k101a_id,
        definition_id: comp_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "K-101A".to_string(),
        parameters: p,
        position: Some(Position2D { x: 1380.0, y: 760.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Recycle/Purge Splitter SPLIT-101
    let mut p = IndexMap::new();
    p.insert("max_flow".to_string(), 50.0);
    unit.equipment.push(EquipmentInstance {
        id: split101_id,
        definition_id: splitter_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "SPLIT-101".to_string(),
        parameters: p,
        position: Some(Position2D { x: 1520.0, y: 760.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Purge Gas Valve PV-102
    let mut p = IndexMap::new();
    p.insert("cv".to_string(), 65.0);
    p.insert("travel".to_string(), 0.25);
    unit.equipment.push(EquipmentInstance {
        id: pv102_id,
        definition_id: valve_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "PV-102".to_string(),
        parameters: p,
        position: Some(Position2D { x: 1660.0, y: 760.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Wash Water Break Tank V-104 (3 vol% = ~6 m3/h)
    let mut p = IndexMap::new();
    p.insert("capacity_m3".to_string(), 80.0);
    p.insert("level".to_string(), 65.0);
    unit.equipment.push(EquipmentInstance {
        id: v104_id,
        definition_id: tank_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "V-104".to_string(),
        parameters: p,
        position: Some(Position2D { x: 440.0, y: 740.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Wash Water Injection Pump P-102A
    let mut p = IndexMap::new();
    p.insert("rated_flow".to_string(), 6.0);
    p.insert("rated_head".to_string(), 2800000.0);
    p.insert("speed_rpm".to_string(), 2950.0);
    unit.equipment.push(EquipmentInstance {
        id: p102a_id,
        definition_id: pump_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "P-102A".to_string(),
        parameters: p,
        position: Some(Position2D { x: 560.0, y: 740.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Wash Water Control Valve FV-103
    let mut p = IndexMap::new();
    p.insert("cv".to_string(), 15.0);
    p.insert("travel".to_string(), 0.50);
    unit.equipment.push(EquipmentInstance {
        id: fv103_id,
        definition_id: valve_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "FV-103".to_string(),
        parameters: p,
        position: Some(Position2D { x: 680.0, y: 740.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // HP Separator Level Valve LV-101
    let mut p = IndexMap::new();
    p.insert("cv".to_string(), 180.0);
    p.insert("travel".to_string(), 0.48);
    unit.equipment.push(EquipmentInstance {
        id: lv101_id,
        definition_id: valve_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "LV-101".to_string(),
        parameters: p,
        position: Some(Position2D { x: 1360.0, y: 600.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Stripper Feed/Bottoms Exchangers E-102A/B
    let mut p = IndexMap::new();
    p.insert("area_m2".to_string(), 240.0);
    p.insert("overall_u".to_string(), 450.0);
    p.insert("design_duty".to_string(), 1600000.0);
    unit.equipment.push(EquipmentInstance {
        id: e102a_id,
        definition_id: hx_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "E-102A".to_string(),
        parameters: p,
        position: Some(Position2D { x: 1480.0, y: 600.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    let mut p = IndexMap::new();
    p.insert("area_m2".to_string(), 240.0);
    p.insert("overall_u".to_string(), 450.0);
    p.insert("design_duty".to_string(), 1600000.0);
    unit.equipment.push(EquipmentInstance {
        id: e102b_id,
        definition_id: hx_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "E-102B".to_string(),
        parameters: p,
        position: Some(Position2D { x: 1600.0, y: 600.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Stripper Column C-101 (8.5 barg, bottoms 215 C, top 95 C)
    let mut p = IndexMap::new();
    p.insert("volume_m3".to_string(), 65.0);
    p.insert("operating_pressure".to_string(), 850000.0);
    p.insert("level".to_string(), 50.0);
    unit.equipment.push(EquipmentInstance {
        id: c101_id,
        definition_id: vessel_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "C-101".to_string(),
        parameters: p,
        position: Some(Position2D { x: 1740.0, y: 600.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Stripper Overhead Condenser E-103
    let mut p = IndexMap::new();
    p.insert("area_m2".to_string(), 310.0);
    p.insert("overall_u".to_string(), 520.0);
    p.insert("design_duty".to_string(), 2200000.0);
    unit.equipment.push(EquipmentInstance {
        id: e103_id,
        definition_id: hx_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "E-103".to_string(),
        parameters: p,
        position: Some(Position2D { x: 1880.0, y: 520.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Stripper Receiver V-105
    let mut p = IndexMap::new();
    p.insert("volume_m3".to_string(), 20.0);
    p.insert("operating_pressure".to_string(), 750000.0);
    p.insert("level".to_string(), 45.0);
    unit.equipment.push(EquipmentInstance {
        id: v105_id,
        definition_id: vessel_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "V-105".to_string(),
        parameters: p,
        position: Some(Position2D { x: 2000.0, y: 520.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Stripper Reflux Pump P-104A
    let mut p = IndexMap::new();
    p.insert("rated_flow".to_string(), 25.0);
    p.insert("rated_head".to_string(), 350000.0);
    p.insert("speed_rpm".to_string(), 2950.0);
    unit.equipment.push(EquipmentInstance {
        id: p104a_id,
        definition_id: pump_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "P-104A".to_string(),
        parameters: p,
        position: Some(Position2D { x: 2120.0, y: 520.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Stripper Bottoms Pump P-103A
    let mut p = IndexMap::new();
    p.insert("rated_flow".to_string(), 185.0);
    p.insert("rated_head".to_string(), 850000.0);
    p.insert("speed_rpm".to_string(), 2950.0);
    unit.equipment.push(EquipmentInstance {
        id: p103a_id,
        definition_id: pump_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "P-103A".to_string(),
        parameters: p,
        position: Some(Position2D { x: 1880.0, y: 680.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Naphtha Splitter Column C-102 (2.5 barg, bottoms 175 C, top 85 C)
    let mut p = IndexMap::new();
    p.insert("volume_m3".to_string(), 90.0);
    p.insert("operating_pressure".to_string(), 250000.0);
    p.insert("level".to_string(), 50.0);
    unit.equipment.push(EquipmentInstance {
        id: c102_id,
        definition_id: vessel_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "C-102".to_string(),
        parameters: p,
        position: Some(Position2D { x: 2020.0, y: 680.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Splitter Overhead Condenser E-104
    let mut p = IndexMap::new();
    p.insert("area_m2".to_string(), 280.0);
    p.insert("overall_u".to_string(), 480.0);
    p.insert("design_duty".to_string(), 1900000.0);
    unit.equipment.push(EquipmentInstance {
        id: e104_id,
        definition_id: hx_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "E-104".to_string(),
        parameters: p,
        position: Some(Position2D { x: 2160.0, y: 620.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Splitter Reflux Drum V-106
    let mut p = IndexMap::new();
    p.insert("volume_m3".to_string(), 18.0);
    p.insert("operating_pressure".to_string(), 200000.0);
    p.insert("level".to_string(), 45.0);
    unit.equipment.push(EquipmentInstance {
        id: v106_id,
        definition_id: vessel_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "V-106".to_string(),
        parameters: p,
        position: Some(Position2D { x: 2280.0, y: 620.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Light Naphtha / Splitter Reflux Pump P-105A
    let mut p = IndexMap::new();
    p.insert("rated_flow".to_string(), 45.0);
    p.insert("rated_head".to_string(), 320000.0);
    p.insert("speed_rpm".to_string(), 2950.0);
    unit.equipment.push(EquipmentInstance {
        id: p105a_id,
        definition_id: pump_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "P-105A".to_string(),
        parameters: p,
        position: Some(Position2D { x: 2400.0, y: 620.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Light Naphtha Product Cooler E-105
    let mut p = IndexMap::new();
    p.insert("area_m2".to_string(), 85.0);
    p.insert("overall_u".to_string(), 450.0);
    p.insert("design_duty".to_string(), 650000.0);
    unit.equipment.push(EquipmentInstance {
        id: e105_id,
        definition_id: hx_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "E-105".to_string(),
        parameters: p,
        position: Some(Position2D { x: 2520.0, y: 620.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Heavy Naphtha Product Cooler E-106
    let mut p = IndexMap::new();
    p.insert("area_m2".to_string(), 160.0);
    p.insert("overall_u".to_string(), 480.0);
    p.insert("design_duty".to_string(), 1400000.0);
    unit.equipment.push(EquipmentInstance {
        id: e106_id,
        definition_id: hx_def,
        definition_version: SchemaVersion::new(1, 0, 0),
        name: "E-106".to_string(),
        parameters: p,
        position: Some(Position2D { x: 2160.0, y: 780.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // 2. Continuous Process Streams (49 Streams connecting physical equipment)
    // Fresh Feed Path
    unit.streams.push(StreamDefinition::new("STR-NHT-00", StreamType::Material, tk101_id, outlet_port, v101_id, feed_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-01", StreamType::Material, v101_id, liquid_out_port, p101a_id, suction_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-02", StreamType::Material, p101a_id, discharge_port, fv101_id, inlet_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-03", StreamType::Material, fv101_id, outlet_port, x101_id, feed_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-04", StreamType::Material, x101_id, liquid_out_port, mix101_id, primary_in_port));

    // Combined Feed Exchanger Train (Cold Tube Pass 1..7)
    unit.streams.push(StreamDefinition::new("STR-NHT-05", StreamType::Material, mix101_id, mixture_out_port, e101a_id, tube_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-06", StreamType::Material, e101a_id, tube_out_port, e101b_id, tube_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-07", StreamType::Material, e101b_id, tube_out_port, e101c_id, tube_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-08", StreamType::Material, e101c_id, tube_out_port, e101d_id, tube_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-09", StreamType::Material, e101d_id, tube_out_port, e101e_id, tube_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-10", StreamType::Material, e101e_id, tube_out_port, e101f_id, tube_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-11", StreamType::Material, e101f_id, tube_out_port, e101g_id, tube_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-12", StreamType::Material, e101g_id, tube_out_port, f101_id, process_in_port));

    // Furnace & Reactor
    unit.streams.push(StreamDefinition::new("STR-NHT-13", StreamType::Material, f101_id, process_out_port, r101_id, feed_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-14", StreamType::Material, r101_id, effluent_port, e101g_id, shell_in_port));

    // Combined Feed Exchanger Train (Hot Shell Pass 7..1 counter-current)
    unit.streams.push(StreamDefinition::new("STR-NHT-15", StreamType::Material, e101g_id, shell_out_port, e101f_id, shell_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-16", StreamType::Material, e101f_id, shell_out_port, e101e_id, shell_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-17", StreamType::Material, e101e_id, shell_out_port, e101d_id, shell_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-18", StreamType::Material, e101d_id, shell_out_port, e101c_id, shell_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-19", StreamType::Material, e101c_id, shell_out_port, e101b_id, shell_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-20", StreamType::Material, e101b_id, shell_out_port, e101a_id, shell_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-21", StreamType::Material, e101a_id, shell_out_port, mix102_id, primary_in_port));

    // Wash Water Injection Circuit
    unit.streams.push(StreamDefinition::new("STR-NHT-22", StreamType::Material, v104_id, outlet_port, p102a_id, suction_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-23", StreamType::Material, p102a_id, discharge_port, fv103_id, inlet_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-24", StreamType::Material, fv103_id, outlet_port, mix102_id, secondary_in_port));

    // Effluent Condensation & High Pressure Separation
    unit.streams.push(StreamDefinition::new("STR-NHT-25", StreamType::Material, mix102_id, mixture_out_port, a101_id, tube_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-26", StreamType::Material, a101_id, tube_out_port, e101trim_id, tube_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-27", StreamType::Material, e101trim_id, tube_out_port, v102_id, feed_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-28", StreamType::Material, v102_id, gas_out_port, v103_id, feed_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-29", StreamType::Material, v103_id, gas_out_port, k101a_id, suction_port));

    // Recycle Compressor Discharge & Purge Gas Split
    unit.streams.push(StreamDefinition::new("STR-NHT-30", StreamType::Material, k101a_id, discharge_port, split101_id, feed_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-31", StreamType::Material, split101_id, branch_a_port, mix101_id, secondary_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-32", StreamType::Material, split101_id, branch_b_port, pv102_id, inlet_port));

    // Stripper Feed Loop (Exchanger Preheat & Debutanization)
    unit.streams.push(StreamDefinition::new("STR-NHT-33", StreamType::Material, v102_id, liquid_out_port, lv101_id, inlet_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-34", StreamType::Material, lv101_id, outlet_port, e102a_id, tube_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-35", StreamType::Material, e102a_id, tube_out_port, e102b_id, tube_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-36", StreamType::Material, e102b_id, tube_out_port, c101_id, feed_port));

    // Stripper Overhead Condensation & Reflux
    unit.streams.push(StreamDefinition::new("STR-NHT-37", StreamType::Material, c101_id, gas_out_port, e103_id, tube_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-38", StreamType::Material, e103_id, tube_out_port, v105_id, feed_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-39", StreamType::Material, v105_id, liquid_out_port, p104a_id, suction_port));

    // Stripper Bottoms & Heat Recovery into Splitter
    unit.streams.push(StreamDefinition::new("STR-NHT-40", StreamType::Material, c101_id, liquid_out_port, p103a_id, suction_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-41", StreamType::Material, p103a_id, discharge_port, e102b_id, shell_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-42", StreamType::Material, e102b_id, shell_out_port, e102a_id, shell_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-43", StreamType::Material, e102a_id, shell_out_port, c102_id, feed_port));

    // Naphtha Splitter Overhead & Products
    unit.streams.push(StreamDefinition::new("STR-NHT-44", StreamType::Material, c102_id, gas_out_port, e104_id, tube_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-45", StreamType::Material, e104_id, tube_out_port, v106_id, feed_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-46", StreamType::Material, v106_id, liquid_out_port, p105a_id, suction_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-47", StreamType::Material, p105a_id, discharge_port, e105_id, tube_in_port));
    unit.streams.push(StreamDefinition::new("STR-NHT-48", StreamType::Material, c102_id, liquid_out_port, e106_id, tube_in_port));

    // 3. Instruments & Transmitters (12 Key Instruments)
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "FT-101".to_string(),
        equipment_id: Some(fv101_id),
        port_id: Some(inlet_port),
        measurement_kind: "Flow".to_string(),
        range_min: 0.0,
        range_max: 250.0,
        unit: "m3/h".to_string(),
        position: Some(Position2D { x: 440.0, y: 300.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "TT-101".to_string(),
        equipment_id: Some(f101_id),
        port_id: Some(process_out_port),
        measurement_kind: "Temperature".to_string(),
        range_min: 0.0,
        range_max: 450.0,
        unit: "°C".to_string(),
        position: Some(Position2D { x: 1720.0, y: 330.0 }),
    });
    // Comprehensive F-101 Charge Heater (Dapur) Instrumentation
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "TI-101-CIT".to_string(),
        equipment_id: Some(f101_id),
        port_id: Some(process_in_port),
        measurement_kind: "Temperature".to_string(),
        range_min: 0.0,
        range_max: 450.0,
        unit: "°C".to_string(),
        position: Some(Position2D { x: 1480.0, y: 310.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "TI-101-SK1".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Temperature".to_string(),
        range_min: 0.0,
        range_max: 600.0,
        unit: "°C".to_string(),
        position: Some(Position2D { x: 1550.0, y: 440.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "TI-101-SK2".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Temperature".to_string(),
        range_min: 0.0,
        range_max: 600.0,
        unit: "°C".to_string(),
        position: Some(Position2D { x: 1650.0, y: 440.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "PT-104".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Pressure".to_string(),
        range_min: 0.0,
        range_max: 10.0,
        unit: "kg/cm2g".to_string(),
        position: Some(Position2D { x: 1600.0, y: 510.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "FT-105".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Flow".to_string(),
        range_min: 0.0,
        range_max: 3000.0,
        unit: "Nm3/h".to_string(),
        position: Some(Position2D { x: 1660.0, y: 510.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "PT-105".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Pressure".to_string(),
        range_min: 0.0,
        range_max: 15.0,
        unit: "kg/cm2g".to_string(),
        position: Some(Position2D { x: 1720.0, y: 510.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "PDT-101".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "DifferentialPressure".to_string(),
        range_min: -20.0,
        range_max: 10.0,
        unit: "mmH2O".to_string(),
        position: Some(Position2D { x: 1480.0, y: 370.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "TI-104".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Temperature".to_string(),
        range_min: 0.0,
        range_max: 400.0,
        unit: "°C".to_string(),
        position: Some(Position2D { x: 1540.0, y: 200.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "AI-101".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Concentration".to_string(),
        range_min: 0.0,
        range_max: 10.0,
        unit: "%".to_string(),
        position: Some(Position2D { x: 1660.0, y: 200.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "TI-101-RIT".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Temperature".to_string(),
        range_min: 0.0,
        range_max: 450.0,
        unit: "°C".to_string(),
        position: Some(Position2D { x: 1530.0, y: 400.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "TI-101-ROT".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Temperature".to_string(),
        range_min: 0.0,
        range_max: 450.0,
        unit: "°C".to_string(),
        position: Some(Position2D { x: 1670.0, y: 400.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "FT-Air".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Flow".to_string(),
        range_min: 0.0,
        range_max: 30000.0,
        unit: "Nm3/h".to_string(),
        position: Some(Position2D { x: 1360.0, y: 510.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "PT-Air".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Pressure".to_string(),
        range_min: 0.0,
        range_max: 50.0,
        unit: "mbar".to_string(),
        position: Some(Position2D { x: 1420.0, y: 510.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "FZ-101".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Position".to_string(),
        range_min: 0.0,
        range_max: 100.0,
        unit: "%".to_string(),
        position: Some(Position2D { x: 1480.0, y: 510.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "FFC-101".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Ratio".to_string(),
        range_min: 0.0,
        range_max: 3.0,
        unit: "ratio".to_string(),
        position: Some(Position2D { x: 1540.0, y: 510.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "PT-107".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Pressure".to_string(),
        range_min: 0.0,
        range_max: 15.0,
        unit: "kg/cm2g".to_string(),
        position: Some(Position2D { x: 1780.0, y: 510.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "FT-107".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Flow".to_string(),
        range_min: 0.0,
        range_max: 500.0,
        unit: "kg/h".to_string(),
        position: Some(Position2D { x: 1840.0, y: 510.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "AI-102".to_string(),
        equipment_id: Some(f101_id),
        port_id: None,
        measurement_kind: "Concentration".to_string(),
        range_min: 0.0,
        range_max: 100.0,
        unit: "ppm".to_string(),
        position: Some(Position2D { x: 1660.0, y: 150.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "TT-102".to_string(),
        equipment_id: Some(r101_id),
        port_id: Some(effluent_port),
        measurement_kind: "Temperature".to_string(),
        range_min: 0.0,
        range_max: 450.0,
        unit: "°C".to_string(),
        position: Some(Position2D { x: 1740.0, y: 300.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "PT-102".to_string(),
        equipment_id: Some(v102_id),
        port_id: Some(gas_out_port),
        measurement_kind: "Pressure".to_string(),
        range_min: 0.0,
        range_max: 35.0,
        unit: "kg/cm2g".to_string(),
        position: Some(Position2D { x: 1220.0, y: 520.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "LT-101".to_string(),
        equipment_id: Some(v102_id),
        port_id: None,
        measurement_kind: "Level".to_string(),
        range_min: 0.0,
        range_max: 1.0,
        unit: "fraction".to_string(),
        position: Some(Position2D { x: 1220.0, y: 660.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "FT-102".to_string(),
        equipment_id: Some(k101a_id),
        port_id: Some(discharge_port),
        measurement_kind: "Flow".to_string(),
        range_min: 0.0,
        range_max: 60000.0,
        unit: "Nm3/h".to_string(),
        position: Some(Position2D { x: 1380.0, y: 700.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "FT-103".to_string(),
        equipment_id: Some(fv103_id),
        port_id: Some(inlet_port),
        measurement_kind: "Flow".to_string(),
        range_min: 0.0,
        range_max: 15.0,
        unit: "m3/h".to_string(),
        position: Some(Position2D { x: 680.0, y: 700.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "LT-102".to_string(),
        equipment_id: Some(c101_id),
        port_id: None,
        measurement_kind: "Level".to_string(),
        range_min: 0.0,
        range_max: 100.0,
        unit: "%".to_string(),
        position: Some(Position2D { x: 1740.0, y: 660.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "PT-103".to_string(),
        equipment_id: Some(v105_id),
        port_id: Some(gas_out_port),
        measurement_kind: "Pressure".to_string(),
        range_min: 0.0,
        range_max: 15.0,
        unit: "kg/cm2g".to_string(),
        position: Some(Position2D { x: 2000.0, y: 460.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "FT-104".to_string(),
        equipment_id: Some(p104a_id),
        port_id: Some(discharge_port),
        measurement_kind: "Flow".to_string(),
        range_min: 0.0,
        range_max: 50.0,
        unit: "m3/h".to_string(),
        position: Some(Position2D { x: 2120.0, y: 460.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "LT-103".to_string(),
        equipment_id: Some(c102_id),
        port_id: None,
        measurement_kind: "Level".to_string(),
        range_min: 0.0,
        range_max: 100.0,
        unit: "%".to_string(),
        position: Some(Position2D { x: 2020.0, y: 740.0 }),
    });
    unit.instruments.push(InstrumentInstance {
        id: EntityId::new(),
        tag: "TT-103".to_string(),
        equipment_id: Some(c102_id),
        port_id: Some(gas_out_port),
        measurement_kind: "Temperature".to_string(),
        range_min: 0.0,
        range_max: 200.0,
        unit: "°C".to_string(),
        position: Some(Position2D { x: 2020.0, y: 620.0 }),
    });

    // 4. Control Loops (12 DCS Loops)
    unit.control_loops.push(ControlLoopInstance {
        id: EntityId::new(),
        name: "FIC-101".to_string(),
        pv_tag: "FT-101".to_string(),
        sp: 199.0,
        out_tag: "FV-101.OPENING".to_string(),
        kp: 0.8,
        ti: 6.0,
        td: 0.0,
        reverse_acting: false,
        mode: "AUTO".to_string(),
        output_min: 0.0,
        output_max: 100.0,
        position: Some(Position2D { x: 440.0, y: 240.0 }),
    });
    unit.control_loops.push(ControlLoopInstance {
        id: EntityId::new(),
        name: "TIC-101".to_string(),
        pv_tag: "TT-101".to_string(),
        sp: 332.0,
        out_tag: "F-101.DUTY".to_string(),
        kp: 1.5,
        ti: 25.0,
        td: 2.0,
        reverse_acting: false,
        mode: "AUTO".to_string(),
        output_min: 0.0,
        output_max: 100.0,
        position: Some(Position2D { x: 1720.0, y: 270.0 }),
    });
    unit.control_loops.push(ControlLoopInstance {
        id: EntityId::new(),
        name: "PIC-101".to_string(),
        pv_tag: "PT-102".to_string(),
        sp: 21.1,
        out_tag: "PV-102.OPENING".to_string(),
        kp: 1.8,
        ti: 15.0,
        td: 0.0,
        reverse_acting: false,
        mode: "AUTO".to_string(),
        output_min: 0.0,
        output_max: 100.0,
        position: Some(Position2D { x: 1220.0, y: 460.0 }),
    });
    unit.control_loops.push(ControlLoopInstance {
        id: EntityId::new(),
        name: "LIC-101".to_string(),
        pv_tag: "LT-101".to_string(),
        sp: 50.0,
        out_tag: "LV-101.OPENING".to_string(),
        kp: 1.2,
        ti: 12.0,
        td: 0.0,
        reverse_acting: false,
        mode: "AUTO".to_string(),
        output_min: 0.0,
        output_max: 100.0,
        position: Some(Position2D { x: 1360.0, y: 520.0 }),
    });
    unit.control_loops.push(ControlLoopInstance {
        id: EntityId::new(),
        name: "FIC-102".to_string(),
        pv_tag: "FT-102".to_string(),
        sp: 39302.0,
        out_tag: "K-101A.SPEED".to_string(),
        kp: 1.0,
        ti: 8.0,
        td: 0.0,
        reverse_acting: false,
        mode: "AUTO".to_string(),
        output_min: 0.0,
        output_max: 100.0,
        position: Some(Position2D { x: 1380.0, y: 640.0 }),
    });
    unit.control_loops.push(ControlLoopInstance {
        id: EntityId::new(),
        name: "FIC-103".to_string(),
        pv_tag: "FT-103".to_string(),
        sp: 6.0,
        out_tag: "FV-103.OPENING".to_string(),
        kp: 0.9,
        ti: 5.0,
        td: 0.0,
        reverse_acting: false,
        mode: "AUTO".to_string(),
        output_min: 0.0,
        output_max: 100.0,
        position: Some(Position2D { x: 680.0, y: 640.0 }),
    });
    unit.control_loops.push(ControlLoopInstance {
        id: EntityId::new(),
        name: "LIC-102".to_string(),
        pv_tag: "LT-102".to_string(),
        sp: 50.0,
        out_tag: "P-103A.FLOW".to_string(),
        kp: 1.4,
        ti: 15.0,
        td: 0.0,
        reverse_acting: false,
        mode: "AUTO".to_string(),
        output_min: 0.0,
        output_max: 100.0,
        position: Some(Position2D { x: 1740.0, y: 720.0 }),
    });
    unit.control_loops.push(ControlLoopInstance {
        id: EntityId::new(),
        name: "PIC-103".to_string(),
        pv_tag: "PT-103".to_string(),
        sp: 8.5,
        out_tag: "V-105.PRESSURE".to_string(),
        kp: 1.6,
        ti: 10.0,
        td: 0.0,
        reverse_acting: false,
        mode: "AUTO".to_string(),
        output_min: 0.0,
        output_max: 100.0,
        position: Some(Position2D { x: 2000.0, y: 400.0 }),
    });
    unit.control_loops.push(ControlLoopInstance {
        id: EntityId::new(),
        name: "TIC-102".to_string(),
        pv_tag: "TT-102".to_string(),
        sp: 215.0,
        out_tag: "C-101.REBOILER_DUTY".to_string(),
        kp: 1.8,
        ti: 20.0,
        td: 1.0,
        reverse_acting: false,
        mode: "AUTO".to_string(),
        output_min: 0.0,
        output_max: 100.0,
        position: Some(Position2D { x: 1740.0, y: 240.0 }),
    });
    unit.control_loops.push(ControlLoopInstance {
        id: EntityId::new(),
        name: "FIC-104".to_string(),
        pv_tag: "FT-104".to_string(),
        sp: 25.0,
        out_tag: "P-104A.FLOW".to_string(),
        kp: 0.9,
        ti: 6.0,
        td: 0.0,
        reverse_acting: false,
        mode: "AUTO".to_string(),
        output_min: 0.0,
        output_max: 100.0,
        position: Some(Position2D { x: 2120.0, y: 400.0 }),
    });
    unit.control_loops.push(ControlLoopInstance {
        id: EntityId::new(),
        name: "LIC-103".to_string(),
        pv_tag: "LT-103".to_string(),
        sp: 50.0,
        out_tag: "E-106.FLOW".to_string(),
        kp: 1.3,
        ti: 14.0,
        td: 0.0,
        reverse_acting: false,
        mode: "AUTO".to_string(),
        output_min: 0.0,
        output_max: 100.0,
        position: Some(Position2D { x: 2020.0, y: 800.0 }),
    });
    unit.control_loops.push(ControlLoopInstance {
        id: EntityId::new(),
        name: "TIC-103".to_string(),
        pv_tag: "TT-103".to_string(),
        sp: 85.0,
        out_tag: "E-104.DUTY".to_string(),
        kp: 1.5,
        ti: 18.0,
        td: 1.0,
        reverse_acting: false,
        mode: "AUTO".to_string(),
        output_min: 0.0,
        output_max: 100.0,
        position: Some(Position2D { x: 2020.0, y: 560.0 }),
    });

    // 5. Alarms & Trips (8 Protective Alarms / First-Out Trips)
    unit.alarms.push(AlarmInstance {
        id: EntityId::new(),
        name: "PAH-101".to_string(),
        pv_tag: "PT-102".to_string(),
        alarm_type: AlarmType::Hi,
        severity: AlarmPriority::High,
        trip_limit: 22.0,
        deadband: 0.2,
        description: "HP Separator high pressure warning".to_string(),
    });
    unit.alarms.push(AlarmInstance {
        id: EntityId::new(),
        name: "PAHH-101".to_string(),
        pv_tag: "PT-102".to_string(),
        alarm_type: AlarmType::HiHi,
        severity: AlarmPriority::Critical,
        trip_limit: 23.0,
        deadband: 0.2,
        description: "HP Separator high high pressure emergency trip".to_string(),
    });
    unit.alarms.push(AlarmInstance {
        id: EntityId::new(),
        name: "TAH-101".to_string(),
        pv_tag: "TT-102".to_string(),
        alarm_type: AlarmType::Hi,
        severity: AlarmPriority::High,
        trip_limit: 340.0,
        deadband: 1.0,
        description: "Reactor bed high temperature warning".to_string(),
    });
    unit.alarms.push(AlarmInstance {
        id: EntityId::new(),
        name: "TAHH-101".to_string(),
        pv_tag: "TT-102".to_string(),
        alarm_type: AlarmType::HiHi,
        severity: AlarmPriority::Critical,
        trip_limit: 343.3,
        deadband: 0.5,
        description: "Reactor maximum allowable temperature trip (343.3 C HPCL Visakh limit)".to_string(),
    });
    unit.alarms.push(AlarmInstance {
        id: EntityId::new(),
        name: "TDAH-101".to_string(),
        pv_tag: "TT-102".to_string(),
        alarm_type: AlarmType::Deviation,
        severity: AlarmPriority::High,
        trip_limit: 10.0,
        deadband: 0.5,
        description: "Reactor bed temperature rise exceeds max Delta T limit (10 C limit)".to_string(),
    });
    unit.alarms.push(AlarmInstance {
        id: EntityId::new(),
        name: "FAL-103".to_string(),
        pv_tag: "FT-103".to_string(),
        alarm_type: AlarmType::Lo,
        severity: AlarmPriority::High,
        trip_limit: 4.5,
        deadband: 0.2,
        description: "Wash water injection flow low - risk of ammonium salt deposition".to_string(),
    });
    unit.alarms.push(AlarmInstance {
        id: EntityId::new(),
        name: "LAHH-101".to_string(),
        pv_tag: "LT-101".to_string(),
        alarm_type: AlarmType::HiHi,
        severity: AlarmPriority::Critical,
        trip_limit: 0.85,
        deadband: 0.01,
        description: "HP Separator high high level trip - compressor liquid carryover protection".to_string(),
    });
    unit.alarms.push(AlarmInstance {
        id: EntityId::new(),
        name: "LAL-101".to_string(),
        pv_tag: "LT-101".to_string(),
        alarm_type: AlarmType::Lo,
        severity: AlarmPriority::High,
        trip_limit: 0.20,
        deadband: 0.01,
        description: "HP Separator low level alarm - gas blowthrough prevention".to_string(),
    });

    // 6. Safety Interlocks
    unit.interlocks.push(InterlockInstance {
        id: EntityId::new(),
        name: "ESD-R101-TEMP".to_string(),
        source_tag: "TT-102".to_string(),
        condition: ">".to_string(),
        threshold: 343.3,
        target_action: "TRIP".to_string(),
        target_equipment: Some(f101_id),
        latched: true,
        description: "Reactor thermal runaway - cut fuel to Charge Heater F-101".to_string(),
    });
    unit.interlocks.push(InterlockInstance {
        id: EntityId::new(),
        name: "ESD-V102-LAHH".to_string(),
        source_tag: "LT-101".to_string(),
        condition: ">".to_string(),
        threshold: 0.85,
        target_action: "TRIP".to_string(),
        target_equipment: Some(k101a_id),
        latched: true,
        description: "HP Separator liquid carryover - trip Recycle Gas Compressor K-101A".to_string(),
    });

    // 6.5 Dynamic Process Boundary Conditions (HPCL Visakh Benchmark Feeds & Utilities)
    let mut naphtha_comp = IndexMap::new();
    naphtha_comp.insert("paraffins".to_string(), 0.55);
    naphtha_comp.insert("naphthenes".to_string(), 0.33);
    naphtha_comp.insert("aromatics".to_string(), 0.12);

    let mut h2_comp = IndexMap::new();
    h2_comp.insert("H2".to_string(), 0.995);
    h2_comp.insert("CH4".to_string(), 0.005);

    let mut water_comp = IndexMap::new();
    water_comp.insert("H2O".to_string(), 1.0);

    let mut fuel_comp = IndexMap::new();
    fuel_comp.insert("CH4".to_string(), 0.85);
    fuel_comp.insert("C2H6".to_string(), 0.10);
    fuel_comp.insert("N2".to_string(), 0.05);

    unit.boundary_conditions.push(BoundaryConditionInstance {
        id: EntityId::new(),
        name: "NHT Fresh Naphtha Feed".to_string(),
        condition_type: "feed".to_string(),
        target_equipment_id: Some(tk101_id),
        target_port_id: Some(inlet_port),
        target_stream_id: None,
        flow_value: 199.0,
        flow_unit: "m3/h".to_string(),
        pressure_value: 5.0,
        pressure_unit: "barg".to_string(),
        temperature_value: 68.0,
        temperature_unit: "°C".to_string(),
        phase: "Liquid".to_string(),
        density: Some(740.0),
        molecular_weight: Some(0.110),
        enthalpy: Some(150_000.0),
        quality: Some(0.0),
        composition: naphtha_comp,
        validity_min_flow: Some(50.0),
        validity_max_flow: Some(260.0),
        source_provenance: "HPCL Visakh NHT Operating Manual (30,033 BPSD design benchmark)".to_string(),
        enabled: true,
    });

    unit.boundary_conditions.push(BoundaryConditionInstance {
        id: EntityId::new(),
        name: "H2 Makeup Gas Supply".to_string(),
        condition_type: "utility".to_string(),
        target_equipment_id: Some(mix101_id),
        target_port_id: Some(secondary_in_port),
        target_stream_id: None,
        flow_value: 2500.0,
        flow_unit: "Nm3/h".to_string(),
        pressure_value: 32.0,
        pressure_unit: "barg".to_string(),
        temperature_value: 40.0,
        temperature_unit: "°C".to_string(),
        phase: "Vapor".to_string(),
        density: Some(2.1),
        molecular_weight: Some(0.002016),
        enthalpy: Some(450_000.0),
        quality: Some(1.0),
        composition: h2_comp,
        validity_min_flow: Some(500.0),
        validity_max_flow: Some(4000.0),
        source_provenance: "Refinery High Pressure Hydrogen Network".to_string(),
        enabled: true,
    });

    unit.boundary_conditions.push(BoundaryConditionInstance {
        id: EntityId::new(),
        name: "Reactor Effluent Wash Water".to_string(),
        condition_type: "utility".to_string(),
        target_equipment_id: Some(v104_id),
        target_port_id: Some(inlet_port),
        target_stream_id: None,
        flow_value: 6.0,
        flow_unit: "m3/h".to_string(),
        pressure_value: 3.5,
        pressure_unit: "barg".to_string(),
        temperature_value: 30.0,
        temperature_unit: "°C".to_string(),
        phase: "Liquid".to_string(),
        density: Some(1000.0),
        molecular_weight: Some(0.018015),
        enthalpy: Some(125_000.0),
        quality: Some(0.0),
        composition: water_comp,
        validity_min_flow: Some(1.0),
        validity_max_flow: Some(15.0),
        source_provenance: "Demineralized Wash Water Utility".to_string(),
        enabled: true,
    });

    unit.boundary_conditions.push(BoundaryConditionInstance {
        id: EntityId::new(),
        name: "Charge Heater Fuel Gas".to_string(),
        condition_type: "utility".to_string(),
        target_equipment_id: Some(f101_id),
        target_port_id: None,
        target_stream_id: None,
        flow_value: 1200.0,
        flow_unit: "kg/h".to_string(),
        pressure_value: 3.0,
        pressure_unit: "barg".to_string(),
        temperature_value: 40.0,
        temperature_unit: "°C".to_string(),
        phase: "Vapor".to_string(),
        density: Some(0.85),
        molecular_weight: Some(0.0185),
        enthalpy: Some(500_000.0),
        quality: Some(1.0),
        composition: fuel_comp,
        validity_min_flow: Some(200.0),
        validity_max_flow: Some(2500.0),
        source_provenance: "Refinery Fuel Gas Header".to_string(),
        enabled: true,
    });

    unit.boundary_conditions.push(BoundaryConditionInstance {
        id: EntityId::new(),
        name: "Cooling Water Supply".to_string(),
        condition_type: "utility".to_string(),
        target_equipment_id: Some(e101trim_id),
        target_port_id: None,
        target_stream_id: None,
        flow_value: 150.0,
        flow_unit: "m3/h".to_string(),
        pressure_value: 4.0,
        pressure_unit: "barg".to_string(),
        temperature_value: 32.0,
        temperature_unit: "°C".to_string(),
        phase: "Liquid".to_string(),
        density: Some(1000.0),
        molecular_weight: Some(0.018015),
        enthalpy: Some(134_000.0),
        quality: Some(0.0),
        composition: IndexMap::new(),
        validity_min_flow: Some(50.0),
        validity_max_flow: Some(300.0),
        source_provenance: "Cooling Water Utility Network".to_string(),
        enabled: true,
    });

    unit.boundary_conditions.push(BoundaryConditionInstance {
        id: EntityId::new(),
        name: "Stripper Stripping Steam HP".to_string(),
        condition_type: "utility".to_string(),
        target_equipment_id: Some(c101_id),
        target_port_id: None,
        target_stream_id: None,
        flow_value: 1500.0,
        flow_unit: "kg/h".to_string(),
        pressure_value: 10.5,
        pressure_unit: "barg".to_string(),
        temperature_value: 210.0,
        temperature_unit: "°C".to_string(),
        phase: "Vapor".to_string(),
        density: Some(5.8),
        molecular_weight: Some(0.018015),
        enthalpy: Some(2_800_000.0),
        quality: Some(1.0),
        composition: IndexMap::new(),
        validity_min_flow: Some(200.0),
        validity_max_flow: Some(3000.0),
        source_provenance: "Medium Pressure Superheated Steam Header".to_string(),
        enabled: true,
    });

    // 6.6 Explicit Initial Internal Conditions (Operating Point Initialization)
    unit.initial_conditions.push(InitialConditionInstance {
        id: EntityId::new(),
        entity_id: "TK-101".to_string(),
        variable: "level".to_string(),
        value: 0.725,
        unit: Some("fraction".to_string()),
        description: Some("Feed storage tank initial level".to_string()),
    });
    unit.initial_conditions.push(InitialConditionInstance {
        id: EntityId::new(),
        entity_id: "V-101".to_string(),
        variable: "level".to_string(),
        value: 0.50,
        unit: Some("fraction".to_string()),
        description: Some("Feed surge drum initial level".to_string()),
    });
    unit.initial_conditions.push(InitialConditionInstance {
        id: EntityId::new(),
        entity_id: "V-102".to_string(),
        variable: "level".to_string(),
        value: 0.52,
        unit: Some("fraction".to_string()),
        description: Some("High pressure separator initial level".to_string()),
    });
    unit.initial_conditions.push(InitialConditionInstance {
        id: EntityId::new(),
        entity_id: "V-103".to_string(),
        variable: "level".to_string(),
        value: 0.30,
        unit: Some("fraction".to_string()),
        description: Some("Recycle compressor KO drum level".to_string()),
    });
    unit.initial_conditions.push(InitialConditionInstance {
        id: EntityId::new(),
        entity_id: "V-104".to_string(),
        variable: "level".to_string(),
        value: 0.60,
        unit: Some("fraction".to_string()),
        description: Some("Wash water surge drum level".to_string()),
    });
    unit.initial_conditions.push(InitialConditionInstance {
        id: EntityId::new(),
        entity_id: "V-105".to_string(),
        variable: "level".to_string(),
        value: 0.48,
        unit: Some("fraction".to_string()),
        description: Some("Stripper reflux drum level".to_string()),
    });
    unit.initial_conditions.push(InitialConditionInstance {
        id: EntityId::new(),
        entity_id: "V-106".to_string(),
        variable: "level".to_string(),
        value: 0.50,
        unit: Some("fraction".to_string()),
        description: Some("Splitter reflux drum level".to_string()),
    });
    unit.initial_conditions.push(InitialConditionInstance {
        id: EntityId::new(),
        entity_id: "P-101A".to_string(),
        variable: "speed".to_string(),
        value: 2950.0,
        unit: Some("rpm".to_string()),
        description: Some("Charge pump P-101A operating speed".to_string()),
    });
    unit.initial_conditions.push(InitialConditionInstance {
        id: EntityId::new(),
        entity_id: "FV-101".to_string(),
        variable: "opening".to_string(),
        value: 55.0,
        unit: Some("%".to_string()),
        description: Some("Feed flow control valve initial opening".to_string()),
    });
    unit.initial_conditions.push(InitialConditionInstance {
        id: EntityId::new(),
        entity_id: "F-101".to_string(),
        variable: "temperature".to_string(),
        value: 325.0,
        unit: Some("°C".to_string()),
        description: Some("Charge heater coil outlet temperature".to_string()),
    });
    unit.initial_conditions.push(InitialConditionInstance {
        id: EntityId::new(),
        entity_id: "R-101".to_string(),
        variable: "temperature".to_string(),
        value: 335.0,
        unit: Some("°C".to_string()),
        description: Some("Reactor bed temperature with exotherm".to_string()),
    });
    unit.initial_conditions.push(InitialConditionInstance {
        id: EntityId::new(),
        entity_id: "K-101A".to_string(),
        variable: "speed".to_string(),
        value: 10450.0,
        unit: Some("rpm".to_string()),
        description: Some("Recycle gas compressor operating speed".to_string()),
    });

    // Assemble physical process graph hierarchy
    area.units.push(unit);
    plant.areas.push(area);
    project.plants.push(plant);

    // 7. Operational Structure (12 Systems, 7 Units across Area 72)
    let sys_feed = ProjectPlantSystem {
        id: "sys-nht-feed".to_string(),
        name: "Naphtha Feed & Surge System".to_string(),
        area_id: Some("area-nht".to_string()),
        default_display_id: Some("disp-nht-feed".to_string()),
        description: Some("Feed day storage, charge pumping, and micro-filtration train".to_string()),
        subsystems: vec![ProjectPlantSubsystem {
            id: "sub-nht-feed-pumping".to_string(),
            name: "Feed Pumping & Filtering".to_string(),
            system_id: Some("sys-nht-feed".to_string()),
            description: None,
            units: vec![ProjectPlantUnit {
                id: "unit-nht-feed".to_string(),
                name: "Feed Preparation Unit".to_string(),
                subsystem_id: Some("sub-nht-feed-pumping".to_string()),
                system_id: Some("sys-nht-feed".to_string()),
                area_id: Some("area-nht".to_string()),
                equipment_ids: vec![
                    tk101_id.to_string(),
                    v101_id.to_string(),
                    p101a_id.to_string(),
                    p101b_id.to_string(),
                    fv101_id.to_string(),
                    x101_id.to_string(),
                ],
                display_ids: vec!["disp-nht-feed".to_string(), "disp-nht-overview".to_string()],
                description: Some("Feed inventory control and primary filtration".to_string()),
            }],
        }],
    };

    let sys_h2 = ProjectPlantSystem {
        id: "sys-nht-h2".to_string(),
        name: "Hydrogen Supply & Make-up System".to_string(),
        area_id: Some("area-nht".to_string()),
        default_display_id: Some("disp-nht-h2-recycle".to_string()),
        description: Some("Make-up hydrogen metering and header pressure regulation".to_string()),
        subsystems: vec![],
    };

    let sys_rx = ProjectPlantSystem {
        id: "sys-nht-rx".to_string(),
        name: "Reactor & Charge Preheating System".to_string(),
        area_id: Some("area-nht".to_string()),
        default_display_id: Some("disp-nht-rx".to_string()),
        description: Some("7-shell CFE heat integration train, charge heater, and trickle-bed catalytic reactor".to_string()),
        subsystems: vec![ProjectPlantSubsystem {
            id: "sub-nht-rx-train".to_string(),
            name: "Reactor Train".to_string(),
            system_id: Some("sys-nht-rx".to_string()),
            description: None,
            units: vec![ProjectPlantUnit {
                id: "unit-nht-rx".to_string(),
                name: "Reactor & Preheating Unit".to_string(),
                subsystem_id: Some("sub-nht-rx-train".to_string()),
                system_id: Some("sys-nht-rx".to_string()),
                area_id: Some("area-nht".to_string()),
                equipment_ids: vec![
                    mix101_id.to_string(),
                    e101a_id.to_string(),
                    e101b_id.to_string(),
                    e101c_id.to_string(),
                    e101d_id.to_string(),
                    e101e_id.to_string(),
                    e101f_id.to_string(),
                    e101g_id.to_string(),
                    f101_id.to_string(),
                    r101_id.to_string(),
                ],
                display_ids: vec!["disp-nht-rx".to_string(), "disp-nht-rx-detail".to_string(), "disp-nht-overview".to_string()],
                description: Some("Fixed-bed desulfurization and denitrogenation reactor loop".to_string()),
            }],
        }],
    };

    let sys_sep = ProjectPlantSystem {
        id: "sys-nht-sep".to_string(),
        name: "HP Separation & Effluent Cooling System".to_string(),
        area_id: Some("area-nht".to_string()),
        default_display_id: Some("disp-nht-sep".to_string()),
        description: Some("Effluent cooling, salt dissolution wash water condensation, and three-phase separation".to_string()),
        subsystems: vec![ProjectPlantSubsystem {
            id: "sub-nht-sep-loop".to_string(),
            name: "HP Separation Train".to_string(),
            system_id: Some("sys-nht-sep".to_string()),
            description: None,
            units: vec![ProjectPlantUnit {
                id: "unit-nht-sep".to_string(),
                name: "HP Separation Unit".to_string(),
                subsystem_id: Some("sub-nht-sep-loop".to_string()),
                system_id: Some("sys-nht-sep".to_string()),
                area_id: Some("area-nht".to_string()),
                equipment_ids: vec![
                    mix102_id.to_string(),
                    a101_id.to_string(),
                    e101trim_id.to_string(),
                    v102_id.to_string(),
                    lv101_id.to_string(),
                ],
                display_ids: vec!["disp-nht-sep".to_string(), "disp-nht-overview".to_string()],
                description: Some("Gas-liquid-water separation at 21.1 kg/cm2g".to_string()),
            }],
        }],
    };

    let sys_recycle = ProjectPlantSystem {
        id: "sys-nht-recycle".to_string(),
        name: "Recycle Gas Compression & Treat Gas System".to_string(),
        area_id: Some("area-nht".to_string()),
        default_display_id: Some("disp-nht-h2-recycle".to_string()),
        description: Some("Recycle gas KO drum, centrifugal compressor, anti-surge and purge gas control".to_string()),
        subsystems: vec![ProjectPlantSubsystem {
            id: "sub-nht-comp-train".to_string(),
            name: "Recycle Compression Train".to_string(),
            system_id: Some("sys-nht-recycle".to_string()),
            description: None,
            units: vec![ProjectPlantUnit {
                id: "unit-nht-recycle".to_string(),
                name: "Recycle Gas Compression Unit".to_string(),
                subsystem_id: Some("sub-nht-comp-train".to_string()),
                system_id: Some("sys-nht-recycle".to_string()),
                area_id: Some("area-nht".to_string()),
                equipment_ids: vec![
                    v103_id.to_string(),
                    k101a_id.to_string(),
                    split101_id.to_string(),
                    pv102_id.to_string(),
                ],
                display_ids: vec!["disp-nht-h2-recycle".to_string(), "disp-nht-comp-detail".to_string(), "disp-nht-overview".to_string()],
                description: Some("High-pressure recycle gas boost back to 36.3 kg/cm2g".to_string()),
            }],
        }],
    };

    let sys_strip = ProjectPlantSystem {
        id: "sys-nht-strip".to_string(),
        name: "Stripper & Debutanizer Fractionation System".to_string(),
        area_id: Some("area-nht".to_string()),
        default_display_id: Some("disp-nht-strip".to_string()),
        description: Some("H2S and light ends stripping from unstabilized hydrotreated naphtha".to_string()),
        subsystems: vec![ProjectPlantSubsystem {
            id: "sub-nht-strip-train".to_string(),
            name: "Stripper Section".to_string(),
            system_id: Some("sys-nht-strip".to_string()),
            description: None,
            units: vec![ProjectPlantUnit {
                id: "unit-nht-strip".to_string(),
                name: "Stripper Fractionation Unit".to_string(),
                subsystem_id: Some("sub-nht-strip-train".to_string()),
                system_id: Some("sys-nht-strip".to_string()),
                area_id: Some("area-nht".to_string()),
                equipment_ids: vec![
                    e102a_id.to_string(),
                    e102b_id.to_string(),
                    c101_id.to_string(),
                    e103_id.to_string(),
                    v105_id.to_string(),
                    p104a_id.to_string(),
                    p103a_id.to_string(),
                ],
                display_ids: vec!["disp-nht-strip".to_string(), "disp-nht-overview".to_string()],
                description: Some("Sour gas distillation and bottoms stabilization".to_string()),
            }],
        }],
    };

    let sys_split = ProjectPlantSystem {
        id: "sys-nht-split".to_string(),
        name: "Naphtha Splitter & Product Run-down System".to_string(),
        area_id: Some("area-nht".to_string()),
        default_display_id: Some("disp-nht-split".to_string()),
        description: Some("Separation of light naphtha (Isom feed) from heavy naphtha (Platformer feed)".to_string()),
        subsystems: vec![ProjectPlantSubsystem {
            id: "sub-nht-split-train".to_string(),
            name: "Splitter Section".to_string(),
            system_id: Some("sys-nht-split".to_string()),
            description: None,
            units: vec![ProjectPlantUnit {
                id: "unit-nht-split".to_string(),
                name: "Naphtha Splitter Unit".to_string(),
                subsystem_id: Some("sub-nht-split-train".to_string()),
                system_id: Some("sys-nht-split".to_string()),
                area_id: Some("area-nht".to_string()),
                equipment_ids: vec![
                    c102_id.to_string(),
                    e104_id.to_string(),
                    v106_id.to_string(),
                    p105a_id.to_string(),
                    e105_id.to_string(),
                    e106_id.to_string(),
                ],
                display_ids: vec!["disp-nht-split".to_string(), "disp-nht-overview".to_string()],
                description: Some("Precision narrow-boiling-range product splitting".to_string()),
            }],
        }],
    };

    let sys_water = ProjectPlantSystem {
        id: "sys-nht-water".to_string(),
        name: "Wash Water Injection System".to_string(),
        area_id: Some("area-nht".to_string()),
        default_display_id: Some("disp-nht-util".to_string()),
        description: Some("Continuous high-pressure condensate injection upstream of air cooler".to_string()),
        subsystems: vec![ProjectPlantSubsystem {
            id: "sub-nht-water-train".to_string(),
            name: "Wash Water Injection".to_string(),
            system_id: Some("sys-nht-water".to_string()),
            description: None,
            units: vec![ProjectPlantUnit {
                id: "unit-nht-util".to_string(),
                name: "Utilities & Wash Water Unit".to_string(),
                subsystem_id: Some("sub-nht-water-train".to_string()),
                system_id: Some("sys-nht-water".to_string()),
                area_id: Some("area-nht".to_string()),
                equipment_ids: vec![
                    v104_id.to_string(),
                    p102a_id.to_string(),
                    fv103_id.to_string(),
                ],
                display_ids: vec!["disp-nht-util".to_string(), "disp-nht-overview".to_string()],
                description: Some("Corrosion prevention via salt washing".to_string()),
            }],
        }],
    };

    let sys_steam = ProjectPlantSystem {
        id: "sys-nht-steam".to_string(),
        name: "Steam & Heat Utilities Distribution System".to_string(),
        area_id: Some("area-nht".to_string()),
        default_display_id: Some("disp-nht-util".to_string()),
        description: Some("High/medium pressure steam distribution to reboilers and pumps".to_string()),
        subsystems: vec![],
    };

    let sys_sour = ProjectPlantSystem {
        id: "sys-nht-sour".to_string(),
        name: "Sour Water & Acid Gas Recovery Header".to_string(),
        area_id: Some("area-nht".to_string()),
        default_display_id: Some("disp-nht-sep".to_string()),
        description: Some("Sour water boot drainage to SWS and H2S off-gas to SRU".to_string()),
        subsystems: vec![],
    };

    let sys_offgas = ProjectPlantSystem {
        id: "sys-nht-offgas".to_string(),
        name: "SRU Off-Gas & Fuel Gas Balancing System".to_string(),
        area_id: Some("area-nht".to_string()),
        default_display_id: Some("disp-nht-h2-recycle".to_string()),
        description: Some("Fuel gas blending and flare header management".to_string()),
        subsystems: vec![],
    };

    let sys_bl = ProjectPlantSystem {
        id: "sys-nht-bl".to_string(),
        name: "Battery Limits & Product Interconnection Network".to_string(),
        area_id: Some("area-nht".to_string()),
        default_display_id: Some("disp-nht-overview".to_string()),
        description: Some("Battery limits transfer to downstream CCR and Isomerization units".to_string()),
        subsystems: vec![],
    };

    // 8. DCS Displays (12 Displays with non-overlapping placements)
    let make_placement = |eid: EquipmentId, x: f64, y: f64| DisplayElementPlacement {
        equipment_id: eid.to_string(),
        x,
        y,
        width: Some(80.0),
        height: Some(80.0),
        rotation: Some(0.0),
    };

    let displays = vec![
        ProjectDisplay {
            id: "disp-nht-overview".to_string(),
            name: "NHT Overall Process Overview".to_string(),
            display_type: "overview".to_string(),
            area_id: Some("area-nht".to_string()),
            system_id: None,
            subsystem_id: None,
            unit_id: None,
            equipment_ids: vec![
                tk101_id.to_string(), v101_id.to_string(), p101a_id.to_string(), fv101_id.to_string(),
                x101_id.to_string(), mix101_id.to_string(), k101a_id.to_string(),
                e101a_id.to_string(), f101_id.to_string(), r101_id.to_string(), a101_id.to_string(),
                v102_id.to_string(), v104_id.to_string(), c101_id.to_string(), c102_id.to_string(),
            ],
            instrument_ids: vec!["FT-101".to_string(), "TT-101".to_string(), "PT-102".to_string()],
            stream_ids: vec![
                "STR-NHT-00".to_string(), "STR-NHT-01".to_string(), "STR-NHT-02".to_string(),
                "STR-NHT-03".to_string(), "STR-NHT-04".to_string(), "STR-NHT-05".to_string(),
                "STR-NHT-12".to_string(), "STR-NHT-13".to_string(), "STR-NHT-14".to_string(),
                "STR-NHT-21".to_string(), "STR-NHT-25".to_string(), "STR-NHT-28".to_string(),
                "STR-NHT-29".to_string(), "STR-NHT-33".to_string(), "STR-NHT-40".to_string(),
                "STR-NHT-43".to_string(),
            ],
            viewport_x: Some(0.0),
            viewport_y: Some(0.0),
            viewport_width: Some(1920.0),
            viewport_height: Some(1080.0),
            viewport_zoom: Some(1.0),
            is_default: true,
            order: Some(1),
            description: Some("Global overview of the 30,033 BPSD Naphtha Hydrotreater flowsheet".to_string()),
            placements: vec![
                // Tier 1: Top Tier (Feed Prep & H2 Compression Loop) - y: 220
                make_placement(tk101_id, 140.0, 220.0),
                make_placement(v101_id, 360.0, 220.0),
                make_placement(p101a_id, 560.0, 220.0),
                make_placement(fv101_id, 720.0, 220.0),
                make_placement(x101_id, 880.0, 220.0),
                make_placement(mix101_id, 1060.0, 220.0),
                make_placement(k101a_id, 1500.0, 220.0),

                // Tier 2: Middle Tier (Reaction & Fired Heater Train) - y: 520
                make_placement(e101a_id, 360.0, 520.0),
                make_placement(f101_id, 720.0, 520.0),
                make_placement(r101_id, 1080.0, 500.0),
                make_placement(a101_id, 1360.0, 520.0),

                // Tier 3: Bottom Tier (Separation, Stripper & Splitter) - y: 800 - 840
                make_placement(v102_id, 1500.0, 820.0),
                make_placement(v104_id, 1160.0, 840.0),
                make_placement(c101_id, 760.0, 800.0),
                make_placement(c102_id, 360.0, 800.0),
            ],
        },
        ProjectDisplay {
            id: "disp-nht-feed".to_string(),
            name: "NHT Feed Preparation & Surge".to_string(),
            display_type: "process".to_string(),
            area_id: Some("area-nht".to_string()),
            system_id: Some("sys-nht-feed".to_string()),
            subsystem_id: None,
            unit_id: Some("unit-nht-feed".to_string()),
            equipment_ids: vec![
                tk101_id.to_string(), v101_id.to_string(), p101a_id.to_string(),
                p101b_id.to_string(), fv101_id.to_string(), x101_id.to_string(),
                mix101_id.to_string(),
            ],
            instrument_ids: vec!["FT-101".to_string()],
            stream_ids: vec![
                "STR-NHT-00".to_string(), "STR-NHT-01".to_string(), "STR-NHT-02".to_string(),
                "STR-NHT-03".to_string(), "STR-NHT-04".to_string(),
            ],
            viewport_x: Some(0.0),
            viewport_y: Some(0.0),
            viewport_width: Some(1920.0),
            viewport_height: Some(1080.0),
            viewport_zoom: Some(1.0),
            is_default: false,
            order: Some(2),
            description: Some("Feed storage, booster charge pumps, and coalescing filter system".to_string()),
            placements: vec![
                make_placement(tk101_id, 140.0, 380.0),
                make_placement(v101_id, 300.0, 380.0),
                make_placement(p101a_id, 460.0, 380.0),
                make_placement(p101b_id, 460.0, 520.0),
                make_placement(fv101_id, 620.0, 380.0),
                make_placement(x101_id, 760.0, 380.0),
                make_placement(mix101_id, 900.0, 380.0),
            ],
        },
        ProjectDisplay {
            id: "disp-nht-rx".to_string(),
            name: "NHT Reactor Train & Combined Feed Exchangers".to_string(),
            display_type: "process".to_string(),
            area_id: Some("area-nht".to_string()),
            system_id: Some("sys-nht-rx".to_string()),
            subsystem_id: None,
            unit_id: Some("unit-nht-rx".to_string()),
            equipment_ids: vec![
                mix101_id.to_string(), e101a_id.to_string(), e101b_id.to_string(),
                e101c_id.to_string(), e101d_id.to_string(), e101e_id.to_string(),
                e101f_id.to_string(), e101g_id.to_string(), f101_id.to_string(),
                r101_id.to_string(),
            ],
            instrument_ids: vec![
                "TT-101".to_string(),
                "TI-101-CIT".to_string(),
                "TI-101-SK1".to_string(),
                "TI-101-SK2".to_string(),
                "PT-104".to_string(),
                "FT-105".to_string(),
                "PT-105".to_string(),
                "PDT-101".to_string(),
                "TI-104".to_string(),
                "AI-101".to_string(),
                "TT-102".to_string(),
            ],
            stream_ids: vec!["STR-NHT-05".to_string(), "STR-NHT-12".to_string(), "STR-NHT-13".to_string(), "STR-NHT-14".to_string()],
            viewport_x: Some(0.0),
            viewport_y: Some(0.0),
            viewport_width: Some(1920.0),
            viewport_height: Some(1080.0),
            viewport_zoom: Some(1.0),
            is_default: false,
            order: Some(3),
            description: Some("7-shell Combined Feed Exchanger train (72-E-01A..G), Charge Heater, and 72-R-01".to_string()),
            placements: vec![
                make_placement(mix101_id, 100.0, 380.0),
                make_placement(e101a_id, 260.0, 380.0),
                make_placement(e101b_id, 430.0, 380.0),
                make_placement(e101c_id, 600.0, 380.0),
                make_placement(e101d_id, 770.0, 380.0),
                make_placement(e101e_id, 940.0, 380.0),
                make_placement(e101f_id, 1110.0, 380.0),
                make_placement(e101g_id, 1280.0, 380.0),
                make_placement(f101_id, 1480.0, 380.0),
                make_placement(r101_id, 1680.0, 360.0),
            ],
        },
        ProjectDisplay {
            id: "disp-nht-sep".to_string(),
            name: "NHT High Pressure Separation".to_string(),
            display_type: "process".to_string(),
            area_id: Some("area-nht".to_string()),
            system_id: Some("sys-nht-sep".to_string()),
            subsystem_id: None,
            unit_id: Some("unit-nht-sep".to_string()),
            equipment_ids: vec![
                mix102_id.to_string(), a101_id.to_string(), e101trim_id.to_string(),
                v102_id.to_string(), lv101_id.to_string(), v103_id.to_string(),
            ],
            instrument_ids: vec!["PT-102".to_string(), "LT-101".to_string()],
            stream_ids: vec!["STR-NHT-25".to_string(), "STR-NHT-27".to_string(), "STR-NHT-28".to_string(), "STR-NHT-33".to_string()],
            viewport_x: Some(0.0),
            viewport_y: Some(0.0),
            viewport_width: Some(1920.0),
            viewport_height: Some(1080.0),
            viewport_zoom: Some(1.0),
            is_default: false,
            order: Some(4),
            description: Some("Product Condenser A-101, Trim Cooler, and High Pressure Separator 72-V-02".to_string()),
            placements: vec![
                make_placement(mix102_id, 140.0, 380.0),
                make_placement(a101_id, 320.0, 380.0),
                make_placement(e101trim_id, 500.0, 380.0),
                make_placement(v102_id, 700.0, 360.0),
                make_placement(lv101_id, 880.0, 380.0),
                make_placement(v103_id, 700.0, 180.0),
            ],
        },
        ProjectDisplay {
            id: "disp-nht-h2-recycle".to_string(),
            name: "NHT Hydrogen & Recycle Compression".to_string(),
            display_type: "process".to_string(),
            area_id: Some("area-nht".to_string()),
            system_id: Some("sys-nht-recycle".to_string()),
            subsystem_id: None,
            unit_id: Some("unit-nht-recycle".to_string()),
            equipment_ids: vec![
                v103_id.to_string(), k101a_id.to_string(), split101_id.to_string(),
                pv102_id.to_string(), mix101_id.to_string(),
            ],
            instrument_ids: vec!["FT-102".to_string()],
            stream_ids: vec!["STR-NHT-29".to_string(), "STR-NHT-30".to_string(), "STR-NHT-31".to_string()],
            viewport_x: Some(0.0),
            viewport_y: Some(0.0),
            viewport_width: Some(1920.0),
            viewport_height: Some(1080.0),
            viewport_zoom: Some(1.0),
            is_default: false,
            order: Some(5),
            description: Some("Recycle Gas Compressor 72-K-01A/B and Make-up H2 injection header".to_string()),
            placements: vec![
                make_placement(v103_id, 220.0, 380.0),
                make_placement(k101a_id, 420.0, 380.0),
                make_placement(split101_id, 620.0, 380.0),
                make_placement(pv102_id, 800.0, 260.0),
                make_placement(mix101_id, 800.0, 500.0),
            ],
        },
        ProjectDisplay {
            id: "disp-nht-strip".to_string(),
            name: "NHT Naphtha Stripper Column".to_string(),
            display_type: "process".to_string(),
            area_id: Some("area-nht".to_string()),
            system_id: Some("sys-nht-strip".to_string()),
            subsystem_id: None,
            unit_id: Some("unit-nht-strip".to_string()),
            equipment_ids: vec![
                e102a_id.to_string(), e102b_id.to_string(), c101_id.to_string(),
                e103_id.to_string(), v105_id.to_string(), p104a_id.to_string(),
                p103a_id.to_string(),
            ],
            instrument_ids: vec!["LT-102".to_string(), "PT-103".to_string(), "FT-104".to_string()],
            stream_ids: vec!["STR-NHT-34".to_string(), "STR-NHT-36".to_string(), "STR-NHT-37".to_string(), "STR-NHT-40".to_string()],
            viewport_x: Some(0.0),
            viewport_y: Some(0.0),
            viewport_width: Some(1920.0),
            viewport_height: Some(1080.0),
            viewport_zoom: Some(1.0),
            is_default: false,
            order: Some(6),
            description: Some("Stripper Column 72-C-01, Overhead Condenser E-103, and Bottoms Reboiling".to_string()),
            placements: vec![
                make_placement(e102a_id, 180.0, 420.0),
                make_placement(e102b_id, 340.0, 420.0),
                make_placement(c101_id, 540.0, 360.0),
                make_placement(e103_id, 760.0, 200.0),
                make_placement(v105_id, 940.0, 220.0),
                make_placement(p104a_id, 1100.0, 240.0),
                make_placement(p103a_id, 760.0, 540.0),
            ],
        },
        ProjectDisplay {
            id: "disp-nht-split".to_string(),
            name: "NHT Naphtha Splitter & Rundown".to_string(),
            display_type: "process".to_string(),
            area_id: Some("area-nht".to_string()),
            system_id: Some("sys-nht-split".to_string()),
            subsystem_id: None,
            unit_id: Some("unit-nht-split".to_string()),
            equipment_ids: vec![
                c102_id.to_string(), e104_id.to_string(), v106_id.to_string(),
                p105a_id.to_string(), e105_id.to_string(), e106_id.to_string(),
            ],
            instrument_ids: vec!["LT-103".to_string(), "TT-103".to_string()],
            stream_ids: vec!["STR-NHT-43".to_string(), "STR-NHT-44".to_string(), "STR-NHT-46".to_string(), "STR-NHT-48".to_string()],
            viewport_x: Some(0.0),
            viewport_y: Some(0.0),
            viewport_width: Some(1920.0),
            viewport_height: Some(1080.0),
            viewport_zoom: Some(1.0),
            is_default: false,
            order: Some(7),
            description: Some("Fractionation into Light Hydrotreated Naphtha (Isom) and Heavy Naphtha (Platformer)".to_string()),
            placements: vec![
                make_placement(c102_id, 360.0, 380.0),
                make_placement(e104_id, 580.0, 200.0),
                make_placement(v106_id, 760.0, 220.0),
                make_placement(p105a_id, 920.0, 240.0),
                make_placement(e105_id, 1100.0, 240.0),
                make_placement(e106_id, 620.0, 540.0),
            ],
        },
        ProjectDisplay {
            id: "disp-nht-util".to_string(),
            name: "NHT Wash Water & Utility Distribution".to_string(),
            display_type: "process".to_string(),
            area_id: Some("area-nht".to_string()),
            system_id: Some("sys-nht-water".to_string()),
            subsystem_id: None,
            unit_id: Some("unit-nht-util".to_string()),
            equipment_ids: vec![
                v104_id.to_string(), p102a_id.to_string(), fv103_id.to_string(),
                mix102_id.to_string(),
            ],
            instrument_ids: vec!["FT-103".to_string()],
            stream_ids: vec!["STR-NHT-22".to_string(), "STR-NHT-23".to_string(), "STR-NHT-24".to_string()],
            viewport_x: Some(0.0),
            viewport_y: Some(0.0),
            viewport_width: Some(1920.0),
            viewport_height: Some(1080.0),
            viewport_zoom: Some(1.0),
            is_default: false,
            order: Some(8),
            description: Some("Wash water break tank 72-V-04, injection pumps, and salt wash circuit".to_string()),
            placements: vec![
                make_placement(v104_id, 220.0, 380.0),
                make_placement(p102a_id, 420.0, 380.0),
                make_placement(fv103_id, 600.0, 380.0),
                make_placement(mix102_id, 780.0, 380.0),
            ],
        },
        ProjectDisplay {
            id: "disp-nht-comp-detail".to_string(),
            name: "Recycle Gas Compressor 72-K-01 Detail".to_string(),
            display_type: "detail".to_string(),
            area_id: Some("area-nht".to_string()),
            system_id: Some("sys-nht-recycle".to_string()),
            subsystem_id: None,
            unit_id: Some("unit-nht-recycle".to_string()),
            equipment_ids: vec![v103_id.to_string(), k101a_id.to_string(), split101_id.to_string()],
            instrument_ids: vec!["FT-102".to_string()],
            stream_ids: vec!["STR-NHT-29".to_string(), "STR-NHT-30".to_string()],
            viewport_x: Some(0.0),
            viewport_y: Some(0.0),
            viewport_width: Some(1920.0),
            viewport_height: Some(1080.0),
            viewport_zoom: Some(1.0),
            is_default: false,
            order: Some(9),
            description: Some("Recycle Compressor performance curve, anti-surge line, and seal oil system".to_string()),
            placements: vec![
                make_placement(v103_id, 260.0, 380.0),
                make_placement(k101a_id, 540.0, 360.0),
                make_placement(split101_id, 820.0, 380.0),
            ],
        },
        ProjectDisplay {
            id: "disp-nht-rx-detail".to_string(),
            name: "Reactor 72-R-01 Bed Profile & Multi-Point Temperatures".to_string(),
            display_type: "detail".to_string(),
            area_id: Some("area-nht".to_string()),
            system_id: Some("sys-nht-rx".to_string()),
            subsystem_id: None,
            unit_id: Some("unit-nht-rx".to_string()),
            equipment_ids: vec![f101_id.to_string(), r101_id.to_string()],
            instrument_ids: vec![
                "TT-101".to_string(),
                "TI-101-CIT".to_string(),
                "TI-101-SK1".to_string(),
                "TI-101-SK2".to_string(),
                "PDT-101".to_string(),
                "TI-104".to_string(),
                "AI-101".to_string(),
                "TT-102".to_string(),
            ],
            stream_ids: vec!["STR-NHT-13".to_string(), "STR-NHT-14".to_string()],
            viewport_x: Some(0.0),
            viewport_y: Some(0.0),
            viewport_width: Some(1920.0),
            viewport_height: Some(1080.0),
            viewport_zoom: Some(1.0),
            is_default: false,
            order: Some(10),
            description: Some("Detailed axial/radial multi-thermocouple temperature profiles across catalyst bed".to_string()),
            placements: vec![
                make_placement(f101_id, 260.0, 380.0),
                make_placement(r101_id, 620.0, 340.0),
            ],
        },
        ProjectDisplay {
            id: "disp-nht-alarm-summary".to_string(),
            name: "NHT Plant Alarm Summary & Matrix".to_string(),
            display_type: "alarm_summary".to_string(),
            area_id: Some("area-nht".to_string()),
            system_id: None,
            subsystem_id: None,
            unit_id: None,
            equipment_ids: vec![r101_id.to_string(), v102_id.to_string(), k101a_id.to_string(), c101_id.to_string()],
            instrument_ids: vec![],
            stream_ids: vec![],
            viewport_x: Some(0.0),
            viewport_y: Some(0.0),
            viewport_width: Some(1920.0),
            viewport_height: Some(1080.0),
            viewport_zoom: Some(1.0),
            is_default: false,
            order: Some(11),
            description: Some("First-out alarm priority matrix and emergency shutdown state overview".to_string()),
            placements: vec![
                make_placement(r101_id, 300.0, 300.0),
                make_placement(v102_id, 600.0, 300.0),
                make_placement(k101a_id, 900.0, 300.0),
                make_placement(c101_id, 1200.0, 300.0),
            ],
        },
        ProjectDisplay {
            id: "disp-nht-ctl-summary".to_string(),
            name: "NHT Plant Control Loops & PID Overview".to_string(),
            display_type: "control_overview".to_string(),
            area_id: Some("area-nht".to_string()),
            system_id: None,
            subsystem_id: None,
            unit_id: None,
            equipment_ids: vec![fv101_id.to_string(), f101_id.to_string(), lv101_id.to_string(), fv103_id.to_string(), pv102_id.to_string()],
            instrument_ids: vec![],
            stream_ids: vec![],
            viewport_x: Some(0.0),
            viewport_y: Some(0.0),
            viewport_width: Some(1920.0),
            viewport_height: Some(1080.0),
            viewport_zoom: Some(1.0),
            is_default: false,
            order: Some(12),
            description: Some("Multi-loop faceplate overview of all 12 plantwide regulatory controllers".to_string()),
            placements: vec![
                make_placement(fv101_id, 200.0, 300.0),
                make_placement(f101_id, 450.0, 300.0),
                make_placement(lv101_id, 700.0, 300.0),
                make_placement(fv103_id, 950.0, 300.0),
                make_placement(pv102_id, 1200.0, 300.0),
            ],
        },
    ];

    // 9. Cross-Display Navigation Links (17 Bidirectional Links)
    let navigation_links = vec![
        ProjectNavigationLink {
            id: "nav-nht-01".to_string(),
            source_display_id: "disp-nht-overview".to_string(),
            source_element_id: Some(p101a_id.to_string()),
            target_display_id: "disp-nht-feed".to_string(),
            target_element_id: Some(p101a_id.to_string()),
            label: "Open Feed System".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Drill down from overview into Feed Preparation graphic".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-02".to_string(),
            source_display_id: "disp-nht-overview".to_string(),
            source_element_id: Some(r101_id.to_string()),
            target_display_id: "disp-nht-rx".to_string(),
            target_element_id: Some(r101_id.to_string()),
            label: "Open Reaction Train".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Drill down into Combined Feed Exchanger and Reactor section".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-03".to_string(),
            source_display_id: "disp-nht-overview".to_string(),
            source_element_id: Some(v102_id.to_string()),
            target_display_id: "disp-nht-sep".to_string(),
            target_element_id: Some(v102_id.to_string()),
            label: "Open HP Separation".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Drill down into High Pressure Separator 72-V-02 graphic".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-04".to_string(),
            source_display_id: "disp-nht-overview".to_string(),
            source_element_id: Some(k101a_id.to_string()),
            target_display_id: "disp-nht-h2-recycle".to_string(),
            target_element_id: Some(k101a_id.to_string()),
            label: "Open Recycle Gas System".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Drill down into Recycle Gas Compression 72-K-01 graphic".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-05".to_string(),
            source_display_id: "disp-nht-overview".to_string(),
            source_element_id: Some(c101_id.to_string()),
            target_display_id: "disp-nht-strip".to_string(),
            target_element_id: Some(c101_id.to_string()),
            label: "Open Stripper Section".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Drill down into Stripper Column 72-C-01 graphic".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-06".to_string(),
            source_display_id: "disp-nht-overview".to_string(),
            source_element_id: Some(c102_id.to_string()),
            target_display_id: "disp-nht-split".to_string(),
            target_element_id: Some(c102_id.to_string()),
            label: "Open Splitter Section".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Drill down into Naphtha Splitter 72-C-02 graphic".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-07".to_string(),
            source_display_id: "disp-nht-feed".to_string(),
            source_element_id: Some(mix101_id.to_string()),
            target_display_id: "disp-nht-rx".to_string(),
            target_element_id: Some(e101a_id.to_string()),
            label: "Follow Stream to Combined Feed Exchangers".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Process stream continuation from Mix Tee into Exchanger Train".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-08".to_string(),
            source_display_id: "disp-nht-rx".to_string(),
            source_element_id: Some(r101_id.to_string()),
            target_display_id: "disp-nht-rx-detail".to_string(),
            target_element_id: Some(r101_id.to_string()),
            label: "Inspect Reactor 72-R-01 Bed Profile".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Detailed view of multi-point thermocouples and bed Delta T".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-09".to_string(),
            source_display_id: "disp-nht-rx".to_string(),
            source_element_id: Some(r101_id.to_string()),
            target_display_id: "disp-nht-sep".to_string(),
            target_element_id: Some(v102_id.to_string()),
            label: "Follow Effluent to HP Separator".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Follow cooled effluent stream into 72-V-02".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-10".to_string(),
            source_display_id: "disp-nht-sep".to_string(),
            source_element_id: Some(v103_id.to_string()),
            target_display_id: "disp-nht-h2-recycle".to_string(),
            target_element_id: Some(k101a_id.to_string()),
            label: "Follow Vapor to Recycle Compressor".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Follow separator overhead gas to Recycle Compressor suction".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-11".to_string(),
            source_display_id: "disp-nht-sep".to_string(),
            source_element_id: Some(lv101_id.to_string()),
            target_display_id: "disp-nht-strip".to_string(),
            target_element_id: Some(e102a_id.to_string()),
            label: "Follow Bottoms to Stripper Train".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Follow hydrocarbon liquid into Stripper Feed Exchanger E-102A".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-12".to_string(),
            source_display_id: "disp-nht-h2-recycle".to_string(),
            source_element_id: Some(k101a_id.to_string()),
            target_display_id: "disp-nht-comp-detail".to_string(),
            target_element_id: Some(k101a_id.to_string()),
            label: "Inspect 72-K-01 Compressor Performance".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Compressor map, anti-surge operating point, and vibration sensors".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-13".to_string(),
            source_display_id: "disp-nht-h2-recycle".to_string(),
            source_element_id: Some(split101_id.to_string()),
            target_display_id: "disp-nht-rx".to_string(),
            target_element_id: Some(mix101_id.to_string()),
            label: "Follow Recycle Gas to Mix Tee".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Recycle gas return loop to fresh feed injection".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-14".to_string(),
            source_display_id: "disp-nht-strip".to_string(),
            source_element_id: Some(c101_id.to_string()),
            target_display_id: "disp-nht-split".to_string(),
            target_element_id: Some(c102_id.to_string()),
            label: "Follow Stripper Bottoms to Splitter".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Stabilized naphtha transfer to Naphtha Splitter Column 72-C-02".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-15".to_string(),
            source_display_id: "disp-nht-split".to_string(),
            source_element_id: Some(c102_id.to_string()),
            target_display_id: "disp-nht-overview".to_string(),
            target_element_id: Some(tk101_id.to_string()),
            label: "Return to NHT Overview".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Return to plantwide overview from Splitter graphic".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-16".to_string(),
            source_display_id: "disp-nht-comp-detail".to_string(),
            source_element_id: Some(k101a_id.to_string()),
            target_display_id: "disp-nht-h2-recycle".to_string(),
            target_element_id: Some(k101a_id.to_string()),
            label: "Return to Compression Overview".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Exit detail graphic back to Recycle System graphic".to_string()),
        },
        ProjectNavigationLink {
            id: "nav-nht-17".to_string(),
            source_display_id: "disp-nht-rx-detail".to_string(),
            source_element_id: Some(r101_id.to_string()),
            target_display_id: "disp-nht-rx".to_string(),
            target_element_id: Some(r101_id.to_string()),
            label: "Return to Reactor Section".to_string(),
            behavior: "focus_and_center".to_string(),
            description: Some("Exit detail graphic back to Reactor Train graphic".to_string()),
        },
    ];

    project.displays = displays.clone();
    project.navigation_links = navigation_links;

    project.plant_structure = Some(ProjectPlantStructure {
        plant_name: "Naphtha Hydrotreater Complex".to_string(),
        areas: vec![ProjectPlantArea {
            id: "area-nht".to_string(),
            name: "Area 72 Hydroprocessing".to_string(),
            description: Some("High-pressure hydrotreating and desulfurization complex".to_string()),
            systems: vec![
                sys_feed, sys_h2, sys_rx, sys_sep, sys_recycle,
                sys_strip, sys_split, sys_water, sys_steam, sys_sour,
                sys_offgas, sys_bl,
            ],
        }],
        displays,
    });

    project
}
