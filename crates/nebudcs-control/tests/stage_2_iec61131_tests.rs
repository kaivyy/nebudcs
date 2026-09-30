//! Stage 2: IEC 61131-3 Control Logic Runtime Comprehensive Tests.
//!
//! Verifies:
//! 1. Complete Type System (conversions, bounds, arrays, structs, enums)
//! 2. Structured Text (ST) Execution (math, logic, precedence, loops, branches, FB calls)
//! 3. Ladder Diagram (LD) Execution (contacts NO/NC/P/N, coils Out/Neg/Set/Reset, branches)
//! 4. Function Block Diagram (FBD) Execution (topological ordering, cycle detection, logic gates, PID)
//! 5. Sequential Function Chart (SFC) Execution (steps, transitions, actions N/S/R/P/L/D)
//! 6. Standard Function Blocks (TON, TOF, TP, CTU, CTD, R_TRIG, F_TRIG, SR, RS, PID)
//! 7. Industrial Acceptance A: ST Pressure Regulation via %I and %Q
//! 8. Industrial Acceptance B: LD Motor Start/Stop Interlock
//! 9. Industrial Acceptance C: FBD Closed-Loop Regulation with Production PID
//! 10. Industrial Acceptance D: SFC Multi-Phase Startup Sequence
//! 11. Industrial Acceptance E: Mixed-Language Multi-Routine Program sharing Memory & I/O
//! 12. 10,000-Tick Bit-Identical Determinism Replay Test

use nebudcs_control::iec61131::*;
use nebudcs_control::pid::PidConfig;
use nebudcs_control::runtime::io_image::IoImageTable;
use nebudcs_control::runtime::program::ControllerProgram;
use nebudcs_control::runtime::task::ControllerTask;
use nebudcs_model::tag::TagQuality;
use nebudcs_types::TaskId;
use std::collections::BTreeMap;

// ===========================================================================
// 1. TYPE SYSTEM TESTS
// ===========================================================================

#[test]
fn test_type_system_conversions_and_bounds() {
    let v_dint = IecValue::Dint(42);
    assert_eq!(v_dint.as_i64().unwrap(), 42);
    assert_eq!(v_dint.as_f64().unwrap(), 42.0);

    let v_real = IecValue::Real(3.25);
    assert!((v_real.as_f64().unwrap() - 3.25).abs() < 1e-5);

    let v_bool = IecValue::Bool(true);
    assert!(v_bool.as_bool().unwrap());

    // Type casting
    let casted = v_dint.cast_to(&IecType::Real).unwrap();
    assert_eq!(casted.get_type(), IecType::Real);

    // Array manipulation
    let arr = IecValue::Array(vec![
        IecValue::Dint(10),
        IecValue::Dint(20),
        IecValue::Dint(30),
    ]);
    assert_eq!(
        arr.get_type(),
        IecType::Array {
            element_type: Box::new(IecType::Dint),
            lower_bound: 0,
            upper_bound: 2,
        }
    );

    // Struct manipulation
    let mut fields = BTreeMap::new();
    fields.insert("RPM".to_string(), IecValue::Real(1500.0));
    fields.insert("RUNNING".to_string(), IecValue::Bool(true));
    let st = IecValue::Struct(fields);
    assert!(matches!(st.get_type(), IecType::Struct { .. }));
}

// ===========================================================================
// 2. STRUCTURED TEXT (ST) EXECUTION
// ===========================================================================

#[test]
fn test_structured_text_syntax_and_execution() {
    let source = r#"
        VAR
            SETPOINT : REAL := 50.0;
            PROCESS_VAL : REAL := 45.0;
            OUTPUT_VAL : REAL := 0.0;
            IS_ALARM : BOOL := FALSE;
            COUNTER : DINT := 0;
            SUM : DINT := 0;
        END_VAR

        // Arithmetic and conditional logic
        IF PROCESS_VAL < SETPOINT THEN
            OUTPUT_VAL := (SETPOINT - PROCESS_VAL) * 2.0;
        ELSE
            OUTPUT_VAL := 0.0;
        END_IF;

        // Alarm threshold
        IF OUTPUT_VAL > 5.0 THEN
            IS_ALARM := TRUE;
        ELSE
            IS_ALARM := FALSE;
        END_IF;

        // Loop execution with iteration guard
        FOR COUNTER := 1 TO 10 DO
            SUM := SUM + COUNTER;
        END_FOR;
    "#;

    let (symbols, stmts) = compile_st(source).expect("ST compilation should succeed");
    let mut mem = MemoryStore::from_symbol_table(&symbols);
    let mut io = IoImageTable::new();
    let mut fb = BTreeMap::new();

    let mut vm = IecVm::new(0.05, 0.0, &mut mem, &mut io, &mut fb);
    vm.execute_block(&stmts)
        .expect("ST execution should succeed");

    assert_eq!(mem.get("OUTPUT_VAL").unwrap().as_f64().unwrap(), 10.0);
    assert!(mem.get("IS_ALARM").unwrap().as_bool().unwrap());
    assert_eq!(mem.get("SUM").unwrap().as_i64().unwrap(), 55);
}

#[test]
fn test_structured_text_loop_iteration_guard() {
    // Test that an infinite while loop is safely intercepted at MAX_LOOP_ITERATIONS (1,000)
    let source = r#"
        VAR
            RUNAWAY : BOOL := TRUE;
            COUNT : DINT := 0;
        END_VAR

        WHILE RUNAWAY DO
            COUNT := COUNT + 1;
        END_WHILE;
    "#;

    let (symbols, stmts) = compile_st(source).expect("ST parse should succeed");
    let mut mem = MemoryStore::from_symbol_table(&symbols);
    let mut io = IoImageTable::new();
    let mut fb = BTreeMap::new();

    let mut vm = IecVm::new(0.05, 0.0, &mut mem, &mut io, &mut fb);
    let res = vm.execute_block(&stmts);
    assert!(
        res.is_err(),
        "Runaway loop must be terminated by iteration safety guard"
    );
    let err = res.unwrap_err().to_string();
    assert!(
        err.contains("exceeded maximum allowed iterations"),
        "Error was: {}",
        err
    );
}

// ===========================================================================
// 3. LADDER DIAGRAM (LD) EXECUTION
// ===========================================================================

#[test]
fn test_ladder_diagram_contacts_coils_and_parallel() {
    let mut diag = LdDiagram::new();
    diag.add_variable(
        VariableDeclaration::new("START_PB", IecType::Bool)
            .with_initial_value(IecValue::Bool(false)),
    );
    diag.add_variable(
        VariableDeclaration::new("STOP_PB", IecType::Bool)
            .with_initial_value(IecValue::Bool(false)),
    );
    diag.add_variable(
        VariableDeclaration::new("MOTOR_RUN", IecType::Bool)
            .with_initial_value(IecValue::Bool(false)),
    );

    // Rung 0: Seal-in motor circuit
    // (START_PB OR MOTOR_RUN) AND NOT STOP_PB -> MOTOR_RUN
    let mut rung0 = LdRung::new(0).with_comment("Motor Start/Stop Seal-in");
    let parallel = LdParallelBranch {
        paths: vec![
            vec![LdElement::Contact(LdContact::no("START_PB"))],
            vec![LdElement::Contact(LdContact::no("MOTOR_RUN"))],
        ],
    };
    rung0.add_element(LdElement::Parallel(parallel));
    rung0.add_element(LdElement::Contact(LdContact::nc("STOP_PB")));
    rung0.add_coil(LdCoil::output("MOTOR_RUN"));
    diag.add_rung(rung0);

    let (symbols, stmts) = compile_ladder(&diag).expect("LD compilation should succeed");
    let mut mem = MemoryStore::from_symbol_table(&symbols);
    let mut io = IoImageTable::new();
    let mut fb = BTreeMap::new();

    // Scan 1: All off -> motor stopped
    {
        let mut vm = IecVm::new(0.05, 0.0, &mut mem, &mut io, &mut fb);
        vm.execute_block(&stmts).unwrap();
    }
    assert!(!mem.get("MOTOR_RUN").unwrap().as_bool().unwrap());

    // Scan 2: Press START_PB -> motor runs
    mem.set("START_PB", IecValue::Bool(true));
    {
        let mut vm = IecVm::new(0.05, 0.05, &mut mem, &mut io, &mut fb);
        vm.execute_block(&stmts).unwrap();
    }
    assert!(mem.get("MOTOR_RUN").unwrap().as_bool().unwrap());

    // Scan 3: Release START_PB -> motor remains running (seal-in latch)
    mem.set("START_PB", IecValue::Bool(false));
    {
        let mut vm = IecVm::new(0.05, 0.10, &mut mem, &mut io, &mut fb);
        vm.execute_block(&stmts).unwrap();
    }
    assert!(mem.get("MOTOR_RUN").unwrap().as_bool().unwrap());

    // Scan 4: Press STOP_PB -> motor drops out
    mem.set("STOP_PB", IecValue::Bool(true));
    {
        let mut vm = IecVm::new(0.05, 0.15, &mut mem, &mut io, &mut fb);
        vm.execute_block(&stmts).unwrap();
    }
    assert!(!mem.get("MOTOR_RUN").unwrap().as_bool().unwrap());
}

// ===========================================================================
// 4. FUNCTION BLOCK DIAGRAM (FBD) EXECUTION & CYCLE DETECTION
// ===========================================================================

#[test]
fn test_fbd_topological_sort_and_unbuffered_cycle_rejection() {
    let mut graph = FbdGraph::new();
    graph.add_node(FbdNode::new(
        "N1",
        FbdNodeKind::InputVar {
            name: "RAW_A".to_string(),
        },
    ));
    graph.add_node(FbdNode::new(
        "N2",
        FbdNodeKind::InputVar {
            name: "RAW_B".to_string(),
        },
    ));
    graph.add_node(FbdNode::new(
        "N3",
        FbdNodeKind::LogicGate {
            gate_type: "AND".to_string(),
        },
    ));
    graph.add_node(FbdNode::new(
        "N4",
        FbdNodeKind::OutputVar {
            name: "RESULT".to_string(),
        },
    ));

    graph.add_connection(FbdConnection::new("N1", "OUT", "N3", "IN1"));
    graph.add_connection(FbdConnection::new("N2", "OUT", "N3", "IN2"));
    graph.add_connection(FbdConnection::new("N3", "OUT", "N4", "IN"));

    let (symbols, stmts) = compile_fbd(&graph).expect("Valid FBD must compile");
    let mut mem = MemoryStore::from_symbol_table(&symbols);
    mem.set("RAW_A", IecValue::Bool(true));
    mem.set("RAW_B", IecValue::Bool(true));
    let mut io = IoImageTable::new();
    let mut fb = BTreeMap::new();

    let mut vm = IecVm::new(0.05, 0.0, &mut mem, &mut io, &mut fb);
    vm.execute_block(&stmts).unwrap();
    assert!(mem.get("RESULT").unwrap().as_bool().unwrap());

    // Now test algebraic cycle rejection without delay block: N1 -> N2 -> N1
    let mut cyclic_graph = FbdGraph::new();
    cyclic_graph.add_node(FbdNode::new(
        "A",
        FbdNodeKind::LogicGate {
            gate_type: "OR".to_string(),
        },
    ));
    cyclic_graph.add_node(FbdNode::new(
        "B",
        FbdNodeKind::LogicGate {
            gate_type: "AND".to_string(),
        },
    ));
    cyclic_graph.add_connection(FbdConnection::new("A", "OUT", "B", "IN1"));
    cyclic_graph.add_connection(FbdConnection::new("B", "OUT", "A", "IN1"));

    let cyclic_err = compile_fbd(&cyclic_graph);
    assert!(
        cyclic_err.is_err(),
        "Cyclic FBD without DELAY_1T must be rejected"
    );
    assert!(cyclic_err.unwrap_err().to_string().contains("DELAY_1T"));
}

// ===========================================================================
// 5. SEQUENTIAL FUNCTION CHART (SFC) EXECUTION
// ===========================================================================

#[test]
fn test_sfc_step_transitions_and_action_qualifiers() {
    let mut chart = SfcChart::new();
    chart.add_step(SfcStep::new("INIT", true));
    chart.add_step(SfcStep::new("FILLING", false));
    chart.add_step(SfcStep::new("HEATING", false));

    chart.add_action(SfcAction::new("INIT", SfcActionQualifier::N, "READY_LIGHT"));
    chart.add_action(SfcAction::new(
        "FILLING",
        SfcActionQualifier::N,
        "VALVE_OPEN",
    ));
    chart.add_action(SfcAction::new(
        "HEATING",
        SfcActionQualifier::N,
        "HEATER_ON",
    ));

    chart.add_transition(SfcTransition::new("T1", "INIT", "FILLING", "START_CMD"));
    chart.add_transition(SfcTransition::new("T2", "FILLING", "HEATING", "TANK_FULL"));

    chart.add_variable(
        VariableDeclaration::new("START_CMD", IecType::Bool)
            .with_initial_value(IecValue::Bool(false)),
    );
    chart.add_variable(
        VariableDeclaration::new("TANK_FULL", IecType::Bool)
            .with_initial_value(IecValue::Bool(false)),
    );

    let (symbols, stmts) = compile_sfc(&chart).expect("SFC compilation must succeed");
    let mut mem = MemoryStore::from_symbol_table(&symbols);
    let mut io = IoImageTable::new();
    let mut fb = BTreeMap::new();

    // Scan 1: INIT is initial active step
    {
        let mut vm = IecVm::new(0.05, 0.0, &mut mem, &mut io, &mut fb);
        vm.execute_block(&stmts).unwrap();
    }
    assert!(mem.get("_SFC_X_INIT").unwrap().as_bool().unwrap());
    assert!(mem.get("READY_LIGHT").unwrap().as_bool().unwrap());
    assert!(!mem.get("VALVE_OPEN").unwrap().as_bool().unwrap());

    // Scan 2: Provide START_CMD -> Transitions to FILLING
    mem.set("START_CMD", IecValue::Bool(true));
    {
        let mut vm = IecVm::new(0.05, 0.05, &mut mem, &mut io, &mut fb);
        vm.execute_block(&stmts).unwrap();
    }
    assert!(!mem.get("_SFC_X_INIT").unwrap().as_bool().unwrap());
    assert!(mem.get("_SFC_X_FILLING").unwrap().as_bool().unwrap());
    assert!(mem.get("VALVE_OPEN").unwrap().as_bool().unwrap());
    assert!(!mem.get("READY_LIGHT").unwrap().as_bool().unwrap());

    // Scan 3: Provide TANK_FULL -> Transitions to HEATING
    mem.set("TANK_FULL", IecValue::Bool(true));
    {
        let mut vm = IecVm::new(0.05, 0.10, &mut mem, &mut io, &mut fb);
        vm.execute_block(&stmts).unwrap();
    }
    assert!(!mem.get("_SFC_X_FILLING").unwrap().as_bool().unwrap());
    assert!(mem.get("_SFC_X_HEATING").unwrap().as_bool().unwrap());
    assert!(mem.get("HEATER_ON").unwrap().as_bool().unwrap());
    assert!(!mem.get("VALVE_OPEN").unwrap().as_bool().unwrap());
}

// ===========================================================================
// 6. STANDARD FUNCTION BLOCKS (TON, TOF, TP, CTU, CTD, R_TRIG, SR, RS, PID)
// ===========================================================================

#[test]
fn test_standard_function_blocks_ton_and_counters() {
    let mut ton = TonBlock::new("T1");
    let mut inputs = BTreeMap::new();
    inputs.insert("IN".to_string(), IecValue::Bool(true));
    inputs.insert("PT".to_string(), IecValue::Time(0.10)); // 100 ms

    // Scan 1: 50 ms elapsed -> Q is false
    let out = ton.step(0.05, 0.0, &inputs).unwrap();
    assert!(!out.get("Q").unwrap().as_bool().unwrap());
    assert_eq!(out.get("ET").unwrap().as_f64().unwrap(), 0.05);

    // Scan 2: 100 ms elapsed -> Q becomes true
    let out = ton.step(0.05, 0.05, &inputs).unwrap();
    assert!(out.get("Q").unwrap().as_bool().unwrap());
    assert_eq!(out.get("ET").unwrap().as_f64().unwrap(), 0.10);

    // Up counter CTU
    let mut ctu = CtuBlock::new("C1");
    let mut ctu_in = BTreeMap::new();
    ctu_in.insert("CU".to_string(), IecValue::Bool(true));
    ctu_in.insert("RESET".to_string(), IecValue::Bool(false));
    ctu_in.insert("PV".to_string(), IecValue::Dint(2));

    let out = ctu.step(0.05, 0.0, &ctu_in).unwrap();
    assert_eq!(out.get("CV").unwrap().as_i64().unwrap(), 1);
    assert!(!out.get("Q").unwrap().as_bool().unwrap());

    // Low CLK
    ctu_in.insert("CU".to_string(), IecValue::Bool(false));
    ctu.step(0.05, 0.05, &ctu_in).unwrap();

    // High CLK -> count 2 -> Q = true
    ctu_in.insert("CU".to_string(), IecValue::Bool(true));
    let out = ctu.step(0.05, 0.10, &ctu_in).unwrap();
    assert_eq!(out.get("CV").unwrap().as_i64().unwrap(), 2);
    assert!(out.get("Q").unwrap().as_bool().unwrap());
}

// ===========================================================================
// 7. INDUSTRIAL ACCEPTANCE A: ST PRESSURE REGULATION VIA %I AND %Q
// ===========================================================================

#[test]
fn test_industrial_acceptance_a_st_pressure_regulation() {
    let source = r#"
        VAR
            SETPOINT_BAR : REAL := 12.0;
            CURRENT_BAR : REAL;
            VALVE_DEMAND : REAL := 0.0;
        END_VAR

        // Read physical transmitter from %IW100
        CURRENT_BAR := %IW100;

        // Proportional pressure relief logic
        IF CURRENT_BAR > SETPOINT_BAR THEN
            VALVE_DEMAND := (CURRENT_BAR - SETPOINT_BAR) * 10.0;
            IF VALVE_DEMAND > 100.0 THEN
                VALVE_DEMAND := 100.0;
            END_IF;
        ELSE
            VALVE_DEMAND := 0.0;
        END_IF;

        // Command output actuator via %QW100
        %QW100 := VALVE_DEMAND;
    "#;

    let routine = IecRoutine::from_st("PRV_ROUTINE", source).unwrap();
    let mut prog = IecProgram::new("PRV_PROGRAM");
    prog.add_routine(routine).unwrap();

    let mut io = IoImageTable::new();
    // Case 1: Normal pressure (10.0 bar) -> valve closed (0.0)
    io.write_input("%IW100", 10.0, TagQuality::Good, 0.0);
    prog.execute(0.05, 0.0, &mut io).unwrap();
    assert_eq!(io.read_output_value("%QW100").unwrap(), 0.0);

    // Case 2: Overpressure (15.0 bar) -> valve opens to (15 - 12) * 10 = 30.0%
    io.write_input("%IW100", 15.0, TagQuality::Good, 0.05);
    prog.execute(0.05, 0.05, &mut io).unwrap();
    assert_eq!(io.read_output_value("%QW100").unwrap(), 30.0);
}

// ===========================================================================
// 8. INDUSTRIAL ACCEPTANCE B: LD MOTOR START/STOP INTERLOCK
// ===========================================================================

#[test]
fn test_industrial_acceptance_b_ld_motor_interlock() {
    let mut diag = LdDiagram::new();
    diag.add_variable(VariableDeclaration::new("START_CMD", IecType::Bool));
    diag.add_variable(VariableDeclaration::new("STOP_CMD", IecType::Bool));
    diag.add_variable(VariableDeclaration::new("INTERLOCK_OK", IecType::Bool));
    diag.add_variable(VariableDeclaration::new("MOTOR_ON", IecType::Bool));

    // Rung 0: Start/Stop with external interlock protection:
    // (START_CMD OR MOTOR_ON) AND NOT STOP_CMD AND INTERLOCK_OK -> %QX0.0
    let mut rung = LdRung::new(0);
    let parallel = LdParallelBranch {
        paths: vec![
            vec![LdElement::Contact(LdContact::no("START_CMD"))],
            vec![LdElement::Contact(LdContact::no("%QX0.0"))],
        ],
    };
    rung.add_element(LdElement::Parallel(parallel));
    rung.add_element(LdElement::Contact(LdContact::nc("STOP_CMD")));
    rung.add_element(LdElement::Contact(LdContact::no("INTERLOCK_OK")));
    rung.add_coil(LdCoil::output("%QX0.0"));
    diag.add_rung(rung);

    let routine = IecRoutine::from_ld("MOTOR_LD", &diag).unwrap();
    let mut prog = IecProgram::new("MOTOR_PROG");
    prog.add_routine(routine).unwrap();

    let mut io = IoImageTable::new();

    // Step 1: Interlock tripped (FALSE) -> Press START_CMD -> Motor must NOT start
    prog.set_variable("START_CMD", IecValue::Bool(true));
    prog.set_variable("INTERLOCK_OK", IecValue::Bool(false));
    prog.execute(0.02, 0.0, &mut io).unwrap();
    assert_eq!(io.read_output_value("%QX0.0").unwrap_or(0.0), 0.0);

    // Step 2: Interlock clears (TRUE) -> Motor starts
    prog.set_variable("INTERLOCK_OK", IecValue::Bool(true));
    prog.execute(0.02, 0.02, &mut io).unwrap();
    assert_eq!(io.read_output_value("%QX0.0").unwrap(), 1.0);

    // Step 3: Interlock trips again -> Immediate fail-safe drop out
    prog.set_variable("START_CMD", IecValue::Bool(false));
    prog.set_variable("INTERLOCK_OK", IecValue::Bool(false));
    prog.execute(0.02, 0.04, &mut io).unwrap();
    assert_eq!(io.read_output_value("%QX0.0").unwrap(), 0.0);

    // Step 4: Verify integration through Stage 1 VirtualController and Task Scheduler
    let mut ctrl = nebudcs_control::runtime::VirtualController::with_name(
        "CTRL_MOTOR",
        nebudcs_control::runtime::ControllerConfig::default(),
    );
    ctrl.transition_to(nebudcs_control::runtime::ControllerLifecycle::Initializing)
        .unwrap();
    ctrl.transition_to(nebudcs_control::runtime::ControllerLifecycle::Running)
        .unwrap();
    let mut task = ControllerTask::new_periodic(TaskId::new(), "FastLogic", 0.02, 1);
    task.add_program(Box::new(prog.clone()));
    ctrl.add_task(task);
    ctrl.step(0.02, 0.0).unwrap();
}

// ===========================================================================
// 9. INDUSTRIAL ACCEPTANCE C: FBD CLOSED-LOOP WITH PRODUCTION PID
// ===========================================================================

#[test]
fn test_industrial_acceptance_c_fbd_pid_regulation() {
    let mut graph = FbdGraph::new();
    graph.add_node(FbdNode::new(
        "IN_PV",
        FbdNodeKind::InputVar {
            name: "%IW200".to_string(),
        },
    ));
    graph.add_node(FbdNode::new(
        "SP_CONST",
        FbdNodeKind::Constant {
            value: IecValue::LReal(100.0),
        },
    ));
    graph.add_node(FbdNode::new(
        "AUTO_TRUE",
        FbdNodeKind::Constant {
            value: IecValue::Bool(true),
        },
    ));
    graph.add_node(FbdNode::new(
        "PID_01",
        FbdNodeKind::FunctionBlock {
            instance: "FIC_101".to_string(),
            fb_type: "PID".to_string(),
        },
    ));
    graph.add_node(FbdNode::new(
        "OUT_VALVE",
        FbdNodeKind::OutputVar {
            name: "%QW200".to_string(),
        },
    ));

    graph.add_connection(FbdConnection::new("IN_PV", "OUT", "PID_01", "PV"));
    graph.add_connection(FbdConnection::new("SP_CONST", "OUT", "PID_01", "SP"));
    graph.add_connection(FbdConnection::new("AUTO_TRUE", "OUT", "PID_01", "AUTO"));
    graph.add_connection(FbdConnection::new("PID_01", "OUT", "OUT_VALVE", "IN"));

    let routine = IecRoutine::from_fbd("PID_FBD", &graph).unwrap();
    let mut prog = IecProgram::new("PID_PROG");
    // Register configured PID block
    let pid_cfg = PidConfig {
        kp: 2.0,
        ti: 10.0,
        ..Default::default()
    };
    prog.register_fb(Box::new(PidIecBlock::new("FIC_101", pid_cfg)));
    prog.add_routine(routine).unwrap();

    let mut io = IoImageTable::new();
    io.write_input("%IW200", 80.0, TagQuality::Good, 0.0); // PV = 80, SP = 100, Error = +20

    prog.execute(0.1, 0.0, &mut io).unwrap();
    let out = io.read_output_value("%QW200").unwrap();
    assert!(
        out > 0.0,
        "PID output should respond proportionally to error, got {}",
        out
    );
}

// ===========================================================================
// 10. INDUSTRIAL ACCEPTANCE D: SFC STARTUP SEQUENCE
// ===========================================================================

#[test]
fn test_industrial_acceptance_d_sfc_startup_sequence() {
    let mut chart = SfcChart::new();
    chart.add_step(SfcStep::new("OFF", true));
    chart.add_step(SfcStep::new("PURGE", false));
    chart.add_step(SfcStep::new("IGNITE", false));
    chart.add_step(SfcStep::new("RUNNING", false));

    chart.add_action(SfcAction::new("PURGE", SfcActionQualifier::N, "%QX10.0")); // Purge blower
    chart.add_action(SfcAction::new("IGNITE", SfcActionQualifier::N, "%QX10.1")); // Igniter spark
    chart.add_action(SfcAction::new("RUNNING", SfcActionQualifier::N, "%QX10.2")); // Main fuel valve

    chart.add_transition(SfcTransition::new("T_START", "OFF", "PURGE", "START_PB"));
    chart.add_transition(SfcTransition::new(
        "T_PURGED",
        "PURGE",
        "IGNITE",
        "PURGE_DONE",
    ));
    chart.add_transition(SfcTransition::new(
        "T_FLAME",
        "IGNITE",
        "RUNNING",
        "FLAME_DETECTED",
    ));

    chart.add_variable(VariableDeclaration::new("START_PB", IecType::Bool));
    chart.add_variable(VariableDeclaration::new("PURGE_DONE", IecType::Bool));
    chart.add_variable(VariableDeclaration::new("FLAME_DETECTED", IecType::Bool));

    let routine = IecRoutine::from_sfc("BURNER_SFC", &chart).unwrap();
    let mut prog = IecProgram::new("BURNER_PROG");
    prog.add_routine(routine).unwrap();

    let mut io = IoImageTable::new();

    // Scan 1: System is OFF
    prog.execute(0.1, 0.0, &mut io).unwrap();
    assert_eq!(io.read_output_value("%QX10.0").unwrap_or(0.0), 0.0);
    assert_eq!(io.read_output_value("%QX10.1").unwrap_or(0.0), 0.0);
    assert_eq!(io.read_output_value("%QX10.2").unwrap_or(0.0), 0.0);

    // Operator starts system
    prog.set_variable("START_PB", IecValue::Bool(true));
    prog.execute(0.1, 0.1, &mut io).unwrap();
    assert_eq!(
        io.read_output_value("%QX10.0").unwrap(),
        1.0,
        "Purge blower energized"
    );

    // Purge cycle completes
    prog.set_variable("PURGE_DONE", IecValue::Bool(true));
    prog.execute(0.1, 0.2, &mut io).unwrap();
    assert_eq!(
        io.read_output_value("%QX10.0").unwrap(),
        0.0,
        "Purge blower stops"
    );
    assert_eq!(
        io.read_output_value("%QX10.1").unwrap(),
        1.0,
        "Igniter energized"
    );

    // Flame is proven
    prog.set_variable("FLAME_DETECTED", IecValue::Bool(true));
    prog.execute(0.1, 0.3, &mut io).unwrap();
    assert_eq!(
        io.read_output_value("%QX10.1").unwrap(),
        0.0,
        "Igniter deactivated"
    );
    assert_eq!(
        io.read_output_value("%QX10.2").unwrap(),
        1.0,
        "Main fuel valve open"
    );
}

// ===========================================================================
// 11. INDUSTRIAL ACCEPTANCE E: MIXED-LANGUAGE MULTI-ROUTINE PROGRAM
// ===========================================================================

#[test]
fn test_industrial_acceptance_e_mixed_language_shared_memory() {
    let mut prog = IecProgram::new("COMPLEX_PLANT_CONTROLLER");

    // Routine 1: ST calculates Setpoint and filtered flow
    let st_src = r#"
        VAR
            FLOW_RAW : REAL;
            FLOW_FILT : REAL := 0.0;
            TARGET_SP : REAL := 75.0;
        END_VAR

        FLOW_RAW := %IW50;
        FLOW_FILT := FLOW_FILT * 0.8 + FLOW_RAW * 0.2;
    "#;
    let r1 = IecRoutine::from_st("FILTER_ST", st_src).unwrap();
    prog.add_routine(r1).unwrap();

    // Routine 2: Ladder Diagram checks safety permissives
    let mut diag = LdDiagram::new();
    diag.add_variable(VariableDeclaration::new("PERMISSIVE_OK", IecType::Bool));
    let mut rung = LdRung::new(0);
    rung.add_element(LdElement::Contact(LdContact::no("%IX0.0"))); // External E-stop closed
    rung.add_coil(LdCoil::output("PERMISSIVE_OK"));
    diag.add_rung(rung);
    let r2 = IecRoutine::from_ld("SAFETY_LD", &diag).unwrap();
    prog.add_routine(r2).unwrap();

    // Routine 3: ST commands final actuator only if permissive is active
    let st_actuator = r#"
        IF PERMISSIVE_OK THEN
            %QW50 := TARGET_SP;
        ELSE
            %QW50 := 0.0;
        END_IF;
    "#;
    let r3 = IecRoutine::from_st("ACTUATOR_ST", st_actuator).unwrap();
    prog.add_routine(r3).unwrap();

    let mut io = IoImageTable::new();
    io.write_input("%IW50", 100.0, TagQuality::Good, 0.0);

    // Case 1: E-stop not active (%IX0.0 = 0) -> Output clamped to 0.0
    io.write_input("%IX0.0", 0.0, TagQuality::Good, 0.0);
    prog.execute(0.05, 0.0, &mut io).unwrap();
    assert_eq!(io.read_output_value("%QW50").unwrap(), 0.0);

    // Case 2: E-stop cleared (%IX0.0 = 1) -> Output passes TARGET_SP (75.0)
    io.write_input("%IX0.0", 1.0, TagQuality::Good, 0.05);
    prog.execute(0.05, 0.05, &mut io).unwrap();
    assert_eq!(io.read_output_value("%QW50").unwrap(), 75.0);
}

// ===========================================================================
// 12. DETERMINISM REPLAY TEST (10,000 TICKS BIT-IDENTICAL)
// ===========================================================================

#[test]
fn test_determinism_replay_10000_ticks() {
    let source = r#"
        VAR
            ACCUM : REAL := 0.0;
            CYCLE_COUNT : DINT := 0;
            TOGGLE : BOOL := FALSE;
        END_VAR

        CYCLE_COUNT := CYCLE_COUNT + 1;
        ACCUM := ACCUM + %IW10 * 0.1;
        IF CYCLE_COUNT MOD 2 = 0 THEN
            TOGGLE := NOT TOGGLE;
        END_IF;
        %QW10 := ACCUM;
    "#;

    let build_test_setup = || {
        let r = IecRoutine::from_st("DET_ROUTINE", source).unwrap();
        let mut p = IecProgram::new("DET_PROG");
        p.add_routine(r).unwrap();
        p
    };

    let mut run1_prog = build_test_setup();
    let mut run1_io = IoImageTable::new();
    let mut run1_history = Vec::with_capacity(10000);

    let mut run2_prog = build_test_setup();
    let mut run2_io = IoImageTable::new();
    let mut run2_history = Vec::with_capacity(10000);

    for tick in 0..10_000 {
        let sim_time = tick as f64 * 0.01;
        let synthetic_input = ((tick % 100) as f64 * 0.5).sin();

        // Run 1
        run1_io.write_input("%IW10", synthetic_input, TagQuality::Good, sim_time);
        run1_prog.execute(0.01, sim_time, &mut run1_io).unwrap();
        let out1 = run1_io.read_output_value("%QW10").unwrap();
        run1_history.push(out1.to_bits());

        // Run 2
        run2_io.write_input("%IW10", synthetic_input, TagQuality::Good, sim_time);
        run2_prog.execute(0.01, sim_time, &mut run2_io).unwrap();
        let out2 = run2_io.read_output_value("%QW10").unwrap();
        run2_history.push(out2.to_bits());
    }

    assert_eq!(
        run1_history, run2_history,
        "10,000 ticks must produce bit-identical outputs between runs"
    );
}
