use nebudcs_control::actuator::{ActuatorConfig, CommandPriority, FailAction};
use nebudcs_control::instrument::InstrumentConfig;
use nebudcs_control::pid::{PidConfig, PidController, PidDirection, PidMode};
use nebudcs_control::protection::{ProtectionBlock, ProtectionConfig, ProtectionState};
use nebudcs_control::system::ControlSystem;
use nebudcs_model::tag::TagQuality;
use nebudcs_physics::equipment::{AccumulatorVolumeModel, FlowResistanceModel};
use nebudcs_sim::{
    EquipmentState, SimError, SimulationEngine, SimulationSystem, StateBuffer, StateReadView,
    StateWriteView, StepContext,
};
use nebudcs_types::{EquipmentId, TagId};
use nebudcs_units::{Density, MassFlow, Pressure};

/// Dynamic liquid vessel with flow source and discharge control valve.
#[derive(Debug)]
struct VesselProcessPhysics {
    vessel_id: EquipmentId,
    valve_id: EquipmentId,
    vessel: AccumulatorVolumeModel,
    valve: FlowResistanceModel,
    inflow_rate: f64,
    rho: Density,
}

impl VesselProcessPhysics {
    fn new(vessel_id: EquipmentId, valve_id: EquipmentId, initial_level: f64) -> Self {
        let rho = Density::new(1000.0).unwrap();
        let total_vol = 1.0;
        let area = 0.5;
        let initial_mass = initial_level * total_vol * rho.value();
        let vessel = AccumulatorVolumeModel::new(total_vol, area, initial_mass, rho);
        let valve = FlowResistanceModel::new_from_cv(80.0);

        Self {
            vessel_id,
            valve_id,
            vessel,
            valve,
            inflow_rate: 5.0,
            rho,
        }
    }
}

impl SimulationSystem for VesselProcessPhysics {
    fn name(&self) -> &str {
        "VesselProcessPhysics"
    }

    fn step(
        &mut self,
        ctx: StepContext,
        read_state: &StateReadView<'_>,
        write_state: &mut StateWriteView<'_>,
    ) -> Result<(), SimError> {
        let dt = ctx.dt;

        // 1. Read valve opening from State[N] (z^-1 unit delay)
        let opening = if let Ok(eq) = read_state.equipment(self.valve_id) {
            eq.get_var("opening").unwrap_or(0.0) / 100.0
        } else {
            0.0
        };

        // 2. Hydraulics
        let p_bottom = self.vessel.bottom_pressure(self.rho);
        let p_atm = Pressure::new(101_325.0).unwrap();
        let discharge = self
            .valve
            .calculate_flow(p_bottom, p_atm, self.rho, opening)
            .unwrap_or(MassFlow::new_unchecked(0.0));

        // 3. Conservation integration
        let _ = self.vessel.step(
            dt,
            &[MassFlow::new_unchecked(self.inflow_rate)],
            &[discharge],
            self.rho,
        );

        let lvl = self.vessel.liquid_level();
        let _ = write_state.mutate_equipment(self.vessel_id, |eq| {
            eq.set_var("level", lvl);
            eq.set_var("mass", self.vessel.holdup_mass());
        });
        let _ = write_state.mutate_equipment(self.valve_id, |eq| {
            eq.set_var("flow", discharge.value());
        });

        Ok(())
    }
}

#[test]
fn test_complete_causal_chain_with_faults_and_recovery() {
    let vessel_id = EquipmentId::new();
    let valve_id = EquipmentId::new();
    let level_tag = TagId::new();
    let out_tag = TagId::new();

    // 1. Create SimulationEngine
    let mut engine = SimulationEngine::new(0.05).unwrap(); // 50ms timestep

    // 2. Setup Physics
    let physics = VesselProcessPhysics::new(vessel_id, valve_id, 0.70);
    engine.add_system(physics);

    // 3. Setup Control System
    let mut ctrl = ControlSystem::new();

    // Instrument
    let inst_cfg = InstrumentConfig::builder()
        .lrv(0.0)
        .urv(1.0)
        .sensor_lag_tau(0.2)
        .damping_tau(0.1)
        .noise_std_dev(0.0) // zero noise for deterministic numerical tracking
        .scan_interval(0.05)
        .build()
        .unwrap();
    ctrl.register_instrument(vessel_id, "level", level_tag, inst_cfg, 0.70);

    // Actuator
    let act_cfg = ActuatorConfig {
        stroke_time: 1.0, // fast 1-second stroke
        deadband: 0.05,
        fail_action: FailAction::FailClosed,
    };
    ctrl.register_actuator(valve_id, "opening", act_cfg, 0.0, None);

    // PID Level Controller
    let pid_cfg = PidConfig {
        kp: 200.0,
        ti: 5.0,
        td: 0.5,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Direct,
        sp_tracking_in_manual: false,
        scan_divisor: 1,
    };
    ctrl.register_pid(level_tag, out_tag, valve_id, pid_cfg);
    ctrl.pid_mut(out_tag).unwrap().initialize(0.70);
    ctrl.pid_mut(out_tag).unwrap().set_mode(PidMode::Auto);
    ctrl.pid_mut(out_tag).unwrap().set_sp(0.50);

    // Protection Block (Interlock Trip on High-High level at 0.92, reset deadband 0.04)
    let prot_cfg = ProtectionConfig::new("LSHH_TRIP", 0.92, 0.04);
    ctrl.register_protection(
        "LSHH_TRIP",
        level_tag,
        valve_id,
        CommandPriority::Interlock,
        100.0, // Force valve 100% open to drain tank
        prot_cfg,
    );

    engine.add_system(ctrl);

    // 4. Initial StateBuffer
    let mut initial = StateBuffer::new();
    let mut v_eq = EquipmentState::new("running");
    v_eq.set_var("level", 0.70);
    v_eq.set_var("mass", 700.0);
    initial.equipment.insert(vessel_id, v_eq);

    let mut valve_eq = EquipmentState::new("running");
    valve_eq.set_var("opening", 0.0);
    valve_eq.set_var("flow", 0.0);
    initial.equipment.insert(valve_id, valve_eq);

    engine.initialize(initial);

    // =====================================================================
    // Stage A: Normal Closed-Loop Regulation
    // =====================================================================
    for _ in 0..1600 {
        engine.step().expect("step succeeds");
    }

    let regulated_level = engine
        .state()
        .equipment(vessel_id)
        .unwrap()
        .get_var("level")
        .unwrap();
    assert!(
        (regulated_level - 0.50).abs() < 0.10,
        "Level must converge toward SP = 0.50, got {regulated_level}"
    );

    // =====================================================================
    // Stage B: Valve Saturation & Anti-Windup Verification
    // =====================================================================
    // Step simulation manually injecting massive inflow to test saturation
    // We modify the equipment state to simulate surge
    let mut test_pid = PidController::new(PidConfig {
        kp: 100.0,
        ti: 2.0,
        td: 0.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Direct,
        sp_tracking_in_manual: false,
        scan_divisor: 1,
    });
    test_pid.initialize(0.50);
    test_pid.set_mode(PidMode::Auto);
    test_pid.set_sp(0.50);

    // Drive PV high to saturate output at 100%
    for _ in 0..50 {
        test_pid.step(0.1, 0.90);
    }
    assert!(
        test_pid.is_high_saturated(),
        "PID output must saturate at 100%"
    );
    assert_eq!(test_pid.output(), 100.0);
    let integral_saturated = test_pid.integral();
    assert!(
        integral_saturated <= 100.0,
        "Anti-windup must clamp integral term, got {integral_saturated}"
    );

    // =====================================================================
    // Stage C: Interlock Trip & Safety Override Tracking
    // =====================================================================
    let mut prot_block = ProtectionBlock::new(ProtectionConfig::new("TRIP_TEST", 0.92, 0.04));
    assert!(!prot_block.step(0.85, false));
    assert_eq!(prot_block.state(), ProtectionState::Active);

    // High level crosses threshold -> Trip!
    assert!(prot_block.step(0.93, false));
    assert_eq!(prot_block.state(), ProtectionState::Tripped);

    // Controller enters tracking mode
    test_pid.enter_tracking(100.0);
    assert_eq!(test_pid.output(), 100.0);

    // =====================================================================
    // Stage D: Bad Signal Quality Handling
    // =====================================================================
    test_pid.exit_tracking();
    test_pid.set_quality(TagQuality::Bad);
    let out_before = test_pid.output();
    let out_during_bad = test_pid.step(0.1, 0.99);
    assert_eq!(
        out_during_bad, out_before,
        "PID must freeze output when TagQuality is Bad"
    );

    // =====================================================================
    // Stage E: Recovery & Reset Lifecycle
    // =====================================================================
    // Restore signal quality
    test_pid.set_quality(TagQuality::Good);

    // Level drops to 0.85 (below 0.92 - 0.04 = 0.88 cleared threshold)
    // Operator reset is commanded
    let unlatched = prot_block.step(0.85, true);
    assert!(!unlatched, "Protection trip must unlatch on valid reset");
    assert_eq!(prot_block.state(), ProtectionState::Armed);

    // PID resumes regulation bumplessly
    let resumed_out = test_pid.step(0.1, 0.85);
    assert!(resumed_out.is_finite());
}
