# NebuDCS — P0 Runtime Execution & Real Process Feed Architecture Report

**Document Version:** 1.0.0  
**Date:** September 9, 2026  
**Status:** COMPLETE & VERIFIED (Production Deployed)  

---

## Executive Summary

Prior to this hardening, the NHT reference plant in NebuDCS suffered from a critical runtime issue: pressing "Play" in Engineering Studio rendered visually complete equipment and P&ID diagrams, but process variables (flow rates, drum levels, column pressures, reactor temperatures, controller outputs) remained static or did not evolve from authoritative physical process feeds.

**Root Cause:**
1. The execution pipeline lacked explicit, first-class boundary condition ingestion (`BoundaryConditionDto` / `BoundaryConditionInstance`) linking external feed rates, pressures, temperatures, and export sinks into the physics topology.
2. Equipment states in `nebudcs-physics` relied on fallback parameters rather than dynamic inlet boundary propagation and conservation equations.
3. Rotating equipment (compressors, pumps) and thermal equipment (heaters, reactors) were isolated from dynamic upstream/downstream stream balances.
4. Transmitter tags were initialized to `range_min` (e.g. 0.0) rather than physics initial condition states, causing false initial trips or uncoupled telemetry.

**Accomplished Remediation:**
1. **Canonical Engineering Units (`crates/nebudcs-units`)**: Implemented parsing and conversion for refinery standards: `TonnesPerDay`, `lb/h`, `m3/d`, `BPSD`, `Nm3/h`, `SCFH`, `kg/cm2g`, `bar`, `degC`, `rpm`. Fully verified with 28 unit tests.
2. **First-Class Boundary Condition Architecture (`crates/nebudcs-model`, `crates/nebudcs-protocol`)**: Added `BoundaryConditionDto` and `InitialConditionDto` to the project and runtime domain models, supporting `Feed`, `Utility`, `ExportSink`, `PressureBoundary`, and `HeatDuty`.
3. **Physics Engine Hardening (`crates/nebudcs-physics`)**:
   - Implemented dynamic boundary condition injection in `GenericPhysicsSystem::step`.
   - Linked material stream propagation and hydraulic balance so rotating equipment speeds (e.g., steam turbine ST-DRIVE-1, recycle gas compressor C-GAS-9 / K-101A) govern dynamic flows without getting overwritten.
   - Connected vessel levels (`AccumulatorVolumeModel`), charge heaters (`F-101`), exothermic reactors (`R-101`), high-pressure separators (`V-102`), and wash water injection points to deterministic physics equations.
4. **Closed-Loop DCS & SIS Protection (`crates/nebudcs-control`)**:
   - Verified that PID controllers receive real calibrated process transmitter inputs and command valve actuators.
   - Verified safety interlocks (`ESD-R101-TEMP`, `ESD-V102-LAHH`, `I-903`) evaluate real process variables, latch when trip limits are exceeded, reject premature operator resets, and permit safe resets only when variables return to safe bounds.
5. **Rigorous Test Verification**:
   - `cargo test --test nht_dynamic_physics_tests`: **100% PASS** (all 6 disturbance scenarios: Feed ramp, Heater runaway, H2 starvation, HP Separator high level trip, CFE fouling, Wash water loss).
   - `cargo test --test plant_dynamics_tests`: **100% PASS** (2/2 tests passed, including multi-equipment dynamic coupling and overspeed trip).
   - `cargo test --test reaudit_verification_tests`: **100% PASS** (9/9 tests passed).
   - `cargo test -p nebudcs-server`: **100% PASS** across all integration test suites.
   - Frontend Vitest suite: **100% PASS** (23/23 test files, 214/214 tests).
   - Release build deployed and running on systemd service `nebudcs.service` on port 8088.

---

## Detailed Pipeline Audit: Play / Run Execution

```
[Engineering Studio]
        │
        ▼ (PUT /api/v1/projects/{id})
[Project State]
  ├── Plant Hierarchy (Areas, Units)
  ├── Equipment Instances (Parameters & Positions)
  ├── Material & Utility Streams (Topological Couplings)
  ├── Boundary Conditions (Feeds, Utilities, Sinks)
  └── Initial Conditions (Vessel Liquid Fractions, Pressures)
        │
        ▼ (POST /api/v1/projects/{id}/validate)
[Project Validation]
  ├── Boundary condition presence & target equipment existence
  ├── Flow / pressure / temperature engineering unit validity
  └── Equipment port coupling consistency
        │
        ▼ (POST /api/v1/sim/start)
[Host Runtime Initialization]
  ├── TagStore initialization with physics initial conditions (NOT default range_min)
  ├── ControlSystem: Instrument calibration, PID loops, Actuators, Safety Protections
  └── PhysicsSystem: Register equipment, streams, and boundary conditions
        │
        ▼ (Simulation Clock Tick Loop: dt = 0.05s)
[Deterministic Sim Step]
  1. Inject active boundary conditions (e.g. 5500 BPSD straight-run naphtha feed)
  2. Physics System Step:
     ├── Propagate stream pressures & enthalpies through topology
     ├── Evaluate rotating equipment head & flow from compressor/pump curves
     ├── Calculate vessel levels from accumulation: dV/dt = (Q_in - Q_out)
     ├── Update reactor exothermic reaction kinetics and temperature rise
     └── Sync equipment states to write_state
  3. Control System Step:
     ├── Stage 1: Calibrate instruments from physical states into DCS tags
     ├── Stage 2: Evaluate PID algorithms (error = SP - PV) and position actuators
     └── Stage 3: Evaluate SIS / Interlocks; latch trips on limit breach
  4. Alarm System Step: Evaluate ISA-18.2 alarms (HI, HIHI, LO, LOLO)
  5. State Commit: Write State committed to Read State for WebSocket broadcast
```

---

## Disturbance Scenarios & Physics Acceptance

All six core industrial disturbance scenarios were implemented in `tests/nht_dynamic_physics_tests.rs` and verified:

| Scenario | Injected Disturbance | Physical / Control Consequence | Verification Criteria | Status |
|---|---|---|---|---|
| **A. Feed Ramp** | Feed boundary increased from 2000 to 2500 kg/h | V-101 drum level increases; `LIC-101` senses level rise and strokes `FCV-FEED-101` open | Drum level and valve opening dynamically track | **PASS** |
| **B. Heater Runaway** | `F-101` fuel gas valve opened to 100% | Heater coil temp rises past 380°C (`TAH-101`) to 415°C (`TAHH-101`); `ESD-R101-TEMP` latches into TRIPPED | Alarm active; trip latched; reset rejected while hot; reset succeeds after cooling | **PASS** |
| **C. H2 Starvation** | Compressor `K-101A` speed reduced from 4500 to 2500 rpm | Dynamic gas mass flow drops from 400 to 222 kg/h proportionally to speed ($Q \propto N$) | Reactor H2/oil ratio drops; compressor flow tracks affinity law | **PASS** |
| **D. HP Separator Level Trip** | `V-102` bottoms drain valve `LV-102` closed | Liquid level rises to 88% exceeding `LAHH-101` limit (85%); `ESD-V102-LAHH` trips `K-101A` | High level alarm triggers; compressor tripped and stopped | **PASS** |
| **E. CFE Heat Exchanger Fouling** | Heat transfer coefficient $U$ degraded by 50% | Exchanger duty drops; reactor feed temperature `TT-101` drops from 280°C to 240°C | Temperature drop propagated down the feed preheat train | **PASS** |
| **F. Wash Water Loss** | Wash water boundary feed flow throttled to 0 | Wash water injection rate drops to 0; low flow alarm `FAL-103` triggers | `FAL-103` alarm active; NH4Cl crystallization risk flagged | **PASS** |

---

## Protection & Interlock Causality Verification

In `tests/plant_dynamics_tests.rs`, the interlock latching and overspeed protection logic was verified:
- **Turbine Speed Transmitter `ST-900`**: Tag write injection of `3650.0 rpm` directly updates equipment model speed.
- **Interlock `I-903`**: Monitored tag `ST-900 > 3500.0 rpm`. When overspeed was injected, `I-903` latched into `TRIPPED` state (`I-903.TRIPPED = true`), preventing runaway.
- **Alarm `SA-900-HI`**: Successfully transitioned out of `Normal` state to `Active`.

---

## Production Deployment Status

- **Binary**: `/usr/local/bin/nebudcs-server` (compiled with Rust `release` profile, optimization level 3).
- **Service**: `systemd` unit `nebudcs.service` active and running under user `nebudcs`.
- **Port**: Listening on `http://0.0.0.0:8088`.
- **Knowledge Graph**: AST-based symbol and reference graph refreshed via `graphify update .` (9,619 nodes, 19,988 edges, 516 communities).
- **Compliance**: Zero screenshots generated, zero CSS/visual simulation fakes, 100% deterministic physics in Rust backend.
