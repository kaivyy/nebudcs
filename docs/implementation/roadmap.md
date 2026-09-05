---
title: Implementation Roadmap
version: 1.0.0
status: draft
date: 2026-09-05
---

# Phase 0: Repository Foundation
- **Objective:** Set up Cargo workspace, CI, tooling. No domain logic yet.
- **Prerequisites:** None.
- **Scope:** Workspace Cargo.toml, rustfmt, clippy, basic CI actions.
- **Non-scope:** Any Rust code.
- **Crates:** Workspace root.
- **Architecture:** ADR-001, ADR-009.
- **Tests:** CI pipeline runs.
- **Exit criteria:** `cargo build` and `cargo test` pass on empty workspace.

# Phase 1: Core Types and Domain Model
- **Objective:** Establish the irreducible type foundation that every other crate depends on.
- **Prerequisites:** Phase 0.
- **Scope:** `nebudcs-types`, `nebudcs-units`, `nebudcs-model`. `EntityId`, quantities, equipment definitions.
- **Non-scope:** Topology, simulation runtime.
- **Crates:** `nebudcs-types`, `nebudcs-units`, `nebudcs-model`.
- **Architecture:** domain-model.md, units.md.
- **Exit criteria:** All types serialize/deserialize correctly; `cargo test` passes.

# Phase 2: Topology and Port System
- **Objective:** Graph-based topology engine with typed ports, streams, and validation.
- **Prerequisites:** Phase 1.
- **Scope:** Port types, MaterialStream, UtilityStream, graph adjacency.
- **Crates:** `nebudcs-topology`.

# Phase 3: Simulation Clock and Runtime State
- **Objective:** Deterministic simulation clock, tick loop, runtime state buffer.
- **Prerequisites:** Phase 2.
- **Scope:** Simulation loop, tick management.
- **Crates:** `nebudcs-sim`.

# Phase 4: Basic Physics
- **Objective:** Simplified physics sufficient for a pump + vessel + control valve.
- **Prerequisites:** Phase 3.
- **Scope:** Ideal Gas, mass balance, simplified hydraulics.
- **Crates:** `nebudcs-physics`.

# Phase 5: Control Engine
- **Objective:** PID, AUTO/MAN, alarm engine, basic interlock.
- **Prerequisites:** Phase 4.
- **Scope:** PID algorithms, interlocks.
- **Crates:** `nebudcs-control`, `nebudcs-alarm`.

# Phase 6: Historian
- **Objective:** Tag value recording, SQLite persistence, downsampling.
- **Prerequisites:** Phase 5.
- **Scope:** SQLite writes, querying.
- **Crates:** `nebudcs-historian`.

# Phase 7: Protocol Layer
- **Objective:** WebSocket subscription model, REST API types.
- **Prerequisites:** Phase 6.
- **Scope:** Network DTOs, connections.
- **Crates:** `nebudcs-protocol`, `apps/nebudcs-server`.

# Phase 8: Vertical Slice 1
- **Objective:** End-to-end proof that the full stack works from simulation to UI.
- **Prerequisites:** Phase 7.
- **Scope:** Pump → Vessel → PID → Alarm → Web UI.

# Phase 9: Scenario and Fault Engine
- **Objective:** Data-driven scenarios, fault injection into runtime model.
- **Prerequisites:** Phase 8.

# Phase 10: Custom Equipment Proof
- **Objective:** Level 1/2/3 custom equipment from data definitions.
- **Prerequisites:** Phase 9.

# Phase 11: Standard Equipment Library
- **Objective:** Generic implementations of the standard equipment types from PRD.

# Phase 12: Advanced Physics and Thermodynamics
- **Objective:** Pluggable PropertyPackage upgrades.

# Phase 13: Refinery Unit Templates
- **Objective:** NHT, CDU, etc., as data packages.

# Phase 14: Desktop Engineering Station
- **Objective:** egui/eframe P&ID editor.

# Phase 15: Hardening and Optimization
- **Objective:** Performance tuning, security.
