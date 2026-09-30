# NebuDCS Stage 14: Comprehensive Pre-Implementation UI/UX Forensic Audit

- **Date:** 2026-09-09
- **Auditor:** Principal Industrial UI/UX Architect, DCS/HMI Architect & Senior Runtime Lead
- **Status:** COMPLETED — AUDIT FIRST DIRECTIVE ENFORCED (READ-ONLY FORENSIC PHASE)
- **Authoritative Baseline:** Phases 0–16, Controlled Platform Expansion, Stages 1–14 (Frozen Baseline; Strictly NO Phase 17)

---

## 1. Executive Summary & Purpose

In compliance with the **Mandatory UI Audit-First Directive**, this document delivers a complete, forensic, code-level and visual audit of the existing NebuDCS Stage 14 web frontend before implementing any architectural redesign or refactoring.

The NebuDCS frontend is technically mature, featuring real-time WebSocket state streaming, native Web Components, comprehensive ISA-101 process graphics, ISA-18.2 alarm management, multi-pen trends, IEC 61131-3 logic authoring, multi-node HA cluster management, and scientific validation benchmarks. However, the application suffers from information fragmentation, redundant subsystem exposure across multiple navigation tiers, an overwhelming 13-tab horizontal toolbar in Engineering, and an incomplete capability-driven inspector model for process equipment.

This audit establishes the definitive baseline, inventories all 34 custom UI components, identifies 16 categorized UX issues (P0 to P3), conducts visual inspection across 20 browser-captured states, and defines the target information architecture modeled after proven industrial engineering workstations (Rockwell Studio 5000, Siemens TIA Portal) and modern DCS operator consoles (WinCC, FactoryTalk View, DeltaV).

---

## 2. Current UI Architecture

The frontend is implemented in TypeScript using native Web Components (custom elements v1) bundled with Vite. It avoids monolithic frameworks (React/Vue/Angular), maintaining minimal runtime overhead, deterministic DOM lifecycle management, and high-performance SVG/Canvas rendering.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           <nebu-app> Shell                              │
├─────────────────────────────────────────────────────────────────────────┤
│ Top App Header: Brand | Role Selector | Sim Controls | Connection State │
├─────────────────────────────────────────────────────────────────────────┤
│ Workspace Navigation: Engineering | Operations | Training | ...        │
├─────────────────────────────────────────────────────────────────────────┤
│ Global Status Bar: Node | Epoch | Repl Lag | Deploy | SIS | Comm | Clock│
├─────────────────────────────────────────────────────────────────────────┤
│ Dynamic Viewport:                                                       │
│  - <nebu-studio>               (Engineering Workspace)                  │
│  - <nebu-dcs-shell>            (Operations Workspace)                   │
│  - <nebu-training-studio>      (Training OTS Workspace)                 │
│  - <nebu-validation-browser>   (Scientific Validation Workspace)        │
│  - <nebu-system-view>          (System Telemetry & Hardware Workspace)  │
│  - Enterprise Panels           (Cluster, Audit, Backup)                 │
├─────────────────────────────────────────────────────────────────────────┤
│ Persistent Footer / Alarm Ticker Banner                                 │
└─────────────────────────────────────────────────────────────────────────┘
```

### State & Networking Layers
- **Authoritative Simulation Core:** Rust backend (`nebudcs-server`) running deterministically at 50 Hz.
- **WebSocket Protocol Client (`WsClient`):** Full-duplex JSON stream communicating on `/ws`. Handles handshakes, bidirectional commands, state snapshots, and delta frames with heartbeat keep-alive.
- **Central Store (`Store`):** Monolithic observable reactive store (`web/src/state/store.ts`) providing subscription mechanisms for active tags, alarms, simulation clock, connection status, and project details.
- **Specialized Stores:** `studioStore` (P&ID selection and dirty tracking), `trainingStore` (OTS sessions and scenarios), `authStore` (RBAC roles and tokens).

---

## 3. Current Workspace Structure

| Workspace | Component | Primary Intended Purpose | Audit Status |
| :--- | :--- | :--- | :--- |
| **Engineering** | `<nebu-studio>` | Plant creation, P&ID flowsheet design, equipment sizing, I/O mapping, logic authoring, pre-flight check, hot deploy. | **Functional but fragmented**: 13 horizontal tabs overflow; canvas disappears when configuring tags or I/O; inspector lacks capability tabs. |
| **Operations** | `<nebu-dcs-shell>` | DCS operator workstation: ISA-101 flowsheet, loop faceplates, multi-pen trend, ISA-18.2 alarm station, process tag table. | **High maturity**: Cleaned of engineering hardware internals; breadcrumb navigation is static; bottom alarm banner is ephemeral. |
| **Training** | `<nebu-training-studio>` | Operator Training Simulator: Instructor orchestration, scenario authoring, fault injection, trainee station, evaluation, replay. | **High maturity**: All tabs visible to all roles; trainee role needs strict view isolation from instructor fault controls. |
| **Validation** | `<nebu-validation-browser>` | Scientific verification, numerical benchmarks, NIST/Perry reference datasets, error metrics (MAE/RMSE), certification manifests. | **Exemplary**: Strict compliance with Certification Language Policy (`"INTERNALLY VALIDATED"`). |
| **System** | `<nebu-system-view>` | Consolidates 10-subsystem runtime telemetry, virtual controllers and task scheduling, I/O HAL racks, fieldbus comms, and diagnostics. | **Functional**: Consolidated subtabs exist; needs tighter diagnostic correlation to plant equipment. |
| **Enterprise** | Multi-panel container | Multi-node HA cluster topology, failover, split-brain epoch fencing, state replication, DR backup/restore, audit journal. | **Exemplary**: Role-based action gating verified; clean subtab navigation. |

---

## 4. Current UI Component Inventory

| Component Name | Purpose | Workspace | Current Location | Primary User | Data Source | Actions | Dependencies | Problems | Keep / Modify / Move / Replace |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `nebu-app` | Root shell & viewport router | Global | `src/components/nebu-app.ts` | All | Store, AuthStore, WsClient | Mode switch, Role switch, Quick alarm ack | Store, AuthStore, WsClient | Header crowded; status bar not role-contextual; alarm banner single-line | **MODIFY** |
| `nebu-sim-controls` | Simulation clock lifecycle controls | Global / Header | `src/components/nebu-sim-controls.ts` | Operator / Engineer | Store, WsClient, ApiClient | Play, Pause, Step, Reset, Speed multiplier | Store, WsClient, ApiClient | Needs explicit state awareness (STARTING, STOPPING, FAULTED) | **MODIFY** |
| `nebu-connection-status` | WS connection & ping monitor | Global / Header | `src/components/nebu-connection-status.ts` | All | Store, WsClient | Reconnect trigger | Store, WsClient | Purely status indicator | **KEEP** |
| `nebu-studio` | Engineering workbench shell | Engineering | `src/components/nebu-studio.ts` | Engineer | StudioStore, ApiClient | Pan, zoom, save, validate, hot-deploy, tab switch | StudioStore, ApiClient | 13 overflowing tabs; canvas unmounted on sub-tool selection | **MODIFY** |
| `nebu-project-explorer` | Hierarchical plant entity tree | Engineering | `src/components/nebu-project-explorer.ts` | Engineer | StudioStore, ApiClient | Select entity, filter | StudioStore | Tree navigation clean; needs cross-reference badges | **KEEP** |
| `nebu-equipment-palette` | Equipment & instrument catalog | Engineering | `src/components/nebu-equipment-palette.ts` | Engineer | StudioStore | Drag/drop equipment onto canvas | StudioStore | Standard catalog symbols clean | **KEEP** |
| `nebu-pid-canvas` | Interactive P&ID SVG/Canvas | Engineering | `src/components/nebu-pid-canvas.ts` | Engineer | StudioStore | Drag, drop, pan, zoom, connect ports, select | StudioStore | Primary activity; needs minimap & alignment guides | **MODIFY** |
| `nebu-property-inspector` | Equipment & stream property editor | Engineering | `src/components/nebu-property-inspector.ts` | Engineer | StudioStore | Edit parameters, delete entity | StudioStore | Lacks capability-driven sections (Hydraulics, Motor, Control, Alarms, Protection) | **MODIFY** |
| `nebu-validation-panel` | Design-time rule validation summary | Engineering | `src/components/nebu-validation-panel.ts` | Engineer | StudioStore, ApiClient | Run validation, filter errors | StudioStore | Standalone panel; should integrate into Project Readiness checklist | **MODIFY** |
| `nebu-iec-editor` | IEC 61131-3 logic editor | Engineering | `src/components/nebu-iec-editor.ts` | Engineer | ApiClient | Switch LD/ST/FBD, edit, compile | ApiClient | Full-page takeover; should be contextual routine drawer | **MODIFY** |
| `nebu-io-panel` | I/O chassis, racks, and modules | System / Engineering | `src/components/nebu-io-panel.ts` | Engineer / Tech | ApiClient | Configure rack, inject channel faults | ApiClient | Dual placement creates confusion | **MOVE** $\to$ System (Monitor) & Drawer (Eng) |
| `nebu-io-mapping-table` | Channel-to-tag signal wiring table | Engineering | `src/components/nebu-io-mapping-table.ts` | Engineer | ApiClient | Map channels, edit EU scaling | ApiClient | Table clean; should be accessible from equipment inspector | **MODIFY** |
| `nebu-tag-database` | Global process tag summary | Engineering | `src/components/nebu-tag-database.ts` | Engineer | ApiClient | Filter tags, edit limits | ApiClient | Standalone view; good for engineering tag oversight | **KEEP** |
| `nebu-cross-reference` | Signal and tag dependency matrix | Engineering | `src/components/nebu-cross-reference.ts` | Engineer | ApiClient | Query tag usage across subsystems | ApiClient | Clicking row does not navigate to owning entity | **MODIFY** |
| `nebu-controller-panel` | Virtual PLC/DCS runtime & tasks | System | `src/components/nebu-controller-panel.ts` | Engineer / Tech | ApiClient | Task scheduling, jitter monitor, switchover | ApiClient | Correctly moved to System; needs faceplate binding link | **KEEP** |
| `nebu-advanced-control` | APC strategies (Cascade, Feed-fwd) | Engineering | `src/components/nebu-advanced-control.ts` | Engineer | ApiClient | Create/edit APC strategies | ApiClient | Clean strategy builder | **KEEP** |
| `nebu-hydraulic-inspector` | Pressure-flow & pump curves | Engineering | `src/components/nebu-hydraulic-inspector.ts` | Engineer | ApiClient | Inspect node pressures, branch flows | ApiClient | Uses independent polling timer; should bind to store | **MODIFY** |
| `nebu-safety-panel` | SIS Cause & Effect matrix & SIFs | Engineering | `src/components/nebu-safety-panel.ts` | Engineer / Safety | ApiClient | SIF bypass, trip reset, proof test | ApiClient | Uses independent timer; should bind to store | **MODIFY** |
| `nebu-domain-browser` | Domain packages & thermo packages | Engineering | `src/components/nebu-domain-browser.ts` | Engineer | ApiClient | Select domain, view thermo packages | ApiClient | Informational browser | **KEEP** |
| `nebu-dcs-shell` | DCS operations workstation | Operations | `src/components/nebu-dcs-shell.ts` | Operator | Store, ApiClient | Tab switch, view unit flowsheet | Store, ApiClient | Polling interval for diagnostics; static breadcrumbs | **MODIFY** |
| `nebu-process-view` | High-performance ISA-101 flowsheet | Operations | `src/components/nebu-process-view.ts` | Operator | Store | Select equipment/loop, view alarms | Store | Core operator screen; needs area zoom & sub-unit panning | **MODIFY** |
| `nebu-faceplate` | Generic DCS faceplate (Loop/Equip) | Operations | `src/components/nebu-faceplate.ts` | Operator | Store, WsClient, ApiClient | Change mode, SP, OUT, Start/Stop pump | Store, WsClient, ApiClient | High fidelity; needs compact vs expanded toggle | **MODIFY** |
| `nebu-trend` | Canvas multi-pen process trend | Operations | `src/components/nebu-trend.ts` | Operator | Store | Select pen group, change time window | Store | Dynamic pen groups clean; needs live vs historical mode | **MODIFY** |
| `nebu-alarm-panel` | ISA-18.2 Alarm station & actions | Operations | `src/components/nebu-alarm-panel.ts` | Operator | Store, WsClient, ApiClient | Ack, Ack All, Reset Interlocks, Reset SIS | Store, WsClient, ApiClient | Clear separation of Ack vs Reset; high maturity | **KEEP** |
| `nebu-tag-table` | Live process tag table | Operations | `src/components/nebu-tag-table.ts` | Operator | Store, WsClient | Search, manual override | Store, WsClient | High-rate DOM re-rendering; needs virtualization | **MODIFY** |
| `nebu-training-studio` | OTS console & scenario manager | Training | `src/components/nebu-training-studio.ts` | Instructor / Trainee | TrainingStore, ApiClient | Start session, inject fault, grade, replay | TrainingStore, ApiClient | Trainee sees instructor tabs; needs strict role view gating | **MODIFY** |
| `nebu-validation-browser`| Scientific benchmark qualification | Validation | `src/components/nebu-validation-browser.ts` | Engineer / Sci | ApiClient | Run benchmarks, inspect error metrics | ApiClient | Exemplary implementation; strict certification wording | **KEEP** |
| `nebu-system-view` | Subsystem telemetry & hardware | System | `src/components/nebu-system-view.ts` | Tech / Admin | ApiClient, WsClient | Subtab switch | ApiClient, WsClient | Consolidates controllers, I/O, comms, diagnostics | **KEEP** |
| `nebu-system-observability`| 10-subsystem runtime telemetry | System / Enterprise | `src/components/nebu-system-observability.ts`| Tech / Admin | ApiClient | Inspect cycle times, dropped frames | ApiClient | Independent polling timer | **MODIFY** |
| `nebu-communications` | Modbus/OPC UA/MQTT fieldbus | System | `src/components/nebu-communications.ts` | Tech / Eng | ApiClient | Inspect communication channels | ApiClient | Independent polling timer | **MODIFY** |
| `nebu-diagnostics-view`| Global cross-subsystem correlation | System / Enterprise | `src/components/nebu-diagnostics-view.ts`| Tech / Eng | ApiClient | Inspect root-cause advisories | ApiClient | Clean advisory cards | **KEEP** |
| `nebu-cluster-panel` | HA cluster topology & failover | Enterprise | `src/components/nebu-cluster-panel.ts` | Admin / Eng | ApiClient | Failover, inject failure, state snapshot | ApiClient | Role gating verified; clean cluster metrics | **KEEP** |
| `nebu-audit-panel` | Tamper-evident audit journal | Enterprise | `src/components/nebu-audit-panel.ts` | Auditor / Admin | ApiClient | Filter events, verify HMAC, export CSV | ApiClient | High maturity compliance ledger | **KEEP** |
| `nebu-backup-panel` | DR backup & dry-run restore | Enterprise | `src/components/nebu-backup-panel.ts` | Admin | ApiClient | Create backup, validate dry-run, restore | ApiClient | Dry-run non-mutating validation verified | **KEEP** |

---

## 5. Current Navigation & Interaction Analysis

### 5.1 Top-Level Routing
The top-level switcher correctly defines 6 workspaces:
`[Engineering] [Operations] [Training (OTS)] [Scientific Validation] [System] [Enterprise & HA]`
- **Strengths:** Clean, predictable switching of the primary main viewport.
- **Defects:**
  - The top bar mixes brand, role switcher, token editing, simulation playback buttons, and connection status in one row.
  - The global status bar below the header contains enterprise-centric items (`NODE`, `EPOCH`, `REPL LAG`) that distract operators, rather than contextualizing status based on active persona.

### 5.2 Engineering Navigation
In `<nebu-studio>`, navigation is fragmented:
1. **Left Sidebar Mode Toggle:** Tabs between `📁 Explorer` and `📦 Catalog`.
2. **Sub-View Switcher Bar:** 13 horizontal tabs scroll off-screen on smaller viewports. Clicking a tab like `Tag Database` completely hides the P&ID canvas and replaces it with a full-screen table.
3. **Property Inspector:** Displays on the right, but collapses whenever unselected, and only displays raw key-value parameter inputs without domain grouping.

### 5.3 Operations Navigation
In `<nebu-dcs-shell>`, navigation is structured around:
- `Unit Flowsheet`: Process Graphic + Faceplate + Trend + Alarm Panel + Tag Table.
- `Plant Overview`: High-level KPI cards and unit summary.
- `Equipment Diagnostics`: Diagnostics table.
- `First-Out Chronology`: Disturbance sequence analysis.
- **Defects:**
  - Breadcrumbs (`ENTERPRISE: Global / SITE: Plant Site 1 / AREA: Area 01 / UNIT: Unit 100`) are purely decorative static text. An operator cannot click "AREA 01" to see all units in Area 1, or click an equipment tag to zoom in on it.
  - The alarm banner at the bottom is hidden when no alarms are active, depriving the operator of standing alarm count awareness.

---

## 6. UX Problem Audit & Issue Categorization

### Priority 0 (P0): Workflow-Blocking Issues
1. **P0-1: Disconnected Equipment Configuration**: Selecting an equipment entity does not provide a complete, capability-driven configuration inspector (Hydraulics, Motor, Control, Alarms, Protection, Historian). Engineers are forced to hop across multiple full-screen tabs.
2. **P0-2: Absence of Project Readiness Matrix**: No pre-flight readiness checklist indicating whether topology, tags, I/O, controllers, and safety functions are verified prior to deployment.
3. **P0-3: Trainee Role Exposure to Instructor Scenarios**: Trainees logging in with `trainee` token can see the Instructor Console and Scenario Authoring tabs.

### Priority 1 (P1): Major Usability & Information Architecture Confusion
1. **P1-1: 13-Tab Horizontal Overload in Studio**: Flowsheet canvas is repeatedly unmounted when inspecting tags, I/O, or logic routines. Secondary engineering tools must be integrated as docked drawers or inspector sub-views.
2. **P1-2: Redundant Subsystem Exposure**: Virtual controllers, I/O racks, and communications appear in both Studio, System, and Enterprise.
3. **P1-3: Ephemeral Alarm Annunciator**: The bottom alarm banner disappears when unacknowledged alarms are silenced, violating ISA-18.2 standing alarm awareness.
4. **P1-4: Static Breadcrumbs**: Operations navigation lacks active drill-down from Plant $\to$ Area $\to$ Unit $\to$ Equipment.

### Priority 2 (P2): Significant Usability Deficiencies
1. **P2-1: Unactionable Empty States**: Empty lists show plain text without actionable guidance buttons (`[Validate Project]`, `[Deploy to Runtime]`, `[Add Equipment]`).
2. **P2-2: Missing Global Command Palette (`Ctrl + K`)**: No keyboard-accelerated search for tags, loops, or navigation shortcuts.
3. **P2-3: Residual Interval Polling Loops**: `setInterval` used in secondary panels (`nebu-safety-panel`, `nebu-communications`, `nebu-hydraulic-inspector`) instead of reactive store binding.
4. **P2-4: Disconnected Cross-Reference Jump Action**: Table rows in `nebu-cross-reference` do not trigger canvas focus or inspector opening when clicked.

### Priority 3 (P3): Visual Polish & Consistency
1. **P3-1: Excessive SaaS Card Styling**: Generic card borders and rounded corners dilute industrial DCS feel.
2. **P3-2: Inconsistent Status Indicators**: Intermittent usage of "Loaded" vs "Deployed".
3. **P3-3: Faceplate Compactness**: Faceplates require vertical scrolling on 1080p when displaying all meters.

---

## 7. Visual Forensic Audit (20 Pre-Implementation States)

All 20 forensic states have been rendered, captured at $1920\times 1080$, and permanently archived in `/root/NebuDCS/docs/screenshots/audit_pre/` and the session artifact directory:

| Evidence Screenshot ID | Description & Inspected State | Visual Audit Findings |
| :--- | :--- | :--- |
| `audit_01_startup.png` | Application cold start in default Operations mode | Clean flowsheet, but header is dense; status bar shows cluster HA details to operator. |
| `audit_02_engineering_overview.png` | Engineering Studio initial view | P&ID canvas dominates center; 13 tabs across top create visual clutter. |
| `audit_03_engineering_pid.png` | P&ID flowsheet canvas with process unit | Equipment symbols (pumps, valves, columns) well rendered; port connection points clear. |
| `audit_04_engineering_equipment_config.png`| Property inspector pane | Inspector is thin; raw parameter key-value table lacks domain groupings. |
| `audit_05_engineering_project_readiness.png`| Design validation report | Validation results shown in modal; needs persistent pre-flight status checklist. |
| `audit_06_engineering_deployment.png` | Hot deploy pipeline | "🚀 Deploy to Runtime" button works well; status transitions to SYNCHRONIZED. |
| `audit_07_operations_overview.png` | Operations console overview | Clean separation of process graphic, faceplate, trend, and alarm station. |
| `audit_08_operations_process_graphic.png`| ISA-101 high-performance graphic | High contrast dark neutral palette; dynamic process lines and callouts. |
| `audit_09_operations_faceplate.png` | Selected control loop faceplate | Vertical PV/SP/OUT meters; AUTO/MAN/CAS mode selector; clear validity badge. |
| `audit_10_operations_alarm_state.png` | Active alarms in alarm panel | Distinct priority colors (Red critical, Yellow warning); ACK ALL / RESET separated. |
| `audit_11_operations_trend.png` | Multi-pen trend recorder | HTML5 Canvas line graphs; rolling window selector (30s to 24h); crosshair readout. |
| `audit_12_training_workspace.png` | OTS training studio | Comprehensive scenario and session management; instructor tabs visible to all. |
| `audit_13_validation_workspace.png` | Scientific validation browser | Error metrics (MAE/RMSE), NIST reference comparisons, and certified manifest export. |
| `audit_14_system_workspace.png` | System telemetry & diagnostics | Subsystem cycle times and jitter clean; virtual controllers and I/O consolidated. |
| `audit_15_enterprise_workspace.png` | Enterprise HA & cluster | Multi-node topology, epoch fencing, state snapshot, and DR backup panels. |
| `audit_16_runtime_stopped.png` | Simulation stopped / paused | `#sbSimClock` turns grey "PAUSED"; play button enabled; pause button disabled. |
| `audit_17_runtime_running.png` | Simulation active at 50 Hz | Simulation clock continuously advances ($2.15\text{s} \to 4.70\text{s}$); green "RUNNING @ 50 Hz". |
| `audit_18_runtime_faulted.png` | Fault injection in cluster | Controlled node lag and partition resilience cleanly reported. |
| `audit_19_empty_state.png` | Filtered empty tag table | Table shows empty rows; lacks actionable "Clear Filter" or "Add Tag" guidance. |
| `audit_20_error_state.png` | Trainee RBAC token active | Header role badge updates to TRAINEE; administrative endpoints return 403 Forbidden. |

---

## 8. Comparison Against Industrial Reference Systems

| Dimension | Rockwell Studio 5000 / FactoryTalk | Siemens TIA Portal / WinCC | Current NebuDCS Implementation | Target Redesign Specification |
| :--- | :--- | :--- | :--- | :--- |
| **Engineering Hierarchy** | Controller Organizer tree $\to$ Tasks $\to$ Programs $\to$ Routines $\to$ Tags | Project Tree $\to$ PLC $\to$ Devices & Networks $\to$ Program Blocks | Project Explorer tree has Plant $\to$ Area $\to$ Unit $\to$ Equipment | Keep Project Explorer; enhance with cross-reference counts and status flags. |
| **P&ID / Canvas Primacy**| Flowsheet remains open while properties dock around it | Network view / LAD editor stays anchored; property inspector tabs at bottom | 13 horizontal tabs unmount P&ID canvas on sub-tool selection | Anchor P&ID Canvas as primary workspace; open IEC, I/O, and logic in docked drawer panels. |
| **Equipment Configuration**| Multi-tab dialog: General, Connection, Configuration, Limits, Alarms | Device Configuration inspector with categorized property tree | Flat parameter key-value table | Capability-driven Inspector: General, Process, Hydraulics, Motor, Limits, Control, Alarms, Protection. |
| **DCS Operator Workflow**| FactoryTalk View SE: Area Overview $\to$ Unit Graphic $\to$ Faceplate $\to$ Alarm Banner | WinCC OA: Screen hierarchy $\to$ Symbol click $\to$ Standard faceplate | Flowsheet with side Faceplate and Trend | Add interactive Plant/Area breadcrumb navigation; persistent 2-row bottom alarm annunciator. |
| **Alarm Annunciation** | ISA-18.2 Alarm Summary banner pinned at screen footer | Pinned alarm line with top 3 unacknowledged alarms and ACK button | Single-line ticker that hides when unacknowledged count is zero | Fixed 2-row alarm annunciator bar with Priority, Timestamp, PV, ACK, and DETAIL jump shortcuts. |
| **Pre-Flight Readiness** | Compile / Verify All with error & warning summary pane | Software & Hardware Consistency Check pane | Modal validation report | Persistent bottom "Project Readiness" checklist with real-time pass/warn/error indicators. |

---

## 9. Recommended Component Moves & Consolidation

1. **Move Secondary Engineering Sub-Views to Docked Drawer Panels**:
   - Consolidate `nebu-iec-editor`, `nebu-io-mapping-table`, `nebu-cross-reference`, and `nebu-safety-panel` so they can be opened as bottom drawer tabs without unmounting `<nebu-pid-canvas>`.
2. **Consolidate Equipment Inspection into Single Capability-Driven Inspector**:
   - Upgrade `<nebu-property-inspector>` to render categorized capability tabs: `General`, `Process`, `Hydraulics`, `Motor`, `Limits`, `Control`, `Alarms`, `Protection`, `Historian`.
3. **Consolidate Project Readiness**:
   - Replace separate modal validation dialog with an integrated, docked Project Readiness bar below the canvas: `✓ Topology | ✓ Equipment | ✓ Streams | ✓ Tags | ✓ I/O | ✓ Controllers | ✓ IEC Logic | ⚠ SIS | ✓ Validation | [Validate Project] [Deploy to Runtime]`.
4. **Implement Global Command Palette (`<nebu-command-palette>`)**:
   - Add global `Ctrl + K` quick-jump modal searching tags, equipment, loops, and workspace navigation.
5. **Contextualize Global Status Bar by Role**:
   - When role is `operator`: show `SIM RUNNING`, `SIM TIME`, `SIS ARMED`, `ALARMS ACTIVE`, `I/O HEALTH`.
   - When role is `engineer`: show `SIM RUNNING`, `MODEL VALID`, `DEPLOYED`, `PROJECT READINESS`.
   - When role is `admin`: show `NODE`, `HA STATE`, `EPOCH`, `REPLICATION LAG`.
6. **Eliminate Residual Interval Polling**:
   - Convert `nebu-safety-panel`, `nebu-communications`, and `nebu-hydraulic-inspector` to reactively observe the central store instead of maintaining independent timers.

---

## 10. Target Information Architecture & Workflow Plan

```
NEBUDCS APPLICATION SHELL
│
├── TOP HEADER: [Brand] [Project / Unit Selector] [Command Palette (Ctrl+K)] [Sim Play/Pause/Step] [Role Switcher]
│
├── TOP WORKSPACES:
│   ├── 1. ENGINEERING (Build the plant)
│   │   ├── Left Sidebar: Project Explorer | Equipment Catalog
│   │   ├── Center Canvas: P&ID Flowsheet (Drag/Drop, Pan, Zoom, Grid, Connect)
│   │   ├── Right Inspector: Contextual Capability Inspector (General, Process, Hydraulics, Control, Alarms, SIS)
│   │   ├── Bottom Drawer: IEC Editor | I/O Mappings | Cross-Reference | SIS Safety Matrix
│   │   └── Bottom Footer: Persistent Project Readiness & Hot-Deploy Action Bar
│   │
│   ├── 2. OPERATIONS (Operate the plant)
│   │   ├── Top Breadcrumb: Plant / Area / Unit Drill-Down
│   │   ├── Flowsheet Viewport: ISA-101 High-Performance Graphic (Equipment run/trip, live values)
│   │   ├── Operational Subgrid: Loop & Equipment Faceplate + Multi-Pen Trend
│   │   ├── Sidebar: Live Tag Table with deadband filtering
│   │   └── Persistent Footer: 2-Row ISA-18.2 Alarm Annunciator Bar ([ACK], [DETAIL])
│   │
│   ├── 3. TRAINING (Train the operator)
│   │   ├── Role-Aware Views: Instructor Console & Scenario Editor (Hidden from Trainee)
│   │   └── Trainee Station: Full DCS Console with Training Objective HUD & Event Scoring
│   │
│   ├── 4. VALIDATION (Validate the model)
│   │   ├── Benchmark Suite: Numerical models, NIST datasets, MAE/RMSE metrics
│   │   └── Governance: Strict certification language policy ("INTERNALLY VALIDATED")
│   │
│   ├── 5. SYSTEM (Manage the runtime)
│   │   ├── 10-Subsystem Telemetry Dashboard
│   │   ├── Virtual Controllers & Task Scheduling
│   │   ├── I/O HAL Racks & Channel Diagnostics
│   │   └── Fieldbus Communications (Modbus, OPC UA, MQTT)
│   │
│   └── 6. ENTERPRISE (Manage the platform)
│       ├── Multi-Node HA Cluster Topology & Split-Brain Fencing
│       ├── State Replication Snapshots (SHA-256) & Failure Injection
│       └── DR Backup Export, Non-Mutating Dry-Run Restore & Audit Journal
│
└── GLOBAL STATUS BAR: Role-contextualized runtime observables subscribed to WebSocket store.
```

---

## 11. Incremental Implementation Order

1. **Step 1: Global Shell, Header, Role-Contextual Status Bar, and Command Palette (`nebu-app.ts`)**:
   - Streamline top header with project selector and `Ctrl + K` palette.
   - Contextualize status bar based on active role (`operator`, `engineer`, `admin`).
   - Implement persistent 2-row bottom alarm annunciator bar with [ACK] and [DETAIL] shortcuts.
2. **Step 2: Engineering Studio Layout & Capability Inspector (`nebu-studio.ts`, `nebu-property-inspector.ts`)**:
   - Anchor P&ID flowsheet canvas; replace 13 overflowing tabs with docked bottom drawer tabs.
   - Upgrade property inspector to capability-driven tabbed sections (General, Hydraulics, Motor, Control, Alarms, Protection).
3. **Step 3: Project Readiness & Pre-Flight Bar (`nebu-studio.ts`)**:
   - Implement persistent readiness matrix (`✓ Topology`, `✓ Tags`, `✓ I/O`, `✓ Logic`, `⚠ SIS`, `✓ Validation`) with direct "Deploy to Runtime" pipeline.
4. **Step 4: Interactive Operations Breadcrumb & Faceplate Integration (`nebu-dcs-shell.ts`, `nebu-faceplate.ts`)**:
   - Make breadcrumbs clickable for Plant $\to$ Area $\to$ Unit $\to$ Equipment navigation.
   - Add compact vs expanded view toggle on faceplates.
5. **Step 5: Trainee Role Isolation in Training Studio (`nebu-training-studio.ts`)**:
   - Hide instructor fault injection and scenario authoring tabs when role is `trainee`.
6. **Step 6: Eliminate Polling Residuals (`nebu-safety-panel.ts`, `nebu-communications.ts`, `nebu-hydraulic-inspector.ts`)**:
   - Refactor interval timers to reactive store subscriptions.
7. **Step 7: Verification & Post-Implementation Audit**:
   - Run Vitest frontend suite (105 tests), Cargo workspace tests, and Playwright verification capturing updated screenshots.

---

## 12. Conclusion & Authorization to Proceed

This forensic audit confirms that NebuDCS possesses all required backend and frontend industrial capabilities. The implementation plan addresses the exact usability, navigation, and inspection bottlenecks identified in the audit without breaking existing functionality or creating unauthorized roadmap stages.

**AUDIT APPROVED. PROCEEDING TO IMPLEMENTATION PLAN EXECUTION.**
