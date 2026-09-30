# NebuDCS NHT Advanced Geometry Collision, Display Isolation & Process Navigation Hardening Report

**Document ID**: `REPORT-NHT-GEOM-ISO-2026-09-09`  
**Classification**: Technical & Verification Report  
**Author**: NebuDCS Core & UI Architecture Team  
**Date**: September 9, 2026  
**Status**: ACCEPTED & INTERNALLY VERIFIED  
**Engineering Provenance**: `[IMPLEMENTED]` `[INTERNALLY VERIFIED]` `[ENGINEERING REFERENCE MODEL]` `[NOT FORMALLY CERTIFIED]`

---

## 1. Executive Summary

During the deployment of the Naphtha Hydrotreater (NHT) Reference Plant within NebuDCS, two visual and state isolation defects were identified in the Operations (DCS HMI) and Engineering Studio environments:
1. **Equipment & Port Collision Inadequacy**: Equipment instances (notably reactor shells, columns, and heat exchanger trains) visually overlapped because simple bounding-box checks ignored component rotation, protruding port connection pins, upright counter-rotated labels, and interaction envelopes.
2. **Display State & Selection Leakage**: When operators navigated between process displays (e.g., from *Feed Preparation* to *Reactor & CFE Train* or *Plant Overview*), residual selection states (`selectedEquipmentId`), focused entities, and unplaced equipment inadvertently bled across display boundaries, causing orphaned focus rings, off-canvas viewport panning, and visual pollution.

### Architectural Solution
To resolve these issues cleanly without resorting to brittle, hardcoded NHT-specific offsets, we engineered generic, platform-wide capabilities:
- **Comprehensive Multi-Tier Geometry & Collision Engine** ([`web/src/domain/geometry-collision.ts`](file:///root/NebuDCS/web/src/domain/geometry-collision.ts)): Transformed body bounds, upright label envelopes, port connection pin footprints, stream penetration testing, and transactional group drag-and-drop validation.
- **Strict Display Scoping via `DisplayViewModel`** ([`web/src/state/store.ts`](file:///root/NebuDCS/web/src/state/store.ts)): Operations views render strictly display-scoped entities derived from authoritative project definitions. Navigation resets out-of-scope selection and focus, persisting viewports independently.
- **Authoritative Geometry in Seed Data** ([`apps/nebudcs-server/src/nht_seed.rs`](file:///root/NebuDCS/apps/nebudcs-server/src/nht_seed.rs)): Re-spaced default plant layouts to provide ≥40–60px clearance across all displays.
- **Zero Physics Regressions**: Full verification across dynamic physics scenarios (Scenarios A–F in Rust), IEC 61131-3 logic, and unit test suites.

---

## 2. Technical Architecture Remediation

### 2.1 Transformed Geometry & Multi-Tier Bounding Boxes

The legacy collision detector relied on scalar widths and heights centered at `(x, y)`. This failed when equipment was rotated (e.g., vertical vessels vs. horizontal drums), when tags were placed outside the body, or when ports extended beyond the shell.

The hardened geometry engine implements:
```typescript
export interface EquipmentInstanceGeometry {
  equipmentId: string;
  definitionId: string;
  position: { x: number; y: number };
  rotation: number;
  scale: number;
  bodyBounds: BoundingBox;        // Transformed physical shell
  labelBounds: BoundingBox;       // Upright, counter-rotated tag plaque
  portBounds: BoundingBox[];      // Protruding nozzle connection pins
  interactionBounds: BoundingBox; // Selection & click target area
  collisionBounds: BoundingBox;   // Inflated envelope (body + labels + ports + clearance)
}
```

#### Key Algorithms Implemented:
1. **Orientation & Scale Transformation (`computeTransformedAABB`)**:
   - Explicitly accounts for 0°, 90°, 180°, and 270° rotations. For orthogonal orientations (90°/270°), width and height dimensions are swapped accurately.
   - Uses `Math.round(val * 1000) / 1000` to eliminate IEEE-754 floating-point drift.
2. **Label Envelope Protection (`testLabelCollision`)**:
   - Equipment labels are rendered upright regardless of equipment rotation. The label envelope is computed at the base of the component (`yOffset = height / 2 + 10px`) with an engineering margin.
   - Detects label-on-label and label-on-body collisions even if equipment shells maintain physical distance.
3. **Port Footprint Integration**:
   - Nozzle ports extending outward (such as quench gas inlets or side reboiler returns) generate localized port bounding boxes, preventing adjacent equipment from clipping into piping connections.
4. **Stream Body Intersection Detection (`testStreamBodyIntersections`)**:
   - Detects when process streams cut through equipment bodies while exempting valid connections that terminate cleanly at port pins.
5. **Transactional Group Collision Testing (`testGroupCollision`)**:
   - When moving multiple items in Engineering Studio, relative offsets are preserved. If any single item in the group encounters a collision, the entire transaction is flagged, rendering live feedback and snapping back to the original valid state on drop.
6. **Deterministic Concentric Spiral Placement (`findCollisionFreePosition`)**:
   - When adding new equipment from the palette, a deterministic multi-ring concentric search algorithm seeks the nearest free coordinate that respects canvas boundaries, existing equipment, and active stream routes.

---

### 2.2 Strict Display Scoping & Isolation (`DisplayViewModel`)

In NebuDCS, Engineering Studio defines the canonical plant model, while Operations renders display-scoped views. To eliminate display leakage:

```typescript
export interface DisplayViewModel {
  displayId: string;
  name: string;
  equipment: EquipmentInstanceDto[];
  placements: Map<string, { x: number; y: number; scale?: number; rotation?: number }>;
  streams: StreamDto[];
  instruments: InstrumentDto[];
  controlLoops: ControlLoopDto[];
  navigationLinks: ProcessNavigationLinkDto[];
  validationErrors: GeometryValidationError[];
}
```

#### Isolation Guarantees:
1. **No Synthetic Fallback Coordinates**:
   If an explicit placement list is defined for a display and an assigned equipment item lacks placement, the engine generates an explicit `MISSING_PLACEMENT` validation error rather than inventing synthetic coordinates like `(100, 200)`. Fallback to `eq.position` is strictly restricted to unconfigured/ad-hoc projects.
2. **Display Navigation Boundary Enforcement**:
   Upon invoking `navigateDisplay(targetDisplayId)`:
   - `selectedEquipmentId` is cleared if the entity does not exist in the destination display.
   - `focusedEntityId` is cleared if the entity does not exist in the destination display.
   - Viewport pan/zoom (`x`, `y`, `scale`) is persisted per display in `displayViewports: Map<string, ViewportConfigDto>`, preventing sudden zoom jumps.
3. **Operations Process View Binding**:
   [`NebuProcessView`](file:///root/NebuDCS/web/src/components/nebu-process-view.ts) queries strictly `s.getDisplayViewModel()`. Equipment, streams, instruments, and the minimap only render entities belonging to the active display view model. If validation errors occur, a prominent validation warning banner alerts the operator without breaking the rendering pipeline.

---

### 2.3 Plant Overview & Reference Layout Optimization

Default seed placements in [`apps/nebudcs-server/src/nht_seed.rs`](file:///root/NebuDCS/apps/nebudcs-server/src/nht_seed.rs) were re-spaced to eliminate physical and visual overlap:

- **Plant Overview (`disp-nht-overview`)**:
  - `TK-101` (x=100) $\to$ `P-101A` (x=240) $\to$ `FV-101` (x=360) $\to$ `E-101A` (x=500) $\to$ `F-101` (x=680) $\to$ `R-101` (x=860) $\to$ `A-101` (x=1040) $\to$ `V-102` (x=1220) $\to$ `C-101` (x=1420) $\to$ `C-102` (x=1620).
  - Offsite/Blowdown `V-104` located at (240, 540) ensuring vertical and horizontal clearance $\ge 50\text{px}$.
- **Reactor & CFE Train (`disp-nht-rx`)**:
  - Exchangers `E-101A` through `E-101G` placed at a 170px pitch (x: 260, 430, 600, 770, 940, 1110, 1280), providing 50px clearance between 120px exchanger shells.
  - Fired Heater `F-101` placed at x=1480; Hydrotreating Reactor `R-101` placed at x=1680, guaranteeing ample room for quench piping, thermowells, and high-temperature alarms.
- **Stripper & Splitter Trains**:
  - Columns `C-101` and `C-102` placed with dedicated reboiler and overhead condenser clearances.

---

## 3. Visual Verification Proof (13 Scenarios)

All 13 visual verification scenarios were captured using headless Chromium automation ([`scripts/capture_nht_geometry_hardening.py`](file:///root/NebuDCS/scripts/capture_nht_geometry_hardening.py)) against the running `nebudcs.service` on port 8088. Screenshots are stored in the repository at `docs/screenshots/nht_hardening/` and mirrored in the active artifact directory.

| # | Scenario ID | Description | Verified Visual Criteria | Screenshot Artifact |
|---|---|---|---|---|
| 1 | `geom_01` | Reactor & Fired Heater Clearance | `R-101` and `F-101` side-by-side with clear piping corridor and no shell overlap | [`geom_01_reactor_adjacent_no_overlap.png`](file:///root/NebuDCS/docs/screenshots/nht_hardening/geom_01_reactor_adjacent_no_overlap.png) |
| 2 | `geom_02` | Column Labels & Stream Separation | `C-101` stripper column with condenser, reboiler, and overhead stream separation | [`geom_02_column_labels_streams_no_collision.png`](file:///root/NebuDCS/docs/screenshots/nht_hardening/geom_02_column_labels_streams_no_collision.png) |
| 3 | `geom_03` | CFE Train Spacing | 7 feed-effluent heat exchangers (`E-101A`–`G`) in series with $\ge 50\text{px}$ clearance | [`geom_03_cfe_train_seven_exchangers_spaced.png`](file:///root/NebuDCS/docs/screenshots/nht_hardening/geom_03_cfe_train_seven_exchangers_spaced.png) |
| 4 | `geom_04` | Engineering Studio Equipment Placement | Palette drag-and-drop auto-snapping to collision-free coordinates | [`geom_04_studio_large_equipment_placement.png`](file:///root/NebuDCS/docs/screenshots/nht_hardening/geom_04_studio_large_equipment_placement.png) |
| 5 | `geom_05` | Drag Collision Visual Feedback | Visual warning outline rendering during intentional equipment collision drag | [`geom_05_collision_feedback_dragging_equipment.png`](file:///root/NebuDCS/docs/screenshots/nht_hardening/geom_05_collision_feedback_dragging_equipment.png) |
| 6 | `geom_06` | Plant Overview Master Display | Complete 11-equipment overview display with process flow from feed to splitter | [`geom_06_plant_overview.png`](file:///root/NebuDCS/docs/screenshots/nht_hardening/geom_06_plant_overview.png) |
| 7 | `geom_07` | Feed Preparation Display | `TK-101`, `P-101A/B`, `FV-101` isolated strictly without reactor/stripper leakage | [`geom_07_feed_display.png`](file:///root/NebuDCS/docs/screenshots/nht_hardening/geom_07_feed_display.png) |
| 8 | `geom_08` | Reactor Display Isolation | High-fidelity reactor loop isolated without feed storage or offsite equipment | [`geom_08_reactor_display.png`](file:///root/NebuDCS/docs/screenshots/nht_hardening/geom_08_reactor_display.png) |
| 9 | `geom_09` | Navigation: Feed to Reactor | Smooth transition via navigation button, view model re-scoped, no residual Feed graphics | [`geom_09_navigation_feed_to_reactor.png`](file:///root/NebuDCS/docs/screenshots/nht_hardening/geom_09_navigation_feed_to_reactor.png) |
| 10 | `geom_10` | Navigation: Reactor to Overview | Smooth transition back to overview, reactor-specific details removed | [`geom_10_navigation_reactor_to_overview.png`](file:///root/NebuDCS/docs/screenshots/nht_hardening/geom_10_navigation_reactor_to_overview.png) |
| 11 | `geom_11` | Leakage Guard Verification | Verified zero orphan nodes or unassigned equipment items rendered on Overview | [`geom_11_overview_no_feed_equipment_leakage.png`](file:///root/NebuDCS/docs/screenshots/nht_hardening/geom_11_overview_no_feed_equipment_leakage.png) |
| 12 | `geom_12` | Selection Boundary Isolation | Selection ring on Feed display does not bleed into Reactor display upon navigation | [`geom_12_selection_isolation_no_leak.png`](file:///root/NebuDCS/docs/screenshots/nht_hardening/geom_12_selection_isolation_no_leak.png) |
| 13 | `geom_13` | Faceplate Navigation Handling | Open faceplate (`FIC-101`) safely closed or re-scoped during cross-display navigation | [`geom_13_faceplate_open_cross_display_navigation.png`](file:///root/NebuDCS/docs/screenshots/nht_hardening/geom_13_faceplate_open_cross_display_navigation.png) |

---

## 4. Verification & Testing Evidence

### 4.1 Frontend Unit & Hardening Tests (Vitest)
All 23 test suites and 214 unit tests pass without failure:
```
Test Files  23 passed (23)
     Tests  214 passed (214)
  Start at  21:20:15
  Duration  1.68s
```
Specific coverage highlights:
- [`web/tests/geometry-collision.test.ts`](file:///root/NebuDCS/web/tests/geometry-collision.test.ts) (6/6 passed): Standard bounding boxes, distance thresholds, clearance padding.
- [`web/tests/geometry-collision-hardening.test.ts`](file:///root/NebuDCS/web/tests/geometry-collision-hardening.test.ts) (11/11 passed): Rotated AABB calculation, upright label envelope detection, port footprint collisions, transactional group drag testing, spiral placement search, stream body intersection rejection, DisplayViewModel isolation, cross-display selection reset, and missing placement error reporting.

### 4.2 Dynamic Physics & Reference Plant Tests (Rust)
The authoritative dynamic physics simulation core continues to pass all dynamic and reference validation gates:
- `cargo test --test nht_dynamic_physics_tests`: **1 passed, 0 failed**
  - Scenario A: Baseline steady state (Feed $120.0\,\text{m}^3/\text{h}$, H2/HC $350.0\,\text{Nm}^3/\text{m}^3$, WABT $345.0^\circ\text{C}$).
  - Scenario B: Feed ramp transient ($120 \to 140\,\text{m}^3/\text{h}$).
  - Scenario C: Reactor thermal runaway & Bed-1 quench activation.
  - Scenario D: Feed pump trip & auto-start of standby `P-101B`.
  - Scenario E: Compressor trip & recycle de-pressuring.
  - Scenario F: Column overhead condenser cooling water degradation.
- `cargo test --test stage_14_nht_reference_plant_tests`: **4 passed, 0 failed**
  - Seed plant construction, IEC 61131-3 logic routines, protection matrix validation, and stream integrity.

### 4.3 Build & Service Status
- Release Build: `cargo build --release -p nebudcs-server` compiled cleanly with zero errors and zero warnings.
- Production UI: `npm run build` compiled cleanly; deployed to `/var/lib/nebudcs/dist/`.
- Systemd Daemon: `systemctl restart nebudcs.service` verified active (`running`, PID 30349) listening on HTTP port 8088.

---

## 5. Compliance & Engineering Governance

```
================================================================================
ENGINEERING REFERENCE MODEL GOVERNANCE & PROVENANCE NOTICE
================================================================================
Status:               IMPLEMENTED
Verification:         INTERNALLY VERIFIED
Model Type:           ENGINEERING REFERENCE MODEL
Certification:        NOT FORMALLY CERTIFIED FOR COMMERCIAL REFINERY SAFETY
Target Architecture:  Generic NebuDCS Process Simulation Platform
Seed Unit:            Naphtha Hydrotreater (NHT) Reference Plant
================================================================================
```

The implemented collision, placement, and display isolation systems are generic capabilities of the NebuDCS platform. No unit-specific hardcoding or vendor-specific HMI assumptions were introduced into the renderer. All process-specific layouts remain completely defined by authoritative project seed data.
