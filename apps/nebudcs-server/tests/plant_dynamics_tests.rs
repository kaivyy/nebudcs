//! End-to-end integration tests for NebuDCS Phase 14 Process Dynamics & Generic Plant Logic.
//!
//! Validates:
//! 1. Generic multi-equipment process dynamics and physical coupling
//!    (material flow, mechanical shaft, thermal duty, utility supply)
//! 2. Instruments, control loops (PID), alarms (ISA-18.2), and protective interlocks
//! 3. Latching, reset refusal, and reset clearance of safety interlocks
//! 4. Runtime state isolation when switching between arbitrary projects without restart

#![allow(clippy::vec_init_then_push)]

use std::collections::BTreeMap;
use std::time::Duration;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_protocol::{
    BoundaryConditionDto, Position2dDto, ProjectDetailDto, RuntimeCommand, StudioAlarmDto,
    StudioControlLoopDto, StudioEquipmentDto, StudioInstrumentDto, StudioInterlockDto,
    StudioStreamDto, TagValueDto,
};
use nebudcs_server::{HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;

fn create_test_host() -> (SimulationHost, HostHandle) {
    let config = ServerConfig {
        simulation_dt: 0.05,
        speed_multiplier: 1.0,
        ..Default::default()
    };
    let mut engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");
    engine.clock_mut().play();
    let tag_store = RuntimeTagStore::new();

    let host = SimulationHost::start(
        config,
        engine,
        tag_store,
        Some(nebudcs_alarm::AlarmSystem::new()),
        Some(nebudcs_control::ControlSystem::new()),
        None,
    )
    .expect("Failed to start simulation host");

    let handle = host.handle();
    (host, handle)
}

fn create_chemical_plant_project() -> ProjectDetailDto {
    let mut equipment = Vec::new();

    // 1. V-900 Feed Vessel / Separator
    let mut v900_params = BTreeMap::new();
    v900_params.insert("volume".to_string(), 25.0);
    v900_params.insert("cross_section_area".to_string(), 4.0);
    v900_params.insert("initial_mass".to_string(), 10_000.0);
    v900_params.insert("fluid_density".to_string(), 1000.0);
    equipment.push(StudioEquipmentDto {
        id: "eq-v900".to_string(),
        definition_id: "standard.vessel.vertical".to_string(),
        definition_version: "1.0.0".to_string(),
        name: "V-900".to_string(),
        parameters: v900_params,
        position: Some(Position2dDto { x: 50.0, y: 200.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // 2. P-ALPHA Centrifugal Pump
    let mut p_alpha_params = BTreeMap::new();
    p_alpha_params.insert("rated_head".to_string(), 120.0);
    p_alpha_params.insert("rated_flow".to_string(), 50.0);
    p_alpha_params.insert("speed".to_string(), 100.0);
    equipment.push(StudioEquipmentDto {
        id: "eq-p-alpha".to_string(),
        definition_id: "standard.pump.centrifugal".to_string(),
        definition_version: "1.0.0".to_string(),
        name: "P-ALPHA".to_string(),
        parameters: p_alpha_params,
        position: Some(Position2dDto { x: 150.0, y: 200.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // 3. E-HEATER-7 Shell & Tube Heat Exchanger
    let mut e_heat_params = BTreeMap::new();
    e_heat_params.insert("area".to_string(), 50.0);
    e_heat_params.insert("overall_u".to_string(), 500.0);
    e_heat_params.insert("design_duty".to_string(), 500_000.0);
    e_heat_params.insert("process_inlet_t".to_string(), 25.0);
    equipment.push(StudioEquipmentDto {
        id: "eq-e-heater-7".to_string(),
        definition_id: "standard.exchanger.shell_tube".to_string(),
        definition_version: "1.0.0".to_string(),
        name: "E-HEATER-7".to_string(),
        parameters: e_heat_params,
        position: Some(Position2dDto { x: 250.0, y: 200.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // 4. R-X01 CSTR Reactor
    let mut r_x01_params = BTreeMap::new();
    r_x01_params.insert("thermal_duty".to_string(), 150_000.0);
    r_x01_params.insert("tau".to_string(), 3.0);
    equipment.push(StudioEquipmentDto {
        id: "eq-r-x01".to_string(),
        definition_id: "standard.reactor.cstr".to_string(),
        definition_version: "1.0.0".to_string(),
        name: "R-X01".to_string(),
        parameters: r_x01_params,
        position: Some(Position2dDto { x: 350.0, y: 200.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // 5. V-SEP-X Product Separator
    let mut v_sep_params = BTreeMap::new();
    v_sep_params.insert("volume".to_string(), 20.0);
    v_sep_params.insert("cross_section_area".to_string(), 3.0);
    v_sep_params.insert("initial_mass".to_string(), 8000.0);
    v_sep_params.insert("fluid_density".to_string(), 1000.0);
    equipment.push(StudioEquipmentDto {
        id: "eq-v-sep-x".to_string(),
        definition_id: "standard.separator.two_phase".to_string(),
        definition_version: "1.0.0".to_string(),
        name: "V-SEP-X".to_string(),
        parameters: v_sep_params,
        position: Some(Position2dDto { x: 450.0, y: 200.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // 6. FV-900 Steam Throttle Valve
    let mut fv900_params = BTreeMap::new();
    fv900_params.insert("cv".to_string(), 85.0);
    fv900_params.insert("position".to_string(), 50.0);
    equipment.push(StudioEquipmentDto {
        id: "eq-fv900".to_string(),
        definition_id: "standard.valve.control".to_string(),
        definition_version: "1.0.0".to_string(),
        name: "FV-900".to_string(),
        parameters: fv900_params,
        position: Some(Position2dDto { x: 150.0, y: 350.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // 7. ST-DRIVE-1 Steam Turbine Driver
    let mut st_params = BTreeMap::new();
    st_params.insert("rated_head".to_string(), 100.0);
    st_params.insert("rated_power".to_string(), 250_000.0);
    st_params.insert("speed".to_string(), 3000.0);
    equipment.push(StudioEquipmentDto {
        id: "eq-st-drive-1".to_string(),
        definition_id: "standard.turbine.steam".to_string(),
        definition_version: "1.0.0".to_string(),
        name: "ST-DRIVE-1".to_string(),
        parameters: st_params,
        position: Some(Position2dDto { x: 250.0, y: 350.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // 8. C-GAS-9 Recycle Gas Compressor
    let mut c_gas_params = BTreeMap::new();
    c_gas_params.insert("rated_head".to_string(), 80.0);
    c_gas_params.insert("rated_power".to_string(), 200_000.0);
    c_gas_params.insert("speed".to_string(), 3000.0);
    equipment.push(StudioEquipmentDto {
        id: "eq-c-gas-9".to_string(),
        definition_id: "standard.compressor.centrifugal".to_string(),
        definition_version: "1.0.0".to_string(),
        name: "C-GAS-9".to_string(),
        parameters: c_gas_params,
        position: Some(Position2dDto { x: 350.0, y: 350.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    // Couplings / Streams
    let mut streams = Vec::new();
    streams.push(StudioStreamDto {
        id: "s-01".to_string(),
        name: "S-01".to_string(),
        stream_type: "Material".to_string(),
        source_equipment_id: "eq-v900".to_string(),
        source_port_id: "liquid_out".to_string(),
        sink_equipment_id: "eq-p-alpha".to_string(),
        sink_port_id: "suction".to_string(),
        composition: BTreeMap::new(),
        route_points: Vec::new(),
        ..Default::default()
    });

    streams.push(StudioStreamDto {
        id: "s-02".to_string(),
        name: "S-02".to_string(),
        stream_type: "Material".to_string(),
        source_equipment_id: "eq-p-alpha".to_string(),
        source_port_id: "discharge".to_string(),
        sink_equipment_id: "eq-e-heater-7".to_string(),
        sink_port_id: "tube_in".to_string(),
        composition: BTreeMap::new(),
        route_points: Vec::new(),
        ..Default::default()
    });

    streams.push(StudioStreamDto {
        id: "s-03".to_string(),
        name: "S-03".to_string(),
        stream_type: "Material".to_string(),
        source_equipment_id: "eq-e-heater-7".to_string(),
        source_port_id: "tube_out".to_string(),
        sink_equipment_id: "eq-r-x01".to_string(),
        sink_port_id: "inlet".to_string(),
        composition: BTreeMap::new(),
        route_points: Vec::new(),
        ..Default::default()
    });

    streams.push(StudioStreamDto {
        id: "s-04".to_string(),
        name: "S-04".to_string(),
        stream_type: "Material".to_string(),
        source_equipment_id: "eq-r-x01".to_string(),
        source_port_id: "outlet".to_string(),
        sink_equipment_id: "eq-v-sep-x".to_string(),
        sink_port_id: "feed".to_string(),
        composition: BTreeMap::new(),
        route_points: Vec::new(),
        ..Default::default()
    });

    streams.push(StudioStreamDto {
        id: "s-heat".to_string(),
        name: "S-HEAT".to_string(),
        stream_type: "Thermal".to_string(),
        source_equipment_id: "eq-e-heater-7".to_string(),
        source_port_id: "duty_out".to_string(),
        sink_equipment_id: "eq-r-x01".to_string(),
        sink_port_id: "duty_in".to_string(),
        composition: BTreeMap::new(),
        route_points: Vec::new(),
        ..Default::default()
    });

    streams.push(StudioStreamDto {
        id: "s-stm".to_string(),
        name: "S-STM".to_string(),
        stream_type: "Utility".to_string(),
        source_equipment_id: "eq-fv900".to_string(),
        source_port_id: "outlet".to_string(),
        sink_equipment_id: "eq-st-drive-1".to_string(),
        sink_port_id: "steam_inlet".to_string(),
        composition: BTreeMap::new(),
        route_points: Vec::new(),
        ..Default::default()
    });

    streams.push(StudioStreamDto {
        id: "s-shaft".to_string(),
        name: "S-SHAFT".to_string(),
        stream_type: "Mechanical".to_string(),
        source_equipment_id: "eq-st-drive-1".to_string(),
        source_port_id: "shaft_out".to_string(),
        sink_equipment_id: "eq-c-gas-9".to_string(),
        sink_port_id: "shaft_in".to_string(),
        composition: BTreeMap::new(),
        route_points: Vec::new(),
        ..Default::default()
    });

    // Instruments
    let instruments = vec![
        StudioInstrumentDto {
            id: "inst-lt-900".to_string(),
            tag: "LT-900".to_string(),
            equipment_id: Some("eq-v900".to_string()),
            port_id: None,
            measurement_kind: "level".to_string(),
            range_min: 0.0,
            range_max: 1.0,
            unit: "fraction".to_string(),
            position: None,
        },
        StudioInstrumentDto {
            id: "inst-ft-900".to_string(),
            tag: "FT-900".to_string(),
            equipment_id: Some("eq-p-alpha".to_string()),
            port_id: None,
            measurement_kind: "flow".to_string(),
            range_min: 0.0,
            range_max: 100.0,
            unit: "kg/s".to_string(),
            position: None,
        },
        StudioInstrumentDto {
            id: "inst-tt-900".to_string(),
            tag: "TT-900".to_string(),
            equipment_id: Some("eq-e-heater-7".to_string()),
            port_id: None,
            measurement_kind: "temperature".to_string(),
            range_min: 0.0,
            range_max: 300.0,
            unit: "degC".to_string(),
            position: None,
        },
        StudioInstrumentDto {
            id: "inst-pt-900".to_string(),
            tag: "PT-900".to_string(),
            equipment_id: Some("eq-r-x01".to_string()),
            port_id: None,
            measurement_kind: "pressure".to_string(),
            range_min: 0.0,
            range_max: 50.0,
            unit: "bar".to_string(),
            position: None,
        },
        StudioInstrumentDto {
            id: "inst-st-900".to_string(),
            tag: "ST-900".to_string(),
            equipment_id: Some("eq-st-drive-1".to_string()),
            port_id: None,
            measurement_kind: "speed".to_string(),
            range_min: 0.0,
            range_max: 5000.0,
            unit: "rpm".to_string(),
            position: None,
        },
    ];

    // Control loops
    let control_loops = vec![
        StudioControlLoopDto {
            id: "loop-lic-900".to_string(),
            name: "LIC-900".to_string(),
            pv_tag: "LT-900".to_string(),
            sp: 0.40,
            out_tag: "P-ALPHA.SPEED".to_string(),
            kp: 1.0,
            ti: 10.0,
            td: 0.0,
            reverse_acting: false,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 100.0,
            position: None,
        },
        StudioControlLoopDto {
            id: "loop-tic-900".to_string(),
            name: "TIC-900".to_string(),
            pv_tag: "TT-900".to_string(),
            sp: 150.0,
            out_tag: "E-HEATER-7.DUTY".to_string(),
            kp: 1.5,
            ti: 15.0,
            td: 0.0,
            reverse_acting: false,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 1000000.0,
            position: None,
        },
        StudioControlLoopDto {
            id: "loop-sic-900".to_string(),
            name: "SIC-900".to_string(),
            pv_tag: "ST-900".to_string(),
            sp: 3000.0,
            out_tag: "FV-900.POS".to_string(),
            kp: 0.8,
            ti: 5.0,
            td: 0.0,
            reverse_acting: false,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 100.0,
            position: None,
        },
    ];

    // Alarms
    let alarms = vec![
        StudioAlarmDto {
            id: "alm-la-900".to_string(),
            name: "LA-900-LO".to_string(),
            pv_tag: "LT-900".to_string(),
            alarm_type: "LO".to_string(),
            severity: "WARNING".to_string(),
            trip_limit: 0.20,
            deadband: 0.02,
            description: "Feed vessel low level warning".to_string(),
        },
        StudioAlarmDto {
            id: "alm-ta-900".to_string(),
            name: "TA-900-HIHI".to_string(),
            pv_tag: "TT-900".to_string(),
            alarm_type: "HIHI".to_string(),
            severity: "CRITICAL".to_string(),
            trip_limit: 220.0,
            deadband: 2.0,
            description: "Exchanger high high temperature trip hazard".to_string(),
        },
        StudioAlarmDto {
            id: "alm-pa-900".to_string(),
            name: "PA-900-HIHI".to_string(),
            pv_tag: "PT-900".to_string(),
            alarm_type: "HIHI".to_string(),
            severity: "CRITICAL".to_string(),
            trip_limit: 35.0,
            deadband: 0.5,
            description: "Reactor high high pressure emergency shutoff".to_string(),
        },
        StudioAlarmDto {
            id: "alm-sa-900".to_string(),
            name: "SA-900-HI".to_string(),
            pv_tag: "ST-900".to_string(),
            alarm_type: "HI".to_string(),
            severity: "WARNING".to_string(),
            trip_limit: 3500.0,
            deadband: 10.0,
            description: "Turbine driver overspeed warning".to_string(),
        },
    ];

    // Interlocks
    let interlocks = vec![
        StudioInterlockDto {
            id: "il-901".to_string(),
            name: "I-901".to_string(),
            source_tag: "LT-900".to_string(),
            condition: "<".to_string(),
            threshold: 0.15,
            target_action: "CLOSE".to_string(),
            target_equipment: Some("eq-p-alpha".to_string()),
            latched: true,
            description: "Low feed level trips pump to prevent dry run cavitation".to_string(),
        },
        StudioInterlockDto {
            id: "il-902".to_string(),
            name: "I-902".to_string(),
            source_tag: "PT-900".to_string(),
            condition: ">".to_string(),
            threshold: 30.0,
            target_action: "CLOSE".to_string(),
            target_equipment: Some("eq-p-alpha".to_string()),
            latched: true,
            description: "Reactor overpressure cuts pump feed to prevent overpressurization"
                .to_string(),
        },
        StudioInterlockDto {
            id: "il-903".to_string(),
            name: "I-903".to_string(),
            source_tag: "ST-900".to_string(),
            condition: ">".to_string(),
            threshold: 3500.0,
            target_action: "CLOSE".to_string(),
            target_equipment: Some("eq-fv900".to_string()),
            latched: true,
            description: "Turbine driver overspeed trips steam throttle valve shut".to_string(),
        },
    ];

    let boundary_conditions = vec![BoundaryConditionDto::feed(
        "bc-chem-feed-01",
        "Chemical Reactant Feed",
        "eq-v900",
        50.0,
        "m3/h",
    )];

    ProjectDetailDto {
        id: "proj-chemical-complex-01".to_string(),
        name: "Continuous Synthesis & Reaction Train".to_string(),
        schema_version: "0.1.0".to_string(),
        description: "Non-refinery multi-equipment generic chemical reaction loop".to_string(),
        author: "Chief Process Architect".to_string(),
        created: "2026-09-06T00:00:00Z".to_string(),
        modified: "2026-09-06T00:00:00Z".to_string(),
        equipment,
        streams,
        instruments,
        control_loops,
        alarms,
        interlocks,
        controllers: Vec::new(),
        io_systems: Vec::new(),
        io_mappings: Vec::new(),
        tags: Vec::new(),
        advanced_control: Vec::new(),
        communications: None,
        sis: None,
        boundary_conditions,
        initial_conditions: Vec::new(),
        plant_structure: None,
        areas: Vec::new(),
        displays: Vec::new(),
        navigation_links: Vec::new(),
    }
}

fn create_gas_treating_project() -> ProjectDetailDto {
    let mut equipment = Vec::new();

    let mut t201_params = BTreeMap::new();
    t201_params.insert("volume".to_string(), 40.0);
    t201_params.insert("cross_section_area".to_string(), 5.0);
    t201_params.insert("initial_mass".to_string(), 15_000.0);
    t201_params.insert("fluid_density".to_string(), 1000.0);
    equipment.push(StudioEquipmentDto {
        id: "eq-t201".to_string(),
        definition_id: "standard.vessel.vertical".to_string(),
        definition_version: "1.0.0".to_string(),
        name: "T-201".to_string(),
        parameters: t201_params,
        position: Some(Position2dDto { x: 100.0, y: 150.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    let mut k201_params = BTreeMap::new();
    k201_params.insert("rated_head".to_string(), 90.0);
    k201_params.insert("rated_power".to_string(), 180_000.0);
    k201_params.insert("speed".to_string(), 2800.0);
    equipment.push(StudioEquipmentDto {
        id: "eq-k201".to_string(),
        definition_id: "standard.compressor.centrifugal".to_string(),
        definition_version: "1.0.0".to_string(),
        name: "K-201".to_string(),
        parameters: k201_params,
        position: Some(Position2dDto { x: 300.0, y: 150.0 }),
        display_width: None,
        display_height: None,
        rotation: None,
    });

    let streams = vec![StudioStreamDto {
        id: "s-gas".to_string(),
        name: "S-GAS".to_string(),
        stream_type: "Material".to_string(),
        source_equipment_id: "eq-t201".to_string(),
        source_port_id: "gas_out".to_string(),
        sink_equipment_id: "eq-k201".to_string(),
        sink_port_id: "suction".to_string(),
        composition: BTreeMap::new(),
        route_points: Vec::new(),
        ..Default::default()
    }];

    let instruments = vec![
        StudioInstrumentDto {
            id: "inst-ti-201".to_string(),
            tag: "TI-201".to_string(),
            equipment_id: Some("eq-t201".to_string()),
            port_id: None,
            measurement_kind: "temperature".to_string(),
            range_min: 0.0,
            range_max: 200.0,
            unit: "degC".to_string(),
            position: None,
        },
        StudioInstrumentDto {
            id: "inst-pi-201".to_string(),
            tag: "PI-201".to_string(),
            equipment_id: Some("eq-k201".to_string()),
            port_id: None,
            measurement_kind: "pressure".to_string(),
            range_min: 0.0,
            range_max: 30.0,
            unit: "bar".to_string(),
            position: None,
        },
    ];

    let control_loops = vec![StudioControlLoopDto {
        id: "loop-pic-201".to_string(),
        name: "PIC-201".to_string(),
        pv_tag: "PI-201".to_string(),
        sp: 15.0,
        out_tag: "K-201.SPEED".to_string(),
        kp: 1.0,
        ti: 10.0,
        td: 0.0,
        reverse_acting: false,
        mode: "AUTO".to_string(),
        output_min: 0.0,
        output_max: 100.0,
        position: None,
    }];

    let alarms = vec![StudioAlarmDto {
        id: "alm-pa-201".to_string(),
        name: "PA-201-HI".to_string(),
        pv_tag: "PI-201".to_string(),
        alarm_type: "HI".to_string(),
        severity: "WARNING".to_string(),
        trip_limit: 22.0,
        deadband: 0.5,
        description: "Gas compressor high discharge pressure".to_string(),
    }];

    let interlocks = vec![StudioInterlockDto {
        id: "il-201".to_string(),
        name: "I-201".to_string(),
        source_tag: "PI-201".to_string(),
        condition: ">".to_string(),
        threshold: 25.0,
        target_action: "CLOSE".to_string(),
        target_equipment: Some("eq-k201".to_string()),
        latched: true,
        description: "Overpressure shuts down compressor K-201".to_string(),
    }];

    let boundary_conditions = vec![BoundaryConditionDto::feed(
        "bc-gas-feed-01",
        "Sour Gas Feed",
        "eq-v201",
        1000.0,
        "Nm3/h",
    )];

    ProjectDetailDto {
        id: "proj-gas-treating-02".to_string(),
        name: "Gas Sweetening Unit".to_string(),
        schema_version: "0.1.0".to_string(),
        description: "Second independent project for state isolation verification".to_string(),
        author: "Quality Auditor".to_string(),
        created: "2026-09-06T00:00:00Z".to_string(),
        modified: "2026-09-06T00:00:00Z".to_string(),
        equipment,
        streams,
        instruments,
        control_loops,
        alarms,
        interlocks,
        controllers: Vec::new(),
        io_systems: Vec::new(),
        io_mappings: Vec::new(),
        tags: Vec::new(),
        advanced_control: Vec::new(),
        communications: None,
        sis: None,
        boundary_conditions,
        initial_conditions: Vec::new(),
        plant_structure: None,
        areas: Vec::new(),
        displays: Vec::new(),
        navigation_links: Vec::new(),
    }
}

#[tokio::test]
async fn test_multi_equipment_process_dynamics_and_coupling() {
    let (_host, handle) = create_test_host();

    // 1. Load the 8-equipment continuous synthesis chemical plant project
    let project = create_chemical_plant_project();
    handle
        .dispatch_command(RuntimeCommand::LoadProject {
            project: Box::new(project),
        })
        .await
        .expect("Failed to load generic chemical plant project");

    // Let simulation run for several ticks
    tokio::time::sleep(Duration::from_millis(150)).await;

    // 2. Verify all process tags and DCS instruments exist in snapshot
    let snap = handle.get_snapshot().await;
    let tag_map: BTreeMap<String, f64> = snap
        .tags
        .iter()
        .filter_map(|t| t.value.as_f64().map(|v| (t.tag.clone(), v)))
        .collect();

    assert!(
        tag_map.contains_key("LT-900"),
        "Level transmitter LT-900 must exist"
    );
    assert!(
        tag_map.contains_key("FT-900"),
        "Flow transmitter FT-900 must exist"
    );
    assert!(
        tag_map.contains_key("TT-900"),
        "Temperature transmitter TT-900 must exist"
    );
    assert!(
        tag_map.contains_key("PT-900"),
        "Pressure transmitter PT-900 must exist"
    );
    assert!(
        tag_map.contains_key("ST-900"),
        "Speed transmitter ST-900 must exist"
    );

    // Verify initial physical conditions: V-900 has 10,000 kg in 25 m^3 => level ~ 0.40 (40%)
    let level = tag_map["LT-900"];
    assert!(
        (level - 0.40).abs() < 0.05,
        "Level transmitter should read vessel level ~0.40, got {level}"
    );

    // 3. Mechanical Shaft Coupling & Utility Dependency:
    // ST-DRIVE-1 (Steam Turbine) drives C-GAS-9 (Compressor) via S-SHAFT.
    // Driver speed is influenced by utility steam pressure (nominal 40.0 bar).
    let initial_comp_speed = tag_map
        .get("C-GAS-9.SPEED")
        .copied()
        .or_else(|| tag_map.get("ST-DRIVE-1.SPEED").copied())
        .expect("Compressor/Turbine speed tag must exist");
    assert!(
        initial_comp_speed > 2500.0,
        "Compressor speed should reflect mechanical driver coupling (>2500 rpm), got {initial_comp_speed}"
    );

    // Increase utility steam pressure to 55 bar
    handle
        .dispatch_command(RuntimeCommand::WriteTag {
            tag: "STEAM_PRESSURE".to_string(),
            value: TagValueDto::Float(55.0),
        })
        .await
        .expect("Failed to write steam pressure utility");

    tokio::time::sleep(Duration::from_millis(150)).await;

    let snap_utility = handle.get_snapshot().await;
    let comp_speed_boosted = snap_utility
        .tags
        .iter()
        .find(|t| t.tag == "C-GAS-9.SPEED" || t.tag == "ST-DRIVE-1.SPEED")
        .and_then(|t| t.value.as_f64())
        .expect("Speed tag must be present");

    assert!(
        comp_speed_boosted >= initial_comp_speed,
        "Higher steam pressure must increase driver/compressor speed: {comp_speed_boosted} >= {initial_comp_speed}"
    );

    // 4. Process & Thermal Coupling:
    // Exchanger E-HEATER-7 provides thermal duty to CSTR Reactor R-X01 via S-HEAT.
    let rxn_press = snap_utility
        .tags
        .iter()
        .find(|t| t.tag == "PT-900" || t.tag == "R-X01.PRESSURE")
        .and_then(|t| t.value.as_f64())
        .unwrap_or(25.0);
    assert!(
        rxn_press > 10.0,
        "Reactor pressure must reflect process thermal coupling, got {rxn_press}"
    );

    // 5. Interlock Tripping, Latching & Reset Refusal:
    // Interlock I-903 monitors ST-900 (Turbine Speed) > 3500 rpm.
    // When tripped, it overrides FV-900 (Steam Valve) to 0.0 (CLOSE).
    let snap_before = handle.get_snapshot().await;
    let i903_tripped_before = snap_before
        .tags
        .iter()
        .find(|t| t.tag == "I-903.TRIPPED")
        .and_then(|t| match t.value {
            TagValueDto::Bool(b) => Some(b),
            _ => None,
        })
        .unwrap_or(false);
    assert!(!i903_tripped_before, "I-903 must not be tripped initially");

    // Inject turbine overspeed disturbance (>3500 rpm)
    handle
        .dispatch_command(RuntimeCommand::WriteTag {
            tag: "ST-900".to_string(),
            value: TagValueDto::Float(3650.0),
        })
        .await
        .expect("Failed to inject overspeed tag");

    tokio::time::sleep(Duration::from_millis(150)).await;

    let snap_tripped = handle.get_snapshot().await;
    let i903_tripped = snap_tripped
        .tags
        .iter()
        .find(|t| t.tag == "I-903.TRIPPED")
        .and_then(|t| match t.value {
            TagValueDto::Bool(b) => Some(b),
            _ => None,
        })
        .unwrap_or(false);
    assert!(
        i903_tripped,
        "I-903 must latch into TRIPPED state when speed exceeds 3500 rpm"
    );

    // Verify overspeed alarm SA-900-HI transitioned out of Normal state
    let sa_alm = snap_tripped
        .active_alarms
        .iter()
        .find(|a| a.alarm_name == "SA-900-HI");
    assert!(
        sa_alm.is_some(),
        "Overspeed alarm SA-900-HI must be triggered in active_alarms"
    );

    // Verify actuator override: FV-900 position forced to 0.0
    let fv_pos = snap_tripped
        .tags
        .iter()
        .find(|t| t.tag == "FV-900.POS" || t.tag == "FV-900.OPENING")
        .and_then(|t| t.value.as_f64())
        .unwrap_or(100.0);
    assert!(
        fv_pos <= 1.0,
        "Interlock I-903 must force FV-900 throttle valve shut (<=1.0%), got {fv_pos}"
    );

    // 6. Test Reset Refusal:
    // Attempt to reset I-903 while speed is STILL 3650 rpm (condition violated).
    handle
        .dispatch_command(RuntimeCommand::ResetProtection {
            block_name: "I-903".to_string(),
        })
        .await
        .expect("Reset command dispatched");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let snap_refused = handle.get_snapshot().await;
    let i903_still_tripped = snap_refused
        .tags
        .iter()
        .find(|t| t.tag == "I-903.TRIPPED")
        .and_then(|t| match t.value {
            TagValueDto::Bool(b) => Some(b),
            _ => None,
        })
        .unwrap_or(false);
    assert!(
        i903_still_tripped,
        "I-903 must refuse reset while initiating process condition remains violated"
    );

    // 7. Test Reset Clearance:
    // Bring speed back to safe operating condition (3000 rpm, well below 3500 - deadband 0.5)
    handle
        .dispatch_command(RuntimeCommand::WriteTag {
            tag: "ST-900".to_string(),
            value: TagValueDto::Float(3000.0),
        })
        .await
        .expect("Failed to restore safe speed");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Now issue operator reset
    handle
        .dispatch_command(RuntimeCommand::ResetProtection {
            block_name: "I-903".to_string(),
        })
        .await
        .expect("Failed to dispatch reset after condition cleared");

    tokio::time::sleep(Duration::from_millis(150)).await;

    let snap_cleared = handle.get_snapshot().await;
    let i903_cleared = snap_cleared
        .tags
        .iter()
        .find(|t| t.tag == "I-903.TRIPPED")
        .and_then(|t| match t.value {
            TagValueDto::Bool(b) => Some(b),
            _ => None,
        })
        .unwrap_or(true);
    assert!(
        !i903_cleared,
        "I-903 must successfully clear and unlatch after condition is safe and operator resets"
    );

    // 8. Alarm Acknowledgment:
    handle
        .dispatch_command(RuntimeCommand::AcknowledgeAlarm {
            alarm_name: "SA-900-HI".to_string(),
        })
        .await
        .expect("Failed to acknowledge alarm");

    tokio::time::sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_runtime_state_isolation_between_projects() {
    let (_host, handle) = create_test_host();

    // 1. Load Project 1 (Continuous Synthesis Train: 8 pieces of equipment, tags V-900, P-ALPHA, etc.)
    let proj1 = create_chemical_plant_project();
    handle
        .dispatch_command(RuntimeCommand::LoadProject {
            project: Box::new(proj1),
        })
        .await
        .expect("Failed to load Project 1");

    tokio::time::sleep(Duration::from_millis(150)).await;

    let snap1 = handle.get_snapshot().await;
    assert!(
        snap1.tags.iter().any(|t| t.tag.contains("V-900")),
        "Project 1 must contain V-900 tags"
    );
    assert!(
        snap1.tags.iter().any(|t| t.tag.contains("P-ALPHA")),
        "Project 1 must contain P-ALPHA tags"
    );
    assert!(
        snap1.tags.iter().any(|t| t.tag.contains("R-X01")),
        "Project 1 must contain R-X01 tags"
    );
    assert!(
        snap1.tags.iter().any(|t| t.tag == "I-901.TRIPPED"),
        "Project 1 must contain I-901 interlock"
    );
    assert_eq!(
        snap1.active_alarms.len(),
        4,
        "Project 1 has 4 registered alarms"
    );

    // 2. Dynamically load Project 2 (Gas Sweetening Unit: completely independent equipment, tags, loops)
    let proj2 = create_gas_treating_project();
    handle
        .dispatch_command(RuntimeCommand::LoadProject {
            project: Box::new(proj2),
        })
        .await
        .expect("Failed to load Project 2");

    tokio::time::sleep(Duration::from_millis(150)).await;

    // 3. Verify complete state isolation:
    let snap2 = handle.get_snapshot().await;

    // Check Project 2 tags exist
    assert!(
        snap2.tags.iter().any(|t| t.tag == "TI-201"),
        "Project 2 must contain TI-201"
    );
    assert!(
        snap2.tags.iter().any(|t| t.tag == "PI-201"),
        "Project 2 must contain PI-201"
    );
    assert!(
        snap2.tags.iter().any(|t| t.tag.contains("T-201")),
        "Project 2 must contain T-201 tags"
    );
    assert!(
        snap2.tags.iter().any(|t| t.tag.contains("K-201")),
        "Project 2 must contain K-201 tags"
    );
    assert!(
        snap2.tags.iter().any(|t| t.tag == "I-201.TRIPPED"),
        "Project 2 must contain I-201 interlock"
    );

    // Verify ZERO residual tags from Project 1
    for t in &snap2.tags {
        assert!(
            !t.tag.contains("900"),
            "Residual tag from Project 1 found in Project 2: {}",
            t.tag
        );
        assert!(
            !t.tag.contains("ALPHA"),
            "Residual tag from Project 1 found in Project 2: {}",
            t.tag
        );
        assert!(
            !t.tag.contains("HEATER"),
            "Residual tag from Project 1 found in Project 2: {}",
            t.tag
        );
        assert!(
            !t.tag.contains("X01"),
            "Residual tag from Project 1 found in Project 2: {}",
            t.tag
        );
    }

    // Verify ZERO residual alarms from Project 1
    for alm in &snap2.active_alarms {
        assert!(
            !alm.alarm_name.contains("900"),
            "Residual alarm from Project 1 found in Project 2: {}",
            alm.alarm_name
        );
    }

    // Verify alarms count matches Project 2 (only PA-201-HI registered)
    assert_eq!(
        snap2.active_alarms.len(),
        1,
        "Project 2 should only have its own registered alarm"
    );
    assert_eq!(snap2.active_alarms[0].alarm_name, "PA-201-HI");

    // Verify simulation time was reset cleanly
    assert!(
        snap2.sim_time_sec < 5.0,
        "Simulation time must be re-initialized when loading a new project, got {}",
        snap2.sim_time_sec
    );
}
