# NebuDCS Stage 6: Advanced Process Physics & Hydraulic Network Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Stage 6: Advanced Process Physics & Hydraulic Network for NebuDCS, providing a coupled hydraulic network solver (Darcy-Weisbach friction, pipe geometry, valves, pumps, compressors, elevation, branches, recycle loops), rigorous thermodynamic integration (composition, enthalpy, flash, phase equilibrium), equipment coupling, Stage 5 control loop coupling, Engineering Studio diagnostics, and verification across 4 generic acceptance plants.

**Architecture:** A modular hydraulic network solver (`crates/nebudcs-physics/src/network/`) using a nodal pressure formulation solved via the existing multidimensional `NewtonSolver` (LU decomposition, Armijo line search) and `RecycleSolver` (Wegstein acceleration). Hydraulic and thermodynamic states are coupled into `GenericPhysicsSystem` during the deterministic single-threaded simulation tick, propagating flow, pressure drop, Reynolds number, phase state, and NPSH margins without altering baseline architectures.

**Tech Stack:** Rust (1.80+ stable), `nebudcs-units`, `nebudcs-types`, `nebudcs-model`, `nebudcs-sim`, `nebudcs-physics`, `nebudcs-topology`, `nebudcs-server`, TypeScript / Web Components / SVG (`nebu-studio.ts`), Playwright, SQLite.

## Global Constraints
- Preserve every existing capability from Phases 0–16, Controlled Platform Expansion, Industrial Domain Packages, and Stages 1–5.
- Do NOT create Phase 17.
- Do NOT create a second simulation engine, second thermodynamic engine, or parallel numerical solver.
- Rust remains the sole authoritative process/control execution layer.
- Strictly deterministic simulation within the single-threaded simulation loop; zero heap allocations on the hot path.
- Support 4 generic acceptance plants: Plant A (branched hydraulic), Plant B (pump NPSH cavitation), Plant C (thermal + recycle), Plant D (compressor network + anti-surge).
- Evidence before assertions: verify every test, benchmark, and Playwright screenshot before sign-off.

---

### Task 1: Hydraulic Parameters in Canonical Project Model (`nebudcs-model`)
**Files:**
- Modify: `crates/nebudcs-model/src/stream.rs`
- Test: `crates/nebudcs-model/src/stream.rs` (unit tests)

**Interfaces:**
- Produces: `StreamDefinition` fields: `diameter`, `length`, `roughness`, `elevation_change`, `loss_k`.

- [ ] **Step 1: Write unit tests for stream hydraulic properties and serde backward compatibility**
- [ ] **Step 2: Add optional hydraulic fields with `#[serde(default)]` to `StreamDefinition`**
- [ ] **Step 3: Run `cargo test -p nebudcs-model` and verify all tests pass**

---

### Task 2: Hydraulic Network Solver Module (`nebudcs-physics::network`)
**Files:**
- Create: `crates/nebudcs-physics/src/network/mod.rs`
- Create: `crates/nebudcs-physics/src/network/node.rs`
- Create: `crates/nebudcs-physics/src/network/edge.rs`
- Create: `crates/nebudcs-physics/src/network/solver.rs`
- Create: `crates/nebudcs-physics/src/network/diagnostics.rs`
- Create: `crates/nebudcs-physics/src/network/recycle.rs`
- Modify: `crates/nebudcs-physics/src/lib.rs`

**Interfaces:**
- Consumes: `ChurchillFrictionFactor`, `ControlValveHydraulics`, `pipe_pressure_drop`, `NewtonSolver`, `RecycleSolver`, `PhysicalState`, `PropertyPackage`.
- Produces: `HydraulicNetwork`, `HydraulicNetworkSolver`, `HydraulicNode`, `HydraulicEdge`, `NetworkDiagnostics`, `RecycleConvergenceManager`.

- [ ] **Step 1: Implement `node.rs` and `edge.rs` with Darcy-Weisbach, minor loss $K$, elevation $\rho g \Delta z$, valve $C_v$, and pump head**
- [ ] **Step 2: Implement `solver.rs` with nodal pressure formulation and `NewtonSolver` integration**
- [ ] **Step 3: Implement `diagnostics.rs` for velocity, Reynolds number, friction factor, residual norm, and convergence status**
- [ ] **Step 4: Implement `recycle.rs` with Wegstein acceleration and damping for recycle loops**
- [ ] **Step 5: Export `network` module in `crates/nebudcs-physics/src/lib.rs`**
- [ ] **Step 6: Run `cargo test -p nebudcs-physics` to verify compilation and baseline integrity**

---

### Task 3: Generic Physics System & Equipment Coupling
**Files:**
- Modify: `crates/nebudcs-physics/src/system.rs`
- Modify: `crates/nebudcs-physics/src/equipment.rs`

**Interfaces:**
- Consumes: `HydraulicNetwork`, `HydraulicNetworkSolver`, `GenericEquipmentPhysics`.
- Produces: `GenericPhysicsSystem::network()`, `GenericPhysicsSystem::network_mut()`, dynamic network solution in `GenericPhysicsSystem::step()`.

- [ ] **Step 1: Integrate network solving into `GenericPhysicsSystem::step()`**
- [ ] **Step 2: Couple pump head, valve opening, accumulator pressures, and compressor recycle into network nodes and edges**
- [ ] **Step 3: Update equipment states and DCS tags with hydraulic network outputs (pressures, flows, velocities, NPSH margins)**
- [ ] **Step 4: Run `cargo test -p nebudcs-physics`**

---

### Task 4: Server REST API & WebSocket Diagnostics
**Files:**
- Modify: `crates/nebudcs-server/src/api/mod.rs` (or `project.rs` / `sim.rs`)
- Test: `crates/nebudcs-server/tests/`

**Interfaces:**
- Produces: REST API endpoint `/api/v1/network/diagnostics` returning `NetworkDiagnostics`.

- [ ] **Step 1: Add network diagnostics endpoint in `nebudcs-server`**
- [ ] **Step 2: Run `cargo test -p nebudcs-server`**

---

### Task 5: Scientific Validation & Generic Acceptance Plant Tests
**Files:**
- Create: `crates/nebudcs-physics/tests/hydraulic_network_tests.rs`

**Interfaces:**
- Verifies:
  - Darcy-Weisbach across laminar, transition, turbulent regimes.
  - Mass conservation ($\sum \dot{m}_{in} = \sum \dot{m}_{out}$).
  - Energy conservation and recycle loop convergence.
  - Plant A: Branched hydraulic network (Tank -> Pump -> Junction -> 2 Valves -> 2 Tanks).
  - Plant B: Pump + NPSH cavitation causal chain (Tank level drop -> NPSHA drop -> cavitation -> vibration surge).
  - Plant C: Heat + thermodynamics + recycle loop convergence (Feed -> Pump -> HX -> CSTR -> Separator -> Recycle).
  - Plant D: Compressor network + Stage 5 anti-surge interaction.
  - 10,000-tick bit-identical determinism test.

- [ ] **Step 1: Implement Darcy-Weisbach and mass conservation tests**
- [ ] **Step 2: Implement Acceptance Plant A (branched network)**
- [ ] **Step 3: Implement Acceptance Plant B (pump NPSH cavitation chain)**
- [ ] **Step 4: Implement Acceptance Plant C (thermodynamics + recycle convergence)**
- [ ] **Step 5: Implement Acceptance Plant D (compressor network + anti-surge)**
- [ ] **Step 6: Implement 10,000-tick determinism test**
- [ ] **Step 7: Run `cargo test --test hydraulic_network_tests` and verify all tests pass**

---

### Task 6: Engineering Studio Frontend Integration
**Files:**
- Modify: `web/src/components/nebu-studio.ts`
- Modify: `web/src/components/nebu-canvas.ts` (if needed)

**Interfaces:**
- Produces: Hydraulic network diagnostics inspector in NebuStudio showing node pressures, branch flows, velocity, Reynolds number, friction factor, and convergence status.

- [ ] **Step 1: Add hydraulic network diagnostics view/tab in `nebu-studio.ts`**
- [ ] **Step 2: Run `npm --prefix web test` and `npm --prefix web run build`**

---

### Task 7: End-to-End Verification, Screenshots & Regression Gate
**Files:**
- Create: `tests/verify_stage_6_advanced_physics.py`
- Output: 8 screenshots `stage_6_01_branched_network` to `stage_6_08_studio_hydraulic_diagnostics`

**Interfaces:**
- Verifies live system behavior on `http://127.0.0.1:8088`.

- [ ] **Step 1: Build release binary and restart `nebudcs.service`**
- [ ] **Step 2: Execute `tests/verify_stage_6_advanced_physics.py` and capture 8 screenshots**
- [ ] **Step 3: Run full regression: `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm test`**
- [ ] **Step 4: Run Stage 4 and Stage 5 regression verification scripts**
- [ ] **Step 5: Run `graphify update .`**

---

### Task 8: Stage 6 Comprehensive Report & Sign-off
**Files:**
- Create: `stage_6_advanced_process_physics_report.md`

- [ ] **Step 1: Author 19-section comprehensive report**
- [ ] **Step 2: Deliver final verdict: `STAGE 6 ADVANCED PROCESS PHYSICS: PASS`**
