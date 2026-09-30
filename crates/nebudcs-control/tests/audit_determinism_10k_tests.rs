use nebudcs_control::actuator::{ActuatorConfig, CommandPriority, FailAction};
use nebudcs_control::instrument::InstrumentConfig;
use nebudcs_control::pid::{PidConfig, PidDirection, PidMode};
use nebudcs_control::protection::ProtectionConfig;
use nebudcs_control::system::ControlSystem;
use nebudcs_physics::equipment::{AccumulatorVolumeModel, FlowResistanceModel};
use nebudcs_sim::{
    EquipmentState, SimError, SimulationEngine, SimulationSystem, StateBuffer, StateReadView,
    StateWriteView, StepContext,
};
use nebudcs_types::{EquipmentId, TagId};
use nebudcs_units::{Density, MassFlow, Pressure};

#[derive(Debug)]
struct TankDrainSystem {
    vessel_id: EquipmentId,
    valve_id: EquipmentId,
    vessel: AccumulatorVolumeModel,
    valve: FlowResistanceModel,
    inflow: f64,
    rho: Density,
}

impl TankDrainSystem {
    fn new(vessel_id: EquipmentId, valve_id: EquipmentId) -> Self {
        let rho = Density::new(1000.0).unwrap();
        let vessel = AccumulatorVolumeModel::new(1.0, 0.5, 800.0, rho);
        let valve = FlowResistanceModel::new_from_cv(80.0);
        Self {
            vessel_id,
            valve_id,
            vessel,
            valve,
            inflow: 5.0,
            rho,
        }
    }
}

impl SimulationSystem for TankDrainSystem {
    fn name(&self) -> &str {
        "TankDrainSystem"
    }

    fn step(
        &mut self,
        ctx: StepContext,
        read_state: &StateReadView<'_>,
        write_state: &mut StateWriteView<'_>,
    ) -> Result<(), SimError> {
        let dt = ctx.dt;
        let opening = if let Ok(eq) = read_state.equipment(self.valve_id) {
            eq.get_var("opening").unwrap_or(0.0) / 100.0
        } else {
            0.0
        };

        let p_bottom = self.vessel.bottom_pressure(self.rho);
        let p_atm = Pressure::new(101_325.0).unwrap();
        let flow = self
            .valve
            .calculate_flow(p_bottom, p_atm, self.rho, opening)
            .unwrap_or(MassFlow::new_unchecked(0.0));

        let _ = self.vessel.step(
            dt,
            &[MassFlow::new_unchecked(self.inflow)],
            &[flow],
            self.rho,
        );

        let lvl = self.vessel.liquid_level();
        let _ = write_state.mutate_equipment(self.vessel_id, |eq| {
            eq.set_var("level", lvl);
            eq.set_var("mass", self.vessel.holdup_mass());
        });
        let _ = write_state.mutate_equipment(self.valve_id, |eq| {
            eq.set_var("flow", flow.value());
        });

        Ok(())
    }
}

struct StateSnapshot {
    level_bits: u64,
    mass_bits: u64,
    valve_opening_bits: u64,
    valve_flow_bits: u64,
    tag_pv_bits: u64,
    tag_out_bits: u64,
}

fn build_simulation() -> (SimulationEngine, EquipmentId, EquipmentId, TagId, TagId) {
    let vessel_id = EquipmentId::new();
    let valve_id = EquipmentId::new();
    let level_tag = TagId::new();
    let out_tag = TagId::new();

    let mut engine = SimulationEngine::new(0.05).unwrap(); // 50ms tick, 20 Hz

    // Physics
    let physics = TankDrainSystem::new(vessel_id, valve_id);
    engine.add_system(physics);

    // Control
    let mut ctrl = ControlSystem::new();
    let inst_cfg = InstrumentConfig::builder()
        .lrv(0.0)
        .urv(1.0)
        .sensor_lag_tau(0.5)
        .damping_tau(0.2)
        .noise_std_dev(0.001) // deterministic PRNG noise
        .scan_interval(0.05)
        .build()
        .unwrap();
    ctrl.register_instrument(vessel_id, "level", level_tag, inst_cfg, 0.8);

    let act_cfg = ActuatorConfig {
        stroke_time: 2.0,
        deadband: 0.05,
        fail_action: FailAction::FailClosed,
    };
    ctrl.register_actuator(valve_id, "opening", act_cfg, 0.0, None);

    let pid_cfg = PidConfig {
        kp: 200.0,
        ti: 10.0,
        td: 0.5,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Direct,
        sp_tracking_in_manual: false,
        scan_divisor: 1,
    };
    ctrl.register_pid(level_tag, out_tag, valve_id, pid_cfg);
    ctrl.pid_mut(out_tag).unwrap().initialize(0.8);
    ctrl.pid_mut(out_tag).unwrap().set_mode(PidMode::Auto);
    ctrl.pid_mut(out_tag).unwrap().set_sp(0.50);

    let prot_cfg = ProtectionConfig::new("HIHI_TRIP", 0.95, 0.05);
    ctrl.register_protection(
        "HIHI_TRIP",
        level_tag,
        valve_id,
        CommandPriority::Interlock,
        100.0, // Fail open on high level to dump
        prot_cfg,
    );
    engine.add_system(ctrl);

    // Initial state
    let mut initial = StateBuffer::new();
    let mut v_eq = EquipmentState::new("running");
    v_eq.set_var("level", 0.8);
    v_eq.set_var("mass", 800.0);
    initial.equipment.insert(vessel_id, v_eq);

    let mut valve_eq = EquipmentState::new("running");
    valve_eq.set_var("opening", 0.0);
    valve_eq.set_var("flow", 0.0);
    initial.equipment.insert(valve_id, valve_eq);

    engine.initialize(initial);

    (engine, vessel_id, valve_id, level_tag, out_tag)
}

#[test]
fn test_10k_ticks_full_state_bit_identical_determinism() {
    let run = || -> Vec<StateSnapshot> {
        let (mut engine, vessel_id, valve_id, level_tag, out_tag) = build_simulation();
        let mut snapshots = Vec::with_capacity(1000);

        for tick in 0..10_000 {
            engine.step().expect("step succeeds");

            // Record snapshot every 10 ticks (1000 total checkpoints)
            if tick % 10 == 0 {
                let st = engine.state();
                let v_eq = st.equipment(vessel_id).unwrap();
                let valve_eq = st.equipment(valve_id).unwrap();
                let tag_pv = match st.tag_value(level_tag).unwrap() {
                    nebudcs_model::tag::TagValue::Float(f) => *f,
                    _ => 0.0,
                };
                let tag_out = match st.tag_value(out_tag).unwrap() {
                    nebudcs_model::tag::TagValue::Float(f) => *f,
                    _ => 0.0,
                };

                snapshots.push(StateSnapshot {
                    level_bits: v_eq.get_var("level").unwrap().to_bits(),
                    mass_bits: v_eq.get_var("mass").unwrap().to_bits(),
                    valve_opening_bits: valve_eq.get_var("opening").unwrap().to_bits(),
                    valve_flow_bits: valve_eq.get_var("flow").unwrap().to_bits(),
                    tag_pv_bits: tag_pv.to_bits(),
                    tag_out_bits: tag_out.to_bits(),
                });
            }
        }
        snapshots
    };

    let run1 = run();
    let run2 = run();

    assert_eq!(run1.len(), 1000);
    assert_eq!(run2.len(), 1000);

    for (i, (s1, s2)) in run1.iter().zip(run2.iter()).enumerate() {
        assert_eq!(
            s1.level_bits, s2.level_bits,
            "Level divergence at checkpoint {i}"
        );
        assert_eq!(
            s1.mass_bits, s2.mass_bits,
            "Mass divergence at checkpoint {i}"
        );
        assert_eq!(
            s1.valve_opening_bits, s2.valve_opening_bits,
            "Valve opening divergence at checkpoint {i}"
        );
        assert_eq!(
            s1.valve_flow_bits, s2.valve_flow_bits,
            "Valve flow divergence at checkpoint {i}"
        );
        assert_eq!(
            s1.tag_pv_bits, s2.tag_pv_bits,
            "Tag PV divergence at checkpoint {i}"
        );
        assert_eq!(
            s1.tag_out_bits, s2.tag_out_bits,
            "Tag OUT divergence at checkpoint {i}"
        );
    }
}
