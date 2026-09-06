---
title: NebuDCS Implementation Roadmap
version: 2.0.0
status: verified-baseline
date: 2026-09-06
---

# NebuDCS Implementation Roadmap

This roadmap defines the definitive 17-phase lifecycle (Phases 0 through 16) for the NebuDCS platform. It is strictly dependency-ordered, architectural-traceability verified, and anchored to the proven baseline of Phases 0–4.

---

## Roadmap Overview & Status Matrix

| Phase | Title | Crate(s) / Targets | Status | Primary Focus |
|---|---|---|---|---|
| **Phase 0** | Repository & Foundation Setup | Workspace root, `.github/workflows` | **COMPLETE** | Cargo workspace, CI, pinned dependencies, zero warnings |
| **Phase 1** | Domain Model & Physical Types | `nebudcs-types`, `nebudcs-units`, `nebudcs-model` | **COMPLETE** | ULID EntityIds, strongly typed quantities, equipment definitions |
| **Phase 2** | Topology, Ports & Graph Analysis | `nebudcs-topology` | **COMPLETE** | Semantic ports, streams, piping, SCC cycles, validation |
| **Phase 3** | Simulation Runtime & Execution | `nebudcs-sim` | **COMPLETE** | Deterministic clock, fixed $\Delta t$, double-buffering, execution plan |
| **Phase 4** | Process Physics & Conservation | `nebudcs-physics` | **PHASE 4 VERIFIED** | Conservation laws, hydraulics, VLE, recycle solving, safeguards |
| **Phase 5** | Control Engine & Automation | `nebudcs-control`, `nebudcs-alarm` | **PHASE 5 VERIFIED** | PID (P/PI/PID, bumpless, anti-windup, filter), logic, alarms |
| **Phase 6** | Historian & Timeseries Persistence | `nebudcs-historian` | **PHASE 6 VERIFIED** | SQLite WAL, non-blocking sampling, bounded ring-buffer, downsampling |
| **Phase 7** | Realtime Protocol & API Server | `nebudcs-protocol`, `apps/nebudcs-server` | **PHASE 7 VERIFIED** | Axum host, WebSocket delta subscription, REST API |
| **Phase 8** | End-to-End Vertical Slice 1 | Full Stack (Simulation to UI) | **PHASE 8 VERIFIED** | Equipment → Port → Instrument → Controller → Actuator → Physics → State → Tag → Protocol → Web/Desktop UI → P&ID/HMI → Faceplate → Alarm/Trend |
| **Phase 9** | Scenario & Fault Injection Engine | `nebudcs-scenario` | **PHASE 9 VERIFIED** | Data-driven scenarios, timeline events, physical fault injection |
| **Phase 10** | Custom Equipment & Runtime Authoring | `nebudcs-scripting` | **`PHASE 10 VERIFIED`** | Level 1/2/3 customization, sandboxed Rhai, no core recompile |
| **Phase 11** | Standard Equipment Library | Template Packages | **PLANNED** | Pumps, compressors, exchangers, columns, valves, furnaces |
| **Phase 12** | Advanced Physics & Pluggable Thermo | `nebudcs-physics` (extensions) | **PLANNED** | Peng-Robinson, SRK, CoolProp, multi-tube, cavitation, DAE |
| **Phase 13** | Engineering Studio Application | `apps/nebudcs-desktop` / Web Studio | **PLANNED** | Equipment Library → Navigator → P&ID Canvas → Inspector |
| **Phase 14** | Operator / DCS Station Console | Web Operator Console / Desktop | **PLANNED** | Plant Navigator + Process Graphic + Alarm Summary + Faceplates |
| **Phase 15** | Training & Scenario Studio | Training Subsystem | **PLANNED** | Instructor station, assessment rubrics, scoring, session replay |
| **Phase 16** | Production Hardening & Deployment | Production targets, Tailscale | **PLANNED** | 1000+ equipment scaling, zero-allocation loop, remote access |

---

## Detailed Phase Specifications

### Phase 0: Repository & Foundation Setup
- **Purpose**: Establish an immaculate, deterministic Rust workspace with strict CI quality gates.
- **Scope**: Root `Cargo.toml`, workspace dependency pinning, rustfmt, clippy deny-warnings, headless CI.
- **Dependencies**: None.
- **Non-Goals**: Writing domain logic or premature architectural abstractions.
- **Architectural Outputs**: Workspace configuration, `.gitignore`, CI pipeline.
- **Acceptance Criteria**: Headless execution, zero warnings, `cargo check` and `cargo test` pass cleanly.
- **Verification Requirements**: Automated CI check runs on clean environment.
- **Upstream / Downstream**: Upstream to all phases.

### Phase 1: Domain Model & Physical Types
- **Purpose**: Create the irreducible, immutable type foundation for identifiers, physical quantities, and data-driven equipment schemas.
- **Scope**:
  - `nebudcs-types`: ULID-based `EntityId`, strongly typed ID newtypes, `SimulationTime`, `SchemaVersion`, `NebuError`.
  - `nebudcs-units`: Strongly typed physical quantities (`Pressure`, `Temperature`, `MassFlow`, `Density`, `Level`, etc.), canonical SI internal representation, display unit conversion, dimensional safety.
  - `nebudcs-model`: DTO hierarchy (`Project`, `Plant`, `Area`, `Unit`), `EquipmentDefinition` schema, `EquipmentInstance`, `StreamDefinition`, `TagValue`, `TagQuality`, `ValidationResult`.
- **Dependencies**: Phase 0.
- **Non-Goals**: Graph topology calculations, simulation loop, physics calculations, UI rendering.
- **Architectural Outputs**: `crates/nebudcs-types`, `crates/nebudcs-units`, `crates/nebudcs-model`.
- **Acceptance Criteria**: Serde roundtrip tests for all definitions; dimensional validation; zero interchangeable IDs.
- **Verification Requirements**: Unit tests, property tests, serde roundtrip tests (49 passing tests).
- **Upstream / Downstream**: Consumed by Phase 2 (Topology) and all subsequent crates.

### Phase 2: Topology, Ports & Graph Analysis
- **Purpose**: Implement a generic, graph-based process and signal topology engine.
- **Scope**:
  - Semantic ports: `PortDomain` (Material, Utility, Electrical, Mechanical, Signal, Energy), `PortDirection`, `PortMedia`, phase compatibility, cardinality validation.
  - Process piping connections and signal network connections.
  - Graph query API: downstream/upstream neighbors, path resolution.
  - Tarjan's Strongly Connected Components (SCC) algorithm for detecting recycle loops and cycles.
  - Deterministic topological ordering for simulation execution planning.
- **Dependencies**: Phase 1 (`nebudcs-types`, `nebudcs-units`, `nebudcs-model`).
- **Non-Goals**: Fluid flow rates, pressure drops, valve opening dynamics, visual layout coordinates.
- **Architectural Outputs**: `crates/nebudcs-topology`.
- **Acceptance Criteria**: Accurate validation of connection constraints; deterministic topological sort; cycle detection.
- **Verification Requirements**: Headless graph validation tests, cycle detection tests, connection validation tests (21 passing tests).
- **Upstream / Downstream**: Consumes Phase 1; consumed by Phase 3 (Simulation Runtime) and Phase 4 (Physics).

### Phase 3: Simulation Runtime & Execution Pipeline
- **Purpose**: Build the deterministic simulation execution infrastructure with fixed timestep and double-buffered state isolation.
- **Scope**:
  - `SimulationClock`: Fixed $\Delta t$ (e.g., 10–20ms), monotonic tick counter, play/pause/step, speed multiplier (without changing physics $\Delta t$).
  - Double-buffered `RuntimeState`: Current read-buffer vs. next write-buffer, atomic buffer swap on tick completion.
  - `ExecutionPlan`: Generates deterministic stage execution order from topology (acyclic stages + recycle SCC stages).
  - `SimulationSystem` trait and tick coordinator.
  - Tag snapshot engine providing immutable point-in-time reads for UI/historian.
- **Dependencies**: Phase 1, Phase 2.
- **Non-Goals**: Process physics, differential equation solving, control algorithms, WebSocket server, UI.
- **Architectural Outputs**: `crates/nebudcs-sim`.
- **Acceptance Criteria**: Bit-identical deterministic execution across repeated runs; strict isolation between read and write buffers; rollback on system failure.
- **Verification Requirements**: Deterministic repeatability tests, double-buffer isolation tests, execution plan tests (10 passing tests).
- **Upstream / Downstream**: Consumes Phase 1 & 2; consumed by Phase 4 (Physics) and Phase 5 (Control).

### Phase 4: Generic Process Physics & Conservation Foundation
- **Status**: **`PHASE 4 VERIFIED`**
- **Purpose**: Provide the mathematical and thermodynamic foundation for process simulation using generic conservation laws and hydraulic solvers.
- **Scope**:
  - Conservation laws: Total mass balance, multicomponent species balance, enthalpy balance, dynamic inventory accumulator (`AccumulatorVolumeModel`).
  - Thermodynamics: Pluggable `PropertyPackage` trait, `IdealGasPackage`, `IncompressibleLiquidPackage`, `SaturatedFluidPackage` (Clausius-Clapeyron VLE, flash from enthalpy, saturation temperature).
  - Hydraulics: Darcy-Weisbach pipe pressure drop with Churchill friction factor (laminar, transitional, turbulent), elevation head $\rho g \Delta z$, signed bidirectional flow, ISA-style control valve hydraulics ($C_v$, forward/reverse, liquid/vapor choked flow).
  - Nonlinear solver: `RecycleSolver` (Successive substitution, Wegstein acceleration, damping factor $\theta \in (0, 1]$, divergence detection, mixed absolute/relative tolerances).
  - Composable equipment capabilities: `EquipmentCapability` enum, `GenericEquipmentPhysics` (pumps, valves, vessels, heat exchangers, pipes).
  - Numerical safeguards: Strict clamping against non-positive temperatures, negative pressures, negative masses, NaN/Inf poisoning; robust error handling via `PhysicsError`.
- **Engineering-Fidelity Assumptions & Limitations**:
  - Assumes simplified mixture thermodynamics (ideal gas or pseudo-single-component saturated fluid approximation via Clausius-Clapeyron).
  - Not an industrial-grade cubic equation of state (Peng-Robinson, SRK, and activity models are deferred to Phase 12).
  - Choked gas flow through valves uses simplified pressure-ratio cutoffs ($P_{ratio} \le 0.5$).
  - Dynamic inventory uses lumped-parameter equilibrium without multi-stage spatial discretization.
- **Dependencies**: Phases 1, 2, 3 (`nebudcs-types`, `nebudcs-units`, `nebudcs-model`, `nebudcs-topology`, `nebudcs-sim`).
- **Non-Goals**: PID controllers, alarms, historian, UI rendering, refinery unit templates, runtime scripting.
- **Architectural Outputs**: `crates/nebudcs-physics`.
- **Acceptance Criteria**: 100% mass, species, and energy conservation across closed systems; monotonic valve curves; stable recycle loop convergence; full integration with `SimulationSystem`.
- **Verification Requirements**: 40 unit and integration tests passing; independent physics and numerical audit completed and approved.
- **Upstream / Downstream**: Consumes Phases 1–3; foundation for Phase 5 (Control Engine) and Phase 8 (Vertical Slice).

### Phase 5: Control Engine & Automation
- **Status**: **`PHASE 5 VERIFIED`**
- **Purpose**: Implement a generic, deterministic DCS/PLC-style control engine executing within the authoritative simulation pipeline.
- **Scope**:
  - Control primitives: PV (Process Variable), SP (Setpoint), MV/OUT (Manipulated Variable), control signals.
  - Algorithms: P, PI, and PID implementations with configurable parameters ($K_p$, $T_i$, $T_d$, filtering coefficient $\alpha$).
  - Operational modes: MANUAL, AUTO, CASCADE (CAS), REMOTE CASCADE (RCAS).
  - Bumpless transfer: Smooth transitions between MANUAL and AUTO; SP tracking in MANUAL; output tracking.
  - Integral handling & anti-windup: Clamping, back-calculation, or conditional integration to prevent integrator saturation at output limits ($OUT_{min}, OUT_{max}$).
  - Derivative filtering: Low-pass filtered derivative action on PV (not error) to prevent derivative kick.
  - Scan rates & deterministic ordering: Configurable execution scan rates (e.g., 10 Hz vs 50 Hz physics); deterministic execution plan based on signal dependency graph.
  - Signal quality integration: Propagation of `TagQuality` (Good, Bad, Uncertain); controller action on signal failure (fail-to-manual, hold last value, or trip to safe state).
  - Logic blocks & interlocks: Boolean gates (AND, OR, NOT, XOR), timers (TON, TOF, TP), latches (SR, RS), permissives, cause-and-effect interlock matrix.
  - Actuator interaction: Realistic valve travel time, rate limits, deadband, hysteresis, and fail-safe position (Fail Open, Fail Closed, Fail In Place).
  - Alarm engine: `nebudcs-alarm` with limit alarms (HI, HIHI, LO, LOLO), deviation alarms, rate-of-change alarms, bad-quality alarms, alarm state machine (normal, active, acknowledged, cleared, shelved).
- **Dependencies**: Phases 1, 2, 3, 4 (`nebudcs-types`, `nebudcs-units`, `nebudcs-model`, `nebudcs-topology`, `nebudcs-sim`, `nebudcs-physics`).
- **Explicit Non-Goals**:
  - No frontend UI, faceplate rendering, or visual editors.
  - No database or SQLite historian writes.
  - No advanced physics modifications.
  - No runtime Rhai scripting or DSL execution.
  - No hardcoded refinery control loops (NHT, CDU, etc.).
- **Architectural Outputs**: `crates/nebudcs-control`, `crates/nebudcs-alarm`.
- **Acceptance Criteria**: Bumpless mode transfers with zero output discontinuities; anti-windup clamping under sustained saturation; filtered derivative response with zero kick on SP step; deterministic execution ordering; robust alarm state transitions.
- **Verification Requirements**: Headless unit and integration test suite; property tests for bumpless transfer and anti-windup; signal quality failure tests.
- **Upstream / Downstream**: Consumes Phases 1–4; consumed by Phase 6 (Historian), Phase 7 (Protocol), and Phase 8 (Vertical Slice).

### Phase 6: Historian & Timeseries Persistence
- **Status**: **`PHASE 6 VERIFIED`**
- **Purpose**: Implement high-throughput, non-blocking asynchronous recording of tag values, alarms, operator events, and scenario markers.
- **Scope**:
  - Asynchronous write pipeline: Ring-buffer queue decoupled from the simulation hot loop.
  - Storage engine: SQLite-based local persistence with optimized schema, indexes, and WAL mode.
  - Sampling strategies: Periodic sampling, on-change sampling, deadband filtering (e.g., boxcar or swinging-door compression).
  - Query API: Time-range queries, downsampling/decimation for UI trends (Min/Max/Average buckets).
  - Replay log foundation: Recording simulation input events to enable deterministic historical playback.
- **Dependencies**: Phases 1, 3, 5 (`nebudcs-types`, `nebudcs-units`, `nebudcs-sim`, `nebudcs-alarm`).
- **Non-Goals**: Remote cloud time-series databases, WebSocket streaming, UI charts.
- **Architectural Outputs**: `crates/nebudcs-historian`.
- **Acceptance Criteria**: Zero allocations or blocking locks in the simulation tick loop ($\le 50\ \mu\text{s}$ sampling latency); sustained background write throughput $\ge 100,000$ samples/second; accurate bucket-aggregated query results preserving min/max spikes (see [Phase 6 Specification](./phase-06-historian.md)).
- **Verification Requirements**: 24-point verification test matrix; benchmarks under simulated load; SQLite WAL integrity tests.
- **Upstream / Downstream**: Consumes Phase 5; consumed by Phase 7 (Protocol), Phase 8 (Vertical Slice), and Phase 9 (Scenarios).

### Phase 7: Realtime Protocol & API Server
- **Status**: **`PHASE 7 VERIFIED`**
- **Purpose**: Provide lightweight, low-latency client-server communication between the headless simulation core and remote/local interfaces.
- **Scope**:
  - Network DTOs: Versioned serialization types decoupled from internal simulation structs (`crates/nebudcs-protocol`).
  - Headless host: `apps/nebudcs-server` built on Axum / Tokio with dedicated background deterministic simulation loop.
  - Realtime transport: WebSocket server supporting tag subscriptions, dirty-state tracking, batched delta updates (never transmitting entire plant state per frame).
  - REST API: Project lifecycle, simulation controls (play, pause, step, speed multiplier), tag writes, alarm ack/shelve, protection reset, historian proxy queries.
  - Connection resiliency: Version negotiation, heartbeat/ping-pong, automatic reconnect and state resynchronization.
- **Dependencies**: Phases 1, 3, 5, 6 (`nebudcs-types`, `nebudcs-units`, `nebudcs-model`, `nebudcs-sim`, `nebudcs-control`, `nebudcs-alarm`, `nebudcs-historian`).
- **Non-Goals**: Frontend visual rendering, desktop GUI, third-party message brokers (RabbitMQ, Kafka), microservices.
- **Architectural Outputs**: `crates/nebudcs-protocol`, `apps/nebudcs-server`.
- **Acceptance Criteria**: Sub-50ms roundtrip latency over WebSocket; delta updates consume minimal bandwidth; graceful client disconnect/reconnect handling.
- **Verification Requirements**: Headless protocol integration tests, client-server handshake tests, delta-update verification tests (10 server tests passing, 5 protocol tests passing, 3 performance benchmarks passing).
- **Upstream / Downstream**: Consumes Phases 1–6; consumed by Phase 8 (Vertical Slice) and UI environments.

### Phase 8: End-to-End Vertical Slice 1
- **Status**: **`PHASE 8 VERIFIED`**
- **Purpose**: Validate and prove the entire end-to-end architecture across every layer of the platform from process physics to live user interface.
- **Scope**:
  - Closed-loop dynamic scenario: Centrifugal pump feeding a liquid accumulator vessel through an automated control valve.
  - The Full Authoritative Pipeline:
    $$\text{Equipment} \to \text{Process Port} \to \text{Instrument} \to \text{Controller} \to \text{Actuator} \to \text{Physics} \to \text{Simulation State} \to \text{Tag} \to \text{Protocol} \to \text{Web/Desktop UI} \to \text{P\&ID/HMI} \to \text{Faceplate} \to \text{Alarm/Trend}$$
  - UI client: Minimal web-based operator faceplate and SVG graphic bound to live WebSocket deltas.
  - Zero fake frontend simulation: All displayed values, animations, and alarms originate directly from the authoritative Rust simulation state.
  - Closed-loop disturbance: Vessel level setpoint step change; pump shutdown fault causing level drop; LO alarm activation; PID valve manipulation.
- **Dependencies**: Phases 0 through 7.
- **Non-Goals**: Full multi-unit refinery modeling, extensive graphic editors, advanced graphics styling.
- **Architectural Outputs**: Full-stack integration test suite, verified demonstration fixture, vertical slice test harness, TypeScript Web Components client (`web/`).
- **Acceptance Criteria**: PID controller successfully regulates vessel level; pump trip cascades to level drop and alarm; WebSocket delta updates update UI within 100ms; zero data drift between core and UI.
- **Verification Requirements**: Automated end-to-end headless test (3 integration tests passing); browser automated verification test (27 Vitest tests passing).
- **Upstream / Downstream**: Validates Phases 0–7; unblocks Phases 9–16.

### Phase 9: Scenario & Fault Injection Engine
- **Status**: **`PHASE 9 VERIFIED`**
- **Purpose**: Implement a deterministic, data-driven scenario and dynamic fault-injection engine for operator training and plant upset studies.
- **Scope**:
  - Scenario schema: Declarative definition containing initial condition snapshots, time-based events, condition-based triggers, fault injections, and objectives.
  - Fault injection subsystem: Dynamic physical faults applied directly to model entities (e.g., pump mechanical seizure, valve stuck-at-position, pipe fouling/blockage, sensor drift/bias/loss-of-signal, heat exchanger tube leak).
  - Instructor controls: Pause, freeze, resume, fast-forward, manual ad-hoc fault injection.
  - Assessment hooks: Tracking operator responses, alarm acknowledge latency, constraint violations, and time-to-recovery.
  - Deterministic replay: Historical playback of entire training sessions using recorded inputs and snapshots.
- **Dependencies**: Phases 1 through 8 (`nebudcs-types`, `nebudcs-model`, `nebudcs-sim`, `nebudcs-physics`, `nebudcs-control`, `nebudcs-alarm`, `nebudcs-historian`).
- **Non-Goals**: UI graphical scenario authoring (belongs to Phase 15), hardcoded scenario movies.
- **Architectural Outputs**: `crates/nebudcs-scenario`.
- **Acceptance Criteria**: Faults alter actual physical calculations rather than superficial UI flags; replay produces bit-identical state trajectories.
- **Verification Requirements**: Headless scenario execution tests, fault cascade verification tests, replay determinism tests.
- **Upstream / Downstream**: Consumes Phases 1–8; consumed by Phase 15 (Training Studio).

### Phase 10: Custom Equipment & Runtime Authoring
- **Status**: **`PHASE 10 VERIFIED`**
- **Purpose**: Enable Level 1, 2, and 3 custom equipment authoring from data without recompiling the Rust simulation core.
- **Scope**:
  - Level 1: Parameter configuration of existing templates via TOML/JSON, 1D/2D performance curves, and operating envelopes.
  - Level 2: Composition of existing capabilities and equipment subgraphs into reusable packages.
  - Level 3: Custom dynamic behavioral equations using sandboxed Rhai scripting.
  - Security & Sandbox: Strict capability whitelisting; instruction counting (strictly enforced 50,000 instruction limit per tick); call level depth limits (16); array/string/map bounds; complete denial of OS, filesystem, network, and process access.
  - Deterministic execution: Sandboxed scripts take pure inputs and produce deterministic, bit-identical outputs with no external side-effects.
  - Pre-execution validation: Rigorous syntax, AST, non-finite float rejection, parameter bounds, curve monotonicity, and dry-run script execution before runtime admission.
  - REST & WebSocket API: Complete endpoints for equipment definition, pre-execution validation, instantiation, parameter overrides, and runtime control (start, stop, trip reset, parameter mutation).
- **Dependencies**: Phases 1 through 5, Phase 7.
- **Non-Goals**: Exposing raw Rust pointers, dynamic C FFI loading, unconstrained Python interpreters, refinery-specific hardcoded structs.
- **Architectural Outputs**: `crates/nebudcs-scripting`, `nebudcs-protocol` equipment DTOs/commands, `apps/nebudcs-server` equipment routes.
- **Acceptance Criteria**: Custom equipment with nonlinear curves and Rhai behavior executes purely from runtime data without recompiling the Rust core; runaway scripts are terminated cleanly by the 50,000 instruction limit; non-finite floats and security breakouts are prevented.
- **Verification Requirements**: 26 unit, integration, and independent verification tests in `nebudcs-scripting` (curves, sandbox limits, security penetration, causal propagation, determinism, lifecycle isolation, subgraphs, no-recompile proof) and 5 end-to-end REST integration tests in `apps/nebudcs-server` (100% passing).
- **Upstream / Downstream**: Consumes Phases 1–5, 7; consumed by Phase 11 (Standard Library) and Phase 13 (Engineering Studio).

### Phase 11: Standard Equipment Library
- **Purpose**: Provide a comprehensive catalog of data-driven equipment definition packages covering industrial process engineering needs.
- **Scope**:
  - Rotating equipment: Centrifugal pumps, positive displacement pumps, centrifugal compressors, reciprocating compressors, steam turbines, electric motors.
  - Heat transfer equipment: Shell-and-tube heat exchangers, plate heat exchangers, air-cooled exchangers, fired heaters/furnaces.
  - Separation equipment: Two-phase flash drums, three-phase separators, vertical accumulators, distillation column sections.
  - Piping & control elements: Control valves (globe, ball, butterfly), manual isolation valves, check valves, pressure safety relief valves (PSV), orifices, static mixers.
  - Utility consumers: Steam traps, cooling water jackets, fuel gas burners, electrical switchgear.
- **Dependencies**: Phases 1, 4, 10.
- **Non-Goals**: Hardcoding equipment as separate Rust structs in the simulation core.
- **Architectural Outputs**: `packages/standard-library/` (validated TOML/JSON definition files and curves).
- **Acceptance Criteria**: All standard equipment instantiable via data packages; operating envelopes correctly validated.
- **Verification Requirements**: Full unit and verification test suite for every library package fixture.
- **Upstream / Downstream**: Consumes Phase 10; foundation for Phase 13 (Engineering Studio) and complex models.

### Phase 12: Advanced Process Physics & Pluggable Thermodynamics
- **Purpose**: Upgrade physical fidelity to rigorous industrial thermodynamics and advanced unit operations without changing plant topology.
- **Scope**:
  - Pluggable thermodynamics: Cubic equations of state (Peng-Robinson, Soave-Redlich-Kwong), departure functions, binary interaction parameters ($k_{ij}$).
  - Rigorous equilibrium: Multicomponent vapor-liquid equilibrium (VLE) flash algorithms, Rachford-Rice solver, Wilson K-values.
  - External property integration: Standardized interface to CoolProp / open thermophysical property databases.
  - Advanced hydraulics: Compressor performance maps (speed, pressure ratio, efficiency, surge/choke margin), pump NPSH and cavitation models, compressible gas flow.
  - Advanced heat transfer: Detailed multi-pass LMTD, NTU-effectiveness, tube-side and shell-side fouling dynamics, radiation heat transfer in fired heater fireboxes.
  - Reaction engineering: Stoichiometric reactors, equilibrium conversion, Arrhenius kinetic rate equations.
  - Numerical solvers: Stiff ODE integrators (Implicit Euler, Radau, BDF), adaptive step size control.
- **Dependencies**: Phase 4, Phase 10.
- **Non-Goals**: Replacing the Phase 4 engineering-fidelity packages (they remain available as lightweight options).
- **Architectural Outputs**: Advanced physics modules in `nebudcs-physics`.
- **Acceptance Criteria**: Multicomponent hydrocarbon flash matches reference chemical engineering data within 1%; surge margin accurately triggers compressor recycling.
- **Verification Requirements**: Validation against chemical engineering benchmark tables; enthalpy and mass balance precision tests ($<10^{-6}$ error).
- **Upstream / Downstream**: Consumes Phase 4; provides high-fidelity options for Studios.

### Phase 13: Engineering Studio Application
- **Purpose**: Deliver the desktop and web design environment for building, configuring, validating, and maintaining authoritative plant models.
- **Scope**:
  - UI Architectural Layout:
    $$\text{Equipment Library} \to \text{Model Navigator} \to \text{P\&ID Canvas} \to \text{Property Inspector}$$
  - Desktop engine: Rust-native `apps/nebudcs-desktop` using `egui` for UI panels and `wgpu` for high-performance vector P&ID rendering.
  - Web engine: Optional browser-based engineering client built with TypeScript and Web Components.
  - Features: Drag-and-drop equipment placement, semantic port snapping and routing, instrument configuration, control loop linking, operating limit configuration, multi-level undo/redo, real-time topology validation engine (reporting ERROR, WARNING, INFO, REVIEW).
  - Authoritative editing: All modifications write to the authoritative project model DTOs, not disconnected visual drawings.
- **Dependencies**: Phases 1, 2, 5, 7, 10, 11.
- **Non-Goals**: Mixing visual coordinates into simulation physics, implementing runtime training scenarios.
- **Architectural Outputs**: `apps/nebudcs-desktop`, `crates/nebudcs-render`.
- **Acceptance Criteria**: Intuitive graphical modeling of arbitrary plant topologies; instant validation feedback; reliable project file serialization.
- **Verification Requirements**: GUI automated integration tests; canvas rendering benchmarks (60 FPS on 1000+ items).
- **Upstream / Downstream**: Consumes core platform; produces project files for Phase 14 and Phase 15.

### Phase 14: Operator / DCS Station Console
- **Purpose**: Deliver an industrial-grade DCS operator station console for monitoring and controlling running simulations.
- **Scope**:
  - Architectural Layout:
    $$\text{Plant Navigator} + \text{Process Graphic / P\&ID} + \text{Alarm Summary} + \text{Equipment Faceplates} + \text{Trends} + \text{Events} + \text{Operator Controls}$$
  - Technology: Lightweight Web Components + SVG for dynamic process graphics + Canvas/WebGL for high-frequency trends; optional native desktop operator view.
  - Live operations: Real-time tag telemetry, dynamic animation driven by physical state (flow direction/velocity, valve travel, vessel level, pump rotation), standardized faceplates (PID, pump, motor, valve).
  - Operator controls: Mode toggles (MAN/AUTO/CAS), setpoint adjustment, manual output nudging, alarm acknowledgement and shelving, emergency stop actions.
  - Security boundary: Topology and equipment definition editing strictly disabled in Operator mode.
- **Dependencies**: Phases 3, 5, 6, 7, 8.
- **Non-Goals**: Topology editing, equipment template authoring, scenario scripting.
- **Architectural Outputs**: `apps/nebudcs-web` (Operator Console).
- **Acceptance Criteria**: Crisp 60 FPS rendering; sub-100ms latency on operator command execution; strict enforcement of read-only topology boundaries.
- **Verification Requirements**: End-to-end browser integration tests; alarm interaction responsiveness tests; accessibility and rendering tests.
- **Upstream / Downstream**: Primary runtime operational interface; consumes running simulation host.

### Phase 15: Training & Scenario Studio
- **Purpose**: Deliver an instructor station and training evaluation platform for running structured operator qualification curricula.
- **Scope**:
  - Instructor console: Launch scenarios, monitor trainee actions in real time, inject unannounced process faults, trigger emergency conditions, pause/accelerate simulation.
  - Assessment engine: Automated evaluation against objective criteria, allowed operating envelopes, forbidden action detection, alarm response time benchmarks.
  - Session reports: Comprehensive trainee scoring, event chronological log, process safety score, recovery efficiency metrics.
  - Replay system: High-fidelity deterministic playback of trainee sessions with synchronized process graphics and operator input markers.
- **Dependencies**: Phases 6, 7, 9, 14.
- **Non-Goals**: Live industrial plant DCS connection.
- **Architectural Outputs**: Training studio modules in `apps/nebudcs-web` and `apps/nebudcs-desktop`.
- **Acceptance Criteria**: Automated grading rubrics accurately reflect trainee performance; instructors can inject faults with zero simulation hiccups; replay is 100% reproducible.
- **Verification Requirements**: Scenario grading accuracy tests; session replay determinism tests; multi-user instructor-trainee synchronization tests.
- **Upstream / Downstream**: Consumes Phase 9 (Scenarios) and Phase 14 (Operator Console).

### Phase 16: Production Hardening, Performance & Deployment
- **Purpose**: Harden the complete platform for production deployment, scale performance to massive multi-unit facilities, and establish zero-friction deployment.
- **Scope**:
  - High-scale simulation performance: Profiling and optimizing the simulation hot loop (memory arena allocation, cache locality, SIMD where applicable), guaranteeing sub-10ms tick duration for 1000+ equipment instances.
  - Memory management: Enforcing zero-allocation hot loop invariant across all systems.
  - Security audit: Comprehensive vulnerability audit of WebSocket handlers, REST endpoints, SQLite parameter bindings, and Rhai sandboxes.
  - Deployment pipeline: Headless Ubuntu Server deployment, automated systemd service configuration, REST/WebSocket API exposure, Tailscale mesh network access for secure remote multi-station operation.
  - Enterprise persistence: Optional PostgreSQL backend adapter for enterprise historian archives.
- **Dependencies**: Phases 0 through 15.
- **Non-Goals**: Breaking backwards compatibility of project files or schemas.
- **Architectural Outputs**: Hardening patches, performance benchmarks, deployment scripts, production documentation.
- **Acceptance Criteria**: 1000+ equipment simulation runs stably under 10ms/tick budget; zero memory leaks during 48-hour soak test; secure remote connectivity over Tailscale verified.
- **Verification Requirements**: Long-running soak tests, memory leak profiling, Criterion performance benchmarks, security penetration suite.
- **Upstream / Downstream**: Final production readiness gate.
