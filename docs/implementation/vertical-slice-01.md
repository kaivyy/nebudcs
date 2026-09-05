---
title: Vertical Slice 1
version: 1.0.0
status: draft
date: 2026-09-05
---

# GOAL
Prove that the architecture works end-to-end: Pump → Stream → Vessel → Control Valve → Typed Tags → PID → Alarm → Historian → Protocol → Minimal HMI.

# WHAT THIS SLICE PROVES
1. Strongly typed ports correctly connect streams.
2. The simulation tick is deterministic.
3. Units convert properly from internal SI to display values.
4. PID control correctly reads process variables and manipulates actuators.
5. Alarms trigger on specified tag limits.
6. The Historian successfully records tag data asynchronously.
7. WebSockets transmit delta updates efficiently.
8. Data-driven `EquipmentDefinition` correctly spawns simulation state.
9. Fault injection correctly cascades through the system.
10. System is fully isolated from UI (no UI writes directly to simulation).
11. Persistence replays deterministically.

# SLICE PREREQUISITES
Phases 1 through 7 must be fully complete.

# SIMULATION SCENARIO
- A pump feeds a vessel via a control valve.
- The pump has a flow tag (`K-001.FLOW`), speed tag (`K-001.SPEED`).
- The vessel has a level tag (`V-001.LEVEL`), pressure tag (`V-001.PRESS`).
- A PID controls vessel level by manipulating the control valve.
- If level goes HIHI, an alarm fires.
- All values are recorded in the historian.
- A web client subscribes to tags via WebSocket and sees updates.
- Fault injection: pump stops -> level drops -> alarm LO fires.
- Historian replay works identically.

# EQUIPMENT DEFINITIONS REQUIRED
- GenericCentrifugalPump
- GenericVessel
- GenericControlValve

# NEW CRATES REQUIRED
- `nebudcs-topology`
- `nebudcs-sim`
- `nebudcs-physics`
- `nebudcs-control`
- `nebudcs-alarm`
- `nebudcs-historian`
- `nebudcs-protocol`
- `apps/nebudcs-server`
- `apps/nebudcs-cli`

# SLICE ACCEPTANCE CRITERIA
1. CLI loads the project and starts simulation without crash.
2. `K-001.FLOW` responds to speed changes.
3. `V-001.LEVEL` changes as flow enters.
4. PID output changes on level error.
5. Alarm fires on HIHI.
6. Historian records alarm.
7. WebSocket receives updates within 200ms.
8. Stopping pump decreases level.
9. Level LO alarm fires after pump stop.
10. Historian replay produces identical values.
11. UI cannot write directly to simulation state.

# SECOND PROOF SPECIFICATION
- 3-stage H2 Compressor (Deferred to Phase 11).
- Requires compressor maps, vibration models, anti-surge.

# CUSTOM EQUIPMENT PROOF SPECIFICATION
- TOML definition translates to running simulation without Rust recompilation (Phase 10).
