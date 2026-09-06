# NebuDCS: Industrial Process Simulation & DCS Training Platform

**NebuDCS** is a lightweight, extensible, high-performance, and deterministic process simulation, distributed control system (DCS), and operator training platform built in Rust.

---

## Core Product Invariants

1. **Generic, Data-Driven & Fully Configurable**:
   NebuDCS is **not** hardcoded around any specific refinery unit (such as CDU, NHT, CCR, RFCC, Platforming, TDHT, or AH Unibon) or vendor DCS/PLC. All refinery units and process facilities are reusable configuration packages built from generic primitives without recompiling the Rust simulation core.
2. **Authoritative Model Principle**:
   The unidirectional flow of truth is strictly enforced:
   $$\text{Project Model} \to \text{Topology} \to \text{Simulation} \to \text{Physical State} \to \text{Tags/Signals} \to \text{Protocol} \to \text{UI}$$
   Visual graphics, P&ID diagrams, faceplates, and trends are purely views observing authoritative simulation state. Frontend code contains zero fake math or duplicate simulation physics.
3. **Deterministic Simulation Runtime**:
   Single-threaded simulation hot loop running at fixed timestep $\Delta t$, strictly monotonic `SimulationTime`, zero system clock lookups in the tick, and double-buffered state isolation with atomic commits.
4. **Three Decoupled Product Environments**:
   - **Engineering Studio**: Authoritative plant model creation, equipment library, topology routing, parameter inspection, and design validation.
   - **Operator / DCS Station**: Runtime process monitoring, interactive graphics, alarm management, and faceplate operation (topology editing disabled).
   - **Training / Scenario Studio**: Data-driven scenario execution, instructor controls, dynamic physical fault injection, rubric-based evaluation, and session replay.
5. **Headless-First Architecture**:
   The simulation core runs headlessly on Ubuntu Server with zero GUI dependencies. Clients connect over lightweight WebSockets / REST, with secure remote access via Tailscale mesh networking.

---

## Current Implementation Baseline

| Phase | Title | Crate(s) / Apps | Status | Test Coverage |
|---|---|---|---|---|
| **Phase 0** | Repository & Foundation Setup | Workspace root | **COMPLETE** | CI Passing |
| **Phase 1** | Domain Model & Physical Types | `nebudcs-types`, `nebudcs-units`, `nebudcs-model` | **COMPLETE** | 71 Tests Passing |
| **Phase 2** | Topology, Ports & Graph Analysis | `nebudcs-topology` | **COMPLETE** | 21 Tests Passing |
| **Phase 3** | Simulation Runtime & Execution | `nebudcs-sim` | **COMPLETE** | 10 Tests Passing |
| **Phase 4** | Process Physics & Conservation | `nebudcs-physics` | **`PHASE 4 VERIFIED`** | 40 Tests Passing |
| **Phase 5** | Control Engine & Automation | `nebudcs-control`, `nebudcs-alarm` | **`PHASE 5 VERIFIED`** | 29 Tests Passing |
| **Phase 6** | Historian & Timeseries Persistence | `nebudcs-historian` | **`PHASE 6 VERIFIED`** | 14 Tests Passing |
| **Phase 7** | Realtime Protocol & API Server | `nebudcs-protocol`, `apps/nebudcs-server` | **`PHASE 7 VERIFIED`** | 29 Tests Passing |
| **Phase 8** | End-to-End Vertical Slice 1 | Full Stack Integration (`web/`) | **`PHASE 8 VERIFIED`** | 31 Web Tests Passing |
| **Phase 9** | Scenario & Fault Injection Engine | `nebudcs-scenario` | **`PHASE 9 VERIFIED`** | 20 Tests Passing |
| **Phase 10** | Custom Equipment & Runtime Authoring | `nebudcs-scripting` | **`PHASE 10 VERIFIED`** | 31 Tests Passing |
| **Phase 11** | Standard Equipment Library | `nebudcs-scripting`, `packages/standard-library/` | **`PHASE 11 VERIFIED`** | 50 Tests Passing |

**Workspace Total**: **All 326 Rust workspace tests and 31/31 TypeScript tests passing cleanly (0 failures, 0 warnings)**.

---

## Crate Workspace Structure

```
crates/
  nebudcs-types/     # Core primitive newtypes (ULID EntityId, TagId, SimulationTime, errors)
  nebudcs-units/     # Strongly typed physical quantities (Pressure, Temperature, MassFlow)
  nebudcs-model/     # Domain hierarchy DTOs, EquipmentDefinition schemas, Tag types
  nebudcs-topology/  # Semantic ports, streams, piping, graph queries, Tarjan SCC cycle detection
  nebudcs-sim/       # Deterministic clock, fixed-step tick loop, double-buffered state, execution plan
  nebudcs-physics/   # Conservation laws, Darcy-Weisbach hydraulics, VLE, nonlinear recycle solver
  nebudcs-control/   # Industrial DCS/PLC control algorithms, PID, bumpless transfer, anti-windup
  nebudcs-alarm/     # Alarm lifecycle state machine, shelving, deadbands, ANSI/ISA-18.2 compliance
  nebudcs-historian/ # Non-blocking SQLite WAL persistence, asynchronous sampling, downsampling
  nebudcs-protocol/  # Versioned network DTOs, WebSocket envelopes, typed commands, delta encoding
  nebudcs-scenario/  # Deterministic scenario engine, timeline triggers, dynamic fault injection, PRNG
  nebudcs-scripting/ # Custom equipment authoring, 1D/2D curves, package composition, sandboxed Rhai
apps/
  nebudcs-server/    # Headless Axum server, isolated simulation execution thread, WebSocket host
web/                 # Native Web Components, SVG process graphics, trend charts, scenario panel
```

---

## Quickstart & Headless Verification

NebuDCS is built and tested entirely in a headless Linux environment:

```bash
# Check code formatting
cargo fmt --check

# Compile all workspace members
cargo check --workspace

# Run full test suite (295 Rust tests)
cargo test --workspace

# Run strict clippy analysis
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Documentation Index

- **[System Architecture](docs/architecture/README.md)**: System context, bounded contexts, deployment modes, tech selection, and data flows.
- **[Implementation Blueprint](docs/implementation/README.md)**: Implementation philosophy, dependency map, coding rules, and testing strategy.
- **[Implementation Roadmap](docs/implementation/roadmap.md)**: Comprehensive 17-phase lifecycle (Phases 0 through 16).
- **[Phase 4 Process Physics Specification](docs/implementation/phase-04-process-physics.md)**: Verified physics architecture, equations, numerical safeguards, and engineering limitations.
- **[Phase 5 Control Engine Specification](docs/implementation/phase-05-control-engine.md)**: Verified specification for PID, bumpless transfer, anti-windup, filters, logic, and alarms.
- **[Phase 6 Historian Specification](docs/implementation/phase-06-historian.md)**: Verified specification for SQLite WAL, ring-buffer backpressure, and downsampling queries.
- **[Phase 7 Realtime Protocol Specification](docs/architecture/protocol.md)**: Network DTOs, WebSocket streaming, REST API, and session management.
- **[Vertical Slice 1 Specification](docs/implementation/vertical-slice-01.md)**: Minimal end-to-end full stack proof (Simulation to UI).
- **[Architecture Decision Records (ADRs)](docs/architecture/adr/)**: Architectural decisions and trade-off rationales.
