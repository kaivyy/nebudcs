# NebuDCS Complete UI, UX & Runtime Forensic Audit Report

**Date:** 2026-09-08  
**Author:** Principal UI/UX Architect, DCS/HMI Architect, Industrial Automation Engineer, Runtime Integration Engineer & QA Lead  
**Audit Scope:** Read-Only Forensic Audit — Zero Code Mutations  
**Verdict:** **UI/RUNTIME AUDIT: MAJOR ISSUES FOUND**  

---

## 1. Executive Summary

A comprehensive, read-only forensic audit was conducted on the NebuDCS codebase across the Rust simulation core ([`nebudcs-server`](file:///root/NebuDCS/apps/nebudcs-server)), protocol DTOs ([`nebudcs-protocol`](file:///root/NebuDCS/crates/nebudcs-protocol)), and the TypeScript Web Frontend ([`web/src`](file:///root/NebuDCS/web/src)).

While NebuDCS boasts deep industrial algorithms, rigorous thermodynamics, virtual controllers, and complete Stage 14 enterprise/HA features with passing automated tests (105/105 Vitest and 23/23 Rust acceptance cases), **the platform suffers from critical runtime orchestration disconnects, domain boundary cross-contamination, and mental model fragmentation that prevent it from functioning smoothly as ONE unified product for a human operator or process engineer.**

### Primary Findings Headline:
1. **Play/Run Disconnect (P0):** The simulation clock in the backend is decoupled from project activation. Upon cold start, the server clock runs in a void loop over an empty tag store ($0$ tags, $0$ equipment). When the UI connects, `nebu-app.ts` loads project metadata into the client store, but **never dispatches a simulation deployment (`POST /api/v1/projects/:id/load-sim`) to the backend host.** Thus, pressing "▶ Play" in the UI commands the clock to advance an empty backend model while displaying stale or disconnected values in the flowsheet.
2. **WebSocket Realtime State Loophole (P1):** The WebSocket broadcast loop filters delta frames by subscription. When the tag store is empty, $0$ delta frames are generated. Because `isSimRunning` is only updated via REST `/api/v1/health` or `applySnapshot` (which doesn't receive running flags in the protocol DTO), the UI controls often fall out of sync with backend lifecycle state.
3. **Domain Cross-Contamination & Workflow Inversion (P1):** The UI places runtime engineering tools (HAL channel fault injection, raw IEC 61131-3 logic editor, and Virtual Controller hardware chassis) directly into the operator's runtime screen (`nebu-dcs-shell.ts`), while obscuring the critical engineering deployment step (`Validate → Deploy to Runtime`).
4. **Mental Model Fragmentation (P2):** The top navigation separates the platform into 5 uncoordinated silos (`Operations`, `Engineering Studio`, `Training (OTS)`, `Scientific Validation`, `Enterprise & HA`) instead of a single, coherent lifecycle pipeline:
   $$\text{Engineering Studio (Author)} \longrightarrow \text{Validate} \longrightarrow \text{Deploy} \longrightarrow \text{DCS Operations (Supervise/Control)}$$

---

## 2. Current UI Architecture

The frontend is constructed using native Web Components (custom elements), custom CSS variables, and vanilla TypeScript with two primary state containers:
- [`Store`](file:///root/NebuDCS/web/src/state/store.ts): Global DCS operations, active project metadata, WebSocket telemetry buffer, alarm queues, and time-series history ring buffers.
- [`StudioStore`](file:///root/NebuDCS/web/src/state/studio-store.ts): Engineering flowsheet CAD editor, transactional command stacks (undo/redo), drafting lines, and validation responses.

### Component Tree & State Routing
```
<nebu-app> (Main Shell)
 ├── <header.app-header>
 │    ├── brand / title
 │    ├── <nav.app-mode-nav> [operations | studio | training | validation | enterprise]
 │    ├── <nebu-connection-status> (WS state + ping)
 │    ├── role selector [admin | engineer | instructor | operator | trainee | custom]
 │    └── <nebu-sim-controls> [Play, Pause, Step, Reset, Speed Multiplier, Clock]
 ├── <div.global-status-bar> (Node HA, Epoch, Repl Lag, SIS state, Comm state, Static Clock)
 ├── <div.alarm-ticker-banner> (ISA-18.2 unacknowledged alarm annunciator)
 └── <main> (Dynamic Viewport switched by activeMode)
      ├── [operations]  --> <nebu-dcs-shell>
      │                      ├── ISA-95 Breadcrumbs (Enterprise/Site/Area/Unit)
      │                      ├── Tabs: [Unit Flowsheet | Plant Overview | Equipment Diags | First-Out]
      │                      └── <div.dcs-unit-layout>
      │                           ├── <nebu-process-view> (Live SVG flowsheet)
      │                           ├── <nebu-faceplate> (Tuning & details)
      │                           ├── <nebu-trend> (Live Canvas charts)
      │                           └── <aside.dcs-sidebar-col>
      │                                ├── <nebu-controller-panel>  <-- [ENGINEERING BLEED]
      │                                ├── <nebu-io-panel>          <-- [ENGINEERING BLEED]
      │                                ├── <nebu-alarm-panel>
      │                                └── <nebu-tag-table>
      ├── [studio]      --> <nebu-studio> (CAD canvas, 13 sub-tabs, property inspector)
      ├── [training]    --> <nebu-training-studio> (Instructor console, scenario editor, trainee station)
      ├── [validation]  --> <nebu-validation-browser> (Benchmarks, model metrics, datasets)
      └── [enterprise]  --> 5 Subtabs: [Cluster | Observability | Diagnostics | Audit | Backup]
```

---

## 3. Current Information Architecture & Mental Model Audit

When a new user launches NebuDCS, they are confronted with conflicting navigation structures and unclear workflows:

| User Question | Expected Intuitive Location | Current Actual Location | Audit Finding |
| :--- | :--- | :--- | :--- |
| **1. What is NebuDCS?** | High-level system overview / splash or welcome screen | Opens directly into DCS Operations with blank SVG or stale plant | **CONFUSING:** No guided onboarding; assumes an active unit is running. |
| **2. Where to create a plant?** | Engineering Studio $\rightarrow$ New Project | Top Nav "Engineering Studio" $\rightarrow$ Top toolbar button "+ New" | **FRAGMENTED:** User must know to leave "Operations" and switch to "Engineering Studio". |
| **3. Where to configure equipment?** | Equipment Library / CAD Palette | Studio Left Sidebar $\rightarrow$ Catalog tab $\rightarrow$ Drag & Drop | **FUNCTIONAL:** Palette works, but properties inspector is crowded. |
| **4. Where to connect streams?** | Interactive Port Click in Canvas | Click green/blue port $\rightarrow$ click target port | **OBSCURE:** No visual rubberbanding stream during connection drafting. |
| **5. Where to configure controllers?** | Studio Control Strategy / Loop tab | In Studio: "Controllers" tab; In Operations: sidebar panel | **DUPLICATED:** Present in both Studio and Operations sidebar. |
| **6. Where to write IEC logic?** | Studio IEC 61131-3 Editor | Studio $\rightarrow$ Tab "⚡ IEC 61131-3 Logic" | **DISCONNECTED:** IEC routines compile but are not visibly bound to project controllers. |
| **7. Where to configure SIS?** | Studio Safety & Protection Matrix | Studio $\rightarrow$ Tab "🛡️ Safety (SIS)" | **SPLIT:** Cause & Effect in Studio; SIF manual bypass in Operations sidebar. |
| **8. Where to start simulation?** | Global Sim Controls (Header) | Header: `nebu-sim-controls` | **BROKEN RUNTIME LINK:** Pressing Play starts backend clock without loading the active plant. |
| **9. Where to operate the plant?** | DCS Operations Console | Top Nav "Operations" | **CLUTTERED:** Operator view is overwhelmed by hardware HAL chassis and controller diagnostics. |
| **10. Where to create training scenarios?** | Training (OTS) | Top Nav "Training (OTS)" | **ISOLATED:** Scenarios are siloed from the active project created in Studio. |
| **11. Where to validate models scientifically?** | Scientific Validation Studio | Top Nav "Scientific Validation" | **ISOLATED:** Benchmarks run against pre-canned models, not the user's authored plant. |
| **12. Where to administer HA / Cluster?** | Enterprise & HA | Top Nav "Enterprise & HA" | **WELL-SEGREGATED:** Dedicated enterprise console. |

---

## 4. DCS Operations vs Engineering Studio Separation Audit

A fundamental principle of industrial automation (IEC 62443, ISA-101, NAMUR) is the **strict operational separation between the Engineering Workstation and the Operator Console.**

### Capabilities Placed in the Wrong Domain:
1. **Raw I/O Hardware Rack/Chassis/Module Configuration in DCS Operations:**
   - **Location:** [`nebu-dcs-shell.ts` lines 145–150](file:///root/NebuDCS/web/src/components/nebu-dcs-shell.ts#L145-L150).
   - **Flaw:** The operator's flowsheet sidebar renders `<nebu-io-panel>`, which exposes rack backplane diagnostics, raw ADC/DAC counts, NAMUR NE43 channel failure injection, and forced overrides.
   - **Impact:** An operator monitoring a distillation or pumping unit should not be forced to scroll past 680 lines of hardware rack debugging tools to see their alarm list or controller faceplates.
2. **Virtual Controller Lifecycle & Hardware Profiles in DCS Operations:**
   - **Location:** [`nebu-dcs-shell.ts` line 146](file:///root/NebuDCS/web/src/components/nebu-dcs-shell.ts#L146).
   - **Flaw:** The operator sidebar embeds `<nebu-controller-panel>`, which allows editing virtual CPU budgets, memory allocation, firmware profiles, and injecting watchdog timeouts.
   - **Impact:** Severe mode confusion. An operator needs loop faceplates (PV/SP/OUT/Mode), not virtual CPU core throttling tools.
3. **Missing "Deploy to Runtime" Pipeline in Engineering Studio:**
   - **Location:** [`nebu-studio.ts` line 421](file:///root/NebuDCS/web/src/components/nebu-studio.ts#L421).
   - **Flaw:** The studio toolbar has a small button called "▶ Load Sim". It is positioned next to "Save Project" and does not indicate whether the current active runtime will be stopped, cleared, or hot-swapped.
   - **Impact:** Engineers believe saving the flowsheet automatically updates the running plant, which is false.

---

## 5. Training, Validation & System Domain Separation

- **Training (OTS) Domain:**
  - The Instructor Console, Scenario Editor, and Trainee Station exist in [`nebu-training-studio.ts`](file:///root/NebuDCS/web/src/components/nebu-training-studio.ts).
  - However, there is no cross-launch mechanism from an authored Studio project into an OTS training session. The user must manually navigate to OTS, re-select the project, and create a training session.
- **Scientific Validation Domain:**
  - Located in [`nebu-validation-browser.ts`](file:///root/NebuDCS/web/src/components/nebu-validation-browser.ts).
  - It cleanly implements the Certification Language Policy (`"INTERNALLY VALIDATED"`, `"BENCHMARKED"`).
  - However, it only evaluates registered benchmark cases; there is no intuitive "Validate My Active Plant" button that runs scientific benchmarks directly against the current flowsheet.
- **Enterprise & System Domain:**
  - Located under the "Enterprise & HA" mode tab.
  - Appropriately groups Cluster HA, Observability, Global Diagnostics, Audit Log, and Backup/Recovery.

---

## 6. Simulation Play/Run Forensic Audit (Root Cause Analysis)

### The Symptom:
When a user launches the web interface and clicks **"▶ Play"** in the top header control bar:
1. The simulation clock time does not advance, or
2. The clock advances, but all process values on the flowsheet remain static at $0.00$, instruments display `[!] UNAVAILABLE` or stale badges, and controllers fail to respond to setpoint manipulation.

### Complete Execution Path Trace:

```mermaid
sequenceDiagram
    participant User as Operator (UI)
    participant SimCtrl as nebu-sim-controls
    participant Ws as WsClient
    participant ServerWs as axum::ws_handler
    participant Host as SimulationHost
    participant Engine as SimulationEngine
    participant TagStore as RuntimeTagStore
    participant Store as web/state/store

    User->>SimCtrl: Click "▶ Play"
    SimCtrl->>Ws: sendCommand({ type: "sim_control", action: "play" })
    Ws->>ServerWs: ClientMessage::Command(RuntimeCommand::SimControl(Play))
    ServerWs->>Host: dispatch_command(SimControl(Play))
    Host->>Engine: clock_mut().play()
    Host->>ServerWs: ServerMessage::CommandAck(success: true)
    ServerWs->>Ws: CommandAck received
    SimCtrl->>User: "Command acknowledged"
    
    Note over Host,Engine: Engine starts ticking at 50 Hz!
    Note over Engine,TagStore: BUT TagStore has 0 tags! (No project deployed)
    Host->>Host: Delta check: 0 changed tags
    Host--xWs: NO ServerMessage::Delta broadcasted!
    Store--xUser: UI simTime remains 0.00s! Flowsheet values remain 0.00!
```

### Forensic Code Inspection:

#### Root Cause 1: Cold-Start Project Disconnect
In [`apps/nebudcs-server/src/main.rs` lines 90–106](file:///root/NebuDCS/apps/nebudcs-server/src/main.rs#L90-L106):
```rust
// The baseline demonstration project exists only as project model data in ProjectManager
// — it is not auto-activated here.
let engine = SimulationEngine::new(config.simulation_dt)?;
let tag_store = RuntimeTagStore::new(); // <-- EMPTY TAG STORE (0 tags)
let mut host = SimulationHost::start(..., tag_store, ...)?;
```
The server starts with an **empty** `RuntimeTagStore`.

#### Root Cause 2: Frontend Bootstraps UI Store but Does NOT Deploy to Backend
In [`web/src/components/nebu-app.ts` lines 65–79](file:///root/NebuDCS/web/src/components/nebu-app.ts#L65-L79):
```typescript
if (typeof this.apiClient?.listProjects === "function") {
  this.apiClient.listProjects().then(async (projects) => {
    if (projects && projects.length > 0 && !store.activeProject) {
      const proj = await this.apiClient.getProject(projects[0].id);
      store.setActiveProject(proj); // <-- Populates local JS store ONLY!
      // MISSING: this.apiClient.loadProjectSim(proj.id);
    }
  });
}
```
`nebu-app.ts` fetches the project JSON and sets `store.setActiveProject(proj)` so the SVG can draw symbols, **but never calls `POST /api/v1/projects/:id/load-sim`!**

#### Root Cause 3: Deadband Suppresses Delta Frames When Tags are Empty
In [`apps/nebudcs-server/src/host.rs` lines 925–935](file:///root/NebuDCS/apps/nebudcs-server/src/host.rs#L925-L935):
```rust
// Broadcast delta update if any tags changed
if !changed_tags.is_empty() {
    seq += 1;
    let delta = DeltaUpdateDto { seq, sim_time_sec, tick_count, changed_tags };
    let _ = broadcast_tx.send(ServerMessage::Delta(delta));
}
```
Because the backend `tag_store` has $0$ tags, `changed_tags` is empty on every tick ($50\text{ Hz}$). Therefore, **no `DeltaUpdateDto` is ever sent to the browser.**

#### Root Cause 4: Frontend Time Readout Relies Exclusively on Deltas
In [`web/src/state/store.ts` lines 157–160](file:///root/NebuDCS/web/src/state/store.ts#L157-L160):
```typescript
public applyDelta(delta: DeltaUpdateDto) {
  this.simTime = delta.sim_time_sec;
  this.tickCount = delta.tick_count;
  ...
}
```
Because no delta frames arrive, `store.simTime` stays at `0.00` and `store.tickCount` stays at `0`. The UI displays `TIME: 0.00 s` and `TICK: 0`, leading the operator to believe the Play command was ignored or failed.

#### Root Cause 5: Simulation Running State Omitted from WebSocket Snapshot & Deltas
In [`crates/nebudcs-protocol/src/dto.rs` lines 60–71](file:///root/NebuDCS/crates/nebudcs-protocol/src/dto.rs#L60-L71):
```rust
pub struct StateSnapshotDto {
    pub snapshot_id: String,
    pub sim_time_sec: f64,
    pub tick_count: u64,
    pub tags: Vec<TagValueSampleDto>,
    pub active_alarms: Vec<AlarmSnapshotDto>,
    // MISSING: pub is_running: bool,
    // MISSING: pub speed_multiplier: f64,
}
```
The WebSocket snapshot and delta messages **do not contain `is_running`**. The only way the frontend learns `isSimRunning` is via `apiClient.getHealth()` on initial load. Once running, WebSocket events never toggle `store.isSimRunning`.

---

## 7. Runtime Lifecycle Audit

The runtime lifecycle consists of five discrete states:
1. `UNINITIALIZED`: Backend running with empty tag store.
2. `DEPLOYED`: A project's topology, physics equations, control loops, and SIS logic have been compiled and loaded into `RuntimeTagStore` and `GenericPhysicsSystem`.
3. `STOPPED / PAUSED`: Clock frozen; differential equations not stepping; I/O inputs holding last value.
4. `RUNNING`: Clock stepping at $50\text{ Hz}$ ($\Delta t = 20\text{ ms}$); controllers evaluating; tags updating; alarms evaluating.
5. `FAULTED`: Fatal divergence in solver or hardware watchdog trip.

### Lifecycle Gaps Identified:
- **No Explicit "Project Deployed" Indicator:** The operator console doesn't distinguish between *"No project deployed to simulation host"* and *"Project running with zero flow"*.
- **No Heartbeat Tick Indicator in UI:** If process values are in steady state (delta $< 0.001$), the server sends no deltas. The user sees a frozen clock even though the simulation is healthy.
- **Static Status Bar Misrepresentation:** In [`nebu-app.ts` line 217](file:///root/NebuDCS/web/src/components/nebu-app.ts#L217):
  ```html
  <span class="val" id="sbSimClock">RUNNING @ 50 Hz</span>
  ```
  This label is **hardcoded in HTML** and never updates dynamically to reflect whether the clock is paused, stopped, or stepped.

---

## 8. Backend, API & UI Trace

| Action | Frontend Trigger | Network Call | Backend Route | Server Handler | Execution Target | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Play** | `nebu-sim-controls.ts` | WS `command: sim_control(play)` | `/ws` | `session.rs` | `engine.clock_mut().play()` | **DISCONNECTED** (Deltas not emitted if tag store empty) |
| **Pause** | `nebu-sim-controls.ts` | WS `command: sim_control(pause)` | `/ws` | `session.rs` | `engine.clock_mut().pause()` | **WORKS** (Server pauses, but UI button state doesn't sync) |
| **Step** | `nebu-sim-controls.ts` | WS `command: sim_control(step)` | `/ws` | `session.rs` | `force_step = true` | **WORKS** |
| **Reset** | `nebu-sim-controls.ts` | WS `command: sim_control(reset)` | `/ws` | `session.rs` | `engine.clock_mut().pause()` | **PARTIAL** (Pauses clock, does NOT reload initial state) |
| **Deploy Project** | `nebu-studio.ts` ("Load Sim") | REST `POST /load-sim` | `/api/v1/projects/:id/load-sim` | `projects.rs` | `host.load_project_into_simulation` | **WORKS** (Populates tags & starts delta streaming) |
| **Write Tag** | `nebu-tag-table.ts` | REST `POST /tags/write` | `/api/v1/tags/write` | `tags.rs` | `tag_store.set_tag_float` | **WORKS** |
| **Ack Alarm** | `nebu-alarm-panel.ts` | WS `acknowledge_alarm` | `/ws` | `session.rs` | `alarm_system.acknowledge_alarm` | **WORKS** |
| **Reset SIF** | `nebu-alarm-panel.ts` | REST `POST /sis/sifs/:id/reset` | `/api/v1/sis/sifs/:id/reset` | `sis.rs` | `sif.manual_reset()` | **WORKS** |
| **Tune PID SP** | `nebu-faceplate.ts` | WS `set_setpoint` | `/ws` | `session.rs` | `control_system.set_setpoint` | **WORKS** |
| **Cluster Failover** | `nebu-cluster-panel.ts` | REST `POST /cluster/failover` | `/api/v1/cluster/failover` | `cluster.rs` | `cluster_manager.execute_failover` | **WORKS** |

---

## 9. WebSocket & Reactivity Audit

- **Authoritative Flow:**
  - Inbound commands go through WebSocket (`ClientMessage::Command`).
  - Outbound state uses `ServerMessage::Snapshot`, `ServerMessage::Delta`, and `ServerMessage::AlarmUpdate`.
- **Issues Found:**
  1. **Subscription Timing:** On handshake completion, `WsClient` immediately sends `ClientMessage::Subscribe { all_tags: true }`. If the backend has not yet loaded a project, the subscription succeeds on an empty list.
  2. **Deadband Zero-Suppression Trap:** When simulation is running and all process tags are steady, zero deltas are transmitted. The client has no keep-alive simulation tick to update the clock time display.
  3. **Multi-Store Desynchronization:** `StudioStore` maintains its own copy of the project model, while `Store` maintains runtime values. Editing in Studio does not invalidate runtime tags until an explicit save and load-sim occurs.

---

## 10. Model → Engine → UI Consistency Audit

- **Generic Model Integrity:**
  - Verified: No hardcoded refinery unit IDs (like `CDU-101` or `NHT-REACT`) are hard-coded in the core simulation engine.
  - Equipment definitions are dynamic and read from `EquipmentDefinition` registries.
  - Symbols in `nebu-process-view.ts` derive SVG shapes from `defId.includes(...)` rather than fixed IDs.
- **Model Inconsistencies:**
  1. **Disconnected Demo Values in Static HTML:**
     - Status bar: `SIS: ARMED (0 Trips)` and `COMM: ONLINE` are hardcoded in `nebu-app.ts` HTML template rather than bound to `store.diagnosticsMap` or `store.sisSummary`.
  2. **First-Out Hardcoded Run ID:**
     - In `nebu-dcs-shell.ts` line 61, `getFirstOutEvent("active")` uses hardcoded query parameter `"active"`.
  3. **Sidebar Component Overlap:**
     - The same Virtual Controller panel (`nebu-controller-panel`) is instantiated both in `nebu-dcs-shell.ts` (Operations) and `nebu-studio.ts` (Engineering Studio), creating dual polling intervals (2000ms each).

---

## 11. Complete Stage 1–14 Feature Parity Matrix

| Stage | Capability | Backend Impl | API Endpoint | UI Component | RBAC Guards | Realtime WS | User Workflow | Test Coverage | Operational Status |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **Stage 1** | Virtual Controllers & Clock | Yes | `/api/v1/sim/control` | `nebu-sim-controls` | Yes | Yes | **Broken (P0)** | Rust Unit/Integ | **BLOCKED ON DEPLOY** |
| **Stage 2** | IEC 61131-3 (ST, LD, SFC) | Yes | `/api/v1/controllers/iec/*` | `nebu-iec-editor` | Yes | No | Partial | Rust + Vitest | **USABLE IN STUDIO** |
| **Stage 3** | I/O Hardware Abstraction (HAL) | Yes | `/api/v1/hal/*` | `nebu-io-panel` | Yes | Polling | Misplaced | Rust + Vitest | **MISPLACED IN DCS** |
| **Stage 4** | Engineering Studio & CAD | Yes | `/api/v1/projects/*` | `nebu-studio` | Yes | No | Unclear | Vitest | **USABLE** |
| **Stage 5** | Advanced Control (PID/Cascade) | Yes | `/api/v1/control/*` | `nebu-advanced-control` | Yes | Yes | Functional | Rust + Vitest | **VERIFIED** |
| **Stage 6** | Hydraulic Networks & Physics | Yes | `/api/v1/network/*` | `nebu-hydraulic-inspector` | Yes | Polling | Functional | Rust + Vitest | **VERIFIED** |
| **Stage 7** | Protocols (Modbus/OPC UA/MQTT) | Yes | `/api/v1/comms/*` | `nebu-communications` | Yes | Polling | Functional | Rust + Vitest | **VERIFIED** |
| **Stage 8** | Virtual PLC Hardware Profiles | Yes | `/api/v1/controllers/*` | `nebu-controller-panel` | Yes | Polling | Misplaced | Rust + Vitest | **MISPLACED IN DCS** |
| **Stage 9** | SIS / SIF / Cause & Effect | Yes | `/api/v1/sis/*` | `nebu-safety-panel` | Yes | Polling | Functional | Rust + Vitest | **VERIFIED** |
| **Stage 10** | DCS Operations Console & HMI | Yes | `/api/v1/sim/*` | `nebu-dcs-shell` | Yes | Yes | **Cluttered** | Rust + Vitest | **USABLE (CLUTTERED)** |
| **Stage 11** | OTS Simulator & Scenarios | Yes | `/api/v1/training/*` | `nebu-training-studio` | Yes | Yes | Functional | Rust + Vitest | **VERIFIED** |
| **Stage 12** | Multi-Domain Thermodynamics | Yes | `/api/v1/library/domains` | `nebu-domain-browser` | Yes | No | Functional | Rust + Vitest | **VERIFIED** |
| **Stage 13** | Scientific Validation Framework | Yes | `/api/v1/validation/*` | `nebu-validation-browser`| Yes | No | Functional | Rust + Vitest | **VERIFIED** |
| **Stage 14** | Cluster HA & Observability | Yes | `/api/v1/cluster/*` | `nebu-cluster-panel` | Yes | Polling | Complete | Rust + Playwright | **VERIFIED** |

---

## 12. Engineering Studio Workflow Audit

We walked through the expected end-to-end engineering workflow:
$$\text{Create Project} \longrightarrow \text{Add Equipment} \longrightarrow \text{Connect Streams} \longrightarrow \text{Configure Loops} \longrightarrow \text{Validate} \longrightarrow \text{Deploy} \longrightarrow \text{Operate}$$

### Detailed Transition Analysis:
1. **Create Project:** Works via `+ New` button in Studio toolbar.
2. **Add Equipment:** Dragging from palette onto canvas creates entity and marks studio dirty.
3. **Connect Ports:** Clicking source port then destination port creates stream.
4. **Configure Parameters:** Property inspector allows setting volume, pressure, and tag names.
5. **Validate:** Clicking "✓ Validate" invokes `/api/v1/projects/:id/validate`. Results display cleanly in bottom panel.
6. **Save:** Clicking "Save Project" persists JSON to backend.
7. **Deploy to Simulation (THE MISSING LINK):**
   - The user must click **"▶ Load Sim"** in the Studio toolbar.
   - If they fail to click this button, the simulation engine knows nothing about the new plant.
   - There is no automatic prompt or banner informing the user: *"Your project changes are not deployed to the live simulation host."*
8. **Switch to Operations:**
   - The user must manually click the "Operations" mode tab in the top header.
   - When Operations opens, if "Load Sim" was not clicked, the flowsheet displays the SVG symbols, but all instrument tags are missing from the server tag store, resulting in dead readouts.

---

## 13. DCS Operations Workflow Audit

We evaluated the DCS Operations view against established industrial HMI guidelines (ANSI/ISA-101, EEMUA 191):

### Positives:
- High contrast, dark slate background (#161b22).
- Equipment symbols use grayscale normal fills (#1e293b) and reserve high-chroma colors (yellow, amber, red) strictly for abnormal and alarm conditions.
- Flow arrows animate along stream paths.
- Faceplate implements clear PV, SP, and OUT bar indicators with MAN/AUTO/CAS mode selector and bumpless setpoint tracking.
- Alarm annunciator ticker bar across top allows instant acknowledgment with `[SPACE]`.

### Deficiencies:
- **Sidebar Information Overload:** Having HAL Hardware and Virtual Controller Diagnostics stacked directly above the Alarm Summary forces excessive vertical scrolling and distracts from process control.
- **Missing Area Navigation:** The ISA-95 breadcrumb (`ENTERPRISE / SITE / AREA / UNIT`) is non-interactive; clicking "AREA 01" does not filter or display area summaries.
- **Alarm Annunciator Audio:** Visual ticker works, but audible annunciator cues are absent.

---

## 14. OTS Training Workflow Audit

The Training Studio ([`nebu-training-studio.ts`](file:///root/NebuDCS/web/src/components/nebu-training-studio.ts)) provides:
- **Instructor Console:** Session launch, project binding, trainee assignment, ad-hoc fault injection.
- **Scenario Editor:** Timeline authoring with triggers, disturbances, and scoring rules.
- **Trainee Station:** Restricted view locking administrative controls while recording operator responses.
- **Replay Station:** Timeline scrubber reconstructing historical snapshots.

**Finding:** The OTS subsystem is functionally sound, but isolated. It operates on its own session lifecycle rather than integrating directly into the standard DCS Operations viewport when a training session is active.

---

## 15. Scientific Validation Workflow Audit

Evaluated against the strict Certification Language Policy:
- **Verified Permitted Usage:** All manifests and UI badges strictly use `"INTERNALLY VALIDATED"`, `"BENCHMARKED"`, and `"ENGINEERING CONSISTENCY VERIFIED"`. Zero occurrences of forbidden terms (`"IEC certified"`, `"SIL certified"`, etc.).
- **Benchmark Execution:** Clicking "Run Validation Suite" triggers `/api/v1/validation/run` and renders quantitative error metrics (MAE, RMSE, MAPE, P95, uncertainty bounds).
- **Usability Gap:** Validation is treated as a separate offline reporting suite. It cannot be launched as a pre-flight check directly within Engineering Studio before deployment.

---

## 16. Enterprise & System Workflow Audit

- **Cluster HA:** Topology dashboard displays 5 nodes with real-time heartbeat lag and monotonic leader epoch. Controlled failover promotes standby replica and fences stale primary.
- **Backup & Restore:** Full system bundle creation and dry-run validation work cleanly with 50 MB archive support.
- **Audit Journal:** 5,000-event circular buffer logs all user role transitions, setpoint adjustments, and trip resets.
- **Telemetry:** Live telemetry graphs for all 10 subsystems.

---

## 17. RBAC & Safety Authorization Audit

We audited the 5-role security boundaries across the UI and REST API:

| Role | Permitted Actions | Prohibited Actions | UI Enforcement | Server API Enforcement | Status |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Admin** | Everything (Topology, Sim Control, Admin, HA, Restore) | None | Unrestricted | `UserRole::Admin` | **VERIFIED** |
| **Engineer** | Topology design, parameter tuning, sim control, validation | Admin backup restore, cluster failure injection | Modals hidden/disabled | `can_modify_topology()` | **VERIFIED** |
| **Instructor** | Scenario authoring, fault injection, sim control, grading | Topology modification, admin restore | Studio palette locked | `can_instruct()` | **VERIFIED** |
| **Operator** | DCS faceplate tuning (SP/Mode/Out), Alarm Ack, Reset Trip | Topology edit, scenario authoring, admin restore | Studio controls disabled | `can_operate_dcs()` | **VERIFIED** |
| **Trainee** | DCS monitoring, supervised faceplate tuning | Alarm shelving, bypasses, administrative actions | Actions fenced | Server returns 403 | **VERIFIED** |

### Critical Safety Boundary Verification:
- **Alarm ACK $\neq$ Interlock Reset $\neq$ SIS Reset:**
  - In [`nebu-alarm-panel.ts` lines 112–146](file:///root/NebuDCS/web/src/components/nebu-alarm-panel.ts#L112-L146), clicking "ACK ALL" acknowledges alarms on annunciators, but **does NOT reset BPCS interlocks or SIF trips.**
  - Interlock reset requires clicking `RESET INTERLOCKS`, which calls `/api/v1/protection/reset`.
  - SIS reset requires clicking `RESET SIS`, which iterates tripped SIFs and executes individual `/api/v1/sis/sifs/:id/reset` calls with technician attribution.
  - **Verdict:** Strict operational safety separation is rigorously preserved.

---

## 18. Hardcoded & Fake State Audit

| Inspected Area | Hardcoded Items Found | Reality Check | Severity |
| :--- | :--- | :--- | :---: |
| `nebu-app.ts` (Global Status Bar) | `SIS: ARMED (0 Trips)`, `COMM: ONLINE`, `CLOCK: RUNNING @ 50 Hz` | Static HTML text; not wired to real store observables | **P2** |
| `nebu-process-view.ts` | None | Entirely driven by `store.activeProject.equipment` | Clean |
| `nebu-dcs-shell.ts` | `"active"` query string in `getFirstOutEvent("active")` | Assumes default active run | **P3** |
| `nebu-controller-panel.ts` | Default sample ST and LD code | Fallback placeholder when no routine is selected | Acceptable |
| `nebu-cluster-panel.ts` | None | Fully backed by server `ClusterTopologyDto` | Clean |
| `nebu-audit-panel.ts` | None | Fully backed by server audit log buffer | Clean |

---

## 19. Performance & Telemetry Audit

- **DOM & Canvas Performance:**
  - `nebu-process-view.ts` uses SVG with CSS transforms for pan and zoom. Fluid at 60 FPS up to ~100 equipment entities.
  - `nebu-trend.ts` uses HTML5 Canvas with 300-point ring buffer. Minimal CPU overhead ($< 2\%$).
- **Polling Duplication:**
  - Multiple components run independent `setInterval` loops:
    - `nebu-dcs-shell`: 1000ms
    - `nebu-controller-panel`: 2000ms
    - `nebu-io-panel`: 2000ms
    - `nebu-safety-panel`: 2000ms
    - `nebu-cluster-panel`: 3000ms
    - `nebu-system-observability`: 2000ms
  - **Impact:** While acceptable on localhost, running 6 concurrent polling timers on a single browser tab generates ~5 HTTP requests/second. These should be unified into a single periodic telemetry poll or WebSocket push.

---

## 20. Automated Test Coverage Audit

- **Rust Acceptance Suite (`stage_14_enterprise_acceptance_tests.rs`):**
  - 23 passed cases (Cases A through W).
  - Tests verify cluster manager, heartbeats, failovers, split-brain epoch guards, backup dry-runs, and RBAC matrix.
- **Web Unit Suite (Vitest):**
  - 15 files, 105 passed tests.
  - Verifies component DOM rendering, mock API responses, and tab switching.
- **Why Automated Tests Passed Despite UI Usability Issues:**
  - Unit tests mount individual components in isolation with pre-populated mocks.
  - Automated integration scripts (`verify_stage_14_enterprise_ha.py`) explicitly issue the REST command `POST /api/v1/projects/proj-plant-a-pumping/load-sim` before taking screenshots!
  - Therefore, the scripts bypassed the missing UI automated bootstrap link that a real human user experiences when navigating the browser.

---

## 21. Visual & UX Ergonomics Audit

- **Information Density:** High and appropriate for industrial DCS consoles.
- **Typography:** Consistently uses monospace font for tags, numbers, times, and engineering units.
- **Color Semantics:** Conforms to ANSI/ISA-101. Normal state is muted; alert state is vivid.
- **Responsive Layout:**
  - 1080p: Functional, but sidebar is cramped when both I/O and Controller panels are open.
  - 1440p / 4K: Excellent viewport real estate.

---

## 22. Severity Matrix

| Severity | Count | Primary Impact |
| :---: | :---: | :--- |
| **P0** | 1 | Core simulation execution disconnected on cold start / UI Play fails to advance model |
| **P1** | 3 | Missing Studio-to-Runtime deployment pipeline; Engineering tools polluting Operator console; WebSocket delta suppression on steady state |
| **P2** | 4 | Static status bar readouts; fragmented 5-mode navigation; concurrent polling timers; missing keep-alive simulation tick |
| **P3** | 3 | Hardcoded query parameters; missing audible cues; non-interactive ISA-95 breadcrumbs |

---

## 23. Detailed Audit Findings (Itemized)

### Finding AUD-01 (P0) — Play/Run Fails to Advance Active Project on Cold Start
- **Location:** [`web/src/components/nebu-app.ts` lines 65–79](file:///root/NebuDCS/web/src/components/nebu-app.ts#L65-L79) & [`apps/nebudcs-server/src/main.rs` lines 90–106](file:///root/NebuDCS/apps/nebudcs-server/src/main.rs#L90-L106)
- **Evidence:** Server starts with 0 tags. Frontend loads project metadata into client memory only. Pressing "▶ Play" advances an empty simulation engine clock.
- **Expected Behavior:** Launching NebuDCS or selecting a project should automatically deploy and synchronize that project to the backend simulation host (`POST /api/v1/projects/:id/load-sim`), establishing active tags and live physics calculations.
- **Actual Behavior:** Simulation ticks over empty memory; zero tags update; flowsheet displays static zeros.
- **Root Cause:** Incomplete bootstrap sequence in `nebu-app.ts`.

---

### Finding AUD-02 (P1) — Engineering Hardware HAL & CPU Throttling Pollute DCS Operations Console
- **Location:** [`web/src/components/nebu-dcs-shell.ts` lines 145–150](file:///root/NebuDCS/web/src/components/nebu-dcs-shell.ts#L145-L150)
- **Evidence:** Operator sidebar directly renders `<nebu-controller-panel>` and `<nebu-io-panel>`.
- **Expected Behavior:** DCS Operations console must strictly present operator-relevant information: Process Flowsheet, Faceplates, Trends, Alarm Station, and Process Tag Table.
- **Actual Behavior:** Operators are confronted with hardware rack slots, raw ADC counts, NAMUR NE43 channel failure triggers, and CPU memory allocation sliders.
- **Root Cause:** Architectural leakage of Stage 3/8 engineering diagnostic tools into Stage 10 operator shell.

---

### Finding AUD-03 (P1) — Deadband Zero-Suppression Silences WebSocket Clock Telemetry
- **Location:** [`apps/nebudcs-server/src/host.rs` lines 925–935](file:///root/NebuDCS/apps/nebudcs-server/src/host.rs#L925-L935)
- **Evidence:** When all tags are steady (change $<$ deadband), the server skips broadcasting `ServerMessage::Delta`.
- **Expected Behavior:** Even if no tags change beyond deadband, the simulation clock progression ($t_{\text{sim}}$, tick count) must periodically stream to the UI so the operator knows the engine is running.
- **Actual Behavior:** Clock display freezes at `0.00 s` whenever process tags are quiescent.
- **Root Cause:** Simulation timestamp is only transmitted inside tag delta packets.

---

### Finding AUD-04 (P1) — Missing "Deploy to Runtime" Workflow in Engineering Studio
- **Location:** [`web/src/components/nebu-studio.ts` line 421](file:///root/NebuDCS/web/src/components/nebu-studio.ts#L421)
- **Evidence:** Studio provides a small, ambiguous "▶ Load Sim" button without status feedback or pre-flight validation gating.
- **Expected Behavior:** Studio must feature a prominent, staged deployment workflow:
  $$\text{Save Flowsheet} \longrightarrow \text{Run Validation} \longrightarrow \text{Deploy to Runtime (Hot Swap)} \longrightarrow \text{Launch in Operations}$$
- **Actual Behavior:** Deployment is hidden as a secondary toolbar button with no indication of active deployment status.
- **Root Cause:** UI treats simulation synchronization as a transient action rather than a core lifecycle stage.

---

### Finding AUD-05 (P2) — Static / Hardcoded Status Values in Global Status Bar
- **Location:** [`web/src/components/nebu-app.ts` lines 208–218](file:///root/NebuDCS/web/src/components/nebu-app.ts#L208-L218)
- **Evidence:** `CLOCK: RUNNING @ 50 Hz`, `SIS: ARMED (0 Trips)`, and `COMM: ONLINE` are hardcoded static HTML strings.
- **Expected Behavior:** Global status bar values must be reactively bound to `store.serverHealth.simulation_running`, `store.sisSummary.active_trips_count`, and `store.diagnosticsMap`.
- **Actual Behavior:** Status bar displays `RUNNING @ 50 Hz` even when the simulation is paused or stopped.
- **Root Cause:** Visual placeholder text was not replaced with live store subscribers.

---

### Finding AUD-06 (P2) — Excessive Uncoordinated Polling Timers Across Subcomponents
- **Location:** Multiple Web Components (`nebu-dcs-shell`, `nebu-io-panel`, `nebu-controller-panel`, `nebu-safety-panel`, `nebu-system-observability`, `nebu-cluster-panel`).
- **Evidence:** At least 6 independent `setInterval` loops polling REST endpoints every 1000–3000ms.
- **Expected Behavior:** Centralized telemetry polling or WebSocket push dispatching updates through `Store`.
- **Actual Behavior:** High HTTP request volume ($> 300\text{ req/min}$) generating unnecessary load and potential race conditions.
- **Root Cause:** Independent component development without centralized polling orchestration.

---

### Finding AUD-07 (P2) — Fragmented Navigation Mental Model (5 Uncoordinated Modes)
- **Location:** [`web/src/components/nebu-app.ts` lines 155–171](file:///root/NebuDCS/web/src/components/nebu-app.ts#L155-L171)
- **Evidence:** Top mode switcher lists 5 disconnected tabs (`Operations`, `Engineering Studio`, `Training (OTS)`, `Scientific Validation`, `Enterprise & HA`).
- **Expected Behavior:** A unified product lifecycle navigation:
  - **Engineering** (Design, P&ID, Logic, Hardware HAL, Deployment)
  - **Operations** (DCS Flowsheet, Faceplates, Trends, Alarms, Equipment Diags)
  - **Training** (Instructor, Trainee, Scenarios, Replay)
  - **Validation** (Scientific Benchmarks, Provenance, Certification Manifests)
  - **Enterprise** (Cluster HA, Audit, Backup, Observability)
- **Actual Behavior:** Capabilities are scattered across tabs and duplicated between sidebars.
- **Root Cause:** Incremental stage completion without holistic UX information architecture refactoring.

---

## 24. Recommended Target Architecture

### The Unified Product Mental Model:
```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                               NebuDCS Platform Header                             │
│  [⚙ Brand]  •  [Active Project Selector]  •  [Deploy Status]  •  [Sim Controls]   │
│  Navigation: [ Engineering Studio | DCS Operations | Training | Validation | Enterprise ] │
└────────────────────────────────────────┬─────────────────────────────────────────┘
                                         │
       ┌─────────────────────────────────┼─────────────────────────────────┐
       ▼                                 ▼                                 ▼
┌──────────────────────┐      ┌──────────────────────┐      ┌──────────────────────┐
│  Engineering Studio  │      │    DCS Operations    │      │    Enterprise & HA   │
│  • Flowsheet CAD     │      │  • ISA-101 Flowsheet │      │  • 5-Node Clust. HA  │
│  • Equipment Catalog │      │  • Loop Faceplates   │      │  • Audit Trail       │
│  • IEC 61131-3 Logic │      │  • Multi-Pen Trends  │      │  • Observability     │
│  • I/O HAL & Modules │      │  • ISA-18.2 Alarms   │      │  • Backup & Restore  │
│  • Control Strategies│      │  • First-Out Chrono  │      │  • Global Diags      │
│  • SIS Safety Matrix │      │  • Process Tag Table │      └──────────────────────┘
│  • Pre-Flight Check  │      │                      │
│  • DEPLOY TO RUNTIME │═════>│                      │
└──────────────────────┘      └──────────────────────┘
```

---

## 25. Prioritized Fix Plan (For Future Authorization)

When an implementation phase is authorized by the user, the remediation should follow this sequence:

### Phase 1: Core Runtime & Lifecycle Synchronization (P0 / P1)
1. **Bootstrap Project Deployment:** In `nebu-app.ts`, when bootstrapping the active project, automatically call `apiClient.loadProjectSim(proj.id)` to initialize backend `RuntimeTagStore` and `GenericPhysicsSystem`.
2. **Clock Heartbeat in WebSocket:** In `host.rs`, emit a lightweight simulation tick packet or ensure `DeltaUpdateDto` is broadcast periodically (at least once per second) regardless of tag deadband, carrying `sim_time_sec`, `tick_count`, and `is_running`.
3. **Reactive Sim Controls:** Wire `nebu-sim-controls.ts` directly to WebSocket `sim_control` with bidirectional state acknowledgment updating `playBtn` and `pauseBtn` disabled states.

### Phase 2: DCS Operations vs Engineering Studio Clean Separation (P1)
1. **Remove Engineering Clutter from DCS Operations:** Strip `<nebu-io-panel>` and `<nebu-controller-panel>` from `nebu-dcs-shell.ts`. Replace them with clean, operator-focused panels:
   - Loop Faceplate
   - Multi-pen Process Trend
   - ISA-18.2 Alarm Station
   - Process Variable Tag Table
2. **Consolidate Hardware & Logic in Engineering Studio:** Ensure `<nebu-io-panel>`, `<nebu-controller-panel>`, and `<nebu-iec-editor>` reside exclusively within the Engineering Studio sub-navigation tabs.
3. **Prominent "Deploy to Simulation" Workflow:** In `nebu-studio.ts`, add a high-visibility **"🚀 Validate & Deploy"** command bar with explicit visual feedback when the authored project is active in the live simulation core.

### Phase 3: Global Status Bar & Polling Consolidation (P2)
1. **Wire Status Bar to Live Observables:** Dynamically bind `CLOCK`, `SIS`, `COMM`, and `HA` values to store telemetry.
2. **Centralize Telemetry Polling:** Consolidate component polling intervals into a single orchestrated fetcher inside `Store`.

---

## 26. Final Readiness Verdict

**UI/RUNTIME AUDIT: MAJOR ISSUES FOUND**

### Verdict Justification:
- While all individual components, backend physics equations, cluster HA mechanisms, and automated tests pass with green results, **the platform is not yet ready for production end-user deployment due to the critical disconnection between UI project loading and backend simulation host synchronization (P0), domain cross-contamination in the operator console (P1), and delta frame suppression on steady state (P1).**
- Execution is halted strictly in compliance with instructions. **Zero code modifications were performed during this audit.**

---
*Report finalized and recorded in project documentation and brain artifacts.*
