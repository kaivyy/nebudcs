//! Project management and persistence subsystem for the Engineering Studio.

use std::collections::BTreeMap;
use std::str::FromStr;

use indexmap::IndexMap;
use nebudcs_model::equipment::{
    AlarmPriority, AlarmType, EquipmentDefinition, EquipmentInstance, Position2D,
};
use nebudcs_model::project::{
    AlarmInstance, Area, BoundaryConditionInstance, ControlLoopInstance, DisplayElementPlacement,
    InitialConditionInstance, InstrumentInstance, InterlockInstance, Plant, Project,
    ProjectAdvancedControlStrategy, ProjectCommunications, ProjectControlModule, ProjectController,
    ProjectCpuProfile, ProjectDisplay, ProjectFirmwareProfile, ProjectIndustrialZone,
    ProjectIoChannel, ProjectIoChassis, ProjectIoMapping, ProjectIoModule, ProjectIoRack,
    ProjectIoSystem, ProjectMemoryProfile, ProjectMetadata, ProjectNavigationLink, ProjectPlantArea,
    ProjectPlantStructure, ProjectPlantSubsystem, ProjectPlantSystem, ProjectPlantUnit,
    ProjectProgram, ProjectProtocolConnection, ProjectRedundancyConfig, ProjectRoutine,
    ProjectSifDefinition, ProjectSifFinalElement, ProjectSisConfig, ProjectTag, ProjectTagMapping,
    ProjectTask, Unit,
};
use nebudcs_model::stream::{PortRef, StreamComposition, StreamDefinition, StreamType};
use nebudcs_protocol::*;
use nebudcs_topology::validation::{
    validate_project as run_validate_project, ProjectValidationSummary,
};
use nebudcs_types::{
    EntityId, EquipmentDefinitionId, EquipmentId, PortId, ProjectId, SchemaVersion, StreamId,
};

/// Default project identifier for the baseline vertical slice plant.
pub const DEFAULT_PROJECT_ID: &str = "proj-vertical-slice-01";
/// Project identifier for the Refinery Crude Distillation Unit (CDU) demonstration plant.
pub const PROJ_REFINERY_CDU: &str = "proj-domain-refinery-cdu";
/// Project identifier for the Power & Steam mechanical turbine drive plant.
pub const PROJ_POWER_TURBINE: &str = "proj-domain-power-turbine";
/// Project identifier for the Natural Gas compression and conditioning train.
pub const PROJ_GAS_COMPRESSION: &str = "proj-domain-gas-compression";
/// Project identifier for the Chemical continuous exothermic reaction train.
pub const PROJ_CHEM_REACTION: &str = "proj-domain-chem-reaction";
/// Project identifier for the Naphtha Hydrotreater (NHT) reference plant.
pub const PROJ_NHT_REFERENCE: &str = crate::nht_seed::PROJ_NHT_REFERENCE;

/// In-memory and persisted project management engine.
#[derive(Debug, Clone)]
pub struct ProjectManager {
    projects: BTreeMap<String, Project>,
    disk_store: Option<crate::persistence::DiskStore>,
}

impl Default for ProjectManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectManager {
    /// Creates a new ProjectManager pre-seeded with the baseline demonstration project
    /// and the four authoritative industrial domain packages.
    pub fn new() -> Self {
        let mut mgr = Self {
            projects: BTreeMap::new(),
            disk_store: None,
        };
        mgr.seed_default_project();
        mgr.seed_domain_projects();
        mgr
    }

    /// Attaches an atomic DiskStore, loading existing projects from disk and seeding defaults if empty.
    pub fn with_disk_store(mut self, disk_store: crate::persistence::DiskStore) -> Self {
        let on_disk = disk_store.load_all_in_category::<Project>("projects");
        if !on_disk.is_empty() {
            for (id, mut project) in on_disk {
                if id == PROJ_NHT_REFERENCE {
                    // Always enforce the authoritative in-code NHT reference plant seed
                    if let Some(seeded) = self.projects.get(&id) {
                        project = seeded.clone();
                    }
                } else if let Some(seeded) = self.projects.get(&id) {
                    if project.controllers.is_empty() && !seeded.controllers.is_empty() {
                        project.controllers = seeded.controllers.clone();
                    }
                    if project.io_systems.is_empty() && !seeded.io_systems.is_empty() {
                        project.io_systems = seeded.io_systems.clone();
                    }
                    if project.io_mappings.is_empty() && !seeded.io_mappings.is_empty() {
                        project.io_mappings = seeded.io_mappings.clone();
                    }
                    if project.tags.is_empty() && !seeded.tags.is_empty() {
                        project.tags = seeded.tags.clone();
                    }
                    if project.advanced_control.is_empty() && !seeded.advanced_control.is_empty() {
                        project.advanced_control = seeded.advanced_control.clone();
                    }
                    if project.sis.is_none() && seeded.sis.is_some() {
                        project.sis = seeded.sis.clone();
                    }
                }
                self.projects.insert(id, project);
            }
        }
        for (id, project) in &self.projects {
            let _ = disk_store.save_json("projects", id, project);
        }
        self.disk_store = Some(disk_store);
        self
    }

    /// Reloads all projects from attached DiskStore.
    pub fn reload_from_disk(&mut self) -> Result<usize, String> {
        if let Some(ref store) = self.disk_store {
            let loaded = store.load_all_in_category::<Project>("projects");
            let count = loaded.len();
            for (id, p) in loaded {
                self.projects.insert(id, p);
            }
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Reference to raw projects map.
    pub fn raw_projects(&self) -> &BTreeMap<String, Project> {
        &self.projects
    }

    /// Seeds the standard Phase 8/9/10/11 closed-loop surge and separation unit.
    fn seed_default_project(&mut self) {
        let project_id = DEFAULT_PROJECT_ID
            .parse::<ProjectId>()
            .unwrap_or_else(|_| ProjectId::new());
        let mut project = Project::new("NebuDCS Closed-Loop Refinery Unit");
        project.id = project_id;
        project.metadata = ProjectMetadata {
            description: "Standard Level-Controlled Surge Loop: K-101 Pump feeding V-101 Vessel through FV-101 Control Valve".to_string(),
            author: "NebuDCS Engineering Studio".to_string(),
            created: "2026-09-06T00:00:00Z".to_string(),
            modified: "2026-09-06T00:00:00Z".to_string(),
        };

        let mut plant = Plant::new("Process Area 10");
        let mut area = Area::new("Feed Fractionation");
        let mut unit = Unit::new("Unit 100 Surge & Separation");

        let p101_id = EquipmentId::new();
        let fv101_id = EquipmentId::new();
        let v101_id = EquipmentId::new();

        let pump_def_id = "standard.pump.centrifugal"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let valve_def_id = "standard.valve.control_globe"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let vessel_def_id = "standard.vessel.vertical_separator"
            .parse::<EquipmentDefinitionId>()
            .unwrap();

        let _suction_port_id = "suction".parse::<PortId>().unwrap();
        let discharge_port_id = "discharge".parse::<PortId>().unwrap();
        let valve_inlet_id = "inlet".parse::<PortId>().unwrap();
        let valve_outlet_id = "outlet".parse::<PortId>().unwrap();
        let vessel_inlet_id = "feed".parse::<PortId>().unwrap();
        let _vessel_outlet_id = "liquid_out".parse::<PortId>().unwrap();

        // 1. Equipment instances
        let mut p101_params = IndexMap::new();
        p101_params.insert("design_flow".to_string(), 5000.0);
        p101_params.insert("rated_head".to_string(), 120.0);
        p101_params.insert("speed_rpm".to_string(), 2980.0);

        unit.equipment.push(EquipmentInstance {
            id: p101_id,
            definition_id: pump_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "K-101".to_string(),
            parameters: p101_params,
            position: Some(Position2D { x: 100.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut fv101_params = IndexMap::new();
        fv101_params.insert("cv".to_string(), 85.0);
        fv101_params.insert("position".to_string(), 55.2);

        unit.equipment.push(EquipmentInstance {
            id: fv101_id,
            definition_id: valve_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "FV-101".to_string(),
            parameters: fv101_params,
            position: Some(Position2D { x: 360.0, y: 95.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut v101_params = IndexMap::new();
        v101_params.insert("volume".to_string(), 25.0);
        v101_params.insert("operating_pressure".to_string(), 101325.0);

        unit.equipment.push(EquipmentInstance {
            id: v101_id,
            definition_id: vessel_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "V-101".to_string(),
            parameters: v101_params,
            position: Some(Position2D { x: 520.0, y: 140.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        // 2. Streams with orthogonal layout waypoints
        let mut s01 = StreamDefinition::new(
            "S-01",
            StreamType::Material,
            p101_id,
            discharge_port_id,
            fv101_id,
            valve_inlet_id,
        );
        s01.route_points = vec![
            Position2D { x: 120.0, y: 180.0 },
            Position2D { x: 120.0, y: 120.0 },
            Position2D { x: 360.0, y: 120.0 },
        ];
        unit.streams.push(s01);

        let mut s02 = StreamDefinition::new(
            "S-02",
            StreamType::Material,
            fv101_id,
            valve_outlet_id,
            v101_id,
            vessel_inlet_id,
        );
        s02.route_points = vec![
            Position2D { x: 400.0, y: 120.0 },
            Position2D { x: 580.0, y: 120.0 },
            Position2D { x: 580.0, y: 150.0 },
        ];
        unit.streams.push(s02);

        // 3. Field Instruments
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "TI-101".to_string(),
            equipment_id: Some(p101_id),
            port_id: Some(discharge_port_id),
            measurement_kind: "Temperature".to_string(),
            range_min: 0.0,
            range_max: 250.0,
            unit: "°C".to_string(),
            position: Some(Position2D { x: 190.0, y: 70.0 }),
        });

        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "PT-101".to_string(),
            equipment_id: Some(p101_id),
            port_id: Some(discharge_port_id),
            measurement_kind: "Pressure".to_string(),
            range_min: 0.0,
            range_max: 50.0,
            unit: "bar".to_string(),
            position: Some(Position2D { x: 260.0, y: 70.0 }),
        });

        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "FT-101".to_string(),
            equipment_id: Some(fv101_id),
            port_id: Some(valve_inlet_id),
            measurement_kind: "Flow".to_string(),
            range_min: 0.0,
            range_max: 6000.0,
            unit: "kg/h".to_string(),
            position: Some(Position2D { x: 450.0, y: 70.0 }),
        });

        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "LT-101".to_string(),
            equipment_id: Some(v101_id),
            port_id: None,
            measurement_kind: "Level".to_string(),
            range_min: 0.0,
            range_max: 100.0,
            unit: "%".to_string(),
            position: Some(Position2D { x: 520.0, y: 90.0 }),
        });

        // 4. Control Loop
        unit.control_loops.push(ControlLoopInstance {
            id: EntityId::new(),
            name: "LIC-101".to_string(),
            pv_tag: "V-101.LEVEL".to_string(),
            sp: 50.0,
            out_tag: "FV-101.POS".to_string(),
            kp: 1.5,
            ti: 20.0,
            td: 2.0,
            reverse_acting: false,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 100.0,
            position: Some(Position2D { x: 450.0, y: 20.0 }),
        });

        // 5. Alarms
        unit.alarms.push(AlarmInstance {
            id: EntityId::new(),
            name: "LAH-101".to_string(),
            pv_tag: "V-101.LEVEL".to_string(),
            alarm_type: AlarmType::Hi,
            severity: AlarmPriority::Critical,
            trip_limit: 80.0,
            deadband: 1.0,
            description: "Accumulator level high warning".to_string(),
        });

        unit.alarms.push(AlarmInstance {
            id: EntityId::new(),
            name: "LAL-101".to_string(),
            pv_tag: "V-101.LEVEL".to_string(),
            alarm_type: AlarmType::Lo,
            severity: AlarmPriority::High,
            trip_limit: 20.0,
            deadband: 1.0,
            description: "Accumulator level low warning".to_string(),
        });

        // 6. Interlocks
        unit.interlocks.push(InterlockInstance {
            id: EntityId::new(),
            name: "TRIP-K101".to_string(),
            source_tag: "V-101.LEVEL".to_string(),
            condition: "<".to_string(),
            threshold: 10.0,
            target_action: "TRIP".to_string(),
            target_equipment: Some(p101_id),
            latched: true,
            description: "Low level trip preventing pump cavitation and dry run".to_string(),
        });

        unit.boundary_conditions.push(BoundaryConditionInstance {
            id: EntityId::new(),
            name: "Feed Inflow".to_string(),
            condition_type: "feed".to_string(),
            target_equipment_id: Some(p101_id),
            target_port_id: None,
            target_stream_id: None,
            flow_value: 50.0,
            flow_unit: "m3/h".to_string(),
            pressure_value: 2.0,
            pressure_unit: "barg".to_string(),
            temperature_value: 25.0,
            temperature_unit: "°C".to_string(),
            phase: "Liquid".to_string(),
            density: Some(1000.0),
            molecular_weight: Some(0.018),
            enthalpy: None,
            quality: None,
            composition: IndexMap::new(),
            validity_min_flow: None,
            validity_max_flow: None,
            source_provenance: "Default Unit Feed Header".to_string(),
            enabled: true,
        });

        area.units.push(unit);
        plant.areas.push(area);
        project.plants.push(plant);

        // Stage 4 Engineering Configuration: Virtual Controller
        project.controllers.push(ProjectController {
            id: "C-101".to_string(),
            name: "Primary DCS Controller".to_string(),
            description: "Surge & Separation Unit Master Process Controller".to_string(),
            watchdog_timeout_seconds: 1.0,
            enabled: true,
            tasks: vec![ProjectTask {
                id: "FastLogic".to_string(),
                name: "Fast Control Scan".to_string(),
                task_type: "periodic".to_string(),
                period_seconds: 0.05,
                priority: 5,
                watchdog_seconds: 0.8,
                enabled: true,
                programs: vec![ProjectProgram {
                    id: "FlowControlLogic".to_string(),
                    name: "Level & Flow Control Program".to_string(),
                    description: "IEC 61131-3 logic for automated surge control".to_string(),
                    program_type: "IEC".to_string(),
                    routines: vec![ProjectRoutine {
                        id: "ST_CONTROL".to_string(),
                        name: "ST_CONTROL".to_string(),
                        language: "ST".to_string(),
                        source_text: "VAR\n    PV_LEVEL : REAL := 50.0;\n    SP_LEVEL : REAL := 50.0;\n    VALVE_DEMAND : REAL := 50.0;\nEND_VAR\n\nIF PV_LEVEL < SP_LEVEL THEN\n    VALVE_DEMAND := VALVE_DEMAND + 0.5;\nELSIF PV_LEVEL > SP_LEVEL THEN\n    VALVE_DEMAND := VALVE_DEMAND - 0.5;\nEND_IF;\n".to_string(),
                        enabled: true,
                    }],
                }],
            }],
            cpu_profile: Some(ProjectCpuProfile {
                execution_budget_us: 50_000,
                max_scan_frequency_hz: 50.0,
                utilization_threshold_pct: 80.0,
                architecture: "VendorNeutral-VirtualVCPU-64".to_string(),
            }),
            memory_profile: Some(ProjectMemoryProfile {
                program_memory_bytes: 16 * 1024 * 1024,
                data_memory_bytes: 32 * 1024 * 1024,
                retentive_memory_bytes: 4 * 1024 * 1024,
            }),
            firmware_profile: Some(ProjectFirmwareProfile {
                version: "v8.1.0".to_string(),
                build: "2026.09".to_string(),
                iec_standard: "IEC 61131-3:2013 Ed.3".to_string(),
            }),
            redundancy: Some(ProjectRedundancyConfig {
                role: "Primary".to_string(),
                partner_controller_id: Some("C-101-STANDBY".to_string()),
                sync_frequency_hz: 20.0,
                switchover_timeout_seconds: 0.1,
            }),
            control_modules: vec![ProjectControlModule {
                id: "LIC-101".to_string(),
                name: "Surge Drum Level Control Module".to_string(),
                description: "DCS regulatory loop regulating Vessel V-101 liquid level".to_string(),
                mode: "AUTO".to_string(),
                execution_rate_seconds: 0.05,
                pv_signal: "V-101.LEVEL".to_string(),
                sp: 50.0,
                out_signal: "FV-101.DEMAND".to_string(),
                strategy_type: "PID".to_string(),
                kp: 2.0,
                ti: 10.0,
                td: 0.0,
                output_min: 0.0,
                output_max: 100.0,
                permissives: Vec::new(),
                interlocks: vec!["ESD.ACTIVE".to_string()],
                safe_position: 0.0,
            }],
        });

        // Stage 4 Engineering Configuration: HAL I/O Hardware Hierarchy
        project.io_systems.push(ProjectIoSystem {
            id: "HAL-MAIN".to_string(),
            name: "Main Process I/O Subsystem".to_string(),
            racks: vec![ProjectIoRack {
                rack_number: 1,
                name: "RACK-01".to_string(),
                power_ok: true,
                chassis: vec![ProjectIoChassis {
                    chassis_number: 1,
                    name: "CHASSIS-01".to_string(),
                    bus_ok: true,
                    modules: vec![
                        ProjectIoModule {
                            slot: 1,
                            name: "AI-01".to_string(),
                            module_type: "AnalogInput8".to_string(),
                            channels: (1..=8)
                                .map(|ch| ProjectIoChannel {
                                    channel_number: ch,
                                    address: format!("R01-C01-M01-AI{:02}", ch),
                                    name: format!("AI-{:02}", ch),
                                    kind: "AI".to_string(),
                                    engineering_unit: "%".to_string(),
                                    lrv: 0.0,
                                    urv: 100.0,
                                    fallback_behavior: None,
                                    forced_value: None,
                                })
                                .collect(),
                        },
                        ProjectIoModule {
                            slot: 2,
                            name: "AO-01".to_string(),
                            module_type: "AnalogOutput8".to_string(),
                            channels: (1..=8)
                                .map(|ch| ProjectIoChannel {
                                    channel_number: ch,
                                    address: format!("R01-C01-M02-AO{:02}", ch),
                                    name: format!("AO-{:02}", ch),
                                    kind: "AO".to_string(),
                                    engineering_unit: "%".to_string(),
                                    lrv: 0.0,
                                    urv: 100.0,
                                    fallback_behavior: Some("FailClosed".to_string()),
                                    forced_value: None,
                                })
                                .collect(),
                        },
                        ProjectIoModule {
                            slot: 3,
                            name: "DI-01".to_string(),
                            module_type: "DigitalInput16".to_string(),
                            channels: (1..=16)
                                .map(|ch| ProjectIoChannel {
                                    channel_number: ch,
                                    address: format!("R01-C01-M03-DI{:02}", ch),
                                    name: format!("DI-{:02}", ch),
                                    kind: "DI".to_string(),
                                    engineering_unit: "".to_string(),
                                    lrv: 0.0,
                                    urv: 1.0,
                                    fallback_behavior: None,
                                    forced_value: None,
                                })
                                .collect(),
                        },
                        ProjectIoModule {
                            slot: 4,
                            name: "DO-01".to_string(),
                            module_type: "DigitalOutput16".to_string(),
                            channels: (1..=16)
                                .map(|ch| ProjectIoChannel {
                                    channel_number: ch,
                                    address: format!("R01-C01-M04-DO{:02}", ch),
                                    name: format!("DO-{:02}", ch),
                                    kind: "DO".to_string(),
                                    engineering_unit: "".to_string(),
                                    lrv: 0.0,
                                    urv: 1.0,
                                    fallback_behavior: Some("FailClosed".to_string()),
                                    forced_value: None,
                                })
                                .collect(),
                        },
                    ],
                }],
            }],
        });

        // Stage 4 Engineering Configuration: Controller-to-I/O Mappings
        project.io_mappings.push(ProjectIoMapping {
            controller_id: "C-101".to_string(),
            signal_key: "PV_LEVEL".to_string(),
            channel_address: "R01-C01-M01-AI01".to_string(),
            direction: "input".to_string(),
            target_tag: Some("V-101.LEVEL".to_string()),
            target_actuator: None,
            priority: Some(5),
        });
        project.io_mappings.push(ProjectIoMapping {
            controller_id: "C-101".to_string(),
            signal_key: "VALVE_DEMAND".to_string(),
            channel_address: "R01-C01-M02-AO01".to_string(),
            direction: "output".to_string(),
            target_tag: Some("FV-101.POS".to_string()),
            target_actuator: Some(fv101_id.to_string()),
            priority: Some(3),
        });

        // Stage 4 Engineering Configuration: Canonical Tag Database
        project.tags.push(ProjectTag {
            id: "tag-v101-lvl".to_string(),
            name: "V-101.LEVEL".to_string(),
            description: "Accumulator Separator V-101 Liquid Level".to_string(),
            data_type: "REAL".to_string(),
            engineering_unit: "%".to_string(),
            source: "field".to_string(),
            direction: "input".to_string(),
            controller_id: Some("C-101".to_string()),
            channel_address: Some("R01-C01-M01-AI01".to_string()),
            process_entity: Some("V-101".to_string()),
            alarm_id: Some("LAH-101".to_string()),
        });
        project.tags.push(ProjectTag {
            id: "tag-fv101-pos".to_string(),
            name: "FV-101.POS".to_string(),
            description: "Control Valve FV-101 Stem Position Demand".to_string(),
            data_type: "REAL".to_string(),
            engineering_unit: "%".to_string(),
            source: "controller".to_string(),
            direction: "output".to_string(),
            controller_id: Some("C-101".to_string()),
            channel_address: Some("R01-C01-M02-AO01".to_string()),
            process_entity: Some("FV-101".to_string()),
            alarm_id: None,
        });
        project.tags.push(ProjectTag {
            id: "tag-p101-spd".to_string(),
            name: "K-101.SPEED".to_string(),
            description: "Centrifugal Pump K-101 Rotational Speed".to_string(),
            data_type: "REAL".to_string(),
            engineering_unit: "rpm".to_string(),
            source: "field".to_string(),
            direction: "input".to_string(),
            controller_id: None,
            channel_address: None,
            process_entity: Some("K-101".to_string()),
            alarm_id: None,
        });
        project.tags.push(ProjectTag {
            id: "tag-ft101-flw".to_string(),
            name: "FT-101.FLOW".to_string(),
            description: "Feed Discharge Mass Flow Rate".to_string(),
            data_type: "REAL".to_string(),
            engineering_unit: "kg/h".to_string(),
            source: "field".to_string(),
            direction: "input".to_string(),
            controller_id: None,
            channel_address: None,
            process_entity: Some("K-101".to_string()),
            alarm_id: None,
        });

        // Seed default Stage 5 Cascade Strategy
        project
            .advanced_control
            .push(ProjectAdvancedControlStrategy {
                id: "strat-cas-101".to_string(),
                name: "Tank Level - Feed Flow Cascade".to_string(),
                strategy_type: "Cascade".to_string(),
                description: "Primary level master driving secondary flow slave setpoint"
                    .to_string(),
                enabled: true,
                parameters: [
                    ("primary_pv".to_string(), "LT-101.LEVEL".to_string()),
                    ("secondary_pv".to_string(), "FT-101.FLOW".to_string()),
                    ("primary_sp".to_string(), "50.0".to_string()),
                    ("primary_kp".to_string(), "1.5".to_string()),
                    ("primary_ti".to_string(), "10.0".to_string()),
                    ("secondary_kp".to_string(), "1.0".to_string()),
                    ("secondary_ti".to_string(), "2.0".to_string()),
                    ("secondary_sp_min".to_string(), "0.0".to_string()),
                    ("secondary_sp_max".to_string(), "100.0".to_string()),
                    ("output_min".to_string(), "0.0".to_string()),
                    ("output_max".to_string(), "100.0".to_string()),
                    ("target_actuator".to_string(), "FV-101".to_string()),
                    ("output_tag".to_string(), "FIC_101.OUT".to_string()),
                ]
                .into_iter()
                .collect(),
            });

        // Stage 9 Safety Instrumented System (SIS) Configuration
        project.sis = Some(ProjectSisConfig {
            enabled: true,
            sifs: vec![ProjectSifDefinition {
                id: "SIF-101".to_string(),
                name: "V-101 High-High Level ESD Trip".to_string(),
                description: "Trips feed valve FV-101 to prevent separator vessel liquid overflow into gas outlet"
                    .to_string(),
                hazard_id: Some("HAZ-OVF-101".to_string()),
                hazard_scenario: Some(
                    "Liquid carryover into compressor suction causing severe mechanical damage".to_string(),
                ),
                target_sil: Some("SIL-2".to_string()),
                direction: "High".to_string(),
                threshold: 90.0,
                reset_deadband: 2.0,
                trip_delay_seconds: 0.1,
                architecture: "2oo3".to_string(),
                degradation_policy: "DegradeTo1oo2".to_string(),
                sensor_tags: vec![
                    "V-101.LEVEL".to_string(),
                    "LT-101.LEVEL".to_string(),
                    "LT-101.PV".to_string(),
                ],
                final_elements: vec![ProjectSifFinalElement {
                    target_equipment: "FV-101".to_string(),
                    actuator_var: "opening".to_string(),
                    safe_state: "ValveClosed".to_string(),
                    de_energize_to_trip: true,
                    action_type: "EmergencyShutdown".to_string(),
                    feedback_tag: Some("FV-101.POS".to_string()),
                }],
                latched: true,
                reset_permissives: vec![],
                pfd_avg_estimate: Some(0.0035),
            }],
        });

        let key = DEFAULT_PROJECT_ID.to_string();
        self.projects.insert(key, project);
    }

    /// Seeds the four authoritative industrial domain packages:
    /// - Refinery (Crude Distillation Unit)
    /// - Power & Steam (Steam Turbine Mechanical Drive)
    /// - Gas Processing (Natural Gas Compression & Anti-Surge)
    /// - Chemical Processing (Exothermic CSTR Reaction Train)
    /// - Naphtha Hydrotreater (NHT Reference Plant - HPCL Visakh / EIL Reference Benchmark)
    pub fn seed_domain_projects(&mut self) {
        self.seed_refinery_cdu_project();
        self.seed_power_steam_project();
        self.seed_gas_compression_project();
        self.seed_chemical_project();
        self.seed_nht_reference_project();
    }

    fn seed_refinery_cdu_project(&mut self) {
        let project_id = PROJ_REFINERY_CDU
            .parse::<ProjectId>()
            .unwrap_or_else(|_| ProjectId::new());
        let mut project = Project::new("Refinery Atmospheric Crude Distillation Unit (CDU)");
        project.id = project_id;
        project.metadata = ProjectMetadata {
            description: "Industrial refinery crude distillation flowsheet featuring crude preheat train, fired process furnace, fractional distillation column, overhead condenser, reflux accumulator, and reflux pump.".to_string(),
            author: "NebuDCS Standard Library".to_string(),
            created: "2026-09-07T00:00:00Z".to_string(),
            modified: "2026-09-07T00:00:00Z".to_string(),
        };

        let mut plant = Plant::new("Refinery Area 10");
        let mut area = Area::new("Crude Distillation Section");
        let mut unit = Unit::new("Unit 110 Crude Fractionation");

        let tk101_id = EquipmentId::new();
        let p101_id = EquipmentId::new();
        let fv101_id = EquipmentId::new();
        let e101_id = EquipmentId::new();
        let h101_id = EquipmentId::new();
        let t101_id = EquipmentId::new();
        let e102_id = EquipmentId::new();
        let v102_id = EquipmentId::new();
        let p102_id = EquipmentId::new();

        let tank_def_id = "standard.tank.atmospheric"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let pump_def_id = "standard.pump.centrifugal"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let valve_def_id = "standard.valve.control_globe"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let hx_def_id = "standard.heat_exchanger.shell_and_tube"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let heater_def_id = "standard.heater.fired_process"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let vessel_def_id = "standard.vessel.vertical_separator"
            .parse::<EquipmentDefinitionId>()
            .unwrap();

        let outlet_port = "outlet".parse::<PortId>().unwrap();
        let inlet_port = "inlet".parse::<PortId>().unwrap();
        let suction_port = "suction".parse::<PortId>().unwrap();
        let discharge_port = "discharge".parse::<PortId>().unwrap();
        let tube_in_port = "tube_in".parse::<PortId>().unwrap();
        let tube_out_port = "tube_out".parse::<PortId>().unwrap();
        let process_in_port = "process_in".parse::<PortId>().unwrap();
        let process_out_port = "process_out".parse::<PortId>().unwrap();
        let feed_port = "feed".parse::<PortId>().unwrap();
        let gas_out_port = "gas_out".parse::<PortId>().unwrap();
        let liquid_out_port = "liquid_out".parse::<PortId>().unwrap();

        // 1. Equipment instances
        let mut tk101_params = IndexMap::new();
        tk101_params.insert("capacity_m3".to_string(), 5000.0);
        tk101_params.insert("level".to_string(), 75.0);
        unit.equipment.push(EquipmentInstance {
            id: tk101_id,
            definition_id: tank_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "TK-101".to_string(),
            parameters: tk101_params,
            position: Some(Position2D { x: 100.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut p101_params = IndexMap::new();
        p101_params.insert("rated_flow".to_string(), 120.0);
        p101_params.insert("rated_head".to_string(), 120.0);
        p101_params.insert("speed_rpm".to_string(), 2950.0);
        unit.equipment.push(EquipmentInstance {
            id: p101_id,
            definition_id: pump_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "P-101".to_string(),
            parameters: p101_params,
            position: Some(Position2D { x: 260.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut fv101_params = IndexMap::new();
        fv101_params.insert("rated_cv".to_string(), 120.0);
        fv101_params.insert("cv".to_string(), 120.0);
        fv101_params.insert("position".to_string(), 65.0);
        unit.equipment.push(EquipmentInstance {
            id: fv101_id,
            definition_id: valve_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "FV-101".to_string(),
            parameters: fv101_params,
            position: Some(Position2D { x: 420.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut e101_params = IndexMap::new();
        e101_params.insert("area_m2".to_string(), 250.0);
        e101_params.insert("overall_u".to_string(), 650.0);
        e101_params.insert("design_duty".to_string(), 2500000.0);
        e101_params.insert("process_inlet_t".to_string(), 35.0);
        unit.equipment.push(EquipmentInstance {
            id: e101_id,
            definition_id: hx_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "E-101".to_string(),
            parameters: e101_params,
            position: Some(Position2D { x: 580.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut h101_params = IndexMap::new();
        h101_params.insert("duty".to_string(), 15000000.0);
        h101_params.insert("design_duty".to_string(), 15000000.0);
        h101_params.insert("efficiency".to_string(), 0.88);
        h101_params.insert("process_inlet_t".to_string(), 180.0);
        unit.equipment.push(EquipmentInstance {
            id: h101_id,
            definition_id: heater_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "H-101".to_string(),
            parameters: h101_params,
            position: Some(Position2D { x: 760.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut t101_params = IndexMap::new();
        t101_params.insert("volume_m3".to_string(), 150.0);
        t101_params.insert("volume".to_string(), 150.0);
        t101_params.insert("operating_pressure".to_string(), 150000.0);
        unit.equipment.push(EquipmentInstance {
            id: t101_id,
            definition_id: vessel_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "T-101".to_string(),
            parameters: t101_params,
            position: Some(Position2D { x: 940.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut e102_params = IndexMap::new();
        e102_params.insert("area_m2".to_string(), 180.0);
        e102_params.insert("overall_u".to_string(), 550.0);
        e102_params.insert("design_duty".to_string(), 1800000.0);
        e102_params.insert("process_inlet_t".to_string(), 110.0);
        unit.equipment.push(EquipmentInstance {
            id: e102_id,
            definition_id: hx_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "E-102".to_string(),
            parameters: e102_params,
            position: Some(Position2D {
                x: 1120.0,
                y: 100.0,
            }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut v102_params = IndexMap::new();
        v102_params.insert("volume_m3".to_string(), 30.0);
        v102_params.insert("volume".to_string(), 30.0);
        v102_params.insert("operating_pressure".to_string(), 130000.0);
        unit.equipment.push(EquipmentInstance {
            id: v102_id,
            definition_id: vessel_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "V-102".to_string(),
            parameters: v102_params,
            position: Some(Position2D {
                x: 1280.0,
                y: 140.0,
            }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut p102_params = IndexMap::new();
        p102_params.insert("rated_flow".to_string(), 60.0);
        p102_params.insert("rated_head".to_string(), 80.0);
        p102_params.insert("speed_rpm".to_string(), 2950.0);
        unit.equipment.push(EquipmentInstance {
            id: p102_id,
            definition_id: pump_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "P-102".to_string(),
            parameters: p102_params,
            position: Some(Position2D {
                x: 1440.0,
                y: 180.0,
            }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        // 2. Streams
        unit.streams.push(StreamDefinition::new(
            "S-CRUDE-01",
            StreamType::Material,
            tk101_id,
            outlet_port,
            p101_id,
            suction_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-CRUDE-02",
            StreamType::Material,
            p101_id,
            discharge_port,
            fv101_id,
            inlet_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-CRUDE-03",
            StreamType::Material,
            fv101_id,
            outlet_port,
            e101_id,
            tube_in_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-CRUDE-04",
            StreamType::Material,
            e101_id,
            tube_out_port,
            h101_id,
            process_in_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-CRUDE-05",
            StreamType::Material,
            h101_id,
            process_out_port,
            t101_id,
            feed_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-OVHD-01",
            StreamType::Material,
            t101_id,
            gas_out_port,
            e102_id,
            tube_in_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-OVHD-02",
            StreamType::Material,
            e102_id,
            tube_out_port,
            v102_id,
            feed_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-REFLUX-01",
            StreamType::Material,
            v102_id,
            liquid_out_port,
            p102_id,
            suction_port,
        ));

        // 3. Instruments
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "FT-101".to_string(),
            equipment_id: Some(fv101_id),
            port_id: Some(inlet_port),
            measurement_kind: "Flow".to_string(),
            range_min: 0.0,
            range_max: 15000.0,
            unit: "kg/h".to_string(),
            position: Some(Position2D { x: 420.0, y: 100.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "TI-101".to_string(),
            equipment_id: Some(e101_id),
            port_id: Some(tube_out_port),
            measurement_kind: "Temperature".to_string(),
            range_min: 0.0,
            range_max: 300.0,
            unit: "°C".to_string(),
            position: Some(Position2D { x: 640.0, y: 100.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "TI-102".to_string(),
            equipment_id: Some(h101_id),
            port_id: Some(process_out_port),
            measurement_kind: "Temperature".to_string(),
            range_min: 0.0,
            range_max: 450.0,
            unit: "°C".to_string(),
            position: Some(Position2D { x: 820.0, y: 100.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "PT-101".to_string(),
            equipment_id: Some(t101_id),
            port_id: Some(gas_out_port),
            measurement_kind: "Pressure".to_string(),
            range_min: 0.0,
            range_max: 5.0,
            unit: "bar".to_string(),
            position: Some(Position2D { x: 940.0, y: 80.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "LT-101".to_string(),
            equipment_id: Some(t101_id),
            port_id: None,
            measurement_kind: "Level".to_string(),
            range_min: 0.0,
            range_max: 100.0,
            unit: "%".to_string(),
            position: Some(Position2D { x: 940.0, y: 260.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "LT-102".to_string(),
            equipment_id: Some(v102_id),
            port_id: None,
            measurement_kind: "Level".to_string(),
            range_min: 0.0,
            range_max: 100.0,
            unit: "%".to_string(),
            position: Some(Position2D {
                x: 1280.0,
                y: 220.0,
            }),
        });

        // 4. Control Loops
        unit.control_loops.push(ControlLoopInstance {
            id: EntityId::new(),
            name: "FIC-101".to_string(),
            pv_tag: "FT-101".to_string(),
            sp: 12000.0,
            out_tag: "FV-101.OPENING".to_string(),
            kp: 1.2,
            ti: 10.0,
            td: 0.0,
            reverse_acting: false,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 100.0,
            position: Some(Position2D { x: 420.0, y: 40.0 }),
        });
        unit.control_loops.push(ControlLoopInstance {
            id: EntityId::new(),
            name: "TIC-102".to_string(),
            pv_tag: "TI-102".to_string(),
            sp: 360.0,
            out_tag: "H-101.DUTY".to_string(),
            kp: 2.0,
            ti: 30.0,
            td: 2.0,
            reverse_acting: false,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 100.0,
            position: Some(Position2D { x: 820.0, y: 40.0 }),
        });
        unit.control_loops.push(ControlLoopInstance {
            id: EntityId::new(),
            name: "LIC-101".to_string(),
            pv_tag: "LT-101".to_string(),
            sp: 50.0,
            out_tag: "T-101.OPENING".to_string(),
            kp: 1.5,
            ti: 25.0,
            td: 0.0,
            reverse_acting: false,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 100.0,
            position: Some(Position2D { x: 940.0, y: 320.0 }),
        });

        // 5. Alarms
        unit.alarms.push(AlarmInstance {
            id: EntityId::new(),
            name: "TAH-102".to_string(),
            pv_tag: "TI-102".to_string(),
            alarm_type: AlarmType::Hi,
            severity: AlarmPriority::High,
            trip_limit: 380.0,
            deadband: 2.0,
            description: "Furnace coil outlet temperature high".to_string(),
        });
        unit.alarms.push(AlarmInstance {
            id: EntityId::new(),
            name: "LAH-101".to_string(),
            pv_tag: "LT-101".to_string(),
            alarm_type: AlarmType::Hi,
            severity: AlarmPriority::Critical,
            trip_limit: 85.0,
            deadband: 1.0,
            description: "Column bottom sump level high".to_string(),
        });
        unit.alarms.push(AlarmInstance {
            id: EntityId::new(),
            name: "LAL-101".to_string(),
            pv_tag: "LT-101".to_string(),
            alarm_type: AlarmType::Lo,
            severity: AlarmPriority::High,
            trip_limit: 20.0,
            deadband: 1.0,
            description: "Column bottom sump level low".to_string(),
        });

        // 6. Interlocks
        unit.interlocks.push(InterlockInstance {
            id: EntityId::new(),
            name: "TRIP-H101".to_string(),
            source_tag: "FT-101".to_string(),
            condition: "<".to_string(),
            threshold: 2000.0,
            target_action: "TRIP".to_string(),
            target_equipment: Some(h101_id),
            latched: true,
            description: "Furnace coil low flow trip".to_string(),
        });
        unit.interlocks.push(InterlockInstance {
            id: EntityId::new(),
            name: "TRIP-P101".to_string(),
            source_tag: "LT-101".to_string(),
            condition: "<".to_string(),
            threshold: 10.0,
            target_action: "TRIP".to_string(),
            target_equipment: Some(p101_id),
            latched: true,
            description: "Charge pump suction dry-run protection".to_string(),
        });

        // Seed Stage 5 Feedforward Strategy
        project.advanced_control.push(ProjectAdvancedControlStrategy {
            id: "strat-ff-101".to_string(),
            name: "Furnace COT Feedforward Disturbance Rejection".to_string(),
            strategy_type: "FeedForward".to_string(),
            description: "Compensates crude feed rate disturbances dynamically before coil outlet temperature drifts".to_string(),
            enabled: true,
            parameters: [
                ("pv_tag".to_string(), "TT-101.TEMP".to_string()),
                ("sp".to_string(), "365.0".to_string()),
                ("disturbance_tag".to_string(), "FT-101.FLOW".to_string()),
                ("t_lead".to_string(), "4.0".to_string()),
                ("t_lag".to_string(), "2.0".to_string()),
                ("gain".to_string(), "0.8".to_string()),
                ("kp".to_string(), "1.5".to_string()),
                ("ti".to_string(), "8.0".to_string()),
                ("td".to_string(), "0.5".to_string()),
                ("output_min".to_string(), "0.0".to_string()),
                ("output_max".to_string(), "100.0".to_string()),
                ("target_actuator".to_string(), "FV-102".to_string()),
                ("output_tag".to_string(), "TIC_101.OUT".to_string()),
            ]
            .into_iter()
            .collect(),
        });

        unit.boundary_conditions.push(BoundaryConditionInstance {
            id: EntityId::new(),
            name: "Crude Oil Feed".to_string(),
            condition_type: "feed".to_string(),
            target_equipment_id: None,
            target_port_id: None,
            target_stream_id: None,
            flow_value: 300.0,
            flow_unit: "m3/h".to_string(),
            pressure_value: 4.5,
            pressure_unit: "barg".to_string(),
            temperature_value: 35.0,
            temperature_unit: "°C".to_string(),
            phase: "Liquid".to_string(),
            density: Some(850.0),
            molecular_weight: Some(0.250),
            enthalpy: None,
            quality: None,
            composition: IndexMap::new(),
            validity_min_flow: None,
            validity_max_flow: None,
            source_provenance: "Crude Storage Battery Limit".to_string(),
            enabled: true,
        });

        area.units.push(unit);
        plant.areas.push(area);
        project.plants.push(plant);
        self.projects.insert(PROJ_REFINERY_CDU.to_string(), project);
    }

    fn seed_power_steam_project(&mut self) {
        let project_id = PROJ_POWER_TURBINE
            .parse::<ProjectId>()
            .unwrap_or_else(|_| ProjectId::new());
        let mut project = Project::new("Thermal Power & Steam Mechanical Drive Plant");
        project.id = project_id;
        project.metadata = ProjectMetadata {
            description: "Industrial steam expansion turbine driver coupled via mechanical shaft to centrifugal process compressor with surface condenser.".to_string(),
            author: "NebuDCS Standard Library".to_string(),
            created: "2026-09-07T00:00:00Z".to_string(),
            modified: "2026-09-07T00:00:00Z".to_string(),
        };

        let mut plant = Plant::new("Power Area 20");
        let mut area = Area::new("Turbine Island");
        let mut unit = Unit::new("Unit 200 Steam Drive Train");

        let tk201_id = EquipmentId::new();
        let tv201_id = EquipmentId::new();
        let st201_id = EquipmentId::new();
        let c201_id = EquipmentId::new();
        let v201_id = EquipmentId::new();
        let v202_id = EquipmentId::new();

        let tank_def_id = "standard.tank.atmospheric"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let valve_def_id = "standard.valve.control_globe"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let turb_def_id = "custom.turbine.steam"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let comp_def_id = "standard.compressor.centrifugal"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let vessel_def_id = "standard.vessel.vertical_separator"
            .parse::<EquipmentDefinitionId>()
            .unwrap();

        let outlet_port = "outlet".parse::<PortId>().unwrap();
        let inlet_port = "inlet".parse::<PortId>().unwrap();
        let steam_inlet_port = "steam_inlet".parse::<PortId>().unwrap();
        let exhaust_port = "exhaust".parse::<PortId>().unwrap();
        let shaft_out_port = "shaft_out".parse::<PortId>().unwrap();
        let shaft_drive_port = "shaft_drive".parse::<PortId>().unwrap();
        let discharge_port = "discharge".parse::<PortId>().unwrap();
        let feed_port = "feed".parse::<PortId>().unwrap();

        let mut tk201_params = IndexMap::new();
        tk201_params.insert("capacity_m3".to_string(), 2000.0);
        tk201_params.insert("level".to_string(), 80.0);
        unit.equipment.push(EquipmentInstance {
            id: tk201_id,
            definition_id: tank_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "TK-201".to_string(),
            parameters: tk201_params,
            position: Some(Position2D { x: 100.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut tv201_params = IndexMap::new();
        tv201_params.insert("rated_cv".to_string(), 150.0);
        tv201_params.insert("cv".to_string(), 150.0);
        tv201_params.insert("position".to_string(), 75.0);
        unit.equipment.push(EquipmentInstance {
            id: tv201_id,
            definition_id: valve_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "TV-201".to_string(),
            parameters: tv201_params,
            position: Some(Position2D { x: 280.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut st201_params = IndexMap::new();
        st201_params.insert("rated_power".to_string(), 500000.0);
        st201_params.insert("speed".to_string(), 3000.0);
        st201_params.insert("rated_head".to_string(), 150.0);
        unit.equipment.push(EquipmentInstance {
            id: st201_id,
            definition_id: turb_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "ST-201".to_string(),
            parameters: st201_params,
            position: Some(Position2D { x: 480.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut c201_params = IndexMap::new();
        c201_params.insert("rated_head".to_string(), 120.0);
        c201_params.insert("rated_power".to_string(), 450000.0);
        c201_params.insert("speed".to_string(), 3000.0);
        unit.equipment.push(EquipmentInstance {
            id: c201_id,
            definition_id: comp_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "C-201".to_string(),
            parameters: c201_params,
            position: Some(Position2D { x: 720.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut v201_params = IndexMap::new();
        v201_params.insert("volume_m3".to_string(), 40.0);
        v201_params.insert("volume".to_string(), 40.0);
        v201_params.insert("operating_pressure".to_string(), 101325.0);
        unit.equipment.push(EquipmentInstance {
            id: v201_id,
            definition_id: vessel_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "V-201".to_string(),
            parameters: v201_params,
            position: Some(Position2D { x: 480.0, y: 360.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut v202_params = IndexMap::new();
        v202_params.insert("volume_m3".to_string(), 50.0);
        v202_params.insert("volume".to_string(), 50.0);
        v202_params.insert("operating_pressure".to_string(), 2500000.0);
        unit.equipment.push(EquipmentInstance {
            id: v202_id,
            definition_id: vessel_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "V-202".to_string(),
            parameters: v202_params,
            position: Some(Position2D { x: 940.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        // 2. Streams
        unit.streams.push(StreamDefinition::new(
            "S-STM-01",
            StreamType::Material,
            tk201_id,
            outlet_port,
            tv201_id,
            inlet_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-STM-02",
            StreamType::Material,
            tv201_id,
            outlet_port,
            st201_id,
            steam_inlet_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-STM-EXH",
            StreamType::Material,
            st201_id,
            exhaust_port,
            v201_id,
            feed_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-SHAFT-01",
            StreamType::MechanicalShaft,
            st201_id,
            shaft_out_port,
            c201_id,
            shaft_drive_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-GAS-DISCH",
            StreamType::Material,
            c201_id,
            discharge_port,
            v202_id,
            feed_port,
        ));

        // 3. Instruments
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "PT-201".to_string(),
            equipment_id: Some(tv201_id),
            port_id: Some(inlet_port),
            measurement_kind: "Pressure".to_string(),
            range_min: 0.0,
            range_max: 100.0,
            unit: "bar".to_string(),
            position: Some(Position2D { x: 280.0, y: 100.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "ST-201_SPEED".to_string(),
            equipment_id: Some(st201_id),
            port_id: None,
            measurement_kind: "Speed".to_string(),
            range_min: 0.0,
            range_max: 4500.0,
            unit: "RPM".to_string(),
            position: Some(Position2D { x: 480.0, y: 100.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "WT-201".to_string(),
            equipment_id: Some(st201_id),
            port_id: None,
            measurement_kind: "Power".to_string(),
            range_min: 0.0,
            range_max: 600.0,
            unit: "kW".to_string(),
            position: Some(Position2D { x: 480.0, y: 40.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "VT-201".to_string(),
            equipment_id: Some(c201_id),
            port_id: None,
            measurement_kind: "Vibration".to_string(),
            range_min: 0.0,
            range_max: 15.0,
            unit: "mm/s".to_string(),
            position: Some(Position2D { x: 720.0, y: 100.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "PT-202".to_string(),
            equipment_id: Some(v202_id),
            port_id: Some(feed_port),
            measurement_kind: "Pressure".to_string(),
            range_min: 0.0,
            range_max: 40.0,
            unit: "bar".to_string(),
            position: Some(Position2D { x: 940.0, y: 100.0 }),
        });

        // 4. Control Loops
        unit.control_loops.push(ControlLoopInstance {
            id: EntityId::new(),
            name: "SIC-201".to_string(),
            pv_tag: "ST-201_SPEED".to_string(),
            sp: 3000.0,
            out_tag: "TV-201.OPENING".to_string(),
            kp: 1.8,
            ti: 15.0,
            td: 0.5,
            reverse_acting: false,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 100.0,
            position: Some(Position2D { x: 380.0, y: 40.0 }),
        });
        unit.control_loops.push(ControlLoopInstance {
            id: EntityId::new(),
            name: "PIC-201".to_string(),
            pv_tag: "PT-202".to_string(),
            sp: 25.0,
            out_tag: "TV-201.OPENING".to_string(),
            kp: 1.0,
            ti: 20.0,
            td: 0.0,
            reverse_acting: false,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 100.0,
            position: Some(Position2D { x: 840.0, y: 40.0 }),
        });

        // 5. Alarms
        unit.alarms.push(AlarmInstance {
            id: EntityId::new(),
            name: "SAH-201".to_string(),
            pv_tag: "ST-201_SPEED".to_string(),
            alarm_type: AlarmType::Hi,
            severity: AlarmPriority::High,
            trip_limit: 3300.0,
            deadband: 20.0,
            description: "Turbine high speed warning".to_string(),
        });
        unit.alarms.push(AlarmInstance {
            id: EntityId::new(),
            name: "SAHH-201".to_string(),
            pv_tag: "ST-201_SPEED".to_string(),
            alarm_type: AlarmType::Hi,
            severity: AlarmPriority::Critical,
            trip_limit: 3600.0,
            deadband: 20.0,
            description: "Turbine emergency overspeed hazard".to_string(),
        });
        unit.alarms.push(AlarmInstance {
            id: EntityId::new(),
            name: "VAH-201".to_string(),
            pv_tag: "VT-201".to_string(),
            alarm_type: AlarmType::Hi,
            severity: AlarmPriority::High,
            trip_limit: 4.5,
            deadband: 0.2,
            description: "Compressor vibration high".to_string(),
        });

        // 6. Interlocks
        unit.interlocks.push(InterlockInstance {
            id: EntityId::new(),
            name: "TRIP-OVERSPEED".to_string(),
            source_tag: "ST-201_SPEED".to_string(),
            condition: ">".to_string(),
            threshold: 3600.0,
            target_action: "TRIP".to_string(),
            target_equipment: Some(st201_id),
            latched: true,
            description: "Turbine overspeed quick shutoff".to_string(),
        });
        unit.interlocks.push(InterlockInstance {
            id: EntityId::new(),
            name: "TRIP-VIB".to_string(),
            source_tag: "VT-201".to_string(),
            condition: ">".to_string(),
            threshold: 8.0,
            target_action: "TRIP".to_string(),
            target_equipment: Some(c201_id),
            latched: true,
            description: "High vibration compressor protection".to_string(),
        });

        // Seed Stage 5 Override Strategy
        project.advanced_control.push(ProjectAdvancedControlStrategy {
            id: "strat-ovr-201".to_string(),
            name: "Turbine Governor Speed/Exhaust Override".to_string(),
            strategy_type: "Override".to_string(),
            description: "Governs turbine speed while overriding valve position if exhaust backpressure reaches safety constraint".to_string(),
            enabled: true,
            parameters: [
                ("primary_pv".to_string(), "ST-201_SPEED".to_string()),
                ("primary_sp".to_string(), "3000.0".to_string()),
                ("primary_kp".to_string(), "0.05".to_string()),
                ("primary_ti".to_string(), "2.0".to_string()),
                ("constraint_pv".to_string(), "PT-202".to_string()),
                ("constraint_limit".to_string(), "5.0".to_string()),
                ("constraint_kp".to_string(), "1.5".to_string()),
                ("constraint_ti".to_string(), "1.0".to_string()),
                ("constraint_direction".to_string(), "reverse".to_string()),
                ("selector_type".to_string(), "low".to_string()),
                ("output_min".to_string(), "0.0".to_string()),
                ("output_max".to_string(), "100.0".to_string()),
                ("target_actuator".to_string(), "TV-201".to_string()),
                ("output_tag".to_string(), "TCV_201.OUT".to_string()),
            ]
            .into_iter()
            .collect(),
        });

        unit.boundary_conditions.push(BoundaryConditionInstance {
            id: EntityId::new(),
            name: "High Pressure Steam Header".to_string(),
            condition_type: "utility".to_string(),
            target_equipment_id: None,
            target_port_id: None,
            target_stream_id: None,
            flow_value: 45.0,
            flow_unit: "t/d".to_string(),
            pressure_value: 42.0,
            pressure_unit: "barg".to_string(),
            temperature_value: 410.0,
            temperature_unit: "°C".to_string(),
            phase: "Vapor".to_string(),
            density: Some(21.0),
            molecular_weight: Some(0.018015),
            enthalpy: None,
            quality: None,
            composition: IndexMap::new(),
            validity_min_flow: None,
            validity_max_flow: None,
            source_provenance: "HP Steam Boiler Plant".to_string(),
            enabled: true,
        });

        area.units.push(unit);
        plant.areas.push(area);
        project.plants.push(plant);
        self.projects
            .insert(PROJ_POWER_TURBINE.to_string(), project);
    }

    fn seed_gas_compression_project(&mut self) {
        let project_id = PROJ_GAS_COMPRESSION
            .parse::<ProjectId>()
            .unwrap_or_else(|_| ProjectId::new());
        let mut project = Project::new("Natural Gas Compression & Anti-Surge Train");
        project.id = project_id;
        project.metadata = ProjectMetadata {
            description: "Multi-stage natural gas compression facility with suction knockout drum, aftercooler, discharge separator, and anti-surge recirculation loop.".to_string(),
            author: "NebuDCS Standard Library".to_string(),
            created: "2026-09-07T00:00:00Z".to_string(),
            modified: "2026-09-07T00:00:00Z".to_string(),
        };

        let mut plant = Plant::new("Gas Plant Area 30");
        let mut area = Area::new("Gas Conditioning Train");
        let mut unit = Unit::new("Unit 300 Compression & Surge Control");

        let v301_id = EquipmentId::new();
        let c301_id = EquipmentId::new();
        let e301_id = EquipmentId::new();
        let v302_id = EquipmentId::new();
        let fv301_id = EquipmentId::new();

        let vessel_def_id = "standard.vessel.vertical_separator"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let comp_def_id = "standard.compressor.centrifugal"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let hx_def_id = "standard.heat_exchanger.shell_and_tube"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let valve_def_id = "standard.valve.control_globe"
            .parse::<EquipmentDefinitionId>()
            .unwrap();

        let gas_out_port = "gas_out".parse::<PortId>().unwrap();
        let suction_port = "suction".parse::<PortId>().unwrap();
        let discharge_port = "discharge".parse::<PortId>().unwrap();
        let recycle_port = "recycle".parse::<PortId>().unwrap();
        let tube_in_port = "tube_in".parse::<PortId>().unwrap();
        let tube_out_port = "tube_out".parse::<PortId>().unwrap();
        let feed_port = "feed".parse::<PortId>().unwrap();
        let inlet_port = "inlet".parse::<PortId>().unwrap();
        let outlet_port = "outlet".parse::<PortId>().unwrap();

        let mut v301_params = IndexMap::new();
        v301_params.insert("volume_m3".to_string(), 30.0);
        v301_params.insert("volume".to_string(), 30.0);
        v301_params.insert("operating_pressure".to_string(), 450000.0);
        unit.equipment.push(EquipmentInstance {
            id: v301_id,
            definition_id: vessel_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "V-301".to_string(),
            parameters: v301_params,
            position: Some(Position2D { x: 120.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut c301_params = IndexMap::new();
        c301_params.insert("rated_head".to_string(), 140.0);
        c301_params.insert("rated_power".to_string(), 600000.0);
        c301_params.insert("speed".to_string(), 3600.0);
        c301_params.insert("rated_flow".to_string(), 25.0);
        unit.equipment.push(EquipmentInstance {
            id: c301_id,
            definition_id: comp_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "C-301".to_string(),
            parameters: c301_params,
            position: Some(Position2D { x: 360.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut e301_params = IndexMap::new();
        e301_params.insert("area_m2".to_string(), 200.0);
        e301_params.insert("overall_u".to_string(), 500.0);
        e301_params.insert("design_duty".to_string(), 1200000.0);
        e301_params.insert("process_inlet_t".to_string(), 125.0);
        unit.equipment.push(EquipmentInstance {
            id: e301_id,
            definition_id: hx_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "E-301".to_string(),
            parameters: e301_params,
            position: Some(Position2D { x: 600.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut v302_params = IndexMap::new();
        v302_params.insert("volume_m3".to_string(), 45.0);
        v302_params.insert("volume".to_string(), 45.0);
        v302_params.insert("operating_pressure".to_string(), 3000000.0);
        unit.equipment.push(EquipmentInstance {
            id: v302_id,
            definition_id: vessel_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "V-302".to_string(),
            parameters: v302_params,
            position: Some(Position2D { x: 840.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut fv301_params = IndexMap::new();
        fv301_params.insert("rated_cv".to_string(), 100.0);
        fv301_params.insert("cv".to_string(), 100.0);
        fv301_params.insert("position".to_string(), 15.0);
        unit.equipment.push(EquipmentInstance {
            id: fv301_id,
            definition_id: valve_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "FV-301".to_string(),
            parameters: fv301_params,
            position: Some(Position2D { x: 600.0, y: 340.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        // 2. Streams
        unit.streams.push(StreamDefinition::new(
            "S-GAS-01",
            StreamType::Material,
            v301_id,
            gas_out_port,
            c301_id,
            suction_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-GAS-02",
            StreamType::Material,
            c301_id,
            discharge_port,
            e301_id,
            tube_in_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-GAS-03",
            StreamType::Material,
            e301_id,
            tube_out_port,
            v302_id,
            feed_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-SURGE-01",
            StreamType::Material,
            v302_id,
            gas_out_port,
            fv301_id,
            inlet_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-SURGE-02",
            StreamType::Material,
            fv301_id,
            outlet_port,
            c301_id,
            recycle_port,
        ));

        // 3. Instruments
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "PT-301".to_string(),
            equipment_id: Some(v301_id),
            port_id: Some(gas_out_port),
            measurement_kind: "Pressure".to_string(),
            range_min: 0.0,
            range_max: 10.0,
            unit: "bar".to_string(),
            position: Some(Position2D { x: 120.0, y: 100.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "FT-301".to_string(),
            equipment_id: Some(c301_id),
            port_id: Some(suction_port),
            measurement_kind: "Flow".to_string(),
            range_min: 0.0,
            range_max: 50.0,
            unit: "kg/s".to_string(),
            position: Some(Position2D { x: 260.0, y: 100.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "PT-302".to_string(),
            equipment_id: Some(v302_id),
            port_id: Some(feed_port),
            measurement_kind: "Pressure".to_string(),
            range_min: 0.0,
            range_max: 50.0,
            unit: "bar".to_string(),
            position: Some(Position2D { x: 840.0, y: 100.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "TI-301".to_string(),
            equipment_id: Some(e301_id),
            port_id: Some(tube_out_port),
            measurement_kind: "Temperature".to_string(),
            range_min: 0.0,
            range_max: 150.0,
            unit: "°C".to_string(),
            position: Some(Position2D { x: 600.0, y: 100.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "LT-301".to_string(),
            equipment_id: Some(v301_id),
            port_id: None,
            measurement_kind: "Level".to_string(),
            range_min: 0.0,
            range_max: 100.0,
            unit: "%".to_string(),
            position: Some(Position2D { x: 120.0, y: 260.0 }),
        });

        // 4. Control Loops
        unit.control_loops.push(ControlLoopInstance {
            id: EntityId::new(),
            name: "FIC-301".to_string(),
            pv_tag: "FT-301".to_string(),
            sp: 15.0,
            out_tag: "FV-301.OPENING".to_string(),
            kp: 1.5,
            ti: 8.0,
            td: 0.0,
            reverse_acting: true,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 100.0,
            position: Some(Position2D { x: 480.0, y: 40.0 }),
        });
        unit.control_loops.push(ControlLoopInstance {
            id: EntityId::new(),
            name: "PIC-301".to_string(),
            pv_tag: "PT-302".to_string(),
            sp: 30.0,
            out_tag: "C-301.SPEED".to_string(),
            kp: 1.0,
            ti: 15.0,
            td: 0.0,
            reverse_acting: false,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 100.0,
            position: Some(Position2D { x: 720.0, y: 40.0 }),
        });
        unit.control_loops.push(ControlLoopInstance {
            id: EntityId::new(),
            name: "TIC-301".to_string(),
            pv_tag: "TI-301".to_string(),
            sp: 45.0,
            out_tag: "E-301.DUTY".to_string(),
            kp: 1.2,
            ti: 20.0,
            td: 0.0,
            reverse_acting: false,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 100.0,
            position: Some(Position2D { x: 600.0, y: 40.0 }),
        });

        // 5. Alarms
        unit.alarms.push(AlarmInstance {
            id: EntityId::new(),
            name: "PAL-301".to_string(),
            pv_tag: "PT-301".to_string(),
            alarm_type: AlarmType::Lo,
            severity: AlarmPriority::High,
            trip_limit: 2.0,
            deadband: 0.2,
            description: "Suction pressure low warning".to_string(),
        });
        unit.alarms.push(AlarmInstance {
            id: EntityId::new(),
            name: "PAH-302".to_string(),
            pv_tag: "PT-302".to_string(),
            alarm_type: AlarmType::Hi,
            severity: AlarmPriority::Critical,
            trip_limit: 35.0,
            deadband: 0.5,
            description: "Compressor discharge overpressure".to_string(),
        });
        unit.alarms.push(AlarmInstance {
            id: EntityId::new(),
            name: "TAH-301".to_string(),
            pv_tag: "TI-301".to_string(),
            alarm_type: AlarmType::Hi,
            severity: AlarmPriority::High,
            trip_limit: 140.0,
            deadband: 2.0,
            description: "Aftercooler outlet temperature high".to_string(),
        });

        // 6. Interlocks
        unit.interlocks.push(InterlockInstance {
            id: EntityId::new(),
            name: "TRIP-C301".to_string(),
            source_tag: "LT-301".to_string(),
            condition: ">".to_string(),
            threshold: 85.0,
            target_action: "TRIP".to_string(),
            target_equipment: Some(c301_id),
            latched: true,
            description: "Liquid carryover prevention trip".to_string(),
        });
        unit.interlocks.push(InterlockInstance {
            id: EntityId::new(),
            name: "OPEN-SURGE".to_string(),
            source_tag: "PT-301".to_string(),
            condition: "<".to_string(),
            threshold: 1.5,
            target_action: "OPEN".to_string(),
            target_equipment: Some(fv301_id),
            latched: false,
            description: "Suction starvation anti-surge full opening".to_string(),
        });

        // Seed Stage 5 Anti-Surge Strategy
        project
            .advanced_control
            .push(ProjectAdvancedControlStrategy {
            id: "strat-surge-301".to_string(),
            name: "Compressor K-301 Anti-Surge Controller".to_string(),
            strategy_type: "AntiSurge".to_string(),
            description:
                "Surge Limit Line and Surge Control Line envelope protection with derivative kick"
                    .to_string(),
            enabled: true,
            parameters: [
                ("flow_tag".to_string(), "FT-301.FLOW".to_string()),
                ("p_suction_tag".to_string(), "PT-301".to_string()),
                ("p_discharge_tag".to_string(), "PT-302".to_string()),
                ("sll_slope_a".to_string(), "0.35".to_string()),
                ("sll_intercept_b".to_string(), "0.50".to_string()),
                ("safety_margin".to_string(), "0.10".to_string()),
                ("kp".to_string(), "2.0".to_string()),
                ("ti".to_string(), "1.0".to_string()),
                ("derivative_kick_gain".to_string(), "1.5".to_string()),
                ("kick_threshold_rate".to_string(), "2.0".to_string()),
                ("target_actuator".to_string(), "FV-301".to_string()),
                ("recycle_valve_tag".to_string(), "ASV-301.OUT".to_string()),
            ]
            .into_iter()
            .collect(),
        });

        unit.boundary_conditions.push(BoundaryConditionInstance {
            id: EntityId::new(),
            name: "Wellhead Raw Gas Feed".to_string(),
            condition_type: "feed".to_string(),
            target_equipment_id: None,
            target_port_id: None,
            target_stream_id: None,
            flow_value: 12000.0,
            flow_unit: "Nm3/h".to_string(),
            pressure_value: 12.0,
            pressure_unit: "barg".to_string(),
            temperature_value: 28.0,
            temperature_unit: "°C".to_string(),
            phase: "Vapor".to_string(),
            density: Some(0.85),
            molecular_weight: Some(0.0175),
            enthalpy: None,
            quality: None,
            composition: IndexMap::new(),
            validity_min_flow: None,
            validity_max_flow: None,
            source_provenance: "Wellhead Gathering Manifold".to_string(),
            enabled: true,
        });

        area.units.push(unit);
        plant.areas.push(area);
        project.plants.push(plant);
        self.projects
            .insert(PROJ_GAS_COMPRESSION.to_string(), project);
    }

    fn seed_chemical_project(&mut self) {
        let project_id = PROJ_CHEM_REACTION
            .parse::<ProjectId>()
            .unwrap_or_else(|_| ProjectId::new());
        let mut project = Project::new("Continuous Exothermic Reaction Train");
        project.id = project_id;
        project.metadata = ProjectMetadata {
            description: "Exothermic chemical synthesis plant featuring reactant charge pump, feed preheater, jacketed CSTR, product degasser, and liquid recycle pump.".to_string(),
            author: "NebuDCS Standard Library".to_string(),
            created: "2026-09-07T00:00:00Z".to_string(),
            modified: "2026-09-07T00:00:00Z".to_string(),
        };

        let mut plant = Plant::new("Chemical Area 40");
        let mut area = Area::new("Polymer Synthesis");
        let mut unit = Unit::new("Unit 400 Exothermic Reactor Loop");

        let tk401_id = EquipmentId::new();
        let p401_id = EquipmentId::new();
        let e401_id = EquipmentId::new();
        let r401_id = EquipmentId::new();
        let v401_id = EquipmentId::new();
        let p402_id = EquipmentId::new();

        let tank_def_id = "standard.tank.atmospheric"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let pump_def_id = "standard.pump.centrifugal"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let hx_def_id = "standard.heat_exchanger.shell_and_tube"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let rx_def_id = "standard.reactor.cstr"
            .parse::<EquipmentDefinitionId>()
            .unwrap();
        let vessel_def_id = "standard.vessel.vertical_separator"
            .parse::<EquipmentDefinitionId>()
            .unwrap();

        let outlet_port = "outlet".parse::<PortId>().unwrap();
        let suction_port = "suction".parse::<PortId>().unwrap();
        let discharge_port = "discharge".parse::<PortId>().unwrap();
        let tube_in_port = "tube_in".parse::<PortId>().unwrap();
        let tube_out_port = "tube_out".parse::<PortId>().unwrap();
        let feed_port = "feed".parse::<PortId>().unwrap();
        let effluent_port = "effluent".parse::<PortId>().unwrap();
        let liquid_out_port = "liquid_out".parse::<PortId>().unwrap();

        let mut tk401_params = IndexMap::new();
        tk401_params.insert("capacity_m3".to_string(), 3000.0);
        tk401_params.insert("level".to_string(), 70.0);
        unit.equipment.push(EquipmentInstance {
            id: tk401_id,
            definition_id: tank_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "TK-401".to_string(),
            parameters: tk401_params,
            position: Some(Position2D { x: 80.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut p401_params = IndexMap::new();
        p401_params.insert("rated_flow".to_string(), 80.0);
        p401_params.insert("rated_head".to_string(), 100.0);
        p401_params.insert("speed_rpm".to_string(), 2950.0);
        unit.equipment.push(EquipmentInstance {
            id: p401_id,
            definition_id: pump_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "P-401".to_string(),
            parameters: p401_params,
            position: Some(Position2D { x: 240.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut e401_params = IndexMap::new();
        e401_params.insert("area_m2".to_string(), 120.0);
        e401_params.insert("overall_u".to_string(), 600.0);
        e401_params.insert("design_duty".to_string(), 800000.0);
        e401_params.insert("process_inlet_t".to_string(), 25.0);
        unit.equipment.push(EquipmentInstance {
            id: e401_id,
            definition_id: hx_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "E-401".to_string(),
            parameters: e401_params,
            position: Some(Position2D { x: 420.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut r401_params = IndexMap::new();
        r401_params.insert("volume_m3".to_string(), 20.0);
        r401_params.insert("volume".to_string(), 20.0);
        r401_params.insert("thermal_duty".to_string(), 450000.0);
        r401_params.insert("tau".to_string(), 5.0);
        unit.equipment.push(EquipmentInstance {
            id: r401_id,
            definition_id: rx_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "R-401".to_string(),
            parameters: r401_params,
            position: Some(Position2D { x: 620.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut v401_params = IndexMap::new();
        v401_params.insert("volume_m3".to_string(), 35.0);
        v401_params.insert("volume".to_string(), 35.0);
        v401_params.insert("operating_pressure".to_string(), 1200000.0);
        unit.equipment.push(EquipmentInstance {
            id: v401_id,
            definition_id: vessel_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "V-401".to_string(),
            parameters: v401_params,
            position: Some(Position2D { x: 840.0, y: 180.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        let mut p402_params = IndexMap::new();
        p402_params.insert("rated_flow".to_string(), 40.0);
        p402_params.insert("rated_head".to_string(), 90.0);
        p402_params.insert("speed_rpm".to_string(), 2950.0);
        unit.equipment.push(EquipmentInstance {
            id: p402_id,
            definition_id: pump_def_id,
            definition_version: SchemaVersion::new(1, 0, 0),
            name: "P-402".to_string(),
            parameters: p402_params,
            position: Some(Position2D { x: 620.0, y: 340.0 }),
            display_width: None,
            display_height: None,
            rotation: None,
        });

        // 2. Streams
        unit.streams.push(StreamDefinition::new(
            "S-CHEM-01",
            StreamType::Material,
            tk401_id,
            outlet_port,
            p401_id,
            suction_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-CHEM-02",
            StreamType::Material,
            p401_id,
            discharge_port,
            e401_id,
            tube_in_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-CHEM-03",
            StreamType::Material,
            e401_id,
            tube_out_port,
            r401_id,
            feed_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-CHEM-04",
            StreamType::Material,
            r401_id,
            effluent_port,
            v401_id,
            feed_port,
        ));
        unit.streams.push(StreamDefinition::new(
            "S-RECYCLE-01",
            StreamType::Material,
            v401_id,
            liquid_out_port,
            p402_id,
            suction_port,
        ));

        // 3. Instruments
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "FT-401".to_string(),
            equipment_id: Some(p401_id),
            port_id: Some(discharge_port),
            measurement_kind: "Flow".to_string(),
            range_min: 0.0,
            range_max: 10000.0,
            unit: "kg/h".to_string(),
            position: Some(Position2D { x: 240.0, y: 100.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "TI-401".to_string(),
            equipment_id: Some(r401_id),
            port_id: None,
            measurement_kind: "Temperature".to_string(),
            range_min: 0.0,
            range_max: 300.0,
            unit: "°C".to_string(),
            position: Some(Position2D { x: 620.0, y: 100.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "PT-401".to_string(),
            equipment_id: Some(r401_id),
            port_id: None,
            measurement_kind: "Pressure".to_string(),
            range_min: 0.0,
            range_max: 30.0,
            unit: "bar".to_string(),
            position: Some(Position2D { x: 620.0, y: 40.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "LT-401".to_string(),
            equipment_id: Some(r401_id),
            port_id: None,
            measurement_kind: "Level".to_string(),
            range_min: 0.0,
            range_max: 100.0,
            unit: "%".to_string(),
            position: Some(Position2D { x: 620.0, y: 260.0 }),
        });
        unit.instruments.push(InstrumentInstance {
            id: EntityId::new(),
            tag: "AT-401".to_string(),
            equipment_id: Some(v401_id),
            port_id: Some(feed_port),
            measurement_kind: "Purity".to_string(),
            range_min: 0.0,
            range_max: 1.0,
            unit: "fraction".to_string(),
            position: Some(Position2D { x: 840.0, y: 100.0 }),
        });

        // 4. Control Loops
        unit.control_loops.push(ControlLoopInstance {
            id: EntityId::new(),
            name: "TIC-401".to_string(),
            pv_tag: "TI-401".to_string(),
            sp: 165.0,
            out_tag: "R-401.COOLING".to_string(),
            kp: 2.5,
            ti: 15.0,
            td: 1.0,
            reverse_acting: false,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 100.0,
            position: Some(Position2D { x: 520.0, y: 40.0 }),
        });
        unit.control_loops.push(ControlLoopInstance {
            id: EntityId::new(),
            name: "LIC-401".to_string(),
            pv_tag: "LT-401".to_string(),
            sp: 65.0,
            out_tag: "R-401.OPENING".to_string(),
            kp: 1.8,
            ti: 20.0,
            td: 0.0,
            reverse_acting: false,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 100.0,
            position: Some(Position2D { x: 720.0, y: 40.0 }),
        });
        unit.control_loops.push(ControlLoopInstance {
            id: EntityId::new(),
            name: "FIC-401".to_string(),
            pv_tag: "FT-401".to_string(),
            sp: 5000.0,
            out_tag: "P-401.SPEED".to_string(),
            kp: 1.0,
            ti: 10.0,
            td: 0.0,
            reverse_acting: false,
            mode: "AUTO".to_string(),
            output_min: 0.0,
            output_max: 100.0,
            position: Some(Position2D { x: 320.0, y: 40.0 }),
        });

        // 5. Alarms
        unit.alarms.push(AlarmInstance {
            id: EntityId::new(),
            name: "TAH-401".to_string(),
            pv_tag: "TI-401".to_string(),
            alarm_type: AlarmType::Hi,
            severity: AlarmPriority::High,
            trip_limit: 190.0,
            deadband: 2.0,
            description: "Reactor temperature high warning".to_string(),
        });
        unit.alarms.push(AlarmInstance {
            id: EntityId::new(),
            name: "TAHH-401".to_string(),
            pv_tag: "TI-401".to_string(),
            alarm_type: AlarmType::Hi,
            severity: AlarmPriority::Critical,
            trip_limit: 210.0,
            deadband: 2.0,
            description: "Emergency thermal runaway condition".to_string(),
        });
        unit.alarms.push(AlarmInstance {
            id: EntityId::new(),
            name: "LAH-401".to_string(),
            pv_tag: "LT-401".to_string(),
            alarm_type: AlarmType::Hi,
            severity: AlarmPriority::High,
            trip_limit: 85.0,
            deadband: 1.0,
            description: "Reactor high level warning".to_string(),
        });

        // 6. Interlocks
        unit.interlocks.push(InterlockInstance {
            id: EntityId::new(),
            name: "TRIP-RUNAWAY".to_string(),
            source_tag: "TI-401".to_string(),
            condition: ">".to_string(),
            threshold: 210.0,
            target_action: "TRIP".to_string(),
            target_equipment: Some(p401_id),
            latched: true,
            description: "Cut feed pump on thermal runaway".to_string(),
        });
        unit.interlocks.push(InterlockInstance {
            id: EntityId::new(),
            name: "TRIP-FEED-LOW".to_string(),
            source_tag: "TK-401.LEVEL".to_string(),
            condition: "<".to_string(),
            threshold: 15.0,
            target_action: "TRIP".to_string(),
            target_equipment: Some(p401_id),
            latched: true,
            description: "Low level pump protection".to_string(),
        });

        // Seed Stage 5 Ratio Strategy
        project
            .advanced_control
            .push(ProjectAdvancedControlStrategy {
                id: "strat-ratio-401".to_string(),
                name: "Reactant Stoichiometric Ratio Control".to_string(),
                strategy_type: "Ratio".to_string(),
                description:
                    "Maintains Reactant B flow at stoichiometric ratio to wild Reactant A feed"
                        .to_string(),
                enabled: true,
                parameters: [
                    ("wild_pv".to_string(), "FT-401.FLOW".to_string()),
                    ("controlled_pv".to_string(), "FT-402.FLOW".to_string()),
                    ("ratio".to_string(), "0.50".to_string()),
                    ("bias".to_string(), "0.0".to_string()),
                    ("ratio_min".to_string(), "0.10".to_string()),
                    ("ratio_max".to_string(), "2.0".to_string()),
                    ("kp".to_string(), "1.2".to_string()),
                    ("ti".to_string(), "5.0".to_string()),
                    ("td".to_string(), "0.0".to_string()),
                    ("output_min".to_string(), "0.0".to_string()),
                    ("output_max".to_string(), "100.0".to_string()),
                    ("target_actuator".to_string(), "FV-402".to_string()),
                    ("output_tag".to_string(), "FIC_402.OUT".to_string()),
                ]
                .into_iter()
                .collect(),
            });

        unit.boundary_conditions.push(BoundaryConditionInstance {
            id: EntityId::new(),
            name: "Reactant A Chemical Feed".to_string(),
            condition_type: "feed".to_string(),
            target_equipment_id: None,
            target_port_id: None,
            target_stream_id: None,
            flow_value: 20.0,
            flow_unit: "m3/h".to_string(),
            pressure_value: 3.5,
            pressure_unit: "barg".to_string(),
            temperature_value: 20.0,
            temperature_unit: "°C".to_string(),
            phase: "Liquid".to_string(),
            density: Some(1050.0),
            molecular_weight: Some(0.092),
            enthalpy: None,
            quality: None,
            composition: IndexMap::new(),
            validity_min_flow: None,
            validity_max_flow: None,
            source_provenance: "Chemical Storage Farm".to_string(),
            enabled: true,
        });

        area.units.push(unit);
        plant.areas.push(area);
        project.plants.push(plant);
        self.projects
            .insert(PROJ_CHEM_REACTION.to_string(), project);
    }

    fn seed_nht_reference_project(&mut self) {
        let project = crate::nht_seed::build_nht_reference_project();
        self.projects
            .insert(PROJ_NHT_REFERENCE.to_string(), project);
    }

    /// Lists high-level summaries of all available projects.
    pub fn list_projects(&self) -> Vec<ProjectSummaryDto> {
        self.projects
            .iter()
            .map(|(key, p)| {
                let mut eq_count = 0;
                let mut str_count = 0;
                for plant in &p.plants {
                    for area in &plant.areas {
                        for unit in &area.units {
                            eq_count += unit.equipment.len();
                            str_count += unit.streams.len();
                        }
                    }
                }

                ProjectSummaryDto {
                    id: key.clone(),
                    name: p.name.clone(),
                    description: p.metadata.description.clone(),
                    author: p.metadata.author.clone(),
                    schema_version: p.schema_version.to_string(),
                    plant_count: p.plants.len(),
                    equipment_count: eq_count,
                    stream_count: str_count,
                    created: p.metadata.created.clone(),
                    modified: p.metadata.modified.clone(),
                }
            })
            .collect()
    }

    /// Retrieves detailed project data ready for the Studio editor.
    pub fn get_project(&self, id: &str) -> Option<ProjectDetailDto> {
        self.projects.get(id).map(|p| {
            let mut dto = Self::project_to_dto(p);
            dto.id = id.to_string();
            dto
        })
    }

    /// Creates a new empty project.
    pub fn create_project(&mut self, req: CreateProjectRequestDto) -> ProjectDetailDto {
        let proj_id = ProjectId::new();
        let key = format!("proj-{}", proj_id);
        let mut project = Project::new(req.name);
        project.id = proj_id;
        project.metadata = ProjectMetadata {
            description: req.description.unwrap_or_default(),
            author: req.author.unwrap_or_else(|| "NebuDCS Engineer".to_string()),
            created: chrono_now_iso(),
            modified: chrono_now_iso(),
        };

        // Create standard default hierarchy
        let mut plant = Plant::new("Plant 01");
        let mut area = Area::new("Area 01");
        let unit = Unit::new("Unit 01");
        area.units.push(unit);
        plant.areas.push(area);
        project.plants.push(plant);

        let mut dto = Self::project_to_dto(&project);
        dto.id = key.clone();
        if let Some(ref store) = self.disk_store {
            let _ = store.save_json("projects", &key, &project);
        }
        self.projects.insert(key, project);
        dto
    }

    /// Saves or updates a project from Studio editor DTOs and persists to disk.
    pub fn save_project(&mut self, req: SaveProjectRequestDto) -> Result<ProjectDetailDto, String> {
        let mut project = Self::dto_to_project(&req.project)?;
        let key = req.project.id.clone();
        project.id = key
            .parse::<ProjectId>()
            .unwrap_or_else(|_| ProjectId::new());
        if let Some(ref store) = self.disk_store {
            if let Err(e) = store.save_json("projects", &key, &project) {
                tracing::warn!("Failed to persist project '{key}' to disk: {e}");
            }
        }
        self.projects.insert(key.clone(), project);
        self.get_project(&key)
            .ok_or_else(|| "Failed to retrieve saved project".to_string())
    }

    /// Deletes a project from memory and persistent disk store.
    pub fn delete_project(&mut self, id: &str) -> Result<bool, String> {
        let removed = self.projects.remove(id).is_some();
        if let Some(ref store) = self.disk_store {
            let _ = store.delete_json("projects", id);
        }
        Ok(removed)
    }

    /// Validates a project against registered equipment definitions.
    pub fn validate_project(
        &self,
        id: &str,
        defs: &IndexMap<EquipmentDefinitionId, EquipmentDefinition>,
    ) -> Result<ProjectValidationResponseDto, String> {
        let project = self
            .projects
            .get(id)
            .ok_or_else(|| format!("Project '{id}' not found"))?;

        let summary = run_validate_project(project, defs);
        let mut dto = Self::validation_summary_to_dto(&summary);

        // Stage 4 validations:
        // 1. Controller and task validation
        for ctrl in &project.controllers {
            if ctrl.watchdog_timeout_seconds <= 0.0 {
                dto.valid = false;
                dto.error_count += 1;
                dto.results.push(ValidationResultDto {
                    severity: "error".to_string(),
                    code: "VAL_CTRL_WATCHDOG_INVALID".to_string(),
                    message: format!(
                        "Controller '{}' watchdog timeout ({:.2}s) must be positive",
                        ctrl.name, ctrl.watchdog_timeout_seconds
                    ),
                    entity_id: Some(ctrl.id.clone()),
                });
            }

            for task in &ctrl.tasks {
                if task.priority < 1 || task.priority > 15 {
                    dto.valid = false;
                    dto.error_count += 1;
                    dto.results.push(ValidationResultDto {
                        severity: "error".to_string(),
                        code: "VAL_TASK_PRIORITY_RANGE".to_string(),
                        message: format!(
                            "Task '{}' priority ({}) must be between 1 and 15",
                            task.name, task.priority
                        ),
                        entity_id: Some(task.id.clone()),
                    });
                }
                if task.period_seconds <= 0.0 {
                    dto.valid = false;
                    dto.error_count += 1;
                    dto.results.push(ValidationResultDto {
                        severity: "error".to_string(),
                        code: "VAL_TASK_PERIOD_INVALID".to_string(),
                        message: format!(
                            "Task '{}' period ({:.4}s) must be positive",
                            task.name, task.period_seconds
                        ),
                        entity_id: Some(task.id.clone()),
                    });
                }
                if task.watchdog_seconds <= 0.0 {
                    dto.valid = false;
                    dto.error_count += 1;
                    dto.results.push(ValidationResultDto {
                        severity: "error".to_string(),
                        code: "VAL_TASK_WATCHDOG_INVALID".to_string(),
                        message: format!(
                            "Task '{}' watchdog ({:.4}s) must be positive",
                            task.name, task.watchdog_seconds
                        ),
                        entity_id: Some(task.id.clone()),
                    });
                }

                // Compile IEC routines
                for prog in &task.programs {
                    for r in &prog.routines {
                        let compile_res = match r.language.to_ascii_uppercase().as_str() {
                            "ST" => {
                                nebudcs_control::iec61131::compile_st(&r.source_text).map(|_| ())
                            }
                            "LD" => {
                                match serde_json::from_str::<nebudcs_control::iec61131::LdDiagram>(
                                    &r.source_text,
                                ) {
                                    Ok(diag) => {
                                        nebudcs_control::iec61131::compile_ladder(&diag).map(|_| ())
                                    }
                                    Err(e) => {
                                        Err(nebudcs_control::error::ControlError::InvalidParameter(
                                            format!("Invalid LD JSON: {e}"),
                                        ))
                                    }
                                }
                            }
                            "FBD" => {
                                match serde_json::from_str::<nebudcs_control::iec61131::FbdGraph>(
                                    &r.source_text,
                                ) {
                                    Ok(graph) => {
                                        nebudcs_control::iec61131::compile_fbd(&graph).map(|_| ())
                                    }
                                    Err(e) => {
                                        Err(nebudcs_control::error::ControlError::InvalidParameter(
                                            format!("Invalid FBD JSON: {e}"),
                                        ))
                                    }
                                }
                            }
                            "SFC" => {
                                match serde_json::from_str::<nebudcs_control::iec61131::SfcChart>(
                                    &r.source_text,
                                ) {
                                    Ok(chart) => {
                                        nebudcs_control::iec61131::compile_sfc(&chart).map(|_| ())
                                    }
                                    Err(e) => {
                                        Err(nebudcs_control::error::ControlError::InvalidParameter(
                                            format!("Invalid SFC JSON: {e}"),
                                        ))
                                    }
                                }
                            }
                            other => Err(nebudcs_control::error::ControlError::InvalidParameter(
                                format!("Unsupported language '{other}'"),
                            )),
                        };

                        if let Err(err) = compile_res {
                            dto.valid = false;
                            dto.error_count += 1;
                            dto.results.push(ValidationResultDto {
                                severity: "error".to_string(),
                                code: "VAL_IEC_COMPILE_ERROR".to_string(),
                                message: format!(
                                    "Controller '{}' Task '{}' Routine '{}' compile error: {err}",
                                    ctrl.name, task.name, r.name
                                ),
                                entity_id: Some(r.id.clone()),
                            });
                        }
                    }
                }
            }
        }

        // 2. I/O Channel and Mapping validation
        let mut output_owners: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        let mut known_channel_kinds: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for sys in &project.io_systems {
            for rack in &sys.racks {
                for chassis in &rack.chassis {
                    for module in &chassis.modules {
                        let is_input_mod = module.module_type.starts_with("AnalogInput")
                            || module.module_type.starts_with("DigitalInput")
                            || module.module_type.starts_with("AI")
                            || module.module_type.starts_with("DI");
                        let is_output_mod = module.module_type.starts_with("AnalogOutput")
                            || module.module_type.starts_with("DigitalOutput")
                            || module.module_type.starts_with("AO")
                            || module.module_type.starts_with("DO");
                        for ch in &module.channels {
                            let k = if is_input_mod
                                || ch.kind.eq_ignore_ascii_case("AI")
                                || ch.kind.eq_ignore_ascii_case("DI")
                            {
                                "input"
                            } else if is_output_mod
                                || ch.kind.eq_ignore_ascii_case("AO")
                                || ch.kind.eq_ignore_ascii_case("DO")
                            {
                                "output"
                            } else {
                                ""
                            };
                            if !k.is_empty() {
                                known_channel_kinds
                                    .insert(ch.address.to_uppercase(), k.to_string());
                            }
                        }
                    }
                }
            }
        }

        for map in &project.io_mappings {
            let addr_upper = map.channel_address.to_uppercase();
            let is_input_addr = known_channel_kinds
                .get(&addr_upper)
                .map(|k| k == "input")
                .unwrap_or(false)
                || addr_upper.contains("-AI")
                || addr_upper.contains("-DI")
                || addr_upper.contains("%I");
            let is_output_addr = known_channel_kinds
                .get(&addr_upper)
                .map(|k| k == "output")
                .unwrap_or(false)
                || addr_upper.contains("-AO")
                || addr_upper.contains("-DO")
                || addr_upper.contains("%Q");

            if map.direction.eq_ignore_ascii_case("input") && is_output_addr {
                dto.valid = false;
                dto.error_count += 1;
                dto.results.push(ValidationResultDto {
                    severity: "error".to_string(),
                    code: "VAL_HAL_DIRECTION_MISMATCH".to_string(),
                    message: format!(
                        "Input mapping for signal '{}' cannot target output channel '{}'",
                        map.signal_key, map.channel_address
                    ),
                    entity_id: Some(map.channel_address.clone()),
                });
            } else if map.direction.eq_ignore_ascii_case("output") && is_input_addr {
                dto.valid = false;
                dto.error_count += 1;
                dto.results.push(ValidationResultDto {
                    severity: "error".to_string(),
                    code: "VAL_HAL_DIRECTION_MISMATCH".to_string(),
                    message: format!(
                        "Output mapping for signal '{}' cannot target input channel '{}'",
                        map.signal_key, map.channel_address
                    ),
                    entity_id: Some(map.channel_address.clone()),
                });
            }

            // Output channel collision check
            if map.direction.eq_ignore_ascii_case("output") {
                if let Some(existing_ctrl) = output_owners.get(&addr_upper) {
                    if existing_ctrl != &map.controller_id {
                        dto.valid = false;
                        dto.error_count += 1;
                        dto.results.push(ValidationResultDto {
                            severity: "error".to_string(),
                            code: "VAL_HAL_OUTPUT_COLLISION".to_string(),
                            message: format!(
                                "Output channel '{}' mapped to multiple controllers: '{}' and '{}'",
                                map.channel_address, existing_ctrl, map.controller_id
                            ),
                            entity_id: Some(map.channel_address.clone()),
                        });
                    }
                } else {
                    output_owners.insert(addr_upper, map.controller_id.clone());
                }
            }
        }

        // 5. Advanced Process Control Validation
        for strat in &project.advanced_control {
            match strat.strategy_type.to_ascii_uppercase().as_str() {
                "CASCADE" => {
                    let prim = strat
                        .parameters
                        .get("primary_pv")
                        .map(|s| s.trim())
                        .unwrap_or("");
                    let sec = strat
                        .parameters
                        .get("secondary_pv")
                        .map(|s| s.trim())
                        .unwrap_or("");
                    if prim.is_empty() {
                        dto.valid = false;
                        dto.error_count += 1;
                        dto.results.push(ValidationResultDto {
                            severity: "error".to_string(),
                            code: "VAL_STRATEGY_MISSING_PV_TAG".to_string(),
                            message: format!(
                                "Cascade strategy '{}' missing required primary PV tag",
                                strat.name
                            ),
                            entity_id: Some(strat.id.clone()),
                        });
                    }
                    if sec.is_empty() {
                        dto.valid = false;
                        dto.error_count += 1;
                        dto.results.push(ValidationResultDto {
                            severity: "error".to_string(),
                            code: "VAL_STRATEGY_MISSING_PV_TAG".to_string(),
                            message: format!(
                                "Cascade strategy '{}' missing required secondary PV tag",
                                strat.name
                            ),
                            entity_id: Some(strat.id.clone()),
                        });
                    }
                }
                "RATIO" => {
                    let wild = strat
                        .parameters
                        .get("wild_pv")
                        .map(|s| s.trim())
                        .unwrap_or("");
                    let ctrl = strat
                        .parameters
                        .get("controlled_pv")
                        .map(|s| s.trim())
                        .unwrap_or("");
                    if wild.is_empty() || ctrl.is_empty() {
                        dto.valid = false;
                        dto.error_count += 1;
                        dto.results.push(ValidationResultDto {
                            severity: "error".to_string(),
                            code: "VAL_STRATEGY_MISSING_PV_TAG".to_string(),
                            message: format!("Ratio strategy '{}' requires both wild and controlled stream PV tags", strat.name),
                            entity_id: Some(strat.id.clone()),
                        });
                    }
                }
                "FEEDFORWARD" => {
                    let pv = strat
                        .parameters
                        .get("pv_tag")
                        .map(|s| s.trim())
                        .unwrap_or("");
                    let dist = strat
                        .parameters
                        .get("disturbance_tag")
                        .map(|s| s.trim())
                        .unwrap_or("");
                    if pv.is_empty() || dist.is_empty() {
                        dto.valid = false;
                        dto.error_count += 1;
                        dto.results.push(ValidationResultDto {
                            severity: "error".to_string(),
                            code: "VAL_STRATEGY_MISSING_PV_TAG".to_string(),
                            message: format!("Feed-forward strategy '{}' requires both regulated PV and disturbance tags", strat.name),
                            entity_id: Some(strat.id.clone()),
                        });
                    }
                }
                "OVERRIDE" => {
                    let prim = strat
                        .parameters
                        .get("primary_pv")
                        .map(|s| s.trim())
                        .unwrap_or("");
                    let constr = strat
                        .parameters
                        .get("constraint_pv")
                        .map(|s| s.trim())
                        .unwrap_or("");
                    if prim.is_empty() || constr.is_empty() {
                        dto.valid = false;
                        dto.error_count += 1;
                        dto.results.push(ValidationResultDto {
                            severity: "error".to_string(),
                            code: "VAL_STRATEGY_MISSING_PV_TAG".to_string(),
                            message: format!("Override strategy '{}' requires both primary and constraint PV tags", strat.name),
                            entity_id: Some(strat.id.clone()),
                        });
                    }
                }
                "SPLITRANGE" => {
                    let pv = strat
                        .parameters
                        .get("pv_tag")
                        .map(|s| s.trim())
                        .unwrap_or("");
                    if pv.is_empty() {
                        dto.valid = false;
                        dto.error_count += 1;
                        dto.results.push(ValidationResultDto {
                            severity: "error".to_string(),
                            code: "VAL_STRATEGY_MISSING_PV_TAG".to_string(),
                            message: format!(
                                "Split-range strategy '{}' requires master PV tag",
                                strat.name
                            ),
                            entity_id: Some(strat.id.clone()),
                        });
                    }
                }
                "ANTISURGE" => {
                    let flow = strat
                        .parameters
                        .get("flow_tag")
                        .map(|s| s.trim())
                        .unwrap_or("");
                    let p_suc = strat
                        .parameters
                        .get("p_suction_tag")
                        .map(|s| s.trim())
                        .unwrap_or("");
                    let p_dis = strat
                        .parameters
                        .get("p_discharge_tag")
                        .map(|s| s.trim())
                        .unwrap_or("");
                    if flow.is_empty() || p_suc.is_empty() || p_dis.is_empty() {
                        dto.valid = false;
                        dto.error_count += 1;
                        dto.results.push(ValidationResultDto {
                            severity: "error".to_string(),
                            code: "VAL_STRATEGY_MISSING_PV_TAG".to_string(),
                            message: format!("Anti-surge strategy '{}' requires flow, suction pressure, and discharge pressure tags", strat.name),
                            entity_id: Some(strat.id.clone()),
                        });
                    }
                }
                "VALVECHARACTERIZATION" => {
                    let in_tag = strat
                        .parameters
                        .get("input_tag")
                        .map(|s| s.trim())
                        .unwrap_or("");
                    if in_tag.is_empty() {
                        dto.valid = false;
                        dto.error_count += 1;
                        dto.results.push(ValidationResultDto {
                            severity: "error".to_string(),
                            code: "VAL_STRATEGY_MISSING_PV_TAG".to_string(),
                            message: format!(
                                "Valve characterization strategy '{}' requires input demand tag",
                                strat.name
                            ),
                            entity_id: Some(strat.id.clone()),
                        });
                    }
                }
                _ => {}
            }
        }

        // 5. Process Boundary Condition Validation
        let mut total_equipment = 0;
        let mut active_boundary_conditions = 0;
        for plant in &project.plants {
            for area in &plant.areas {
                for unit in &area.units {
                    total_equipment += unit.equipment.len();
                    active_boundary_conditions +=
                        unit.boundary_conditions.iter().filter(|b| b.enabled).count();

                    for bc in &unit.boundary_conditions {
                        if bc.flow_value < 0.0 || !bc.flow_value.is_finite() {
                            dto.valid = false;
                            dto.error_count += 1;
                            dto.results.push(ValidationResultDto {
                                severity: "error".to_string(),
                                code: "VAL_BOUNDARY_FLOW_INVALID".to_string(),
                                message: format!(
                                    "Boundary condition '{}' has invalid flow rate: {}",
                                    bc.name, bc.flow_value
                                ),
                                entity_id: Some(bc.id.to_string()),
                            });
                        }
                    }
                }
            }
        }

        if total_equipment > 0 && active_boundary_conditions == 0 {
            dto.warning_count += 1;
            dto.results.push(ValidationResultDto {
                severity: "warning".to_string(),
                code: "MISSING_PROCESS_BOUNDARY_CONDITION".to_string(),
                message: format!(
                    "Project has {} equipment items across process units but zero active process boundary conditions (feeds, utilities, or export sinks). An executable process simulation requires defined boundary conditions.",
                    total_equipment
                ),
                entity_id: Some(id.to_string()),
            });
        }

        Ok(dto)
    }

    /// Queries cross-references across instruments, loops, alarms, interlocks, controllers, routines, mappings, and tags.
    pub fn get_cross_reference(
        &self,
        id: &str,
        query: CrossReferenceQueryDto,
    ) -> Result<CrossReferenceResultDto, String> {
        let project = self
            .projects
            .get(id)
            .ok_or_else(|| format!("Project '{id}' not found"))?;

        let search_target = query.tag_or_entity.trim();
        let target_lower = search_target.to_lowercase();
        let match_all = search_target.is_empty() || search_target == "*";

        let mut entries = Vec::new();

        // 1. Canonical Project Tags
        for tag in &project.tags {
            if match_all
                || tag.name.to_lowercase().contains(&target_lower)
                || tag.id.to_lowercase().contains(&target_lower)
            {
                entries.push(CrossReferenceEntryDto {
                    tag: tag.name.clone(),
                    entity_type: "Tag".to_string(),
                    role: "Defined".to_string(),
                    location: "Tag Database".to_string(),
                    details: format!(
                        "Type: {}, Direction: {}, Source: {}",
                        tag.data_type, tag.direction, tag.source
                    ),
                });
            }
        }

        // 2. Instruments in Units
        for plant in &project.plants {
            for area in &plant.areas {
                for unit in &area.units {
                    for inst in &unit.instruments {
                        if match_all || inst.tag.to_lowercase().contains(&target_lower) {
                            entries.push(CrossReferenceEntryDto {
                                tag: inst.tag.clone(),
                                entity_type: "Instrument".to_string(),
                                role: "Measured".to_string(),
                                location: format!("P&ID ({})", unit.name),
                                details: format!(
                                    "Kind: {}, Range: {:.1}..{:.1} {}",
                                    inst.measurement_kind,
                                    inst.range_min,
                                    inst.range_max,
                                    inst.unit
                                ),
                            });
                        }
                    }

                    // 3. Control Loops
                    for cl in &unit.control_loops {
                        if match_all
                            || cl.pv_tag.to_lowercase().contains(&target_lower)
                            || cl.name.to_lowercase().contains(&target_lower)
                        {
                            entries.push(CrossReferenceEntryDto {
                                tag: cl.pv_tag.clone(),
                                entity_type: "ControlLoop".to_string(),
                                role: "Read (PV)".to_string(),
                                location: format!("Control Loop ({})", cl.name),
                                details: format!("Mode: {}, SP: {:.2}", cl.mode, cl.sp),
                            });
                        }
                        if match_all
                            || cl.out_tag.to_lowercase().contains(&target_lower)
                            || cl.name.to_lowercase().contains(&target_lower)
                        {
                            entries.push(CrossReferenceEntryDto {
                                tag: cl.out_tag.clone(),
                                entity_type: "ControlLoop".to_string(),
                                role: "Written (OUT)".to_string(),
                                location: format!("Control Loop ({})", cl.name),
                                details: format!("Mode: {}, Kp: {:.2}", cl.mode, cl.kp),
                            });
                        }
                    }

                    // 4. Alarms
                    for alm in &unit.alarms {
                        if match_all
                            || alm.pv_tag.to_lowercase().contains(&target_lower)
                            || alm.name.to_lowercase().contains(&target_lower)
                        {
                            entries.push(CrossReferenceEntryDto {
                                tag: alm.pv_tag.clone(),
                                entity_type: "Alarm".to_string(),
                                role: "Monitored".to_string(),
                                location: format!("Alarm ({})", alm.name),
                                details: format!(
                                    "Type: {:?}, Limit: {:.2}",
                                    alm.alarm_type, alm.trip_limit
                                ),
                            });
                        }
                    }

                    // 5. Interlocks
                    for ilk in &unit.interlocks {
                        if match_all
                            || ilk.source_tag.to_lowercase().contains(&target_lower)
                            || ilk.name.to_lowercase().contains(&target_lower)
                        {
                            entries.push(CrossReferenceEntryDto {
                                tag: ilk.source_tag.clone(),
                                entity_type: "Interlock".to_string(),
                                role: "Monitored (Condition)".to_string(),
                                location: format!("Interlock ({})", ilk.name),
                                details: format!(
                                    "Action: {}, Threshold: {:.2}",
                                    ilk.target_action, ilk.threshold
                                ),
                            });
                        }
                    }
                }
            }
        }

        // 6. Controllers, Tasks & IEC Routines
        for ctrl in &project.controllers {
            for task in &ctrl.tasks {
                for prog in &task.programs {
                    for routine in &prog.routines {
                        let contains_ref = match_all
                            || routine.source_text.to_lowercase().contains(&target_lower)
                            || routine.name.to_lowercase().contains(&target_lower);
                        if contains_ref {
                            entries.push(CrossReferenceEntryDto {
                                tag: search_target.to_string(),
                                entity_type: "Routine".to_string(),
                                role: "Logic (Referenced)".to_string(),
                                location: format!("{}/{}/{}", ctrl.name, task.name, routine.name),
                                details: format!(
                                    "Lang: {}, Enabled: {}",
                                    routine.language, routine.enabled
                                ),
                            });
                        }
                    }
                }
            }
        }

        // 7. I/O Channel Mappings
        for map in &project.io_mappings {
            let matched = match_all
                || map.signal_key.to_lowercase().contains(&target_lower)
                || map.channel_address.to_lowercase().contains(&target_lower)
                || map
                    .target_tag
                    .as_ref()
                    .map(|t| t.to_lowercase().contains(&target_lower))
                    .unwrap_or(false);
            if matched {
                entries.push(CrossReferenceEntryDto {
                    tag: map
                        .target_tag
                        .clone()
                        .unwrap_or_else(|| map.signal_key.clone()),
                    entity_type: "ChannelMapping".to_string(),
                    role: format!("Mapped ({})", map.direction),
                    location: format!("HAL Channel {}", map.channel_address),
                    details: format!(
                        "Controller: {}, Signal: {}",
                        map.controller_id, map.signal_key
                    ),
                });
            }
        }

        Ok(CrossReferenceResultDto {
            target: search_target.to_string(),
            entries,
        })
    }

    /// Converts domain Project into transport ProjectDetailDto.
    pub fn project_to_dto(project: &Project) -> ProjectDetailDto {
        let mut equipment_dtos = Vec::new();
        let mut stream_dtos = Vec::new();
        let mut inst_dtos = Vec::new();
        let mut loop_dtos = Vec::new();
        let mut alm_dtos = Vec::new();
        let mut ilk_dtos = Vec::new();
        let mut boundary_dtos = Vec::new();
        let mut initial_dtos = Vec::new();

        for plant in &project.plants {
            for area in &plant.areas {
                for unit in &area.units {
                    for eq in &unit.equipment {
                        let mut params = BTreeMap::new();
                        for (k, v) in &eq.parameters {
                            params.insert(k.clone(), *v);
                        }
                        equipment_dtos.push(StudioEquipmentDto {
                            id: eq.id.to_string(),
                            definition_id: eq.definition_id.to_string(),
                            definition_version: eq.definition_version.to_string(),
                            name: eq.name.clone(),
                            parameters: params,
                            position: eq.position.map(|p| Position2dDto { x: p.x, y: p.y }),
                            display_width: eq.display_width,
                            display_height: eq.display_height,
                            rotation: eq.rotation,
                        });
                    }

                    for st in &unit.streams {
                        let mut comp = BTreeMap::new();
                        if let Some(c) = &st.composition {
                            for (k, v) in &c.components {
                                comp.insert(k.clone(), *v);
                            }
                        }
                        stream_dtos.push(StudioStreamDto {
                            id: st.id.to_string(),
                            name: st.name.clone(),
                            stream_type: format!("{:?}", st.stream_type),
                            source_equipment_id: st.source.equipment_id.to_string(),
                            source_port_id: st.source.port_id.to_string(),
                            sink_equipment_id: st.sink.equipment_id.to_string(),
                            sink_port_id: st.sink.port_id.to_string(),
                            composition: comp,
                            route_points: st
                                .route_points
                                .iter()
                                .map(|p| Position2dDto { x: p.x, y: p.y })
                                .collect(),
                            diameter: st.diameter,
                            length: st.length,
                            roughness: st.roughness,
                            elevation_change: st.elevation_change,
                            loss_k: st.loss_k,
                        });
                    }

                    for inst in &unit.instruments {
                        inst_dtos.push(StudioInstrumentDto {
                            id: inst.id.to_string(),
                            tag: inst.tag.clone(),
                            equipment_id: inst.equipment_id.map(|e| e.to_string()),
                            port_id: inst.port_id.map(|p| p.to_string()),
                            measurement_kind: inst.measurement_kind.clone(),
                            range_min: inst.range_min,
                            range_max: inst.range_max,
                            unit: inst.unit.clone(),
                            position: inst.position.map(|p| Position2dDto { x: p.x, y: p.y }),
                        });
                    }

                    for cl in &unit.control_loops {
                        loop_dtos.push(StudioControlLoopDto {
                            id: cl.id.to_string(),
                            name: cl.name.clone(),
                            pv_tag: cl.pv_tag.clone(),
                            sp: cl.sp,
                            out_tag: cl.out_tag.clone(),
                            kp: cl.kp,
                            ti: cl.ti,
                            td: cl.td,
                            reverse_acting: cl.reverse_acting,
                            mode: cl.mode.clone(),
                            output_min: cl.output_min,
                            output_max: cl.output_max,
                            position: cl.position.map(|p| Position2dDto { x: p.x, y: p.y }),
                        });
                    }

                    for alm in &unit.alarms {
                        alm_dtos.push(StudioAlarmDto {
                            id: alm.id.to_string(),
                            name: alm.name.clone(),
                            pv_tag: alm.pv_tag.clone(),
                            alarm_type: format!("{:?}", alm.alarm_type),
                            severity: format!("{:?}", alm.severity),
                            trip_limit: alm.trip_limit,
                            deadband: alm.deadband,
                            description: alm.description.clone(),
                        });
                    }

                    for ilk in &unit.interlocks {
                        ilk_dtos.push(StudioInterlockDto {
                            id: ilk.id.to_string(),
                            name: ilk.name.clone(),
                            source_tag: ilk.source_tag.clone(),
                            condition: ilk.condition.clone(),
                            threshold: ilk.threshold,
                            target_action: ilk.target_action.clone(),
                            target_equipment: ilk.target_equipment.map(|e| e.to_string()),
                            latched: ilk.latched,
                            description: ilk.description.clone(),
                        });
                    }

                    for bc in &unit.boundary_conditions {
                        let mut comp = BTreeMap::new();
                        for (k, v) in &bc.composition {
                            comp.insert(k.clone(), *v);
                        }
                        boundary_dtos.push(BoundaryConditionDto {
                            id: bc.id.to_string(),
                            name: bc.name.clone(),
                            condition_type: bc.condition_type.clone(),
                            target_equipment_id: bc.target_equipment_id.map(|e| e.to_string()),
                            target_port: bc.target_port_id.map(|p| p.to_string()),
                            target_stream_id: bc.target_stream_id.clone(),
                            flow_value: bc.flow_value,
                            flow_unit: bc.flow_unit.clone(),
                            pressure_value: bc.pressure_value,
                            pressure_unit: bc.pressure_unit.clone(),
                            temperature_value: bc.temperature_value,
                            temperature_unit: bc.temperature_unit.clone(),
                            phase: bc.phase.clone(),
                            density: bc.density,
                            molecular_weight: bc.molecular_weight,
                            enthalpy: bc.enthalpy,
                            quality: bc.quality,
                            composition: comp,
                            validity_min_flow: bc.validity_min_flow,
                            validity_max_flow: bc.validity_max_flow,
                            source_provenance: bc.source_provenance.clone(),
                            enabled: bc.enabled,
                        });
                    }

                    for ic in &unit.initial_conditions {
                        initial_dtos.push(InitialConditionDto {
                            id: ic.id.to_string(),
                            entity_id: ic.entity_id.clone(),
                            variable: ic.variable.clone(),
                            value: ic.value,
                            unit: ic.unit.clone(),
                            description: ic.description.clone(),
                        });
                    }
                }
            }
        }

        ProjectDetailDto {
            id: project.id.to_string(),
            name: project.name.clone(),
            schema_version: project.schema_version.to_string(),
            description: project.metadata.description.clone(),
            author: project.metadata.author.clone(),
            created: project.metadata.created.clone(),
            modified: chrono_now_iso(),
            equipment: equipment_dtos,
            streams: stream_dtos,
            instruments: inst_dtos,
            control_loops: loop_dtos,
            alarms: alm_dtos,
            interlocks: ilk_dtos,
            controllers: project
                .controllers
                .iter()
                .map(Self::controller_to_dto)
                .collect(),
            io_systems: project
                .io_systems
                .iter()
                .map(|s| Self::io_system_to_dto(s, &project.io_mappings))
                .collect(),
            io_mappings: project
                .io_mappings
                .iter()
                .map(Self::io_mapping_to_dto)
                .collect(),
            tags: project.tags.iter().map(Self::tag_to_dto).collect(),
            advanced_control: project
                .advanced_control
                .iter()
                .map(Self::advanced_control_to_dto)
                .collect(),
            communications: project
                .communications
                .as_ref()
                .map(|c| CommunicationsConfigDto {
                    connections: c
                        .connections
                        .iter()
                        .map(|conn| ProtocolConnectionDto {
                            id: conn.id.clone(),
                            name: conn.name.clone(),
                            protocol: conn.protocol.clone(),
                            endpoint: conn.endpoint.clone(),
                            enabled: conn.enabled,
                            state: "Disconnected".to_string(),
                            zone_id: conn.zone_id.clone(),
                            timeout_ms: conn.timeout_ms,
                            reconnect_delay_ms: conn.reconnect_delay_ms,
                            use_tls: conn.use_tls,
                            active_tags: 0,
                            last_comm_ms: 0,
                        })
                        .collect(),
                    tag_mappings: c
                        .tag_mappings
                        .iter()
                        .map(|tm| TagMappingDto {
                            id: tm.id.clone(),
                            connection_id: tm.connection_id.clone(),
                            external_address: tm.external_address.clone(),
                            canonical_tag: tm.canonical_tag.clone(),
                            direction: tm.direction.clone(),
                            datatype: tm.datatype.clone(),
                            scale: tm.scale,
                            offset: tm.offset,
                            engineering_unit: None,
                            deadband: tm.deadband,
                            update_rate_ms: tm.update_rate_ms,
                            stale_timeout_ms: tm.stale_timeout_ms,
                            access: tm.access.clone(),
                            current_value: None,
                            quality: "Good".to_string(),
                        })
                        .collect(),
                    zones: c
                        .zones
                        .iter()
                        .map(|z| IndustrialZoneDto {
                            id: z.id.clone(),
                            name: z.name.clone(),
                            level: z.level.clone(),
                            permitted_protocols: z.permitted_protocols.clone(),
                            trust_boundary: z.trust_boundary,
                            security_policy: z.security_policy.clone(),
                        })
                        .collect(),
                }),
            sis: project.sis.as_ref().map(|s| SisConfigDto {
                enabled: s.enabled,
                sifs: s
                    .sifs
                    .iter()
                    .map(|sif| SifConfigDto {
                        id: sif.id.clone(),
                        name: sif.name.clone(),
                        description: sif.description.clone(),
                        hazard_id: sif.hazard_id.clone(),
                        hazard_scenario: sif.hazard_scenario.clone(),
                        target_sil: sif.target_sil.clone(),
                        direction: sif.direction.clone(),
                        threshold: sif.threshold,
                        reset_deadband: sif.reset_deadband,
                        trip_delay_seconds: sif.trip_delay_seconds,
                        architecture: sif.architecture.clone(),
                        degradation_policy: sif.degradation_policy.clone(),
                        sensor_tags: sif.sensor_tags.clone(),
                        final_elements: sif
                            .final_elements
                            .iter()
                            .map(|fe| SifFinalElementConfigDto {
                                target_equipment: fe.target_equipment.clone(),
                                actuator_var: fe.actuator_var.clone(),
                                safe_state: fe.safe_state.clone(),
                                de_energize_to_trip: fe.de_energize_to_trip,
                                action_type: fe.action_type.clone(),
                                feedback_tag: fe.feedback_tag.clone(),
                            })
                            .collect(),
                        latched: sif.latched,
                        reset_permissives: sif.reset_permissives.clone(),
                        pfd_avg_estimate: sif.pfd_avg_estimate,
                    })
                    .collect(),
            }),
            plant_structure: project.plant_structure.as_ref().map(|ps| PlantStructureDto {
                plant_name: ps.plant_name.clone(),
                areas: ps.areas.iter().map(|a| PlantAreaDto {
                    id: a.id.clone(),
                    name: a.name.clone(),
                    description: a.description.clone(),
                    systems: a.systems.iter().map(|s| PlantSystemDto {
                        id: s.id.clone(),
                        name: s.name.clone(),
                        area_id: s.area_id.clone(),
                        default_display_id: s.default_display_id.clone(),
                        description: s.description.clone(),
                        subsystems: s.subsystems.iter().map(|sub| PlantSubsystemDto {
                            id: sub.id.clone(),
                            name: sub.name.clone(),
                            system_id: sub.system_id.clone(),
                            description: sub.description.clone(),
                            units: sub.units.iter().map(|u| PlantUnitDto {
                                id: u.id.clone(),
                                name: u.name.clone(),
                                subsystem_id: u.subsystem_id.clone(),
                                system_id: u.system_id.clone(),
                                area_id: u.area_id.clone(),
                                equipment_ids: u.equipment_ids.clone(),
                                display_ids: u.display_ids.clone(),
                                description: u.description.clone(),
                            }).collect(),
                        }).collect(),
                    }).collect(),
                }).collect(),
                displays: ps.displays.iter().map(|d| DisplayDto {
                    id: d.id.clone(),
                    name: d.name.clone(),
                    display_type: d.display_type.clone(),
                    area_id: d.area_id.clone(),
                    system_id: d.system_id.clone(),
                    subsystem_id: d.subsystem_id.clone(),
                    unit_id: d.unit_id.clone(),
                    equipment_ids: d.equipment_ids.clone(),
                    instrument_ids: d.instrument_ids.clone(),
                    stream_ids: d.stream_ids.clone(),
                    viewport: d.viewport_x.map(|x| ViewportConfigDto {
                        x,
                        y: d.viewport_y.unwrap_or(0.0),
                        width: d.viewport_width.unwrap_or(1920.0),
                        height: d.viewport_height.unwrap_or(1080.0),
                        zoom: d.viewport_zoom.unwrap_or(1.0),
                    }),
                    is_default: d.is_default,
                    order: d.order,
                    description: d.description.clone(),
                    placements: d.placements.iter().map(|p| DisplayElementPlacementDto {
                        equipment_id: p.equipment_id.clone(),
                        x: p.x,
                        y: p.y,
                        width: p.width,
                        height: p.height,
                        rotation: p.rotation,
                    }).collect(),
                }).collect(),
            }),
            areas: project.plant_structure.as_ref().map(|ps| {
                ps.areas.iter().map(|a| PlantAreaDto {
                    id: a.id.clone(),
                    name: a.name.clone(),
                    description: a.description.clone(),
                    systems: a.systems.iter().map(|s| PlantSystemDto {
                        id: s.id.clone(),
                        name: s.name.clone(),
                        area_id: s.area_id.clone(),
                        default_display_id: s.default_display_id.clone(),
                        description: s.description.clone(),
                        subsystems: s.subsystems.iter().map(|sub| PlantSubsystemDto {
                            id: sub.id.clone(),
                            name: sub.name.clone(),
                            system_id: sub.system_id.clone(),
                            description: sub.description.clone(),
                            units: sub.units.iter().map(|u| PlantUnitDto {
                                id: u.id.clone(),
                                name: u.name.clone(),
                                subsystem_id: u.subsystem_id.clone(),
                                system_id: u.system_id.clone(),
                                area_id: u.area_id.clone(),
                                equipment_ids: u.equipment_ids.clone(),
                                display_ids: u.display_ids.clone(),
                                description: u.description.clone(),
                            }).collect(),
                        }).collect(),
                    }).collect(),
                }).collect()
            }).unwrap_or_default(),
            displays: if !project.displays.is_empty() {
                project.displays.iter().map(|d| DisplayDto {
                    id: d.id.clone(),
                    name: d.name.clone(),
                    display_type: d.display_type.clone(),
                    area_id: d.area_id.clone(),
                    system_id: d.system_id.clone(),
                    subsystem_id: d.subsystem_id.clone(),
                    unit_id: d.unit_id.clone(),
                    equipment_ids: d.equipment_ids.clone(),
                    instrument_ids: d.instrument_ids.clone(),
                    stream_ids: d.stream_ids.clone(),
                    viewport: d.viewport_x.map(|x| ViewportConfigDto {
                        x,
                        y: d.viewport_y.unwrap_or(0.0),
                        width: d.viewport_width.unwrap_or(1920.0),
                        height: d.viewport_height.unwrap_or(1080.0),
                        zoom: d.viewport_zoom.unwrap_or(1.0),
                    }),
                    is_default: d.is_default,
                    order: d.order,
                    description: d.description.clone(),
                    placements: d.placements.iter().map(|p| DisplayElementPlacementDto {
                        equipment_id: p.equipment_id.clone(),
                        x: p.x,
                        y: p.y,
                        width: p.width,
                        height: p.height,
                        rotation: p.rotation,
                    }).collect(),
                }).collect()
            } else if let Some(ref ps) = project.plant_structure {
                ps.displays.iter().map(|d| DisplayDto {
                    id: d.id.clone(),
                    name: d.name.clone(),
                    display_type: d.display_type.clone(),
                    area_id: d.area_id.clone(),
                    system_id: d.system_id.clone(),
                    subsystem_id: d.subsystem_id.clone(),
                    unit_id: d.unit_id.clone(),
                    equipment_ids: d.equipment_ids.clone(),
                    instrument_ids: d.instrument_ids.clone(),
                    stream_ids: d.stream_ids.clone(),
                    viewport: d.viewport_x.map(|x| ViewportConfigDto {
                        x,
                        y: d.viewport_y.unwrap_or(0.0),
                        width: d.viewport_width.unwrap_or(1920.0),
                        height: d.viewport_height.unwrap_or(1080.0),
                        zoom: d.viewport_zoom.unwrap_or(1.0),
                    }),
                    is_default: d.is_default,
                    order: d.order,
                    description: d.description.clone(),
                    placements: d.placements.iter().map(|p| DisplayElementPlacementDto {
                        equipment_id: p.equipment_id.clone(),
                        x: p.x,
                        y: p.y,
                        width: p.width,
                        height: p.height,
                        rotation: p.rotation,
                    }).collect(),
                }).collect()
            } else {
                Vec::new()
            },
            navigation_links: project.navigation_links.iter().map(|nl| NavigationLinkDto {
                id: nl.id.clone(),
                source_display_id: nl.source_display_id.clone(),
                source_element_id: nl.source_element_id.clone(),
                target_display_id: nl.target_display_id.clone(),
                target_element_id: nl.target_element_id.clone(),
                label: nl.label.clone(),
                behavior: nl.behavior.clone(),
                description: nl.description.clone(),
            }).collect(),
            boundary_conditions: boundary_dtos,
            initial_conditions: initial_dtos,
        }
    }

    /// Converts transport ProjectDetailDto back into domain Project.
    pub fn dto_to_project(dto: &ProjectDetailDto) -> Result<Project, String> {
        let proj_id = ProjectId::from_str(&dto.id).unwrap_or_else(|_| ProjectId::new());
        let schema_ver = dto
            .schema_version
            .parse::<SchemaVersion>()
            .unwrap_or(SchemaVersion::CURRENT);

        let mut project = Project::new(&dto.name);
        project.id = proj_id;
        project.schema_version = schema_ver;
        project.metadata = ProjectMetadata {
            description: dto.description.clone(),
            author: dto.author.clone(),
            created: dto.created.clone(),
            modified: chrono_now_iso(),
        };

        let mut plant = Plant::new(format!("Plant ({})", dto.name));
        let mut area = Area::new("Main Area");
        let mut unit = Unit::new("Main Unit");

        // Convert equipment
        for eq_dto in &dto.equipment {
            let eq_id = EquipmentId::from_str(&eq_dto.id)
                .map_err(|e| format!("Invalid equipment ID '{}': {e}", eq_dto.id))?;
            let def_id = EquipmentDefinitionId::from_str(&eq_dto.definition_id)
                .unwrap_or_else(|_| EquipmentDefinitionId::new());
            let def_ver = eq_dto
                .definition_version
                .parse::<SchemaVersion>()
                .unwrap_or(SchemaVersion::CURRENT);

            let mut params = IndexMap::new();
            for (k, v) in &eq_dto.parameters {
                params.insert(k.clone(), *v);
            }

            unit.equipment.push(EquipmentInstance {
                id: eq_id,
                definition_id: def_id,
                definition_version: def_ver,
                name: eq_dto.name.clone(),
                parameters: params,
                position: eq_dto
                    .position
                    .as_ref()
                    .map(|p| Position2D { x: p.x, y: p.y }),
                display_width: eq_dto.display_width,
                display_height: eq_dto.display_height,
                rotation: eq_dto.rotation,
            });
        }

        // Convert streams
        for st_dto in &dto.streams {
            let st_id = StreamId::from_str(&st_dto.id).unwrap_or_else(|_| StreamId::new());
            let src_eq = EquipmentId::from_str(&st_dto.source_equipment_id)
                .map_err(|e| format!("Invalid stream source ID: {e}"))?;
            let src_port = PortId::from_str(&st_dto.source_port_id)
                .map_err(|e| format!("Invalid stream source port ID: {e}"))?;
            let snk_eq = EquipmentId::from_str(&st_dto.sink_equipment_id)
                .map_err(|e| format!("Invalid stream sink ID: {e}"))?;
            let snk_port = PortId::from_str(&st_dto.sink_port_id)
                .map_err(|e| format!("Invalid stream sink port ID: {e}"))?;

            let mut comp = None;
            if !st_dto.composition.is_empty() {
                let mut map = IndexMap::new();
                for (k, v) in &st_dto.composition {
                    map.insert(k.clone(), *v);
                }
                comp = Some(StreamComposition { components: map });
            }

            let stream_type = match st_dto.stream_type.to_lowercase().as_str() {
                "mechanical" | "mechanicalshaft" | "mechanical_shaft" => {
                    StreamType::MechanicalShaft
                }
                "utility_hydrogen" | "utilityhydrogen" => StreamType::UtilityHydrogen,
                "utility_steam_hp" | "utilitysteamhp" => StreamType::UtilitySteamHp,
                "utility_steam_mp" | "utilitysteammp" => StreamType::UtilitySteamMp,
                "utility_steam_lp" | "utilitysteamlp" => StreamType::UtilitySteamLp,
                "utility_cooling_water" | "utilitycoolingwater" => StreamType::UtilityCoolingWater,
                "utility_instrument_air" | "utilityinstrumentair" => {
                    StreamType::UtilityInstrumentAir
                }
                "utility_plant_air" | "utilityplantair" => StreamType::UtilityPlantAir,
                "utility_nitrogen" | "utilitynitrogen" => StreamType::UtilityNitrogen,
                "utility_fuel_gas" | "utilityfuelgas" => StreamType::UtilityFuelGas,
                "utility_electrical" | "utilityelectrical" => StreamType::UtilityElectrical,
                "utility_flare" | "utilityflare" => StreamType::UtilityFlare,
                "catalyst_transport" | "catalysttransport" => StreamType::CatalystTransport,
                _ => StreamType::Material,
            };

            unit.streams.push(StreamDefinition {
                id: st_id,
                name: st_dto.name.clone(),
                stream_type,
                source: PortRef {
                    equipment_id: src_eq,
                    port_id: src_port,
                },
                sink: PortRef {
                    equipment_id: snk_eq,
                    port_id: snk_port,
                },
                composition: comp,
                route_points: st_dto
                    .route_points
                    .iter()
                    .map(|p| Position2D { x: p.x, y: p.y })
                    .collect(),
                diameter: st_dto.diameter,
                length: st_dto.length,
                roughness: st_dto.roughness,
                elevation_change: st_dto.elevation_change,
                loss_k: st_dto.loss_k,
            });
        }

        // Convert instruments
        for inst_dto in &dto.instruments {
            let id = EntityId::from_str(&inst_dto.id).unwrap_or_else(|_| EntityId::new());
            let eq_id = inst_dto
                .equipment_id
                .as_deref()
                .and_then(|s| EquipmentId::from_str(s).ok());
            let port_id = inst_dto
                .port_id
                .as_deref()
                .and_then(|s| PortId::from_str(s).ok());

            unit.instruments.push(InstrumentInstance {
                id,
                tag: inst_dto.tag.clone(),
                equipment_id: eq_id,
                port_id,
                measurement_kind: inst_dto.measurement_kind.clone(),
                range_min: inst_dto.range_min,
                range_max: inst_dto.range_max,
                unit: inst_dto.unit.clone(),
                position: inst_dto
                    .position
                    .as_ref()
                    .map(|p| Position2D { x: p.x, y: p.y }),
            });
        }

        // Convert control loops
        for cl_dto in &dto.control_loops {
            let id = EntityId::from_str(&cl_dto.id).unwrap_or_else(|_| EntityId::new());

            unit.control_loops.push(ControlLoopInstance {
                id,
                name: cl_dto.name.clone(),
                pv_tag: cl_dto.pv_tag.clone(),
                sp: cl_dto.sp,
                out_tag: cl_dto.out_tag.clone(),
                kp: cl_dto.kp,
                ti: cl_dto.ti,
                td: cl_dto.td,
                reverse_acting: cl_dto.reverse_acting,
                mode: cl_dto.mode.clone(),
                output_min: cl_dto.output_min,
                output_max: cl_dto.output_max,
                position: cl_dto
                    .position
                    .as_ref()
                    .map(|p| Position2D { x: p.x, y: p.y }),
            });
        }

        // Convert alarms
        for alm_dto in &dto.alarms {
            let id = EntityId::from_str(&alm_dto.id).unwrap_or_else(|_| EntityId::new());

            unit.alarms.push(AlarmInstance {
                id,
                name: alm_dto.name.clone(),
                pv_tag: alm_dto.pv_tag.clone(),
                alarm_type: AlarmType::Hi,
                severity: AlarmPriority::High,
                trip_limit: alm_dto.trip_limit,
                deadband: alm_dto.deadband,
                description: alm_dto.description.clone(),
            });
        }

        // Convert interlocks
        for ilk_dto in &dto.interlocks {
            let id = EntityId::from_str(&ilk_dto.id).unwrap_or_else(|_| EntityId::new());
            let tgt_eq = ilk_dto
                .target_equipment
                .as_deref()
                .and_then(|s| EquipmentId::from_str(s).ok());

            unit.interlocks.push(InterlockInstance {
                id,
                name: ilk_dto.name.clone(),
                source_tag: ilk_dto.source_tag.clone(),
                condition: ilk_dto.condition.clone(),
                threshold: ilk_dto.threshold,
                target_action: ilk_dto.target_action.clone(),
                target_equipment: tgt_eq,
                latched: ilk_dto.latched,
                description: ilk_dto.description.clone(),
            });
        }

        // Convert boundary conditions
        for bc_dto in &dto.boundary_conditions {
            let bc_id = EntityId::from_str(&bc_dto.id).unwrap_or_else(|_| EntityId::new());
            let target_eq = bc_dto
                .target_equipment_id
                .as_deref()
                .and_then(|s| EquipmentId::from_str(s).ok());
            let target_port = bc_dto
                .target_port
                .as_deref()
                .and_then(|s| PortId::from_str(s).ok());

            let mut comp = IndexMap::new();
            for (k, v) in &bc_dto.composition {
                comp.insert(k.clone(), *v);
            }

            unit.boundary_conditions.push(BoundaryConditionInstance {
                id: bc_id,
                name: bc_dto.name.clone(),
                condition_type: bc_dto.condition_type.clone(),
                target_equipment_id: target_eq,
                target_port_id: target_port,
                target_stream_id: bc_dto.target_stream_id.clone(),
                flow_value: bc_dto.flow_value,
                flow_unit: bc_dto.flow_unit.clone(),
                pressure_value: bc_dto.pressure_value,
                pressure_unit: bc_dto.pressure_unit.clone(),
                temperature_value: bc_dto.temperature_value,
                temperature_unit: bc_dto.temperature_unit.clone(),
                phase: bc_dto.phase.clone(),
                density: bc_dto.density,
                molecular_weight: bc_dto.molecular_weight,
                enthalpy: bc_dto.enthalpy,
                quality: bc_dto.quality,
                composition: comp,
                validity_min_flow: bc_dto.validity_min_flow,
                validity_max_flow: bc_dto.validity_max_flow,
                source_provenance: bc_dto.source_provenance.clone(),
                enabled: bc_dto.enabled,
            });
        }

        // Convert initial conditions
        for ic_dto in &dto.initial_conditions {
            let ic_id = EntityId::from_str(&ic_dto.id).unwrap_or_else(|_| EntityId::new());
            unit.initial_conditions.push(InitialConditionInstance {
                id: ic_id,
                entity_id: ic_dto.entity_id.clone(),
                variable: ic_dto.variable.clone(),
                value: ic_dto.value,
                unit: ic_dto.unit.clone(),
                description: ic_dto.description.clone(),
            });
        }

        area.units.push(unit);
        plant.areas.push(area);
        project.plants.push(plant);

        // Convert controllers
        project.controllers = dto
            .controllers
            .iter()
            .map(Self::dto_to_controller)
            .collect();

        // Convert io_systems
        project.io_systems = dto.io_systems.iter().map(Self::dto_to_io_system).collect();

        // Convert io_mappings
        project.io_mappings = dto
            .io_mappings
            .iter()
            .map(Self::dto_to_io_mapping)
            .collect();

        // Convert tags
        project.tags = dto.tags.iter().map(Self::dto_to_tag).collect();

        // Convert advanced_control
        project.advanced_control = dto
            .advanced_control
            .iter()
            .map(Self::dto_to_advanced_control)
            .collect();

        // Convert communications
        project.communications = dto.communications.as_ref().map(|c| ProjectCommunications {
            connections: c
                .connections
                .iter()
                .map(|conn| ProjectProtocolConnection {
                    id: conn.id.clone(),
                    name: conn.name.clone(),
                    protocol: conn.protocol.clone(),
                    endpoint: conn.endpoint.clone(),
                    enabled: conn.enabled,
                    zone_id: conn.zone_id.clone(),
                    timeout_ms: conn.timeout_ms,
                    reconnect_delay_ms: conn.reconnect_delay_ms,
                    use_tls: conn.use_tls,
                })
                .collect(),
            tag_mappings: c
                .tag_mappings
                .iter()
                .map(|tm| ProjectTagMapping {
                    id: tm.id.clone(),
                    connection_id: tm.connection_id.clone(),
                    external_address: tm.external_address.clone(),
                    canonical_tag: tm.canonical_tag.clone(),
                    direction: tm.direction.clone(),
                    datatype: tm.datatype.clone(),
                    scale: tm.scale,
                    offset: tm.offset,
                    deadband: tm.deadband,
                    update_rate_ms: tm.update_rate_ms,
                    stale_timeout_ms: tm.stale_timeout_ms,
                    access: tm.access.clone(),
                })
                .collect(),
            zones: c
                .zones
                .iter()
                .map(|z| ProjectIndustrialZone {
                    id: z.id.clone(),
                    name: z.name.clone(),
                    level: z.level.clone(),
                    permitted_protocols: z.permitted_protocols.clone(),
                    trust_boundary: z.trust_boundary,
                    security_policy: z.security_policy.clone(),
                })
                .collect(),
        });

        // Convert sis
        project.sis = dto.sis.as_ref().map(|s| ProjectSisConfig {
            enabled: s.enabled,
            sifs: s
                .sifs
                .iter()
                .map(|sif| ProjectSifDefinition {
                    id: sif.id.clone(),
                    name: sif.name.clone(),
                    description: sif.description.clone(),
                    hazard_id: sif.hazard_id.clone(),
                    hazard_scenario: sif.hazard_scenario.clone(),
                    target_sil: sif.target_sil.clone(),
                    direction: sif.direction.clone(),
                    threshold: sif.threshold,
                    reset_deadband: sif.reset_deadband,
                    trip_delay_seconds: sif.trip_delay_seconds,
                    architecture: sif.architecture.clone(),
                    degradation_policy: sif.degradation_policy.clone(),
                    sensor_tags: sif.sensor_tags.clone(),
                    final_elements: sif
                        .final_elements
                        .iter()
                        .map(|fe| ProjectSifFinalElement {
                            target_equipment: fe.target_equipment.clone(),
                            actuator_var: fe.actuator_var.clone(),
                            safe_state: fe.safe_state.clone(),
                            de_energize_to_trip: fe.de_energize_to_trip,
                            action_type: fe.action_type.clone(),
                            feedback_tag: fe.feedback_tag.clone(),
                        })
                        .collect(),
                    latched: sif.latched,
                    reset_permissives: sif.reset_permissives.clone(),
                    pfd_avg_estimate: sif.pfd_avg_estimate,
                })
                .collect(),
        });

        // Convert displays
        let mut displays = dto.displays.clone();
        if displays.is_empty() {
            if let Some(ref ps) = dto.plant_structure {
                displays = ps.displays.clone();
            }
        }
        project.displays = displays
            .iter()
            .map(|d| ProjectDisplay {
                id: d.id.clone(),
                name: d.name.clone(),
                display_type: d.display_type.clone(),
                area_id: d.area_id.clone(),
                system_id: d.system_id.clone(),
                subsystem_id: d.subsystem_id.clone(),
                unit_id: d.unit_id.clone(),
                equipment_ids: d.equipment_ids.clone(),
                instrument_ids: d.instrument_ids.clone(),
                stream_ids: d.stream_ids.clone(),
                viewport_x: d.viewport.as_ref().map(|v| v.x),
                viewport_y: d.viewport.as_ref().map(|v| v.y),
                viewport_width: d.viewport.as_ref().map(|v| v.width),
                viewport_height: d.viewport.as_ref().map(|v| v.height),
                viewport_zoom: d.viewport.as_ref().map(|v| v.zoom),
                is_default: d.is_default,
                order: d.order,
                description: d.description.clone(),
                placements: d.placements.iter().map(|p| DisplayElementPlacement {
                    equipment_id: p.equipment_id.clone(),
                    x: p.x,
                    y: p.y,
                    width: p.width,
                    height: p.height,
                    rotation: p.rotation,
                }).collect(),
            })
            .collect();

        // Convert navigation links
        project.navigation_links = dto
            .navigation_links
            .iter()
            .map(|nl| ProjectNavigationLink {
                id: nl.id.clone(),
                source_display_id: nl.source_display_id.clone(),
                source_element_id: nl.source_element_id.clone(),
                target_display_id: nl.target_display_id.clone(),
                target_element_id: nl.target_element_id.clone(),
                label: nl.label.clone(),
                behavior: nl.behavior.clone(),
                description: nl.description.clone(),
            })
            .collect();

        // Convert plant structure
        let mut plant_struct = dto.plant_structure.clone();
        if plant_struct.is_none() && !dto.areas.is_empty() {
            plant_struct = Some(PlantStructureDto {
                plant_name: dto.name.clone(),
                areas: dto.areas.clone(),
                displays: dto.displays.clone(),
            });
        }
        project.plant_structure = plant_struct.map(|ps| ProjectPlantStructure {
            plant_name: ps.plant_name,
            areas: ps
                .areas
                .iter()
                .map(|a| ProjectPlantArea {
                    id: a.id.clone(),
                    name: a.name.clone(),
                    description: a.description.clone(),
                    systems: a
                        .systems
                        .iter()
                        .map(|s| ProjectPlantSystem {
                            id: s.id.clone(),
                            name: s.name.clone(),
                            area_id: s.area_id.clone(),
                            default_display_id: s.default_display_id.clone(),
                            description: s.description.clone(),
                            subsystems: s
                                .subsystems
                                .iter()
                                .map(|sub| ProjectPlantSubsystem {
                                    id: sub.id.clone(),
                                    name: sub.name.clone(),
                                    system_id: sub.system_id.clone(),
                                    description: sub.description.clone(),
                                    units: sub
                                        .units
                                        .iter()
                                        .map(|u| ProjectPlantUnit {
                                            id: u.id.clone(),
                                            name: u.name.clone(),
                                            subsystem_id: u.subsystem_id.clone(),
                                            system_id: u.system_id.clone(),
                                            area_id: u.area_id.clone(),
                                            equipment_ids: u.equipment_ids.clone(),
                                            display_ids: u.display_ids.clone(),
                                            description: u.description.clone(),
                                        })
                                        .collect(),
                                })
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
            displays: project.displays.clone(),
        });

        Ok(project)
    }

    pub fn controller_to_dto(c: &ProjectController) -> ControllerConfigDto {
        ControllerConfigDto {
            id: c.id.clone(),
            name: c.name.clone(),
            description: c.description.clone(),
            watchdog_timeout_seconds: c.watchdog_timeout_seconds,
            enabled: c.enabled,
            tasks: c
                .tasks
                .iter()
                .map(|t| TaskConfigDto {
                    id: t.id.clone(),
                    name: t.name.clone(),
                    task_type: t.task_type.clone(),
                    period_seconds: t.period_seconds,
                    priority: t.priority,
                    watchdog_seconds: t.watchdog_seconds,
                    enabled: t.enabled,
                    programs: t
                        .programs
                        .iter()
                        .map(|p| ProgramConfigDto {
                            id: p.id.clone(),
                            name: p.name.clone(),
                            description: p.description.clone(),
                            program_type: p.program_type.clone(),
                            routines: p
                                .routines
                                .iter()
                                .map(|r| IecRoutineDto {
                                    name: r.name.clone(),
                                    language: r.language.clone(),
                                    source_text: r.source_text.clone(),
                                    enabled: r.enabled,
                                    statement_count: 0,
                                    symbol_count: 0,
                                })
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
            cpu_profile: c.cpu_profile.as_ref().map(|cpu| CpuProfileDto {
                execution_budget_us: cpu.execution_budget_us,
                max_scan_frequency_hz: cpu.max_scan_frequency_hz,
                utilization_threshold_pct: cpu.utilization_threshold_pct,
                architecture: cpu.architecture.clone(),
            }),
            memory_profile: c.memory_profile.as_ref().map(|mem| MemoryProfileDto {
                program_memory_bytes: mem.program_memory_bytes,
                data_memory_bytes: mem.data_memory_bytes,
                retentive_memory_bytes: mem.retentive_memory_bytes,
                program_memory_used_bytes: 0,
                data_memory_used_bytes: 0,
                retentive_memory_used_bytes: 0,
            }),
            firmware_profile: c.firmware_profile.as_ref().map(|fw| FirmwareProfileDto {
                version: fw.version.clone(),
                build: fw.build.clone(),
                iec_standard: fw.iec_standard.clone(),
            }),
            redundancy: c.redundancy.as_ref().map(|red| RedundancyConfigDto {
                role: red.role.clone(),
                partner_controller_id: red.partner_controller_id.clone(),
                sync_frequency_hz: red.sync_frequency_hz,
                switchover_timeout_seconds: red.switchover_timeout_seconds,
            }),
            control_modules: c
                .control_modules
                .iter()
                .map(|cm| DcsControlModuleDto {
                    id: cm.id.clone(),
                    name: cm.name.clone(),
                    description: cm.description.clone(),
                    mode: cm.mode.clone(),
                    execution_rate_seconds: cm.execution_rate_seconds,
                    pv_signal: cm.pv_signal.clone(),
                    sp: cm.sp,
                    out_signal: cm.out_signal.clone(),
                    strategy_type: cm.strategy_type.clone(),
                    kp: cm.kp,
                    ti: cm.ti,
                    td: cm.td,
                    output_min: cm.output_min,
                    output_max: cm.output_max,
                    permissives: cm.permissives.clone(),
                    interlocks: cm.interlocks.clone(),
                    safe_position: cm.safe_position,
                    current_pv: 0.0,
                    current_out: 0.0,
                    is_interlocked: false,
                    permissives_ok: true,
                    tracking_active: false,
                })
                .collect(),
        }
    }

    pub fn dto_to_controller(dto: &ControllerConfigDto) -> ProjectController {
        ProjectController {
            id: dto.id.clone(),
            name: dto.name.clone(),
            description: dto.description.clone(),
            watchdog_timeout_seconds: dto.watchdog_timeout_seconds,
            enabled: dto.enabled,
            tasks: dto
                .tasks
                .iter()
                .map(|t| ProjectTask {
                    id: t.id.clone(),
                    name: t.name.clone(),
                    task_type: t.task_type.clone(),
                    period_seconds: t.period_seconds,
                    priority: t.priority,
                    watchdog_seconds: t.watchdog_seconds,
                    enabled: t.enabled,
                    programs: t
                        .programs
                        .iter()
                        .map(|p| ProjectProgram {
                            id: p.id.clone(),
                            name: p.name.clone(),
                            description: p.description.clone(),
                            program_type: p.program_type.clone(),
                            routines: p
                                .routines
                                .iter()
                                .map(|r| ProjectRoutine {
                                    id: r.name.clone(),
                                    name: r.name.clone(),
                                    language: r.language.clone(),
                                    source_text: r.source_text.clone(),
                                    enabled: r.enabled,
                                })
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
            cpu_profile: dto.cpu_profile.as_ref().map(|cpu| ProjectCpuProfile {
                execution_budget_us: cpu.execution_budget_us,
                max_scan_frequency_hz: cpu.max_scan_frequency_hz,
                utilization_threshold_pct: cpu.utilization_threshold_pct,
                architecture: cpu.architecture.clone(),
            }),
            memory_profile: dto.memory_profile.as_ref().map(|mem| ProjectMemoryProfile {
                program_memory_bytes: mem.program_memory_bytes,
                data_memory_bytes: mem.data_memory_bytes,
                retentive_memory_bytes: mem.retentive_memory_bytes,
            }),
            firmware_profile: dto
                .firmware_profile
                .as_ref()
                .map(|fw| ProjectFirmwareProfile {
                    version: fw.version.clone(),
                    build: fw.build.clone(),
                    iec_standard: fw.iec_standard.clone(),
                }),
            redundancy: dto.redundancy.as_ref().map(|red| ProjectRedundancyConfig {
                role: red.role.clone(),
                partner_controller_id: red.partner_controller_id.clone(),
                sync_frequency_hz: red.sync_frequency_hz,
                switchover_timeout_seconds: red.switchover_timeout_seconds,
            }),
            control_modules: dto
                .control_modules
                .iter()
                .map(|cm| ProjectControlModule {
                    id: cm.id.clone(),
                    name: cm.name.clone(),
                    description: cm.description.clone(),
                    mode: cm.mode.clone(),
                    execution_rate_seconds: cm.execution_rate_seconds,
                    pv_signal: cm.pv_signal.clone(),
                    sp: cm.sp,
                    out_signal: cm.out_signal.clone(),
                    strategy_type: cm.strategy_type.clone(),
                    kp: cm.kp,
                    ti: cm.ti,
                    td: cm.td,
                    output_min: cm.output_min,
                    output_max: cm.output_max,
                    permissives: cm.permissives.clone(),
                    interlocks: cm.interlocks.clone(),
                    safe_position: cm.safe_position,
                })
                .collect(),
        }
    }

    pub fn io_system_to_dto(
        s: &ProjectIoSystem,
        mappings: &[ProjectIoMapping],
    ) -> HalSystemSummaryDto {
        let mut total_modules = 0;
        let mut total_channels = 0;

        let racks: Vec<HalRackDto> = s
            .racks
            .iter()
            .map(|r| HalRackDto {
                rack_number: r.rack_number,
                name: r.name.clone(),
                power_ok: r.power_ok,
                chassis: r
                    .chassis
                    .iter()
                    .map(|c| HalChassisDto {
                        chassis_number: c.chassis_number,
                        name: c.name.clone(),
                        bus_ok: c.bus_ok,
                        modules: c
                            .modules
                            .iter()
                            .map(|m| {
                                total_modules += 1;
                                total_channels += m.channels.len();
                                HalModuleDto {
                                    slot: m.slot,
                                    name: m.name.clone(),
                                    module_type: m.module_type.clone(),
                                    status: "Healthy".to_string(),
                                    channels: m
                                        .channels
                                        .iter()
                                        .map(|ch| HalChannelDto {
                                            id: format!("ch-{}", ch.channel_number),
                                            channel_number: ch.channel_number,
                                            address: ch.address.clone(),
                                            name: ch.name.clone(),
                                            kind: ch.kind.clone(),
                                            value: ch.forced_value.unwrap_or(0.0),
                                            raw_counts: 0,
                                            quality: "Good".to_string(),
                                            fault_state: "Normal".to_string(),
                                            status: "Healthy".to_string(),
                                            engineering_unit: ch.engineering_unit.clone(),
                                            lrv: ch.lrv,
                                            urv: ch.urv,
                                            fallback_behavior: ch.fallback_behavior.clone(),
                                            forced_value: ch.forced_value,
                                        })
                                        .collect(),
                                }
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect();

        HalSystemSummaryDto {
            id: s.id.clone(),
            name: s.name.clone(),
            rack_count: s.racks.len(),
            module_count: total_modules,
            channel_count: total_channels,
            mapping_count: mappings.len(),
            diagnostics: HalDiagnosticsDto {
                channels_scanned: total_channels as u64,
                faulted_channels: 0,
                last_scan_duration_us: 0,
                max_scan_duration_us: 0,
                avg_scan_duration_us: 0.0,
            },
            racks,
            mappings: mappings.iter().map(Self::io_mapping_to_dto).collect(),
        }
    }

    pub fn dto_to_io_system(dto: &HalSystemSummaryDto) -> ProjectIoSystem {
        ProjectIoSystem {
            id: dto.id.clone(),
            name: dto.name.clone(),
            racks: dto
                .racks
                .iter()
                .map(|r| ProjectIoRack {
                    rack_number: r.rack_number,
                    name: r.name.clone(),
                    power_ok: r.power_ok,
                    chassis: r
                        .chassis
                        .iter()
                        .map(|c| ProjectIoChassis {
                            chassis_number: c.chassis_number,
                            name: c.name.clone(),
                            bus_ok: c.bus_ok,
                            modules: c
                                .modules
                                .iter()
                                .map(|m| ProjectIoModule {
                                    slot: m.slot,
                                    name: m.name.clone(),
                                    module_type: m.module_type.clone(),
                                    channels: m
                                        .channels
                                        .iter()
                                        .map(|ch| ProjectIoChannel {
                                            channel_number: ch.channel_number,
                                            address: ch.address.clone(),
                                            name: ch.name.clone(),
                                            kind: ch.kind.clone(),
                                            engineering_unit: ch.engineering_unit.clone(),
                                            lrv: ch.lrv,
                                            urv: ch.urv,
                                            fallback_behavior: ch.fallback_behavior.clone(),
                                            forced_value: ch.forced_value,
                                        })
                                        .collect(),
                                })
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    pub fn io_mapping_to_dto(m: &ProjectIoMapping) -> HalMappingDto {
        HalMappingDto {
            controller_id: m.controller_id.clone(),
            signal_key: m.signal_key.clone(),
            channel_address: m.channel_address.clone(),
            direction: m.direction.clone(),
            target_tag: m.target_tag.clone(),
            target_actuator: m.target_actuator.clone(),
            priority: m.priority,
        }
    }

    pub fn dto_to_io_mapping(dto: &HalMappingDto) -> ProjectIoMapping {
        ProjectIoMapping {
            controller_id: dto.controller_id.clone(),
            signal_key: dto.signal_key.clone(),
            channel_address: dto.channel_address.clone(),
            direction: dto.direction.clone(),
            target_tag: dto.target_tag.clone(),
            target_actuator: dto.target_actuator.clone(),
            priority: dto.priority,
        }
    }

    pub fn tag_to_dto(t: &ProjectTag) -> ProjectTagDto {
        ProjectTagDto {
            id: t.id.clone(),
            name: t.name.clone(),
            description: t.description.clone(),
            data_type: t.data_type.clone(),
            engineering_unit: t.engineering_unit.clone(),
            source: t.source.clone(),
            direction: t.direction.clone(),
            controller_id: t.controller_id.clone(),
            channel_address: t.channel_address.clone(),
            process_entity: t.process_entity.clone(),
            alarm_id: t.alarm_id.clone(),
        }
    }

    pub fn dto_to_tag(dto: &ProjectTagDto) -> ProjectTag {
        ProjectTag {
            id: dto.id.clone(),
            name: dto.name.clone(),
            description: dto.description.clone(),
            data_type: dto.data_type.clone(),
            engineering_unit: dto.engineering_unit.clone(),
            source: dto.source.clone(),
            direction: dto.direction.clone(),
            controller_id: dto.controller_id.clone(),
            channel_address: dto.channel_address.clone(),
            process_entity: dto.process_entity.clone(),
            alarm_id: dto.alarm_id.clone(),
        }
    }

    pub fn advanced_control_to_dto(
        s: &ProjectAdvancedControlStrategy,
    ) -> AdvancedControlStrategyDto {
        AdvancedControlStrategyDto {
            id: s.id.clone(),
            name: s.name.clone(),
            description: s.description.clone(),
            enabled: s.enabled,
            strategy_type: s.strategy_type.clone(),
            parameters: s.parameters.clone(),
        }
    }

    pub fn dto_to_advanced_control(
        dto: &AdvancedControlStrategyDto,
    ) -> ProjectAdvancedControlStrategy {
        ProjectAdvancedControlStrategy {
            id: dto.id.clone(),
            name: dto.name.clone(),
            description: dto.description.clone(),
            enabled: dto.enabled,
            strategy_type: dto.strategy_type.clone(),
            parameters: dto.parameters.clone(),
        }
    }

    /// Returns advanced control strategies for a project.
    pub fn get_advanced_control(&self, id: &str) -> Option<Vec<AdvancedControlStrategyDto>> {
        self.projects.get(id).map(|p| {
            p.advanced_control
                .iter()
                .map(Self::advanced_control_to_dto)
                .collect()
        })
    }

    /// Adds or updates an advanced control strategy in a project.
    pub fn save_advanced_control(
        &mut self,
        id: &str,
        strategy: AdvancedControlStrategyDto,
    ) -> Result<AdvancedControlStrategyDto, String> {
        let project = self
            .projects
            .get_mut(id)
            .ok_or_else(|| format!("Project '{id}' not found"))?;

        let dom = Self::dto_to_advanced_control(&strategy);
        if let Some(pos) = project
            .advanced_control
            .iter()
            .position(|s| s.id == strategy.id)
        {
            project.advanced_control[pos] = dom;
        } else {
            project.advanced_control.push(dom);
        }

        if let Some(ref store) = self.disk_store {
            let _ = store.save_json("projects", id, project);
        }

        Ok(strategy)
    }

    /// Deletes an advanced control strategy from a project.
    pub fn delete_advanced_control(&mut self, id: &str, strategy_id: &str) -> Result<bool, String> {
        let project = self
            .projects
            .get_mut(id)
            .ok_or_else(|| format!("Project '{id}' not found"))?;

        let initial_len = project.advanced_control.len();
        project.advanced_control.retain(|s| s.id != strategy_id);
        let removed = project.advanced_control.len() < initial_len;

        if removed {
            if let Some(ref store) = self.disk_store {
                let _ = store.save_json("projects", id, project);
            }
        }

        Ok(removed)
    }

    fn validation_summary_to_dto(
        summary: &ProjectValidationSummary,
    ) -> ProjectValidationResponseDto {
        ProjectValidationResponseDto {
            valid: summary.is_valid,
            error_count: summary.error_count,
            warning_count: summary.warning_count,
            info_count: summary.info_count,
            review_count: summary.review_count,
            results: summary
                .results
                .iter()
                .map(|r| ValidationResultDto {
                    severity: match r.severity {
                        nebudcs_model::validation::ValidationSeverity::Error => "error".to_string(),
                        nebudcs_model::validation::ValidationSeverity::Warning => {
                            "warning".to_string()
                        }
                        nebudcs_model::validation::ValidationSeverity::Info => "info".to_string(),
                        nebudcs_model::validation::ValidationSeverity::EngineeringReview => {
                            "engineering_review".to_string()
                        }
                    },
                    code: r.code.clone(),
                    message: r.message.clone(),
                    entity_id: r.entity.map(|e| e.to_string()),
                })
                .collect(),
        }
    }
}

fn chrono_now_iso() -> String {
    // Generates a simple timestamp string
    "2026-09-06T00:00:00Z".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nebudcs_scripting::CustomEquipmentManager;

    #[test]
    fn test_all_domain_plants_validate_cleanly() {
        let mgr = ProjectManager::new();
        assert_eq!(mgr.projects.len(), 6);

        let eq_mgr = CustomEquipmentManager::new();
        let mut defs = IndexMap::new();
        for def in eq_mgr.list_definition_objects() {
            let d = def.to_domain_definition();
            defs.insert(d.id, d);
        }
        for t in eq_mgr.library().list_templates(None, None) {
            let d = t.definition.to_domain_definition();
            defs.insert(d.id, d);
        }
        for d in nebudcs_scripting::standard_domain_equipment_definitions() {
            defs.insert(d.id, d);
        }

        for pid in &[
            DEFAULT_PROJECT_ID,
            PROJ_REFINERY_CDU,
            PROJ_POWER_TURBINE,
            PROJ_GAS_COMPRESSION,
            PROJ_CHEM_REACTION,
            PROJ_NHT_REFERENCE,
        ] {
            let res = mgr.validate_project(pid, &defs).expect("Project exists");
            assert!(
                res.valid,
                "Project {} failed validation with {} errors: {:?}",
                pid, res.error_count, res.results
            );
            assert_eq!(res.error_count, 0, "Project {} had validation errors", pid);
        }

        for (pid, pkg_dir) in &[
            (PROJ_REFINERY_CDU, "packages/domain-refinery"),
            (PROJ_POWER_TURBINE, "packages/domain-power"),
            (PROJ_GAS_COMPRESSION, "packages/domain-gas"),
            (PROJ_CHEM_REACTION, "packages/domain-chemical"),
        ] {
            if let Some(dto) = mgr.get_project(pid) {
                let path = format!("../../{pkg_dir}/plant_template.json");
                // Also write relative to workspace root
                let root_path = format!("{pkg_dir}/plant_template.json");
                let json = serde_json::to_string_pretty(&dto).unwrap();
                let _ = std::fs::write(&path, &json);
                let _ = std::fs::write(&root_path, &json);
            }
        }
    }
}