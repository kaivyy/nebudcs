# Graph Report - NebuDCS  (2026-09-05)

## Corpus Check
- 87 files · ~59,484 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1354 nodes · 2210 edges · 82 communities (76 shown, 6 thin omitted)
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS · INFERRED: 6 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

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
- EquipmentId
- GenericEquipmentPhysics
- SignalConnection
- graph.rs
- .new
- EquipmentInstance
- nebudcs-types/src/error.rs
- .get_port_definition

## God Nodes (most connected - your core abstractions)
1. `String` - 66 edges
2. `PhysicsError` - 56 edges
3. `TopologyGraph` - 49 edges
4. `NebuDCS` - 40 edges
5. `ComponentComposition` - 38 edges
6. `TopologyError` - 27 edges
7. `NebuDCS Architecture Review Report` - 27 edges
8. `AccumulatorVolumeModel` - 26 edges
9. `GenericEquipmentPhysics` - 26 edges
10. `SimError` - 24 edges

## Surprising Connections (you probably didn't know these)
- `test_network_source_pipe_valve_vessel_dynamic_simulation()` --calls--> `pipe_pressure_drop()`  [INFERRED]
  crates/nebudcs-physics/tests/physics_tests.rs → crates/nebudcs-physics/src/hydraulics.rs
- `test_pipe_pressure_drop_with_elevation_and_reverse_flow()` --calls--> `pipe_pressure_drop()`  [INFERRED]
  crates/nebudcs-physics/tests/physics_tests.rs → crates/nebudcs-physics/src/hydraulics.rs
- `NebuError` --references--> `String`  [EXTRACTED]
  crates/nebudcs-types/src/error.rs → crates/nebudcs-types/src/version.rs
- `PortDefinition` --references--> `String`  [EXTRACTED]
  crates/nebudcs-model/src/equipment.rs → crates/nebudcs-types/src/version.rs
- `make_def()` --references--> `PortDefinition`  [EXTRACTED]
  crates/nebudcs-sim/tests/sim_tests.rs → crates/nebudcs-model/src/equipment.rs

## Import Cycles
- None detected.

## Communities (82 total, 6 thin omitted)

### Community 0 - "TopologyGraph"
Cohesion: 0.20
Nodes (12): PipeEndpoint, PipeSegment, PortRef, EquipmentId, From, Into, Option, PipeSegmentId (+4 more)

### Community 1 - "String"
Cohesion: 0.11
Nodes (36): AlarmDefinitionId, AlarmDefinition, AlarmPriority, AlarmType, Capability, EnvelopeDimension, EquipmentCategory, EquipmentDefinition (+28 more)

### Community 2 - "TopologyError"
Cohesion: 0.06
Nodes (56): Cardinality, PortDefinition, PortDirection, PortDomain, PortId, UtilityKind, StreamType, make_port() (+48 more)

### Community 3 - "conversion.rs"
Cohesion: 0.05
Nodes (24): Power, Self, Speed, test_pressure_bar_conversion(), test_pressure_psi_conversion(), test_round_trip_conversion(), test_round_trip_pressure_psi(), test_speed_round_trip_rpm() (+16 more)

### Community 4 - "System Architecture"
Cohesion: 0.05
Nodes (36): Core Domain Types (Rust Pseudocode), Domain Hierarchy, Domain Model, Engineering Unit System, Full Domain Entity-Relationship Diagram, State Separation, Tag System Design, Crate Workspace Overview (+28 more)

### Community 5 - "EntityId"
Cohesion: 0.10
Nodes (24): Into, Option, Self, test_validation_error_is_blocking(), test_validation_result_serde(), test_validation_result_with_entity(), test_validation_warning_not_blocking(), ValidationResult (+16 more)

### Community 6 - "NebuDCS Architecture Review Report"
Cohesion: 0.06
Nodes (34): 10. Solver/Convergence Assessment, 11. Control/Protection Assessment, 12. HMI/P&ID Assessment, 13. Protocol Assessment, 14. Versioning Assessment, 15. Security Review, 16. Dependency Review, 17. Performance Strategy (+26 more)

### Community 7 - "sim_tests.rs"
Cohesion: 0.12
Nodes (10): Send, SimulationSystem, FailingSystem, make_def(), EquipmentId, Vec, SystemA, SystemB (+2 more)

### Community 8 - "ComponentComposition"
Cohesion: 0.12
Nodes (6): Phase, PhysicalState, Density, SpecificEnthalpy, StreamId, StreamPhysicalState

### Community 9 - "PhysicsError"
Cohesion: 0.20
Nodes (16): AreaId, Area, build_project(), Plant, Project, ProjectMetadata, Into, Self (+8 more)

### Community 10 - "Protection System Architecture"
Cohesion: 0.06
Nodes (28): 1. Authoritative Control Pipeline, 2. Execution Pipeline, 3. Control Primitives, 4. Control Loop Faceplate Data Model, 5. PLC Ladder-Style Logic Interface (Conceptual), Advanced Logic, Cascade Control, Control Engine Architecture (+20 more)

### Community 11 - "NebuDCS"
Cohesion: 0.06
Nodes (30): 10. Process Streams, 11. Utilities, 12. Control System, 13. Tag System, 16. Alarm System, 17. Historian, 18. Fault Injection, 1. Product Vision (+22 more)

### Community 12 - "SchemaVersion"
Cohesion: 0.11
Nodes (14): Display, Formatter, NebuResult, Option, Result, Self, SchemaVersion, test_schema_version_display() (+6 more)

### Community 13 - "StateBuffer"
Cohesion: 0.17
Nodes (8): DoubleBufferedState, EquipmentState, BTreeMap, EquipmentId, Into, Option, Self, StateBuffer

### Community 14 - "physics_tests.rs"
Cohesion: 0.05
Nodes (11): churchill_friction_factor(), ControlValveHydraulics, pipe_pressure_drop(), Density, DifferentialPressure, Result, Self, test_darcy_weisbach_churchill_friction_laminar() (+3 more)

### Community 15 - "Project Format Architecture"
Cohesion: 0.08
Nodes (23): 1. Overview, 2. What a Project Contains, 3. Project File Format Recommendation, 4. Schema Versioning & Migrations, 5. Dependency Resolution, 6. Deterministic Reproduction, 7. SQLite Schema Outline (Conceptual), 8. Architecture Diagrams (+15 more)

### Community 16 - "StreamDefinition"
Cohesion: 0.14
Nodes (13): make_stream(), PortRef, EquipmentId, IndexMap, Into, PortId, Self, StreamComposition (+5 more)

### Community 17 - "SimulationTime"
Cohesion: 0.15
Nodes (11): Self, TagId, t0(), TagSnapshot, test_tag_quality_good_and_bad(), test_tag_snapshot_serde(), Default, Display (+3 more)

### Community 18 - "MassFlow"
Cohesion: 0.21
Nodes (8): PhaseEquilibrium, Density, Phase, Result, SpecificEnthalpy, SaturatedFluidPackage, Pressure, Temperature

### Community 19 - "Pressure"
Cohesion: 0.16
Nodes (4): AccumulatorVolumeModel, Density, Option, SpecificEnthalpy

### Community 20 - "SimulationEngine"
Cohesion: 0.15
Nodes (9): Box, EquipmentId, Option, Result, Self, TagId, Vec, SimulationEngine (+1 more)

### Community 21 - "SimulationClock"
Cohesion: 0.10
Nodes (10): ClockState, Result, Self, SimulationClock, NebuError, EquipmentId, From, Self (+2 more)

### Community 22 - "roadmap.md"
Cohesion: 0.12
Nodes (16): Phase 0: Repository Foundation, Phase 10: Custom Equipment Proof, Phase 11: Standard Equipment Library, Phase 12: Advanced Physics and Thermodynamics, Phase 13: Refinery Unit Templates, Phase 14: Desktop Engineering Station, Phase 15: Hardening and Optimization, Phase 1: Core Types and Domain Model (+8 more)

### Community 23 - "ExecutionPlan"
Cohesion: 0.19
Nodes (5): ExecutionBlock, ExecutionPlan, EquipmentId, Self, Vec

### Community 24 - "Equipment SDK Architecture"
Cohesion: 0.13
Nodes (14): 1. Separation of Concerns, 2. Capability Trait System, 3. Port Definition, 4. Performance Curves & Operating Envelopes, 5. Equipment State Machine, 6. Instrument & Actuator Dynamics, 7. Standard Equipment Library Architecture, 8. Feasibility Examples (+6 more)

### Community 25 - "HMI & P&ID Architecture"
Cohesion: 0.13
Nodes (14): 1. Core Philosophy: Three Independent Concerns, 2. P&ID Editor Capabilities, 3. HMI Capabilities, 4. Faceplate Data Model, 5. Desktop vs. Web Architecture, 6. Objective Evaluation of Web Frameworks, 7. Architecture Diagrams, Desktop Application Architecture (+6 more)

### Community 26 - "Protocol Architecture"
Cohesion: 0.13
Nodes (14): 1. Overview, 2. Communication Paradigms, 3. Performance & Optimization, 4. Message Types (WebSocket), 5. Versioning and Capability Negotiation, 6. Binary Protocol Consideration, 7. Reconnect and Resynchronization, 8. Security (+6 more)

### Community 27 - "TagValue"
Cohesion: 0.24
Nodes (7): TagQuality, TagValue, F, Result, TagId, StateReadView<'a>, StateWriteView<'a>

### Community 28 - ".solve_vector"
Cohesion: 0.20
Nodes (9): RecycleSolver, Default, F, Result, Self, Vec, SolverConfig, SolverReport (+1 more)

### Community 29 - "StateReadView"
Cohesion: 0.22
Nodes (7): Result, StateReadView, StateWriteView, StepContext, Result, test_double_buffer_isolation_proof(), test_failing_system_does_not_commit_state()

### Community 30 - "time.rs"
Cohesion: 0.22
Nodes (10): NebuResult, Self, test_advance_panics_on_negative_dt(), test_simulation_time_display(), test_simulation_time_monotonic(), test_simulation_time_positive(), test_simulation_time_rejects_nan(), test_simulation_time_rejects_negative() (+2 more)

### Community 31 - "2. Historian Architecture"
Cohesion: 0.15
Nodes (12): 1. Alarm Engine, 2. Historian Architecture, Alarm Engine & Historian Architecture, Alarm Lifecycle State Machine, Alarm Suppression & Shelving, Alarm Types, Data Pipeline & Downsampling, Replay System (+4 more)

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
Cohesion: 0.17
Nodes (11): ADR-009: Crate Architecture and Boundaries, Alternatives Considered, Consequences, Context, Decision, Future Reconsideration Criteria, Problem, Rationale (+3 more)

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
Cohesion: 0.14
Nodes (8): BTreeMap, BTreeSet, EquipmentDefinitionId, Option, PipeSegmentId, SignalConnectionId, StreamId, TopologyGraph

### Community 45 - "phase-01-foundation.md"
Cohesion: 0.18
Nodes (10): CARGO.TOML CONTENT, CRATE-BY-CRATE SPECIFICATION, EXIT CRITERIA, EXPLICITLY EXCLUDED, GOAL, `nebudcs-model`, `nebudcs-types`, `nebudcs-units` (+2 more)

### Community 46 - "Temperature"
Cohesion: 0.09
Nodes (16): EnergyBalance, EnthalpyAccumulationInputs, MassBalance, MassBalanceResult, Result, SpecificEnthalpy, SpeciesBalance, EquipmentExecutionContext (+8 more)

### Community 47 - "pipe_pressure_drop"
Cohesion: 0.17
Nodes (11): GasComponentProperty, IdealGasMixture, IncompressibleLiquid, PropertyPackage, BTreeMap, Into, Self, Send (+3 more)

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
Cohesion: 0.20
Nodes (9): Allowed Dependencies, Component Classification Table, Concrete Dependency Graph, Crate Assignment Table, Cycle Risks, Forbidden Dependencies, Ownership Boundaries, Serialization Boundaries (+1 more)

### Community 52 - "testing-strategy.md"
Cohesion: 0.20
Nodes (9): ARCHITECTURE TESTS, CI PIPELINE DEFINITION, DETERMINISM TESTS, FIXTURE FILES, INTEGRATION TESTS, PERFORMANCE TESTS, PHASE 1 TESTS, PHILOSOPHY (+1 more)

### Community 53 - "vertical-slice-01.md"
Cohesion: 0.20
Nodes (9): CUSTOM EQUIPMENT PROOF SPECIFICATION, EQUIPMENT DEFINITIONS REQUIRED, GOAL, NEW CRATES REQUIRED, SECOND PROOF SPECIFICATION, SIMULATION SCENARIO, SLICE ACCEPTANCE CRITERIA, SLICE PREREQUISITES (+1 more)

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
Cohesion: 0.29
Nodes (6): 1. Three Levels of Customization, 2. Package Architecture, 3. Scripting Sandbox Design, 4. Package Registry & Trust Boundaries, 5. Feasibility Example 4: Custom Equipment, Custom Equipment Architecture

### Community 58 - "Performance Architecture"
Cohesion: 0.29
Nodes (6): 1. Overview, 2. Performance Targets, 3. Simulation Tick Budget, 4. Performance Techniques, 5. Monitoring and Benchmarking, Performance Architecture

### Community 59 - "7. Equipment Parameters"
Cohesion: 0.29
Nodes (7): 7. Equipment Parameters, Compressor, Fin Fan, Heat Exchanger, Pump, Turbine, Vessel

### Community 60 - "Solver Architecture"
Cohesion: 0.33
Nodes (5): 1. Abstractions, 2. Convergence State Machine, 3. Simulation Execution Loop, 4. Recycle & Initialization, Solver Architecture

### Community 61 - "Thermodynamics Architecture"
Cohesion: 0.33
Nodes (5): 1. Pluggable Property Package, 2. Component Registry & Composition, 3. Hydrogen Networks, 4. Phase Equilibrium & Evolution, Thermodynamics Architecture

### Community 62 - "NebuDCS Implementation Blueprint"
Cohesion: 0.33
Nodes (6): How a coding agent should use these documents, Implementation Philosophy, Navigation Table, NebuDCS Implementation Blueprint, Relation to Architecture Docs and PRD, What this is and how to use it

### Community 63 - "Physics Architecture"
Cohesion: 0.40
Nodes (4): 1. Fidelity Levels, 2. Core Balances & Hydraulics, 3. Specific Models, Physics Architecture

### Community 64 - "implementation/README.md"
Cohesion: 0.40
Nodes (3): Architecture Consistency Audit, PRD Delta, Requirements Traceability Matrix

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

### Community 74 - "EquipmentId"
Cohesion: 0.28
Nodes (3): EquipmentId, Self, Vec

### Community 75 - "GenericEquipmentPhysics"
Cohesion: 0.07
Nodes (19): C, EquipmentCapability, GenericEquipmentPhysics, Box, BTreeMap, EquipmentId, Into, Self (+11 more)

### Community 76 - "SignalConnection"
Cohesion: 0.19
Nodes (10): EquipmentId, Into, Option, PortId, Self, SignalConnectionId, TagId, SignalConnection (+2 more)

### Community 77 - "graph.rs"
Cohesion: 0.29
Nodes (7): IndexMap, find_strongly_connected_components(), BTreeMap, BTreeSet, EquipmentId, Vec, SccAnalysisResult

### Community 78 - ".new"
Cohesion: 0.27
Nodes (7): ComponentComposition, CompositionBasis, BTreeMap, IndexMap, Into, Result, Self

### Community 79 - "EquipmentInstance"
Cohesion: 0.20
Nodes (9): 1. Architectural Principles, 2.1 State & Composition (`state`), 2.2 Property Packages (`properties`), 2.3 Hydraulics & Flow Elements (`hydraulics`), 2.4 Conservation Laws (`conservation`), 2.5 Solvers (`solver`), 2.6 Equipment Capability Framework (`equipment` & `system`), 2. Modules & Governing Physics (+1 more)

### Community 81 - ".get_port_definition"
Cohesion: 0.25
Nodes (6): Option, PortRef, StreamId, StreamDefinition, PortId, Result

## Knowledge Gaps
- **440 isolated node(s):** `Generic Process Simulation Platform & DCS/PLC Training Environment`, `1. Product Vision`, `2. Core Architectural Principle`, `Core`, `Desktop` (+435 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **6 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `String` connect `pipe_pressure_drop` to `TopologyGraph`, `String`, `TopologyError`, `EntityId`, `PhysicsError`, `GenericEquipmentPhysics`, `SignalConnection`, `StateBuffer`, `Temperature`, `.new`, `StreamDefinition`, `.get_port_definition`, `MassFlow`, `nebudcs-types/src/error.rs`, `SchemaVersion`, `SimulationClock`, `TagValue`?**
  _High betweenness centrality (0.242) - this node is a cross-community bridge._
- **Why does `PhysicsError` connect `Temperature` to `ComponentComposition`, `GenericEquipmentPhysics`, `physics_tests.rs`, `pipe_pressure_drop`, `.new`, `MassFlow`, `.solve_vector`?**
  _High betweenness centrality (0.078) - this node is a cross-community bridge._
- **Why does `TopologyGraph` connect `SimError` to `TopologyGraph`, `String`, `sim_tests.rs`, `EquipmentId`, `SignalConnection`, `graph.rs`, `.get_port_definition`, `ExecutionPlan`?**
  _High betweenness centrality (0.059) - this node is a cross-community bridge._
- **What connects `Generic Process Simulation Platform & DCS/PLC Training Environment`, `1. Product Vision`, `2. Core Architectural Principle` to the rest of the system?**
  _440 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `String` be split into smaller, more focused modules?**
  _Cohesion score 0.10796221322537113 - nodes in this community are weakly interconnected._
- **Should `TopologyError` be split into smaller, more focused modules?**
  _Cohesion score 0.06144393241167435 - nodes in this community are weakly interconnected._
- **Should `conversion.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.052597402597402594 - nodes in this community are weakly interconnected._