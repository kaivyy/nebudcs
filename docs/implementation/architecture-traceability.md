---
title: Architecture Traceability
version: 1.0.0
status: draft
date: 2026-09-05
---

# Architecture Consistency Audit

- **Contradictions found:** None critical. The PRD and architecture align on the data-driven approach.
- **Missing implementation-critical decisions:** Exact serialization boundary for WebSocket messages (added to Phase 7).
- **Premature architecture decisions:** None detected.
- **Insufficiently specified decisions:** `EquipmentDefinition` schema required deeper specification (addressed in Phase 1).
- **Decisions verified as sound:** The separation of `EquipmentDefinition` from `RuntimeState` is robust.
- **Custom-equipment/no-recompile requirement verification:** Verified. Level 1/2 customization via TOML/JSON definitions works without recompiling the core.
- **Deterministic simulation boundary verification:** Verified. The tick loop relies solely on `SimulationTime`.
- **Same-model-for-all-modes verification:** Verified. The core engine does not branch for training vs. engineering modes.
- **Units, topology, equipment, runtime state, controls, validation ownership verification:** Verified. Ownership is strictly stratified.

# PRD Delta
No major PRD updates required. Minor clarifications added to the data structure requirements in Section 6.

# Requirements Traceability Matrix

| PRD Requirement | Architecture Doc | Implementation Phase | Classification |
|-----------------|------------------|----------------------|----------------|
| Core Types | domain-model.md | Phase 1 | REQUIRED NOW |
| Unit System | units.md | Phase 1 | REQUIRED NOW |
| Plant Hierarchy | domain-model.md | Phase 1 | REQUIRED NOW |
| Topology Graph | topology.md | Phase 2 | REQUIRED SOON |
| Deterministic Sim | simulation-engine.md | Phase 3 | REQUIRED SOON |
| Basic Physics | physics.md | Phase 4 | REQUIRED SOON |
| Controls/PID | control-engine.md | Phase 5 | REQUIRED SOON |
| Alarms | alarm-historian.md | Phase 5 | REQUIRED SOON |
| Historian | alarm-historian.md | Phase 6 | REQUIRED SOON |
| Network Protocol | protocol.md | Phase 7 | REQUIRED SOON |
| Scenario Engine | scenario-engine.md | Phase 9 | EXTENSION POINT |
| Custom Equipment | custom-equipment.md | Phase 10 | DEFERRED |
| Standard Library | equipment-sdk.md | Phase 11 | DEFERRED |
| Adv. Thermodynamics | thermodynamics.md | Phase 12 | FUTURE |
| Refinery Templates | PRD Section 6 | Phase 13 | FUTURE |
