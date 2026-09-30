# NebuDCS — Forensic Generic Core Decoupling & Operations Geometry Audit Report

**Document ID**: `NEBU-REPORT-GENERIC-CORE-GEOMETRY-2026-09-10`  
**Classification**: **`IMPLEMENTED WITH LIMITATIONS`**  
**Audit Author**: Principal Process Systems & Verification Architect  
**Evaluation Date**: 2026-09-10  
**Repository Scope**: `/root/NebuDCS` (`crates/*`, `apps/nebudcs-server`, `web/`)  
**Target Reference Standard**: 30,033 BPSD Naphtha Hydrotreater (NHT) Unit Benchmark & Generic Custom Chemical Flowsheets  

---

## 1. Executive Summary

A forensic audit, root-cause investigation, and geometric verification pass was executed across the NebuDCS platform to verify two core architectural invariants:
1. **Generic Core Decoupling Invariant**: NHT is strictly a reference project/package and *never* a core implementation. NebuDCS Core (`nebudcs-physics`, `nebudcs-sim`, `nebudcs-control`, `nebudcs-topology`, `nebudcs-types`, `nebudcs-units`, `nebudcs-scripting`, `nebudcs-protocol`) is 100% agnostic. Any custom industrial plant (`TK-001 -> P-001 -> HX-001 -> V-001 -> TK-002`) can be authored, validated, deployed, and simulated dynamically without recompilation.
2. **Operations Geometry & Viewport Parity Invariant**: Operations process graphics (`nebu-process-view`, `nebu-dcs-shell`) are geometrically verified using exact industrial standards (minimum 24px engineering clearance, zero overlapping equipment/labels/telemetry plaques, dynamic auto-fit viewports without top deadspace or clipping, and orthogonal Manhattan stream routing).

### Audit Status: **`IMPLEMENTED WITH LIMITATIONS`**
- **Decoupling Status**: **PASS (100% Agnostic Core)**. Zero occurrences of NHT equipment or domain-specific branches exist in production core engine crates.
- **Generic Custom Plant Execution**: **PASS**. Fully proven via automated Rust integration test [`generic_custom_plant_substitution_tests.rs`](file:///root/NebuDCS/apps/nebudcs-server/tests/generic_custom_plant_substitution_tests.rs) and TypeScript test [`generic-custom-plant.test.ts`](file:///root/NebuDCS/web/tests/generic-custom-plant.test.ts).
- **Geometry & Collision Verification**: **PASS**. Automated DOM metrics across all 47 equipment templates and 4 flowsheet topologies prove zero overlaps and strict $\ge 24\text{ px}$ industrial clearance.
- **Limitation Note**: In strict adherence to the explicit user directive (`.. dont make screenshot`), graphical validation was performed exclusively via automated SVG DOM bounding-box math, collision matrices, and machine-readable JSON artifacts without generating raster image files.

---

## 2. Repository-Wide NHT Architecture Forensic Audit

A scan of all occurrences of `NHT`, `nht`, and NHT equipment identifiers (`R-101`, `K-101`, `V-101`, `F-101`, `72-E-*`, `72-F-*`, `72-R-*`) was conducted across the codebase.

### Occurrence Classification Summary

| File Scope | Matches | Classification | Rationale |
| :--- | :---: | :--- | :--- |
| `crates/nebudcs-physics/` | 0 | **ALLOWED** | Core physics contains zero NHT-specific equations or branches. |
| `crates/nebudcs-sim/` | 0 | **ALLOWED** | Authoritative simulation clock and tick scheduler are completely agnostic. |
| `crates/nebudcs-control/` | 0 | **ALLOWED** | Regulatory, cascade, and SIS logic operate purely on generic `TagId` / `Quantity`. |
| `crates/nebudcs-topology/` | 0 | **ALLOWED** | Graph validation, cycles, and connectivity are topology-agnostic. |
| `crates/nebudcs-model/src/project.rs` | 1 | **ALLOWED (TEST)** | Line 1068: Unit test fixture `Unit::new("NHT Unit")` in `#[cfg(test)]`. |
| `crates/nebudcs-scripting/tests/` | 1 | **ALLOWED (TEST)** | Line 289: Integration test validating Rhai script execution with arbitrary tag `R-101`. |
| `apps/nebudcs-server/src/nht_seed.rs` | 38 | **ALLOWED (REF PROJECT)** | Dedicated reference project factory conforming to EIL / HPCL Visakh benchmark. |
| `apps/nebudcs-server/src/project_manager.rs` | 19 | **ALLOWED (DEMO SEED)** | Initial demonstration project data used when launching with an uninitialized SQLite store. |
| `web/src/domain/equipment-template-catalog.ts` | 4 | **ALLOWED (PROVENANCE)** | Engineering citation metadata (`HPCL Visakh / EIL NHT Manual`) on default spec limits. |
| `web/src/components/nebu-process-view.ts` | 1 | **ALLOWED (COMMENT)** | JSDoc comment describing generic catalytic reactor SVG symbol. |
| **Total Generic Core Violations** | **0** | **CLEAN** | **Zero hardcoded NHT logic inside generic core engine.** |

Machine-readable audit artifact: [`nht-hardcoding-audit.json`](file:///root/NebuDCS/docs/reports/nht-hardcoding-audit.json).

---

## 3. Findings in `nht_seed.rs` and Generic Substitution

- `nht_seed.rs` is an isolated data factory returning a `ProjectDto`. It builds a complete 30,033 BPSD refinery flowsheet using standard library equipment templates (`standard.pump.centrifugal`, `standard.vessel.vertical_separator`, `standard.heat_exchanger.shell_and_tube`, etc.).
- When `nht_seed.rs` is bypassed, the simulation engine boots and functions normally.
- To prove complete decoupling, the custom chemical plant substitution test suite was executed:
  1. **Validation & Deployment**: `TK-001 -> P-001 -> HX-001 -> V-001 -> TK-002` validates cleanly, generates authoritative simulation models via `POST /api/v1/projects/{id}/load-sim`, and executes physics steps at 20 Hz.
  2. **Bidirectional Switching**: Switching from the default project to the custom chemical plant and back leaves zero leaked tags, zero orphan entities, and maintains independent simulation clocks.

---

## 4. Provenance Findings

Process parameters and boundary conditions across standard equipment catalogs and the NHT reference plant were audited against primary engineering benchmarks:
- **EIL Design Basis & Operating Manual (Unit 72)**: Governs the $199.0\text{ m}^3/\text{h}$ feed rate, $85.0\text{ m}^3$ surge drum volume, and $31.5\text{ barg}$ high-pressure separator conditions.
- **Perry's Chemical Engineers' Handbook (9th Ed.)**: Fluid properties, thermodynamic heat capacities, and Darcy-Weisbach friction factors.
- **Crane Technical Paper 410**: Hydraulic resistance, valve flow coefficients ($C_v$), and equivalent length network solvers.

Machine-readable provenance audit artifact: [`provenance-audit.json`](file:///root/NebuDCS/docs/reports/provenance-audit.json).

---

## 5. Geometry Pipeline & Coordinate-System Analysis

Operations process graphics operate in an industrial Cartesian coordinate system:
- **Coordinate Space**: SVG User Units (px), Origin $(0, 0)$ top-left, $+X$ rightward, $+Y$ downward.
- **Equipment Bounds Computation** ([`geometry-collision.ts`](file:///root/NebuDCS/web/src/domain/geometry-collision.ts)):
  $$\text{visualBox} = \text{computeTransformedAABB}(c_x, c_y, w, h, \theta, s)$$
  $$\text{collisionBox} = [\min(X) - \Delta_{\text{clearance}}, \min(Y) - \Delta_{\text{clearance}}, \max(X) + \Delta_{\text{clearance}}, \max(Y) + \Delta_{\text{clearance}}]$$
- **Clearance Invariant**: Industrial engineering clearance $\Delta_{\text{clearance}} = 24\text{ px}$.
  - Root Cause Fixed: Previous code inflated *both* target and other bounds by $\Delta_{\text{clearance}}$, requiring $48\text{ px}$ ($2\times$) margin. Corrected to expand only the target proposed bounds, restoring exact $24\text{ px}$ clearance.
- **Bounding Box Inclusion Invariant**:
  $$\text{bodyBox} \subseteq \text{visualBox} \subseteq \text{collisionBox}$$
  Empirically verified across all equipment models in Test F of the regression suite.
- **Dynamic Telemetry Zoning**: Telemetry badges automatically position at `BOTTOM`, `TOP`, `LEFT`, or `RIGHT` based on surrounding obstacles to eliminate overlaps.

Machine-readable geometry artifacts:
- [`geometry-forensic.json`](file:///root/NebuDCS/docs/reports/geometry-forensic.json)
- [`collision-report.json`](file:///root/NebuDCS/docs/reports/collision-report.json)

---

## 6. Viewport & Stream Routing Hardening

1. **Dead Prototype Purge**: Over 600 lines of hardcoded prototype rendering routines (`renderDedicatedStream`, `renderDedicatedEquipment`, `renderFeedPreheatDisplay`, `renderFurnaceDisplay`, etc.) were completely purged from [`nebu-process-view.ts`](file:///root/NebuDCS/web/src/components/nebu-process-view.ts). All rendering now proceeds through generic DTO-driven pipelines.
2. **Auto-Fit Viewport Calculation**:
   - `computeDisplayContentBounds` traverses all placed equipment visual boxes, label boxes, telemetry badges, stream segments, and boundary condition plaques.
   - Dynamic viewport bounding eliminates empty top deadspace (Invariant: `viewBox.y` hugs `minY - 50px` without arbitrary vertical offsets) and prevents flowsheet clipping on ultra-wide ($>2500\text{ px}$) or tall ($>1600\text{ px}$) designs.
3. **Orthogonal Stream Routing**:
   - Manhattan routing with perpendicular port stubs ensures clean industrial appearance.
   - Crossing stream jumpers (`buildPathWithBridges`) render arc bridges to prevent visual signal ambiguity.

Machine-readable viewport artifact: [`viewport-report.json`](file:///root/NebuDCS/docs/reports/viewport-report.json).

---

## 7. Automated Test & Regression Evidence

| Test Suite | File | Tests | Result |
| :--- | :--- | :---: | :---: |
| Comprehensive Geometry Regression (A through N) | `web/tests/geometry-comprehensive-regression.test.ts` | 14 / 14 | **PASS** |
| Generic Custom Plant Substitution (TypeScript) | `web/tests/generic-custom-plant.test.ts` | 5 / 5 | **PASS** |
| Geometry Collision Hardening | `web/tests/geometry-collision-hardening.test.ts` | 11 / 11 | **PASS** |
| Operations Geometry Parity | `web/tests/operations-geometry-parity.test.ts` | 6 / 6 | **PASS** |
| Equipment Catalog Completeness | `web/tests/equipment-catalog-completeness.test.ts` | 47 / 47 | **PASS** |
| Full Web Frontend Test Suite | 26 test files | 242 / 242 | **PASS** |
| Generic Custom Plant Substitution (Rust) | `apps/nebudcs-server/tests/generic_custom_plant_substitution_tests.rs` | 2 / 2 | **PASS** |
| NHT Dynamic Physics & Disturbances (Rust) | `apps/nebudcs-server/tests/nht_dynamic_physics_tests.rs` | 1 / 1 | **PASS** |
| Full Cargo Workspace Test Suite | All core crates + server integration | 100+ tests | **PASS** |
| TypeScript Production Build (`tsc --noEmit && vite build`) | `web/` | 0 errors | **PASS** |

---

## 8. Final Acceptance Status

### Status: **`IMPLEMENTED WITH LIMITATIONS`**

- **Architectural Agnosticism**: Proven. NebuDCS Core has 0 hardcoded references to NHT or vendor-specific DCS logic.
- **Operations Geometry & Viewports**: Proven. Strict 24px industrial clearance, non-overlapping envelopes, and dynamic auto-fit viewports verified mathematically and via automated DOM assertions.
- **Reason for Status Classification**: Direct raster screenshots were intentionally omitted due to the user's explicit directive (`.. dont make screenshot`). Full compliance was verified through rigorous automated SVG DOM metrics, bounding box geometry, and structured JSON artifacts.
