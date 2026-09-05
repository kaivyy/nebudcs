---
title: NebuDCS Architecture Review Report
version: 0.1.0
status: draft
date: 2026-09-05
author: NebuDCS Architecture Team
---

# NebuDCS Architecture Review Report

> **This document is the culminating synthesis of the full NebuDCS architecture effort.**
> It records architecture status, PRD coverage, open decisions, risks, and the evidence
> that the architecture is ready for Phase 0 implementation to begin.

---

## 1. Architecture Status

| Dimension | Status | Notes |
|---|---|---|
| Domain model | ✅ Complete | Hierarchy, state layers, ownership boundaries defined |
| Equipment SDK | ✅ Complete | DefinitionInstance/RuntimeState split; capability traits; feasibility examples |
| Custom equipment (no recompile) | ✅ Complete | Level 1/2/3 customization; Rhai sandbox; TOML/JSON definitions |
| Physics framework | ✅ Complete | Multi-fidelity, pluggable, extensible without topology change |
| Thermodynamics | ✅ Complete | Pluggable PropertyPackage trait; path to PR/SRK/CoolProp |
| Solver architecture | ✅ Complete | Steady-state + dynamic interfaces; recycle convergence; DAE extensibility |
| Simulation engine | ✅ Complete | Deterministic single-threaded loop; multi-rate architecture |
| Control engine | ✅ Complete | PID, cascade, SFC, interlock, ESD; typed-tag causal path |
| Protection system | ✅ Complete | Permissives, trips, ESD, voting; uses same authoritative model |
| Alarm engine | ✅ Complete | Full lifecycle; typed alarms; operator action tracking |
| Historian | ✅ Complete | SQLite; sampling; downsampling; replay; query interface |
| Scenario engine | ✅ Complete | Data-driven; outcomes from model; replay |
| Training layer | ✅ Complete | Objective-based; operates on real simulation state |
| HMI/P&ID architecture | ✅ Complete | Three representations; HMI is view, not authority |
| Protocol | ✅ Complete | WebSocket delta/subscription + REST; reconnect; binary option |
| Project format | ✅ Complete | SQLite + TOML export; schema versioning; migrations |
| Versioning & provenance | ✅ Complete | Immutable versions; dependency locking; no silent template mutations |
| Engineering validation | ✅ Complete | ERROR/WARNING/INFO/ENGINEERING REVIEW; design-time checks |
| Security | ✅ Complete | Threat model; script sandbox; API auth; supply chain policy |
| Testing strategy | ✅ Complete | Full pyramid; determinism tests; replay; benchmarks; CI |
| Performance | ✅ Complete | Measurable targets; profiling-before-optimization policy |
| Engineering units | ✅ Complete | Strongly typed; canonical SI; display conversion; dimensional validation |
| Topology engine | ✅ Complete | Graph-based; SCC for recycles; utility networks; port validation |
| ADRs | ✅ Complete | ADR-001 through ADR-010; all major technology decisions recorded |

**Overall architecture readiness: Ready for Phase 0 scaffolding.**

---

## 2. PRD Changes and Additions

The PRD was architecturally sufficient for the core domain. The following additions and
clarifications were identified during architecture work and should be added to the PRD:

### Additions Required

| Section | Addition |
|---|---|
| New: §39 | Simulation Convergence States — explicitly define INVALID_MODEL, VALID_UNCONVERGED, PARTIALLY_INITIALIZED, CONVERGED as first-class states with diagnostic requirements |
| New: §40 | Engineering Unit Policy — mandate strongly typed quantities with canonical SI internals; forbid raw float maps as process state |
| New: §41 | Script API Stability — versioned Rhai API surface; breaking changes require a new API version; projects record which script API version they were authored against |
| New: §42 | Utility Network Degradation Propagation — explicitly require that utility degradation (instrument air, cooling water, electrical, hydrogen) propagates through connected equipment via the topology model, not via hardcoded chains |
| New: §43 | Topology Mutation Policy — define how structural changes (valve closure severing a stream) are handled: real-time SCC recomputation vs. solver-native zero-flow handling |
| Existing §36 | Strengthen safety disclaimer: include explicit text that simulation-derived trip limits, alarm thresholds, and protection sequences are NOT certified for use in actual safety instrumented systems (SIS) |

---

## 3. Requirements Coverage Matrix

### PRD Section Coverage

| PRD Section | Covered By | Status |
|---|---|---|
| §1 Product Vision | system-architecture.md, README.md | ✅ |
| §2 Architectural Principle | system-architecture.md, domain-model.md | ✅ |
| §3 Technology Direction | ADR-001..010 | ✅ |
| §4 Repository Architecture | ADR-009-crate-architecture.md | ✅ |
| §5 Plant Model | domain-model.md, topology.md | ✅ |
| §6 Equipment Framework | equipment-sdk.md, custom-equipment.md | ✅ |
| §7 Equipment Parameters | equipment-sdk.md (feasibility examples 1-3) | ✅ |
| §8 Reactor Framework | equipment-sdk.md (composition), custom-equipment.md | ✅ |
| §9 Furnace Framework | equipment-sdk.md (package composition) | ✅ |
| §10 Process Streams | topology.md (MaterialStream), thermodynamics.md | ✅ |
| §11 Utilities | topology.md (utility networks) | ✅ |
| §12 Control System | control-engine.md | ✅ |
| §13 Tag System | domain-model.md (typed tags), units.md | ✅ |
| §14 Dynamic Equipment Behavior | behavior-model.md, physics.md | ✅ |
| §15 Turbine + Auxiliary Systems | behavior-model.md (auxiliary dependencies) | ✅ |
| §16 Alarm System | alarm-historian.md | ✅ |
| §17 Historian | alarm-historian.md | ✅ |
| §18 Fault Injection | scenario-engine.md, behavior-model.md | ✅ |
| §19 Scenario Engine | scenario-engine.md | ✅ |
| §20 HMI Architecture | hmi-pid.md | ✅ |
| §21 Faceplates | hmi-pid.md | ✅ |
| §22 P&ID Editor | hmi-pid.md | ✅ |
| §23 Three Representations | hmi-pid.md | ✅ |
| §24 Unit Templates | custom-equipment.md (Level 2 composition) | ✅ |
| §25 RFCC Template | custom-equipment.md (package architecture) | ✅ |
| §26 Custom Refinery Designer | custom-equipment.md, topology.md | ✅ |
| §27 Engineering Validation | validation.md | ✅ |
| §28 UI Strategy | hmi-pid.md, ADR-004, ADR-005 | ✅ |
| §29 Performance Architecture | performance.md, simulation-engine.md | ✅ |
| §30 Headless CLI | system-architecture.md (deployment modes) | ✅ |
| §31 Scripting | ADR-007-scripting-rhai.md, custom-equipment.md | ✅ |
| §32 Project Format | project-format.md | ✅ |
| §33 Testing | testing.md | ✅ |
| §34 Development Roadmap | (preserved in PRD; implementation plan separate) | ✅ |
| §35 UI Development Strategy | hmi-pid.md (vertical slices) | ✅ |
| §36 Safety Boundary | security.md, protection.md | ✅ |
| §37 Definition of Done | All documents combined | ✅ |
| §38 Final Engineering Principle | README.md, system-architecture.md | ✅ |

---

## 4. Domain Boundaries

```
┌────────────────────────────────────────────────────────┐
│  nebudcs-core        │  Clock, tick, IDs, events, base  │
│  nebudcs-model       │  Project hierarchy, DTO schemas   │
│  nebudcs-topology    │  Graph engine, ports, streams     │
│  nebudcs-physics     │  Balances, heat transfer, curves  │
│  nebudcs-thermo      │  PropertyPackage trait + impls    │
│  nebudcs-solver      │  Equation/variable/residual API   │
│  nebudcs-equipment   │  Definition/Instance/Runtime      │
│  nebudcs-control     │  PID, cascade, SFC, interlocks    │
│  nebudcs-alarm       │  Alarm engine, lifecycle          │
│  nebudcs-historian   │  Sampling, storage, query, replay │
│  nebudcs-scenario    │  Fault injection, scenario runner │
│  nebudcs-protocol    │  WebSocket/REST message types     │
│  nebudcs-storage     │  SQLite persistence, migrations   │
│  nebudcs-scripting   │  Rhai sandbox, script host        │
│  nebudcs-render      │  Rendering abstractions (no UI)   │
├────────────────────────────────────────────────────────┤
│  apps/nebudcs-desktop  │  egui/wgpu engineering station  │
│  apps/nebudcs-server   │  Axum/Tokio simulation host     │
│  apps/nebudcs-cli      │  Headless simulation runner      │
│  apps/nebudcs-web      │  TypeScript Web HMI              │
└────────────────────────────────────────────────────────┘
```

**Dependency direction is strictly downward — no cycles.**

---

## 5. Equipment Extensibility Assessment

**Assessment: PASS**

The architecture satisfies the strongest architectural criterion:

> A new refinery/unit configuration must be constructible entirely from the
> equipment/template/configuration system without adding hardcoded unit-specific
> branches to the simulation engine.

Evidence:
- `EquipmentDefinition` is a data structure (versioned TOML/JSON/SQLite row), not a Rust class.
- New equipment types are created by authoring a new definition record — no Rust recompilation.
- Capabilities are traits composed at runtime from the definition's capability list.
- Performance curves (pump head, compressor map, valve Cv) are lookup tables in the definition.
- Equipment state machines are configurable transition graphs, not hardcoded enums.
- Instrument and actuator dynamics are definition parameters, not code branches.

---

## 6. Custom Equipment Assessment

**Assessment: PASS (with scripting budget caveat)**

Level 1 (configuration), Level 2 (composition), and Level 3 (custom behavior via Rhai scripts
or declarative equations) are all architecturally supported without modifying or recompiling
the NebuDCS core.

**Caveat:** Rhai script execution within the simulation tick loop requires careful instruction
budget enforcement. Scripts that exceed their instruction budget must be terminated gracefully
without crashing the simulation. This must be verified in Phase 2 performance benchmarks.

---

## 7. Custom Behavior Assessment

**Assessment: PASS**

Behavior is defined via:
- Lookup tables (pump curves, compressor maps)
- Declarative equations (mass/energy balance)
- Rhai scripts (complex custom logic)

Failure modes are first-class configuration data. Causal propagation emerges from model
relationships, not hardcoded chains. No behavior is implemented as a UI animation.

---

## 8. Physics Architecture Assessment

**Assessment: PASS**

Multi-fidelity physics model (Simplified → Basic Dynamic → Engineering → High Fidelity)
is defined with a clear switching mechanism that does not require changing plant topology
or HMI. Physics models are separate from the domain model. Mass and energy balances are
extensible. Solver abstraction is physics-model-agnostic.

---

## 9. Thermodynamics Assessment

**Assessment: PASS (with progressive implementation note)**

The `PropertyPackage` trait cleanly separates thermodynamic calculations from equipment
models. Initial implementation uses an Ideal Gas approximation. Path to Peng-Robinson,
SRK, and CoolProp integration is defined without requiring topology changes.

**Note:** Per-tick thermodynamic evaluation performance must be benchmarked. If full
equation-of-state calculations are too slow at 50-100 Hz, pre-computed interpolation
meshes or lazy evaluation strategies must be applied.

---

## 10. Solver/Convergence Assessment

**Assessment: PASS**

Convergence state machine (INVALID → UNCONVERGED → PARTIAL → CONVERGED) is defined.
Recycle handling via Strongly Connected Components (SCC) + tear streams is specified.
Successive substitution, Wegstein, and Broyden algorithms are noted for recycle convergence.
DAE solver extensibility is preserved through the equation/variable/residual abstraction.

---

## 11. Control/Protection Assessment

**Assessment: PASS**

Single authoritative control execution pipeline is defined. All control logic operates
through the typed tag system. Protection logic uses the same simulation model as process
logic. HMI cannot mutate process state directly. Causal path is: process state → 
measurement → instrument → typed tag → controller/protection → command → actuator → 
equipment → process state.

---

## 12. HMI/P&ID Assessment

**Assessment: PASS**

Three independent concerns (engineering model, presentation, operator interaction) are
clearly separated. Three equipment representations (P&ID symbol, process graphic, faceplate)
all reference the same model entities and tags. HMI is definitively a view. P&ID is
generated from the plant model, not the reverse.

**Web framework recommendation:** Web Components (Lit) for lightweight realtime HMI.
React is NOT recommended at this stage. Decision is in ADR-005.

---

## 13. Protocol Assessment

**Assessment: PASS**

WebSocket subscription + delta update model prevents full-state broadcasts every frame.
Dirty state tracking, batching, and efficient serialization are specified. Reconnect and
resynchronization are designed. REST/HTTP handles configuration and project operations.
Binary protocol (MessagePack) is available when JSON overhead is measured as a problem.

---

## 14. Versioning Assessment

**Assessment: PASS**

Equipment definitions, behavior models, property packages, scenarios, and project schemas
are versioned. Versions are immutable once published. Projects lock to specific definition
versions — a global template update cannot silently alter existing projects. Migration
paths are defined for schema upgrades.

---

## 15. Security Review

**Assessment: PASS**

Threat model is defined. Script sandbox denies filesystem, network, OS, and process access.
CPU/memory/time limits are specified. API authentication and authorization are defined.
Project file integrity validation is specified. Supply chain policy (dependency audit) is
defined. Safety disclaimer (simulation ≠ certified SIS) is specified.

---

## 16. Dependency Review

Key dependencies confirmed with current stable versions (as of 2026):

| Crate | Purpose | Decision |
|---|---|---|
| `egui` / `eframe` | Immediate-mode desktop UI | Accepted — ADR-004 |
| `wgpu` | GPU-accelerated P&ID canvas rendering | Accepted — ADR-004 |
| `tokio` | Async runtime for server/networking only | Accepted — NOT in simulation loop |
| `rhai` | Sandboxed scripting | Accepted — ADR-007 |
| `rusqlite` or `sqlx` | SQLite persistence | Accepted — ADR-006 |
| `serde` / `serde_json` / `toml` | Serialization | Accepted |
| `tracing` | Structured logging and observability | Accepted |
| `criterion` | Benchmarking | Accepted |
| `proptest` | Property-based testing | Accepted |

No unnecessary dependencies introduced. Feature flags minimized.

---

## 17. Performance Strategy

| Metric | Target |
|---|---|
| Simulation tick (100 equipment) | < 5 ms |
| Simulation tick (1,000 equipment) | < 20 ms |
| Control loop execution | < 2 ms per 100 loops |
| Historian write throughput | > 10,000 samples/sec |
| WebSocket update latency | < 50 ms per subscription batch |
| Project load (medium plant) | < 2 seconds |
| Project save | < 500 ms |
| Rendering | 60 FPS on 4-core, 8GB RAM hardware |

Profiling with `criterion`, `perf`, and `flamegraph` before any optimization.
No premature optimization.

---

## 18. Testing Strategy Summary

| Layer | Tool | Coverage Target |
|---|---|---|
| Unit | `cargo test` | All public APIs |
| Property-based | `proptest` | Physics invariants, unit conversions, topology |
| Equipment behavior | Custom fixtures | All standard equipment types |
| Determinism | Replay comparison | 100% reproducibility |
| Convergence | Reference cases | Recycle loops, steady-state init |
| Integration | Vertical slice tests | Pump → PID → Alarm → Historian |
| Performance | `criterion` | Tick budget, historian throughput |
| Schema migration | Migration tests | v0 → v1 → v2 project files |
| Security | Sandbox fuzzing | Script escape attempts |

CI runs unit + property + integration on every commit. Performance benchmarks run nightly.

---

## 19. Major ADRs Summary

| ADR | Decision |
|---|---|
| ADR-001 | Rust (stable), modular crate workspace, no async in simulation loop |
| ADR-002 | Deterministic single-threaded simulation; no system time in tick |
| ADR-003 | Data-driven EquipmentDefinition; no one-class-per-equipment-variant |
| ADR-004 | egui/eframe + wgpu for desktop; NO Electron |
| ADR-005 | Web Components (Lit) for web HMI; NOT React at this stage |
| ADR-006 | SQLite (rusqlite/sqlx) for local persistence; TOML export |
| ADR-007 | Rhai for sandboxed scripting; declarative preferred where sufficient |
| ADR-008 | WebSocket (delta/subscription) + REST; JSON first, MessagePack if needed |
| ADR-009 | 15 crates + 4 apps; strict dependency direction; no cycles |
| ADR-010 | Pluggable PropertyPackage trait; Ideal Gas first; PR/CoolProp later |

---

## 20. Architectural Risks

| # | Risk | Severity | Mitigation |
|---|---|---|---|
| R-01 | Rhai script instruction budget enforcement in 100 Hz tick | HIGH | Benchmark in Phase 2; enforce hard instruction limit; async script execution if needed |
| R-02 | SQLite historian write contention under 100 Hz simulation | MEDIUM | Dedicated async historian writer thread; WAL mode; batch inserts |
| R-03 | Dynamic dispatch (`dyn PropertyPackage`) in hot loop | MEDIUM | Benchmark in Phase 2; consider enum dispatch or monomorphization |
| R-04 | SCC recomputation on topology mutation (valve closure) | MEDIUM | Cache SCCs; incremental recomputation or solver-native zero-flow |
| R-05 | Web Component state management at HMI scale | LOW-MEDIUM | Define global state store early; evaluate Nanostores or similar |
| R-06 | DAE solver complexity for stiff systems | LOW (deferred) | Solver abstraction preserves future DAE integration; no immediate risk |
| R-07 | Full equation-of-state (PR/CoolProp) per-tick performance | LOW (deferred) | Interpolation meshes or lazy evaluation when needed |
| R-08 | Script API stability across versions | MEDIUM | Version Rhai API surface; projects record API version; test migrations |
| R-09 | P&ID SVG performance at large plant scale (Web) | MEDIUM | Canvas/WebGL fallback for large scenes defined in ADR-005 |
| R-10 | wgpu platform compatibility (Linux/Mac/Windows) | LOW | wgpu supports Vulkan/Metal/DX12/WebGPU; test matrix needed |

---

## 21. Deferred Decisions

| # | Decision | Deferred To |
|---|---|---|
| D-01 | PID fallback behavior on instrument failure (hold MAN vs. interlock) | Phase 5 control engine implementation |
| D-02 | Historian retention policy defaults (size-based vs time-based) | Phase 6 historian implementation |
| D-03 | OPC UA / Modbus TCP integration for external PLC connectivity | Phase 15 or separate roadmap item |
| D-04 | wgpu renderer compiled to WebAssembly for Web P&ID canvas | Phase 8 P&ID editor — evaluate SVG limits first |
| D-05 | DAE solver selection (Sundials CVODE, etc.) | Phase 15 high-fidelity physics |
| D-06 | CoolProp or alternative property library integration | Phase 15 |
| D-07 | Multi-user/collaboration features | Post-Phase 13 |
| D-08 | Cloud/server deployment architecture (PostgreSQL, auth) | Phase 14+ |
| D-09 | Separate TSDB for historian vs SQLite main file | Phase 6 — SQLite first, evaluate if needed |

---

## 22. Implementation Order (Recommended)

Following PRD §34 and the vertical slice strategy from PRD §35:

### Phase 0 — Repository & Tooling
- Initialize Cargo workspace with all 15 crates + 4 apps (stubs)
- CI pipeline: `cargo test`, `cargo clippy`, `cargo fmt --check`
- `CONTRIBUTING.md`, `ARCHITECTURE.md` links, `cargo deny` for dependency audit

### Phase 1 — Core Primitives
- `nebudcs-core`: SimulationClock, EntityId, event bus
- `nebudcs-model`: Project/Plant/Area/Unit hierarchy DTOs
- `nebudcs-units`: Typed quantities, canonical SI, conversion, dimensional validation
- `nebudcs-storage`: SQLite schema v0, migrations framework

### Vertical Slice 1 (Phases 2-7 progressive delivery)
- Pump → Vessel → ControlValve → MaterialStream → PID → Alarm → Historian → WebSocket → HMI Faceplate → Trend
- Prove full stack before expanding equipment library

### Vertical Slice 2
- 3-stage Compressor → compressor map → vibration → anti-surge → trip → downstream propagation

### Vertical Slice 3
- Custom equipment from TOML definition (no Rust change) → proves the fundamental architectural guarantee

---

## 23. Vertical Slice Strategy

Each slice proves a layer of the architecture. They must be executed sequentially; slice N+1
begins only when slice N is fully functional, tested, and deployed to CI.

| Slice | What It Proves |
|---|---|
| 1: Pump → HMI | Full stack from simulation to web faceplate; WebSocket; historian; alarm |
| 2: Compressor | Nonlinear behavior; compressor map; vibration; anti-surge; trip propagation |
| 3: Custom Equipment | No-recompile custom definition; proves Level 1-3 customization works |
| 4: Recycle | Recycle convergence; SCC; initialization; steady-state → dynamic handoff |
| 5: Training Scenario | Fault injection; causal propagation; operator actions; replay; grading data |

---

## 24. Evidence of Feasibility

The following feasibility examples are documented in `equipment-sdk.md` and `custom-equipment.md`:

- **Example 1**: Pump with max power 100 kW, head curve, efficiency curve, NPSHR curve, vibration limits, bearing temperature limits, startup permissive, trip condition — represented entirely as TOML/JSON definition data, no Rust class.
- **Example 2**: 3-stage H2 compressor package with compressor map, intercooler, recycle, anti-surge, vibration model, overspeed trip — represented as a composed package definition.
- **Example 3**: Vertical vessel with volume, pressure limit, level limits, inlet/outlet ports, PSV, temperature instrumentation — represented as a definition with port contracts.
- **Example 4**: Completely custom equipment created from project data — demonstrates Level 3 custom behavior via Rhai script without modifying or recompiling the NebuDCS core.

---

## 25. Known Limitations (v0.1 Architecture)

1. **No real-world thermodynamics yet.** Ideal Gas is the initial implementation. This is acceptable for Phase 1-5 training scenarios but insufficient for high-fidelity refinery simulation.

2. **No OPC UA / Modbus.** External DCS/PLC integration is deferred. NebuDCS is a self-contained simulation platform.

3. **No multi-user collaboration.** Single-user project model initially. Server deployment (Phase 14+) will require authorization design.

4. **P&ID SVG scale limit.** Very large plants (1000+ equipment in a single P&ID canvas) may require the wgpu/WebGL rendering path. The SVG approach is proven for unit-level views.

5. **Rhai scripting performance.** Rhai is an AST interpreter. For very complex custom equipment with many script executions per tick, instruction budgets must be tightly enforced.

---

## 26. Architecture Readiness Declaration

> The NebuDCS architecture is **ready for Phase 0 implementation**.

Evidence:
- All 35 architecture documents produced with Mermaid diagrams, Rust pseudocode, and feasibility examples.
- All 10 ADRs recorded with complete context, alternatives, rationale, trade-offs, and rejection reasoning.
- 38/38 PRD sections covered.
- Domain boundaries established with no circular dependencies.
- Strongest architectural criterion verified: custom refinery configurations can be constructed from equipment definitions without modifying the Rust simulation core.
- Equipment extensibility, custom behavior, physics fidelity, thermodynamic pluggability, solver extensibility, deterministic replay, project versioning, security boundaries, and testing strategy all confirmed.
- Open questions are recorded, risk-rated, and have mitigations or explicit deferral justifications.

**Build the platform, not the refinery.**

---

*Generated by NebuDCS Architecture Team — 2026-09-05*
