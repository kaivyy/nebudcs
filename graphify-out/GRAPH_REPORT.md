# Graph Report - NebuDCS  (2026-09-06)

## Corpus Check
- 207 files · ~162,350 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 2856 nodes · 5405 edges · 179 communities (160 shown, 19 thin omitted)
- Extraction: 99% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 76 edges (avg confidence: 0.76)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `d0b546e4`
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
- ProtocolError
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
- vite.config.ts
- .fmt
- NebuConnectionStatus
- independent_verification_tests.rs
- ProtocolError
- command.rs
- nebudcs-historian/src/lib.rs
- EquipmentInstance
- 3. Actuator Arbitration & Priority Hierarchy
- NebuApp
- AlarmError
- .alarms
- 22. Implementation Order (Historical Note & Authoritative Roadmap Reference)
- Vertical Slice 1: End-to-End Architecture Proof
- NebuDCS: Industrial Process Simulation & DCS Training Platform
- .subscribe_broadcast
- mod.rs

## God Nodes (most connected - your core abstractions)
1. `String` - 184 edges
2. `HostHandle` - 81 edges
3. `RuntimeTagStore` - 63 edges
4. `PhysicsError` - 58 edges
5. `TopologyGraph` - 49 edges
6. `SimulationTime` - 46 edges
7. `TagQuality` - 42 edges
8. `Store` - 40 edges
9. `NebuDCS` - 40 edges
10. `ComponentComposition` - 39 edges

## Surprising Connections (you probably didn't know these)
- `define_equipment()` --calls--> `validate_definition()`  [INFERRED]
  apps/nebudcs-server/src/routes/equipment.rs → crates/nebudcs-scripting/src/validation.rs
- `handle_ws_session()` --calls--> `is_compatible_protocol_version()`  [INFERRED]
  apps/nebudcs-server/src/session.rs → crates/nebudcs-protocol/src/version.rs
- `extract_token_from_query()` --references--> `String`  [EXTRACTED]
  apps/nebudcs-server/src/auth.rs → crates/nebudcs-types/src/version.rs
- `ServerConfig` --references--> `String`  [EXTRACTED]
  apps/nebudcs-server/src/config.rs → crates/nebudcs-types/src/version.rs
- `ServerError` --references--> `String`  [EXTRACTED]
  apps/nebudcs-server/src/error.rs → crates/nebudcs-types/src/version.rs

## Import Cycles
- None detected.

## Communities (179 total, 19 thin omitted)

### Community 0 - "TopologyGraph"
Cohesion: 0.15
Nodes (8): Default, Option, ScenarioResult, Self, Vec, ScenarioEngine, ScenarioStatus, ScenarioState

### Community 1 - "String"
Cohesion: 0.15
Nodes (11): CustomEquipmentInstance, CustomEquipmentManager, BTreeMap, Default, EquipmentId, HashMap, Into, Option (+3 more)

### Community 2 - "TopologyError"
Cohesion: 0.12
Nodes (24): StreamType, NebuError, EquipmentId, From, Phase, PipeSegmentId, PortId, Self (+16 more)

### Community 3 - "conversion.rs"
Cohesion: 0.05
Nodes (23): Power, Self, Speed, test_pressure_bar_conversion(), test_pressure_psi_conversion(), test_round_trip_conversion(), test_round_trip_pressure_psi(), test_speed_round_trip_rpm() (+15 more)

### Community 4 - "System Architecture"
Cohesion: 0.20
Nodes (9): Cycle and Recycle Handling, Graph-Based Topology Engine, Port Connection Validation, Port System, Process Topology, Stream Model, Topology vs P&ID Layout Separation, Utility Degradation Propagation (+1 more)

### Community 5 - "EntityId"
Cohesion: 0.10
Nodes (24): Into, Option, Self, test_validation_error_is_blocking(), test_validation_result_serde(), test_validation_result_with_entity(), test_validation_warning_not_blocking(), ValidationResult (+16 more)

### Community 6 - "NebuDCS Architecture Review Report"
Cohesion: 0.07
Nodes (28): 10. Solver/Convergence Assessment, 11. Control/Protection Assessment, 12. HMI/P&ID Assessment, 13. Protocol Assessment, 14. Versioning Assessment, 15. Security Review, 16. Dependency Review, 17. Performance Strategy (+20 more)

### Community 7 - "sim_tests.rs"
Cohesion: 0.10
Nodes (39): AlarmDefinitionId, AlarmDefinition, AlarmPriority, AlarmType, Capability, EnvelopeDimension, EquipmentDefinition, EquipmentInstance (+31 more)

### Community 8 - "ComponentComposition"
Cohesion: 0.04
Nodes (46): 10.1 Configurable Retention Modes, 10.2 Safe Chunked Pruning, 10. Retention & Storage Policy, 11.1 Bucket Aggregation Contract, 11. Downsampling & Bucket Aggregation Model, 12.1 Historized Event Types, 12. Event Historian (Discrete DCS Events), 13. Project, Session & Model Isolation (+38 more)

### Community 9 - "PhysicsError"
Cohesion: 0.18
Nodes (11): HistoricalEvent, Into, Option, Self, HistoricalSample, Self, Default, Display (+3 more)

### Community 10 - "Protection System Architecture"
Cohesion: 0.18
Nodes (10): 1. Simulation Execution Architecture, 2. Multi-Rate Update Architecture, 3. Worker Thread & Ownership Architecture, 4. State Management and Memory, 5. Simulation Modes, Clock & Tick Loop, Deterministic Execution Boundary, Rust Pseudocode (+2 more)

### Community 11 - "NebuDCS"
Cohesion: 0.06
Nodes (30): 10. Process Streams, 11. Utilities, 12. Control System, 13. Tag System, 16. Alarm System, 17. Historian, 18. Fault Injection, 1. Product Vision (+22 more)

### Community 12 - "SchemaVersion"
Cohesion: 0.11
Nodes (14): Display, Formatter, NebuResult, Option, Ord, Ordering, PartialOrd, Result (+6 more)

### Community 13 - "StateBuffer"
Cohesion: 0.13
Nodes (11): ActuatorArbiter, ActuatorCommand, ActuatorConfig, ActuatorModel, FailAction, Default, Into, Option (+3 more)

### Community 14 - "physics_tests.rs"
Cohesion: 0.05
Nodes (6): churchill_friction_factor(), test_darcy_weisbach_churchill_friction_laminar(), test_darcy_weisbach_churchill_friction_turbulent(), test_dynamic_causal_propagation_network(), test_network_source_pipe_valve_vessel_dynamic_simulation(), test_pipe_pressure_drop_with_elevation_and_reverse_flow()

### Community 15 - "Project Format Architecture"
Cohesion: 0.17
Nodes (12): 1. Overview, 2. What a Project Contains, 3. Project File Format Recommendation, 4. Schema Versioning & Migrations, 5. Dependency Resolution, 6. Deterministic Reproduction, 7. SQLite Schema Outline (Conceptual), 8. Architecture Diagrams (+4 more)

### Community 16 - "StreamDefinition"
Cohesion: 0.12
Nodes (17): make_stream(), PortRef, EquipmentId, IndexMap, Into, Item, Option, PortId (+9 more)

### Community 17 - "SimulationTime"
Cohesion: 0.18
Nodes (8): ActiveFault, ActiveFaultSummary, FaultManager, HashMap, Option, ScenarioResult, Self, Vec

### Community 18 - "MassFlow"
Cohesion: 0.11
Nodes (13): ComponentComposition, CompositionBasis, Phase, PhysicalState, BTreeMap, Density, IndexMap, Into (+5 more)

### Community 19 - "Pressure"
Cohesion: 0.12
Nodes (4): AccumulatorVolumeModel, Density, Option, SpecificEnthalpy

### Community 20 - "SimulationEngine"
Cohesion: 0.11
Nodes (10): C, GenericEquipmentPhysics, Box, BTreeMap, EquipmentId, Into, Self, TagId (+2 more)

### Community 21 - "SimulationClock"
Cohesion: 0.18
Nodes (18): ComparisonOp, ControllerInitialState, DisturbanceDefinition, DisturbanceType, EquipmentInitialState, EventTrigger, FaultDefinition, FaultType (+10 more)

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
Cohesion: 0.20
Nodes (9): RecycleSolver, Default, F, Result, Self, Vec, SolverConfig, SolverReport (+1 more)

### Community 29 - "StateReadView"
Cohesion: 0.12
Nodes (10): EquipmentState, EquipmentId, F, Into, Option, Result, Self, TagId (+2 more)

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
Cohesion: 0.19
Nodes (10): ResolvedTag, BTreeMap, Into, Option, Self, TagId, Vec, RuntimeTagStore (+2 more)

### Community 45 - "phase-01-foundation.md"
Cohesion: 0.18
Nodes (10): CARGO.TOML CONTENT, CRATE-BY-CRATE SPECIFICATION, EXIT CRITERIA, EXPLICITLY EXCLUDED, GOAL, `nebudcs-model`, `nebudcs-types`, `nebudcs-units` (+2 more)

### Community 46 - "Temperature"
Cohesion: 0.20
Nodes (7): EnergyBalance, EnthalpyAccumulationInputs, MassBalance, MassBalanceResult, Result, SpecificEnthalpy, SpeciesBalance

### Community 47 - "pipe_pressure_drop"
Cohesion: 0.08
Nodes (9): ClockState, Result, Self, SimulationClock, ExecutionBlock, ExecutionPlan, EquipmentId, Self (+1 more)

### Community 48 - "Security Architecture"
Cohesion: 0.20
Nodes (9): 1. Overview, 2. Threat Model, 3. Security Boundaries, 4. Script Sandboxing, 5. Project File Security, 6. API and WebSocket Security, 7. Filesystem and Network Access Boundaries, 8. Package and Plugin Trust Boundaries (+1 more)

### Community 49 - "Training Architecture"
Cohesion: 0.20
Nodes (9): 1. Overview, 2. Training Layer Design, 3. Instructor Station, 4. Assessment and Grading, 5. Architecture Diagrams, Instructor / Trainee Architecture, Objective Evaluation Flow, Training Architecture (+1 more)

### Community 50 - "coding-agent-rules.md"
Cohesion: 0.20
Nodes (9): ARCHITECTURAL INVARIANTS, BEFORE STARTING ANY IMPLEMENTATION TASK, CODING RULES, COMMIT MESSAGE FORMAT, FORBIDDEN PATTERNS, PREAMBLE, WHEN A TEST FAILS, WHEN ADDING A NEW DEPENDENCY (+1 more)

### Community 51 - "dependency-map.md"
Cohesion: 0.25
Nodes (7): 1. Component Classification & Status Matrix, 2. Concrete Crate Dependency Graph, 3. Allowed vs. Forbidden Dependencies, 4. Ownership & State Boundaries, Allowed Import Rules, Crate Dependency Map & Workspace Boundaries, Forbidden Dependencies

### Community 52 - "testing-strategy.md"
Cohesion: 0.14
Nodes (13): 1. Multi-Layered Verification Architecture, 2. The Eight Verification Layers, 3. Future Security, Versioning & Governance Requirements, 4. Current Test Suite Status (Phases 0–8 Verified), Layer 1: Software Correctness (Per-Crate Unit Tests), Layer 2: Numerical Correctness & Safeguards, Layer 3: Physical & Model Verification, Layer 4: Deterministic Replay & Temporal Invariance (+5 more)

### Community 53 - "vertical-slice-01.md"
Cohesion: 0.23
Nodes (6): ApiClientConfig, PenConfig, AlarmSnapshotDto, ScenarioStatusDto, StateSnapshotDto, StoreListener

### Community 54 - "Behavior Model Architecture"
Cohesion: 0.25
Nodes (7): 1. Behavioral Definitions, 2. Failure Mode Architecture, 3. Causal & Failure Propagation, 4. Failure Propagation through Utility Networks, 5. Auxiliary System Dependencies, 6. Dynamic Equipment Behavior, Behavior Model Architecture

### Community 55 - "Engineering Units Architecture"
Cohesion: 0.25
Nodes (7): 1. Canonical Internal Units, 2. Dimensional Validation, 3. Display Unit Conversion Flow, 4. Unit-Aware Tags & Serialization, 5. Coverage of Equipment Parameters, Engineering Units Architecture, Rust Pseudocode

### Community 56 - "Engineering and Design Validation"
Cohesion: 0.25
Nodes (7): 1. Overview, 2. Validation Categories, 3. Engineering Validation (Design-Time), 4. Validation Pipeline, 5. Reference Case Architecture, 6. Validation Rule Engine, Engineering and Design Validation

### Community 57 - "Custom Equipment Architecture"
Cohesion: 0.21
Nodes (8): Curve1D, Curve2D, ExtrapolationMode, InterpolationMethod, Into, ScriptingResult, Self, Vec

### Community 58 - "Performance Architecture"
Cohesion: 0.09
Nodes (21): happy-dom, @types/node, typescript, vite, vitest, description, devDependencies, happy-dom (+13 more)

### Community 59 - "7. Equipment Parameters"
Cohesion: 0.29
Nodes (7): 7. Equipment Parameters, Compressor, Fin Fan, Heat Exchanger, Pump, Turbine, Vessel

### Community 60 - "Solver Architecture"
Cohesion: 0.22
Nodes (8): 1. Solver Architecture & Execution Hierarchy, 2.1 Solver Configuration & Algorithms, 2.2 Numerical Acceleration & Convergence Math, 2.3 Convergence Criteria & Safeguards, 2. Verified Phase 4 Solver: `RecycleSolver`, 3. Dynamic Integration & Timing, 4. Advanced Solvers Roadmap (Phase 12), Numerical Solver Architecture

### Community 61 - "Thermodynamics Architecture"
Cohesion: 0.22
Nodes (8): 1. Pluggable Property Package System, 2.1 The `PropertyPackage` Trait Contract, 2.2 Verified Implementations, 2. Verified Phase 4 Property Packages, 3. Component Composition Tracking, 4. Advanced Thermodynamics Roadmap (Phase 12), Composition Invariants (`src/state.rs`), Thermodynamics & Property Package Architecture

### Community 63 - "Physics Architecture"
Cohesion: 0.18
Nodes (10): 1. Physical Architecture Overview, 2.1 Conservation Laws, 2.2 Fluid Hydraulics & Pressure-Flow Equations, 2.3 Thermodynamics & Engineering VLE, 2.4 Nonlinear Recycle Solver (`RecycleSolver`), 2.5 Numerical Safeguards & Stability Invariants, 2. Verified Phase 4 Physics Scope, 3. Engineering-Fidelity Assumptions & Known Limitations (+2 more)

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
Cohesion: 0.12
Nodes (10): FailingSystem, make_def(), EquipmentId, Vec, SystemA, SystemB, test_double_buffer_isolation_proof(), test_execution_plan_acyclic_upstream_to_downstream() (+2 more)

### Community 75 - "GenericEquipmentPhysics"
Cohesion: 0.19
Nodes (6): GenericPhysicsSystem, BTreeMap, EquipmentId, Option, Self, TagId

### Community 76 - "SimulationTime"
Cohesion: 0.10
Nodes (8): Result, ControlError, EquipmentId, TagId, InstrumentConfig, InstrumentModel, Result, Result

### Community 77 - "SimError"
Cohesion: 0.08
Nodes (9): PermissiveGuard, BTreeMap, Into, Self, Vec, RsLatch, SrLatch, TofTimer (+1 more)

### Community 78 - "String"
Cohesion: 0.17
Nodes (15): AggregatedBucket, downsample_to_buckets(), quality_rank(), Vec, AggregatedQueryRequest, RawQueryRequest, Connection, HistorianResult (+7 more)

### Community 79 - "EquipmentInstance"
Cohesion: 0.18
Nodes (10): 1. Architectural Principles, 2.1 State & Composition (`state`), 2.2 Property Packages (`properties`), 2.3 Hydraulics & Flow Elements (`hydraulics`), 2.4 Conservation Laws (`conservation`), 2.5 Solvers (`solver`), 2.6 Equipment Capability Framework (`equipment` & `system`), 2. Modules & Governing Physics (+2 more)

### Community 80 - "nebudcs-types/src/error.rs"
Cohesion: 0.16
Nodes (10): QueuedEvent, Option, Ord, Ordering, PartialOrd, Self, Vec, TimelineScheduler (+2 more)

### Community 82 - "PidController"
Cohesion: 0.07
Nodes (8): CascadePair, Self, PidConfig, PidController, PidDirection, PidMode, Default, Self

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
Cohesion: 0.18
Nodes (12): PipeEndpoint, PipeSegment, PortRef, EquipmentId, From, Into, Option, PipeSegmentId (+4 more)

### Community 87 - "Scenario Engine Architecture"
Cohesion: 0.18
Nodes (11): 1. Overview, 2. Data-Driven Scenario Architecture, 3. Fault Injection System, 4. Scenario Lifecycle, 5. Scenario Replay, 6. Architecture Diagrams, 7. Scenario File Format Sketch, Fault Injection into Simulation Model (+3 more)

### Community 88 - "NebuDCS Architecture Overview"
Cohesion: 0.12
Nodes (12): BTreeMap, BTreeSet, EquipmentDefinitionId, EquipmentId, Option, PipeSegmentId, PortId, Self (+4 more)

### Community 89 - "conservation.rs"
Cohesion: 0.21
Nodes (21): HostHandle, ScenarioStatusDto, ServerHealthDto, StateSnapshotDto, abort_scenario(), AbortScenarioRequest, clear_fault(), ClearFaultRequest (+13 more)

### Community 90 - "Protection System Architecture"
Cohesion: 0.17
Nodes (11): 1. Safety Disclaimer, 2. Protection Layer Architecture & Priority Hierarchy, 3. Cause-and-Effect Matrix Design, 4. Voting Logic (1oo1, 1oo2, 2oo3), 5. Latching and Reset Mechanics, 6. Emergency Shutdown (ESD) Architecture, 7. Runtime Architecture & Execution Invariants, Actuator Command Priority Order (+3 more)

### Community 91 - "AlarmBlock"
Cohesion: 0.19
Nodes (3): AlarmBlock, Option, AlarmState

### Community 92 - "Domain Model"
Cohesion: 0.25
Nodes (7): Core Domain Types (Rust Pseudocode), Domain Hierarchy, Domain Model, Engineering Unit System, Full Domain Entity-Relationship Diagram, State Separation, Tag System Design

### Community 93 - "Phase 4: Generic Process Physics & Conservation Foundation"
Cohesion: 0.29
Nodes (7): 1. Verified Architecture & Module Breakdown, 2. Engineering-Fidelity Assumptions & Known Limitations, 3. Numerical Safeguards & Stability Invariants, 4. Verification Evidence, Key Modules & Capabilities, Phase 4: Generic Process Physics & Conservation Foundation, Status: PHASE 4 VERIFIED

### Community 94 - "Architecture Traceability Matrix & Consistency Audit"
Cohesion: 0.33
Nodes (5): 1. Architecture Consistency Audit, 2. Comprehensive Requirements Traceability Matrix (Phases 0–16), 3. Product Environment Matrix, Architecture Traceability Matrix & Consistency Audit, Key Findings & Alignments

### Community 95 - "AlarmEngine"
Cohesion: 0.13
Nodes (17): Default, Option, Self, Vec, ServerConfig, create_vertical_slice_host(), ServerMessage, Vec (+9 more)

### Community 96 - "NebuDCS Implementation Blueprint & Execution Guide"
Cohesion: 0.33
Nodes (6): Core Product Principles & Invariants, Current Implementation Baseline, Executive Overview, How a Coding Agent Must Use These Documents, Navigation & Implementation Document Index, NebuDCS Implementation Blueprint & Execution Guide

### Community 97 - "NebuDCS: Industrial Process Simulation & DCS Training Platform"
Cohesion: 0.14
Nodes (20): validate_equipment(), EquipmentCategory, CompositionDefinition, CustomEquipmentDefinition, CustomizationTier, default_entry_point(), default_tripped_state(), InterlockRuleDefinition (+12 more)

### Community 98 - "SignalConnection"
Cohesion: 0.19
Nodes (10): EquipmentId, Into, Option, PortId, Self, SignalConnectionId, TagId, SignalConnection (+2 more)

### Community 99 - "block.rs"
Cohesion: 0.20
Nodes (16): AreaId, Area, build_project(), Plant, Project, ProjectMetadata, Into, ProjectId (+8 more)

### Community 100 - "TankDrainPhysicsSystem"
Cohesion: 0.29
Nodes (7): Density, EquipmentId, Result, Self, TankDrainPhysicsSystem, test_closed_loop_50k_ticks_deterministic_replay(), test_closed_loop_vessel_level_regulation()

### Community 101 - "pid.rs"
Cohesion: 0.16
Nodes (9): Result, Self, NebuError, EquipmentId, From, Self, TagId, SimError (+1 more)

### Community 102 - "graph.rs"
Cohesion: 0.29
Nodes (7): IndexMap, find_strongly_connected_components(), BTreeMap, BTreeSet, EquipmentId, Vec, SccAnalysisResult

### Community 103 - "AlarmSystem"
Cohesion: 0.23
Nodes (9): build_simulation(), Density, EquipmentId, Result, Self, TagId, StateSnapshot, TankDrainSystem (+1 more)

### Community 105 - "nebudcs-alarm/tests/alloc_test.rs"
Cohesion: 0.33
Nodes (3): GlobalAlloc, Layout, TrackingAllocator

### Community 106 - "AlarmBlockConfig"
Cohesion: 0.12
Nodes (17): GasComponentProperty, IdealGasMixture, IncompressibleLiquid, PhaseEquilibrium, PropertyPackage, BTreeMap, Density, Into (+9 more)

### Community 107 - ".alarms"
Cohesion: 0.16
Nodes (10): CommandPriority, RegisteredInstrument, RegisteredPid, RegisteredProtection, BTreeMap, EquipmentId, Into, Option (+2 more)

### Community 108 - "VesselProcessPhysics"
Cohesion: 0.29
Nodes (6): Density, EquipmentId, Result, Self, test_complete_causal_chain_with_faults_and_recovery(), VesselProcessPhysics

### Community 109 - "EquipmentInstance"
Cohesion: 0.13
Nodes (24): generate_run_id(), HistorianRunMeta, Into, Option, ProjectId, Self, flush_batch(), HistorianWriterHandle (+16 more)

### Community 111 - "alarm_tests.rs"
Cohesion: 0.19
Nodes (23): create_historian_channel(), t(), test_event_retention_pruning(), test_retention_multi_run_multi_point_without_rowid(), test_v01_deterministic_sampling(), test_v02_deadband_filtering(), test_v03_boxcar_heartbeat(), test_v04_quality_change_priority() (+15 more)

### Community 112 - "nebudcs-alarm/src/engine.rs"
Cohesion: 0.15
Nodes (16): auth_middleware(), extract_token_from_query(), Option, Response, Result, ServerResult, State, validate_request_auth() (+8 more)

### Community 113 - "NebuDCS Architecture Overview"
Cohesion: 0.20
Nodes (10): 1. What NebuDCS Is and Is Not, 2. Core Architectural Principles, 3. Crate Workspace Overview, 4. The Three Product Environments, 5. Deployment Architecture: Headless-First, 6. Architecture Document Navigation Index, 7. Architecture Decision Records (ADRs), NebuDCS Architecture Overview (+2 more)

### Community 114 - ".new"
Cohesion: 0.15
Nodes (9): DegradationPolicy, ProtectionBlock, ProtectionConfig, ProtectionState, Result, Self, SensorInput, VotingResult (+1 more)

### Community 115 - "StepContext"
Cohesion: 0.13
Nodes (10): HistoricalSampleValue, HistorianSampler, PointSlot, Default, HistorianResult, Item, Iterator, Option (+2 more)

### Community 116 - "StreamDefinition"
Cohesion: 0.14
Nodes (12): EquipmentCapability, EquipmentExecutionContext, FlowResistanceModel, HeadSourceModel, HeatExchangeModel, DifferentialPressure, Result, Send (+4 more)

### Community 117 - "IdealGasMixture"
Cohesion: 0.14
Nodes (7): ObjectiveStatus, ObjectiveEvaluator, ObjectiveTrackerItem, Option, Self, Vec, Engine

### Community 118 - "HistorianPointConfig"
Cohesion: 0.13
Nodes (14): HistorianError, Error, TagId, HistorianPointConfig, HistorianPointId, Default, Display, Formatter (+6 more)

### Community 119 - "SqliteStorage"
Cohesion: 0.70
Nodes (4): get_health(), Json, ServerHealthDto, State

### Community 120 - "ControlError"
Cohesion: 0.30
Nodes (14): acknowledge_alarm(), AcknowledgeAlarmRequest, AlarmListResponse, list_alarms(), reset_protection(), ResetProtectionRequest, AlarmSnapshotDto, Json (+6 more)

### Community 121 - "InstrumentConfigBuilder"
Cohesion: 0.27
Nodes (3): InstrumentConfigBuilder, Default, Self

### Community 122 - "HistorianPointId"
Cohesion: 0.21
Nodes (18): make_port(), Cardinality, Phase, PortDefinition, PortDirection, PortDomain, PortId, UtilityKind (+10 more)

### Community 123 - "String"
Cohesion: 0.20
Nodes (9): Response, State, ws_upgrade_handler(), ClientGuard, handle_ws_session(), Drop, is_compatible_protocol_version(), WebSocket (+1 more)

### Community 124 - "tag.rs"
Cohesion: 0.05
Nodes (61): build_app(), Router, main(), Box, Error, Result, shutdown_signal(), create_test_host() (+53 more)

### Community 125 - "AlarmSystem"
Cohesion: 0.14
Nodes (23): apply_command(), build_scenario_status_dto(), build_snapshot_dto(), EnqueuedCommand, Arc, AtomicBool, Drop, JoinHandle (+15 more)

### Community 126 - "audit_causality_pid_actuator_tests.rs"
Cohesion: 0.29
Nodes (12): EventQueryParams, HistoryQueryParams, HistoryQueryResponse, query_events(), query_history(), HistoricalEventDto, Json, Option (+4 more)

### Community 127 - "HistorianReceiver"
Cohesion: 0.13
Nodes (11): HistorianReceiver, HistorianRecord, HistorianSender, HistorianSendError, Duration, Receiver, Result, Sender (+3 more)

### Community 128 - "HistorianRunMeta"
Cohesion: 0.24
Nodes (13): get_tag(), list_tags(), Json, Path, ServerResult, State, TagValueDto, Value (+5 more)

### Community 129 - "HistorianHealthTracker"
Cohesion: 0.18
Nodes (5): AtomicU64, HistorianHealth, HistorianHealthTracker, AtomicBool, Self

### Community 130 - "nebudcs-control/tests/alloc_test.rs"
Cohesion: 0.33
Nodes (3): GlobalAlloc, Layout, TrackingAllocator

### Community 131 - "retention.rs"
Cohesion: 0.38
Nodes (11): Json, ServerResult, State, Value, set_manual_output(), set_pid_mode(), set_pid_setpoint(), SetManualOutputRequest (+3 more)

### Community 132 - "ProtocolError"
Cohesion: 0.29
Nodes (6): ControlValveHydraulics, pipe_pressure_drop(), Density, DifferentialPressure, Result, Self

### Community 133 - "ServerConfig"
Cohesion: 0.13
Nodes (11): Result, Result, Result, Result, DoubleBufferedState, BTreeMap, StateBuffer, StateReadView (+3 more)

### Community 134 - "post_sim_control"
Cohesion: 0.43
Nodes (7): get_snapshot(), post_sim_control(), Json, ServerResult, State, StateSnapshotDto, Value

### Community 138 - ".alarms"
Cohesion: 0.25
Nodes (9): RhaiSandbox, AST, BTreeMap, Default, Option, ScriptingResult, Self, ScriptExecutionContext (+1 more)

### Community 139 - "ControlError"
Cohesion: 0.18
Nodes (9): build_alarm_snapshots(), AlarmSnapshotDto, Vec, AlarmSystem, BTreeSet, Self, TagId, Send (+1 more)

### Community 140 - "tag.rs"
Cohesion: 0.31
Nodes (6): CompiledDefinition, CustomEquipmentSystem, Arc, AST, Mutex, Self

### Community 141 - "pid.rs"
Cohesion: 0.21
Nodes (5): ActiveDisturbance, DisturbanceManager, HashMap, Self, Vec

### Community 142 - "audit_causality_pid_actuator_tests.rs"
Cohesion: 0.30
Nodes (5): DeterministicPrng, Default, Self, test_deterministic_repeatability(), test_range_bounds()

### Community 143 - "retention.rs"
Cohesion: 0.25
Nodes (5): Connection, HistorianResult, P, Self, SqliteStorage

### Community 144 - "hydraulics.rs"
Cohesion: 0.24
Nodes (4): ServerHealthDto, TagValueSampleDto, Store, ConnectionState

### Community 145 - "EquipmentInstance"
Cohesion: 0.11
Nodes (18): ProtocolError, ProtocolErrorCode, Into, Option, Self, ClientCapabilities, ClientMessage, AlarmSnapshotDto (+10 more)

### Community 146 - "StreamDefinition"
Cohesion: 0.23
Nodes (3): NebuFaceplate, NebuProcessView, PidModeDto

### Community 147 - ".get_port_definition"
Cohesion: 0.24
Nodes (3): helper_param(), test_stage_a1_three_customization_levels_generic_and_data_driven(), test_stage_a2_runtime_lifecycle_and_multi_instance_isolation()

### Community 148 - "vertical_slice_e2e_tests.rs"
Cohesion: 0.18
Nodes (20): ActiveFaultDto, AlarmSeverityDto, AlarmSnapshotDto, AlarmStateDto, DeltaUpdateDto, EquipmentControlRequestDto, EquipmentDefinitionSummaryDto, EquipmentInstanceSummaryDto (+12 more)

### Community 149 - "AlarmSystem"
Cohesion: 0.22
Nodes (6): 1. Three Levels of Equipment Customization, 2. Future Level 3 Scripting Sandbox Architecture (Phase 10), 3. Package Architecture & Hierarchical Composition, Custom Equipment & Runtime Authoring Architecture, Security & Determinism Invariants, Tier Definitions

### Community 150 - "command.rs"
Cohesion: 0.40
Nodes (13): control_equipment(), define_equipment(), get_definition(), get_instance(), instantiate_equipment(), list_definitions(), list_instances(), Json (+5 more)

### Community 151 - "ServerMessage"
Cohesion: 0.11
Nodes (18): ActiveFaultDto, AlarmSeverityDto, AlarmStateDto, ClientCapabilities, ClientMessage, DeltaUpdateDto, HistoricalEventDto, HistoricalSampleDto (+10 more)

### Community 155 - "AlarmError"
Cohesion: 0.16
Nodes (4): AlarmEngine, BTreeMap, Self, TagId

### Community 158 - "vite.config.ts"
Cohesion: 0.56
Nodes (8): ScenarioResult, validate_disturbance(), validate_event(), validate_fault(), validate_initial_conditions(), validate_objective(), validate_operator_action(), validate_scenario()

### Community 162 - "independent_verification_tests.rs"
Cohesion: 0.29
Nodes (9): create_test_rig(), TagId, test_adversarial_validation_rejects_malformed_definitions(), test_all_12_fault_types_verified(), test_critical_multi_fault_composition_different_targets_and_overlapping_lifetimes(), test_critical_multi_fault_composition_same_tag(), test_disturbance_analytic_boundaries_and_seeded_prng(), test_simulation_time_independence_from_wall_clock() (+1 more)

### Community 165 - "ProtocolError"
Cohesion: 0.20
Nodes (6): AlarmType, AlarmBlockConfig, Self, TagId, AlarmSeverity, AlarmType

### Community 166 - "command.rs"
Cohesion: 0.33
Nodes (8): EquipmentCommand, PidModeDto, BTreeMap, Option, RuntimeCommand, ScenarioCommand, SimControlCommand, TagValueDto

### Community 167 - "nebudcs-historian/src/lib.rs"
Cohesion: 0.36
Nodes (5): prune_events_older_than(), prune_older_than(), RetentionPolicy, Connection, HistorianResult

### Community 168 - "EquipmentInstance"
Cohesion: 0.29
Nodes (6): 1. Overview, 2. Performance Targets, 3. Simulation Tick Budget, 4. Performance Techniques, 5. Monitoring and Benchmarking, Performance Architecture

### Community 169 - "3. Actuator Arbitration & Priority Hierarchy"
Cohesion: 0.40
Nodes (3): EquipmentId, Option, TagId

### Community 174 - "22. Implementation Order (Historical Note & Authoritative Roadmap Reference)"
Cohesion: 0.33
Nodes (6): 22. Implementation Order (Historical Note & Authoritative Roadmap Reference), Initial Pre-Implementation Proposal:, Phase 1 — Core Primitives, Vertical Slice 1 (Phases 2-7 progressive delivery), Vertical Slice 2, Vertical Slice 3

### Community 175 - "Vertical Slice 1: End-to-End Architecture Proof"
Cohesion: 0.33
Nodes (6): 1. The Authoritative Flow of Truth, 2. Dynamic Scenario Description, 3. Required Crates & Subsystems, 4. Acceptance Criteria & Verification Gates, Goal & Purpose, Vertical Slice 1: End-to-End Architecture Proof

### Community 176 - "NebuDCS: Industrial Process Simulation & DCS Training Platform"
Cohesion: 0.33
Nodes (6): Core Product Invariants, Crate Workspace Structure, Current Implementation Baseline, Documentation Index, NebuDCS: Industrial Process Simulation & DCS Training Platform, Quickstart & Headless Verification

## Knowledge Gaps
- **547 isolated node(s):** `AlarmType`, `StateSnapshot`, `RetentionPolicy`, `name`, `version` (+542 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **19 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `String` connect `sim_tests.rs` to `HistorianRunMeta`, `TopologyGraph`, `String`, `retention.rs`, `TopologyError`, `EntityId`, `nebudcs-types/src/error.rs`, `PhysicsError`, `.alarms`, `SchemaVersion`, `StateBuffer`, `pid.rs`, `StreamDefinition`, `EquipmentInstance`, `MassFlow`, `SimulationTime`, `SimulationEngine`, `vertical_slice_e2e_tests.rs`, `command.rs`, `SimulationClock`, `AlarmError`, `StateReadView`, `ProtocolError`, `command.rs`, `AlarmError`, `SimError`, `.alarms`, `Custom Equipment Architecture`, `SimulationTime`, `SimError`, `String`, `nebudcs-types/src/error.rs`, `PipeSegment`, `conservation.rs`, `AlarmEngine`, `NebuDCS: Industrial Process Simulation & DCS Training Platform`, `SignalConnection`, `block.rs`, `pid.rs`, `AlarmBlockConfig`, `.alarms`, `EquipmentInstance`, `nebudcs-alarm/src/engine.rs`, `.new`, `StreamDefinition`, `IdealGasMixture`, `HistorianPointConfig`, `ControlError`, `HistorianPointId`, `AlarmSystem`, `audit_causality_pid_actuator_tests.rs`?**
  _High betweenness centrality (0.345) - this node is a cross-community bridge._
- **Why does `PhysicsError` connect `StreamDefinition` to `ProtocolError`, `sim_tests.rs`, `AlarmBlockConfig`, `Temperature`, `MassFlow`, `Pressure`, `SimulationEngine`, `.solve_vector`?**
  _High betweenness centrality (0.064) - this node is a cross-community bridge._
- **Why does `HostHandle` connect `conservation.rs` to `HistorianRunMeta`, `String`, `retention.rs`, `post_sim_control`, `ControlError`, `nebudcs-alarm/src/engine.rs`, `.subscribe_broadcast`, `mod.rs`, `command.rs`, `SqliteStorage`, `ControlError`, `String`, `tag.rs`, `AlarmSystem`, `audit_causality_pid_actuator_tests.rs`, `AlarmEngine`?**
  _High betweenness centrality (0.036) - this node is a cross-community bridge._
- **What connects `AlarmType`, `StateSnapshot`, `RetentionPolicy` to the rest of the system?**
  _547 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `String` be split into smaller, more focused modules?**
  _Cohesion score 0.1477832512315271 - nodes in this community are weakly interconnected._
- **Should `TopologyError` be split into smaller, more focused modules?**
  _Cohesion score 0.12258064516129032 - nodes in this community are weakly interconnected._
- **Should `conversion.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.052525252525252523 - nodes in this community are weakly interconnected._