# NebuDCS Stage 4: Unified PLC/DCS Engineering Studio Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unify process simulation, controller runtime, IEC 61131-3 logic runtime, I/O HAL, P&ID flowsheet editor, instrumentation, controls, alarms, and project persistence into a coherent, vendor-neutral industrial engineering studio.

**Architecture:** Approach 1 (Modular Unified Project Hierarchy). Extend canonical `Project` and `ProjectDetailDto` in Rust with `controllers`, `io_systems`, `io_mappings`, and `tags` using `#[serde(default)]`. Enable project-driven runtime deployment in `apply_load_project` dynamically instantiating virtual controllers, tasks, IEC programs, and HAL mappings. In the frontend, enhance `<nebu-studio>` with a left-side Project Explorer tree and central workspace tabs (P&ID Flowsheet, IEC Logic Editor, I/O System, I/O Mapping Table, Tag Database, Cross-Reference).

**Tech Stack:** Rust (Axum, Tokio, Serde), TypeScript, Web Components, Vite, Vitest, Playwright/Python.

## Global Constraints

- Baseline: Phases 0–16, Controlled Expansion, Domain Packages, Stage 1 (PASS), Stage 2 (PASS), Stage 3 (PASS).
- Do NOT create Phase 17.
- Do NOT create duplicate simulation engines, controller runtimes, schedulers, IEC runtimes, or I/O HALs.
- Single-threaded Rust simulation engine remains the sole authoritative physics and control executor.
- Rust remains authoritative for validation, compilation, deployment, and persistence; frontend is an engineering interface only.
- 100% backward compatibility: existing legacy projects without controller/HAL sections must load cleanly without regressions.
- No refinery-specific hardcoded IDs or topologies.

---

### Task 1: Canonical Domain Model Extensions & Backward Compatibility

**Files:**
- Modify: `crates/nebudcs-model/src/project.rs`
- Modify: `crates/nebudcs-protocol/src/dto.rs`
- Modify: `crates/nebudcs-protocol/src/lib.rs`

**Interfaces:**
- Produces: `ProjectControllerConfig`, `ProjectTaskConfig`, `ProjectProgramConfig`, `ProjectTagDto`, `CrossReferenceResultDto`
- Extends: `Project`, `ProjectDetailDto` with `controllers`, `io_systems`, `io_mappings`, `tags`

- [ ] **Step 1: Write model tests in `crates/nebudcs-model/src/project.rs`**
  - Verify that legacy JSON projects without controllers/io_systems deserialize with empty defaults.
  - Verify that complete unified projects with controllers, tasks, and programs serialize and deserialize identically.
- [ ] **Step 2: Run test to verify failure**
- [ ] **Step 3: Implement struct definitions in `crates/nebudcs-model/src/project.rs` and `crates/nebudcs-protocol/src/dto.rs`**
- [ ] **Step 4: Run test to verify pass**
- [ ] **Step 5: Verify workspace compiles cleanly**

---

### Task 2: Project-Driven Simulation Deployment & Transactional Synchronization

**Files:**
- Modify: `apps/nebudcs-server/src/project_manager.rs`
- Modify: `apps/nebudcs-server/src/host.rs`
- Modify: `crates/nebudcs-control/src/system.rs`

**Interfaces:**
- Consumes: `ProjectDetailDto` (controllers, io_systems, io_mappings)
- Produces: Dynamic instantiation of `VirtualController`, `ControllerTask`, `IecProgram` (ST/LD/FBD/SFC), `IoSystem`, `IoMappingTable` inside `apply_load_project`

- [ ] **Step 1: Write integration test in `apps/nebudcs-server/tests/project_simulation_deployment_tests.rs`**
  - Define a test project with an authored controller, periodic task, and ST IEC program writing `%QW100 := %IW100 * 2.0;`.
  - Verify that loading the project into simulation registers the virtual controller, links the HAL channel, and updates output on simulation tick.
- [ ] **Step 2: Run test to verify failure**
- [ ] **Step 3: Implement project-driven deployment in `host.rs::apply_load_project`**
  - Iterate `project.controllers`: instantiate `VirtualController`, add `ControllerTask`s with periods and priorities, compile `IecRoutine`s into `IecProgram`, add to tasks, register on `control_system`.
  - Iterate `project.io_systems`: instantiate `IoSystem`, racks, chassis, modules, channels, or fallback to default if empty.
  - Iterate `project.io_mappings`: register input bindings and output actuator mappings.
  - Enforce transactional safety: if compilation or mapping validation fails, abort without modifying running simulation.
- [ ] **Step 4: Run test to verify pass**

---

### Task 3: Authoritative Unified Validation & Cross-Reference Engine

**Files:**
- Modify: `crates/nebudcs-topology/src/validation.rs`
- Modify: `apps/nebudcs-server/src/project_manager.rs`
- Modify: `apps/nebudcs-server/src/routes/projects.rs`

**Interfaces:**
- Produces: `GET /api/v1/projects/{id}/cross-reference`, comprehensive `validate_project`

- [ ] **Step 1: Write tests for project validation & cross-reference**
  - Verify invalid IEC syntax in routine is caught during project validation.
  - Verify duplicate output ownership is caught during validation.
  - Verify cross-reference lookup finds definition, readers, writers, I/O channel, and controller for a given tag.
- [ ] **Step 2: Run test to verify failure**
- [ ] **Step 3: Implement cross-reference indexer and unified validation**
- [ ] **Step 4: Run test to verify pass**

---

### Task 4: Engineering Studio Project Explorer & Multi-View Workspace

**Files:**
- Create: `web/src/components/nebu-project-explorer.ts`
- Modify: `web/src/components/nebu-studio.ts`
- Modify: `web/src/state/studio-store.ts`
- Modify: `web/src/protocol/types.ts`
- Modify: `web/src/api/client.ts`

**Interfaces:**
- Produces: `<nebu-project-explorer>` Web Component
- Connects: Project tree navigation to workspace view switcher (P&ID Flowsheet, Logic Editor, I/O System, I/O Mappings, Tag Database, Cross-Reference)

- [ ] **Step 1: Write frontend tests in `web/tests/studio.test.ts`**
  - Test project explorer tree renders canonical nodes (Plant, Controllers, I/O Systems, Tags).
  - Test clicking a controller or routine selects it and switches workspace tab.
- [ ] **Step 2: Run test to verify failure**
- [ ] **Step 3: Implement `<nebu-project-explorer>` and workspace tab switcher in `<nebu-studio>`**
- [ ] **Step 4: Run test to verify pass**

---

### Task 5: Studio Engineering Panels (I/O Mapping Table, Tag Database, Cross-Reference)

**Files:**
- Create: `web/src/components/nebu-io-mapping-table.ts`
- Create: `web/src/components/nebu-tag-database.ts`
- Create: `web/src/components/nebu-cross-reference.ts`
- Modify: `web/src/components/nebu-iec-editor.ts`

**Interfaces:**
- Produces: `<nebu-io-mapping-table>`, `<nebu-tag-database>`, `<nebu-cross-reference>`

- [ ] **Step 1: Write component tests in `web/tests/components.test.ts`**
- [ ] **Step 2: Implement mapping table, tag database, and cross-reference views**
- [ ] **Step 3: Connect to live API client and studio store**
- [ ] **Step 4: Run test to verify pass**

---

### Task 6: Generic Plant Acceptance Configurations & Verification Script

**Files:**
- Modify: `apps/nebudcs-server/src/project_manager.rs`
- Create: `tests/verify_stage_4_engineering_studio.py`

**Interfaces:**
- Produces: Generic demonstration plants (Plant A: Pumping/FT/ST logic; Plant B: Exothermic Reactor/LD interlock; Plant C: Steam Turbine/Compressor/FBD regulation) pre-seeded and verified.

- [ ] **Step 1: Seed generic plants with complete Controller, Task, IEC Routine, and I/O mapping definitions**
- [ ] **Step 2: Write comprehensive Python verification script `tests/verify_stage_4_engineering_studio.py`**
- [ ] **Step 3: Execute verification script against live server and confirm 100% pass**

---

### Task 7: Full Workspace Regression Gates

- [ ] **Step 1: Run `cargo test --workspace` (all 13 crates pass)**
- [ ] **Step 2: Run `cargo clippy --workspace --all-targets -- -D warnings` (0 warnings)**
- [ ] **Step 3: Run `cargo fmt --check` (clean format)**
- [ ] **Step 4: Run `npm --prefix web test` (all files pass)**
- [ ] **Step 5: Run `npm --prefix web run build` (clean Vite build)**
- [ ] **Step 6: Run `python3 tests/verify_industrial_domain_packages.py` (PASS)**
- [ ] **Step 7: Run `python3 tests/verify_generic_plant_acceptance.py` (PASS)**
- [ ] **Step 8: Deploy release binary, restart service, verify live health & endpoints**

---

### Task 8: Stage 4 Engineering Studio Verification Report

**Files:**
- Create: `/root/NebuDCS/stage_4_engineering_studio_report.md`
- Create artifact: `/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/stage_4_engineering_studio_report.md`

- [ ] **Step 1: Compile comprehensive 27-section report**
- [ ] **Step 2: Verify all criteria satisfied**
- [ ] **Step 3: Issue authoritative verdict: `STAGE 4 ENGINEERING STUDIO: PASS`**
