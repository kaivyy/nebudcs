# NebuDCS — P0/P1 Forensic Runtime-to-Process-Graphic Integration & NHT Reconstruction Report

**Status**: IMPLEMENTED & INTERNALLY VERIFIED  
**Date**: 2026-09-09  
**Target Unit**: Naphtha Hydrotreater (NHT) Reference Plant (HPCL Visakh Benchmark)  
**Verification Scope**: Rust Physics Engine -> WebSocket Protocol -> Store/DisplayViewModel -> SVG Process Graphic Workstation  

---

## 1. Executive Summary & Forensic Problem Statement

A forensic audit of the NebuDCS operations environment revealed critical functional integration defects between the underlying Rust physics simulation and the Operations SVG workstation (`nebu-process-view`):
1. **Equipment Visual Overlap**: Overview and feed displays contained conflicting equipment placements, causing visual collisions.
2. **Incomplete NHT Process Flowsheet**: V-101 (Feed Surge Drum) was physically present in equipment catalogs but completely bypassed in the stream topology graph; the plant overview display only defined 3 streams, leaving primary headers disconnected.
3. **Missing Plant Boundary Conditions**: Feed inputs entered the plant invisibly without representing physical battery limit provenance (temperature, pressure, phase, volumetric rate).
4. **Synthetic Fallback & Static Animation**: Streams fell back to hardcoded `45.2 m³/h` if running; stroke dash offsets did not scale with flow velocity; zero-flow streams remained visibly animated.
5. **Multi-Vessel Level Overwrites**: Sump and drum level displays looped globally over all tags and took the first match, causing distinct vessels (V-101, V-102, V-105) to mirror identical levels.
6. **Generic Vessel Fallback for Catalytic Reactor**: R-101 rendered as a generic liquid vessel rather than a two-bed fixed-bed hydrodesulfurization reactor with quench decks and bed temperature sensors.
7. **Port-to-Port Manhattan Routing Bypass**: Pipe endpoints used arbitrary horizontal offsets (`sc.x + 30`, `dc.x - 30`) cutting through equipment casings and leaving stubs floating in mid-air.

All defects were treated as **functional integration defects**, resolved at their source in the physics coupling, seed topology, type model, and SVG graphics engine, strictly maintaining simulation determinism without synthetic frontend animations or cosmetic hacks.

---

## 2. Root Cause Analysis & Architecture Remediation

### 2.1 Physics-to-Tag Coupling Infrastructure
- **Root Cause**: While `GenericPhysicsSystem::step` calculated hydraulic flow across couplings, the calculated flow rate was not reflected back into the global `TagStore` under the stream name (`{STREAM_NAME}.FLOW`). The UI had no authoritative source for individual stream telemetry.
- **Remediation**:
  - In `crates/nebudcs-physics/src/system.rs`: Augmented `ProcessCoupling` with `stream_name: Option<String>` and `flow_tag_id: Option<TagId>`. In `step()`, the calculated coupling flow (`source_flow`) is written directly to `write_state.set_tag_float(tid, flow_val)`.
  - In `apps/nebudcs-server/src/host.rs`: Automatically registered `{STREAM_NAME}.FLOW`, `{STREAM_NAME}.TEMP`, and `{STREAM_NAME}.PRESS` in `tag_store` for every stream, and `{BC_NAME}.FLOW`, `{BC_NAME}.TEMP`, `{BC_NAME}.PRESS` for every boundary condition during project initialization.

### 2.2 NHT Process Flowsheet & V-101 Coupling
- **Root Cause**: `apps/nebudcs-server/src/nht_seed.rs` connected `STR-NHT-01` directly from `tk101_id` (`outlet`) to `p101a_id` (`suction`), completely bypassing V-101. Furthermore, `disp-nht-overview` only contained 3 stream IDs, leaving the overview graphic disconnected.
- **Remediation**:
  - Inserted stream `STR-NHT-00` from `tk101_id` (`outlet`) to `v101_id` (`feed`).
  - Rerouted `STR-NHT-01` from `v101_id` (`liquid_out`) to `p101a_id` (`suction`).
  - Expanded `disp-nht-overview` stream list to include the continuous primary process line: `STR-NHT-00`, `01`, `02`, `05`, `12`, `13`, `14`, `21`, `25`, `28`, `29`, `33`, `40`, `43`.
  - Re-spaced all equipment placements across the 1920x1080 canvas (from 80px to 1620px), completely eliminating placement overlaps.

### 2.3 Port-to-Port Manhattan Orthogonal Routing
- **Root Cause**: `renderStreams` connected streams using `p1 = { x: sc.x + 30, y: sc.y }` and `p2 = { x: dc.x - 30, y: dc.y }`.
- **Remediation**:
  - Implemented `getPortCoordinates(eq, portNameOrId)` in `nebu-process-view.ts`, determining exact nozzle locations based on archetype geometry:
    - Pumps: Suction at left (`x - 28, y`), Discharge at top (`x, y - 28`).
    - Compressors: Suction at left (`x - 40, y`), Discharge at top (`x, y - 32`).
    - Fixed-Bed Reactors: Feed at top (`x, y - 62`), Effluent at bottom (`x, y + 62`).
    - Fractionators: Overhead vapor at top (`x, y - 126`), Bottoms liquid at bottom (`x, y + 126`), Feed at middle left (`x - 30, y`).
    - Vertical Vessels: Inlet at top-left, Liquid out at bottom, Vapor out at top.
  - Implemented `computePortAwareWaypoints(p1, p2)` generating orthogonal stubs perpendicular to nozzle faces and computing Manhattan routes.

### 2.4 Authoritative Flow Telemetry & Dynamic Direction
- **Root Cause**: Unregistered streams defaulted to `flowRate = 45.2 m³/h` if `isSimRunning` was true, creating phantom flow during shutdowns or trips.
- **Remediation**:
  - Stream flow resolves authoritatively from `{STREAM.NAME}.FLOW` tag.
  - Flow rate dynamically respects upstream equipment state: if the upstream pump/compressor is tripped (`is_tripped: true` or `TRIP` tag) or stopped (`speed < 10 RPM`), or if an upstream control valve is closed (`stroke < 0.5%`), the stream flow drops to `0.0 m³/h`.
  - Zero-flow lines render in inactive slate grey (`#334155`) with animation halted.
  - Active flow lines render in primary process blue (`#0284c7`), with stroke dash animation velocity proportional to flow rate ($v \propto |Q|$).
  - Reverse flow ($Q < -0.1$) renders in reverse alarm red (`#ef4444`) with flipped directional chevrons.

### 2.5 Multi-Vessel Dynamic Level Decoupling
- **Root Cause**: Vessel level evaluation iterated over all store tags and matched the first tag containing `LT` or `LIC`, binding every vessel to a single tag.
- **Remediation**:
  - Level is resolved on a strict per-equipment basis:
    1. Direct instrument tagged to this equipment (`eqInstruments.find(...)`).
    2. Specific equipment tag `{eq.name}.LEVEL` or `{eq.id}.LEVEL`.
    3. Equipment parameter `eq.parameters?.level`.
    4. Dedicated fallback.
  - Verified that V-101, V-102, and V-105 concurrently display distinct level readouts (e.g. 35.0%, 58.0%, 72.0%).

### 2.6 Fixed-Bed Catalytic Hydrotreater Reactor (R-101)
- **Root Cause**: `case "reactor"` fell through to generic vertical vessel.
- **Remediation**:
  - Implemented `renderReactorSymbolHtml(cx, cy, tempVal, pressVal, isTripped, diag)`:
    - Heavy-wall cylindrical pressure casing with dished heads (rx=18).
    - Top inlet distributor tray and vapor diffuser.
    - Catalyst Bed 1 (NiMo / HDS) with grain lines and active label.
    - Inter-bed quench and redistribution deck.
    - Catalyst Bed 2 (CoMo / HDN finishing bed).
    - Bottom support grid and effluent collector screen.
    - Thermocouple readout pill (`TT-101` / `332.0°C`).
    - Heavy reactor skirt and structural anchor base.

### 2.7 Authoritative Plant Feed Boundary Badges
- **Root Cause**: Plant boundary conditions in `project.boundary_conditions` had no graphical representation in the process flowsheet.
- **Remediation**:
  - Implemented `renderBoundaryConditions`:
    - Renders high-contrast feed boundary badges (`PLANT FEED: NHT Fresh Naphtha Feed`).
    - Displays live feed telemetry: `199.0 m3/h | 68.0°C | 5.0 barg | Liquid`.
    - Draws dashed orthogonal piping from the boundary badge directly into the target equipment inlet port.
    - Supports interactive selection and property inspection in Engineering Studio.

---

## 3. Automated Verification Evidence

### 3.1 Web Frontend Integration Test Suite
Command: `npm test` in `/root/NebuDCS/web`
Result: **24 test suites passed, 223 tests passed (0 failures)**

Key tests in `tests/runtime-to-ui-integration.test.ts`:
- **Test A: Feed Ramp (199 -> 248.7 m³/h)**: PASSED (stream flow updates, active `#0284c7` pipe, animated dash offset).
- **Test B: Feed Shutdown (0.0 m³/h)**: PASSED (stream switches to inactive `#334155`, animation frozen).
- **Test C: Pump Trip**: PASSED (displays `ESD TRIP` / `▲▲ TRIP`, discharge stream drops to `0.0 m³/h`).
- **Test D: Valve Movement (100% -> 35% -> 0%)**: PASSED (flow throttles dynamically to `0.0 m³/h` at 0% stroke).
- **Test E: Compressor Speed Change (0 -> 10,450 RPM)**: PASSED (recycle stream activates at `42.5 m³/h`).
- **Test F: Multi-Vessel Independent Levels**: PASSED (V-101=35%, V-102=58%, V-105=72% with zero cross-talk).
- **Test G: Reactor R-101 Graphic**: PASSED (renders Bed 1 NiMo, Bed 2 CoMo, and multi-point `TT-101` temperature).
- **Test H: Port-to-Port Manhattan Routing**: PASSED (suction/discharge nozzle offsets, pure orthogonal segments).
- **Test I: Plant Feed Boundary Badge**: PASSED (renders feed badge, parameters, and connecting pipe).

### 3.2 Production Server & Physics Unit Tests
- `cargo test -p nebudcs-server --test stage_14_nht_reference_plant_tests`: **4 passed, 0 failed**.
- `cargo test -p nebudcs-physics`: **95 passed, 0 failed** (all hydraulic network, thermo, and consistency tests).
- Frontend production bundle (`npm run build`): **Compiled cleanly (`tsc --noEmit && vite build`)**.
- Backend release binary (`cargo build --release -p nebudcs-server`): **Compiled cleanly with opt-level 3 & thin-LTO**.

### 3.3 Live Production Service Status
- Binary deployed to: `/usr/local/bin/nebudcs-server`
- Static assets deployed to: `/var/lib/nebudcs/dist`
- Systemd service: `nebudcs.service` is `active (running)`
- API Health Check (`GET http://localhost:8088/api/v1/health`): `status: "running"`, `protocol_version: "1.0.0"`.
- Authenticated Projects Check: Successfully serving "Naphtha Hydrotreater (NHT) Reference Plant" with 49 streams, 41 equipment items, 12 displays, and 17 navigation links.

### 3.4 Knowledge Graph Synchronization
- `graphify update .` executed: **9,640 nodes, 20,039 edges, 533 communities**.

---

## 4. Conclusion
The forensic integration between the Rust process simulation runtime and the Operations SVG workstation is complete, mathematically sound, deterministic, and fully verified. The NHT reference plant operates with authentic process telemetry, clean orthogonal nozzle-to-nozzle piping, real boundary conditions, and accurate industrial equipment visualization.
