//! Comprehensive tests for nebudcs-sim:
//! - Clock determinism, pause/play, single step, speed multiplier
//! - Double-buffered state isolation: strict read-from-N and write-to-N+1
//! - Isolation proof: System A writes x, System B reads x in same tick
//! - Deterministic execution plan derived from topology (acyclic and recycle loops)
//! - SimulationEngine commit lifecycle and transactional failure handling

use indexmap::IndexMap;

use nebudcs_model::equipment::{
    Cardinality, EquipmentCategory, EquipmentDefinition, Phase, PortDefinition, PortDirection,
    PortDomain,
};
use nebudcs_model::stream::{StreamDefinition, StreamType};
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_sim::{
    ClockState, EquipmentState, ExecutionPlan, SimError, SimulationClock, SimulationEngine,
    SimulationSystem, StateBuffer, StateReadView, StateWriteView, StepContext,
};
use nebudcs_topology::TopologyGraph;
use nebudcs_types::{
    EquipmentDefinitionId, EquipmentId, PortId, SchemaVersion, SimulationTime, TagId,
};

/// Helper to create a dummy equipment definition.
fn make_def(name: &str, ports: Vec<PortDefinition>) -> EquipmentDefinition {
    EquipmentDefinition {
        id: EquipmentDefinitionId::new(),
        name: name.to_owned(),
        version: SchemaVersion::new(0, 1, 0),
        category: EquipmentCategory::Process,
        capabilities: vec![],
        parameters: IndexMap::new(),
        ports,
        tags: vec![],
        operating_envelope: Default::default(),
        alarm_definitions: vec![],
        failure_modes: vec![],
        state_machine: Default::default(),
        metadata: Default::default(),
    }
}

fn make_port(id: PortId, name: &str, dir: PortDirection) -> PortDefinition {
    PortDefinition {
        id,
        name: name.to_owned(),
        domain: PortDomain::Material,
        direction: dir,
        phase_compatibility: vec![Phase::Liquid],
        required: true,
        cardinality: Cardinality::OneOrMore,
        description: String::new(),
    }
}

// ===========================================================================
// 1. SimulationClock Tests
// ===========================================================================

#[test]
fn test_clock_initial_state() {
    let clock = SimulationClock::new(0.05).unwrap();
    assert_eq!(clock.dt(), 0.05);
    assert_eq!(clock.time(), SimulationTime::ZERO);
    assert_eq!(clock.tick_count(), 0);
    assert_eq!(clock.state(), ClockState::Paused);
    assert!(clock.is_paused());
    assert!(!clock.is_running());
    assert_eq!(clock.speed_multiplier(), 1.0);
}

#[test]
fn test_clock_rejects_invalid_dt() {
    assert!(matches!(
        SimulationClock::new(0.0),
        Err(SimError::InvalidTimestep { .. })
    ));
    assert!(matches!(
        SimulationClock::new(-0.01),
        Err(SimError::InvalidTimestep { .. })
    ));
    assert!(matches!(
        SimulationClock::new(f64::NAN),
        Err(SimError::InvalidTimestep { .. })
    ));
    assert!(matches!(
        SimulationClock::new(f64::INFINITY),
        Err(SimError::InvalidTimestep { .. })
    ));
}

#[test]
fn test_clock_play_pause_and_advance() {
    let mut clock = SimulationClock::new(0.1).unwrap();
    clock.play();
    assert!(clock.is_running());

    clock.advance_tick();
    assert_eq!(clock.tick_count(), 1);
    assert!((clock.time().seconds() - 0.1).abs() < 1e-12);

    clock.advance_tick();
    assert_eq!(clock.tick_count(), 2);
    assert!((clock.time().seconds() - 0.2).abs() < 1e-12);

    clock.pause();
    assert!(clock.is_paused());
}

#[test]
fn test_clock_speed_multiplier_does_not_change_dt() {
    let mut clock = SimulationClock::new(0.02).unwrap();
    clock.set_speed_multiplier(5.0).unwrap();
    assert_eq!(clock.speed_multiplier(), 5.0);
    assert_eq!(clock.dt(), 0.02); // dt must NOT change

    clock.advance_tick();
    assert!((clock.time().seconds() - 0.02).abs() < 1e-12);

    assert!(matches!(
        clock.set_speed_multiplier(-1.0),
        Err(SimError::InvalidSpeedMultiplier { .. })
    ));
    assert!(matches!(
        clock.set_speed_multiplier(0.0),
        Err(SimError::InvalidSpeedMultiplier { .. })
    ));
}

// ===========================================================================
// 2. Double-Buffered State & Proof of Isolation
// ===========================================================================

/// Test system that increments variable `x` by 1: `x_{N+1} = x_N + 1`.
struct SystemA {
    eq_id: EquipmentId,
}
impl SimulationSystem for SystemA {
    fn name(&self) -> &str {
        "SystemA"
    }
    fn step(
        &mut self,
        _ctx: StepContext,
        read: &StateReadView<'_>,
        write: &mut StateWriteView<'_>,
    ) -> Result<(), SimError> {
        let x = read.equipment(self.eq_id)?.get_var("x").unwrap_or(0.0);
        write.mutate_equipment(self.eq_id, |eq| {
            eq.set_var("x", x + 1.0);
        })?;
        Ok(())
    }
}

/// Test system that reads variable `x` and writes it to `y`: `y_{N+1} = x_N`.
struct SystemB {
    eq_id: EquipmentId,
}
impl SimulationSystem for SystemB {
    fn name(&self) -> &str {
        "SystemB"
    }
    fn step(
        &mut self,
        _ctx: StepContext,
        read: &StateReadView<'_>,
        write: &mut StateWriteView<'_>,
    ) -> Result<(), SimError> {
        let x = read.equipment(self.eq_id)?.get_var("x").unwrap_or(0.0);
        write.mutate_equipment(self.eq_id, |eq| {
            eq.set_var("y", x);
        })?;
        Ok(())
    }
}

/// Strict double-buffering proof:
///
/// Initial state: x = 10, y = 0.
/// In tick 1:
/// - System A executes first: reads x = 10 from State[0], writes x = 11 to State[1].
/// - System B executes second: reads x from State[0]. Must observe x = 10, NOT 11. Writes y = 10 to State[1].
///
/// After tick 1 commit:
/// - x = 11
/// - y = 10
#[test]
fn test_double_buffer_isolation_proof() {
    let mut engine = SimulationEngine::new(0.1).unwrap();
    let eq_id = EquipmentId::new();

    let mut initial_buffer = StateBuffer::new();
    let mut eq_state = EquipmentState::new("running");
    eq_state.set_var("x", 10.0);
    eq_state.set_var("y", 0.0);
    initial_buffer.equipment.insert(eq_id, eq_state);

    engine.initialize(initial_buffer);

    // Register System A (writes x) then System B (reads x, writes y)
    engine.add_system(SystemA { eq_id });
    engine.add_system(SystemB { eq_id });

    // Execute exactly one tick
    engine.step().unwrap();

    let committed = engine.get_equipment(eq_id).unwrap();
    let final_x = committed.get_var("x").unwrap();
    let final_y = committed.get_var("y").unwrap();

    assert_eq!(final_x, 11.0, "x should be updated to 11 in State[1]");
    assert_eq!(
        final_y, 10.0,
        "y must be 10.0 because SystemB must observe State[0], NOT partially written State[1]"
    );
}

// ===========================================================================
// 3. Deterministic Execution Plan Tests
// ===========================================================================

#[test]
fn test_execution_plan_acyclic_upstream_to_downstream() {
    let mut topo = TopologyGraph::new();

    let pa_out = PortId::new();
    let pb_in = PortId::new();
    let pb_out = PortId::new();
    let pc_in = PortId::new();

    let def_a = make_def("A", vec![make_port(pa_out, "out", PortDirection::Outlet)]);
    let def_b = make_def(
        "B",
        vec![
            make_port(pb_in, "in", PortDirection::Inlet),
            make_port(pb_out, "out", PortDirection::Outlet),
        ],
    );
    let def_c = make_def("C", vec![make_port(pc_in, "in", PortDirection::Inlet)]);

    let da = def_a.id;
    let db = def_b.id;
    let dc = def_c.id;

    topo.register_definition(def_a);
    topo.register_definition(def_b);
    topo.register_definition(def_c);

    let eq_a = EquipmentId::new();
    let eq_b = EquipmentId::new();
    let eq_c = EquipmentId::new();

    topo.add_equipment(eq_a, da).unwrap();
    topo.add_equipment(eq_b, db).unwrap();
    topo.add_equipment(eq_c, dc).unwrap();

    // Stream A -> B and B -> C
    topo.add_stream(StreamDefinition::new(
        "A->B",
        StreamType::Material,
        eq_a,
        pa_out,
        eq_b,
        pb_in,
    ))
    .unwrap();
    topo.add_stream(StreamDefinition::new(
        "B->C",
        StreamType::Material,
        eq_b,
        pb_out,
        eq_c,
        pc_in,
    ))
    .unwrap();

    let plan = ExecutionPlan::from_topology(&topo);
    assert_eq!(plan.len(), 3);

    let flat_order = plan.flattened_execution_order();
    // Must execute upstream (A) -> intermediate (B) -> downstream (C)
    assert_eq!(flat_order, vec![eq_a, eq_b, eq_c]);
}

#[test]
fn test_execution_plan_recycle_loop_detection() {
    let mut topo = TopologyGraph::new();

    let p_out1 = PortId::new();
    let p_in1 = PortId::new();
    let p_out2 = PortId::new();
    let p_in2 = PortId::new();

    let def = make_def(
        "Unit",
        vec![
            make_port(p_out1, "out", PortDirection::Outlet),
            make_port(p_in1, "in", PortDirection::Inlet),
            make_port(p_out2, "out2", PortDirection::Outlet),
            make_port(p_in2, "in2", PortDirection::Inlet),
        ],
    );
    let def_id = def.id;
    topo.register_definition(def);

    let n1 = EquipmentId::new();
    let n2 = EquipmentId::new();
    topo.add_equipment(n1, def_id).unwrap();
    topo.add_equipment(n2, def_id).unwrap();

    // Recycle: n1 <-> n2
    topo.add_stream(StreamDefinition::new(
        "1->2",
        StreamType::Material,
        n1,
        p_out1,
        n2,
        p_in2,
    ))
    .unwrap();
    topo.add_stream(StreamDefinition::new(
        "2->1",
        StreamType::Material,
        n2,
        p_out2,
        n1,
        p_in1,
    ))
    .unwrap();

    let plan = ExecutionPlan::from_topology(&topo);
    assert_eq!(plan.len(), 1);
    assert!(plan.blocks()[0].is_recycle());
    assert_eq!(plan.blocks()[0].equipment().len(), 2);
}

// ===========================================================================
// 4. Engine Lifecycle & Transactional Failure Tests
// ===========================================================================

struct FailingSystem;
impl SimulationSystem for FailingSystem {
    fn name(&self) -> &str {
        "FailingSystem"
    }
    fn step(
        &mut self,
        _ctx: StepContext,
        _read: &StateReadView<'_>,
        _write: &mut StateWriteView<'_>,
    ) -> Result<(), SimError> {
        Err(SimError::LifecycleError {
            reason: "simulated failure".to_string(),
        })
    }
}

#[test]
fn test_failing_system_does_not_commit_state() {
    let mut engine = SimulationEngine::new(0.1).unwrap();
    let eq_id = EquipmentId::new();

    let mut initial = StateBuffer::new();
    let mut eq = EquipmentState::new("running");
    eq.set_var("x", 50.0);
    initial.equipment.insert(eq_id, eq);
    engine.initialize(initial);

    // System A increments x
    engine.add_system(SystemA { eq_id });
    // System B fails
    engine.add_system(FailingSystem);

    let res = engine.step();
    assert!(res.is_err(), "Step must fail if any system fails");

    // Verify State[0] was NOT modified and clock was NOT advanced
    assert_eq!(engine.clock().tick_count(), 0);
    assert_eq!(engine.clock().time(), SimulationTime::ZERO);
    let committed = engine.get_equipment(eq_id).unwrap();
    assert_eq!(
        committed.get_var("x"),
        Some(50.0),
        "State must remain unchanged on failure"
    );
}

#[test]
fn test_tick_if_running_behavior() {
    let mut engine = SimulationEngine::new(0.1).unwrap();
    assert!(engine.clock().is_paused());

    // When paused, tick_if_running returns Ok(false) and does not advance
    assert!(!engine.tick_if_running().unwrap());
    assert_eq!(engine.clock().tick_count(), 0);

    // When running, tick_if_running advances
    engine.clock_mut().play();
    assert!(engine.tick_if_running().unwrap());
    assert_eq!(engine.clock().tick_count(), 1);
    assert!((engine.clock().time().seconds() - 0.1).abs() < 1e-12);
}

#[test]
fn test_tag_snapshot_and_read_view() {
    let mut engine = SimulationEngine::new(0.1).unwrap();
    let tag_id = TagId::new();

    let mut initial = StateBuffer::new();
    initial.tags.insert(tag_id, TagValue::Float(101.3));
    initial.qualities.insert(tag_id, TagQuality::Good);
    engine.initialize(initial);

    let snap = engine.state().tag_snapshot(tag_id).unwrap();
    assert_eq!(snap.tag_id, tag_id);
    assert_eq!(snap.value, TagValue::Float(101.3));
    assert_eq!(snap.quality, TagQuality::Good);
    assert_eq!(snap.timestamp, SimulationTime::ZERO);
}
