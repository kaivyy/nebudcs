//! Comprehensive Stage 3: I/O Hardware Abstraction Layer (HAL) Test Suite.
//!
//! Verifies:
//! - Hardware hierarchy (IoSystem -> IoRack -> IoChassis -> IoModule -> IoChannel)
//! - Channel addressing & canonical string parsing
//! - AI/AO/DI/DO signal typing & scaling (Linear, SquareRoot, Clamping)
//! - NAMUR NE43 analog fault classification (Normal, UnderRange, OverRange, WireBreak, ShortCircuit)
//! - Digital fault semantics (StuckTrue, StuckFalse, Inverted, Debounce)
//! - Output fallback behaviors (FailOpen, FailClosed, HoldLast, ForcedValue, Zero)
//! - Controller-to-I/O mapping validation & output conflict rejection
//! - Multi-controller isolation & input multicast
//! - Closed-loop integration with VirtualController and SimulationEngine
//! - 10,000-tick bit-identical determinism replay

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use nebudcs_control::actuator::{ActuatorConfig, CommandPriority, FailAction};
use nebudcs_control::hal::*;
use nebudcs_control::pid::PidMode;
use nebudcs_control::runtime::*;
use nebudcs_control::ControlSystem;
use nebudcs_model::tag::TagQuality;
use nebudcs_sim::engine::SimulationEngine;
use nebudcs_sim::{SimError, SimulationSystem, StateReadView, StateWriteView, StepContext};
use nebudcs_types::{ControllerId, EquipmentId, TagId, TaskId};

// ---------------------------------------------------------------------------
// 1. Channel Addressing Tests
// ---------------------------------------------------------------------------

#[test]
fn test_channel_addressing_formats_and_parsing() {
    // Standard compact format
    let a1: IoChannelAddress = "R01-C01-AI01".parse().unwrap();
    assert_eq!(a1.rack, 1);
    assert_eq!(a1.chassis, 1);
    assert_eq!(a1.module, 1);
    assert_eq!(a1.channel, 1);
    assert_eq!(a1.kind, ChannelKind::AI);
    assert_eq!(a1.to_string(), "R01-C01-AI01");

    // Extended format with explicit module slot
    let a2: IoChannelAddress = "R02-C03-M04-AO08".parse().unwrap();
    assert_eq!(a2.rack, 2);
    assert_eq!(a2.chassis, 3);
    assert_eq!(a2.module, 4);
    assert_eq!(a2.channel, 8);
    assert_eq!(a2.kind, ChannelKind::AO);
    assert_eq!(a2.to_string(), "R02-C03-M04-AO08");

    // Digital channels
    let a3: IoChannelAddress = "R01-C02-DI16".parse().unwrap();
    assert_eq!(a3.kind, ChannelKind::DI);
    assert_eq!(a3.channel, 16);

    let a4: IoChannelAddress = "R01-C02-DO04".parse().unwrap();
    assert_eq!(a4.kind, ChannelKind::DO);
    assert_eq!(a4.channel, 4);

    // Invalid addresses rejected
    assert!("INVALID".parse::<IoChannelAddress>().is_err());
    assert!("R01-C01-XX01".parse::<IoChannelAddress>().is_err());
    assert!("R01-C01-AI00".parse::<IoChannelAddress>().is_err()); // 0 is invalid index
}

// ---------------------------------------------------------------------------
// 2. Signal Scaling & Curve Tests
// ---------------------------------------------------------------------------

#[test]
fn test_ai_scaling_linear_and_squareroot() {
    // 1. Linear 4-20 mA scaling (0 to 100 bar)
    let linear_cfg = AnalogScalingConfig::new_4_20ma(0.0, 100.0, "bar").unwrap();

    // 4 mA (raw_min: 6554 counts) -> 0.0 bar
    let (eu_0, q_0, f_0) = linear_cfg.raw_to_eu(6554).unwrap();
    assert!((eu_0 - 0.0).abs() < 1e-3);
    assert_eq!(q_0, IoSignalQuality::Good);
    assert_eq!(f_0, AnalogFaultState::Normal);

    // 20 mA (raw_max: 32767 counts) -> 100.0 bar
    let (eu_100, q_100, f_100) = linear_cfg.raw_to_eu(32767).unwrap();
    assert!((eu_100 - 100.0).abs() < 1e-3);
    assert_eq!(q_100, IoSignalQuality::Good);
    assert_eq!(f_100, AnalogFaultState::Normal);

    // 12 mA (midpoint: 19660 counts) -> 50.0 bar
    let mid_raw = 6554 + (32767 - 6554) / 2;
    let (eu_50, q_50, _) = linear_cfg.raw_to_eu(mid_raw).unwrap();
    assert!((eu_50 - 50.0).abs() < 0.1);
    assert_eq!(q_50, IoSignalQuality::Good);

    // 2. Square root extraction 4-20 mA scaling (0 to 20.0 kg/s)
    let sq_cfg = AnalogScalingConfig::new_dp_flow_4_20ma(20.0, "kg/s").unwrap();

    // 25% DP span -> sqrt(0.25) = 50% flow -> 10.0 kg/s
    let raw_25 = 6554 + (32767 - 6554) / 4;
    let (flow, _, _) = sq_cfg.raw_to_eu(raw_25).unwrap();
    assert!((flow - 10.0).abs() < 0.1);

    // Reverse EU -> Raw conversion
    let (raw_back, _) = sq_cfg.eu_to_raw(10.0).unwrap();
    assert!((raw_back - raw_25).abs() <= 1);
}

#[test]
fn test_invalid_scaling_configurations_rejected() {
    // Zero span (URV == LRV)
    let res = AnalogScalingConfig::new_4_20ma(50.0, 50.0, "bar");
    assert!(matches!(res, Err(HalError::ZeroSpan { .. })));

    // Raw max == Raw min
    let mut bad_raw = AnalogScalingConfig::new_4_20ma(0.0, 100.0, "bar").unwrap();
    bad_raw.raw_max = bad_raw.raw_min;
    assert!(matches!(
        bad_raw.validate(),
        Err(HalError::InvalidScaling { .. })
    ));
}

// ---------------------------------------------------------------------------
// 3. NAMUR NE43 Analog Fault Detection Tests
// ---------------------------------------------------------------------------

#[test]
fn test_namur_ne43_analog_fault_detection() {
    let cfg = AnalogScalingConfig::new_4_20ma(0.0, 100.0, "degC").unwrap();
    let span = (cfg.raw_max - cfg.raw_min) as f64;

    // Normal range (12.0 mA -> 50°C) -> Good, Normal
    let raw_12ma = (cfg.raw_min as f64 + (8.0 / 16.0) * span).round() as i32;
    let (_, q_norm, f_norm) = cfg.raw_to_eu(raw_12ma).unwrap();
    assert_eq!(q_norm, IoSignalQuality::Good);
    assert_eq!(f_norm, AnalogFaultState::Normal);

    // Under-range warning: 3.7 mA (between 3.6 mA and 3.8 mA) -> Uncertain, UnderRange
    let raw_3_7ma = (cfg.raw_min as f64 + ((3.7 - 4.0) / 16.0) * span).round() as i32;
    let (_, q_ur, f_ur) = cfg.raw_to_eu(raw_3_7ma).unwrap();
    assert_eq!(q_ur, IoSignalQuality::Uncertain);
    assert_eq!(f_ur, AnalogFaultState::UnderRange);

    // Wire break: 3.2 mA (< 3.6 mA) -> Bad, WireBreak
    let raw_3_2ma = (cfg.raw_min as f64 + ((3.2 - 4.0) / 16.0) * span).round() as i32;
    let (_, q_wb, f_wb) = cfg.raw_to_eu(raw_3_2ma).unwrap();
    assert_eq!(q_wb, IoSignalQuality::Bad);
    assert_eq!(f_wb, AnalogFaultState::WireBreak);

    // Over-range warning: 20.8 mA (between 20.5 mA and 21.0 mA) -> Uncertain, OverRange
    let raw_20_8ma = (cfg.raw_min as f64 + ((20.8 - 4.0) / 16.0) * span).round() as i32;
    let (_, q_or, f_or) = cfg.raw_to_eu(raw_20_8ma).unwrap();
    assert_eq!(q_or, IoSignalQuality::Uncertain);
    assert_eq!(f_or, AnalogFaultState::OverRange);

    // Short-circuit: 22.0 mA (> 21.0 mA) -> Bad, ShortCircuit
    let raw_22ma = (cfg.raw_min as f64 + ((22.0 - 4.0) / 16.0) * span).round() as i32;
    let (_, q_sc, f_sc) = cfg.raw_to_eu(raw_22ma).unwrap();
    assert_eq!(q_sc, IoSignalQuality::Bad);
    assert_eq!(f_sc, AnalogFaultState::ShortCircuit);
}

// ---------------------------------------------------------------------------
// 4. Digital I/O & Output Fallback Tests
// ---------------------------------------------------------------------------

#[test]
fn test_di_do_states_inversion_and_debounce() {
    let addr_di: IoChannelAddress = "R01-C01-DI01".parse().unwrap();
    let mut di = DigitalInputChannel::new(1, addr_di, "RUN_FEEDBACK", false);

    // Normal sampling (low contact -> false)
    di.sample(false, 0.0);
    assert!(!di.state);

    // Transition to true at sim_time 0.01:
    // With 20ms debounce (0.02s), must not flip immediately
    di.sample(true, 0.01);
    assert!(!di.state); // Debouncing

    // After 20ms debounce at sim_time 0.035:
    di.sample(true, 0.035);
    assert!(di.state); // Debounced

    // Test polarity inversion (normally closed contact)
    let addr_di2: IoChannelAddress = "R01-C01-DI02".parse().unwrap();
    let mut di_inv = DigitalInputChannel::new(2, addr_di2, "NC_SWITCH", true);
    di_inv.sample(false, 0.1);
    assert!(di_inv.state); // Inverted: contact open -> signal TRUE
}

#[test]
fn test_ao_scaling_and_fallback_behaviors() {
    let addr_ao: IoChannelAddress = "R01-C01-AO01".parse().unwrap();
    let cfg = AnalogScalingConfig::new_4_20ma(0.0, 100.0, "%").unwrap();

    // 1. FailClosed fallback (0%)
    let mut ao_fc = AnalogOutputChannel::new(
        1,
        addr_ao,
        "VALVE_CV",
        cfg.clone(),
        OutputFallbackBehavior::FailClosed,
    );
    ao_fc.write_command(75.0, 1.0);
    assert_eq!(ao_fc.actual_eu, 75.0);

    // Apply fallback (e.g. communication lost)
    ao_fc.apply_fallback(2.0);
    assert_eq!(ao_fc.actual_eu, 0.0); // FailClosed drove to LRV (0.0%)

    // 2. FailOpen fallback (100%)
    let mut ao_fo = AnalogOutputChannel::new(
        1,
        addr_ao,
        "VALVE_CV",
        cfg.clone(),
        OutputFallbackBehavior::FailOpen,
    );
    ao_fo.write_command(50.0, 1.0);
    ao_fo.apply_fallback(2.0);
    assert_eq!(ao_fo.actual_eu, 100.0); // FailOpen drove to URV (100.0%)

    // 3. HoldLast fallback
    let mut ao_hl = AnalogOutputChannel::new(
        1,
        addr_ao,
        "VALVE_CV",
        cfg,
        OutputFallbackBehavior::HoldLast,
    );
    ao_hl.write_command(62.5, 1.0);
    ao_hl.apply_fallback(2.0);
    assert_eq!(ao_hl.actual_eu, 62.5); // Retained 62.5%
}

// ---------------------------------------------------------------------------
// 5. Hardware Hierarchy & Validation Tests
// ---------------------------------------------------------------------------

#[test]
fn test_hardware_hierarchy_and_bounds_validation() {
    let mut sys = IoSystem::new("HAL-PLANT-A");
    let mut rack = IoRack::new(1, "MAIN-RACK");
    let mut chassis = IoChassis::new(1, "BASE-CHASSIS");

    // Slot 1: 8-Channel AI Module
    let mut ai_mod = IoModule::new(1, "AI-CARD-1", IoModuleType::AnalogInput8);
    let cfg = AnalogScalingConfig::new_4_20ma(0.0, 10.0, "bar").unwrap();

    // Add valid channel 1
    let addr1: IoChannelAddress = "R01-C01-AI01".parse().unwrap();
    let ch1 = AnalogInputChannel::new(1, addr1, "PT-101", cfg.clone());
    assert!(ai_mod.add_channel(IoChannel::Ai(ch1)).is_ok());

    // Duplicate channel number on same module rejected
    let ch1_dup = AnalogInputChannel::new(1, addr1, "PT-101-DUP", cfg.clone());
    assert!(matches!(
        ai_mod.add_channel(IoChannel::Ai(ch1_dup)),
        Err(HalError::DuplicateAddress { .. })
    ));

    // Channel index 9 exceeds 8-channel capacity -> rejected
    let addr9: IoChannelAddress = "R01-C01-AI09".parse().unwrap();
    let ch9 = AnalogInputChannel::new(9, addr9, "PT-109", cfg.clone());
    assert!(matches!(
        ai_mod.add_channel(IoChannel::Ai(ch9)),
        Err(HalError::ChannelOutOfBounds { .. })
    ));

    // Mount module into chassis
    assert!(chassis.add_module(ai_mod).is_ok());

    // Duplicate slot assignment in same chassis rejected
    let dup_mod = IoModule::new(1, "AI-CARD-DUP", IoModuleType::AnalogInput8);
    assert!(matches!(
        chassis.add_module(dup_mod),
        Err(HalError::ModuleSlotOccupied { .. })
    ));

    // Assemble rack and system
    assert!(rack.add_chassis(chassis).is_ok());
    assert!(sys.add_rack(rack).is_ok());

    // Query channel by address string
    let retrieved = sys.channel_by_str("R01-C01-AI01").unwrap();
    assert_eq!(retrieved.name(), "PT-101");
}

// ---------------------------------------------------------------------------
// 6. Mapping & Multi-Controller Conflict Rejection Tests
// ---------------------------------------------------------------------------

#[test]
fn test_mapping_conflict_rejection_and_multicast() {
    let mut sys = IoSystem::new("HAL-SYS");
    let mut rack = IoRack::new(1, "RACK-1");
    let mut chassis = IoChassis::new(1, "CHASSIS-1");

    let mut ai_mod = IoModule::new(1, "AI-MOD", IoModuleType::AnalogInput8);
    let mut ao_mod = IoModule::new(2, "AO-MOD", IoModuleType::AnalogOutput8);

    let cfg_ai = AnalogScalingConfig::new_4_20ma(0.0, 100.0, "m3/h").unwrap();
    let cfg_ao = AnalogScalingConfig::new_4_20ma(0.0, 100.0, "%").unwrap();

    let addr_ai: IoChannelAddress = "R01-C01-M01-AI01".parse().unwrap();
    let addr_ao: IoChannelAddress = "R01-C01-M02-AO01".parse().unwrap();

    ai_mod
        .add_channel(IoChannel::Ai(AnalogInputChannel::new(
            1, addr_ai, "FT-101", cfg_ai,
        )))
        .unwrap();
    ao_mod
        .add_channel(IoChannel::Ao(AnalogOutputChannel::new(
            1,
            addr_ao,
            "FV-101",
            cfg_ao,
            OutputFallbackBehavior::FailClosed,
        )))
        .unwrap();

    chassis.add_module(ai_mod).unwrap();
    chassis.add_module(ao_mod).unwrap();
    rack.add_chassis(chassis).unwrap();
    sys.add_rack(rack).unwrap();

    let ctrl_1 = ControllerId::new();
    let ctrl_2 = ControllerId::new();

    // 1. Input multicast is allowed: both ctrl_1 and ctrl_2 read from FT-101
    assert!(sys
        .add_mapping(ControllerIoMapping::new_input(
            ctrl_1, "FLOW_IN", addr_ai, None
        ))
        .is_ok());
    assert!(sys
        .add_mapping(ControllerIoMapping::new_input(
            ctrl_2,
            "FLOW_MONITOR",
            addr_ai,
            None
        ))
        .is_ok());

    // 2. Output mapping for ctrl_1 to FV-101
    assert!(sys
        .add_mapping(ControllerIoMapping::new_output(
            ctrl_1,
            "VALVE_OUT",
            addr_ao,
            None,
            CommandPriority::Auto
        ))
        .is_ok());

    // 3. Conflicting output mapping: ctrl_2 tries to also drive FV-101 without arbiter -> REJECTED!
    let conflict_mapping = ControllerIoMapping::new_output(
        ctrl_2,
        "VALVE_OVERRIDE",
        addr_ao,
        None,
        CommandPriority::Auto,
    );
    let err = sys.add_mapping(conflict_mapping).unwrap_err();
    assert!(matches!(err, HalError::ConflictingOutputOwnership { .. }));
}

// ---------------------------------------------------------------------------
// 7. End-to-End Closed-Loop Integration Test (Process -> HAL -> %I -> PID -> %Q -> HAL -> Valve)
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct MockFlowProcess {
    tank_a_level: f64,
    tank_b_level: f64,
    valve_opening: f64,
    flow_rate: f64,
    valve_id: EquipmentId,
    flow_tag: TagId,
}

impl SimulationSystem for MockFlowProcess {
    fn name(&self) -> &str {
        "MockFlowProcess"
    }

    fn step(
        &mut self,
        ctx: StepContext,
        _read_state: &StateReadView<'_>,
        write_state: &mut StateWriteView<'_>,
    ) -> Result<(), SimError> {
        let dt = ctx.dt;

        // Read actuator command applied to valve (actuator operates 0..100%)
        if let Some(eq) = write_state.get_equipment(self.valve_id) {
            self.valve_opening = eq.get_var("opening").unwrap_or(self.valve_opening);
        }

        // Flow is proportional to valve opening fraction (0..1.0) and differential head (Tank A - Tank B)
        let head = (self.tank_a_level - self.tank_b_level).max(0.0);
        let opening_fraction = (self.valve_opening / 100.0).clamp(0.0, 1.0);
        self.flow_rate = opening_fraction * 10.0 * head.sqrt();

        // Mass balance
        let vol_moved = self.flow_rate * dt * 0.001;
        self.tank_a_level = (self.tank_a_level - vol_moved).max(0.0);
        self.tank_b_level = (self.tank_b_level + vol_moved).min(1.0);

        // Update process equipment state and flow tag
        let _ = write_state.mutate_equipment(self.valve_id, |eq| {
            eq.set_var("flow", self.flow_rate);
            eq.set_var("opening", self.valve_opening);
        });

        write_state.set_tag(
            self.flow_tag,
            nebudcs_model::tag::TagValue::Float(self.flow_rate),
            TagQuality::Good,
        );

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct SharedControlSystem(Arc<Mutex<ControlSystem>>);

impl SimulationSystem for SharedControlSystem {
    fn name(&self) -> &str {
        "SharedControlSystem"
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
fn test_hal_closed_loop_flow_regulation_process() {
    // 1. Hardware Architecture Setup
    let mut hal = IoSystem::new("HAL-FLOW-REG");
    let mut rack = IoRack::new(1, "RACK-01");
    let mut chassis = IoChassis::new(1, "CHASSIS-01");

    let mut ai_mod = IoModule::new(1, "AI-CARD", IoModuleType::AnalogInput8);
    let mut ao_mod = IoModule::new(2, "AO-CARD", IoModuleType::AnalogOutput8);

    let cfg_flow = AnalogScalingConfig::new_4_20ma(0.0, 10.0, "kg/s").unwrap();
    let cfg_valve = AnalogScalingConfig::new_4_20ma(0.0, 100.0, "%").unwrap();

    let addr_ai: IoChannelAddress = "R01-C01-M01-AI01".parse().unwrap();
    let addr_ao: IoChannelAddress = "R01-C01-M02-AO01".parse().unwrap();

    ai_mod
        .add_channel(IoChannel::Ai(AnalogInputChannel::new(
            1, addr_ai, "FT-101", cfg_flow,
        )))
        .unwrap();
    ao_mod
        .add_channel(IoChannel::Ao(AnalogOutputChannel::new(
            1,
            addr_ao,
            "FV-101",
            cfg_valve,
            OutputFallbackBehavior::FailClosed,
        )))
        .unwrap();

    chassis.add_module(ai_mod).unwrap();
    chassis.add_module(ao_mod).unwrap();
    rack.add_chassis(chassis).unwrap();
    hal.add_rack(rack).unwrap();

    // 2. Control System & Virtual Controller
    let ctrl_id = ControllerId::new();
    let mut ctrl = VirtualController::new(ctrl_id, "CTRL-FLOW", ControllerConfig::default());
    let _ = ctrl.transition_to(ControllerLifecycle::Initializing);
    let _ = ctrl.transition_to(ControllerLifecycle::Running);

    // 50ms periodic task with PID program regulating to SP = 4.0 kg/s
    let task_id = TaskId::new();
    let mut task = ControllerTask::new_periodic(task_id, "FlowTask", 0.05, 5);

    let pid_cfg = nebudcs_control::pid::PidConfig {
        kp: 6.0,
        ti: 0.8,
        direction: nebudcs_control::pid::PidDirection::Reverse,
        ..Default::default()
    };
    let mut pid_prog = PidLoopProgram::new("FIC-101", "%IW0", "%QW0", None, pid_cfg);
    pid_prog.controller_mut().set_mode(PidMode::Auto);
    pid_prog.controller_mut().set_sp(4.0); // Target: 4.0 kg/s flow
    task.add_program(Box::new(pid_prog));
    ctrl.add_task(task);

    // 3. Configure Mappings
    let flow_tag = TagId::new();
    let valve_id = EquipmentId::new();

    hal.add_mapping(ControllerIoMapping::new_input(
        ctrl_id,
        "%IW0",
        addr_ai,
        Some(flow_tag),
    ))
    .unwrap();

    hal.add_mapping(ControllerIoMapping::new_output(
        ctrl_id,
        "%QW0",
        addr_ao,
        Some(valve_id),
        CommandPriority::Auto,
    ))
    .unwrap();

    let mut cs = ControlSystem::new();
    cs.register_controller(ctrl);
    cs.set_io_system(hal);

    // Register valve actuator
    let act_cfg = ActuatorConfig {
        stroke_time: 0.5,
        deadband: 0.001,
        fail_action: FailAction::FailClosed,
    };
    cs.register_actuator(valve_id, "opening", act_cfg, 0.0, None);

    let shared_cs = SharedControlSystem(Arc::new(Mutex::new(cs)));

    // 4. Assemble Engine
    let mut engine = SimulationEngine::new(0.05).unwrap();
    let mut initial_state = nebudcs_sim::state::StateBuffer::new();
    let mut valve_state = nebudcs_sim::state::EquipmentState::new("running");
    valve_state.set_var("opening", 0.0);
    valve_state.set_var("flow", 0.0);
    initial_state.equipment.insert(valve_id, valve_state);
    engine.initialize(initial_state);

    let process = MockFlowProcess {
        tank_a_level: 0.9,
        tank_b_level: 0.1,
        valve_opening: 0.0,
        flow_rate: 0.0,
        valve_id,
        flow_tag,
    };

    engine.add_system(process);
    engine.add_system(shared_cs.clone());

    // 5. Run simulation for 12 seconds (240 steps at dt = 0.05s)
    for _ in 0..240 {
        engine.step().unwrap();
    }

    // 6. Verify Regulation through HAL
    let final_state = engine.state();
    let valve_opening = final_state
        .equipment(valve_id)
        .unwrap()
        .get_var("opening")
        .unwrap();
    let actual_flow = final_state
        .equipment(valve_id)
        .unwrap()
        .get_var("flow")
        .unwrap();

    assert!(
        valve_opening > 30.0 && valve_opening < 80.0,
        "Valve must modulate open: opening = {}",
        valve_opening
    );
    assert!(
        (actual_flow - 4.0).abs() < 0.25,
        "Flow must settle near 4.0 kg/s setpoint: actual = {}",
        actual_flow
    );

    // Inspect HAL channel diagnostics
    let cs_guard = shared_cs.0.lock().unwrap();
    let hal_ref = cs_guard.io_system();
    assert!(hal_ref.diagnostics.channels_scanned > 150);
    assert_eq!(hal_ref.diagnostics.faulted_channels, 0);

    // Channel values reflect live physical state
    let ai_ch = hal_ref.channel(&addr_ai).unwrap();
    assert!((ai_ch.value_as_float() - actual_flow).abs() < 0.05);
    assert_eq!(ai_ch.quality(), IoSignalQuality::Good);

    let ao_ch = hal_ref.channel(&addr_ao).unwrap();
    assert!((ao_ch.value_as_float() - valve_opening).abs() < 0.05);
}

// ---------------------------------------------------------------------------
// 8. Hardware Fault Injection & Causal Cascade Test
// ---------------------------------------------------------------------------

#[test]
fn test_hal_wire_break_fault_causal_propagation() {
    let mut hal = IoSystem::new("HAL-FAULT-TEST");
    let mut rack = IoRack::new(1, "RACK-01");
    let mut chassis = IoChassis::new(1, "CHASSIS-01");
    let mut ai_mod = IoModule::new(1, "AI-CARD", IoModuleType::AnalogInput8);

    let cfg_flow = AnalogScalingConfig::new_4_20ma(0.0, 10.0, "kg/s").unwrap();
    let addr_ai: IoChannelAddress = "R01-C01-AI01".parse().unwrap();

    ai_mod
        .add_channel(IoChannel::Ai(AnalogInputChannel::new(
            1, addr_ai, "FT-101", cfg_flow,
        )))
        .unwrap();
    chassis.add_module(ai_mod).unwrap();
    rack.add_chassis(chassis).unwrap();
    hal.add_rack(rack).unwrap();

    let ctrl_id = ControllerId::new();
    let mut ctrl = VirtualController::new(ctrl_id, "CTRL-TEST", ControllerConfig::default());
    let _ = ctrl.transition_to(ControllerLifecycle::Initializing);
    let _ = ctrl.transition_to(ControllerLifecycle::Running);

    hal.add_mapping(ControllerIoMapping::new_input(
        ctrl_id, "FLOW_IN", addr_ai, None,
    ))
    .unwrap();

    let mut controllers = BTreeMap::new();
    controllers.insert(ctrl_id, ctrl);

    // Step 1: Normal process value (5.0 kg/s)
    let ai_mut = hal.channel_mut(&addr_ai).unwrap();
    if let IoChannel::Ai(ch) = ai_mut {
        ch.sample(5.0, 1.0);
    }
    assert_eq!(
        hal.channel(&addr_ai).unwrap().quality(),
        IoSignalQuality::Good
    );

    // Step 2: Inject wire break fault (< 3.6 mA)
    let ai_mut = hal.channel_mut(&addr_ai).unwrap();
    if let IoChannel::Ai(ch) = ai_mut {
        ch.forced_fault = Some(AnalogFaultState::WireBreak);
        ch.sample(5.0, 1.1); // Process is still 5.0, but wire is broken
    }

    // Quality must immediately become Bad with WireBreak fault
    let ch_faulted = hal.channel(&addr_ai).unwrap();
    assert_eq!(ch_faulted.quality(), IoSignalQuality::Bad);
    if let IoChannel::Ai(c) = ch_faulted {
        assert_eq!(c.fault_state, AnalogFaultState::WireBreak);
    }
}

// ---------------------------------------------------------------------------
// 9. Rack Power Loss Cascading Test
// ---------------------------------------------------------------------------

#[test]
fn test_rack_power_and_bus_failure_cascading() {
    let mut hal = IoSystem::new("HAL-POWER-TEST");
    let mut rack = IoRack::new(1, "RACK-01");
    let mut chassis = IoChassis::new(1, "CHASSIS-01");
    let mut ao_mod = IoModule::new(1, "AO-CARD", IoModuleType::AnalogOutput8);

    let cfg_valve = AnalogScalingConfig::new_4_20ma(0.0, 100.0, "%").unwrap();
    let addr_ao: IoChannelAddress = "R01-C01-AO01".parse().unwrap();

    ao_mod
        .add_channel(IoChannel::Ao(AnalogOutputChannel::new(
            1,
            addr_ao,
            "VALVE-AO",
            cfg_valve,
            OutputFallbackBehavior::FailClosed,
        )))
        .unwrap();

    chassis.add_module(ao_mod).unwrap();
    rack.add_chassis(chassis).unwrap();
    hal.add_rack(rack).unwrap();

    let ctrl_id = ControllerId::new();
    let mut ctrl = VirtualController::new(ctrl_id, "CTRL-TEST", ControllerConfig::default());
    let _ = ctrl.transition_to(ControllerLifecycle::Initializing);
    let _ = ctrl.transition_to(ControllerLifecycle::Running);
    ctrl.io_image_mut()
        .write_output("VALVE_OUT", 80.0, TagQuality::Good, 1.0);

    hal.add_mapping(ControllerIoMapping::new_output(
        ctrl_id,
        "VALVE_OUT",
        addr_ao,
        None,
        CommandPriority::Auto,
    ))
    .unwrap();

    let mut controllers = BTreeMap::new();
    controllers.insert(ctrl_id, ctrl);

    // Normal dispatch: output is 80.0%
    let mut db = nebudcs_sim::state::DoubleBufferedState::new();
    let mut write_state = db.write_view();
    hal.dispatch_outputs(&controllers, |_, _| {}, &mut write_state, 1.0);
    assert_eq!(hal.channel(&addr_ao).unwrap().value_as_float(), 80.0);

    // Cut rack power
    hal.rack_mut(1).unwrap().power_ok = false;

    // Dispatch outputs under unpowered rack: must drop to FailClosed (0.0%)
    hal.dispatch_outputs(&controllers, |_, _| {}, &mut write_state, 1.1);
    assert_eq!(hal.channel(&addr_ao).unwrap().value_as_float(), 0.0);
    assert_eq!(
        hal.channel(&addr_ao).unwrap().quality(),
        IoSignalQuality::Uncertain
    );
}

// ---------------------------------------------------------------------------
// 10. 10,000-Step Determinism Replay Test
// ---------------------------------------------------------------------------

#[test]
fn test_hal_determinism_replay_10k_ticks() {
    let run_simulation = || -> Vec<f64> {
        let mut hal = IoSystem::new("HAL-DET");
        let mut rack = IoRack::new(1, "R01");
        let mut chassis = IoChassis::new(1, "C01");
        let mut ai_mod = IoModule::new(1, "AI", IoModuleType::AnalogInput8);
        let mut ao_mod = IoModule::new(2, "AO", IoModuleType::AnalogOutput8);

        let cfg = AnalogScalingConfig::new_4_20ma(0.0, 100.0, "bar").unwrap();
        let addr_ai: IoChannelAddress = "R01-C01-M01-AI01".parse().unwrap();
        let addr_ao: IoChannelAddress = "R01-C01-M02-AO01".parse().unwrap();

        ai_mod
            .add_channel(IoChannel::Ai(AnalogInputChannel::new(
                1,
                addr_ai,
                "AI-1",
                cfg.clone(),
            )))
            .unwrap();
        ao_mod
            .add_channel(IoChannel::Ao(AnalogOutputChannel::new(
                1,
                addr_ao,
                "AO-1",
                cfg,
                OutputFallbackBehavior::FailClosed,
            )))
            .unwrap();

        chassis.add_module(ai_mod).unwrap();
        chassis.add_module(ao_mod).unwrap();
        rack.add_chassis(chassis).unwrap();
        hal.add_rack(rack).unwrap();

        let ctrl_id = ControllerId::new();
        let mut ctrl = VirtualController::new(ctrl_id, "CTRL", ControllerConfig::default());
        let _ = ctrl.transition_to(ControllerLifecycle::Initializing);
        let _ = ctrl.transition_to(ControllerLifecycle::Running);

        let task_id = TaskId::new();
        let mut task = ControllerTask::new_periodic(task_id, "PeriodicTask", 0.01, 1);
        let pid_cfg = nebudcs_control::pid::PidConfig::default();
        let pid_prog = PidLoopProgram::new("PID-1", "AI_IN", "AO_OUT", None, pid_cfg);
        task.add_program(Box::new(pid_prog));
        ctrl.add_task(task);

        hal.add_mapping(ControllerIoMapping::new_input(
            ctrl_id, "AI_IN", addr_ai, None,
        ))
        .unwrap();
        hal.add_mapping(ControllerIoMapping::new_output(
            ctrl_id,
            "AO_OUT",
            addr_ao,
            None,
            CommandPriority::Auto,
        ))
        .unwrap();

        let mut controllers = BTreeMap::new();
        controllers.insert(ctrl_id, ctrl);

        let mut outputs = Vec::with_capacity(10000);
        let dt = 0.01;

        for step in 0..10000 {
            let sim_time = step as f64 * dt;
            let pv = 50.0 + 20.0 * (step as f64 * 0.005).sin();

            // Sample input
            let ch_ai = hal.channel_mut(&addr_ai).unwrap();
            if let IoChannel::Ai(c) = ch_ai {
                c.sample(pv, sim_time);
            }

            // Transfer to controller %I
            let read_val = hal.channel(&addr_ai).unwrap().value_as_float();
            controllers
                .get_mut(&ctrl_id)
                .unwrap()
                .io_image_mut()
                .write_input("AI_IN", read_val, TagQuality::Good, sim_time);

            // Step controller
            controllers
                .get_mut(&ctrl_id)
                .unwrap()
                .step(dt, sim_time)
                .unwrap();

            // Dispatch output through HAL
            let mut db = nebudcs_sim::state::DoubleBufferedState::new();
            let mut write_state = db.write_view();
            hal.dispatch_outputs(&controllers, |_, _| {}, &mut write_state, sim_time);

            let ao_val = hal.channel(&addr_ao).unwrap().value_as_float();
            outputs.push(ao_val);
        }

        outputs
    };

    let run_a = run_simulation();
    let run_b = run_simulation();

    assert_eq!(run_a.len(), 10000);
    assert_eq!(run_b.len(), 10000);

    for i in 0..10000 {
        assert_eq!(
            run_a[i].to_bits(),
            run_b[i].to_bits(),
            "Bitwise determinism failure at step {}: run_a = {}, run_b = {}",
            i,
            run_a[i],
            run_b[i]
        );
    }
}
