---
title: Architecture Traceability Matrix & Consistency Audit
version: 2.0.0
status: verified-baseline
date: 2026-09-06
---

# Architecture Traceability Matrix & Consistency Audit

## 1. Architecture Consistency Audit

This audit validates complete coherence between the PRD, the architecture documentation, and the actual verified implementation baseline (Phases 0–4).

### Key Findings & Alignments
1. **Core Product Principle**:
   - **PRD Invariant**: NebuDCS is a generic, configurable process simulation platform and DCS/PLC training environment. It must **NOT** be architected around hardcoded refinery units (CDU, NHT, CCR, RFCC, Platforming, TDHT, AH Unibon).
   - **Verification**: Verified. `crates/nebudcs-model`, `nebudcs-topology`, and `nebudcs-physics` contain zero refinery-specific hardcoded types. All equipment are composed dynamically from generic capabilities (`Pump`, `Valve`, `Vessel`, `HeatExchanger`, `Pipe`). Specific refinery units are data packages planned for Phase 11.
2. **Authoritative Model**:
   - **PRD Invariant**: Simulation engine is the single source of truth. Visual graphics never become process state.
   - **Verification**: Verified. Unidirectional flow: $\text{Project Model} \to \text{Topology} \to \text{Simulation} \to \text{Physical State} \to \text{Tags} \to \text{Protocol} \to \text{UI}$. P&ID layouts and HMI views are strictly read-only observers of simulation state.
3. **Deterministic Simulation Boundary**:
   - **PRD Invariant**: Single-threaded, deterministic fixed-step tick loop with zero system time or async execution in the hot loop.
   - **Verification**: Verified. `crates/nebudcs-sim` implements `SimulationClock` using pure `SimulationTime` and double-buffered `RuntimeState` with atomic commit.
4. **Physical Fidelity Baseline**:
   - **PRD Invariant**: Extensible physics with verified conservation laws and numerical stability.
   - **Verification**: Verified as **`PHASE 4 VERIFIED`**. Implements mass, species, and enthalpy conservation, dynamic accumulator inventory, Churchill Darcy-Weisbach pipe hydraulics, ISA valve dynamics, Clausius-Clapeyron VLE, and damped Wegstein recycle solving. Engineering-fidelity assumptions are explicitly documented; cubic equations of state are deferred to Phase 12.
5. **Equipment Extensibility**:
   - **Current Implementation**: Strongly typed Rust capability traits (`EquipmentCapability`, `GenericEquipmentPhysics`).
   - **Future Implementation (Phase 10)**: Runtime data-driven authoring and sandboxed Rhai scripting with instruction and memory limits. Runtime no-recompile authoring is NOT yet implemented.
6. **Three Product Environments**:
   - **PRD Invariant**: Clear separation of user roles and workflows into Engineering Studio, Operator/DCS Station, and Training/Scenario Studio.
   - **Verification**: Verified. Architecturally separated into dedicated phases (Phases 13, 14, 15) and distinct UI boundaries.

---

## 2. Comprehensive Requirements Traceability Matrix (Phases 0–16)

| PRD Section / Requirement | Architecture Document | Crate(s) / Target | Roadmap Phase | Implementation Status |
|---|---|---|---|---|
| Section 1–4: System Context & Workspace | `system-architecture.md`, ADR-001, ADR-009 | Root, Workspace | **Phase 0** | **COMPLETE** |
| Section 5: Strongly Typed Domain Hierarchy | `domain-model.md`, `units.md` | `nebudcs-types`, `nebudcs-units`, `nebudcs-model` | **Phase 1** | **COMPLETE** |
| Section 7: Strongly Typed Port & Stream Topology | `topology.md` | `nebudcs-topology` | **Phase 2** | **COMPLETE** |
| Section 8: Graph Analysis & Cycle Detection (SCC) | `topology.md`, `solver.md` | `nebudcs-topology` | **Phase 2** | **COMPLETE** |
| Section 9: Deterministic Simulation Clock & Runtime | `simulation-engine.md`, ADR-002 | `nebudcs-sim` | **Phase 3** | **COMPLETE** |
| Section 10: Double-Buffered State & Execution Plan | `simulation-engine.md` | `nebudcs-sim` | **Phase 3** | **COMPLETE** |
| Section 11: Conservation Laws & Physical State | `physics.md`, `thermodynamics.md` | `nebudcs-physics` | **Phase 4** | **PHASE 4 VERIFIED** |
| Section 12: Pipe & Valve Hydraulics | `physics.md`, `solver.md` | `nebudcs-physics` | **Phase 4** | **PHASE 4 VERIFIED** |
| Section 13: Pluggable Thermodynamics & VLE | `thermodynamics.md`, ADR-010 | `nebudcs-physics` | **Phase 4** | **PHASE 4 VERIFIED** |
| Section 14: Nonlinear Solver & Recycle Loops | `solver.md` | `nebudcs-physics` | **Phase 4** | **PHASE 4 VERIFIED** |
| Section 15: DCS/PLC Control Primitives & PID | `control-engine.md`, `phase-05-control-engine.md` | `nebudcs-control` | **Phase 5** | **PHASE 5 VERIFIED** |
| Section 16: Interlocks, Permissives & Alarms | `protection.md`, `alarm-historian.md` | `nebudcs-control`, `nebudcs-alarm` | **Phase 5** | **PHASE 5 VERIFIED** |
| Section 17: SQLite Historian & Timeseries | `alarm-historian.md`, `phase-06-historian.md`, ADR-006 | `nebudcs-historian` | **Phase 6** | **PHASE 6 VERIFIED** |
| Section 18: Realtime WebSocket Protocol & Server | `protocol.md`, ADR-008 | `nebudcs-protocol`, `apps/nebudcs-server` | **Phase 7** | **PHASE 7 VERIFIED** |
| Section 19: End-to-End Dynamic Vertical Slice | `vertical-slice-01.md` | Full Stack Proof | **Phase 8** | **PHASE 8 VERIFIED** |
| Section 20: Scenario Engine & Fault Injection | `scenario-engine.md`, `training.md` | `nebudcs-scenario` | **Phase 9** | **PHASE 9 VERIFIED** |
| Section 6 & 21: Data-Driven Custom Equipment & Rhai | `custom-equipment.md`, ADR-003, ADR-007 | `nebudcs-scripting` | **Phase 10** | **PLANNED** |
| Section 6: Standard Equipment Package Library | `equipment-sdk.md` | `packages/standard-library/` | **Phase 11** | **PLANNED** |
| Section 22: Advanced Thermodynamics & DAE Solvers | `thermodynamics.md`, `physics.md` | `nebudcs-physics` (extensions) | **Phase 12** | **PLANNED** |
| Section 23: Desktop Engineering Studio (P&ID Editor) | `hmi-pid.md`, ADR-004 | `apps/nebudcs-desktop` | **Phase 13** | **PLANNED** |
| Section 24: Web Operator / DCS Station Console | `hmi-pid.md`, ADR-005 | `apps/nebudcs-web` | **Phase 14** | **PLANNED** |
| Section 25: Training & Scenario Studio (Instructor) | `training.md`, `scenario-engine.md` | Training Studio Apps | **Phase 15** | **PLANNED** |
| Section 26–28: Hardening, Performance & Deployment | `performance.md`, `security.md` | Deployment Targets, Tailscale | **Phase 16** | **PLANNED** |

---

## 3. Product Environment Matrix

| Feature / Concern | Engineering Studio (Phase 13) | Operator / DCS Station (Phase 14) | Training & Scenario Studio (Phase 15) |
|---|---|---|---|
| **Primary Persona** | Process / Automation Engineer | Plant Operator / DCS Technician | Instructor / Trainee Evaluator |
| **Primary Goal** | Build, configure, validate plant models | Monitor & control running plant | Train, inject faults, evaluate response |
| **P&ID / Topology Editing** | **Full Access** (Add, delete, route connections) | **Disabled** (Read-only process graphic) | **Disabled** (Read-only process graphic) |
| **Parameter Configuration** | **Full Access** (Pipe diameter, pump curves, volumes) | **Disabled** (Operator cannot re-engineer equipment) | **Scenario-Scoped** (Fault injection overrides) |
| **Control Operations** | Configure loops, set tuning $K_p, T_i, T_d$ | Nudge MV, adjust SP, switch MAN/AUTO/CAS | Monitor operator actions, log response times |
| **Alarm Operations** | Define trip points, limits, deadbands | Acknowledge alarms, shelve alarms | Review alarm reaction latency, score alarms |
| **Scenario Control** | Define initial conditions | Execute standard operating procedures | Start, pause, inject faults, grade, replay |
| **UI Layout** | Library $\to$ Navigator $\to$ Canvas $\to$ Inspector | Navigator + Graphic + Alarms + Faceplates | Instructor Console + Student Console + Replay |
| **Technology** | Rust `egui/wgpu` Desktop & Web Studio | Web Components + SVG / Canvas Web Console | Web & Desktop Scenario Control Console |
