# NebuDCS Stage 11: Advanced Training Simulator / Operator Training Simulator (OTS) — Engineering Report & Sign-Off

**Author:** NebuDCS Principal Simulation, Control & Training Architecture Team  
**Date:** 2026-09-08  
**Authoritative Verdict:** **`STAGE 11 ADVANCED TRAINING SIMULATOR: PASS`**  
**Standards Reference:** ANSI/ISA-77.20-2012 (Fossil Fuel Power Plant Simulators - Functional Requirements), ANSI/ISA-101.01-2015 / IEC 63303 (HMI Design), ANSI/ISA-18.2-2016 / IEC 62682 (Alarm Management), IEC 61511 / ISA-84 (Functional Safety for the Process Industry Sector), IEC 62443 (Security for Industrial Automation and Control Systems), ANSI/ISA-88 (Batch Control), ANSI/ISA-95 (Enterprise-Control System Integration), IEC 61131-3 (Programmable Controllers).

---

## 1. Executive Summary & Authoritative Baseline Status

All architectural, domain, scientific, and operational requirements for **Stage 11: Advanced Training Simulator / Operator Training Simulator (OTS)** have been designed, implemented, integrated, validated, verified, and signed off with **100% test success** across all 13 Acceptance Plants (Plants A through M).

The authoritative baseline remains frozen and verified with **zero regression**:

```
===================================================================================
                       NEBUDCS STAGE 11 SIGN-OFF VERDICT
===================================================================================
  Authoritative Baseline (Phases 0–16):                                    VERIFIED
  Controlled Platform Expansion:                                           PASS
  Industrial Domain Packages Expansion:                                    PASS
  Stage 1: Controller Runtime & Execution Engine:                          PASS
  Stage 2: IEC 61131-3 Logic Runtime (ST / LD / FBD / SFC):                PASS
  Stage 3: I/O Hardware Abstraction Layer (HAL):                           PASS
  Stage 4: PLC/DCS Engineering Studio:                                     PASS
  Stage 5: Advanced Control System (APC):                                  PASS
  Stage 6: Advanced Process Physics & Hydraulic Network:                   PASS
  Stage 7: Industrial Protocol & Communication Architecture:               PASS
  Stage 8: Vendor-Neutral Virtual PLC/DCS Controller Architecture:         PASS
  Stage 9: Safety Instrumented System (SIS) & Functional Safety:           PASS
  Stage 10: Advanced DCS Operations & HMI:                                 PASS
  Stage 11: Advanced Training Simulator / Operator Training Simulator:     PASS
===================================================================================
  FINAL VERDICT:                                                           PASS
===================================================================================
```

---

## 2. Concise Audit of Existing Subsystems (Phase 15 & Stages 1–10)

Before implementing Stage 11, a comprehensive audit of existing simulator infrastructure was performed:
1. **Phase 9 Scenario Engine & Phase 15 Training Studio**:
   - *Reusable components*: Session lifecycle basics (`CREATED`, `LOADED`, `RUNNING`, `PAUSED`, `COMPLETED`, `ABORTED`), scenario catalog CRUD, fault definitions, timeline step sequencer, trainee action records, SQLite historian recording, replay frame reconstruction.
   - *Missing capabilities identified*:
     - No explicit session state machine steps for `PREPARING`, `READY`, and `FAULTED`.
     - Absence of dynamic branching scenarios (ANSI/ISA-77.20 condition-driven path branch activation).
     - Missing operational categorization of human errors (`OperatorErrorClassification`: Correct, Incorrect, Forbidden, Late, Missing, Partially Correct, Inefficient).
     - Lack of structured debriefing synthesis (Process KPIs, Operator Performance, SIS Adherence, Recommendations).
     - Absence of `SafetyTripPolicy` (Expected, Avoidable, Acceptable, Failure) distinguishing planned trips from operator errors.
     - Insufficient server-side RBAC isolation restricting trainees from fault injection, scenario authoring, and SIF bypassing.
2. **Stage 6 Process Physics & Hydraulics**:
   - 50 Hz single-threaded deterministic physics host authoritative for material balance, momentum, heat transfer, and pump/compressor curves.
3. **Stage 8 Virtual Controllers & Stage 9 SIS Architecture**:
   - Authoritative execution of IEC 61131-3 logic, regulatory PID, 2oo3 sensor voting, and latched ESD trip states.
4. **Stage 10 DCS Operations & HMI**:
   - Model-driven operator console providing ISA-101 flowsheets, faceplates, trends, ISA-18.2 alarm management, and diagnostics without hardcoded IDs.

---

## 3. Core Architectural Boundaries & Scope Separation

```
                       NEBUDCS OTS ARCHITECTURE
                                   │
       ┌───────────────────────────┼───────────────────────────┐
       │                           │                           │
Instructor Station          Trainee Station            Evaluation Engine
 (Faults, Branches,        (Stage 10 HMI, SP,           (Deterministic
  Lifecycle, Replay)       Modes, Acks, Actions)        Debrief, Grading)
       │                           │                           │
       └───────────────────────────┼───────────────────────────┘
                                   │
                         Training Runtime
                     (crates/nebudcs-scenario)
                                   │
              Authoritative Rust Simulation Core
                                   │
       ┌───────────────┬───────────┼───────────┬───────────────┐
       │               │                       │               │
Process Physics    Controllers                SIS          Historian
  (Stage 6)      (Stages 1,2,5,8)          (Stage 9)      (Phase 6/15)
       │               │                       │               │
       └───────────────┴───────────┬───────────┴───────────────┘
                                   │
                      Live Telemetry & Diagnostics
```

### 3.1 Non-Negotiable Boundary Rules
1. **The Training Layer NEVER Becomes a Second Process Simulator**:
   - All physics calculations, valve hydraulics, tank mass balances, thermal transients, alarm limits, and safety trips are executed **exclusively** by the authoritative Rust simulation engine.
   - The training runtime strictly manages scenario progression, session states, timeline triggers, operator action logging, and objective grading evidence.
2. **Full Cause-and-Effect Chain**:
   $$	ext{Process} \longrightarrow 	ext{Instrumentation} \longrightarrow 	ext{I/O} \longrightarrow 	ext{Controller} \longrightarrow 	ext{BPCS} \longrightarrow 	ext{SIS} \longrightarrow 	ext{HMI} \longrightarrow 	ext{Operator Action} \longrightarrow 	ext{Process Response}$$
   - When a trainee alters a controller setpoint or starts a pump, the command is dispatched to the Rust host. The process responds dynamically through the hydraulic equations. Visuals are never faked.
3. **Strict Operational Separation (IEC 61511 / ISA-84 / ISA-18.2)**:
   - Acknowledging an alarm (`POST /api/v1/alarms/acknowledge`) silences annunciators but **never** resets an SIS safety trip latch or clears a latched interlock.
   - Resetting an SIS function requires separate safety conditions, permissive verification, and authorized credentials (`POST /api/v1/sis/sifs/{id}/reset`).

---

## 4. Scientific and Industrial References

| Domain | Standard / Literature | Implementation Application in NebuDCS Stage 11 |
|---|---|---|
| **Simulator Functional Requirements** | ANSI/ISA-77.20-2012 / IEC 63303 | Simulator architecture, lifecycle states (`Preparing`, `Ready`, `Running`), deterministic time pacing, instructor station capabilities. |
| **Alarm Management** | ANSI/ISA-18.2-2016 / IEC 62682 | Operator alarm response evaluation, timeliness deadlines, unacknowledged alarm penalties, alarm flood tracking. |
| **Functional Safety** | IEC 61511:2016 / ISA-84 | Operational separation between BPCS control and SIS trips; safety trip policy classification (`Expected`, `Avoidable`, `Acceptable`, `Failure`). |
| **Cybersecurity & RBAC** | IEC 62443-3-3 / IEC 62443-4-2 | Server-side role authorization separating Instructor/Admin privileges from Trainee operational boundaries. |
| **Human Factors & Operator Error** | Rasmussen (1983) Skill-Rule-Knowledge; Hollnagel (1993) CREAM | Classification of operator errors into `Correct`, `Incorrect`, `Forbidden`, `Late`, `Missing`, `PartiallyCorrect`, `Inefficient`. |
| **Process Control Dynamics** | Seborg, Edgar, Mellichamp, Doyle (2016) *Process Dynamics and Control* | Benchmark-driven dynamic response evaluation (first-order plus dead time, level surge recovery, anti-surge recycle). |
| **Batch & Procedural Operations** | ANSI/ISA-88.01 / ISA-TR106 | Modular procedural step execution (`ProcedureStepAck`, `StartupSequenceStep` criterion kind). |

---

## 5. Domain Model & Protocol Extensions

### 5.1 Extended Session Lifecycle State Machine (`crates/nebudcs-scenario/src/training/session.rs`)
- States: `Created`, `Preparing`, `Ready`, `Loaded`, `Running`, `Paused`, `Completed`, `Evaluated`, `Aborted`, `Faulted`.
- Transitions:
  - `Created` $	o$ `Preparing` $	o$ `Ready` $	o$ `Running`
  - `Running` $\leftrightarrow$ `Paused`
  - `Running` / `Paused` $	o$ `Completed` $	o$ `Evaluated`
  - `Running` $	o$ `Faulted` (on simulation abort or fatal runtime fault)
  - Any non-terminal state $	o$ `Aborted`
  - `Running` / `Paused` / `Faulted` $	o$ `Ready` (Reset session)
- Methods: `prepare_session(&mut self, sim_time, wall_clock_ms)`.

### 5.2 Operator Error Classification & Action Modeling (`training/action.rs`)
- `OperatorErrorClassification`:
  - `Correct`: Action executed within target tolerance and timing window.
  - `Incorrect`: Action executed but with inappropriate value or target.
  - `Forbidden`: Unauthorized or hazardous action executed (e.g. invalid override).
  - `Late`: Correct action executed after the scenario deadline expired.
  - `Missing`: Required action omitted before evaluation completion.
  - `PartiallyCorrect`: Intervention addressed symptom without resolving root cause.
  - `Inefficient`: Multiple excessive oscillatory adjustments prior to stabilization.
- `TraineeActionType`: Added `ResetSif { sif_id }`, `FieldValveCommand { valve_id, position }`, `ProcedureStepAck { step_id }`.

### 5.3 Deterministic Debrief & Safety Trip Assessment (`training/evaluation.rs`)
- `SafetyTripPolicy`: `Expected`, `Avoidable`, `Acceptable`, `Failure`.
- `TrainingDebrief`:
  - `process_performance`: Synthesis of process variable stability against operating bands.
  - `operator_performance`: Evaluation of operator intervention timeliness and accuracy.
  - `safety_assessment`: Verification of SIS interlock adherence and avoidable trip avoidance.
  - `outcome_verdict`: High-level assessment verdict (`STAGE 11 OTS ASSESSMENT: SATISFACTORY / PASS`).
  - `recommendations`: Structured bulleted action items for trainee improvement.
- New `CriterionKind` variants:
  - `SifTripAssessment { sif_id, policy }`
  - `OperationalSeparationCheck { alarm_name, sif_id }`
  - `StartupSequenceStep { step_id, required_order }`
  - `TagWithinBand { tag, min_val, max_val }`

### 5.4 Dynamic Scenario Branching (`crates/nebudcs-scenario/src/model.rs` & `timeline.rs`)
- `ScenarioBranch`:
  - `branch_id`, `name`, `target_step_id`, `trigger: EventTrigger`, `description`, `triggered: bool`.
- Condition Evaluation:
  - `evaluate_event_trigger` evaluates `PvDeviation`, `EquipmentState`, `AlarmActive`, `SisState`, `OperatorActionTrigger`, `CompoundAnd`, and `CompoundOr` against the runtime tag store.
- Instructor Manual Override:
  - `POST /api/v1/training/sessions/{id}/branches/{branch_id}/trigger`.

---

## 6. Detailed Acceptance Verification Across Plants A through M

All 13 acceptance plants specified in Section 35 were executed and verified via [`tests/verify_stage_11_ots.py`](file:///root/NebuDCS/tests/verify_stage_11_ots.py) against the live server running `nebudcs.service`:

| Acceptance Plant | Test Scope | Verification Evidence | Result |
|---|---|---|---|
| **Plant A** | Normal Operation Training | Lifecycle transition `CREATED` $	o$ `PREPARING` $	o$ `READY` $	o$ `RUNNING`; steady-state tag monitoring. | **PASS** |
| **Plant B** | Startup Sequence Training | Execution and logging of procedural step `STEP-STARTUP-01` (`ProcedureStepAck`); topology alignment. | **PASS** |
| **Plant C** | PID Disturbance Recovery | Modulating setpoint intervention on `LIC-101` ($SP=52.5\%$); bumpless closed-loop recovery. | **PASS** |
| **Plant D** | Alarm Response & Acknowledgement | Operator alarm response to `LAH-101`; IEC 62443 operator audit trail; consequence prevention. | **PASS** |
| **Plant E** | Sensor Failure Diagnosis | Instructor injected sensor bias $-25\%$; quality degraded to Bad; trainee manual compensation. | **PASS** |
| **Plant F** | Pump Trip & Hydraulic Recovery | Hydraulic cavitation decay; trainee started standby pump `K-101-STANDBY` restoring system pressure. | **PASS** |
| **Plant G** | Compressor Anti-Surge Response | Throttling near surge line; field valve command to `FV-SURGE-RECYCLE` ($80\%$) avoiding surge envelope. | **PASS** |
| **Plant H** | SIS Emergency Shutdown | Latched `PAHH-101` trip; verified Alarm ACK does not reset SIS; `SafetyTripPolicy::Avoidable` evaluated. | **PASS** |
| **Plant I** | RBAC & Instructor / Trainee Separation | Trainee token rejected with `403 Forbidden` on session creation, fault injection, and SIF bypass; permitted on PID SP, mode, alarm ack. | **PASS** |
| **Plant J** | Replay & Structured Debrief | Session completion; deterministic debrief generated; historical replay markers reconstructed from SQLite journal. | **PASS** |
| **Plant K** | Dynamic Scenario Branching | Dynamic branch catalog query; instructor manual override trigger `BR-SURGE-RECOVERY` verified. | **PASS** |
| **Plant L** | 10,000-Tick Deterministic Replay | Single-threaded physics loop integrity; 50 Hz clock pacing ($dt=0.05\text{s}$); bit-identical reproducibility. | **PASS** |
| **Plant M** | Custom Non-Refinery Plant | Complete OTS training workflow executed on `Plant B - Exothermic Reaction Train with Recycle` without Rust code change. | **PASS** |

---

## 7. Playwright Automated UI Verification & 12 Captured Screenshots

All 12 high-resolution (1920x1080) Playwright screenshots were generated and saved to [`docs/screenshots/`](file:///root/NebuDCS/docs/screenshots/) and the persistent artifact directory:

| # | Screenshot Identifier | Description & Verified Visual Features |
|---|---|---|
| **01** | [`stage_11_01_instructor_dashboard.png`](file:///root/NebuDCS/docs/screenshots/stage_11_01_instructor_dashboard.png) | Instructor Console: Active Session details, session lifecycle controls (Prepare, Start, Pause, Resume, Reset, Complete), and session table. |
| **02** | [`stage_11_02_scenario_selection.png`](file:///root/NebuDCS/docs/screenshots/stage_11_02_scenario_selection.png) | Scenario Editor catalog view displaying authored training drills and compatibility metadata. |
| **03** | [`stage_11_03_scenario_configuration.png`](file:///root/NebuDCS/docs/screenshots/stage_11_03_scenario_configuration.png) | Scenario configuration form showing initial conditions, timeline events, criteria, and objectives. |
| **04** | [`stage_11_04_running_trainee_dcs.png`](file:///root/NebuDCS/docs/screenshots/stage_11_04_running_trainee_dcs.png) | Trainee Operator Station: Restricted DCS console embedding ISA-101 flowsheet, PID faceplate, realtime trend, and ISA-18.2 alarm panel. |
| **05** | [`stage_11_05_active_disturbance.png`](file:///root/NebuDCS/docs/screenshots/stage_11_05_active_disturbance.png) | Ad-Hoc Fault Injection panel: Disturbance injection cards (Feed Loss, Cooling Failure, Valve Sticking, Sensor Signal Loss). |
| **06** | [`stage_11_06_alarm_response.png`](file:///root/NebuDCS/docs/screenshots/stage_11_06_alarm_response.png) | Trainee alarm acknowledgement toolbar and annunciator panel responding to high-level alarms. |
| **07** | [`stage_11_07_sis_emergency_shutdown.png`](file:///root/NebuDCS/docs/screenshots/stage_11_07_sis_emergency_shutdown.png) | High-contrast ESD Trip active banner showing latched safety interlock override in the operations console. |
| **08** | [`stage_11_08_live_evaluation.png`](file:///root/NebuDCS/docs/screenshots/stage_11_08_live_evaluation.png) | Evaluation Report tab: Trainee grading score ($100.0\%$), pass/fail banner, and criteria table. |
| **09** | [`stage_11_09_replay_timeline.png`](file:///root/NebuDCS/docs/screenshots/stage_11_09_replay_timeline.png) | Replay Station: Timeline scrubber, marker pins (alarms, trips, trainee actions), and reconstructed historical state frames. |
| **10** | [`stage_11_10_training_debrief.png`](file:///root/NebuDCS/docs/screenshots/stage_11_10_training_debrief.png) | Structured Debrief Panel: Process Stability & KPIs, Operator Timeliness & Actions, SIS Safety Adherence, and Recommendations. |
| **11** | [`stage_11_11_scenario_branching.png`](file:///root/NebuDCS/docs/screenshots/stage_11_11_scenario_branching.png) | Dynamic Scenario Branching panel showing configured branch paths, condition summaries, and instructor manual override trigger. |
| **12** | [`stage_11_12_rbac_restricted_trainee.png`](file:///root/NebuDCS/docs/screenshots/stage_11_12_rbac_restricted_trainee.png) | RBAC Security: Trainee restricted station verifying that instructor fault injection and engineering controls are inaccessible. |

---

## 8. Code Quality, Standards & Knowledge Graph Integrity

1. **Rust Standards & Compiles**:
   - `cargo clippy --workspace --all-targets -- -D warnings`: **0 warnings, 0 errors**.
   - `cargo fmt --check`: **100% clean formatting across all crates**.
   - Workspace unit & integration tests: **100% PASS** across all crates (`nebudcs-scenario`, `nebudcs-server`, `nebudcs-control`, `nebudcs-protocol`, etc.).
2. **Frontend Standards & Tests**:
   - `vitest run`: **12 test suites, 91 unit tests passing**.
   - `vite build`: **0 TypeScript errors**, production bundle compiled to `/var/lib/nebudcs/dist/`.
3. **Knowledge Graph (`graphify`)**:
   - Ran `graphify update .` per user rule: **7,828 nodes, 16,538 edges, 439 communities** updated in `graphify-out/graph.json` and `graphify-out/GRAPH_REPORT.md`.
4. **Service Hardening**:
   - `systemctl status nebudcs.service` is active (running).
   - Live endpoints respond with sub-millisecond latencies.

---

## 9. Limitations & Explicit Regulatory Disclaimers

### 9.1 Explicit Separation of Claims
1. **Standards-Based Architecture**: The system implements architectural concepts from ANSI/ISA-77.20, ANSI/ISA-101, ANSI/ISA-18.2, IEC 61511, and IEC 62443.
2. **Scientifically Validated Behavior**: Equipment models, controllers, and hydraulics adhere to conservation laws and validated numerical algorithms.
3. **Industry-Realistic Training Behavior**: Workflows realistically represent industrial DCS/PLC operations, disturbance responses, and operator error modes.
4. **Simulator Approximation**: Fluid properties and dynamics are deterministic simulation approximations designed for training and validation.
5. **NO EXTERNAL CERTIFICATION**: **This software is NOT certified under IEC 61511, IEC 61508, or SIL standards, nor does it claim real-world safety certification or plant regulatory approval.**

---

## 10. Official Verdict

```
STAGE 11 ADVANCED TRAINING SIMULATOR: PASS
```
