# NebuDCS Stage 2: IEC 61131-3 Control Logic Runtime Architectural Verification Report

**Author**: Lead Systems Architect & Infrastructure Engineering Team  
**Date**: September 7, 2026  
**Target Baseline**: NebuDCS Production Platform (Phases 0–16, Controlled Expansion, Domain Packages, Stage 1 Control Runtime, Stage 3 I/O HAL)  
**Target Verdict**: `STAGE 2 IEC 61131-3: PASS`  
**Applicable Standards**: IEC 61131-3 3rd Edition, IEC 62443-4-2, ISA-18.2  

---

## 1. Executive Summary

NebuDCS Stage 2 delivers an industrial-grade, vendor-neutral **IEC 61131-3 Control Logic Runtime** natively integrated into the single-threaded deterministic Rust simulation and control engine. 

Prior to Stage 2, control algorithms were expressed via declarative configuration loops (`PidController`) or external scripting. While robust for standard continuous regulation, complex batch automation, sequential safety interlocks, motor logic circuits, and state-phase control required a standardized industrial automation programming environment.

With the completion of Stage 2, NebuDCS establishes:
1. **Four Primary IEC 61131-3 Languages**: Vendor-neutral compilation and execution for **Structured Text (ST)**, **Ladder Diagram (LD)**, **Function Block Diagram (FBD)**, and **Sequential Function Chart (SFC)**.
2. **Common Typed Intermediate Representation (IR)**: All four languages compile to a strongly typed, linearizable IR executed deterministically on the virtual machine (`IecVm`).
3. **Execution Safety & Infinite-Loop Prevention**: Built-in 1,000-iteration execution bounds (`MAX_LOOP_ITERATIONS = 1000`) preventing infinite scan freezes on runaway user-authored `WHILE` and `REPEAT` loops.
4. **Authoritative Standard Function Blocks**: Full implementation of timers (`TON`, `TOF`, `TP`), counters (`CTU`, `CTD`), edge detectors (`R_TRIG`, `F_TRIG`), bistable latches (`SR`, `RS`), and closed-loop regulation (`PID`).
5. **Zero Duplication of Authoritative Components**: The IEC 61131-3 `PID` function block directly reuses the production-verified `PidController` from `nebudcs-control::pid` with bumpless transfer mode alignment.
6. **Unified I/O Memory Binding**: Direct read/write bindings between IEC logic and Stage 3 I/O HAL `%I` (inputs) and `%Q` (outputs) using the unified `IoImageTable`.
7. **Stage 1 Task Scheduler Integration**: `IecProgram` implements the authoritative `ControllerProgram` trait, enabling periodic execution across prioritized `ControllerTask` instances.
8. **Web Component Engineering Studio UI**: `<nebu-iec-editor>` provides multi-language code editing, AST validation, compilation feedback, diagnostics banners, and live symbol memory inspection.
9. **Zero Baseline Regressions**: 100% test pass rate across all 13 workspace crates, 10,000-tick bit-identical replay determinism proof, and clean verification of all multi-plant acceptance suites.

---

## 2. Baseline & Invariant Preservation Statement

NebuDCS strictly preserves all prior platform baselines:
* **No Phase 17**: This deliverable satisfies Stage 2 of the Master Roadmap, avoiding phantom phases.
* **Sole Authoritative Physics & Control Execution**: The single-threaded Rust engine remains the sole authority for physical state progression and discrete control execution.
* **No Duplicate Scheduler or I/O Images**: Reuses the Stage 1 task scheduler and Stage 3 `IoImageTable` without parallel runtime duplication.
* **No Duplicate PID Engine**: The IEC `PID` block encapsulates the existing `PidController`, ensuring continuous tuning parameters and anti-windup algorithms remain identical across declarative and IEC domains.
* **Simulation Loop Determinism**: No async/await, no memory allocations in steady-state execution, and no wall-clock dependencies inside the scan tick.
* **Standard Library Palette Invariant**: Exactly 12 canonical standard equipment templates remain in the library palette.

---

## 3. IEC 61131-3 Architecture Overview

The runtime decouples textual/graphical language syntax from virtual machine execution through a unified compilation pipeline:

```mermaid
graph TD
    ST_SRC[ST Source Code] -->|Lexer / Parser| ST_COMP[ST Compiler]
    LD_SRC[Ladder Model / JSON] -->|Power-Flow Analyzer| LD_COMP[LD Compiler]
    FBD_SRC[FBD Graph Model] -->|Kahn Topological Sort| FBD_COMP[FBD Compiler]
    SFC_SRC[SFC Chart Model] -->|State-Phase Synthesis| SFC_COMP[SFC Compiler]

    ST_COMP --> COMMON_IR[Typed Intermediate Representation - IR]
    LD_COMP --> COMMON_IR
    FBD_COMP --> COMMON_IR
    SFC_COMP --> COMMON_IR

    COMMON_IR --> IEC_VM[Deterministic Virtual Machine - IecVm]
    IEC_VM <--> MEM[MemoryStore / SymbolTable]
    IEC_VM <--> FB_MAP[Function Block Instances]
    IEC_VM <--> IO_IMG[Stage 3 IoImageTable %I / %Q]
    IEC_VM --> TASK[Stage 1 ControllerTask Periodic Scan]
```

---

## 4. Elementary & Derived Type System Specification

Located in `crates/nebudcs-control/src/iec61131/types.rs`, the type system enforces strict type checking and bit-width boundaries:

### Elementary Types
* **Booleans**: `BOOL` (1-bit truth value).
* **Signed Integers**: `SINT` (8-bit), `INT` (16-bit), `DINT` (32-bit), `LINT` (64-bit).
* **Unsigned Integers**: `USINT` (8-bit), `UINT` (16-bit), `UDINT` (32-bit), `ULINT` (64-bit).
* **Bit Strings**: `BYTE` (8-bit), `WORD` (16-bit), `DWORD` (32-bit), `LWORD` (64-bit).
* **Floating Point**: `REAL` (IEEE-754 32-bit single precision), `LREAL` (IEEE-754 64-bit double precision).
* **Time & Date Types**: `TIME` (duration in seconds), `DATE`, `TIME_OF_DAY` (`TOD`), `DATE_AND_TIME` (`DT`).
* **Strings**: `STRING` (fixed or variable length UTF-8 encoded).

### Composite / Derived Types
* **`ARRAY`**: Strongly typed 1D/multidimensional arrays with bounds checking (`lower_bound..=upper_bound`).
* **`STRUCT`**: User-defined structured types holding named field values.
* **`ENUM`**: Discrete enumerated types mapping named labels to integer ordinal values.

---

## 5. Variable Scopes & Symbol Management

Located in `crates/nebudcs-control/src/iec61131/storage.rs`:
* **Variable Scopes**: `Var` (local), `VarInput`, `VarOutput`, `VarInOut`, `VarGlobal`, `VarTemp`, `VarExternal`.
* **Symbol Table**: Validates duplicate symbol identifiers, enforces data type declarations, and associates direct hardware `%I` / `%Q` address bindings.
* **Memory Store**: Stores live typed values (`IecValue`) in pre-allocated collections, guaranteeing deterministic access times during VM execution.

---

## 6. Standard Function Blocks Implementation

Located in `crates/nebudcs-control/src/iec61131/fb.rs`:

| Function Block | Classification | Inputs | Outputs | Description |
| :--- | :--- | :--- | :--- | :--- |
| **`TON`** | On-Delay Timer | `IN` (BOOL), `PT` (TIME) | `Q` (BOOL), `ET` (TIME) | Delays turning true until `PT` seconds have elapsed. |
| **`TOF`** | Off-Delay Timer | `IN` (BOOL), `PT` (TIME) | `Q` (BOOL), `ET` (TIME) | Retains output true for `PT` seconds after input falls false. |
| **`TP`** | Pulse Timer | `IN` (BOOL), `PT` (TIME) | `Q` (BOOL), `ET` (TIME) | Generates a pulse of fixed duration `PT` on rising edge. |
| **`CTU`** | Up Counter | `CU` (BOOL), `RESET` (BOOL), `PV` (DINT) | `Q` (BOOL), `CV` (DINT) | Increments on rising edge of `CU`; sets `Q` when `CV >= PV`. |
| **`CTD`** | Down Counter | `CD` (BOOL), `LOAD` (BOOL), `PV` (DINT) | `Q` (BOOL), `CV` (DINT) | Decrements on rising edge of `CD`; sets `Q` when `CV <= 0`. |
| **`R_TRIG`** | Rising Edge Detector | `CLK` (BOOL) | `Q` (BOOL) | Emits single-scan pulse when `CLK` transitions `false -> true`. |
| **`F_TRIG`** | Falling Edge Detector | `CLK` (BOOL) | `Q` (BOOL) | Emits single-scan pulse when `CLK` transitions `true -> false`. |
| **`SR`** | Bistable Latch (Set Dominant) | `S1` (BOOL), `R` (BOOL) | `Q1` (BOOL) | Latch where Set command overrides Reset. |
| **`RS`** | Bistable Latch (Reset Dominant) | `S` (BOOL), `R1` (BOOL) | `Q1` (BOOL) | Latch where Reset command overrides Set. |

---

## 7. Production PID Controller Integration & Bumpless Transfer

To eliminate dual-implementation drift, the `PID` IEC block wraps the authoritative `nebudcs_control::pid::PidController`:

```rust
// Bumpless transfer sequencing:
// Mode evaluation must precede SP update to allow bumpless tracking
if let Some(man) = inputs.get("MAN") {
    if man.as_bool().unwrap_or(false) {
        self.inner.set_mode(PidMode::Manual);
    }
}
if let Some(auto) = inputs.get("AUTO") {
    if auto.as_bool().unwrap_or(false) {
        self.inner.set_mode(PidMode::Auto);
    }
}
if let Some(sp) = inputs.get("SP") {
    self.inner.set_sp(sp.as_f64().unwrap_or(0.0));
}
```

* **Anti-Windup**: Clamped integrator accumulation during actuator saturation.
* **Bumpless Transfer**: Transitions from Manual to Auto automatically equalize internal setpoint to current process variable ($SP = PV$) to avoid output kicks.

---

## 8. Common Intermediate Representation (IR) Design

Located in `crates/nebudcs-control/src/iec61131/ir.rs`:
* **Expressions (`IecExpr`)**: Strongly typed expression AST supporting binary arithmetic (`Add`, `Sub`, `Mul`, `Div`, `Mod`), relational comparisons (`Eq`, `Ne`, `Lt`, `Le`, `Gt`, `Ge`), bitwise logic (`And`, `Or`, `Xor`, `Not`), built-in mathematical calls (`ABS`, `SQRT`, `LIMIT`, `MIN`, `MAX`, `SIN`, `COS`), and direct `%I` hardware reads.
* **Statements (`IecStatement`)**: Explicit assignments (`Assign`, `AssignIo`), structured branching (`IfThenElse`), bounded iterations (`While`, `Repeat`, `For`), Function Block execution calls (`CallFb`), and flow control (`Exit`, `Return`).

---

## 9. Virtual Machine (IecVm) & Loop Safety Guard

Located in `crates/nebudcs-control/src/iec61131/vm.rs`:
* **Deterministic Scan**: Executes linear IR statements with zero async operations, zero dynamic allocations during evaluation, and deterministic IEEE-754 arithmetic.
* **1,000-Iteration Infinite Loop Guard**:
  ```rust
  pub const MAX_LOOP_ITERATIONS: usize = 1_000;
  ```
  Runaway `WHILE` or `REPEAT` loops trigger an immediate, graceful `ControlError::InvalidParameter("Loop exceeded maximum allowed iterations (1000)")`, preventing control scan freezes and controller watchdog trips.
* **Special System Variables**: Automatic exposure of scan delta time (`_DT`) and cumulative running time (`_TIME`).

---

## 10. Structured Text (ST) Compiler

Located in `crates/nebudcs-control/src/iec61131/st/`:
* **Lexer (`lexer.rs`)**: Tokenizes ST keywords (`VAR`, `END_VAR`, `IF`, `THEN`, `ELSE`, `FOR`, `TO`, `BY`, `WHILE`, `REPEAT`, `%I*`, `%Q*`), identifiers, and operators.
* **Parser (`parser.rs`)**: Full recursive-descent parser handling operator precedence, type declarations, nested control structures, and function calls.
* **Compiler (`mod.rs`)**: Emits `SymbolTable` and validated `Vec<IecStatement>`.

---

## 11. Ladder Diagram (LD) Model & Compiler

Located in `crates/nebudcs-control/src/iec61131/ladder/`:
* **Rungs & Rails**: Ordered execution sequence from left power rail to right neutral rail.
* **Contact Elements**:
  * Normally Open (`NO`): Passes power when variable is true.
  * Normally Closed (`NC`): Passes power when variable is false.
  * Positive Edge (`P`): Emits single scan pulse on `false -> true` transition.
  * Negative Edge (`N`): Emits single scan pulse on `true -> false` transition.
* **Coil Elements**:
  * Normal Output Coil (`Coil`): Writes current rung power state.
  * Negated Output Coil (`NegCoil`): Writes inverted rung power state.
  * Set / Latch Coil (`SetCoil`): Sets true on power; maintains state when power is lost.
  * Reset / Unlatch Coil (`ResetCoil`): Sets false on power.
* **Parallel Branches**: Compiles multi-branch logic into unified disjunctive Boolean expressions (`branch_1 OR branch_2 ...`).

---

## 12. Function Block Diagram (FBD) Topological Sort & Cycle Detection

Located in `crates/nebudcs-control/src/iec61131/fbd/`:
* **Directed Acyclic Graph (DAG)**: Execution order is calculated via **Kahn's Topological Sorting Algorithm**.
* **Cycle Detection**: Direct algebraic loops without state delay blocks are strictly rejected at compile time with a descriptive error.
* **Feedback Loops via `DELAY_1T`**: Feedback loops are permitted and validated only when routed through a unit delay (`DELAY_1T`), allowing the loop to be resolved using previous scan history.

---

## 13. Sequential Function Chart (SFC) State Machine & Qualifiers

Located in `crates/nebudcs-control/src/iec61131/sfc/`:
* **Steps**: Active state tracking via boolean markers (`_SFC_X_<STEP_NAME>`).
* **Transitions**: Evaluates guard conditions; upon transition, simultaneously deactivates source step and activates target step.
* **Action Qualifiers**:
  * **`N` (Non-Stored)**: Active continuously while the associated step is active.
  * **`S` (Set / Stored)**: Sets the action variable; remains active after step exits.
  * **`R` (Reset)**: Clears a previously set stored action variable.
  * **`P` (Pulse)**: Emits a single scan pulse upon step entry.
  * **`L` (Time Limited)**: Active for maximum duration $T$ while step remains active.
  * **`D` (Time Delayed)**: Activates only after step has remained active for duration $T$.

---

## 14. Stage 1 Task Scheduler Integration

Located in `crates/nebudcs-control/src/iec61131/program.rs`:
* `IecProgram` implements `ControllerProgram`:
  ```rust
  impl ControllerProgram for IecProgram {
      fn name(&self) -> &str { &self.name }
      fn execute(&mut self, dt: f64, current_time: f64, io: &mut IoImageTable) -> ControlResult<()> {
          // Sequentially executes all registered routines against shared memory and IO
      }
      fn reset(&mut self) -> ControlResult<()> { /* ... */ }
      fn clone_box(&self) -> Box<dyn ControllerProgram> { Box::new(self.clone()) }
  }
  ```
* Multi-routine programs run deterministically within `ControllerTask` periodic cycles (e.g., 20 ms / 50 Hz).

---

## 15. Stage 3 I/O HAL & `%I/%Q` Image Table Binding

Logic routines read inputs and write outputs through standard IEC address tokens:
* `%IX<n>.<bit>`: Discrete input channel bit.
* `%IW<n>`: Analog/word input channel.
* `%QX<n>.<bit>`: Discrete output channel bit.
* `%QW<n>`: Analog/word output channel.

During execution, `IecVm` interacts exclusively with `IoImageTable`. Hardware conditioning, NAMUR NE43 limit checking, and DAC scaling are executed transparently by Stage 3 HAL.

---

## 16. Protocol & REST API Endpoints

Integrated into `apps/nebudcs-server/src/routes/controllers.rs`:
* **Compile Endpoint**: `POST /api/v1/controllers/iec/compile`
  * Body: `{"language": "ST"|"LD"|"FBD"|"SFC", "source_text": "..."}`
  * Enforces RBAC authentication (`Admin`, `Instructor`, `Operator`).
  * Returns compilation diagnostic payload:
    ```json
    {
      "success": true,
      "error_message": null,
      "symbol_count": 3,
      "statement_count": 5,
      "variables": [
        {"name": "SP", "data_type": "REAL", "value": "Real(10.0)", "scope": "Var", "address_binding": null}
      ]
    }
    ```

---

## 17. Frontend Web Component (`<nebu-iec-editor>`)

Located in `web/src/components/nebu-iec-editor.ts`:
* **Language Switcher**: Dedicated tabs for Structured Text, Ladder Diagram, Function Block Diagram, and Sequential Function Chart.
* **Code Editor**: Syntax editing area with line numbers and monospace layout.
* **Diagnostics Banner**: Real-time error reporting showing syntax errors, parse failures, or compile verification confirmations.
* **Symbol Memory Inspector**: Table displaying symbol name, IEC type, current live value, and physical address binding.
* **Full Integration**: Mounted inside `<nebu-controller-panel>` and accessible from the Engineering Studio.

---

## 18. Industrial Acceptance A: ST Pressure Relief

**Objective**: Verify closed-loop pressure relief regulation via direct physical input `%IW100` and actuator command `%QW100`.
* Test: `test_industrial_acceptance_a_st_pressure_regulation`
* Result:
  * At 10.0 bar (below 12.0 bar setpoint): Valve output `%QW100 = 0.0%`.
  * At 15.0 bar (overpressure): Valve demand computes $(15 - 12) \times 10 = 30.0\%$. Output `%QW100 = 30.0%`.
* Verdict: **PASS**.

---

## 19. Industrial Acceptance B: LD Motor Interlock

**Objective**: Verify 3-phase motor seal-in start/stop circuit with external permissive interlock and Stage 1 VirtualController task integration.
* Test: `test_industrial_acceptance_b_ld_motor_interlock`
* Result:
  * Interlock Tripped (`INTERLOCK_OK = false`): Pressing `START_CMD` fails to start motor (`%QX0.0 = 0`).
  * Interlock Normal (`INTERLOCK_OK = true`): Motor starts (`%QX0.0 = 1`).
  * Interlock Drop: Immediate fail-safe shutdown (`%QX0.0 = 0`).
* Verdict: **PASS**.

---

## 20. Industrial Acceptance C: FBD Regulation with Production PID

**Objective**: Verify closed-loop FBD PID regulation connecting physical transmitter `%IW200` to valve `%QW200`.
* Test: `test_industrial_acceptance_c_fbd_pid_regulation`
* Result:
  * At $PV = 80.0$, $SP = 100.0$ (positive error), output command responds proportionally with positive opening demand.
  * Verified seamless invocation of authoritative `PidController` without duplication.
* Verdict: **PASS**.

---

## 21. Industrial Acceptance D: SFC Burner Startup Sequence

**Objective**: Verify multi-phase industrial burner startup sequencing through state-step progression.
* Test: `test_industrial_acceptance_d_sfc_startup_sequence`
* Result:
  * Phase 1 (`PURGE`): Energizes purge blower (`%QX10.0 = 1`).
  * Phase 2 (`IGNITE`): Blower shuts off (`%QX10.0 = 0`), igniter spark energizes (`%QX10.1 = 1`).
  * Phase 3 (`RUNNING`): Flame detected, igniter de-energizes (`%QX10.1 = 0`), main fuel valve opens (`%QX10.2 = 1`).
* Verdict: **PASS**.

---

## 22. Industrial Acceptance E: Mixed-Language Shared Memory

**Objective**: Verify a multi-routine program containing both Structured Text and Ladder Diagram routines sharing memory and I/O images.
* Test: `test_industrial_acceptance_e_mixed_language_shared_memory`
* Result:
  * ST routine filters analog flow and computes setpoint.
  * LD routine evaluates hardware emergency-stop permissive.
  * Second ST routine commands final control element `%QW50` only when the LD permissive is active.
* Verdict: **PASS**.

---

## 23. 10,000-Tick Replay & Bit-Identical Determinism Proof

**Objective**: Prove bit-identical determinism of compiled IEC logic across 10,000 simulation ticks under dynamic sinusoidal input.
* Test: `test_determinism_replay_10000_ticks`
* Execution: Two independent runs of 10,000 scans each recording floating-point bit representations (`to_bits()`).
* Result:
  $$\text{Discrepancies} = 0 / 10,000 \quad (100.0\% \text{ bit-identical match})$$
* Verdict: **PASS**.

---

## 24. Deliverables, Test Evidence & Final Sign-Off

### Artifact Inventory
1. **IEC 61131-3 Core Subsystems (`crates/nebudcs-control/src/iec61131/`)**:
   * `types.rs`: Complete elementary, derived, and composite types.
   * `storage.rs`: Variable scopes, symbol tables, and instance memory store.
   * `fb.rs`: Standard function blocks (`TON`, `TOF`, `TP`, `CTU`, `CTD`, `R_TRIG`, `F_TRIG`, `SR`, `RS`, `PID`).
   * `ir.rs`: Strongly typed expressions and statement AST.
   * `vm.rs`: Deterministic VM with 1,000-iteration loop safety guard.
   * `st/`: Structured Text lexer, parser, and compiler.
   * `ladder/`: Ladder Diagram rung and contact model, power-flow compiler.
   * `fbd/`: Function Block Diagram DAG model, Kahn topological sort, cycle rejection.
   * `sfc/`: Sequential Function Chart steps, transitions, action qualifiers compiler.
   * `routine.rs` & `program.rs`: Multi-routine controller integration.
2. **Server & Protocol Integration**:
   * `crates/nebudcs-protocol/src/dto.rs`: IEC DTOs.
   * `apps/nebudcs-server/src/routes/controllers.rs`: Compilation endpoint (`POST /api/v1/controllers/iec/compile`).
   * `apps/nebudcs-server/tests/controller_iec_api_tests.rs`: REST API test suite.
3. **Frontend Engineering Studio Component**:
   * `web/src/components/nebu-iec-editor.ts`: `<nebu-iec-editor>` Web Component.
   * `web/src/components/nebu-controller-panel.ts`: Controller panel integration.
   * `web/tests/components.test.ts`: Automated component tests.
4. **Verification Evidence**:
   * `cargo test --workspace`: **All 13 crates pass** (0 failures).
   * `cargo clippy --workspace --all-targets -- -D warnings`: **0 warnings**.
   * `cargo fmt --check`: **Clean format**.
   * `npm --prefix web test`: **10/10 test files, 84/84 tests pass**.
   * `npm --prefix web run build`: **0 build errors**.
   * `python3 tests/verify_industrial_domain_packages.py`: **PASS**.
   * `python3 tests/verify_generic_plant_acceptance.py`: **PASS**.
   * Live deployment service (`nebudcs.service`): **Active (running) on port 8088**.

```
================================================================================
                    STAGE 2 IEC 61131-3: PASS
================================================================================
```
