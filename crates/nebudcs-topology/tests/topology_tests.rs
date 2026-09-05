//! Unit and integration tests for nebudcs-topology.

use indexmap::IndexMap;
use nebudcs_model::equipment::{
    Cardinality, EquipmentCategory, EquipmentDefinition, Phase, PortDefinition, PortDirection,
    PortDomain, UtilityKind,
};
use nebudcs_model::stream::{StreamDefinition, StreamType};
use nebudcs_topology::{
    PipeEndpoint, PipeSegment, SignalConnection, SignalEndpoint, SignalMedium, TopologyError,
    TopologyGraph,
};
use nebudcs_types::{EquipmentDefinitionId, EquipmentId, PortId, SchemaVersion, TagId};

/// Helper to create a generic equipment definition with specified ports.
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

/// Helper to create a material port.
fn make_material_port(
    id: PortId,
    name: &str,
    dir: PortDirection,
    phases: Vec<Phase>,
    card: Cardinality,
) -> PortDefinition {
    PortDefinition {
        id,
        name: name.to_owned(),
        domain: PortDomain::Material,
        direction: dir,
        phase_compatibility: phases,
        required: true,
        cardinality: card,
        description: String::new(),
    }
}

/// Helper to create a utility port.
fn make_utility_port(
    id: PortId,
    name: &str,
    util: UtilityKind,
    dir: PortDirection,
    card: Cardinality,
) -> PortDefinition {
    PortDefinition {
        id,
        name: name.to_owned(),
        domain: PortDomain::Utility(util),
        direction: dir,
        phase_compatibility: vec![],
        required: true,
        cardinality: card,
        description: String::new(),
    }
}

/// Helper to create a signal port.
fn make_signal_port(id: PortId, name: &str, dir: PortDirection) -> PortDefinition {
    PortDefinition {
        id,
        name: name.to_owned(),
        domain: PortDomain::Signal,
        direction: dir,
        phase_compatibility: vec![],
        required: false,
        cardinality: Cardinality::ZeroOrMore,
        description: String::new(),
    }
}

// ===========================================================================
// 1. Basic Topology Operations
// ===========================================================================

#[test]
fn test_empty_topology() {
    let topo = TopologyGraph::new();
    assert_eq!(topo.equipment_count(), 0);
    assert_eq!(topo.stream_count(), 0);
    assert_eq!(topo.pipe_count(), 0);
    assert_eq!(topo.signal_count(), 0);
    assert!(topo.equipment_ids().is_empty());
}

#[test]
fn test_add_equipment_and_query() {
    let mut topo = TopologyGraph::new();

    let p1_id = PortId::new();
    let def = make_def(
        "Pump",
        vec![make_material_port(
            p1_id,
            "out",
            PortDirection::Outlet,
            vec![Phase::Liquid],
            Cardinality::ExactlyOne,
        )],
    );
    let def_id = def.id;
    topo.register_definition(def);

    let eq_id = EquipmentId::new();
    assert!(topo.add_equipment(eq_id, def_id).is_ok());
    assert_eq!(topo.equipment_count(), 1);
    assert!(topo.contains_equipment(eq_id));
    assert_eq!(topo.equipment_ids(), vec![eq_id]);

    let port_def = topo.get_port_definition(eq_id, p1_id).unwrap();
    assert_eq!(port_def.name, "out");
    assert_eq!(port_def.direction, PortDirection::Outlet);
}

#[test]
fn test_add_valid_process_connection() {
    let mut topo = TopologyGraph::new();

    let p_out = PortId::new();
    let def_a = make_def(
        "Source",
        vec![make_material_port(
            p_out,
            "out",
            PortDirection::Outlet,
            vec![Phase::Liquid],
            Cardinality::ExactlyOne,
        )],
    );
    let def_a_id = def_a.id;
    topo.register_definition(def_a);

    let p_in = PortId::new();
    let def_b = make_def(
        "Sink",
        vec![make_material_port(
            p_in,
            "in",
            PortDirection::Inlet,
            vec![Phase::Liquid],
            Cardinality::ExactlyOne,
        )],
    );
    let def_b_id = def_b.id;
    topo.register_definition(def_b);

    let eq_a = EquipmentId::new();
    let eq_b = EquipmentId::new();
    topo.add_equipment(eq_a, def_a_id).unwrap();
    topo.add_equipment(eq_b, def_b_id).unwrap();

    let stream = StreamDefinition::new("Stream-1", StreamType::Material, eq_a, p_out, eq_b, p_in);
    let stream_id = stream.id;
    assert!(topo.add_stream(stream).is_ok());

    assert_eq!(topo.stream_count(), 1);
    assert!(topo.get_stream(stream_id).is_some());
    assert_eq!(topo.downstream_equipment(eq_a), vec![eq_b]);
    assert_eq!(topo.upstream_equipment(eq_b), vec![eq_a]);
    assert_eq!(topo.streams_at_port(eq_a, p_out).len(), 1);
    assert_eq!(topo.streams_at_port(eq_b, p_in).len(), 1);
}

// ===========================================================================
// 2. Port Validation & Invariants
// ===========================================================================

#[test]
fn test_validation_nonexistent_equipment() {
    let mut topo = TopologyGraph::new();
    let stream = StreamDefinition::new(
        "S1",
        StreamType::Material,
        EquipmentId::new(),
        PortId::new(),
        EquipmentId::new(),
        PortId::new(),
    );
    let res = topo.add_stream(stream);
    assert!(matches!(res, Err(TopologyError::EquipmentNotFound { .. })));
}

#[test]
fn test_validation_nonexistent_port() {
    let mut topo = TopologyGraph::new();
    let def = make_def("Vessel", vec![]);
    let def_id = def.id;
    topo.register_definition(def);

    let eq_a = EquipmentId::new();
    let eq_b = EquipmentId::new();
    topo.add_equipment(eq_a, def_id).unwrap();
    topo.add_equipment(eq_b, def_id).unwrap();

    let stream = StreamDefinition::new(
        "S1",
        StreamType::Material,
        eq_a,
        PortId::new(),
        eq_b,
        PortId::new(),
    );
    let res = topo.add_stream(stream);
    assert!(matches!(res, Err(TopologyError::PortNotFound { .. })));
}

#[test]
fn test_validation_incompatible_domain() {
    let mut topo = TopologyGraph::new();
    let p_mat = PortId::new();
    let def_a = make_def(
        "A",
        vec![make_material_port(
            p_mat,
            "out",
            PortDirection::Outlet,
            vec![Phase::Liquid],
            Cardinality::ExactlyOne,
        )],
    );
    let p_elec = PortId::new();
    let def_b = make_def(
        "B",
        vec![PortDefinition {
            id: p_elec,
            name: "power".to_owned(),
            domain: PortDomain::Electrical,
            direction: PortDirection::Inlet,
            phase_compatibility: vec![],
            required: true,
            cardinality: Cardinality::ExactlyOne,
            description: String::new(),
        }],
    );

    let def_a_id = def_a.id;
    let def_b_id = def_b.id;
    topo.register_definition(def_a);
    topo.register_definition(def_b);

    let eq_a = EquipmentId::new();
    let eq_b = EquipmentId::new();
    topo.add_equipment(eq_a, def_a_id).unwrap();
    topo.add_equipment(eq_b, def_b_id).unwrap();

    let stream = StreamDefinition::new("S1", StreamType::Material, eq_a, p_mat, eq_b, p_elec);
    let res = topo.add_stream(stream);
    assert!(matches!(res, Err(TopologyError::DomainMismatch { .. })));
}

#[test]
fn test_validation_incompatible_direction() {
    let mut topo = TopologyGraph::new();
    let p_in1 = PortId::new();
    let def_a = make_def(
        "A",
        vec![make_material_port(
            p_in1,
            "in1",
            PortDirection::Inlet,
            vec![Phase::Liquid],
            Cardinality::ExactlyOne,
        )],
    );
    let p_in2 = PortId::new();
    let def_b = make_def(
        "B",
        vec![make_material_port(
            p_in2,
            "in2",
            PortDirection::Inlet,
            vec![Phase::Liquid],
            Cardinality::ExactlyOne,
        )],
    );

    let def_a_id = def_a.id;
    let def_b_id = def_b.id;
    topo.register_definition(def_a);
    topo.register_definition(def_b);

    let eq_a = EquipmentId::new();
    let eq_b = EquipmentId::new();
    topo.add_equipment(eq_a, def_a_id).unwrap();
    topo.add_equipment(eq_b, def_b_id).unwrap();

    // Connecting Inlet to Inlet must be rejected
    let stream = StreamDefinition::new("S1", StreamType::Material, eq_a, p_in1, eq_b, p_in2);
    let res = topo.add_stream(stream);
    assert!(matches!(res, Err(TopologyError::DirectionMismatch { .. })));
}

#[test]
fn test_validation_incompatible_phase() {
    let mut topo = TopologyGraph::new();
    let p_out = PortId::new();
    let def_a = make_def(
        "LiquidOut",
        vec![make_material_port(
            p_out,
            "out",
            PortDirection::Outlet,
            vec![Phase::Liquid],
            Cardinality::ExactlyOne,
        )],
    );
    let p_in = PortId::new();
    let def_b = make_def(
        "GasIn",
        vec![make_material_port(
            p_in,
            "in",
            PortDirection::Inlet,
            vec![Phase::Gas],
            Cardinality::ExactlyOne,
        )],
    );

    let def_a_id = def_a.id;
    let def_b_id = def_b.id;
    topo.register_definition(def_a);
    topo.register_definition(def_b);

    let eq_a = EquipmentId::new();
    let eq_b = EquipmentId::new();
    topo.add_equipment(eq_a, def_a_id).unwrap();
    topo.add_equipment(eq_b, def_b_id).unwrap();

    let stream = StreamDefinition::new("S1", StreamType::Material, eq_a, p_out, eq_b, p_in);
    let res = topo.add_stream(stream);
    assert!(matches!(res, Err(TopologyError::PhaseMismatch { .. })));
}

#[test]
fn test_validation_cardinality_violation() {
    let mut topo = TopologyGraph::new();
    let p_out = PortId::new();
    let def_a = make_def(
        "SingleOut",
        vec![make_material_port(
            p_out,
            "out",
            PortDirection::Outlet,
            vec![Phase::Liquid],
            Cardinality::ExactlyOne,
        )],
    );
    let p_in1 = PortId::new();
    let def_b1 = make_def(
        "In1",
        vec![make_material_port(
            p_in1,
            "in",
            PortDirection::Inlet,
            vec![Phase::Liquid],
            Cardinality::ExactlyOne,
        )],
    );
    let p_in2 = PortId::new();
    let def_b2 = make_def(
        "In2",
        vec![make_material_port(
            p_in2,
            "in",
            PortDirection::Inlet,
            vec![Phase::Liquid],
            Cardinality::ExactlyOne,
        )],
    );

    let def_a_id = def_a.id;
    let def_b1_id = def_b1.id;
    let def_b2_id = def_b2.id;
    topo.register_definition(def_a);
    topo.register_definition(def_b1);
    topo.register_definition(def_b2);

    let eq_a = EquipmentId::new();
    let eq_b1 = EquipmentId::new();
    let eq_b2 = EquipmentId::new();
    topo.add_equipment(eq_a, def_a_id).unwrap();
    topo.add_equipment(eq_b1, def_b1_id).unwrap();
    topo.add_equipment(eq_b2, def_b2_id).unwrap();

    // First connection succeeds
    let s1 = StreamDefinition::new("S1", StreamType::Material, eq_a, p_out, eq_b1, p_in1);
    assert!(topo.add_stream(s1).is_ok());

    // Second connection from same ExactlyOne outlet must fail
    let s2 = StreamDefinition::new("S2", StreamType::Material, eq_a, p_out, eq_b2, p_in2);
    let res = topo.add_stream(s2);
    assert!(matches!(
        res,
        Err(TopologyError::CardinalityExceeded { .. })
    ));
}

#[test]
fn test_validation_duplicate_connection() {
    let mut topo = TopologyGraph::new();
    let p_out = PortId::new();
    let def_a = make_def(
        "OutMany",
        vec![make_material_port(
            p_out,
            "out",
            PortDirection::Outlet,
            vec![Phase::Liquid],
            Cardinality::OneOrMore,
        )],
    );
    let p_in = PortId::new();
    let def_b = make_def(
        "InMany",
        vec![make_material_port(
            p_in,
            "in",
            PortDirection::Inlet,
            vec![Phase::Liquid],
            Cardinality::OneOrMore,
        )],
    );

    let def_a_id = def_a.id;
    let def_b_id = def_b.id;
    topo.register_definition(def_a);
    topo.register_definition(def_b);

    let eq_a = EquipmentId::new();
    let eq_b = EquipmentId::new();
    topo.add_equipment(eq_a, def_a_id).unwrap();
    topo.add_equipment(eq_b, def_b_id).unwrap();

    let s1 = StreamDefinition::new("S1", StreamType::Material, eq_a, p_out, eq_b, p_in);
    assert!(topo.add_stream(s1).is_ok());

    // Duplicate connection between same ports must be rejected
    let s2 = StreamDefinition::new("S2", StreamType::Material, eq_a, p_out, eq_b, p_in);
    let res = topo.add_stream(s2);
    assert!(matches!(
        res,
        Err(TopologyError::DuplicateConnection { .. })
    ));
}

// ===========================================================================
// 3. Mutation Safety (Validate-Then-Mutate)
// ===========================================================================

#[test]
fn test_mutation_safety_leaves_topology_unchanged() {
    let mut topo = TopologyGraph::new();
    let p_out = PortId::new();
    let def_a = make_def(
        "A",
        vec![make_material_port(
            p_out,
            "out",
            PortDirection::Outlet,
            vec![Phase::Liquid],
            Cardinality::ExactlyOne,
        )],
    );
    let p_in = PortId::new();
    let def_b = make_def(
        "B",
        vec![make_material_port(
            p_in,
            "in",
            PortDirection::Inlet,
            vec![Phase::Gas],
            Cardinality::ExactlyOne,
        )],
    ); // Phase mismatch!

    let def_a_id = def_a.id;
    let def_b_id = def_b.id;
    topo.register_definition(def_a);
    topo.register_definition(def_b);

    let eq_a = EquipmentId::new();
    let eq_b = EquipmentId::new();
    topo.add_equipment(eq_a, def_a_id).unwrap();
    topo.add_equipment(eq_b, def_b_id).unwrap();

    let invalid_stream =
        StreamDefinition::new("S-Bad", StreamType::Material, eq_a, p_out, eq_b, p_in);
    let res = topo.add_stream(invalid_stream);
    assert!(res.is_err());

    // Verify topology remained completely clean
    assert_eq!(topo.stream_count(), 0);
    assert!(topo.downstream_equipment(eq_a).is_empty());
    assert!(topo.upstream_equipment(eq_b).is_empty());
    assert!(topo.streams_at_port(eq_a, p_out).is_empty());
    assert!(topo.streams_at_port(eq_b, p_in).is_empty());
}

// ===========================================================================
// 4. Graph Queries: Linear, Branching, Merging, Reachability
// ===========================================================================

#[test]
fn test_linear_branching_merging_graph() {
    let mut topo = TopologyGraph::new();

    // Node A (1 out) -> Node B (branching: 2 out) -> Node C & Node D -> Node E (merging: 2 in)
    let pa_out = PortId::new();
    let pb_in = PortId::new();
    let pb_out1 = PortId::new();
    let pb_out2 = PortId::new();
    let pc_in = PortId::new();
    let pc_out = PortId::new();
    let pd_in = PortId::new();
    let pd_out = PortId::new();
    let pe_in1 = PortId::new();
    let pe_in2 = PortId::new();

    let def_a = make_def(
        "A",
        vec![make_material_port(
            pa_out,
            "out",
            PortDirection::Outlet,
            vec![Phase::Liquid],
            Cardinality::ExactlyOne,
        )],
    );
    let def_b = make_def(
        "B",
        vec![
            make_material_port(
                pb_in,
                "in",
                PortDirection::Inlet,
                vec![Phase::Liquid],
                Cardinality::ExactlyOne,
            ),
            make_material_port(
                pb_out1,
                "out1",
                PortDirection::Outlet,
                vec![Phase::Liquid],
                Cardinality::ExactlyOne,
            ),
            make_material_port(
                pb_out2,
                "out2",
                PortDirection::Outlet,
                vec![Phase::Liquid],
                Cardinality::ExactlyOne,
            ),
        ],
    );
    let def_c = make_def(
        "C",
        vec![
            make_material_port(
                pc_in,
                "in",
                PortDirection::Inlet,
                vec![Phase::Liquid],
                Cardinality::ExactlyOne,
            ),
            make_material_port(
                pc_out,
                "out",
                PortDirection::Outlet,
                vec![Phase::Liquid],
                Cardinality::ExactlyOne,
            ),
        ],
    );
    let def_d = make_def(
        "D",
        vec![
            make_material_port(
                pd_in,
                "in",
                PortDirection::Inlet,
                vec![Phase::Liquid],
                Cardinality::ExactlyOne,
            ),
            make_material_port(
                pd_out,
                "out",
                PortDirection::Outlet,
                vec![Phase::Liquid],
                Cardinality::ExactlyOne,
            ),
        ],
    );
    let def_e = make_def(
        "E",
        vec![
            make_material_port(
                pe_in1,
                "in1",
                PortDirection::Inlet,
                vec![Phase::Liquid],
                Cardinality::ExactlyOne,
            ),
            make_material_port(
                pe_in2,
                "in2",
                PortDirection::Inlet,
                vec![Phase::Liquid],
                Cardinality::ExactlyOne,
            ),
        ],
    );

    let def_a_id = def_a.id;
    let def_b_id = def_b.id;
    let def_c_id = def_c.id;
    let def_d_id = def_d.id;
    let def_e_id = def_e.id;

    topo.register_definition(def_a);
    topo.register_definition(def_b);
    topo.register_definition(def_c);
    topo.register_definition(def_d);
    topo.register_definition(def_e);

    let eq_a = EquipmentId::new();
    let eq_b = EquipmentId::new();
    let eq_c = EquipmentId::new();
    let eq_d = EquipmentId::new();
    let eq_e = EquipmentId::new();

    topo.add_equipment(eq_a, def_a_id).unwrap();
    topo.add_equipment(eq_b, def_b_id).unwrap();
    topo.add_equipment(eq_c, def_c_id).unwrap();
    topo.add_equipment(eq_d, def_d_id).unwrap();
    topo.add_equipment(eq_e, def_e_id).unwrap();

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
        pb_out1,
        eq_c,
        pc_in,
    ))
    .unwrap();
    topo.add_stream(StreamDefinition::new(
        "B->D",
        StreamType::Material,
        eq_b,
        pb_out2,
        eq_d,
        pd_in,
    ))
    .unwrap();
    topo.add_stream(StreamDefinition::new(
        "C->E",
        StreamType::Material,
        eq_c,
        pc_out,
        eq_e,
        pe_in1,
    ))
    .unwrap();
    topo.add_stream(StreamDefinition::new(
        "D->E",
        StreamType::Material,
        eq_d,
        pd_out,
        eq_e,
        pe_in2,
    ))
    .unwrap();

    // Query reachability
    let reachable_from_a = topo.reachable_equipment(eq_a);
    assert_eq!(reachable_from_a.len(), 5);
    assert!(topo.path_exists(eq_a, eq_e));
    assert!(topo.path_exists(eq_b, eq_c));
    assert!(topo.path_exists(eq_b, eq_d));
    assert!(!topo.path_exists(eq_c, eq_d));
    assert!(!topo.path_exists(eq_e, eq_a));

    // Upstream & Downstream verification
    let mut down_b = topo.downstream_equipment(eq_b);
    down_b.sort_by_key(|id| id.to_string());
    let mut expected_down_b = vec![eq_c, eq_d];
    expected_down_b.sort_by_key(|id| id.to_string());
    assert_eq!(down_b, expected_down_b);

    let mut up_e = topo.upstream_equipment(eq_e);
    up_e.sort_by_key(|id| id.to_string());
    let mut expected_up_e = vec![eq_c, eq_d];
    expected_up_e.sort_by_key(|id| id.to_string());
    assert_eq!(up_e, expected_up_e);
}

#[test]
fn test_disconnected_components() {
    let mut topo = TopologyGraph::new();

    // Subgraph 1: A -> B
    let p_a = PortId::new();
    let p_b = PortId::new();
    let def_ab = make_def(
        "AB",
        vec![
            make_material_port(
                p_a,
                "out",
                PortDirection::Outlet,
                vec![Phase::Liquid],
                Cardinality::ExactlyOne,
            ),
            make_material_port(
                p_b,
                "in",
                PortDirection::Inlet,
                vec![Phase::Liquid],
                Cardinality::ExactlyOne,
            ),
        ],
    );
    let def_ab_id = def_ab.id;
    topo.register_definition(def_ab);

    let eq_a = EquipmentId::new();
    let eq_b = EquipmentId::new();
    topo.add_equipment(eq_a, def_ab_id).unwrap();
    topo.add_equipment(eq_b, def_ab_id).unwrap();
    topo.add_stream(StreamDefinition::new(
        "S-AB",
        StreamType::Material,
        eq_a,
        p_a,
        eq_b,
        p_b,
    ))
    .unwrap();

    // Subgraph 2: C (isolated)
    let eq_c = EquipmentId::new();
    topo.add_equipment(eq_c, def_ab_id).unwrap();

    let components = topo.connected_components();
    assert_eq!(components.len(), 2);
}

// ===========================================================================
// 5. SCC & Recycle Loop Detection (Tarjan)
// ===========================================================================

#[test]
fn test_scc_acyclic() {
    let mut topo = TopologyGraph::new();
    let p_out = PortId::new();
    let p_in = PortId::new();
    let def = make_def(
        "Node",
        vec![
            make_material_port(
                p_out,
                "out",
                PortDirection::Outlet,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
            make_material_port(
                p_in,
                "in",
                PortDirection::Inlet,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
        ],
    );
    let def_id = def.id;
    topo.register_definition(def);

    let n1 = EquipmentId::new();
    let n2 = EquipmentId::new();
    let n3 = EquipmentId::new();
    topo.add_equipment(n1, def_id).unwrap();
    topo.add_equipment(n2, def_id).unwrap();
    topo.add_equipment(n3, def_id).unwrap();

    topo.add_stream(StreamDefinition::new(
        "1->2",
        StreamType::Material,
        n1,
        p_out,
        n2,
        p_in,
    ))
    .unwrap();
    topo.add_stream(StreamDefinition::new(
        "2->3",
        StreamType::Material,
        n2,
        p_out,
        n3,
        p_in,
    ))
    .unwrap();

    let scc = topo.analyze_scc();
    assert_eq!(scc.components.len(), 3);
    assert!(
        scc.recycle_loops.is_empty(),
        "Acyclic graph must have no recycle loops"
    );
}

#[test]
fn test_scc_simple_cycle() {
    let mut topo = TopologyGraph::new();
    let p_out1 = PortId::new();
    let p_in1 = PortId::new();
    let p_out2 = PortId::new();
    let p_in2 = PortId::new();

    let def_a = make_def(
        "A",
        vec![
            make_material_port(
                p_out1,
                "out",
                PortDirection::Outlet,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
            make_material_port(
                p_in1,
                "in",
                PortDirection::Inlet,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
        ],
    );
    let def_b = make_def(
        "B",
        vec![
            make_material_port(
                p_out2,
                "out",
                PortDirection::Outlet,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
            make_material_port(
                p_in2,
                "in",
                PortDirection::Inlet,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
        ],
    );
    let def_a_id = def_a.id;
    let def_b_id = def_b.id;
    topo.register_definition(def_a);
    topo.register_definition(def_b);

    let eq_a = EquipmentId::new();
    let eq_b = EquipmentId::new();
    topo.add_equipment(eq_a, def_a_id).unwrap();
    topo.add_equipment(eq_b, def_b_id).unwrap();

    // A -> B and B -> A
    topo.add_stream(StreamDefinition::new(
        "A->B",
        StreamType::Material,
        eq_a,
        p_out1,
        eq_b,
        p_in2,
    ))
    .unwrap();
    topo.add_stream(StreamDefinition::new(
        "B->A",
        StreamType::Material,
        eq_b,
        p_out2,
        eq_a,
        p_in1,
    ))
    .unwrap();

    let scc = topo.analyze_scc();
    assert_eq!(scc.recycle_loops.len(), 1);
    assert_eq!(scc.recycle_loops[0].len(), 2);
}

#[test]
fn test_scc_two_independent_cycles() {
    let mut topo = TopologyGraph::new();
    let p_out = PortId::new();
    let p_in = PortId::new();
    let def = make_def(
        "Generic",
        vec![
            make_material_port(
                p_out,
                "out",
                PortDirection::Outlet,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
            make_material_port(
                p_in,
                "in",
                PortDirection::Inlet,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
        ],
    );
    let def_id = def.id;
    topo.register_definition(def);

    // Cycle 1: 1 <-> 2
    let c1_a = EquipmentId::new();
    let c1_b = EquipmentId::new();
    topo.add_equipment(c1_a, def_id).unwrap();
    topo.add_equipment(c1_b, def_id).unwrap();
    topo.add_stream(StreamDefinition::new(
        "1a->1b",
        StreamType::Material,
        c1_a,
        p_out,
        c1_b,
        p_in,
    ))
    .unwrap();
    topo.add_stream(StreamDefinition::new(
        "1b->1a",
        StreamType::Material,
        c1_b,
        p_out,
        c1_a,
        p_in,
    ))
    .unwrap();

    // Cycle 2: 3 <-> 4
    let c2_a = EquipmentId::new();
    let c2_b = EquipmentId::new();
    topo.add_equipment(c2_a, def_id).unwrap();
    topo.add_equipment(c2_b, def_id).unwrap();
    topo.add_stream(StreamDefinition::new(
        "2a->2b",
        StreamType::Material,
        c2_a,
        p_out,
        c2_b,
        p_in,
    ))
    .unwrap();
    topo.add_stream(StreamDefinition::new(
        "2b->2a",
        StreamType::Material,
        c2_b,
        p_out,
        c2_a,
        p_in,
    ))
    .unwrap();

    let scc = topo.analyze_scc();
    assert_eq!(
        scc.recycle_loops.len(),
        2,
        "Must detect two independent recycle loops"
    );
}

#[test]
fn test_scc_interacting_nested_cycles() {
    let mut topo = TopologyGraph::new();
    let p_out1 = PortId::new();
    let p_out2 = PortId::new();
    let p_in1 = PortId::new();
    let p_in2 = PortId::new();

    let def = make_def(
        "Multi",
        vec![
            make_material_port(
                p_out1,
                "out1",
                PortDirection::Outlet,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
            make_material_port(
                p_out2,
                "out2",
                PortDirection::Outlet,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
            make_material_port(
                p_in1,
                "in1",
                PortDirection::Inlet,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
            make_material_port(
                p_in2,
                "in2",
                PortDirection::Inlet,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
        ],
    );
    let def_id = def.id;
    topo.register_definition(def);

    // Nodes 1, 2, 3 in big loop 1 -> 2 -> 3 -> 1
    // Plus smaller loop 2 <-> 3
    let n1 = EquipmentId::new();
    let n2 = EquipmentId::new();
    let n3 = EquipmentId::new();
    topo.add_equipment(n1, def_id).unwrap();
    topo.add_equipment(n2, def_id).unwrap();
    topo.add_equipment(n3, def_id).unwrap();

    topo.add_stream(StreamDefinition::new(
        "1->2",
        StreamType::Material,
        n1,
        p_out1,
        n2,
        p_in1,
    ))
    .unwrap();
    topo.add_stream(StreamDefinition::new(
        "2->3",
        StreamType::Material,
        n2,
        p_out1,
        n3,
        p_in1,
    ))
    .unwrap();
    topo.add_stream(StreamDefinition::new(
        "3->1",
        StreamType::Material,
        n3,
        p_out1,
        n1,
        p_in1,
    ))
    .unwrap();
    topo.add_stream(StreamDefinition::new(
        "3->2",
        StreamType::Material,
        n3,
        p_out2,
        n2,
        p_in2,
    ))
    .unwrap();

    let scc = topo.analyze_scc();
    assert_eq!(
        scc.recycle_loops.len(),
        1,
        "Interacting cycles merge into one strongly connected component"
    );
    assert_eq!(scc.recycle_loops[0].len(), 3);
}

// ===========================================================================
// 6. Multi-Network Topology (Material, Utility, Signal)
// ===========================================================================

#[test]
fn test_utility_network_representation() {
    let mut topo = TopologyGraph::new();

    let cw_supply_p = PortId::new();
    let cw_return_p = PortId::new();
    let hex_cw_in = PortId::new();
    let hex_cw_out = PortId::new();

    let def_cw = make_def(
        "CW Header",
        vec![
            make_utility_port(
                cw_supply_p,
                "supply",
                UtilityKind::CoolingWater,
                PortDirection::Outlet,
                Cardinality::OneOrMore,
            ),
            make_utility_port(
                cw_return_p,
                "return",
                UtilityKind::CoolingWater,
                PortDirection::Inlet,
                Cardinality::OneOrMore,
            ),
        ],
    );
    let def_hex = make_def(
        "Exchanger",
        vec![
            make_utility_port(
                hex_cw_in,
                "cw_in",
                UtilityKind::CoolingWater,
                PortDirection::Inlet,
                Cardinality::ExactlyOne,
            ),
            make_utility_port(
                hex_cw_out,
                "cw_out",
                UtilityKind::CoolingWater,
                PortDirection::Outlet,
                Cardinality::ExactlyOne,
            ),
        ],
    );

    let def_cw_id = def_cw.id;
    let def_hex_id = def_hex.id;
    topo.register_definition(def_cw);
    topo.register_definition(def_hex);

    let eq_cw = EquipmentId::new();
    let eq_hex = EquipmentId::new();
    topo.add_equipment(eq_cw, def_cw_id).unwrap();
    topo.add_equipment(eq_hex, def_hex_id).unwrap();

    // CW Supply -> Heat Exchanger -> CW Return
    let s_supply = StreamDefinition::new(
        "CW-Supply",
        StreamType::UtilityCoolingWater,
        eq_cw,
        cw_supply_p,
        eq_hex,
        hex_cw_in,
    );
    let s_return = StreamDefinition::new(
        "CW-Return",
        StreamType::UtilityCoolingWater,
        eq_hex,
        hex_cw_out,
        eq_cw,
        cw_return_p,
    );

    assert!(topo.add_stream(s_supply).is_ok());
    assert!(topo.add_stream(s_return).is_ok());
    assert_eq!(topo.stream_count(), 2);
}

#[test]
fn test_instrument_signal_topology() {
    let mut topo = TopologyGraph::new();

    let pt_proc = PortId::new();
    let pt_sig = PortId::new();
    let fcv_proc = PortId::new();
    let fcv_sig = PortId::new();

    let def_pt = make_def(
        "PT-101",
        vec![
            make_material_port(
                pt_proc,
                "tap",
                PortDirection::Inlet,
                vec![Phase::Liquid],
                Cardinality::ExactlyOne,
            ),
            make_signal_port(pt_sig, "sig_out", PortDirection::Outlet),
        ],
    );
    let def_fcv = make_def(
        "FCV-101",
        vec![
            make_material_port(
                fcv_proc,
                "valve_in",
                PortDirection::Inlet,
                vec![Phase::Liquid],
                Cardinality::ExactlyOne,
            ),
            make_signal_port(fcv_sig, "actuator_in", PortDirection::Inlet),
        ],
    );

    let def_pt_id = def_pt.id;
    let def_fcv_id = def_fcv.id;
    topo.register_definition(def_pt);
    topo.register_definition(def_fcv);

    let eq_pt = EquipmentId::new();
    let eq_fcv = EquipmentId::new();
    topo.add_equipment(eq_pt, def_pt_id).unwrap();
    topo.add_equipment(eq_fcv, def_fcv_id).unwrap();

    let pv_tag = TagId::new();
    // Signal 1: PT-101 Port -> PV Tag
    let sig1 = SignalConnection::new(
        "PT-to-PV",
        SignalEndpoint::Port {
            equipment_id: eq_pt,
            port_id: pt_sig,
        },
        SignalEndpoint::Tag(pv_tag),
        SignalMedium::AnalogCurrent,
    );
    assert!(topo.add_signal(sig1).is_ok());

    // Signal 2: Controller Output Tag -> FCV Actuator Port
    let mv_tag = TagId::new();
    let sig2 = SignalConnection::new(
        "MV-to-Valve",
        SignalEndpoint::Tag(mv_tag),
        SignalEndpoint::Port {
            equipment_id: eq_fcv,
            port_id: fcv_sig,
        },
        SignalMedium::Pneumatic,
    );
    assert!(topo.add_signal(sig2).is_ok());

    assert_eq!(topo.signal_count(), 2);
    assert_eq!(topo.signals_from_equipment(eq_pt).len(), 1);
    assert_eq!(topo.signals_to_equipment(eq_fcv).len(), 1);
}

// ===========================================================================
// 7. Physical Piping Model
// ===========================================================================

#[test]
fn test_pump_pipe_valve_vessel_topology() {
    let mut topo = TopologyGraph::new();

    let p_pump_out = PortId::new();
    let p_pipe1_a = PortId::new();
    let p_pipe1_b = PortId::new();
    let p_valve_in = PortId::new();
    let p_valve_out = PortId::new();
    let p_pipe2_a = PortId::new();
    let p_pipe2_b = PortId::new();
    let p_vessel_in = PortId::new();

    let def_pump = make_def(
        "Pump",
        vec![make_material_port(
            p_pump_out,
            "out",
            PortDirection::Outlet,
            vec![Phase::Liquid],
            Cardinality::OneOrMore,
        )],
    );
    let def_pipe_eq1 = make_def(
        "PipeManifold1",
        vec![
            make_material_port(
                p_pipe1_a,
                "end_a",
                PortDirection::Bidirectional,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
            make_material_port(
                p_pipe1_b,
                "end_b",
                PortDirection::Bidirectional,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
        ],
    );
    let def_valve = make_def(
        "Valve",
        vec![
            make_material_port(
                p_valve_in,
                "in",
                PortDirection::Inlet,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
            make_material_port(
                p_valve_out,
                "out",
                PortDirection::Outlet,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
        ],
    );
    let def_pipe_eq2 = make_def(
        "PipeManifold2",
        vec![
            make_material_port(
                p_pipe2_a,
                "end_a",
                PortDirection::Bidirectional,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
            make_material_port(
                p_pipe2_b,
                "end_b",
                PortDirection::Bidirectional,
                vec![Phase::Liquid],
                Cardinality::OneOrMore,
            ),
        ],
    );
    let def_vessel = make_def(
        "Vessel",
        vec![make_material_port(
            p_vessel_in,
            "in",
            PortDirection::Inlet,
            vec![Phase::Liquid],
            Cardinality::OneOrMore,
        )],
    );

    let dpump = def_pump.id;
    let dpipe1 = def_pipe_eq1.id;
    let dvalve = def_valve.id;
    let dpipe2 = def_pipe_eq2.id;
    let dvessel = def_vessel.id;

    topo.register_definition(def_pump);
    topo.register_definition(def_pipe_eq1);
    topo.register_definition(def_valve);
    topo.register_definition(def_pipe_eq2);
    topo.register_definition(def_vessel);

    let eq_pump = EquipmentId::new();
    let eq_p1 = EquipmentId::new();
    let eq_valve = EquipmentId::new();
    let eq_p2 = EquipmentId::new();
    let eq_vessel = EquipmentId::new();

    topo.add_equipment(eq_pump, dpump).unwrap();
    topo.add_equipment(eq_p1, dpipe1).unwrap();
    topo.add_equipment(eq_valve, dvalve).unwrap();
    topo.add_equipment(eq_p2, dpipe2).unwrap();
    topo.add_equipment(eq_vessel, dvessel).unwrap();

    // Connect using pipe segments
    let pipe1 = PipeSegment::new(
        "P-101",
        PipeEndpoint {
            equipment_id: eq_pump,
            port_id: p_pump_out,
        },
        PipeEndpoint {
            equipment_id: eq_valve,
            port_id: p_valve_in,
        },
    );
    let pipe2 = PipeSegment::new(
        "P-102",
        PipeEndpoint {
            equipment_id: eq_valve,
            port_id: p_valve_out,
        },
        PipeEndpoint {
            equipment_id: eq_vessel,
            port_id: p_vessel_in,
        },
    );

    assert!(topo.add_pipe(pipe1).is_ok());
    assert!(topo.add_pipe(pipe2).is_ok());
    assert_eq!(topo.pipe_count(), 2);

    // Verify pipe neighbors
    let mut neighbors = topo.piping_neighbors(eq_valve);
    neighbors.sort_by_key(|a| a.to_string());
    let mut expected = vec![eq_pump, eq_vessel];
    expected.sort_by_key(|a| a.to_string());
    assert_eq!(neighbors, expected);
    assert_eq!(topo.piping_neighbors(eq_pump), vec![eq_valve]);
    assert_eq!(topo.pipes_at_port(eq_pump, p_pump_out).len(), 1);
}

#[test]
fn test_pipe_bidirectional_opposite_endpoint() {
    let p_a = PortId::new();
    let p_b = PortId::new();
    let eq_a = EquipmentId::new();
    let eq_b = EquipmentId::new();

    let pipe = PipeSegment::new(
        "Pipe-AB",
        PipeEndpoint {
            equipment_id: eq_a,
            port_id: p_a,
        },
        PipeEndpoint {
            equipment_id: eq_b,
            port_id: p_b,
        },
    );

    assert!(pipe.connects_to(eq_a, p_a));
    assert!(pipe.connects_to(eq_b, p_b));
    assert_eq!(
        pipe.opposite_endpoint(eq_a, p_a),
        Some(&PipeEndpoint {
            equipment_id: eq_b,
            port_id: p_b
        })
    );
    assert_eq!(
        pipe.opposite_endpoint(eq_b, p_b),
        Some(&PipeEndpoint {
            equipment_id: eq_a,
            port_id: p_a
        })
    );
}
