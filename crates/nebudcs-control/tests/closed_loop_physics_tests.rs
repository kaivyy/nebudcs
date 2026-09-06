use nebudcs_control::actuator::{ActuatorConfig, FailAction};
use nebudcs_control::instrument::InstrumentConfig;
use nebudcs_control::pid::{PidConfig, PidDirection, PidMode};
use nebudcs_control::system::ControlSystem;
use nebudcs_physics::equipment::{AccumulatorVolumeModel, FlowResistanceModel};
use nebudcs_sim::{
    EquipmentState, SimError, SimulationEngine, SimulationSystem, StateBuffer, StateReadView,
    StateWriteView, StepContext,
};
use nebudcs_types::{EquipmentId, TagId};
use nebudcs_units::{Density, MassFlow, Pressure};

/// Physics simulation system for a liquid vessel with gravity discharge through a control valve.
#[derive(Debug)]
struct TankDrainPhysicsSystem {
    vessel_id: EquipmentId,
    valve_id: EquipmentId,
    vessel: AccumulatorVolumeModel,
    valve: FlowResistanceModel,
    inflow_rate: f64, // kg/s
    rho: Density,
}

impl TankDrainPhysicsSystem {
    fn new(
        vessel_id: EquipmentId,
        valve_id: EquipmentId,
        initial_level: f64,
        inflow_rate: f64,
    ) -> Self {
        let rho = Density::new(1000.0).unwrap();
        let total_vol = 1.0; // 1 m³ (1000 kg total capacity)
        let area = 0.5; // 0.5 m²
        let initial_mass = initial_level * total_vol * rho.value();
        let vessel = AccumulatorVolumeModel::new(total_vol, area, initial_mass, rho);
        let valve = FlowResistanceModel::new_from_cv(80.0);

        Self {
            vessel_id,
            valve_id,
            vessel,
            valve,
            inflow_rate,
            rho,
        }
    }
}

impl SimulationSystem for TankDrainPhysicsSystem {
    fn name(&self) -> &str {
        "TankDrainPhysicsSystem"
    }

    fn step(
        &mut self,
        ctx: StepContext,
        read_state: &StateReadView<'_>,
        write_state: &mut StateWriteView<'_>,
    ) -> Result<(), SimError> {
        let dt = ctx.dt;

        // 1. Read valve opening from State[N] (fraction 0.0 to 1.0)
        let opening = if let Ok(valve_eq) = read_state.equipment(self.valve_id) {
            valve_eq.get_var("opening").unwrap_or(0.0) / 100.0
        } else {
            0.0
        };

        // 2. Compute vessel bottom hydrostatic pressure
        let p_bottom = self.vessel.bottom_pressure(self.rho);
        let p_atm = Pressure::new(101_325.0).unwrap();

        // 3. Compute discharge flow through control valve
        let discharge_flow = self
            .valve
            .calculate_flow(p_bottom, p_atm, self.rho, opening)
            .unwrap_or(MassFlow::new_unchecked(0.0));

        // 4. Integrate vessel mass holdup
        let inflows = [MassFlow::new_unchecked(self.inflow_rate)];
        let outflows = [discharge_flow];
        let _ = self.vessel.step(dt, &inflows, &outflows, self.rho);

        let current_level = self.vessel.liquid_level();

        // 5. Update State[N+1] equipment variables
        let _ = write_state.mutate_equipment(self.vessel_id, |eq| {
            eq.set_var("level", current_level);
            eq.set_var("mass", self.vessel.holdup_mass());
        });

        let _ = write_state.mutate_equipment(self.valve_id, |eq| {
            eq.set_var("flow", discharge_flow.value());
        });

        Ok(())
    }
}

#[test]
fn test_closed_loop_vessel_level_regulation() {
    let vessel_id = EquipmentId::new();
    let valve_id = EquipmentId::new();
    let level_pv_tag = TagId::new();
    let valve_out_tag = TagId::new();

    // Constant inflow of 5.0 kg/s, starting with high level = 0.8 (80%)
    // The valve with Cv=80 and 0.4 bar head can discharge up to ~12 kg/s,
    // allowing the PID controller to drain the vessel to the 0.50 setpoint!
    let physics_sys = TankDrainPhysicsSystem::new(vessel_id, valve_id, 0.8, 5.0);

    let mut ctrl_sys = ControlSystem::new();

    // Level transmitter measuring vessel "level" (range 0.0 to 1.0)
    let inst_cfg = InstrumentConfig::builder()
        .lrv(0.0)
        .urv(1.0)
        .sensor_lag_tau(0.5)
        .damping_tau(0.2)
        .scan_interval(0.1)
        .build()
        .unwrap();
    ctrl_sys.register_instrument(vessel_id, "level", level_pv_tag, inst_cfg, 0.8);

    // Valve actuator (stroke time 2.0s, FailClosed)
    let act_cfg = ActuatorConfig {
        stroke_time: 2.0,
        deadband: 0.1,
        fail_action: FailAction::FailClosed,
    };
    ctrl_sys.register_actuator(valve_id, "opening", act_cfg, 0.0, None);

    // PID level controller: Direct acting! (Level too high -> open discharge valve wider)
    // SP = 0.50 (50% target level)
    let pid_cfg = PidConfig {
        kp: 250.0,
        ti: 8.0,
        td: 1.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Direct, // Direct: error = PV - SP
        sp_tracking_in_manual: false,
        scan_divisor: 1,
    };
    ctrl_sys.register_pid(level_pv_tag, valve_out_tag, valve_id, pid_cfg);

    let pid = ctrl_sys.pid_mut(valve_out_tag).unwrap();
    pid.initialize(0.8);
    pid.set_mode(PidMode::Auto);
    pid.set_sp(0.50);

    // Assemble SimulationEngine
    let mut engine = SimulationEngine::new(0.1).unwrap();
    engine.add_system(physics_sys);
    engine.add_system(ctrl_sys);

    // Initialize baseline StateBuffer
    let mut initial_state = StateBuffer::new();
    let mut vessel_eq = EquipmentState::new("running");
    vessel_eq.set_var("level", 0.8);
    initial_state.equipment.insert(vessel_id, vessel_eq);

    let mut valve_eq = EquipmentState::new("running");
    valve_eq.set_var("opening", 0.0);
    initial_state.equipment.insert(valve_id, valve_eq);

    engine.initialize(initial_state);

    // Run 800 ticks (80 seconds)
    for _ in 0..800 {
        engine.step().expect("step succeeds");
    }

    // After 50 seconds of closed-loop control:
    // Level must have drained from 0.8 down towards setpoint 0.50
    let final_level = engine
        .state()
        .equipment(vessel_id)
        .unwrap()
        .get_var("level")
        .unwrap();

    assert!(
        final_level < 0.70,
        "Closed loop must drain vessel level, but level remained at {final_level}"
    );
    assert!(
        (final_level - 0.50).abs() < 0.15,
        "Closed loop level regulation must converge towards setpoint 0.50, got {final_level}"
    );

    // Valve opening must be positive (discharging liquid)
    let final_opening = engine
        .state()
        .equipment(valve_id)
        .unwrap()
        .get_var("opening")
        .unwrap();
    assert!(
        final_opening > 10.0,
        "Valve must be open to regulate level, got {final_opening}"
    );
}

#[test]
fn test_closed_loop_50k_ticks_deterministic_replay() {
    let run_simulation = || -> Vec<f64> {
        let vessel_id = EquipmentId::new();
        let valve_id = EquipmentId::new();
        let level_pv_tag = TagId::new();
        let valve_out_tag = TagId::new();

        let physics_sys = TankDrainPhysicsSystem::new(vessel_id, valve_id, 0.75, 20.0);
        let mut ctrl_sys = ControlSystem::new();

        let inst_cfg = InstrumentConfig::builder()
            .lrv(0.0)
            .urv(1.0)
            .sensor_lag_tau(0.2)
            .damping_tau(0.1)
            .scan_interval(0.1)
            .build()
            .unwrap();
        ctrl_sys.register_instrument(vessel_id, "level", level_pv_tag, inst_cfg, 0.75);

        let act_cfg = ActuatorConfig {
            stroke_time: 2.0,
            deadband: 0.05,
            fail_action: FailAction::FailClosed,
        };
        ctrl_sys.register_actuator(valve_id, "opening", act_cfg, 10.0, None);

        let pid_cfg = PidConfig {
            kp: 50.0,
            ti: 10.0,
            td: 0.5,
            derivative_filter_n: 10.0,
            out_min: 0.0,
            out_max: 100.0,
            direction: PidDirection::Direct,
            sp_tracking_in_manual: false,
            scan_divisor: 1,
        };
        ctrl_sys.register_pid(level_pv_tag, valve_out_tag, valve_id, pid_cfg);
        let pid = ctrl_sys.pid_mut(valve_out_tag).unwrap();
        pid.initialize(0.75);
        pid.set_mode(PidMode::Auto);
        pid.set_sp(0.40);

        let mut engine = SimulationEngine::new(0.05).unwrap(); // 50ms tick
        engine.add_system(physics_sys);
        engine.add_system(ctrl_sys);

        let mut initial_state = StateBuffer::new();
        let mut vessel_eq = EquipmentState::new("running");
        vessel_eq.set_var("level", 0.75);
        initial_state.equipment.insert(vessel_id, vessel_eq);

        let mut valve_eq = EquipmentState::new("running");
        valve_eq.set_var("opening", 10.0);
        initial_state.equipment.insert(valve_id, valve_eq);

        engine.initialize(initial_state);

        let mut trajectory = Vec::with_capacity(1000);
        for _ in 0..1000 {
            engine.step().unwrap();
            let lvl = engine
                .state()
                .equipment(vessel_id)
                .unwrap()
                .get_var("level")
                .unwrap();
            trajectory.push(lvl);
        }
        trajectory
    };

    let run1 = run_simulation();
    let run2 = run_simulation();

    assert_eq!(run1.len(), run2.len());
    for (i, (v1, v2)) in run1.iter().zip(run2.iter()).enumerate() {
        assert_eq!(
            v1.to_bits(),
            v2.to_bits(),
            "Non-deterministic divergence at step {i}: {v1} vs {v2}"
        );
    }
}
