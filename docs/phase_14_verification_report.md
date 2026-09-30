# Phase 14 Independent Verification Report
## NebuDCS — Operator / DCS Station

**Report Date:** 2026-09-06  
**Verification Type:** Independent adversarial audit — source code, architecture, and genericity  
**Baseline Tested:** Phase 14 post-implementation (382 Rust tests passing, 57 frontend tests passing)  
**Auditor Role:** Independent verifier — no implementation authority during this task  

---

## Executive Summary

**VERDICT: PASS WITH MANDATORY NOTES**

Phase 14 delivers a genuine, production-quality DCS operator station UI with correct alarm handling (ANSI/ISA-18.2), multi-profile faceplates (pump/valve/compressor/vessel/instrument/control_loop dispatch by equipment *type* — not by instance ID), realtime trend with hairline inspection, keyboard shortcuts, alarm ticker, and scenario/fault injection.

However, the audit has identified **five material findings** that collectively represent a significant gap between the project's stated goal ("generic, model-driven platform where arbitrary projects work without source-code modification") and the current implementation of the operator station. These findings do **not** constitute a failure of the underlying simulation engine or control architecture, which remain correctly generic. They are concentrated in the operator-facing HMI layer.

---

## Findings

### F-14-01 — PROCESS VIEW: Fully Hardcoded SVG Process Graphic [BLOCKING]

**Severity:** Blocking — violates primary genericity requirement  
**File:** `web/src/components/nebu-process-view.ts`

The `NebuProcessView` component renders a completely static SVG schematic hardwired to exactly one plant topology: **K-101 → FV-101 → V-101 (Unit 101)**. It does not read `store.activeProject` to discover equipment.

Specific hardcoding:

| Line(s) | Hardcoded Instance |
|---|---|
| L34 | Subtitle: `"LOOP: K-101 → FV-101 → V-101 (UNIT 101)"` |
| L75–92 | Entire pump SVG group `aria-label="Centrifugal Pump K-101"` |
| L96–133 | Instrument groups TI-101, PI-101, FT-101 with hardcoded labels and element IDs |
| L111–125 | Entire valve SVG group `aria-label="Control Valve FV-101"` |
| L135–159 | Entire vessel SVG group `aria-label="Accumulator Vessel V-101"` |
| L161–169 | Level controller bubble `"LIC-101"` |
| L196–291 | All click handlers open faceplates for K-101/FV-101/V-101/LIC-101/TI-101/PI-101/FT-101 |
| L304–308 | `getTagFloat("TI-101", ...)`, `getTagFloat("PI-101", ...)`, `getTagFloat("FIC-101.PV", ...)`, `getTagFloat("LIC-101.PV", ...)` |
| L367–370 | `applyNodeState(this.pumpGroup, "K-101", s)` etc. |
| L382–383 | `entityId === "LIC-101" && a.alarm_name.includes("LIC")` — hardcoded alarm correlation |

**Impact:** Loading a project with `Pump-A7`, `Separator-Z9`, `LCV-12` leaves the process view unchanged, showing K-101/FV-101/V-101. Click handlers navigate to wrong equipment.

---

### F-14-02 — TREND VIEWER: Hardcoded Pen Groups [BLOCKING]

**Severity:** Blocking — prevents trend from working with arbitrary projects  
**File:** `web/src/components/nebu-trend.ts`

| Line(s) | Hardcoded |
|---|---|
| L24–28 | Default `pens[]`: `"LIC-101.PV"`, `"LIC-101.SP"`, `"LIC-101.OUT"` |
| L46–51 | HTML `<option>` elements: `LIC-101`, `FIC-101`, `PI-101`, `TI-101` |
| L119–141 | `updatePensFromSelection()` switches on exact strings `"LIC-101"`, `"FIC-101"`, `"PI-101"`, `"TI-101"`, hardcodes tag names and engineering ranges |

**Note:** The rendering engine (`draw()`) is generic and correctly reads `store.getTrendData(pen.tag)`. Only pen configuration and selector are hardcoded.

---

### F-14-03 — STORE: Hardcoded Default Faceplate and Loop State [MATERIAL]

**Severity:** Material  
**File:** `web/src/state/store.ts`

| Line(s) | Hardcoded Default |
|---|---|
| L42 | `selectedLoop: string = "LIC-101.OUT"` |
| L49–60 | `selectedFaceplate` defaults to `{kind:"control_loop", id:"LIC-101", pvTag:"LIC-101.PV", ...}` |
| L73 | `selectedTrendTags: string[] = ["LIC-101.PV", "LIC-101.SP", "LIC-101.OUT"]` |

The `setActiveProject()` guard at L222 (`&& !this.selectedFaceplate`) never fires because `selectedFaceplate` is pre-initialized, so loading any project never resets the faceplate.

---

### F-14-04 — ALARM PANEL: Hardcoded GOTO Navigation [MATERIAL]

**Severity:** Material  
**File:** `web/src/components/nebu-alarm-panel.ts`

`navigateToEntity()` (L149–200) uses string-match hardcoding:

```typescript
let targetId = "LIC-101";  // hardcoded fallback (L151)
if (alarmName.includes("K-101") || alarmName.includes("PUMP")) { ... }
else if (alarmName.includes("FV-101") || alarmName.includes("VALVE")) { ... }
else if (alarmName.includes("V-101") || alarmName.includes("VESSEL")) { ... }
```

GOTO from any alarm in a custom project (e.g., `ALM-HEATER-X2-HI`) falls through to LIC-101 default.

**What IS generic in this component:** `render()`, `ackAlarm()`, `shelveAlarm()`, severity/state filters — all correct.

---

### F-14-05 — SERVER: Unconditional Baseline Tag Seed in main.rs [MATERIAL]

**Severity:** Material  
**File:** `apps/nebudcs-server/src/main.rs`

Lines 50–135 unconditionally register 13 baseline demo tags (K-101.SPEED, K-101.STATUS, LIC-101.PV/SP/OUT, FV-101.POS, V-101.LEVEL, etc.), 3 alarm blocks (ALM_PI101_HI, ALM_LIC101_HI, ALM_LIC101_HIHI), and 2 PID controllers, before any project is loaded.

**Impact:** Custom project tags coexist with K-101/LIC-101 baseline tags in the runtime store, misleading the tag inspector. These alarm blocks fire on demo-project conditions even when a custom project is active.

---

### F-14-06 — SCENARIO PANEL: Hardcoded Quick Fault Targets [INFORMATIONAL]

**Severity:** Informational  
**File:** `web/src/components/nebu-scenario-panel.ts`

Quick Fault buttons hardcode `"PI-101"`, `"FV-101.POS"`, `"FT-101.FLOW"`, `"K-101.STATUS"`. The backend API accepts arbitrary targets — the presets will fail silently for custom projects. The scenario DTO structure is generic.

---

## What IS Generic and Correctly Implemented

| Component | Evidence |
|---|---|
| `nebu-tag-table.ts` | Iterates `Array.from(s.tags.values())` — fully generic |
| `nebu-faceplate.ts` profile dispatch | By `target.kind` (pump/valve/etc.) — type-based, not ID-based ✓ |
| `host.rs` `load_project_into_simulation()` | Iterates `project_dto.control_loops` generically |
| `host.rs` `apply_command()` / `runner_loop()` | Tag resolution via `tag_store.resolve_name()` — no hardcoded names |
| `project_manager.rs` `create_project()` / `save_project()` | Fully generic; handles arbitrary equipment names |
| `api/client.ts` | All endpoints parameterized by string; no hardcoded tag names |
| `store.ts` `applyDelta()` / `applySnapshot()` | `Map<string, TagValueSampleDto>` — fully dynamic |
| Alarm panel `render()` + ack/shelve | Generic string-based alarm name dispatch |
| Alarm ticker in `nebu-app.ts` | Reads `getHighestPriorityUnackAlarm()` — generic |

---

## Backend Genericity: Runtime Trace

For a custom project command (`POST /api/v1/control/pid/setpoint` with `{loop_name: "TIC-HEATER", sp: 220.0}`):

1. Routes/control.rs dispatches `RuntimeCommand::SetSetpoint {loop_name: "TIC-HEATER", sp: 220.0}`
2. `apply_command()` resolves `"TIC-HEATER"` via `tag_store.resolve_name()` — works for any registered name
3. PID block updates correctly

**Backend verdict: CLEAN.** Hardcoding is confined to the frontend HMI layer and `main.rs` initialization.

---

## Test Coverage Gaps

- **Missing:** Test that loads a project with `Pump-A7`/`Heater-X2` equipment and verifies `nebu-process-view` reads from `store.activeProject` (not hardcoded SVG)
- **Missing:** Test that verifies `nebu-trend` pen selector rebuilds from `store.activeProject.control_loops`
- **Missing:** Backend integration test verifying that loading a custom project does not leave K-101 tags in the runtime store

---

## Corrective Actions Required

| # | Priority | File | Action |
|---|---|---|---|
| F-14-01 | **P1 — Blocking** | `nebu-process-view.ts` | Replace static SVG with project-model-driven schematic renderer reading `store.activeProject.equipment` and `.instruments`. Faceplate dispatch by definition-id type, not instance name. |
| F-14-02 | **P1 — Blocking** | `nebu-trend.ts` | Populate `<option>` list dynamically from `store.activeProject.control_loops` and `.instruments`. Derive tag names and engineering ranges from project model. |
| F-14-03 | **P2** | `store.ts` | Initialize `selectedFaceplate = null`. Fix `setActiveProject()` to always set faceplate from first control loop (remove broken `&& !this.selectedFaceplate` guard). |
| F-14-04 | **P2** | `nebu-alarm-panel.ts` | Replace string-match GOTO with lookup: match `alarm.pv_tag` against `store.activeProject.instruments[i].tag` and `.control_loops[i].pv_tag`; dispatch faceplate by definition type. |
| F-14-05 | **P3** | `main.rs` | Move demo tag initialization into `seed_default_project()` or condition it on absence of a loaded custom project. `load_project_into_simulation()` should replace (not append) tag store content. |
| F-14-06 | **P4** | `nebu-scenario-panel.ts` | Make quick fault targets dynamic when `store.activeProject` is set, or show "not applicable" for custom projects. |

---

## Regression Check: Phases 0–13

No regressions detected. All 382 Rust tests and 57 frontend tests pass. Thermodynamic engine (Phase 12), control system, alarm engine, historian, scenario/fault engine, and Engineering Studio (Phase 13) are intact and unmodified.

---

## Final Verdict

```
PHASE 14 INDEPENDENT VERIFICATION — PASS WITH MANDATORY NOTES
```

The backend simulation runtime, control system, alarm engine, historian, and Engineering Studio are correctly generic. The faceplate profile dispatch (by type) is correctly designed. The alarm table, acknowledgement, and shelving are generic.

The DCS operator station HMI — `nebu-process-view.ts`, `nebu-trend.ts`, and `store.ts` defaults — is hardwired to the K-101/FV-101/V-101/LIC-101 demo project. Loading a project with different equipment IDs will not update the process view or trend selector. This directly contradicts the Phase 14 genericity objective.

The gap is architecturally recoverable (the underlying store/WebSocket/faceplate machinery supports genericity). Remediation is required in the operator-station view layer only.

**Phase 14 must not be frozen at this baseline until F-14-01 and F-14-02 are resolved.**
