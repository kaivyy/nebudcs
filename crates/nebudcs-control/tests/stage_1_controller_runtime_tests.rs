//! Comprehensive Stage 1 Control Runtime Verification Suite.
//!
//! Tests virtual controller lifecycle, task scheduling determinism, priority preemption,
//! watchdog & task overrun detection, multi-controller isolation, and end-to-end
//! closed-loop acceptance on a generic plant (Tank-A -> FT-A1 -> Control Valve -> Tank-B).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use nebudcs_control::actuator::{ActuatorConfig, CommandPriority, FailAction};
use nebudcs_control::error::ControlError;
use nebudcs_control::instrument::InstrumentConfig;
use nebudcs_control::pid::{PidConfig, PidDirection, PidMode};
use nebudcs_control::protection::TripDirection;
use nebudcs_control::runtime::{
    ControllerConfig, ControllerLifecycle, ControllerProgram, ControllerTask, InterlockProgram,
    IoImageTable, PidLoopProgram, RoutineProgram, VirtualController,
};
use nebudcs_control::system::ControlSystem;
use nebudcs_model::tag::TagQuality;
use nebudcs_physics::equipment::{AccumulatorVolumeModel, FlowResistanceModel};
use nebudcs_sim::{
    EquipmentState, SimError, SimulationEngine, SimulationSystem, StateBuffer, StateReadView,
    StateWriteView, StepContext,
};
use nebudcs_types::{ControllerId, EquipmentId, TagId, TaskId};
use nebudcs_units::{Density, MassFlow};

// ---------------------------------------------------------------------------
// 1. Controller Lifecycle State Machine Tests
// ---------------------------------------------------------------------------

#[test]
fn test_controller_lifecycle_state_machine() {
    let mut ctrl = VirtualController::with_name("CTRL-LIFE", ControllerConfig::default());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Stopped);

    // Stopped -> Initializing -> Running
    assert!(ctrl
        .transition_to(ControllerLifecycle::Initializing)
        .is_ok());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Initializing);

    assert!(ctrl.transition_to(ControllerLifecycle::Running).is_ok());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Running);

    // Running -> Paused -> Running
    assert!(ctrl.transition_to(ControllerLifecycle::Paused).is_ok());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Paused);

    assert!(ctrl.transition_to(ControllerLifecycle::Running).is_ok());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Running);

    // Running -> Stopped
    assert!(ctrl.transition_to(ControllerLifecycle::Stopped).is_ok());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Stopped);

    // Invalid transitions
    assert!(matches!(
        ctrl.transition_to(ControllerLifecycle::Paused),
        Err(ControlError::InvalidLifecycleTransition { .. })
    ));
}

// ---------------------------------------------------------------------------
// 2. Deterministic Task Scheduling & Priority Ordering Tests
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct PriorityLoggingProgram {
    name: String,
    priority: u8,
    log: Arc<std::sync::Mutex<Vec<u8>>>,
}

impl ControllerProgram for PriorityLoggingProgram {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(
        &mut self,
        _dt: f64,
        _sim_time: f64,
        _io_image: &mut IoImageTable,
    ) -> Result<(), ControlError> {
        self.log.lock().unwrap().push(self.priority);
        Ok(())
    }

    fn reset(&mut self) {}

    fn clone_box(&self) -> Box<dyn ControllerProgram> {
        Box::new(self.clone())
    }
}

#[test]
fn test_task_periodic_scheduling_and_priority_ordering() {
    let mut ctrl = VirtualController::with_name("CTRL-SCHED", ControllerConfig::default());
    let _ = ctrl.transition_to(ControllerLifecycle::Initializing);
    let _ = ctrl.transition_to(ControllerLifecycle::Running);

    let execution_log = Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));

    // Task 1: 10ms (0.01s), Priority 1
    let task1_id = TaskId::new();
    let mut task1 = ControllerTask::new_periodic(task1_id, "Fast_10ms", 0.01, 1);
    task1.add_program(Box::new(PriorityLoggingProgram {
        name: "Log_P1".into(),
        priority: 1,
        log: Arc::clone(&execution_log),
    }));
    ctrl.add_task(task1);

    // Task 2: 50ms (0.05s), Priority 5
    let task2_id = TaskId::new();
    let mut task2 = ControllerTask::new_periodic(task2_id, "Base_50ms", 0.05, 5);
    task2.add_program(Box::new(PriorityLoggingProgram {
        name: "Log_P5".into(),
        priority: 5,
        log: Arc::clone(&execution_log),
    }));
    ctrl.add_task(task2);

    // Task 3: 250ms (0.25s), Priority 10
    let task3_id = TaskId::new();
    let mut task3 = ControllerTask::new_periodic(task3_id, "Slow_250ms", 0.25, 10);
    task3.add_program(Box::new(PriorityLoggingProgram {
        name: "Log_P10".into(),
        priority: 10,
        log: Arc::clone(&execution_log),
    }));
    ctrl.add_task(task3);

    // Run 25 steps of 10ms (total 250ms)
    let dt = 0.01;
    for step in 1..=25 {
        let sim_time = step as f64 * dt;
        ctrl.step(dt, sim_time).expect("Step should succeed");
    }

    // Verify execution counts
    let t1 = ctrl.task(task1_id).unwrap();
    let t2 = ctrl.task(task2_id).unwrap();
    let t3 = ctrl.task(task3_id).unwrap();

    assert_eq!(
        t1.execution_count(),
        25,
        "10ms task must execute 25 times in 250ms"
    );
    assert_eq!(
        t2.execution_count(),
        5,
        "50ms task must execute 5 times in 250ms"
    );
    assert_eq!(
        t3.execution_count(),
        1,
        "250ms task must execute 1 time in 250ms"
    );

    // Verify priority ordering: at tick 5 (50ms), P1 runs before P5
    // At tick 25 (250ms), P1 runs before P5, and P5 runs before P10
    let log = execution_log.lock().unwrap().clone();
    let len = log.len();
    assert!(len >= 3);
    assert_eq!(
        &log[len - 3..len],
        &[1, 5, 10],
        "Tasks executed at same tick must follow priority order 1, 5, 10"
    );
}

// ---------------------------------------------------------------------------
// 3. Disabled Task Behavior Tests
// ---------------------------------------------------------------------------

#[test]
fn test_disabled_task_does_not_execute() {
    let mut ctrl = VirtualController::with_name("CTRL-DIS", ControllerConfig::default());
    let _ = ctrl.transition_to(ControllerLifecycle::Initializing);
    let _ = ctrl.transition_to(ControllerLifecycle::Running);

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    let task_id = TaskId::new();
    let mut task = ControllerTask::new_periodic(task_id, "DisabledTask", 0.01, 5);
    task.set_enabled(false);
    task.add_program(Box::new(RoutineProgram::new(
        "IncRoutine",
        move |_dt, _sim_time, _io| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
    )));
    ctrl.add_task(task);

    for step in 1..=10 {
        ctrl.step(0.01, step as f64 * 0.01).unwrap();
    }

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(ctrl.task(task_id).unwrap().execution_count(), 0);
}

// ---------------------------------------------------------------------------
// 4. Watchdog & Task Overrun Detection Tests
// ---------------------------------------------------------------------------

#[test]
fn test_task_overrun_and_controller_watchdog() {
    // Config with 1ms watchdog timeout
    let cfg = ControllerConfig {
        watchdog_timeout_seconds: 0.001,
        ..Default::default()
    };

    let mut ctrl = VirtualController::with_name("CTRL-WDOG", cfg);
    let _ = ctrl.transition_to(ControllerLifecycle::Initializing);
    let _ = ctrl.transition_to(ControllerLifecycle::Running);

    let task_id = TaskId::new();
    // Task with 500us watchdog limit
    let mut task =
        ControllerTask::new_periodic(task_id, "SlowTask", 0.01, 1).with_watchdog_limit(0.0005);

    task.add_program(Box::new(RoutineProgram::new(
        "SleepRoutine",
        |_dt, _sim_time, _io| {
            sleep(Duration::from_millis(2)); // Sleep 2ms, exceeding 500us and 1ms
            Ok(())
        },
    )));
    ctrl.add_task(task);

    ctrl.step(0.01, 0.01).unwrap();

    let task_diag = ctrl.task(task_id).unwrap();
    assert!(
        task_diag.overrun_count() > 0,
        "Task overrun must be detected when duration exceeds limit"
    );

    let ctrl_diag = ctrl.diagnostics();
    assert!(
        ctrl_diag.watchdog_trips > 0,
        "Controller watchdog trip must be recorded"
    );
}

// ---------------------------------------------------------------------------
// 5. Fatal Error Fault Handling Tests
// ---------------------------------------------------------------------------

#[test]
fn test_controller_fatal_fault_tripping() {
    let mut ctrl = VirtualController::with_name("CTRL-FAULT", ControllerConfig::default());
    let _ = ctrl.transition_to(ControllerLifecycle::Initializing);
    let _ = ctrl.transition_to(ControllerLifecycle::Running);

    let task_id = TaskId::new();
    let mut task = ControllerTask::new_periodic(task_id, "FailingTask", 0.01, 1);
    task.add_program(Box::new(RoutineProgram::new(
        "FailRoutine",
        |_dt, _sim_time, _io| {
            Err(ControlError::TaskExecutionError {
                task_id: "FailingTask".to_string(),
                detail: "Simulated divide by zero in routine".to_string(),
            })
        },
    )));
    ctrl.add_task(task);

    let res = ctrl.step(0.01, 0.01);
    assert!(res.is_err());
    assert_eq!(ctrl.lifecycle(), ControllerLifecycle::Faulted);
    assert!(ctrl.fault().is_some());
    assert_eq!(ctrl.fault().unwrap().code, "TASK_EXECUTION_ERROR");

    // Subsequent step while faulted must return error and not execute
    let next_res = ctrl.step(0.01, 0.02);
    assert!(matches!(
        next_res,
        Err(ControlError::ControllerFaulted { .. })
    ));
}

// ---------------------------------------------------------------------------
// 6. Multi-Controller I/O Image Table Isolation Tests
// ---------------------------------------------------------------------------

#[test]
fn test_multi_controller_isolation() {
    let mut ctrl_a = VirtualController::with_name("CTRL-UNIT-A", ControllerConfig::default());
    let mut ctrl_b = VirtualController::with_name("CTRL-UNIT-B", ControllerConfig::default());

    let _ = ctrl_a.transition_to(ControllerLifecycle::Running);
    let _ = ctrl_b.transition_to(ControllerLifecycle::Running);

    // Controller A task writes to %Q "PUMP_SPEED"
    let mut task_a = ControllerTask::new_periodic(TaskId::new(), "TaskA", 0.05, 5);
    task_a.add_program(Box::new(RoutineProgram::new(
        "WriteA",
        |_dt, sim_time, io| {
            io.write_output("PUMP_SPEED", 1800.0, TagQuality::Good, sim_time);
            Ok(())
        },
    )));
    ctrl_a.add_task(task_a);

    // Controller B task writes to %Q "VALVE_POS"
    let mut task_b = ControllerTask::new_periodic(TaskId::new(), "TaskB", 0.05, 5);
    task_b.add_program(Box::new(RoutineProgram::new(
        "WriteB",
        |_dt, sim_time, io| {
            io.write_output("VALVE_POS", 75.0, TagQuality::Good, sim_time);
            Ok(())
        },
    )));
    ctrl_b.add_task(task_b);

    ctrl_a.step(0.05, 0.05).unwrap();
    ctrl_b.step(0.05, 0.05).unwrap();

    // Verify memory isolation between Controller A and Controller B
    assert_eq!(
        ctrl_a.io_image().read_output_value("PUMP_SPEED"),
        Some(1800.0)
    );
    assert_eq!(
        ctrl_a.io_image().read_output_value("VALVE_POS"),
        None,
        "CTRL-A must not see CTRL-B outputs"
    );

    assert_eq!(ctrl_b.io_image().read_output_value("VALVE_POS"), Some(75.0));
    assert_eq!(
        ctrl_b.io_image().read_output_value("PUMP_SPEED"),
        None,
        "CTRL-B must not see CTRL-A outputs"
    );
}

// ---------------------------------------------------------------------------
// 7. Deterministic Reset Tests
// ---------------------------------------------------------------------------

#[test]
fn test_controller_deterministic_reset() {
    let mut ctrl = VirtualController::with_name("CTRL-RESET", ControllerConfig::default());
    let _ = ctrl.transition_to(ControllerLifecycle::Running);

    let task_id = TaskId::new();
    let mut task = ControllerTask::new_periodic(task_id, "PeriodicTask", 0.05, 5);
    task.add_program(Box::new(RoutineProgram::new(
        "AccumRoutine",
        |_dt, sim_time, io| {
            let prev = io.read_output_value("ACCUM").unwrap_or(0.0);
            io.write_output("ACCUM", prev + 1.0, TagQuality::Good, sim_time);
            Ok(())
        },
    )));
    ctrl.add_task(task);

    // Run 5 steps
    for step in 1..=5 {
        ctrl.step(0.05, step as f64 * 0.05).unwrap();
    }
    assert_eq!(ctrl.task(task_id).unwrap().execution_count(), 5);
    assert_eq!(ctrl.io_image().read_output_value("ACCUM"), Some(5.0));

    // Reset controller
    ctrl.reset().unwrap();
    assert_eq!(ctrl.task(task_id).unwrap().execution_count(), 0);
    assert_eq!(ctrl.diagnostics().scan_count, 0);
    assert_eq!(ctrl.diagnostics().task_execution_count, 0);
    assert_eq!(ctrl.io_image().outputs().len(), 0);
    assert_eq!(ctrl.io_image().inputs().len(), 0);
}

// ---------------------------------------------------------------------------
// 8. Full Generic Plant Acceptance Scenario:
//    TANK-A -> FT-A1 -> CONTROL_VALVE-A1 -> TANK-B
//    Controller: CTRL-001 (50ms task, Priority 5, PID Flow Regulation)
//    Protective Interlock: FSHH-A1 (Trips valve if flow > 8.0 kg/s)
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct TwoTankFlowPhysicsSystem {
    tank_a_id: EquipmentId,
    tank_b_id: EquipmentId,
    valve_id: EquipmentId,
    tank_a: AccumulatorVolumeModel,
    tank_b: AccumulatorVolumeModel,
    valve: FlowResistanceModel,
    rho: Density,
}

impl TwoTankFlowPhysicsSystem {
    fn new(tank_a_id: EquipmentId, tank_b_id: EquipmentId, valve_id: EquipmentId) -> Self {
        let rho = Density::new(1000.0).unwrap();
        let total_vol = 2.0;
        let area = 1.0;
        let initial_mass_a = 0.8 * total_vol * rho.value(); // 80% full
        let initial_mass_b = 0.1 * total_vol * rho.value(); // 10% full

        Self {
            tank_a_id,
            tank_b_id,
            valve_id,
            tank_a: AccumulatorVolumeModel::new(total_vol, area, initial_mass_a, rho),
            tank_b: AccumulatorVolumeModel::new(total_vol, area, initial_mass_b, rho),
            valve: FlowResistanceModel::new_from_cv(100.0),
            rho,
        }
    }
}

impl SimulationSystem for TwoTankFlowPhysicsSystem {
    fn name(&self) -> &str {
        "TwoTankFlowPhysicsSystem"
    }

    fn step(
        &mut self,
        ctx: StepContext,
        read_state: &StateReadView<'_>,
        write_state: &mut StateWriteView<'_>,
    ) -> Result<(), SimError> {
        let dt = ctx.dt;

        // Read valve actual position from State[N] (fraction 0.0 to 1.0)
        let opening = if let Ok(v_eq) = read_state.equipment(self.valve_id) {
            v_eq.get_var("opening").unwrap_or(0.0) / 100.0
        } else {
            0.0
        };

        // Pressures
        let p_tank_a = self.tank_a.bottom_pressure(self.rho);
        let p_tank_b = self.tank_b.bottom_pressure(self.rho);

        // Flow from Tank A to Tank B through Control Valve
        let flow = self
            .valve
            .calculate_flow(p_tank_a, p_tank_b, self.rho, opening)
            .unwrap_or(MassFlow::new_unchecked(0.0));

        // Mass balance
        let inflows_a: [MassFlow; 0] = [];
        let outflows_a = [flow];
        let _ = self.tank_a.step(dt, &inflows_a, &outflows_a, self.rho);

        let inflows_b = [flow];
        let outflows_b: [MassFlow; 0] = [];
        let _ = self.tank_b.step(dt, &inflows_b, &outflows_b, self.rho);

        // Update equipment variables in State[N+1]
        let _ = write_state.mutate_equipment(self.tank_a_id, |eq| {
            eq.set_var("level", self.tank_a.liquid_level());
            eq.set_var("mass", self.tank_a.holdup_mass());
        });

        let _ = write_state.mutate_equipment(self.tank_b_id, |eq| {
            eq.set_var("level", self.tank_b.liquid_level());
            eq.set_var("mass", self.tank_b.holdup_mass());
        });

        let _ = write_state.mutate_equipment(self.valve_id, |eq| {
            eq.set_var("flow", flow.value());
        });

        Ok(())
    }
}

/// SimulationSystem wrapper holding an Arc<Mutex<ControlSystem>> for test inspection.
#[derive(Debug, Clone)]
struct SharedControlSystem(Arc<std::sync::Mutex<ControlSystem>>);

impl SimulationSystem for SharedControlSystem {
    fn name(&self) -> &str {
        "ControlSystem"
    }

    fn step(
        &mut self,
        ctx: StepContext,
        read_state: &StateReadView<'_>,
        write_state: &mut StateWriteView<'_>,
    ) -> Result<(), SimError> {
        self.0.lock().unwrap().step(ctx, read_state, write_state)
    }
}

#[test]
fn test_generic_plant_acceptance_scenario_closed_loop() {
    let tank_a_id = EquipmentId::new();
    let tank_b_id = EquipmentId::new();
    let valve_id = EquipmentId::new();

    let ft_a1_tag = TagId::new();
    let valve_out_tag = TagId::new();

    // 1. Instantiate physics
    let physics_sys = TwoTankFlowPhysicsSystem::new(tank_a_id, tank_b_id, valve_id);

    // 2. Instantiate control system
    let ctrl_sys_shared = Arc::new(std::sync::Mutex::new(ControlSystem::new()));

    // Field instrument: FT-A1 measuring flow through valve_id (0 to 10 kg/s)
    let inst_cfg = InstrumentConfig::builder()
        .lrv(0.0)
        .urv(10.0)
        .sensor_lag_tau(0.2)
        .damping_tau(0.1)
        .scan_interval(0.05)
        .build()
        .unwrap();
    ctrl_sys_shared
        .lock()
        .unwrap()
        .register_instrument(valve_id, "flow", ft_a1_tag, inst_cfg, 0.0);

    // Valve actuator: 1.0s full stroke, FailClosed
    let act_cfg = ActuatorConfig {
        stroke_time: 1.0,
        deadband: 0.05,
        fail_action: FailAction::FailClosed,
    };
    ctrl_sys_shared
        .lock()
        .unwrap()
        .register_actuator(valve_id, "opening", act_cfg, 0.0, None);

    // 3. Virtual Controller CTRL-001
    let ctrl_id = ControllerId::new();
    let mut ctrl_001 = VirtualController::new(ctrl_id, "CTRL-001", ControllerConfig::default());
    let _ = ctrl_001.transition_to(ControllerLifecycle::Initializing);
    let _ = ctrl_001.transition_to(ControllerLifecycle::Running);

    // Base periodic task: 50ms (0.05s), Priority 5
    let task_id = TaskId::new();
    let mut task_50ms = ControllerTask::new_periodic(task_id, "Task_50ms", 0.05, 5);

    // Flow control PID: SP = 3.5 kg/s, Reverse acting: flow < SP -> increase opening
    let pid_cfg = PidConfig {
        kp: 15.0,
        ti: 2.0,
        td: 0.1,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Reverse, // Reverse: error = SP - PV -> error > 0 increases output
        sp_tracking_in_manual: false,
        scan_divisor: 1,
    };

    let mut pid_program =
        PidLoopProgram::new("FIC-A1", "FT-A1", "VALVE-A1", Some(valve_id), pid_cfg);
    pid_program.controller_mut().initialize(0.0);
    pid_program.controller_mut().set_mode(PidMode::Auto);
    pid_program.controller_mut().set_sp(3.5); // Target 3.5 kg/s

    task_50ms.add_program(Box::new(pid_program));
    ctrl_001.add_task(task_50ms);

    // Register controller and configure I/O bindings
    {
        let mut cs = ctrl_sys_shared.lock().unwrap();
        cs.register_controller(ctrl_001);
        cs.bind_input(ft_a1_tag, ctrl_id, "FT-A1");
        cs.bind_output_actuator(ctrl_id, "VALVE-A1", valve_id, CommandPriority::Auto);
        cs.bind_output_tag(ctrl_id, "VALVE-A1", valve_out_tag);
    }

    // 4. Setup second independent controller CTRL-002 to verify coexistence
    let ctrl_b_id = ControllerId::new();
    let mut ctrl_002 = VirtualController::new(ctrl_b_id, "CTRL-002", ControllerConfig::default());
    let _ = ctrl_002.transition_to(ControllerLifecycle::Running);
    let mut task_b = ControllerTask::new_periodic(TaskId::new(), "Task_B_100ms", 0.10, 8);
    task_b.add_program(Box::new(RoutineProgram::new(
        "AuxMonitor",
        |_dt, sim_time, io| {
            io.write_output("HEARTBEAT", 1.0, TagQuality::Good, sim_time);
            Ok(())
        },
    )));
    ctrl_002.add_task(task_b);
    ctrl_sys_shared
        .lock()
        .unwrap()
        .register_controller(ctrl_002);

    // 5. Assemble SimulationEngine with 50ms timestep (0.05s)
    let mut engine = SimulationEngine::new(0.05).unwrap();
    engine.add_system(physics_sys);
    engine.add_system(SharedControlSystem(Arc::clone(&ctrl_sys_shared)));

    // Initialize state
    let mut initial_state = StateBuffer::new();
    let mut tank_a_state = EquipmentState::new("running");
    tank_a_state.set_var("level", 0.8);
    initial_state.equipment.insert(tank_a_id, tank_a_state);

    let mut tank_b_state = EquipmentState::new("running");
    tank_b_state.set_var("level", 0.1);
    initial_state.equipment.insert(tank_b_id, tank_b_state);

    let mut valve_state = EquipmentState::new("running");
    valve_state.set_var("opening", 0.0);
    valve_state.set_var("flow", 0.0);
    initial_state.equipment.insert(valve_id, valve_state);

    engine.initialize(initial_state);

    // 6. Run simulation for 200 ticks (10.0 seconds of simulation time)
    for _tick in 1..=200 {
        engine.step().expect("Simulation step must succeed");
    }

    // 7. Verify Acceptance Criteria:
    let final_state = engine.state();
    let valve_opening = final_state
        .equipment(valve_id)
        .unwrap()
        .get_var("opening")
        .unwrap();
    let flow_actual = final_state
        .equipment(valve_id)
        .unwrap()
        .get_var("flow")
        .unwrap();

    // Valve must have opened and regulated flow toward setpoint (3.5 kg/s)
    assert!(
        valve_opening > 10.0,
        "Valve opening ({:.2}%) must have increased from 0% under PID control",
        valve_opening
    );
    assert!(
        (flow_actual - 3.5).abs() < 0.5,
        "Flow ({:.2} kg/s) must be tightly regulated near 3.5 kg/s SP (within ±0.5)",
        flow_actual
    );

    // Verify Tank levels shifted (Tank A drained, Tank B received liquid)
    let level_a = final_state
        .equipment(tank_a_id)
        .unwrap()
        .get_var("level")
        .unwrap();
    let level_b = final_state
        .equipment(tank_b_id)
        .unwrap()
        .get_var("level")
        .unwrap();
    assert!(
        level_a < 0.8,
        "Tank A must have drained from initial 0.8 level"
    );
    assert!(
        level_b > 0.1,
        "Tank B must have accumulated liquid above initial 0.1 level"
    );

    // Retrieve ControlSystem reference from shared pointer
    let cs_guard = ctrl_sys_shared.lock().unwrap();
    let ctrl_1 = cs_guard.controller(ctrl_id).unwrap();
    let ctrl_2 = cs_guard.controller(ctrl_b_id).unwrap();

    // Check task counters
    let t_50ms = ctrl_1.task(task_id).unwrap();
    assert_eq!(
        t_50ms.execution_count(),
        200,
        "50ms task in 10s sim must have executed 200 times"
    );
    assert_eq!(
        t_50ms.overrun_count(),
        0,
        "No overruns under normal execution"
    );

    // Watchdog healthy
    assert_eq!(ctrl_1.diagnostics().watchdog_trips, 0);
    assert_eq!(ctrl_1.lifecycle(), ControllerLifecycle::Running);

    // Controller coexistence check: CTRL-002 executed independently without state interference
    assert_eq!(ctrl_2.io_image().read_output_value("HEARTBEAT"), Some(1.0));
    assert_eq!(
        ctrl_2.io_image().read_output_value("VALVE-A1"),
        None,
        "CTRL-002 must not contain CTRL-001 signals"
    );
}

// ---------------------------------------------------------------------------
// 9. Protective Interlock Action Test within Virtual Controller
// ---------------------------------------------------------------------------

#[test]
fn test_controller_interlock_program_safety_trip() {
    let mut ctrl = VirtualController::with_name("CTRL-SAFETY", ControllerConfig::default());
    let _ = ctrl.transition_to(ControllerLifecycle::Running);

    let valve_id = EquipmentId::new();
    let task_id = TaskId::new();
    let mut task = ControllerTask::new_periodic(task_id, "FastSafety", 0.01, 1);

    // Interlock: High flow trip at 8.0 kg/s with 0.5 kg/s deadband
    let interlock = InterlockProgram::new(
        "FSHH-101",
        "FLOW_INPUT",
        "TRIP_OUTPUT",
        Some(valve_id),
        0.0, // Safe position: closed
        8.0,
        0.5,
        TripDirection::High,
    );
    task.add_program(Box::new(interlock));
    ctrl.add_task(task);

    // Step 1: Normal flow (5.0 kg/s) -> Not tripped
    ctrl.io_image_mut()
        .write_input("FLOW_INPUT", 5.0, TagQuality::Good, 0.01);
    ctrl.step(0.01, 0.01).unwrap();
    assert_eq!(ctrl.io_image().read_output_value("TRIP_OUTPUT"), Some(0.0));

    // Step 2: High flow breach (9.2 kg/s) -> Tripped!
    ctrl.io_image_mut()
        .write_input("FLOW_INPUT", 9.2, TagQuality::Good, 0.02);
    ctrl.step(0.01, 0.02).unwrap();
    assert_eq!(
        ctrl.io_image().read_output_value("TRIP_OUTPUT"),
        Some(1.0),
        "Safety interlock must trip on threshold breach"
    );

    // Step 3: Flow drops to 7.7 kg/s (within deadband) -> Latched!
    ctrl.io_image_mut()
        .write_input("FLOW_INPUT", 7.7, TagQuality::Good, 0.03);
    ctrl.step(0.01, 0.03).unwrap();
    assert_eq!(
        ctrl.io_image().read_output_value("TRIP_OUTPUT"),
        Some(1.0),
        "Safety interlock must remain latched"
    );
}

// ---------------------------------------------------------------------------
// 10. Bit-Identical 10,000-Tick Determinism Replay Verification
// ---------------------------------------------------------------------------

#[test]
fn test_controller_determinism_replay_10k_ticks() {
    let run_scenario = || -> (Vec<f64>, u64, u64) {
        let mut ctrl = VirtualController::with_name("CTRL-DET", ControllerConfig::default());
        let _ = ctrl.transition_to(ControllerLifecycle::Running);

        let t1_id = TaskId::new();
        let mut t1 = ControllerTask::new_periodic(t1_id, "Fast", 0.01, 1);
        let pid_cfg = PidConfig {
            kp: 2.5,
            ti: 1.0,
            td: 0.05,
            derivative_filter_n: 10.0,
            out_min: 0.0,
            out_max: 100.0,
            direction: PidDirection::Reverse,
            sp_tracking_in_manual: false,
            scan_divisor: 1,
        };
        let mut pid_prog = PidLoopProgram::new("PID-FAST", "PV", "OUT", None, pid_cfg);
        pid_prog.controller_mut().initialize(50.0);
        pid_prog.controller_mut().set_mode(PidMode::Auto);
        pid_prog.controller_mut().set_sp(60.0);
        t1.add_program(Box::new(pid_prog));
        ctrl.add_task(t1);

        let dt = 0.01;
        let mut recorded_outputs = Vec::with_capacity(10_000);

        for step in 1..=10_000 {
            let sim_time = step as f64 * dt;
            // Simulated synthetic process reading
            let pv = 50.0 + 10.0 * (step as f64 * 0.005).sin();
            ctrl.io_image_mut()
                .write_input("PV", pv, TagQuality::Good, sim_time);

            ctrl.step(dt, sim_time).unwrap();

            let out = ctrl.io_image().read_output_value("OUT").unwrap_or(0.0);
            recorded_outputs.push(out);
        }

        let task_execs = ctrl.task(t1_id).unwrap().execution_count();
        let total_scans = ctrl.diagnostics().scan_count;
        (recorded_outputs, task_execs, total_scans)
    };

    let (run1_outputs, run1_execs, run1_scans) = run_scenario();
    let (run2_outputs, run2_execs, run2_scans) = run_scenario();

    assert_eq!(run1_execs, 10_000);
    assert_eq!(run2_execs, 10_000);
    assert_eq!(run1_scans, 10_000);
    assert_eq!(run2_scans, 10_000);
    assert_eq!(
        run1_outputs.len(),
        run2_outputs.len(),
        "Both runs must record exactly 10k outputs"
    );

    for i in 0..10_000 {
        assert_eq!(
            run1_outputs[i].to_bits(),
            run2_outputs[i].to_bits(),
            "Bit-identical mismatch at tick {}: run1={} vs run2={}",
            i,
            run1_outputs[i],
            run2_outputs[i]
        );
    }
}
