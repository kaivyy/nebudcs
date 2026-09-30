# Graph Report - NebuDCS  (2026-09-08)

## Corpus Check
- 583 files · ~1,213,681 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 7828 nodes · 16538 edges · 439 communities (395 shown, 44 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 275 edges (avg confidence: 0.77)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `e8bd5f77`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- TopologyGraph
- String
- TopologyError
- conversion.rs
- System Architecture
- EntityId
- NebuDCS Architecture Review Report
- sim_tests.rs
- ComponentComposition
- PhysicsError
- Protection System Architecture
- NebuDCS
- SchemaVersion
- StateBuffer
- physics_tests.rs
- Project Format Architecture
- StreamDefinition
- SimulationTime
- MassFlow
- Pressure
- SimulationEngine
- SimulationClock
- roadmap.md
- ExecutionPlan
- Equipment SDK Architecture
- HMI & P&ID Architecture
- Protocol Architecture
- TagValue
- .solve_vector
- StateReadView
- time.rs
- 2. Historian Architecture
- ADR-001: Rust as the Core Language
- ADR-002: Deterministic Simulation Engine
- ADR-003: Data-Driven Equipment Models
- ADR-004: Desktop UI Framework (egui and wgpu)
- ADR-005: Web UI Framework
- ADR-006: Persistence with SQLite
- ADR-007: Scripting with Rhai
- ADR-008: Network Protocol (WebSocket + REST)
- ADR-009: Crate Architecture and Boundaries
- ADR-010: Pluggable Thermodynamics and Properties
- 3. Test Categories
- Versioning Architecture
- SimError
- phase-01-foundation.md
- Temperature
- pipe_pressure_drop
- Security Architecture
- Training Architecture
- coding-agent-rules.md
- dependency-map.md
- testing-strategy.md
- vertical-slice-01.md
- Behavior Model Architecture
- Engineering Units Architecture
- Engineering and Design Validation
- Custom Equipment Architecture
- Performance Architecture
- 7. Equipment Parameters
- Solver Architecture
- Thermodynamics Architecture
- NebuDCS Implementation Blueprint
- Physics Architecture
- implementation/README.md
- 3. Technology Direction
- 21. Faceplates
- 8. Reactor Framework
- 28. UI Strategy
- 14. Dynamic Equipment Behavior
- 15. Turbine + Auxiliary Systems
- 19. Scenario Engine
- 33. Testing
- 6. Equipment Framework
- SimulationSystem
- GenericEquipmentPhysics
- SimulationTime
- SimError
- String
- EquipmentInstance
- nebudcs-types/src/error.rs
- PidController
- Control Engine & Automation Architecture
- NebuDCS System Architecture
- Phase 5: Control Engine & Automation Specification
- PipeSegment
- Scenario Engine Architecture
- NebuDCS Architecture Overview
- conservation.rs
- Protection System Architecture
- AlarmBlock
- Domain Model
- Phase 4: Generic Process Physics & Conservation Foundation
- Architecture Traceability Matrix & Consistency Audit
- AlarmEngine
- NebuDCS Implementation Blueprint & Execution Guide
- NebuDCS: Industrial Process Simulation & DCS Training Platform
- SignalConnection
- block.rs
- TankDrainPhysicsSystem
- pid.rs
- graph.rs
- AlarmSystem
- CascadePair
- nebudcs-alarm/tests/alloc_test.rs
- AlarmBlockConfig
- .alarms
- VesselProcessPhysics
- EquipmentInstance
- audit_alarm_tests.rs
- alarm_tests.rs
- nebudcs-alarm/src/engine.rs
- NebuDCS Architecture Overview
- .new
- StepContext
- StreamDefinition
- IdealGasMixture
- HistorianPointConfig
- SqliteStorage
- ControlError
- InstrumentConfigBuilder
- HistorianPointId
- String
- tag.rs
- AlarmSystem
- audit_causality_pid_actuator_tests.rs
- HistorianReceiver
- HistorianRunMeta
- HistorianHealthTracker
- nebudcs-control/tests/alloc_test.rs
- retention.rs
- ServerConfig
- post_sim_control
- nebudcs-types/src/error.rs
- .alarms
- ControlError
- tag.rs
- pid.rs
- audit_causality_pid_actuator_tests.rs
- retention.rs
- hydraulics.rs
- EquipmentInstance
- StreamDefinition
- .get_port_definition
- vertical_slice_e2e_tests.rs
- AlarmSystem
- command.rs
- ServerMessage
- NebuAlarmPanel
- NebuSimControls
- NebuTrend
- AlarmError
- NebuTagTable
- vite.config.js
- vite.config.ts
- .fmt
- NebuConnectionStatus
- independent_verification_tests.rs
- audit_causality_pid_actuator_tests.rs
- HistorianRunMeta
- ProtocolError
- command.rs
- nebudcs-historian/src/lib.rs
- EquipmentInstance
- 3. Actuator Arbitration & Priority Hierarchy
- NebuApp
- AlarmError
- alarm_tests.rs
- .alarms
- 22. Implementation Order (Historical Note & Authoritative Roadmap Reference)
- Vertical Slice 1: End-to-End Architecture Proof
- NebuDCS: Industrial Process Simulation & DCS Training Platform
- .subscribe_broadcast
- MockWebSocket
- NebuPropertyInspector
- pipe_pressure_drop
- NebuError
- 8. SQLite Storage Architecture & Schema
- DeterministicPrng
- Phase 14 Generic Runtime Upgrade — Implementation Plan
- custom_equipment_tests.rs
- ControlError
- custom_equipment_api_tests.rs
- .flash_tp
- HistorianPointId
- .trace_phase_envelope
- Engineering and Design Validation
- Self
- Custom Equipment & Runtime Authoring Architecture
- ReferenceBenchmarkCase
- custom_equipment_tests.rs
- audit_alarm_tests.rs
- PHASE 15 FINAL INDEPENDENT VERIFICATION REPORT
- Phase 14 Generic Runtime Upgrade — SDD Progress Ledger
- .alarms
- ServerConfig
- vertical_slice_e2e_tests.rs
- ScenarioError
- create_verification_host
- audit_causality_pid_actuator_tests.rs
- 2. System Architecture & Components
- HistoricalEvent
- custom_equipment_api_tests.rs
- Phase 4: Generic Process Physics & Conservation Foundation
- HistorianRunMeta
- churchill_friction_factor
- AlarmSystem
- Domain Model
- NebuScenarioPanel
- create_test_host
- audit_alarm_tests.rs
- Architecture Traceability Matrix & Consistency Audit
- Vertical Slice 1: End-to-End Architecture Proof
- NebuDCS: Industrial Process Simulation & DCS Training Platform
- AlarmError
- alarm_tests.rs
- NebuPropertyInspector
- NebuValidationPanel
- smoke_test_runner.py
- churchill_friction_factor
- create_test_host
- NebuEquipmentPalette
- retention.rs
- NebuTagTable
- create_verification_host
- TemplateSummaryDto
- custom_equipment_api_tests.rs
- browser_functional_acceptance_suite.py
- custom_equipment_tests.rs
- .tag_snapshot
- create_test_host
- sandbox_tests.rs
- Behavior Model Architecture
- NebuAlarmPanel
- create_test_host
- create_test_host
- .new
- .new
- main
- NebuValidationPanel
- discharge_pressure
- compressor_discharge_pressure
- operating_ranges
- equipment_templates
- 22. Implementation Order (Historical Note & Authoritative Roadmap Reference)
- Architecture Traceability Matrix & Consistency Audit
- NebuDCS Implementation Blueprint & Execution Guide
- Vertical Slice 1: End-to-End Architecture Proof
- reactor_core_temp
- reactor_liquid_level
- reactor_pressure
- compressor_vibration
- discharge_temperature
- suction_pressure
- steam_header_pressure
- turbine_shaft_power
- turbine_shaft_speed
- column_overhead_pressure
- column_top_temp
- control_strategies
- furnace_coil_outlet_temp
- reflux_ratio
- NebuDCS: Industrial Process Simulation & DCS Training Platform
- control_strategies
- utility_requirements
- scenario_presets
- composition_defaults
- control_strategies
- equipment_templates
- scenario_presets
- control_strategies
- equipment_templates
- utility_requirements
- scenario_presets
- composition_defaults
- utility_requirements
- scenario_presets
- thermodynamic_requirements
- utility_requirements
- thermodynamic_requirements
- thermodynamic_requirements
- thermodynamic_requirements
- validation_rules
- verify_industrial_domain_packages.py
- .subscribe_broadcast
- composition_defaults
- validation_rules
- .new
- .list_scenarios
- 13. Stage 9: Safety Instrumented Systems (SIS / IEC 61511 Concepts)
- 16.1 Sector Maturation Profiles & Scientific Confidence
- 1. Executive Summary & Strategic Vision
- 20. Stage 16: Scalability & Performance Engineering
- 2. Current Verified Baseline & Capability Inventory
- 5. Stage 1: Control Runtime Expansion
- 6. Stage 2: IEC 61131-3 Automation Expansion
- 15. Stage 11: Enterprise Operator Training Platform (OTS)
- 17. Stage 13: Scientific Validation, Verification & Metrological Benchmarking
- 18. Stage 14: Distributed & Redundant High-Availability Runtime
- 21. Stage 17: Schema Compatibility, Migration & Rollback
- 24. Stage 20: No-Code-Loss & Preservation Strategy
- 4. Master Architecture: Dual-Axis Industrial Control & Digital Plant Model
- 9. Stage 5: Advanced Control & Process Automation
- .to_molar_flow
- .from_display
- .from_display
- .to_specific_heat_capacity
- NebuValidationPanel
- routes/mod.rs
- list_sessions
- IoSignalQuality
- InstrumentModel
- nebudcs-scenario/tests/independent_verification_tests.rs
- TaskId
- PermissiveGuard
- VesselProcessPhysics
- TraineeActionType
- create_test_host
- instrument.rs
- NebuScenarioPanel
- create_test_host
- create_test_host
- InstrumentConfig
- main
- nebudcs-alarm/src/engine.rs
- .list_projects
- .alarms
- ScriptingError
- Global Constraints
- audit_causality_pid_actuator_tests.rs
- Security Architecture
- PidConfig
- verify_stage_4_engineering_studio.py
- .from_request_parts
- ServerError
- post_sim_control
- CascadePair
- Custom Equipment & Runtime Authoring Architecture
- Performance Architecture
- NebuProjectExplorer
- pid.rs
- .execute
- NebuCrossReference
- AlarmError
- .new
- NebuIecEditor
- ProjectIoMapping
- ScenarioError
- TrainingError
- custom_equipment_api_tests.rs
- .new
- run_migration
- .new
- create_test_host
- create_test_host
- sha256.rs
- main
- NebuTagDatabase
- create_test_host
- StrategyKind
- NebuValidationPanel
- .list_scenarios
- validation_rules
- nebudcs-alarm/src/engine.rs
- nebudcs-control/tests/alloc_test.rs
- 22. Implementation Order (Historical Note & Authoritative Roadmap Reference)
- Architecture Traceability Matrix & Consistency Audit
- NebuDCS Implementation Blueprint & Execution Guide
- Vertical Slice 1: End-to-End Architecture Proof
- NebuDCS: Industrial Process Simulation & DCS Training Platform
- PermissiveGuard
- NebuIecEditor
- CommunicationsConfigDto
- 8. SQLite Storage Architecture & Schema
- ScenarioError
- InstrumentConfig
- compile_ladder
- Performance Architecture
- create_test_host
- NebuAlarmPanel
- sha256.rs
- NebuTagDatabase
- NebuIoMappingTable
- stage_8_controller_integration_tests.rs
- .list_scenarios
- .alarms
- NebuDCS Stage 10: Advanced DCS Operations & HMI — Comprehensive Engineering Report
- MockWebSocket
- ServerError
- create_test_host
- create_test_host
- validation_rules
- ScenarioError
- create_stage_9_test_host
- compile_ladder
- ParameterDefinition
- Domain Model
- create_test_host
- Option
- create_test_host
- create_stage_10_test_host
- 22. Implementation Order (Historical Note & Authoritative Roadmap Reference)
- NebuDCS Implementation Blueprint & Execution Guide
- Vertical Slice 1: End-to-End Architecture Proof
- NebuDCS: Industrial Process Simulation & DCS Training Platform
- NebuTrend
- create_stage_11_test_host
- nebudcs-alarm/src/engine.rs
- CauseEffectMatrixDto
- CommunicationsConfigDto
- NebuConnectionStatus
- TrainingError
- ScriptingError

## God Nodes (most connected - your core abstractions)
1. `String` - 826 edges
2. `HostHandle` - 231 edges
3. `PhysicsError` - 142 edges
4. `ApiClient` - 137 edges
5. `ControlError` - 122 edges
6. `Store` - 95 edges
7. `VirtualController` - 91 edges
8. `ControlSystem` - 83 edges
9. `IecValue` - 71 edges
10. `build_app()` - 70 edges

## Surprising Connections (you probably didn't know these)
- `BackupSummaryDto` --references--> `String`  [EXTRACTED]
  apps/nebudcs-server/src/backup/mod.rs → crates/nebudcs-types/src/version.rs
- `validate_equipment()` --calls--> `validate_definition()`  [INFERRED]
  apps/nebudcs-server/src/routes/equipment.rs → crates/nebudcs-scripting/src/validation.rs
- `define_equipment()` --calls--> `validate_definition()`  [INFERRED]
  apps/nebudcs-server/src/routes/equipment.rs → crates/nebudcs-scripting/src/validation.rs
- `handle_ws_session()` --calls--> `is_compatible_protocol_version()`  [INFERRED]
  apps/nebudcs-server/src/session.rs → crates/nebudcs-protocol/src/version.rs
- `test_audit_historian_event_and_state_sequence_recording()` --calls--> `create_historian_channel()`  [INFERRED]
  apps/nebudcs-server/tests/reaudit_verification_tests.rs → crates/nebudcs-historian/src/buffer.rs

## Import Cycles
- 2-file cycle: `apps/nebudcs-server/src/host.rs -> apps/nebudcs-server/src/routes/hal.rs -> apps/nebudcs-server/src/host.rs`

## Communities (439 total, 44 thin omitted)

### Community 0 - "TopologyGraph"
Cohesion: 0.16
Nodes (9): DiskStore, Option, Path, PathBuf, Result, Self, T, Vec (+1 more)

### Community 1 - "String"
Cohesion: 0.11
Nodes (17): CompiledDefinition, CustomEquipmentInstance, CustomEquipmentManager, CustomEquipmentSystem, Arc, AST, BTreeMap, Default (+9 more)

### Community 2 - "TopologyError"
Cohesion: 0.14
Nodes (23): StreamType, NebuError, EquipmentId, From, Phase, PipeSegmentId, PortId, Self (+15 more)

### Community 3 - "conversion.rs"
Cohesion: 0.12
Nodes (5): Pressure, Self, Temperature, test_pressure_gauge_conversion(), test_temperature_celsius_conversion()

### Community 4 - "System Architecture"
Cohesion: 0.20
Nodes (9): Cycle and Recycle Handling, Graph-Based Topology Engine, Port Connection Validation, Port System, Process Topology, Stream Model, Topology vs P&ID Layout Separation, Utility Degradation Propagation (+1 more)

### Community 5 - "EntityId"
Cohesion: 0.19
Nodes (16): DomainPackage, IndustrialDomainKind, load_all_standard_domain_packages(), make_chemical_processing_domain_package(), make_gas_processing_domain_package(), make_power_steam_domain_package(), make_refinery_cdu_domain_package(), OperatingRangeDefinition (+8 more)

### Community 6 - "NebuDCS Architecture Review Report"
Cohesion: 0.07
Nodes (28): 10. Solver/Convergence Assessment, 11. Control/Protection Assessment, 12. HMI/P&ID Assessment, 13. Protocol Assessment, 14. Versioning Assessment, 15. Security Review, 16. Dependency Review, 17. Performance Strategy (+20 more)

### Community 7 - "sim_tests.rs"
Cohesion: 0.10
Nodes (35): AlarmDefinitionId, AlarmDefinition, AlarmPriority, AlarmType, Capability, EnvelopeDimension, EquipmentDefinition, EquipmentInstance (+27 more)

### Community 8 - "ComponentComposition"
Cohesion: 0.04
Nodes (46): 10.1 Configurable Retention Modes, 10.2 Safe Chunked Pruning, 10. Retention & Storage Policy, 11.1 Bucket Aggregation Contract, 11. Downsampling & Bucket Aggregation Model, 12.1 Historized Event Types, 12. Event Historian (Discrete DCS Events), 13. Project, Session & Model Isolation (+38 more)

### Community 9 - "PhysicsError"
Cohesion: 0.13
Nodes (10): HistorianSender, Sender, Default, Option, ScenarioResult, Self, Vec, ScenarioEngine (+2 more)

### Community 10 - "Protection System Architecture"
Cohesion: 0.18
Nodes (10): 1. Simulation Execution Architecture, 2. Multi-Rate Update Architecture, 3. Worker Thread & Ownership Architecture, 4. State Management and Memory, 5. Simulation Modes, Clock & Tick Loop, Deterministic Execution Boundary, Rust Pseudocode (+2 more)

### Community 11 - "NebuDCS"
Cohesion: 0.06
Nodes (30): 10. Process Streams, 11. Utilities, 12. Control System, 13. Tag System, 16. Alarm System, 17. Historian, 18. Fault Injection, 1. Product Vision (+22 more)

### Community 12 - "SchemaVersion"
Cohesion: 0.13
Nodes (9): MassFlow, Power, Self, Temperature, VolumetricFlow, MassFlowUnit, PowerUnit, TemperatureUnit (+1 more)

### Community 13 - "StateBuffer"
Cohesion: 0.11
Nodes (12): ActuatorArbiter, ActuatorCommand, ActuatorConfig, ActuatorModel, CommandPriority, FailAction, Default, Into (+4 more)

### Community 14 - "physics_tests.rs"
Cohesion: 0.04
Nodes (12): churchill_friction_factor(), ControlValveHydraulics, pipe_pressure_drop(), DifferentialPressure, MassFlow, Pressure, Result, Self (+4 more)

### Community 15 - "Project Format Architecture"
Cohesion: 0.17
Nodes (12): 1. Overview, 2. What a Project Contains, 3. Project File Format Recommendation, 4. Schema Versioning & Migrations, 5. Dependency Resolution, 6. Deterministic Reproduction, 7. SQLite Schema Outline (Conceptual), 8. Architecture Diagrams (+4 more)

### Community 16 - "StreamDefinition"
Cohesion: 0.11
Nodes (20): make_stream(), PortRef, EquipmentId, IndexMap, Into, Item, Option, PortId (+12 more)

### Community 17 - "SimulationTime"
Cohesion: 0.18
Nodes (10): ResolvedTag, BTreeMap, Into, Option, Self, TagId, TagQuality, Vec (+2 more)

### Community 18 - "MassFlow"
Cohesion: 0.10
Nodes (13): CompositionBasis, Phase, PhysicalState, BTreeMap, IndexMap, Into, MassFlow, Pressure (+5 more)

### Community 19 - "Pressure"
Cohesion: 0.07
Nodes (34): NewtonConfig, NewtonReport, NewtonSolver, Default, F, Result, Self, Vec (+26 more)

### Community 20 - "SimulationEngine"
Cohesion: 0.17
Nodes (12): CubicEos, CubicEosState, CubicEosType, PhaseTarget, PureComponentParameters, IndexMap, Result, Vec (+4 more)

### Community 21 - "SimulationClock"
Cohesion: 0.07
Nodes (44): ComparisonOp, ControllerInitialState, default_seed(), DisturbanceDefinition, DisturbanceType, EquipmentInitialState, EventTrigger, FaultDefinition (+36 more)

### Community 22 - "roadmap.md"
Cohesion: 0.10
Nodes (20): Detailed Phase Specifications, NebuDCS Implementation Roadmap, Phase 0: Repository & Foundation Setup, Phase 10: Custom Equipment & Runtime Authoring, Phase 11: Standard Equipment Library, Phase 12: Advanced Process Physics & Pluggable Thermodynamics, Phase 13: Engineering Studio Application, Phase 14: Operator / DCS Station Console (+12 more)

### Community 23 - "ExecutionPlan"
Cohesion: 0.07
Nodes (26): DOM, DOM.Iterable, ES2022, src/**/*, tests/**/*, vite/client, vite.config.ts, compilerOptions (+18 more)

### Community 24 - "Equipment SDK Architecture"
Cohesion: 0.20
Nodes (9): 1. Core Architectural Distinction, 2. Current Extensibility vs. Future Runtime Authoring, 3. Data-Driven Schemas & Types (`crates/nebudcs-model`), 4. Semantic Port Model, 5. Equipment State Machine, 6. Standard Equipment Library Architecture (Phase 11), Current Rust Capability System (`crates/nebudcs-physics/src/equipment.rs`), Equipment SDK & Capability Architecture (+1 more)

### Community 25 - "HMI & P&ID Architecture"
Cohesion: 0.11
Nodes (18): 1. Separation of Three Independent Concerns, 2.1 Engineering Studio, 2.2 Operator / DCS Station, 2.3 Training / Scenario Studio, 2. The Three Product Environments, 3. UI Technology Architecture: Lightweight & Native, 4. Headless Server & Network Deployment, 5.1 Presentation vs. Domain Decoupling Invariant (+10 more)

### Community 26 - "Protocol Architecture"
Cohesion: 0.18
Nodes (11): 1. Overview & Core Invariants, 2. Communication Paradigms, 3. Implemented REST API Endpoints, 4.1 Inbound Messages (`ClientMessage`), 4.2 Outbound Messages (`ServerMessage`), 4. Implemented WebSocket Protocol, 5. Security & Authentication, 6.1 Subsystem Boundaries (+3 more)

### Community 27 - "TagValue"
Cohesion: 0.16
Nodes (19): make_def(), test_add_equipment_and_query(), test_add_valid_process_connection(), test_disconnected_components(), test_instrument_signal_topology(), test_linear_branching_merging_graph(), test_mutation_safety_leaves_topology_unchanged(), test_pump_pipe_valve_vessel_topology() (+11 more)

### Community 28 - ".solve_vector"
Cohesion: 0.11
Nodes (17): RecycleConvergenceManager, RecycleDiagnostics, Default, F, Into, Result, Self, Vec (+9 more)

### Community 29 - "StateReadView"
Cohesion: 0.18
Nodes (5): Into, Result, Self, Vec, ThermodynamicComponent

### Community 30 - "time.rs"
Cohesion: 0.22
Nodes (10): NebuResult, Self, test_advance_panics_on_negative_dt(), test_simulation_time_display(), test_simulation_time_monotonic(), test_simulation_time_positive(), test_simulation_time_rejects_nan(), test_simulation_time_rejects_negative() (+2 more)

### Community 31 - "2. Historian Architecture"
Cohesion: 0.15
Nodes (12): 1. Alarm Engine, 2. Historian Architecture, Alarm Engine & Historian Architecture, Alarm Lifecycle State Machine, Alarm Suppression & Shelving, Alarm Types, Deterministic Sampling & Clock Basis, Reader Trait & Downsampling Contract (+4 more)

### Community 32 - "ADR-001: Rust as the Core Language"
Cohesion: 0.17
Nodes (11): ADR-001: Rust as the Core Language, Alternatives Considered, Consequences, Context, Decision, Future Reconsideration Criteria, Problem, Rationale (+3 more)

### Community 33 - "ADR-002: Deterministic Simulation Engine"
Cohesion: 0.17
Nodes (11): ADR-002: Deterministic Simulation Engine, Alternatives Considered, Consequences, Context, Decision, Future Reconsideration Criteria, Problem, Rationale (+3 more)

### Community 34 - "ADR-003: Data-Driven Equipment Models"
Cohesion: 0.17
Nodes (11): ADR-003: Data-Driven Equipment Models, Alternatives Considered, Consequences, Context, Decision, Future Reconsideration Criteria, Problem, Rationale (+3 more)

### Community 35 - "ADR-004: Desktop UI Framework (egui and wgpu)"
Cohesion: 0.17
Nodes (11): ADR-004: Desktop UI Framework (egui and wgpu), Alternatives Considered, Consequences, Context, Decision, Future Reconsideration Criteria, Problem, Rationale (+3 more)

### Community 36 - "ADR-005: Web UI Framework"
Cohesion: 0.17
Nodes (11): ADR-005: Web UI Framework, Alternatives Considered, Consequences, Context, Decision, Future Reconsideration Criteria, Problem, Rationale (+3 more)

### Community 37 - "ADR-006: Persistence with SQLite"
Cohesion: 0.17
Nodes (11): ADR-006: Persistence with SQLite, Alternatives Considered, Consequences, Context, Decision, Future Reconsideration Criteria, Problem, Rationale (+3 more)

### Community 38 - "ADR-007: Scripting with Rhai"
Cohesion: 0.17
Nodes (11): ADR-007: Scripting with Rhai, Alternatives Considered, Consequences, Context, Decision, Future Reconsideration Criteria, Problem, Rationale (+3 more)

### Community 39 - "ADR-008: Network Protocol (WebSocket + REST)"
Cohesion: 0.17
Nodes (11): ADR-008: Network Protocol (WebSocket + REST), Alternatives Considered, Consequences, Context, Decision, Future Reconsideration Criteria, Problem, Rationale (+3 more)

### Community 40 - "ADR-009: Crate Architecture and Boundaries"
Cohesion: 0.22
Nodes (8): ADR-009: Crate Architecture and Workspace Boundaries, Alternatives Considered, Consequences, Context, Crate Responsibilities & Invariants, Decision, Rationale, Status

### Community 41 - "ADR-010: Pluggable Thermodynamics and Properties"
Cohesion: 0.17
Nodes (11): ADR-010: Pluggable Thermodynamics and Properties, Alternatives Considered, Consequences, Context, Decision, Future Reconsideration Criteria, Problem, Rationale (+3 more)

### Community 42 - "3. Test Categories"
Cohesion: 0.17
Nodes (11): 1. Overview, 2. Testing Pyramid, 3.1 Unit & Property Tests, 3.2 Equipment & Physics Tests, 3.3 Topology & Control Tests, 3.4 System & Engineering Tests, 3.5 Operational & UI Tests, 3. Test Categories (+3 more)

### Community 43 - "Versioning Architecture"
Cohesion: 0.17
Nodes (11): 1. Overview, 2. Immutable Versions & Provenance, 3. Dependency Locking, 4. Equipment Definition Version Lifecycle, 5. Upgrade and Migration Paths, 6. Rollback Capability, 7. Architecture Diagrams, Migration Sequence (+3 more)

### Community 44 - "SimError"
Cohesion: 0.13
Nodes (55): AbortSessionRequest, abort_session(), complete_session(), create_session(), delete_scenario(), duplicate_scenario(), DuplicateScenarioQuery, get_branches() (+47 more)

### Community 45 - "phase-01-foundation.md"
Cohesion: 0.18
Nodes (10): CARGO.TOML CONTENT, CRATE-BY-CRATE SPECIFICATION, EXIT CRITERIA, EXPLICITLY EXCLUDED, GOAL, `nebudcs-model`, `nebudcs-types`, `nebudcs-units` (+2 more)

### Community 46 - "Temperature"
Cohesion: 0.07
Nodes (22): ModelProvenance, Display, Formatter, Into, Result, Self, VersionedKey, VersionLifecycle (+14 more)

### Community 47 - "pipe_pressure_drop"
Cohesion: 0.05
Nodes (20): NebuEquipmentPalette, ProjectItemSelectedDetail, NebuValidationPanel, HalMappingDto, ProjectDetailDto, ProjectTagDto, ProjectValidationResponseDto, StudioAlarmDto (+12 more)

### Community 48 - "Security Architecture"
Cohesion: 0.17
Nodes (14): BackupBundle, BackupManifestEntry, BackupSummaryDto, chrono_now_iso(), create_backup(), restore_backup(), RestoreResultDto, BTreeMap (+6 more)

### Community 49 - "Training Architecture"
Cohesion: 0.20
Nodes (9): 1. Overview, 2. Training Layer Design, 3. Instructor Station, 4. Assessment and Grading, 5. Architecture Diagrams, Instructor / Trainee Architecture, Objective Evaluation Flow, Training Architecture (+1 more)

### Community 50 - "coding-agent-rules.md"
Cohesion: 0.08
Nodes (16): Box<dyn ControllerProgram>, CustomRoutineProgram, InterlockProgram, PidLoopProgram, Box, Clone, Debug, EquipmentId (+8 more)

### Community 51 - "dependency-map.md"
Cohesion: 0.25
Nodes (7): 1. Component Classification & Status Matrix, 2. Concrete Crate Dependency Graph, 3. Allowed vs. Forbidden Dependencies, 4. Ownership & State Boundaries, Allowed Import Rules, Crate Dependency Map & Workspace Boundaries, Forbidden Dependencies

### Community 52 - "testing-strategy.md"
Cohesion: 0.14
Nodes (13): 1. Multi-Layered Verification Architecture, 2. The Eight Verification Layers, 3. Future Security, Versioning & Governance Requirements, 4. Current Test Suite Status (Phases 0–8 Verified), Layer 1: Software Correctness (Per-Crate Unit Tests), Layer 2: Numerical Correctness & Safeguards, Layer 3: Physical & Model Verification, Layer 4: Deterministic Replay & Temporal Invariance (+5 more)

### Community 53 - "vertical-slice-01.md"
Cohesion: 0.03
Nodes (28): AntiSurgeBlock, Box<dyn FunctionBlockInstance>, CtdBlock, CtuBlock, FilterFirstOrderBlock, FTrigBlock, FunctionBlockInstance, HighSelectorBlock (+20 more)

### Community 54 - "Behavior Model Architecture"
Cohesion: 0.04
Nodes (47): 10.1 Compressor Map & Surge Boundary, 10.2 Dynamic Anti-Surge Valve Response, 10. Scientific Acceptance Plant D: Compressor Network + Stage 5 Anti-Surge Coupling, 11.1 10,000-Tick Bit-Identical Determinism, 11.2 Hot-Path Execution Budget & Memory Allocations, 11. Numerical Determinism & Performance Benchmark, 12.1 Project Model (`nebudcs-model`), 12.2 Protocol DTOs & REST API (`nebudcs-protocol` & `nebudcs-server`) (+39 more)

### Community 55 - "Engineering Units Architecture"
Cohesion: 0.25
Nodes (7): 1. Canonical Internal Units, 2. Dimensional Validation, 3. Display Unit Conversion Flow, 4. Unit-Aware Tags & Serialization, 5. Coverage of Equipment Parameters, Engineering Units Architecture, Rust Pseudocode

### Community 56 - "Engineering and Design Validation"
Cohesion: 0.05
Nodes (36): 10. Anti-Surge Compressor Protection (SLL, SCL, Margin & Rate Kick), 11. Valve Trim Characterization, 12. IEC 61131-3 Advanced Control Function Blocks, 13. Domain Plant A Implementation: Tank Level-Flow Cascade, 14. Domain Plant B Implementation: Refinery CDU Furnace Feedforward, 15. Domain Plant C Implementation: Natural Gas Compressor Anti-Surge, 16. Domain Plant D Implementation: Power Turbine Governor Override, 17. Domain Plant E Implementation: Chemical Reaction Stoichiometric Ratio (+28 more)

### Community 57 - "Custom Equipment Architecture"
Cohesion: 0.06
Nodes (18): ControllerConfig, ControllerDiagnostics, ControllerFault, ControllerLifecycle, CpuProfile, FirmwareProfile, MemoryProfile, RedundancyRole (+10 more)

### Community 58 - "Performance Architecture"
Cohesion: 0.09
Nodes (21): happy-dom, @types/node, typescript, vite, vitest, description, devDependencies, happy-dom (+13 more)

### Community 59 - "7. Equipment Parameters"
Cohesion: 0.29
Nodes (7): 7. Equipment Parameters, Compressor, Fin Fan, Heat Exchanger, Pump, Turbine, Vessel

### Community 60 - "Solver Architecture"
Cohesion: 0.17
Nodes (11): 1. Solver Architecture & Execution Hierarchy, 2.1 `RecycleSolver` (Phase 4 Foundation), 2.2 `NewtonSolver` (Phase 12 Multidimensional Nonlinear Solver), 2. Verified Nonlinear Solvers, 3.1 `ImplicitEulerSolver`, 3.2 `Bdf2Solver`, 3.3 `RadauIIASolver`, 3.4 `DaeSystem` Interface Contract (+3 more)

### Community 61 - "Thermodynamics Architecture"
Cohesion: 0.25
Nodes (10): IecExpr, IecStatement, Box, Option, Vec, Token, Parser, Result (+2 more)

### Community 62 - "NebuDCS Implementation Blueprint"
Cohesion: 0.04
Nodes (47): 10.1 Compressor Map & Surge Boundary, 10.2 Dynamic Anti-Surge Valve Response, 10. Scientific Acceptance Plant D: Compressor Network + Stage 5 Anti-Surge Coupling, 11.1 10,000-Tick Bit-Identical Determinism, 11.2 Hot-Path Execution Budget & Memory Allocations, 11. Numerical Determinism & Performance Benchmark, 12.1 Project Model (`nebudcs-model`), 12.2 Protocol DTOs & REST API (`nebudcs-protocol` & `nebudcs-server`) (+39 more)

### Community 63 - "Physics Architecture"
Cohesion: 0.20
Nodes (9): 1. Physical Architecture Overview, 2.1 Conservation Laws, 2.2 Fluid Hydraulics & Pressure-Flow Equations, 2. Conservation Laws & Core Physics (Phase 4 Foundation), 3.1 Centrifugal Compressor Performance Maps (`compressor.rs`), 3.2 Centrifugal Pump Cavitation & NPSH (`pump.rs`), 3. Phase 12 Advanced Rotating Equipment Physics, 4. Phase 12 Advanced Numerical Solvers (+1 more)

### Community 64 - "implementation/README.md"
Cohesion: 0.07
Nodes (11): ControlModuleMode, ControlModuleStatus, DcsControlModule, ModuleStrategy, Box, Into, Option, Result (+3 more)

### Community 65 - "3. Technology Direction"
Cohesion: 0.40
Nodes (5): 3. Technology Direction, Core, Desktop, Persistence, Web

### Community 66 - "21. Faceplates"
Cohesion: 0.50
Nodes (4): 21. Faceplates, Compressor Faceplate, Control Valve Faceplate, Pump Faceplate

### Community 67 - "8. Reactor Framework"
Cohesion: 0.50
Nodes (4): 8. Reactor Framework, Catalyst Circulation, CCR Reactor Trains, Regenerator Zones

### Community 68 - "28. UI Strategy"
Cohesion: 0.67
Nodes (3): 28. UI Strategy, Desktop Engineering Station, Web Operator/Training Station

### Community 74 - "SimulationSystem"
Cohesion: 0.08
Nodes (22): chrono_now_iso(), ProjectManager, AdvancedControlStrategyDto, BTreeMap, ControllerConfigDto, CreateProjectRequestDto, CrossReferenceQueryDto, CrossReferenceResultDto (+14 more)

### Community 75 - "GenericEquipmentPhysics"
Cohesion: 0.15
Nodes (37): export_package_endpoint(), export_template_endpoint(), ExportParams, get_domain(), get_package(), get_template(), import_package_endpoint(), import_template_endpoint() (+29 more)

### Community 76 - "SimulationTime"
Cohesion: 0.14
Nodes (3): IdealGasCpCorrelation, Result, ValidityState

### Community 77 - "SimError"
Cohesion: 0.08
Nodes (9): PermissiveGuard, BTreeMap, Into, Self, Vec, RsLatch, SrLatch, TofTimer (+1 more)

### Community 78 - "String"
Cohesion: 0.16
Nodes (15): AggregatedBucket, downsample_to_buckets(), quality_rank(), TagQuality, Vec, AggregatedQueryRequest, RawQueryRequest, Connection (+7 more)

### Community 79 - "EquipmentInstance"
Cohesion: 0.18
Nodes (10): 1. Architectural Principles, 2.1 State & Composition (`state`), 2.2 Property Packages (`properties`), 2.3 Hydraulics & Flow Elements (`hydraulics`), 2.4 Conservation Laws (`conservation`), 2.5 Solvers (`solver`), 2.6 Equipment Capability Framework (`equipment` & `system`), 2. Modules & Governing Physics (+2 more)

### Community 80 - "nebudcs-types/src/error.rs"
Cohesion: 0.12
Nodes (15): MockOpcUaServer, OpcUaAdapter, OpcUaDataValue, OpcUaMonitoredItem, OpcUaNodeId, OpcUaStatusCode, Arc, BTreeMap (+7 more)

### Community 82 - "PidController"
Cohesion: 0.07
Nodes (4): CascadePair, PidController, PidMode, TagQuality

### Community 83 - "Control Engine & Automation Architecture"
Cohesion: 0.15
Nodes (12): 1. Authoritative Control Pipeline, 2.1 Variables & Operating Modes, 2.2 Bumpless Transfer, 2.3 PID Mathematics & Filtering, 2.4 Signal Quality & Failsafe Handling, 2. Control Primitives & Mathematical Algorithms, 3. Automation, Logic Blocks & Protection, 4. Actuator Dynamics & Control Valves (+4 more)

### Community 84 - "NebuDCS System Architecture"
Cohesion: 0.15
Nodes (12): 1. System Context & Bounded Contexts, 2. The Authoritative Model Flow, 3. The Three Product Environments, 4. Crate Dependency Graph, 5. Technology Selection & Rationale, 6. Deployment Pipeline: Headless Ubuntu Server to Browser, 7. Concurrency & Threading Architecture, Bounded Contexts & Domain Ownership (+4 more)

### Community 85 - "Phase 5: Control Engine & Automation Specification"
Cohesion: 0.06
Nodes (31): 10. Performance Requirements, 11. Enhanced 7-Category Verification Matrix, 12. Phase Boundary Audit & Non-Goals, 1. Executive Architectural Assessment, 2.1 Mode Transition Contracts, 2.2 Controller Internal Memory & State Structures, 2. PID Controller Lifecycle & State Transitions, 3.1 Priority Hierarchy & Resolution Rules (+23 more)

### Community 86 - "PipeSegment"
Cohesion: 0.20
Nodes (12): PipeEndpoint, PipeSegment, PortRef, EquipmentId, From, Into, Option, PipeSegmentId (+4 more)

### Community 87 - "Scenario Engine Architecture"
Cohesion: 0.18
Nodes (11): 1. Overview, 2. Data-Driven Scenario Architecture, 3. Fault Injection System, 4. Scenario Lifecycle, 5. Scenario Replay, 6. Architecture Diagrams, 7. Scenario File Format Sketch, Fault Injection into Simulation Model (+3 more)

### Community 88 - "NebuDCS Architecture Overview"
Cohesion: 0.10
Nodes (17): BTreeMap, BTreeSet, EquipmentDefinitionId, EquipmentId, IndexMap, Option, PipeSegmentId, PortId (+9 more)

### Community 89 - "conservation.rs"
Cohesion: 0.35
Nodes (17): abort_scenario(), AbortScenarioRequest, clear_fault(), ClearFaultRequest, get_scenario_status(), inject_fault(), load_scenario(), pause_scenario() (+9 more)

### Community 90 - "Protection System Architecture"
Cohesion: 0.17
Nodes (11): 1. Safety Disclaimer, 2. Protection Layer Architecture & Priority Hierarchy, 3. Cause-and-Effect Matrix Design, 4. Voting Logic (1oo1, 1oo2, 2oo3), 5. Latching and Reset Mechanics, 6. Emergency Shutdown (ESD) Architecture, 7. Runtime Architecture & Execution Invariants, Actuator Command Priority Order (+3 more)

### Community 91 - "AlarmBlock"
Cohesion: 0.06
Nodes (30): 10. HMI End-to-End Evidence, 11. Historian / Event Evidence, 12. Custom Project Evidence, 13. State Isolation Evidence, 14. Hardcode Audit, 15. Scientific Integrity Audit, 16. Regression Results, 17. Findings (+22 more)

### Community 92 - "Domain Model"
Cohesion: 0.12
Nodes (16): Backend Genericity: Runtime Trace, Corrective Actions Required, Executive Summary, F-14-01 — PROCESS VIEW: Fully Hardcoded SVG Process Graphic [BLOCKING], F-14-02 — TREND VIEWER: Hardcoded Pen Groups [BLOCKING], F-14-03 — STORE: Hardcoded Default Faceplate and Loop State [MATERIAL], F-14-04 — ALARM PANEL: Hardcoded GOTO Navigation [MATERIAL], F-14-05 — SERVER: Unconditional Baseline Tag Seed in main.rs [MATERIAL] (+8 more)

### Community 93 - "Phase 4: Generic Process Physics & Conservation Foundation"
Cohesion: 0.36
Nodes (12): create_custom_audit_project(), create_test_host(), ProjectDetailDto, test_audit_closed_loop_control_and_tracking(), test_audit_determinism_and_multi_run_repeatability(), test_audit_historian_event_and_state_sequence_recording(), test_audit_independent_trip_causality_no_unconditional_cascade(), test_audit_material_flow_chain_propagation() (+4 more)

### Community 94 - "Architecture Traceability Matrix & Consistency Audit"
Cohesion: 0.13
Nodes (19): ProtocolError, ProtocolErrorCode, Option, MockModbusServer, ModbusAddressSpec, ModbusByteOrder, ModbusCodec, ModbusTable (+11 more)

### Community 95 - "AlarmEngine"
Cohesion: 0.13
Nodes (15): PhysicsError, Phase, CoolPropAdapter, ExternalPropertyPackage, ExternalPropertyProvider, Box, Debug, Default (+7 more)

### Community 96 - "NebuDCS Implementation Blueprint & Execution Guide"
Cohesion: 0.10
Nodes (20): MappingDirection, BTreeMap, Into, Option, Result, Self, Vec, TagMapping (+12 more)

### Community 97 - "NebuDCS: Industrial Process Simulation & DCS Training Platform"
Cohesion: 0.10
Nodes (20): 1. Executive Summary, 2. Host & Environment Specification, 3.1 Backend Release Binary, 3.2 Web Frontend Production Bundle, 3. Production Artifact Compilation, 4.1 Unit File (`/etc/systemd/system/nebudcs.service`), 4.2 Sandboxing & Hardening Directives Confirmed Active, 4. Systemd Sandboxing & Installation (+12 more)

### Community 98 - "SignalConnection"
Cohesion: 0.15
Nodes (3): AlarmEngine, Option, Self

### Community 99 - "block.rs"
Cohesion: 0.03
Nodes (71): HalMappingDto, HalSystemSummaryDto, ProjectTagDto, AreaId, AlarmInstance, Area, build_project(), default_access() (+63 more)

### Community 100 - "TankDrainPhysicsSystem"
Cohesion: 0.10
Nodes (19): HistorianPointConfig, Default, Into, Option, Self, TagId, SamplingMode, HistoricalSampleValue (+11 more)

### Community 101 - "pid.rs"
Cohesion: 0.19
Nodes (5): ExecutionBlock, ExecutionPlan, EquipmentId, Self, Vec

### Community 102 - "graph.rs"
Cohesion: 0.33
Nodes (6): find_strongly_connected_components(), BTreeMap, BTreeSet, EquipmentId, Vec, SccAnalysisResult

### Community 103 - "AlarmSystem"
Cohesion: 0.32
Nodes (6): build_simulation(), EquipmentId, Self, TagId, StateSnapshot, test_10k_ticks_full_state_bit_identical_determinism()

### Community 104 - "CascadePair"
Cohesion: 0.04
Nodes (6): ApiClient, CommunicationsConfigDto, ProtocolConnectionDto, ScenarioBranchDto, TagMappingDto, TrainingSessionDetailDto

### Community 105 - "nebudcs-alarm/tests/alloc_test.rs"
Cohesion: 0.33
Nodes (3): GlobalAlloc, Layout, TrackingAllocator

### Community 106 - "AlarmBlockConfig"
Cohesion: 0.13
Nodes (14): GasComponentProperty, IdealGasMixture, IncompressibleLiquid, PhaseEquilibrium, BTreeMap, Into, Phase, Pressure (+6 more)

### Community 107 - ".alarms"
Cohesion: 0.08
Nodes (21): AdvancedControlStrategy, BTreeMap, EquipmentId, Into, Self, ControllerInputBinding, ControllerOutputBinding, ControlSystem (+13 more)

### Community 108 - "VesselProcessPhysics"
Cohesion: 0.14
Nodes (10): Option, Result, TripDirection, Vec, SafetyInstrumentedFunction, SifBypass, SifConfig, SifDiagnostics (+2 more)

### Community 109 - "EquipmentInstance"
Cohesion: 0.19
Nodes (18): flush_batch(), HistorianWriterHandle, Arc, AtomicBool, Default, Drop, Duration, HistorianResult (+10 more)

### Community 111 - "alarm_tests.rs"
Cohesion: 0.19
Nodes (23): create_historian_channel(), t(), test_event_retention_pruning(), test_retention_multi_run_multi_point_without_rowid(), test_v01_deterministic_sampling(), test_v02_deadband_filtering(), test_v03_boxcar_heartbeat(), test_v04_quality_change_priority() (+15 more)

### Community 112 - "nebudcs-alarm/src/engine.rs"
Cohesion: 0.15
Nodes (22): auth_middleware(), extract_token_from_query(), extract_token_from_request(), resolve_token_role(), Option, Response, Result, S (+14 more)

### Community 113 - "NebuDCS Architecture Overview"
Cohesion: 0.20
Nodes (10): 1. What NebuDCS Is and Is Not, 2. Core Architectural Principles, 3. Crate Workspace Overview, 4. The Three Product Environments, 5. Deployment Architecture: Headless-First, 6. Architecture Document Navigation Index, 7. Architecture Decision Records (ADRs), NebuDCS Architecture Overview (+2 more)

### Community 114 - ".new"
Cohesion: 0.09
Nodes (31): EquipmentCategory, default_created_at(), default_schema_version(), BTreeMap, Default, EquipmentId, IndexMap, Into (+23 more)

### Community 115 - "StepContext"
Cohesion: 0.10
Nodes (20): HistorianError, Error, TagId, HistoricalEvent, Into, Option, Self, HistorianPointId (+12 more)

### Community 116 - "StreamDefinition"
Cohesion: 0.04
Nodes (32): C, CheckValveModel, DistillationColumnModel, EquipmentCapability, EquipmentExecutionContext, EquipmentExecutionContext<'a>, FlowResistanceModel, GenericEquipmentPhysics (+24 more)

### Community 117 - "IdealGasMixture"
Cohesion: 0.12
Nodes (15): CommunicationsManager, Arc, BTreeMap, CommunicationsConfigDto, ConnectionTestResultDto, Default, IndustrialZoneDto, Option (+7 more)

### Community 118 - "HistorianPointConfig"
Cohesion: 0.13
Nodes (12): EquipmentLibrary, BTreeMap, EquipmentId, HashMap, Into, Option, ScriptingResult, Self (+4 more)

### Community 119 - "SqliteStorage"
Cohesion: 0.07
Nodes (26): 10. Conclusion & Roadmap Transition, 1.1 Scope & Delivered Capabilities, 1. Executive Summary & Authoritative Verdict, 2.1 Subsystem Non-Regression Matrix, 2.2 Workspace Verification Metrics, 2. Authoritative Baseline Protection & Non-Regression Audit, 3.1 Purdue Zone Segmentation & Security Boundaries, 3. Industrial Communication Architecture (ISA-95 Levels 0–4) (+18 more)

### Community 120 - "ControlError"
Cohesion: 0.24
Nodes (17): AlarmSummaryDto, acknowledge_alarm(), AcknowledgeAlarmRequest, AlarmListResponse, get_alarm_summary(), list_alarms(), reset_protection(), ResetProtectionRequest (+9 more)

### Community 121 - "InstrumentConfigBuilder"
Cohesion: 0.06
Nodes (30): 10. I/O Configuration, 11. I/O Mapping, 12. P&ID Integration, 13. Control-Loop Integration, 14. Cross-Reference Engine, 15. Design-Time Validation Workflow, 16. Compile / Validate / Deploy Workflow, 17. Online Engineering Mode (+22 more)

### Community 122 - "HistorianPointId"
Cohesion: 0.13
Nodes (8): Curve1D, Curve2D, ExtrapolationMode, InterpolationMethod, Into, ScriptingResult, Self, Vec

### Community 123 - "String"
Cohesion: 0.07
Nodes (34): HostHandle, CauseEffectMatrixDto, ScenarioStatusDto, ServerHealthDto, SisStatusSummaryDto, StateSnapshotDto, get_all_diagnostics_inner(), get_equipment_diagnostics() (+26 more)

### Community 124 - "tag.rs"
Cohesion: 0.21
Nodes (19): build_app(), Router, create_test_host(), test_adversarial_rest_api_rejection(), test_fault_injection_and_clearing_api(), test_scenario_rest_lifecycle(), test_scenario_security_token_enforcement(), test_websocket_scenario_status_streaming() (+11 more)

### Community 125 - "AlarmSystem"
Cohesion: 0.05
Nodes (54): apply_command(), apply_load_project(), build_advanced_control_strategy(), build_alarm_snapshots(), build_cause_effect_dto(), build_controllers_dtos(), build_default_hal_system(), build_hal_dto() (+46 more)

### Community 126 - "audit_causality_pid_actuator_tests.rs"
Cohesion: 0.27
Nodes (15): EventQueryParams, FirstOutQueryParams, HistoryQueryParams, HistoryQueryResponse, query_events(), query_first_out_event(), query_history(), HistoricalEventDto (+7 more)

### Community 127 - "HistorianReceiver"
Cohesion: 0.15
Nodes (9): HistorianReceiver, HistorianRecord, HistorianSendError, Duration, Receiver, Result, SharedHealthTracker, RecvTimeoutError (+1 more)

### Community 128 - "HistorianRunMeta"
Cohesion: 0.24
Nodes (13): get_tag(), list_tags(), Json, Path, ServerResult, State, TagValueDto, Value (+5 more)

### Community 129 - "HistorianHealthTracker"
Cohesion: 0.16
Nodes (13): NebuTrainingStudio, CreateTrainingSessionRequest, EvaluationReportDto, LoadSessionProjectRequest, ReplayFrameDto, ReplaySessionSummaryDto, ScenarioAuthoringDto, ScenarioSummaryDto (+5 more)

### Community 130 - "nebudcs-control/tests/alloc_test.rs"
Cohesion: 0.11
Nodes (13): Result, TagId, TagQuality, Result, Result, Result, Result, Result (+5 more)

### Community 131 - "retention.rs"
Cohesion: 0.38
Nodes (11): Json, ServerResult, State, Value, set_manual_output(), set_pid_mode(), set_pid_setpoint(), SetManualOutputRequest (+3 more)

### Community 133 - "ServerConfig"
Cohesion: 0.11
Nodes (11): FailingSystem, make_def(), EquipmentId, Result, Vec, SystemA, SystemB, test_double_buffer_isolation_proof() (+3 more)

### Community 134 - "post_sim_control"
Cohesion: 0.13
Nodes (17): AtomicU8, ConnectionConfig, Arc, BTreeMap, ConnectionState, Into, Option, Result (+9 more)

### Community 138 - ".alarms"
Cohesion: 0.31
Nodes (23): load_all_standard_packages(), load_all_standard_templates(), make_param(), make_standard_centrifugal_compressor(), make_standard_centrifugal_pump(), make_standard_control_valve(), make_standard_cstr_reactor(), make_standard_fan() (+15 more)

### Community 139 - "ControlError"
Cohesion: 0.20
Nodes (9): CompressorMapModel, CompressorMapPoint, CompressorOperatingState, CompressorSpeedLine, Option, Pressure, Result, Self (+1 more)

### Community 140 - "tag.rs"
Cohesion: 0.18
Nodes (6): CavitationRegime, PumpCavitationModel, Option, Pressure, Result, Self

### Community 141 - "pid.rs"
Cohesion: 0.13
Nodes (10): ActiveDisturbance, DisturbanceManager, HashMap, Self, Vec, DeterministicPrng, Default, Self (+2 more)

### Community 142 - "audit_causality_pid_actuator_tests.rs"
Cohesion: 0.14
Nodes (11): get_network_diagnostics(), Json, State, EdgeDiagnostic, FlowRegime, NetworkDiagnostics, NodeDiagnostic, Default (+3 more)

### Community 143 - "retention.rs"
Cohesion: 0.15
Nodes (11): generate_run_id(), HistorianRunMeta, Into, Option, ProjectId, Self, Connection, HistorianResult (+3 more)

### Community 144 - "hydraulics.rs"
Cohesion: 0.12
Nodes (15): 1. Model-Driven Generic Equipment Placement, 2. Canonical Equipment Ports & Semantic Orientation, 3. Deterministic Orthogonal Routing & Visual Snapping, 4. Inline Stream Instrumentation (ISA-5.1), 5. Schema-Driven Property Inspection, 6. Flowsheet Validation & Project Persistence, Arbitrary Plant Acceptance Test Walkthrough, Conclusion (+7 more)

### Community 145 - "EquipmentInstance"
Cohesion: 0.10
Nodes (24): EquipmentCommand, LibraryCommand, PidModeDto, Box, BTreeMap, Option, ProjectDetailDto, RuntimeCommand (+16 more)

### Community 146 - "StreamDefinition"
Cohesion: 0.08
Nodes (11): NebuFaceplate, renderValidityBadge(), AlarmFloodMetricsDto, AlarmSnapshotDto, FaceplateTarget, FirstOutEventDto, PidModeDto, ServerHealthDto (+3 more)

### Community 147 - ".get_port_definition"
Cohesion: 0.10
Nodes (6): RackettLiquidDensityCorrelation, Send, Sync, ScientificCorrelation, WatsonHeatOfVaporizationCorrelation, ValidityDomain

### Community 148 - "vertical_slice_e2e_tests.rs"
Cohesion: 0.04
Nodes (128): ActiveFaultDto, AdvancedControlStrategyDto, AlarmFloodMetricsDto, AlarmSeverityDto, AlarmSnapshotDto, AlarmStateDto, AlarmSummaryDto, CauseEffectCellDto (+120 more)

### Community 149 - "AlarmSystem"
Cohesion: 0.09
Nodes (21): 1. Executive Summary, 2. Root Cause Diagnostics & Architecture Solutions, 3. Step-by-Step User Journey Verification Results, 4.1 Vitest Web Unit Test Suite, 4.2 Rust Server Training Integration Tests, 4.3 End-to-End Playwright Acceptance Suite Summary, 4. Test Suite Evidence, 5. Visual Evidence Artifacts (+13 more)

### Community 150 - "command.rs"
Cohesion: 0.08
Nodes (13): ControllerProgram, Send, Sync, ControllerTask, Box, Clone, Into, Option (+5 more)

### Community 151 - "ServerMessage"
Cohesion: 0.03
Nodes (110): ApiClientConfig, sharedApiClient, ControllerTab, PEN_COLORS, PenConfig, PenGroupOption, AbortSessionRequest, ActiveFaultDto (+102 more)

### Community 152 - "NebuAlarmPanel"
Cohesion: 0.13
Nodes (13): DegradationPolicy, evaluate_voting(), Into, Option, Self, TagQuality, SensorChannel, SifVotingResult (+5 more)

### Community 153 - "NebuSimControls"
Cohesion: 0.22
Nodes (20): execute_proof_test(), get_cause_effect_matrix(), get_sif(), get_sis_summary(), list_sifs(), reset_sif(), CauseEffectMatrixDto, Json (+12 more)

### Community 154 - "NebuTrend"
Cohesion: 0.21
Nodes (9): create_test_host(), test_communications_rest_endpoints_and_ssrf(), Self, TagId, t0(), TagQuality, TagSnapshot, test_tag_quality_good_and_bad() (+1 more)

### Community 155 - "AlarmError"
Cohesion: 0.13
Nodes (7): BTreeMap, Default, Option, Self, Vec, SafetyAuditEvent, SafetyInstrumentedSystem

### Community 156 - "NebuTagTable"
Cohesion: 0.15
Nodes (12): 1. Executive Summary, 2. Findings Resolution Matrix, 3. Generic Process Graphics Architecture, 4. Custom Project Proof: Alpha Separation Train, 5. State Isolation & Project Switching Proof, 6. Comprehensive Hardcode Audit, 7. Complex Coupling & Physics Architecture Verification, 8. Quality Gates Status (+4 more)

### Community 157 - "vite.config.js"
Cohesion: 0.12
Nodes (5): DeadTimeBuffer, FirstOrderFilter, LeadLagCompensator, Self, Vec

### Community 158 - "vite.config.ts"
Cohesion: 0.10
Nodes (20): author, dependencies, description, external_ports, id, internal_streams, metadata, name (+12 more)

### Community 159 - ".fmt"
Cohesion: 0.10
Nodes (19): 1. Executive Summary, 2. UI/UX Audit Findings & Resolutions, 3. Industrial Design System Tokens, 4. Multi-Resolution Responsive Verification, 5.1 Empty Engineering Studio Layout, 5.2 Categorized Equipment Palette, 5.3 Populated P&ID Flowsheet, 5.4 Active Pipe Connection & Snap Preview (+11 more)

### Community 161 - "NebuConnectionStatus"
Cohesion: 0.10
Nodes (20): author, dependencies, description, external_ports, id, internal_streams, metadata, name (+12 more)

### Community 162 - "independent_verification_tests.rs"
Cohesion: 0.14
Nodes (17): CriterionKind, CriterionResult, CriterionSeverity, default_severity(), evaluate_session(), EvaluationCriterion, EvaluationReport, BTreeMap (+9 more)

### Community 163 - "audit_causality_pid_actuator_tests.rs"
Cohesion: 0.18
Nodes (10): HydraulicNode, NodeKind, EquipmentId, Into, Option, Phase, PortId, Pressure (+2 more)

### Community 164 - "HistorianRunMeta"
Cohesion: 0.10
Nodes (18): export_package(), export_template(), import_package(), import_template(), ScriptingResult, SerializationFormat, validate_identifier(), make_standard_pump_bypass_package() (+10 more)

### Community 166 - "command.rs"
Cohesion: 0.07
Nodes (26): 10. Conclusion & Roadmap Transition, 1.1 Scope & Delivered Capabilities, 1. Executive Summary & Authoritative Verdict, 2.1 Subsystem Non-Regression Matrix, 2.2 Workspace Verification Metrics, 2. Authoritative Baseline Protection & Non-Regression Audit, 3.1 Purdue Zone Segmentation & Security Boundaries, 3. Industrial Communication Architecture (ISA-95 Levels 0–4) (+18 more)

### Community 167 - "nebudcs-historian/src/lib.rs"
Cohesion: 0.14
Nodes (13): 1. Executive Verdict, 2. Requirement Traceability Matrix, 3.1 Security & Threat Model Enforcement, 3.2 Storage Reliability & Crash Recovery, 3.3 Backup, Restore & Schema Migration, 3.4 Observability & Monitoring, 3.5 Automated Test Suite Execution Evidence, 3.6 Hardcode Scan & Architectural Cleanliness (+5 more)

### Community 168 - "EquipmentInstance"
Cohesion: 0.22
Nodes (13): default_author(), EquipmentPackage, PackageInstantiationResult, PackageInternalStream, PackageParameterMapping, PackagePortMapping, PackageSubComponent, BTreeMap (+5 more)

### Community 169 - "3. Actuator Arbitration & Priority Hierarchy"
Cohesion: 0.26
Nodes (10): create_test_tag_store(), TagId, test_deterministic_disturbances(), test_deterministic_replay_identical_trajectories(), test_deterministic_timeline_ordering_at_identical_timestamps(), test_fault_lifecycle_and_physical_effects(), test_historian_event_logging(), test_initial_conditions_application() (+2 more)

### Community 170 - "NebuApp"
Cohesion: 0.18
Nodes (9): BTreeMap, Default, Option, ProjectDetailDto, Result, ScenarioAuthoringDto, ScenarioValidationResultDto, Self (+1 more)

### Community 171 - "AlarmError"
Cohesion: 0.13
Nodes (10): CouplingKind, GenericPhysicsSystem, ProcessCoupling, BTreeMap, EquipmentId, Into, Option, Self (+2 more)

### Community 172 - "alarm_tests.rs"
Cohesion: 0.13
Nodes (9): RhaiSandbox, AST, BTreeMap, Default, Option, ScriptingResult, Self, ScriptExecutionContext (+1 more)

### Community 173 - ".alarms"
Cohesion: 0.10
Nodes (19): 1. Executive Summary, 2.1 Package Schema & Discovery, 2. Industrial Domain Package Architecture, 3.1 Refinery Domain: Atmospheric Crude Distillation Unit (CDU), 3.2 Power Domain: Thermal Power & Steam Mechanical Drive Train, 3.3 Natural Gas Domain: Compression & Anti-Surge Conditioning Train, 3.4 Chemical Domain: Continuous Exothermic CSTR Reaction Train, 3. Specifications of the 4 Industrial Domains (+11 more)

### Community 174 - "22. Implementation Order (Historical Note & Authoritative Roadmap Reference)"
Cohesion: 0.18
Nodes (7): HydraulicEdge, DifferentialPressure, Into, MassFlow, Pressure, Result, Self

### Community 175 - "Vertical Slice 1: End-to-End Architecture Proof"
Cohesion: 0.16
Nodes (16): create_session_req(), generic_authoring_dto(), CreateTrainingSessionRequest, ScenarioAuthoringDto, test_action_recording_rejected_in_non_running_session(), test_create_and_retrieve_session(), test_replay_data_available_after_complete(), test_scenario_crud_and_persistence() (+8 more)

### Community 176 - "NebuDCS: Industrial Process Simulation & DCS Training Platform"
Cohesion: 0.14
Nodes (10): EquipmentState, BTreeMap, EquipmentId, Into, Option, Self, TagId, TagQuality (+2 more)

### Community 177 - ".subscribe_broadcast"
Cohesion: 0.33
Nodes (15): control_equipment(), define_equipment(), get_definition(), get_instance(), instantiate_equipment(), list_definitions(), list_instances(), Json (+7 more)

### Community 178 - "MockWebSocket"
Cohesion: 0.14
Nodes (13): 1. Executive Summary, 2. Freeze Boundaries & Constraints Compliance, 3.1 Delineation of Implemented vs Deferred Capabilities, 3.2 Corrections Applied, 3. Documentation Consistency Audit, 4.1 Debris & Debug Statements, 4.2 Test Determinism & Flakiness Remediation, 4. Repository Integrity & Cleanliness Audit (+5 more)

### Community 179 - "NebuPropertyInspector"
Cohesion: 0.13
Nodes (19): PriorityLoggingProgram, Arc, Box, EquipmentId, Mutex, Result, Self, Vec (+11 more)

### Community 180 - "pipe_pressure_drop"
Cohesion: 0.15
Nodes (12): 1. Overview & Architecture Guarantees, 2.1 Building the Production Binary, 2.2 User & Directory Provisioning, 2.3 Environment Configuration, 2.4 Enabling Systemd Service, 2. Server Installation & Systemd Deployment, 3. Remote Operations via Tailscale Mesh Network, 4.1 Automated Daily Backups (+4 more)

### Community 181 - "NebuError"
Cohesion: 0.10
Nodes (19): 10.1 Key Engineering Studio Advancements, 10. Stage 6: Unified Engineering Environment, 11.1 Protocols & Specifications, 11. Stage 7: Open Industrial Protocols & Connectivity, 12.1 Distributed Controller Topologies, 12. Stage 8: Vendor-Neutral Virtual PLC / DCS Controller Architecture, 14.1 High-Performance HMI Design Principles, 14. Stage 10: Advanced DCS Operations & Human-Machine Interface (+11 more)

### Community 182 - "8. SQLite Storage Architecture & Schema"
Cohesion: 0.16
Nodes (23): AbortSessionRequest, CreateTrainingSessionRequest, CriterionResultDto, EvaluationReportDto, LoadSessionProjectRequest, OperatorErrorClassificationDto, RecordTraineeActionRequest, ReplayFrameDto (+15 more)

### Community 184 - "DeterministicPrng"
Cohesion: 0.17
Nodes (11): 1. Executive Summary, 2. Capability Matrix & Architectural Scope, 3.1 Crate Enhancements, 3. Implemented Enhancements & Architecture, 4. End-to-End Automated Verification, 5. Quality Gate Summary, 6. Documented Capability Gaps (Explicit & Honest), 7. Declaration (+3 more)

### Community 185 - "Phase 14 Generic Runtime Upgrade — Implementation Plan"
Cohesion: 0.16
Nodes (10): BTreeMap, Display, Formatter, Into, Option, Result, Self, SessionLifecycleState (+2 more)

### Community 186 - "custom_equipment_tests.rs"
Cohesion: 0.18
Nodes (7): DragState, NebuPidCanvas, PanState, PortLayout, Position2dDto, StudioEquipmentDto, TemplatePortDto

### Community 187 - "ControlError"
Cohesion: 0.11
Nodes (19): 23.1 Deliverable Catalog, Deliverable P0-1: Virtual Controller Runtime & Task Scheduler, Deliverable P0-2: I/O Hardware Abstraction & Channel Mapping Subsystem, Deliverable P0-3: IEC 61131-3 Structured Text (ST) Execution Engine, Deliverable P1-1: Hydraulic Network Pressure-Flow Solver, Deliverable P1-2: Multi-Component VLE Flash & Advanced Property Packages, Deliverable P1-3: IEC 61131-3 Ladder Diagram (LD) & Function Block Diagram (FBD) Engine, Deliverable P1-4: Safety Instrumented Systems (SIS) & Cause-and-Effect Matrix (CEM) (+11 more)

### Community 188 - "custom_equipment_api_tests.rs"
Cohesion: 0.28
Nodes (8): TraineeActionDto, ActionStatus, OperatorErrorClassification, Into, Option, Self, TraineeActionRecord, TraineeActionType

### Community 189 - ".flash_tp"
Cohesion: 0.32
Nodes (10): FlashResult, PhaseStabilityState, IndexMap, Option, Phase, Pressure, Result, Temperature (+2 more)

### Community 190 - "HistorianPointId"
Cohesion: 0.22
Nodes (30): create_connection(), create_mapping(), delete_connection(), delete_mapping(), get_architecture(), get_connection(), get_diagnostics(), get_mapping() (+22 more)

### Community 191 - ".trace_phase_envelope"
Cohesion: 0.14
Nodes (13): ChassisId, IoChassis, BTreeMap, Into, Result, Self, IoRack, BTreeMap (+5 more)

### Community 192 - "Engineering and Design Validation"
Cohesion: 0.07
Nodes (29): 1. Executive Summary & Authoritative Verdict, 1-out-of-1 ($1\text{oo}1$) Architecture, 1-out-of-2 ($1\text{oo}2$) Architecture (Safety-Biased), 2-out-of-2 ($2\text{oo}2$) Architecture (Availability-Biased), 2-out-of-3 ($2\text{oo}3$) Triple Modular Redundant (TMR) Architecture, 2. Scope & Delivered Capabilities, 3. Authoritative Baseline Protection & Non-Regression Audit, 4.1 Multi-Channel Voting Mathematics (+21 more)

### Community 193 - "Self"
Cohesion: 0.24
Nodes (4): PropertyPackageCapabilities, Default, Self, ToleranceClass

### Community 194 - "Custom Equipment & Runtime Authoring Architecture"
Cohesion: 0.06
Nodes (34): 10. API and Realtime Integration, 11. Security & RBAC Enforcement, 12. Performance & Benchmark Metrics, 13. Test Results, 14. Acceptance Scenario Execution, 15. Regression Results, 16. Files Changed, 17. Known Limitations (+26 more)

### Community 195 - "ReferenceBenchmarkCase"
Cohesion: 0.29
Nodes (8): BenchmarkClass, BenchmarkVerificationResult, ReferenceBenchmarkCase, ReferenceBenchmarkDatabase, IndexMap, Option, Self, Vec

### Community 196 - "custom_equipment_tests.rs"
Cohesion: 0.09
Nodes (26): Into, Option, Self, test_validation_error_is_blocking(), test_validation_result_serde(), test_validation_result_with_entity(), test_validation_warning_not_blocking(), ValidationResult (+18 more)

### Community 197 - "audit_alarm_tests.rs"
Cohesion: 0.40
Nodes (3): NebuIoPanel, HalChannelDto, HalSystemSummaryDto

### Community 198 - "PHASE 15 FINAL INDEPENDENT VERIFICATION REPORT"
Cohesion: 0.12
Nodes (15): 1. Executive Verdict, 2. Requirement Traceability Matrix, 3.1 Session Lifecycle State Machine, 3.2 Role Separation & Command Filtering, 3.3 Evaluation Engine Verification, 3.4 Read-Only Historical Replay, 3. Detailed Verification Results, 4. Test Execution Summary (+7 more)

### Community 200 - "Phase 14 Generic Runtime Upgrade — SDD Progress Ledger"
Cohesion: 0.14
Nodes (13): ControlError, EquipmentId, TagId, BTreeMap, Result, IecType, IecValue, Box (+5 more)

### Community 201 - ".alarms"
Cohesion: 0.20
Nodes (27): create_project(), delete_advanced_control_endpoint(), delete_project(), get_cross_reference_endpoint(), get_project(), list_advanced_control_endpoint(), list_projects(), load_project_sim_endpoint() (+19 more)

### Community 202 - "ServerConfig"
Cohesion: 0.18
Nodes (7): Default, Into, Option, Self, Vec, SplitRangeController, SplitRangeSegment

### Community 203 - "vertical_slice_e2e_tests.rs"
Cohesion: 0.06
Nodes (30): 10. Multi-Controller Ownership Rules & Conflict Detection, 11. Double-Buffered State & Actuator Dispatch Architecture, 12. Diagnostics, Scan Metrics & Hardware Bus Health, 13. REST API Specification & Endpoint Documentation, 14. WebSocket Realtime Streaming & Subscriptions, 15. ISA-62443 Security, RBAC & Fault Injection Authorization, 16. Web Component Architecture (`<nebu-io-panel>`), 17. Unit and Integration Test Matrix (+22 more)

### Community 204 - "ScenarioError"
Cohesion: 0.18
Nodes (10): 1. Executive Summary, 2. Existing Phase 13–16 Capabilities, 3.1 Port Mismatch Between Canvas and Library Definitions, 3.2 Pipe Stream Wiring & Routing UX, 3.3 Instrumentation on Process Streams, 3.4 Refinery-Specific Hardcoded Identifiers, 3. Discrepancies & Gaps Identified, 4. Canonical Data Flow Architecture (+2 more)

### Community 205 - "create_verification_host"
Cohesion: 0.17
Nodes (11): File Structure & Module Decomposition, Global Constraints, New Core Modules & Files, Phase 16: Production Hardening Implementation Plan, Task 1: Security Hardening — Constant-Time Auth, Server-Side RBAC, Path Traversal Defense & Rate Limiting, Task 2: Reliability & Persistence Hardening — Atomic Disk Storage & Crash Recovery, Task 3: Backup, Restore & Schema Migration Engine, Task 4: Observability, Metrics & Production Health Probes (+3 more)

### Community 207 - "2. System Architecture & Components"
Cohesion: 0.20
Nodes (9): 1. Executive Summary, 2.1 Domain Layer (`crates/nebudcs-scenario/src/training/`), 2.2 Protocol Layer (`crates/nebudcs-protocol/src/training.rs`), 2.3 Server Subsystems (`apps/nebudcs-server/`), 2.4 Web Training Studio (`web/src/`), 2. System Architecture & Components, 3. Verification & Quality Gates, 4. Deliverables Checklist (+1 more)

### Community 208 - "HistoricalEvent"
Cohesion: 0.18
Nodes (8): IoImageTable, IoSignalValue, BTreeMap, Into, Option, Self, TagQuality, test_io_image_table_crud()

### Community 209 - "custom_equipment_api_tests.rs"
Cohesion: 0.33
Nodes (6): EquipmentId, Result, Self, TankDrainPhysicsSystem, test_closed_loop_50k_ticks_deterministic_replay(), test_closed_loop_vessel_level_regulation()

### Community 210 - "Phase 4: Generic Process Physics & Conservation Foundation"
Cohesion: 0.29
Nodes (7): 1. Verified Architecture & Module Breakdown, 2. Engineering-Fidelity Assumptions & Known Limitations, 3. Numerical Safeguards & Stability Invariants, 4. Verification Evidence, Key Modules & Capabilities, Phase 4: Generic Process Physics & Conservation Foundation, Status: PHASE 4 VERIFIED

### Community 211 - "HistorianRunMeta"
Cohesion: 0.10
Nodes (17): ChannelId, AnalogInputChannel, AnalogOutputChannel, DigitalInputChannel, DigitalOutputChannel, IoChannel, Option, ChannelStatus (+9 more)

### Community 212 - "churchill_friction_factor"
Cohesion: 0.20
Nodes (9): 1. Executive Summary, 2.1 Security & Access Control Subsystem (`apps/nebudcs-server/src/security/`), 2.2 Persistence & Crash Recovery Engine (`apps/nebudcs-server/src/persistence/`), 2.3 Cryptographic Backup, Restore & Schema Migration (`apps/nebudcs-server/src/backup/`), 2.4 Observability, Telemetry & Health Probes (`apps/nebudcs-server/src/observability/`), 2.5 Packaging & Deployment Assets (`deploy/` and `docs/deployment/`), 2. Architecture & Delivered Modules, 3. Verification & Quality Gate Summary (+1 more)

### Community 213 - "AlarmSystem"
Cohesion: 0.11
Nodes (7): TankDrainSystem, EquipmentId, Self, VesselProcessPhysics, AccumulatorVolumeModel, Pressure, Density

### Community 214 - "Domain Model"
Cohesion: 0.29
Nodes (6): 1. Three Levels of Equipment Customization, 2. Future Level 3 Scripting Sandbox Architecture (Phase 10), 3. Package Architecture & Hierarchical Composition, Custom Equipment & Runtime Authoring Architecture, Security & Determinism Invariants, Tier Definitions

### Community 215 - "NebuScenarioPanel"
Cohesion: 0.14
Nodes (11): HalDiagnostics, IoSystem, BTreeMap, ControllerId, Default, F, Into, Option (+3 more)

### Community 216 - "create_test_host"
Cohesion: 0.16
Nodes (13): MockMqttBroker, MqttAdapter, MqttMessage, MqttQoS, Arc, BTreeMap, ConnectionState, Default (+5 more)

### Community 217 - "audit_alarm_tests.rs"
Cohesion: 0.26
Nodes (12): api_request(), build_plant_a(), build_plant_b(), build_plant_c(), main(), Plant A: Pumping line with level regulation, Plant B: Exothermic reaction train with separator & recycle, Ensures custom.turbine.steam is registered for Plant C (+4 more)

### Community 218 - "Architecture Traceability Matrix & Consistency Audit"
Cohesion: 0.15
Nodes (12): 1. Executive Summary, 2. Platform Subsystem Capability Matrix, 3.1 Fluid & Turbomachinery, 3.2 Thermal & Separation, 3. Generic Equipment Archetypes & Scientific Physics, 4. Dynamic Physical Propagation Architecture, 5. Generic Multi-Plant Acceptance Results, 6. Verification Evidence & Quality Gates (+4 more)

### Community 219 - "Vertical Slice 1: End-to-End Architecture Proof"
Cohesion: 0.17
Nodes (11): author, description, domain_kind, id, name, plant_template_ids, validation_rules, version (+3 more)

### Community 221 - "AlarmError"
Cohesion: 0.17
Nodes (11): author, description, domain_kind, id, name, plant_template_ids, validation_rules, version (+3 more)

### Community 222 - "alarm_tests.rs"
Cohesion: 0.26
Nodes (7): Bucket, RateLimiter, Default, HashMap, Instant, Mutex, Self

### Community 224 - "NebuPropertyInspector"
Cohesion: 0.18
Nodes (10): author, composition_defaults, Steam_H2O, description, domain_kind, id, name, plant_template_ids (+2 more)

### Community 225 - "NebuValidationPanel"
Cohesion: 0.23
Nodes (20): compile_iec(), get_controller(), inject_controller_fault(), list_controllers(), ControllerDetailDto, ControllerSummaryDto, Json, Path (+12 more)

### Community 226 - "smoke_test_runner.py"
Cohesion: 0.83
Nodes (3): http_req(), log_test(), run_suite()

### Community 227 - "churchill_friction_factor"
Cohesion: 0.15
Nodes (9): Option, IoModule, IoModuleType, Into, Option, Result, Self, Vec (+1 more)

### Community 228 - "create_test_host"
Cohesion: 0.18
Nodes (13): test_pressure_bar_conversion(), test_pressure_psi_conversion(), test_round_trip_conversion(), test_round_trip_pressure_psi(), test_speed_round_trip_rpm(), test_speed_rpm_to_rad_per_sec(), test_temperature_celsius_conversion(), test_temperature_fahrenheit_conversion() (+5 more)

### Community 229 - "NebuEquipmentPalette"
Cohesion: 0.20
Nodes (7): EnergyBalance, EnthalpyAccumulationInputs, MassBalance, MassBalanceResult, MassFlow, Result, SpeciesBalance

### Community 230 - "retention.rs"
Cohesion: 0.53
Nodes (5): prune_events_older_than(), prune_older_than(), RetentionPolicy, Connection, HistorianResult

### Community 231 - "NebuTagTable"
Cohesion: 0.07
Nodes (27): 1.1 Scope & Delivered Capabilities, 1. Executive Summary & Authoritative Verdict, 2.1 Subsystem Non-Regression Matrix, 2.2 Workspace Verification Metrics, 2. Authoritative Baseline Protection & Non-Regression Audit, 3.1 Controller Lifecycle State Machine, 3.2 DCS Control Module Mode Transitions & Bumpless Transfer, 3. Virtual PLC/DCS Architecture (+19 more)

### Community 232 - "create_verification_host"
Cohesion: 0.22
Nodes (8): MolarDensity, MolarEnthalpy, MolarEntropy, MolarVolume, NebuResult, SpecificEntropy, test_enthalpy_entropy_molar_conversions(), test_molar_density_and_volume()

### Community 233 - "TemplateSummaryDto"
Cohesion: 0.07
Nodes (28): 10. Structured Text (ST) Compiler, 11. Ladder Diagram (LD) Model & Compiler, 12. Function Block Diagram (FBD) Topological Sort & Cycle Detection, 13. Sequential Function Chart (SFC) State Machine & Qualifiers, 14. Stage 1 Task Scheduler Integration, 15. Stage 3 I/O HAL & `%I/%Q` Image Table Binding, 16. Protocol & REST API Endpoints, 17. Frontend Web Component (`<nebu-iec-editor>`) (+20 more)

### Community 234 - "custom_equipment_api_tests.rs"
Cohesion: 0.24
Nodes (6): CubicPropertyPackage, Into, Pressure, Result, Self, Temperature

### Community 236 - "custom_equipment_tests.rs"
Cohesion: 0.13
Nodes (10): ChannelKind, IoChannelAddress, Display, Err, Formatter, FromStr, Result, Self (+2 more)

### Community 237 - ".tag_snapshot"
Cohesion: 0.11
Nodes (14): F, Result, S, Self, NebuError, EquipmentId, From, Self (+6 more)

### Community 238 - "create_test_host"
Cohesion: 0.16
Nodes (12): LdCoil, LdCoilKind, LdContact, LdContactKind, LdDiagram, LdElement, LdParallelBranch, LdRung (+4 more)

### Community 239 - "sandbox_tests.rs"
Cohesion: 0.22
Nodes (8): author, description, domain_kind, id, name, plant_template_ids, version, proj-domain-refinery-cdu

### Community 240 - "Behavior Model Architecture"
Cohesion: 0.20
Nodes (8): Into, Result, ProofTestRecord, ProofTestStepResult, Into, Vec, run_simulated_proof_test(), test_07_proof_test_simulation()

### Community 241 - "NebuAlarmPanel"
Cohesion: 0.30
Nodes (7): default_action_type(), default_actuator_var(), Into, Self, SifActionType, SifFinalElement, SifSafeState

### Community 242 - "create_test_host"
Cohesion: 0.24
Nodes (10): create_vertical_slice_host(), ServerMessage, Vec, test_vertical_slice_rest_api_consistency(), test_vertical_slice_static_web_delivery_and_auth_bypass(), test_vertical_slice_websocket_full_lifecycle(), wait_for_command_ack(), MaybeTlsStream (+2 more)

### Community 244 - "create_test_host"
Cohesion: 0.22
Nodes (12): HalError, AnalogScalingConfig, Into, Option, Result, Self, test_linear_scaling_and_roundtrip(), test_namur_ne43_fault_detection() (+4 more)

### Community 246 - ".new"
Cohesion: 0.17
Nodes (8): ActiveFault, ActiveFaultSummary, FaultManager, HashMap, Option, ScenarioResult, Self, Vec

### Community 247 - ".new"
Cohesion: 0.29
Nodes (7): equipment_templates, standard.heat_exchanger.shell_and_tube, standard.pump.centrifugal, standard.tank.atmospheric, standard.valve.control_globe, standard.vessel.vertical_separator, standard.reactor.cstr

### Community 248 - "main"
Cohesion: 0.10
Nodes (8): MockFlowProcess, Arc, EquipmentId, Mutex, TagId, SharedControlSystem, test_hal_closed_loop_flow_regulation_process(), test_hal_determinism_replay_10k_ticks()

### Community 249 - "NebuValidationPanel"
Cohesion: 0.29
Nodes (7): operating_ranges, reactant_conversion, max_value, min_value, normal_operating_value, quantity, unit

### Community 250 - "discharge_pressure"
Cohesion: 0.29
Nodes (7): max_value, min_value, normal_operating_value, quantity, unit, operating_ranges, discharge_pressure

### Community 251 - "compressor_discharge_pressure"
Cohesion: 0.29
Nodes (7): max_value, min_value, normal_operating_value, quantity, unit, operating_ranges, compressor_discharge_pressure

### Community 252 - "operating_ranges"
Cohesion: 0.29
Nodes (7): max_value, min_value, normal_operating_value, quantity, unit, operating_ranges, column_bottom_temp

### Community 253 - "equipment_templates"
Cohesion: 0.29
Nodes (7): equipment_templates, standard.heat_exchanger.shell_and_tube, standard.pump.centrifugal, standard.tank.atmospheric, standard.valve.control_globe, standard.vessel.vertical_separator, standard.heater.fired_process

### Community 255 - "Architecture Traceability Matrix & Consistency Audit"
Cohesion: 0.17
Nodes (13): ControllerIoMapping, MappingDirection, ControllerId, EquipmentId, Into, Option, Result, Self (+5 more)

### Community 256 - "NebuDCS Implementation Blueprint & Execution Guide"
Cohesion: 0.12
Nodes (14): IecProgram, Box, BTreeMap, Clone, Into, Option, Result, Self (+6 more)

### Community 257 - "Vertical Slice 1: End-to-End Architecture Proof"
Cohesion: 0.23
Nodes (15): get_hal_channel(), list_hal_mappings(), list_hal_systems(), HalChannelDto, HalMappingDto, HalSystemSummaryDto, Json, Path (+7 more)

### Community 258 - "reactor_core_temp"
Cohesion: 0.33
Nodes (6): reactor_core_temp, max_value, min_value, normal_operating_value, quantity, unit

### Community 259 - "reactor_liquid_level"
Cohesion: 0.33
Nodes (6): reactor_liquid_level, max_value, min_value, normal_operating_value, quantity, unit

### Community 260 - "reactor_pressure"
Cohesion: 0.33
Nodes (6): reactor_pressure, max_value, min_value, normal_operating_value, quantity, unit

### Community 261 - "compressor_vibration"
Cohesion: 0.33
Nodes (6): max_value, min_value, normal_operating_value, quantity, unit, compressor_vibration

### Community 262 - "discharge_temperature"
Cohesion: 0.33
Nodes (6): max_value, min_value, normal_operating_value, quantity, unit, discharge_temperature

### Community 263 - "suction_pressure"
Cohesion: 0.33
Nodes (6): suction_pressure, max_value, min_value, normal_operating_value, quantity, unit

### Community 264 - "steam_header_pressure"
Cohesion: 0.33
Nodes (6): steam_header_pressure, max_value, min_value, normal_operating_value, quantity, unit

### Community 265 - "turbine_shaft_power"
Cohesion: 0.33
Nodes (6): turbine_shaft_power, max_value, min_value, normal_operating_value, quantity, unit

### Community 266 - "turbine_shaft_speed"
Cohesion: 0.33
Nodes (6): turbine_shaft_speed, max_value, min_value, normal_operating_value, quantity, unit

### Community 267 - "column_overhead_pressure"
Cohesion: 0.33
Nodes (6): max_value, min_value, normal_operating_value, quantity, unit, column_overhead_pressure

### Community 268 - "column_top_temp"
Cohesion: 0.33
Nodes (6): max_value, min_value, normal_operating_value, quantity, unit, column_top_temp

### Community 269 - "control_strategies"
Cohesion: 0.33
Nodes (6): control_strategies, LIC-101 Column Bottoms Sump Level Control via Residue Draw, LIC-102 Reflux Drum Level Control via Overhead Distillate Take-off, PIC-101 Column Head Pressure Control via Overhead Vapor/Condenser, TIC-101 Column Top Temperature Control via Reflux Flow, TIC-102 Furnace Coil Outlet Temperature Control via Fuel Gas Throttle

### Community 270 - "furnace_coil_outlet_temp"
Cohesion: 0.33
Nodes (6): max_value, min_value, normal_operating_value, quantity, unit, furnace_coil_outlet_temp

### Community 271 - "reflux_ratio"
Cohesion: 0.33
Nodes (6): reflux_ratio, max_value, min_value, normal_operating_value, quantity, unit

### Community 272 - "NebuDCS: Industrial Process Simulation & DCS Training Platform"
Cohesion: 0.50
Nodes (4): 27.1 Repository Audit Confirmation, 27.2 Recommended Immediate Next Execution Stage, 27.3 Final Program Status, 27. Program Execution Summary & Immediate Recommended Action

### Community 273 - "control_strategies"
Cohesion: 0.40
Nodes (5): control_strategies, FIC-401 Fresh Reactant Feed Pacing Control, High-Temperature Runaway Emergency Interlock (Trip Feed, Flood Coolant), LIC-401 Reactor Level Regulation via Bottoms Effluent Valve, TIC-401 Exothermic Reactor Temperature Control via Cooling Water Jacket Valve

### Community 274 - "utility_requirements"
Cohesion: 0.40
Nodes (5): COOLING_WATER, INSTRUMENT_AIR, STEAM_PRESSURE, utility_requirements, NITROGEN

### Community 275 - "scenario_presets"
Cohesion: 0.40
Nodes (5): scenario_presets, scn-chem-cooling-jacket-failure, scn-chem-feed-composition-disturbance, scn-chem-recycle-pump-trip, scn-chem-thermal-runaway

### Community 276 - "composition_defaults"
Cohesion: 0.40
Nodes (5): composition_defaults, Ethane_C2H6, Methane_CH4, Nitrogen_N2, Propane_C3H8

### Community 277 - "control_strategies"
Cohesion: 0.40
Nodes (5): control_strategies, FIC-301 Minimum-Flow Anti-Surge Recirculation Loop, LIC-301 Suction Knockout Drum Condensate Drain Level Control, PIC-301 Gas Header Discharge Pressure Control, TIC-301 Gas Aftercooler Outlet Temperature Control via Cooling Water

### Community 278 - "equipment_templates"
Cohesion: 0.40
Nodes (5): equipment_templates, standard.compressor.centrifugal, standard.heat_exchanger.shell_and_tube, standard.valve.control_globe, standard.vessel.vertical_separator

### Community 279 - "scenario_presets"
Cohesion: 0.40
Nodes (5): scenario_presets, scn-gas-antisurge-valve-failure, scn-gas-high-vibration-trip, scn-gas-loss-of-cooling, scn-gas-suction-blockage

### Community 280 - "control_strategies"
Cohesion: 0.40
Nodes (5): control_strategies, Emergency Overspeed Quick-Trip Latch (> 3800 RPM shutoff), PIC-201 Downstream Compressor Discharge Pressure Regulation, PIC-202 Surface Condenser Vacuum Pressure Control, SIC-201 Electronic Governor Speed Regulation via Throttle Valve

### Community 281 - "equipment_templates"
Cohesion: 0.40
Nodes (5): equipment_templates, standard.compressor.centrifugal, standard.heat_exchanger.shell_and_tube, standard.valve.control_globe, standard.vessel.vertical_separator

### Community 282 - "utility_requirements"
Cohesion: 0.40
Nodes (5): COOLING_WATER, INSTRUMENT_AIR, STEAM_PRESSURE, utility_requirements, ELECTRICITY

### Community 283 - "scenario_presets"
Cohesion: 0.40
Nodes (5): scenario_presets, scn-power-compressor-surge, scn-power-condenser-vacuum-loss, scn-power-steam-loss, scn-power-turbine-overspeed

### Community 284 - "composition_defaults"
Cohesion: 0.40
Nodes (5): composition_defaults, Atmospheric_Residue, Diesel, Kerosene, Naphtha

### Community 285 - "utility_requirements"
Cohesion: 0.40
Nodes (5): COOLING_WATER, FUEL_GAS, INSTRUMENT_AIR, STEAM_PRESSURE, utility_requirements

### Community 286 - "scenario_presets"
Cohesion: 0.40
Nodes (5): scenario_presets, scn-refinery-column-overpressure, scn-refinery-feed-loss, scn-refinery-furnace-flameout, scn-refinery-reflux-pump-trip

### Community 287 - "thermodynamic_requirements"
Cohesion: 0.50
Nodes (4): thermodynamic_requirements, Arrhenius Reaction Kinetics (k = A * exp(-Ea / RT)), Exothermic Heat of Reaction Release Coupling, Flash Vapor-Liquid Equilibrium Phase Separation

### Community 288 - "utility_requirements"
Cohesion: 0.50
Nodes (4): COOLING_WATER, FUEL_GAS, INSTRUMENT_AIR, utility_requirements

### Community 289 - "thermodynamic_requirements"
Cohesion: 0.50
Nodes (4): thermodynamic_requirements, Polytropic Head and Compression Work Curves, Real Gas Compressibility (Z-factor, Peng-Robinson), Two-Phase Liquid Knockout Separation

### Community 290 - "thermodynamic_requirements"
Cohesion: 0.50
Nodes (4): thermodynamic_requirements, Isentropic Turbine Expansion Efficiency, Rankine Cycle Thermodynamics, Steam Enthalpy-Entropy State Functions (IAPWS-IF97)

### Community 291 - "thermodynamic_requirements"
Cohesion: 0.50
Nodes (4): thermodynamic_requirements, Hydrocarbon Characterization (TBP/ASTM D86), LMTD / NTU Heat Transfer, Vapor-Liquid Equilibrium (Peng-Robinson / Chao-Seader)

### Community 292 - "validation_rules"
Cohesion: 0.50
Nodes (4): validation_rules, VLD-REF01: Distillation column requires connected overhead condenser and reflux return, VLD-REF02: Fired heater fuel gas supply must be interlocked with process coil minimum flow, VLD-REF03: Atmospheric crude charge pump must maintain positive NPSH margin > 1.5m

### Community 293 - "verify_industrial_domain_packages.py"
Cohesion: 0.83
Nodes (3): api_request(), main(), save_screenshot()

### Community 294 - ".subscribe_broadcast"
Cohesion: 0.50
Nodes (4): 8.1 Hydraulic Network Solver, 8.2 Thermodynamics & Multi-Component Property Packages, 8.3 Rigorous Equipment Models, 8. Stage 4: Advanced Process Physics & Thermodynamics Maturation

### Community 295 - "composition_defaults"
Cohesion: 0.67
Nodes (3): composition_defaults, Reactant_A, Solvent

### Community 296 - "validation_rules"
Cohesion: 0.07
Nodes (24): IecLanguage, compile_sfc(), create_assignment(), parse_condition_expression(), Result, Vec, Lexer, Vec (+16 more)

### Community 297 - ".new"
Cohesion: 0.20
Nodes (19): make_port(), Cardinality, PortDefinition, PortDirection, PortDomain, PortId, UtilityKind, make_port() (+11 more)

### Community 298 - ".list_scenarios"
Cohesion: 0.13
Nodes (9): BinaryInteractionMatrix, ComponentDatabase, ReferenceStateConvention, BTreeMap, Default, IndexMap, Option, TransportPropertiesSchema (+1 more)

### Community 299 - "13. Stage 9: Safety Instrumented Systems (SIS / IEC 61511 Concepts)"
Cohesion: 0.67
Nodes (3): 13.1 SIS Architecture & Separation, 13.2 Safety Management Features, 13. Stage 9: Safety Instrumented Systems (SIS / IEC 61511 Concepts)

### Community 300 - "16.1 Sector Maturation Profiles & Scientific Confidence"
Cohesion: 0.67
Nodes (3): 16.1 Sector Maturation Profiles & Scientific Confidence, 16. Stage 12: Industrial Domain Maturation, Detailed Sector Specifications

### Community 301 - "1. Executive Summary & Strategic Vision"
Cohesion: 0.67
Nodes (3): 1.1 Primary Strategic Objective, 1.2 The Unified Dual-Axis Engineering Model, 1. Executive Summary & Strategic Vision

### Community 302 - "20. Stage 16: Scalability & Performance Engineering"
Cohesion: 0.67
Nodes (3): 20.1 Quantitative Performance Budgets, 20.2 Architectural Techniques for Real-Time Execution, 20. Stage 16: Scalability & Performance Engineering

### Community 303 - "2. Current Verified Baseline & Capability Inventory"
Cohesion: 0.67
Nodes (3): 2.1 Subsystem Baseline Inventory, 2.2 Baseline Verification Metrics, 2. Current Verified Baseline & Capability Inventory

### Community 304 - "5. Stage 1: Control Runtime Expansion"
Cohesion: 0.67
Nodes (3): 5.1 Virtual Controller Architecture & Execution Model, 5.2 Controller Specifications & Features, 5. Stage 1: Control Runtime Expansion

### Community 305 - "6. Stage 2: IEC 61131-3 Automation Expansion"
Cohesion: 0.67
Nodes (3): 6.1 Language Implementations, 6.2 Standard Library & Data Types, 6. Stage 2: IEC 61131-3 Automation Expansion

### Community 313 - ".to_molar_flow"
Cohesion: 0.60
Nodes (3): MassFlow, MolarFlow, test_molar_and_mass_flow_conversion()

### Community 317 - "NebuValidationPanel"
Cohesion: 0.50
Nodes (4): Lexer<'a>, Option, Result, Self

### Community 318 - "routes/mod.rs"
Cohesion: 0.18
Nodes (9): Into, Option, Self, Vec, SfcAction, SfcActionQualifier, SfcChart, SfcStep (+1 more)

### Community 319 - "list_sessions"
Cohesion: 0.15
Nodes (4): RetentiveStore, BTreeMap, Option, TaskId

### Community 321 - "InstrumentModel"
Cohesion: 0.11
Nodes (15): default_trip_direction(), DegradationPolicy, ProtectionBlock, ProtectionConfig, ProtectionState, Into, Result, Self (+7 more)

### Community 322 - "nebudcs-scenario/tests/independent_verification_tests.rs"
Cohesion: 0.19
Nodes (18): ProjectValidationSummary, EquipmentDefinitionId, IndexMap, Option, Self, T, Vec, to_entity_id() (+10 more)

### Community 323 - "TaskId"
Cohesion: 0.15
Nodes (8): AntoineVaporPressureCorrelation, Self, Into, Option, T, ScientificParameter, ScientificParameter<T>, ScientificProvenance

### Community 324 - "PermissiveGuard"
Cohesion: 0.29
Nodes (5): now_millis(), CreateTrainingSessionRequest, Result, TrainingSessionDetailDto, session_to_dto()

### Community 325 - "VesselProcessPhysics"
Cohesion: 0.23
Nodes (3): AlarmBlock, TagQuality, AlarmState

### Community 326 - "TraineeActionType"
Cohesion: 0.12
Nodes (9): compile_st(), Result, Vec, test_fbd_topological_sort_and_unbuffered_cycle_rejection(), test_ladder_diagram_contacts_coils_and_parallel(), test_sfc_step_transitions_and_action_qualifiers(), test_structured_text_loop_iteration_guard(), test_structured_text_syntax_and_execution() (+1 more)

### Community 327 - "create_test_host"
Cohesion: 0.16
Nodes (16): build_arithmetic(), build_logic_gate(), collect_inputs(), compile_fbd(), lookup_source_pin(), BTreeMap, Result, Vec (+8 more)

### Community 328 - "instrument.rs"
Cohesion: 0.12
Nodes (11): AtomicU32, AtomicDiagnostics, ConnectionHealth, ConnectionSecurityConfig, ConnectionState, ProtocolAdapter, ProtocolDiagnostics, AtomicU64 (+3 more)

### Community 331 - "create_test_host"
Cohesion: 0.28
Nodes (18): api_request(), main(), save_screenshot(), test_section_10_plant_j_replay_and_structured_debrief(), test_section_11_plant_k_branching_scenario(), test_section_12_plant_l_10k_tick_deterministic_replay(), test_section_13_plant_m_custom_non_refinery_plant(), test_section_14_playwright_screenshots() (+10 more)

### Community 332 - "InstrumentConfig"
Cohesion: 0.17
Nodes (6): HighSelector, LowSelector, OverrideSelector, Self, SelectedSource, SelectorType

### Community 333 - "main"
Cohesion: 0.11
Nodes (7): InstrumentConfig, InstrumentConfigBuilder, InstrumentModel, Default, Result, Self, TagQuality

### Community 334 - "nebudcs-alarm/src/engine.rs"
Cohesion: 0.33
Nodes (4): Self, Vec, ValveCharacteristic, ValveCharacterizer

### Community 336 - ".alarms"
Cohesion: 0.25
Nodes (14): api_request(), save_screenshot(), test_section_10_acceptance_plant_j_deterministic_simulation(), test_section_11_playwright_ui_and_screenshots(), test_section_12_baseline_regression(), test_section_1_acceptance_plant_a_operator_hierarchy(), test_section_2_acceptance_plant_b_pid_faceplate_and_bumpless(), test_section_3_acceptance_plant_c_alarm_flood_and_historian() (+6 more)

### Community 337 - "ScriptingError"
Cohesion: 0.12
Nodes (4): ClockState, Result, Self, SimulationClock

### Community 338 - "Global Constraints"
Cohesion: 0.18
Nodes (10): Global Constraints, NebuDCS Stage 4: Unified PLC/DCS Engineering Studio Implementation Plan, Task 1: Canonical Domain Model Extensions & Backward Compatibility, Task 2: Project-Driven Simulation Deployment & Transactional Synchronization, Task 3: Authoritative Unified Validation & Cross-Reference Engine, Task 4: Engineering Studio Project Explorer & Multi-View Workspace, Task 5: Studio Engineering Panels (I/O Mapping Table, Tag Database, Cross-Reference), Task 6: Generic Plant Acceptance Configurations & Verification Script (+2 more)

### Community 339 - "audit_causality_pid_actuator_tests.rs"
Cohesion: 0.17
Nodes (11): 1. Pluggable Property Package System, 2. The `PropertyPackage` Contract, 3.1 Component Database & Binary Interaction Matrix (`component.rs`), 3.2 Generic Cubic Equation of State (`cubic.rs`), 3.3 Multicomponent Vapor-Liquid Equilibrium (VLE) Flash Solver (`flash.rs`), 3.4 External High-Precision Database Adapter (`external.rs`), 3. Phase 12 Advanced Thermodynamic Framework, 4. Component Composition Tracking (+3 more)

### Community 340 - "Security Architecture"
Cohesion: 0.51
Nodes (9): post_backup(), post_migrate(), post_reload(), post_restore(), Json, ServerResult, State, UserRole (+1 more)

### Community 341 - "PidConfig"
Cohesion: 0.22
Nodes (3): AntiSurgeAlarmState, AntiSurgeController, Self

### Community 342 - "verify_stage_4_engineering_studio.py"
Cohesion: 0.50
Nodes (8): api_request(), main(), save_screenshot(), test_section_1_canonical_hierarchy(), test_section_2_cross_reference_query(), test_section_3_validation_rules(), test_section_4_project_driven_simulation_deployment(), test_section_5_browser_ui()

### Community 343 - ".from_request_parts"
Cohesion: 0.26
Nodes (9): PhaseEnvelope, PhaseEnvelopePoint, PhaseEnvelopeSolver, PhaseEnvelopeSolver<'a>, IndexMap, Option, Result, Self (+1 more)

### Community 344 - "ServerError"
Cohesion: 0.31
Nodes (4): HydraulicNetworkSolver, Default, Result, Self

### Community 345 - "post_sim_control"
Cohesion: 0.14
Nodes (4): RatioController, Self, Option, StrategyKind

### Community 346 - "CascadePair"
Cohesion: 0.13
Nodes (7): OverflowPolicy, HashSet, Result, Self, Vec, StoreAndForwardQueue, VecDeque

### Community 348 - "Performance Architecture"
Cohesion: 0.33
Nodes (9): api_request(), save_screenshot(), test_section_1_acceptance_plant_a_simple_shutdown(), test_section_3_acceptance_plant_c_2oo3_voting_truth_table(), test_section_4_acceptance_plant_d_process_sif_feedback_and_matrix(), test_section_5_acceptance_plant_e_bpcs_sis_arbitration(), test_section_6_fault_injection_bypass_and_proof_testing(), test_section_7_playwright_studio_screenshots() (+1 more)

### Community 350 - "pid.rs"
Cohesion: 0.38
Nodes (9): create_verification_host(), test_audit_determinism_multi_run_repeatability(), test_audit_exhaustive_rest_endpoints(), test_audit_graceful_shutdown(), test_audit_lagged_client_resync(), test_audit_linearizability_read_consistency(), test_audit_numeric_input_validation(), test_audit_security_token_and_payload_limits() (+1 more)

### Community 351 - ".execute"
Cohesion: 0.18
Nodes (10): Global Constraints, NebuDCS Stage 6: Advanced Process Physics & Hydraulic Network Implementation Plan, Task 1: Hydraulic Parameters in Canonical Project Model (`nebudcs-model`), Task 2: Hydraulic Network Solver Module (`nebudcs-physics::network`), Task 3: Generic Physics System & Equipment Coupling, Task 4: Server REST API & WebSocket Diagnostics, Task 5: Scientific Validation & Generic Acceptance Plant Tests, Task 6: Engineering Studio Frontend Integration (+2 more)

### Community 354 - ".new"
Cohesion: 0.49
Nodes (10): api_request(), get_tag_map(), main(), save_screenshot(), test_section_1_advanced_control_rest_api(), test_section_2_acceptance_plant_a_cascade(), test_section_3_acceptance_plant_b_feedforward(), test_section_4_acceptance_plant_c_antisurge() (+2 more)

### Community 355 - "NebuIecEditor"
Cohesion: 0.15
Nodes (6): Display, Formatter, Option, Result, Self, UserRole

### Community 356 - "ProjectIoMapping"
Cohesion: 0.29
Nodes (7): CauseEffectCell, CauseEffectMatrix, CauseEntry, EffectEntry, Option, Self, Vec

### Community 357 - "ScenarioError"
Cohesion: 0.20
Nodes (9): 1. Overview, 2. Threat Model, 3. Security Boundaries, 4. Script Sandboxing, 5. Project File Security, 6. API and WebSocket Security, 7. Filesystem and Network Access Boundaries, 8. Package and Plugin Trust Boundaries (+1 more)

### Community 358 - "TrainingError"
Cohesion: 0.21
Nodes (4): HydraulicNetwork, BTreeMap, Option, Vec

### Community 362 - "custom_equipment_api_tests.rs"
Cohesion: 0.16
Nodes (12): Default, Option, PathBuf, Self, Vec, ServerConfig, create_test_host_with_controller(), ControllerId (+4 more)

### Community 363 - ".new"
Cohesion: 0.25
Nodes (7): EquipmentId, Into, PortId, Self, TagId, SignalEndpoint, SignalMedium

### Community 364 - "run_migration"
Cohesion: 0.12
Nodes (7): Self, PidConfig, PidDirection, Default, Result, Self, test_complete_causal_chain_with_faults_and_recovery()

### Community 365 - ".new"
Cohesion: 0.26
Nodes (13): collect_metrics(), read_process_rss_bytes(), Instant, server_start_time(), ServerMetricsDto, get_health(), get_live(), get_metrics() (+5 more)

### Community 367 - "create_test_host"
Cohesion: 0.14
Nodes (7): ObjectiveStatus, ObjectiveEvaluator, ObjectiveTrackerItem, Option, Self, Vec, Engine

### Community 370 - "sha256.rs"
Cohesion: 0.33
Nodes (10): api_request(), save_screenshot(), test_section_1_acceptance_plant_a_modbus(), test_section_2_acceptance_plant_b_opcua(), test_section_3_acceptance_plant_c_sparkplug_b(), test_section_4_acceptance_plant_d_robustness(), test_section_5_acceptance_plant_e_isa95_architecture(), test_section_6_cybersecurity_ssrf_and_conflict() (+2 more)

### Community 371 - "main"
Cohesion: 0.18
Nodes (10): DependencyNode, DependencyResolver, BTreeMap, HashMap, Into, ScriptingResult, Self, Vec (+2 more)

### Community 373 - "create_test_host"
Cohesion: 0.14
Nodes (12): ProtocolType, Into, Self, IndustrialConduit, IndustrialZoneDefinition, Isa95Level, Default, Into (+4 more)

### Community 374 - "StrategyKind"
Cohesion: 0.13
Nodes (15): ManagedSession, PersistedTrainingRecord, BTreeMap, Default, EvaluationReportDto, Option, ReplayFrameDto, ReplaySessionSummaryDto (+7 more)

### Community 376 - ".list_scenarios"
Cohesion: 0.48
Nodes (6): api_request(), build_plant_a_project(), run_playwright_verification(), save_screenshot(), test_section_1_network_diagnostics_endpoint(), test_section_2_acceptance_plant_a_branched_network()

### Community 377 - "validation_rules"
Cohesion: 0.14
Nodes (8): AlarmBlockConfig, AlarmType, Result, Self, TagId, AlarmError, AlarmSeverity, AlarmType

### Community 378 - "nebudcs-alarm/src/engine.rs"
Cohesion: 0.57
Nodes (6): create_chemical_plant_project(), create_gas_treating_project(), create_test_host(), ProjectDetailDto, test_multi_equipment_process_dynamics_and_coupling(), test_runtime_state_isolation_between_projects()

### Community 379 - "nebudcs-control/tests/alloc_test.rs"
Cohesion: 0.27
Nodes (5): equipmentKind(), instrumentAbbrev(), NebuProcessView, renderEquipmentSymbolHtml(), FaceplateKind

### Community 380 - "22. Implementation Order (Historical Note & Authoritative Roadmap Reference)"
Cohesion: 0.33
Nodes (10): api_request(), save_screenshot(), test_section_1_acceptance_plant_a_plc_ladder(), test_section_2_acceptance_plant_b_closed_loop_pid(), test_section_3_acceptance_plant_c_dcs_control_module(), test_section_4_acceptance_plant_d_redundancy_and_failure(), test_section_5_acceptance_plant_e_hardware_profiles_nvram(), test_section_6_fault_injection_api() (+2 more)

### Community 381 - "Architecture Traceability Matrix & Consistency Audit"
Cohesion: 0.18
Nodes (5): HistorianHealth, HistorianHealthTracker, AtomicBool, AtomicU64, Self

### Community 382 - "NebuDCS Implementation Blueprint & Execution Guide"
Cohesion: 0.40
Nodes (5): main(), Box, Error, Result, shutdown_signal()

### Community 383 - "Vertical Slice 1: End-to-End Architecture Proof"
Cohesion: 0.24
Nodes (3): helper_param(), test_stage_a1_three_customization_levels_generic_and_data_driven(), test_stage_a2_runtime_lifecycle_and_multi_instance_isolation()

### Community 385 - "PermissiveGuard"
Cohesion: 0.29
Nodes (9): create_test_rig(), TagId, test_adversarial_validation_rejects_malformed_definitions(), test_all_12_fault_types_verified(), test_critical_multi_fault_composition_different_targets_and_overlapping_lifetimes(), test_critical_multi_fault_composition_same_tag(), test_disturbance_analytic_boundaries_and_seeded_prng(), test_simulation_time_independence_from_wall_clock() (+1 more)

### Community 389 - "CommunicationsConfigDto"
Cohesion: 0.40
Nodes (5): SifDetailDto, SifBypassDto, SifDiagnosticsDto, SifFinalElementDto, SifSensorChannelDto

### Community 390 - "8. SQLite Storage Architecture & Schema"
Cohesion: 0.29
Nodes (7): ReplayFrame, ReplayMarker, ReplayMarkerKind, ReplaySessionData, BTreeMap, Self, Vec

### Community 391 - "ScenarioError"
Cohesion: 0.43
Nodes (7): migrate_project_json(), migrate_scenario_json(), MigrationReportDto, Result, Value, Vec, run_migration()

### Community 392 - "InstrumentConfig"
Cohesion: 0.53
Nodes (8): create_test_host(), make_param(), sample_pump_definition(), test_equipment_api_security_and_adversarial_rejections(), test_equipment_definition_and_query_api(), test_equipment_instantiation_and_control_lifecycle(), test_equipment_validation_api(), test_level3_scripted_equipment_api()

### Community 399 - "compile_ladder"
Cohesion: 0.20
Nodes (9): ARCHITECTURAL INVARIANTS, BEFORE STARTING ANY IMPLEMENTATION TASK, CODING RULES, COMMIT MESSAGE FORMAT, FORBIDDEN PATTERNS, PREAMBLE, WHEN A TEST FAILS, WHEN ADDING A NEW DEPENDENCY (+1 more)

### Community 400 - "Performance Architecture"
Cohesion: 0.12
Nodes (13): 1. Behavioral Definitions, 2. Failure Mode Architecture, 3. Causal & Failure Propagation, 4. Failure Propagation through Utility Networks, 5. Auxiliary System Dependencies, 6. Dynamic Equipment Behavior, Behavior Model Architecture, 1. Overview (+5 more)

### Community 401 - "create_test_host"
Cohesion: 0.25
Nodes (7): 1. Overview, 2. Validation Categories, 3. Engineering Validation (Design-Time), 4. Validation Pipeline, 5. Reference Case Architecture, 6. Validation Rule Engine, Engineering and Design Validation

### Community 403 - "NebuAlarmPanel"
Cohesion: 0.33
Nodes (5): 1. Architecture Consistency Audit, 2. Comprehensive Requirements Traceability Matrix (Phases 0–16), 3. Product Environment Matrix, Architecture Traceability Matrix & Consistency Audit, Key Findings & Alignments

### Community 404 - "sha256.rs"
Cohesion: 0.33
Nodes (3): GlobalAlloc, Layout, TrackingAllocator

### Community 409 - "stage_8_controller_integration_tests.rs"
Cohesion: 0.40
Nodes (3): create_stage_8_test_host(), ControllerId, test_stage_8_rest_endpoints_profiles_redundancy_and_modules()

### Community 412 - "NebuDCS Stage 10: Advanced DCS Operations & HMI — Comprehensive Engineering Report"
Cohesion: 0.22
Nodes (8): 1. Executive Summary & Authoritative Verdict, 2. Scope & Delivered Capabilities, 3. Acceptance Verification Across Plants A through J, 4. Playwright Automated UI Verification & Captured Artifacts, 5. Architectural Standards & Quality Compliance, 6. Baseline Non-Regression Summary, 7. Sign-Off & Official Conclusion, NebuDCS Stage 10: Advanced DCS Operations & HMI — Comprehensive Engineering Report

### Community 413 - "MockWebSocket"
Cohesion: 0.08
Nodes (8): NebuSimControls, NebuTagTable, ClientMessage, isCompatibleProtocol(), RuntimeCommand, ServerMessage, WsClient, MockWebSocket

### Community 414 - "ServerError"
Cohesion: 0.39
Nodes (4): IntoResponse, Response, ServerError, StatusCode

### Community 415 - "create_test_host"
Cohesion: 0.46
Nodes (7): create_test_host(), test_end_to_end_studio_lifecycle_workflow(), test_project_create_save_and_retrieve(), test_project_list_and_get_default(), test_project_load_simulation_endpoint(), test_project_security_and_not_found(), test_project_validation_endpoint()

### Community 416 - "create_test_host"
Cohesion: 0.46
Nodes (7): create_test_host(), test_cross_reference_endpoint(), test_default_project_has_stage_4_hierarchy(), test_load_sim_deploys_controllers_and_hal(), test_project_validation_catches_hal_direction_mismatch(), test_project_validation_catches_iec_syntax_error(), test_project_validation_catches_invalid_task_priority()

### Community 417 - "validation_rules"
Cohesion: 0.67
Nodes (3): validation_rules, VLD-PWR01: Steam turbine shaft_out must connect to rotating load with PortDomain::Mechanical, VLD-PWR02: Turbine throttle valve must fail closed (FailClosed) upon trip or loss of signal

### Community 419 - "create_stage_9_test_host"
Cohesion: 0.46
Nodes (7): create_stage_9_test_host(), EquipmentId, test_01_sis_rest_summary_and_sif_queries(), test_02_sif_administrative_bypass_rest(), test_03_cause_effect_matrix_rest(), test_04_simulated_proof_test_rest(), test_05_sif_trip_and_manual_reset_rest()

### Community 420 - "compile_ladder"
Cohesion: 0.61
Nodes (7): combine_and(), combine_or(), compile_ladder(), compile_parallel_branch(), compile_rung_elements(), Result, Vec

### Community 421 - "ParameterDefinition"
Cohesion: 0.36
Nodes (5): ParameterDefinition, make_param(), test_level1_declarative_pump_curves_and_trips(), test_level3_scripted_dynamic_behavior(), test_multiple_instances_state_isolation()

### Community 422 - "Domain Model"
Cohesion: 0.25
Nodes (7): Core Domain Types (Rust Pseudocode), Domain Hierarchy, Domain Model, Engineering Unit System, Full Domain Entity-Relationship Diagram, State Separation, Tag System Design

### Community 423 - "create_test_host"
Cohesion: 0.52
Nodes (6): create_test_host(), test_library_packages_endpoints_and_instantiate(), test_library_template_detail_and_not_found(), test_library_template_export_import_roundtrip(), test_library_template_instantiate_and_tags(), test_library_templates_list_and_filter()

### Community 424 - "Option"
Cohesion: 0.33
Nodes (4): EquipmentId, Option, TagId, TagQuality

### Community 425 - "create_test_host"
Cohesion: 0.53
Nodes (5): create_test_host(), test_iec_compile_fbd_and_sfc_api(), test_iec_compile_ld_api(), test_iec_compile_st_api(), IecCompileResponseDto

### Community 426 - "create_stage_10_test_host"
Cohesion: 0.60
Nodes (5): create_stage_10_test_host(), test_alarm_shelve_rbac_operational_separation(), test_alarm_summary_and_flood_metrics(), test_equipment_diagnostics_endpoint(), test_first_out_event_sequence_endpoint()

### Community 427 - "22. Implementation Order (Historical Note & Authoritative Roadmap Reference)"
Cohesion: 0.33
Nodes (6): 22. Implementation Order (Historical Note & Authoritative Roadmap Reference), Initial Pre-Implementation Proposal:, Phase 1 — Core Primitives, Vertical Slice 1 (Phases 2-7 progressive delivery), Vertical Slice 2, Vertical Slice 3

### Community 428 - "NebuDCS Implementation Blueprint & Execution Guide"
Cohesion: 0.33
Nodes (6): Core Product Principles & Invariants, Current Implementation Baseline, Executive Overview, How a Coding Agent Must Use These Documents, Navigation & Implementation Document Index, NebuDCS Implementation Blueprint & Execution Guide

### Community 429 - "Vertical Slice 1: End-to-End Architecture Proof"
Cohesion: 0.33
Nodes (6): 1. The Authoritative Flow of Truth, 2. Dynamic Scenario Description, 3. Required Crates & Subsystems, 4. Acceptance Criteria & Verification Gates, Goal & Purpose, Vertical Slice 1: End-to-End Architecture Proof

### Community 430 - "NebuDCS: Industrial Process Simulation & DCS Training Platform"
Cohesion: 0.33
Nodes (6): Core Product Invariants, Crate Workspace Structure, Current Implementation Baseline, Documentation Index, NebuDCS: Industrial Process Simulation & DCS Training Platform, Quickstart & Headless Verification

### Community 432 - "create_stage_11_test_host"
Cohesion: 0.70
Nodes (4): create_stage_11_test_host(), test_stage_11_branching_endpoints(), test_stage_11_rbac_trainee_operational_isolation(), test_stage_11_session_lifecycle_prepare_start_complete_debrief()

### Community 433 - "nebudcs-alarm/src/engine.rs"
Cohesion: 0.50
Nodes (3): BTreeMap, TagId, TagQuality

### Community 434 - "CauseEffectMatrixDto"
Cohesion: 0.50
Nodes (4): CauseEffectCellDto, CauseEntryDto, CauseEffectMatrixDto, EffectEntryDto

### Community 435 - "CommunicationsConfigDto"
Cohesion: 0.50
Nodes (4): CommunicationsConfigDto, IndustrialZoneDto, ProtocolConnectionDto, TagMappingDto

## Knowledge Gaps
- **1434 isolated node(s):** `AlarmType`, `StateSnapshot`, `RetentionPolicy`, `DaeSystem`, `ScientificParameter<T>` (+1429 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **44 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `String` connect `vertical_slice_e2e_tests.rs` to `TopologyGraph`, `String`, `TopologyError`, `EntityId`, `sim_tests.rs`, `PhysicsError`, `StateBuffer`, `StreamDefinition`, `SimulationTime`, `MassFlow`, `SimulationEngine`, `SimulationClock`, `.solve_vector`, `StateReadView`, `SimError`, `Temperature`, `Security Architecture`, `coding-agent-rules.md`, `vertical-slice-01.md`, `Custom Equipment Architecture`, `Thermodynamics Architecture`, `implementation/README.md`, `SimulationSystem`, `GenericEquipmentPhysics`, `SimError`, `String`, `nebudcs-types/src/error.rs`, `PipeSegment`, `NebuDCS Architecture Overview`, `conservation.rs`, `Architecture Traceability Matrix & Consistency Audit`, `AlarmEngine`, `NebuDCS Implementation Blueprint & Execution Guide`, `SignalConnection`, `block.rs`, `TankDrainPhysicsSystem`, `AlarmBlockConfig`, `.alarms`, `VesselProcessPhysics`, `EquipmentInstance`, `nebudcs-alarm/src/engine.rs`, `.new`, `StepContext`, `StreamDefinition`, `IdealGasMixture`, `HistorianPointConfig`, `ControlError`, `HistorianPointId`, `String`, `AlarmSystem`, `audit_causality_pid_actuator_tests.rs`, `HistorianRunMeta`, `retention.rs`, `post_sim_control`, `nebudcs-types/src/error.rs`, `.alarms`, `pid.rs`, `audit_causality_pid_actuator_tests.rs`, `retention.rs`, `EquipmentInstance`, `command.rs`, `NebuAlarmPanel`, `NebuSimControls`, `AlarmError`, `independent_verification_tests.rs`, `audit_causality_pid_actuator_tests.rs`, `HistorianRunMeta`, `EquipmentInstance`, `NebuApp`, `AlarmError`, `alarm_tests.rs`, `22. Implementation Order (Historical Note & Authoritative Roadmap Reference)`, `NebuDCS: Industrial Process Simulation & DCS Training Platform`, `.subscribe_broadcast`, `NebuPropertyInspector`, `8. SQLite Storage Architecture & Schema`, `Phase 14 Generic Runtime Upgrade — Implementation Plan`, `custom_equipment_api_tests.rs`, `.flash_tp`, `HistorianPointId`, `.trace_phase_envelope`, `ReferenceBenchmarkCase`, `custom_equipment_tests.rs`, `Phase 14 Generic Runtime Upgrade — SDD Progress Ledger`, `.alarms`, `ServerConfig`, `HistoricalEvent`, `HistorianRunMeta`, `NebuScenarioPanel`, `create_test_host`, `alarm_tests.rs`, `NebuValidationPanel`, `churchill_friction_factor`, `custom_equipment_api_tests.rs`, `custom_equipment_tests.rs`, `.tag_snapshot`, `create_test_host`, `Behavior Model Architecture`, `NebuAlarmPanel`, `create_test_host`, `.new`, `Architecture Traceability Matrix & Consistency Audit`, `NebuDCS Implementation Blueprint & Execution Guide`, `Vertical Slice 1: End-to-End Architecture Proof`, `validation_rules`, `.new`, `.list_scenarios`, `NebuValidationPanel`, `routes/mod.rs`, `list_sessions`, `InstrumentModel`, `TaskId`, `PermissiveGuard`, `create_test_host`, `instrument.rs`, `.from_request_parts`, `CascadePair`, `ProjectIoMapping`, `TrainingError`, `custom_equipment_api_tests.rs`, `.new`, `.new`, `create_test_host`, `main`, `NebuTagDatabase`, `create_test_host`, `StrategyKind`, `validation_rules`, `CommunicationsConfigDto`, `8. SQLite Storage Architecture & Schema`, `ScenarioError`, `stage_8_controller_integration_tests.rs`, `.alarms`, `ServerError`, `ScenarioError`, `create_stage_9_test_host`, `ParameterDefinition`, `TrainingError`, `ScriptingError`?**
  _High betweenness centrality (0.381) - this node is a cross-community bridge._
- **Why does `PhysicsError` connect `AlarmEngine` to `ControlError`, `tag.rs`, `physics_tests.rs`, `MassFlow`, `Pressure`, `vertical_slice_e2e_tests.rs`, `.get_port_definition`, `SimulationEngine`, `.solve_vector`, `StateReadView`, `.list_scenarios`, `22. Implementation Order (Historical Note & Authoritative Roadmap Reference)`, `.flash_tp`, `SimulationTime`, `AlarmSystem`, `.from_request_parts`, `ServerError`, `NebuEquipmentPalette`, `TrainingError`, `AlarmBlockConfig`, `custom_equipment_api_tests.rs`, `StreamDefinition`?**
  _High betweenness centrality (0.048) - this node is a cross-community bridge._
- **Why does `ControlSystem` connect `.alarms` to `InstrumentModel`, `nebudcs-control/tests/alloc_test.rs`, `PhysicsError`, `run_migration`, `StateBuffer`, `NebuPropertyInspector`, `vertical_slice_e2e_tests.rs`, `.new`, `NebuScenarioPanel`, `main`, `Custom Equipment Architecture`, `AlarmError`, `AlarmSystem`?**
  _High betweenness centrality (0.032) - this node is a cross-community bridge._
- **What connects `AlarmType`, `StateSnapshot`, `RetentionPolicy` to the rest of the system?**
  _1434 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `String` be split into smaller, more focused modules?**
  _Cohesion score 0.1101010101010101 - nodes in this community are weakly interconnected._
- **Should `TopologyError` be split into smaller, more focused modules?**
  _Cohesion score 0.13846153846153847 - nodes in this community are weakly interconnected._
- **Should `conversion.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.11695906432748537 - nodes in this community are weakly interconnected._