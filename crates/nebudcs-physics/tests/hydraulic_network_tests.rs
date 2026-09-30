//! Scientific validation and acceptance plant test suite for NebuDCS Stage 6.
//!
//! Validates:
//! 1. Darcy-Weisbach friction & Churchill equation across laminar, transitional, turbulent regimes.
//! 2. Mass conservation at network junctions (sum m_in = sum m_out).
//! 3. Acceptance Plant A: Branched Hydraulic Network (1 Tank -> Pump -> Tee Junction -> 2 Valves -> 2 Tanks).
//! 4. Acceptance Plant B: Pump + NPSH Cavitation & Vibration Causal Chain.
//! 5. Acceptance Plant C: Thermal Recycle Loop Convergence (Wegstein acceleration).
//! 6. Acceptance Plant D: Compressor Network + Anti-Surge Recycle Valve Dynamics.
//! 7. 10,000-tick bit-identical determinism benchmark.

use nebudcs_physics::network::{
    FlowRegime, HydraulicEdge, HydraulicNetwork, HydraulicNetworkSolver, HydraulicNode,
    RecycleConvergenceManager,
};
use nebudcs_physics::rotating::{
    CavitationRegime, CompressorMapModel, CompressorOperatingState, PumpCavitationModel,
};
use nebudcs_units::Pressure;

// ---------------------------------------------------------------------------
// 1. Darcy-Weisbach & Churchill Friction Factor Across Regimes
// ---------------------------------------------------------------------------

#[test]
fn test_churchill_friction_factor_laminar_transition_turbulent() {
    let edge = HydraulicEdge::new_pipe("p1", "TestPipe", "n1", "n2", 100.0, 0.1, 0.000045, 1.5);

    // 1. Laminar Regime: Re = 1000
    // Analytical Hagen-Poiseuille: f = 64 / Re = 0.064
    let f_laminar = edge.churchill_friction_factor(1000.0, 0.000045, 0.1);
    let f_expected_laminar = 64.0 / 1000.0;
    let err_laminar = (f_laminar - f_expected_laminar).abs() / f_expected_laminar;
    assert!(
        err_laminar < 0.005,
        "Laminar friction factor error too large: f={f_laminar}, expected={f_expected_laminar}"
    );

    // 2. Turbulent Regime: Re = 100,000, e/D = 0.00045
    // Swamee-Jain / Colebrook-White reference: f ≈ 0.018 - 0.022
    let f_turb = edge.churchill_friction_factor(100_000.0, 0.000045, 0.1);
    assert!(
        f_turb > 0.016 && f_turb < 0.024,
        "Turbulent friction factor out of bounds: f={f_turb}"
    );

    // 3. Transition Regime: Re = 2500
    // Must be smooth, strictly positive, and bounded between laminar and turbulent values
    let f_trans = edge.churchill_friction_factor(2500.0, 0.000045, 0.1);
    assert!(
        f_trans > 0.01 && f_trans < 0.08,
        "Transition friction factor invalid: f={f_trans}"
    );

    // 4. Regime classification check
    assert_eq!(edge.diagnose_regime(500.0), FlowRegime::Laminar);
    assert_eq!(edge.diagnose_regime(2500.0), FlowRegime::Transitional);
    assert_eq!(edge.diagnose_regime(50_000.0), FlowRegime::Turbulent);
}

// ---------------------------------------------------------------------------
// 2. Mass Conservation Across Network Junctions
// ---------------------------------------------------------------------------

#[test]
fn test_mass_conservation_at_junction() {
    let mut net = HydraulicNetwork::new();

    // Boundary 1: High pressure supply (300 kPa)
    net.add_node(HydraulicNode::new_boundary(
        "src",
        "SupplyHeader",
        Pressure::new_unchecked(300_000.0),
        0.0,
    ));

    // Internal Junction: Tee (unknown pressure to be solved)
    net.add_node(HydraulicNode::new_junction("tee", "MixTee", 0.0));

    // Boundary 2: Low pressure discharge A (100 kPa)
    net.add_node(HydraulicNode::new_boundary(
        "snk_a",
        "DischargeTankA",
        Pressure::new_unchecked(100_000.0),
        0.0,
    ));

    // Boundary 3: Low pressure discharge B (120 kPa)
    net.add_node(HydraulicNode::new_boundary(
        "snk_b",
        "DischargeTankB",
        Pressure::new_unchecked(120_000.0),
        0.0,
    ));

    // Inflow branch: src -> tee
    net.add_edge(HydraulicEdge::new_pipe(
        "pipe_in",
        "InletPipe",
        "src",
        "tee",
        20.0,
        0.15,
        0.000045,
        1.0,
    ));

    // Outflow branch A: tee -> snk_a
    net.add_edge(HydraulicEdge::new_pipe(
        "pipe_out_a",
        "OutletA",
        "tee",
        "snk_a",
        30.0,
        0.10,
        0.000045,
        2.0,
    ));

    // Outflow branch B: tee -> snk_b
    net.add_edge(HydraulicEdge::new_pipe(
        "pipe_out_b",
        "OutletB",
        "tee",
        "snk_b",
        40.0,
        0.10,
        0.000045,
        2.0,
    ));

    let solver = HydraulicNetworkSolver::new();
    let diags = solver.solve(&mut net).expect("Network solve must converge");

    assert!(diags.converged, "Solver must indicate convergence");

    // Retrieve mass flows
    let m_in = net.edges[0].mass_flow.value();
    let m_out_a = net.edges[1].mass_flow.value();
    let m_out_b = net.edges[2].mass_flow.value();

    assert!(m_in > 0.0, "Inflow must be positive: {m_in}");
    assert!(m_out_a > 0.0, "Outflow A must be positive: {m_out_a}");
    assert!(m_out_b > 0.0, "Outflow B must be positive: {m_out_b}");

    // Mass conservation residual at junction: m_in - (m_out_a + m_out_b) = 0
    let mass_imbalance = (m_in - (m_out_a + m_out_b)).abs();
    assert!(
        mass_imbalance < 1e-4,
        "Mass conservation violated at junction: In={m_in}, OutA={m_out_a}, OutB={m_out_b}, Diff={mass_imbalance}"
    );
}

// ---------------------------------------------------------------------------
// 3. Acceptance Plant A: Branched Hydraulic Network
// Tank -> Pump -> Tee Junction -> 2 Valves -> 2 Tanks
// ---------------------------------------------------------------------------

#[test]
fn test_acceptance_plant_a_branched_network() {
    let mut net = HydraulicNetwork::new();

    // Source Tank: Atmospheric head 101.325 kPa + 5m liquid level = 150.3 kPa
    let p_feed = 101_325.0 + 1000.0 * 9.80665 * 5.0;
    net.add_node(HydraulicNode::new_boundary(
        "tk_feed",
        "FeedTank",
        Pressure::new_unchecked(p_feed),
        0.0,
    ));

    // Pump suction node
    net.add_node(HydraulicNode::new_junction("p_suct", "PumpSuction", 0.0));

    // Pump discharge / distribution manifold tee
    net.add_node(HydraulicNode::new_junction("tee_dist", "DistTee", 0.0));

    // Discharge Tank 1 (101.3 kPa, 2m elevation)
    net.add_node(HydraulicNode::new_boundary(
        "tk_dest1",
        "DestTank1",
        Pressure::new_unchecked(101_325.0),
        2.0,
    ));

    // Discharge Tank 2 (120 kPa pressurized blanket, 0m elevation)
    net.add_node(HydraulicNode::new_boundary(
        "tk_dest2",
        "DestTank2",
        Pressure::new_unchecked(120_000.0),
        0.0,
    ));

    // Edge 1: Tank to Pump Suction pipe
    net.add_edge(HydraulicEdge::new_pipe(
        "e_suct",
        "SuctionLine",
        "tk_feed",
        "p_suct",
        5.0,
        0.15,
        0.000045,
        0.5,
    ));

    // Edge 2: Centrifugal Pump (shutoff delta P = 800 kPa, flow_coeff = 500)
    let pump_edge = HydraulicEdge::new_pump(
        "e_pump", "FeedPump", "p_suct", "tee_dist", 800_000.0, 500.0, 1.0,
    );
    net.add_edge(pump_edge);

    // Edge 3: Branch 1 with Control Valve (Cv = 80, 100% open)
    let valve1_edge = HydraulicEdge::new_piping_with_valve(
        "e_br1",
        "BranchLine1",
        "tee_dist",
        "tk_dest1",
        25.0,
        0.10,
        0.000045,
        1.5,
        80.0,
        1.0,
    );
    net.add_edge(valve1_edge);

    // Edge 4: Branch 2 with Control Valve (Cv = 80, 50% throttled)
    let valve2_edge = HydraulicEdge::new_piping_with_valve(
        "e_br2",
        "BranchLine2",
        "tee_dist",
        "tk_dest2",
        30.0,
        0.10,
        0.000045,
        1.5,
        80.0,
        0.5,
    );
    net.add_edge(valve2_edge);

    let solver = HydraulicNetworkSolver::new();
    let diags = solver.solve(&mut net).expect("Plant A solve must succeed");
    println!(
        "Plant A diags: converged={}, iters={}, max_res={}, err={:?}",
        diags.converged, diags.iterations, diags.max_residual, diags.error_message
    );
    assert!(
        diags.converged,
        "Plant A failed to converge: {:?}",
        diags.error_message
    );
    let m_pump = net.edges[1].mass_flow.value();
    let m_br1 = net.edges[2].mass_flow.value();
    let m_br2 = net.edges[3].mass_flow.value();

    assert!(m_pump > 15.0, "Pump flow must be substantial: {m_pump}");
    assert!(m_br1 > 0.0);
    assert!(m_br2 > 0.0);

    // Branch 1 (100% open) should carry more flow than Branch 2 (50% open + higher backpressure)
    assert!(
        m_br1 > m_br2,
        "Branch 1 flow ({m_br1}) should exceed Branch 2 ({m_br2})"
    );

    // Mass balance: m_pump = m_br1 + m_br2
    let err = (m_pump - (m_br1 + m_br2)).abs();
    assert!(err < 1e-4, "Mass balance error: {err}");

    // Dynamic redistribution test: Throttle Valve 1 to 20%
    net.edges[2].set_opening(0.20);
    let diags_throttled = solver.solve(&mut net).expect("Throttled solve");
    assert!(diags_throttled.converged);

    let m_pump_th = net.edges[1].mass_flow.value();
    let m_br1_th = net.edges[2].mass_flow.value();
    let _m_br2_th = net.edges[3].mass_flow.value();

    assert!(
        m_br1_th < m_br1,
        "Branch 1 flow must decrease after throttling: was {m_br1}, now {m_br1_th}"
    );
    assert!(
        m_pump_th < m_pump,
        "Total pump flow must decrease when downstream resistance increases"
    );
}

// ---------------------------------------------------------------------------
// 4. Acceptance Plant B: Pump + NPSH Cavitation & Vibration Causal Chain
// ---------------------------------------------------------------------------

#[test]
fn test_acceptance_plant_b_npsh_cavitation_and_vibration_causality() {
    let mut cavitation_model = PumpCavitationModel::new(0.05, 3000.0, 3.2, 1000.0).unwrap();

    // 1. High Level: Tank at 80% (height 8m, p_atm = 101.3 kPa)
    // Suction Pressure = 101.325 kPa + 1000 * 9.80665 * 8.0 - friction_loss (10 kPa) ≈ 170 kPa
    cavitation_model.suction_pressure = Pressure::new(170_000.0).unwrap();
    cavitation_model.evaluate_cavitation(45.0, 1.0);

    assert_eq!(
        cavitation_model.last_regime,
        CavitationRegime::Safe,
        "High liquid level must maintain Safe cavitation regime"
    );
    assert_eq!(cavitation_model.last_head_degradation, 1.0);
    let vib_normal = cavitation_model.last_vibration_mm_s;
    assert!(
        vib_normal < 3.0,
        "Normal baseline vibration should be low: {vib_normal} mm/s"
    );

    // 2. Liquid Level Drops to 20% (height 2m): Suction Pressure = 110 kPa
    cavitation_model.suction_pressure = Pressure::new(110_000.0).unwrap();
    cavitation_model.evaluate_cavitation(45.0, 1.0);
    assert!(
        cavitation_model.last_cavitation_margin_m > 0.0,
        "Cavitation margin still positive at 20% level"
    );

    // 3. Liquid Level Drops to 5% with partially blocked strainer: Suction Pressure = 25 kPa
    // NPSHA = (25,000 - 3,169) / (1000 * 9.80665) = 2.22 m
    // NPSHR at 45 kg/s ≈ 2.8 m -> NPSHA < NPSHR!
    cavitation_model.suction_pressure = Pressure::new(25_000.0).unwrap();
    cavitation_model.evaluate_cavitation(45.0, 1.0);

    assert!(
        cavitation_model.last_regime == CavitationRegime::Incipient
            || cavitation_model.last_regime == CavitationRegime::Cavitation,
        "Sub-NPSHR suction pressure must trigger Incipient or Cavitation regime"
    );

    // 4. Extreme Starvation: Suction Pressure = 8 kPa (Flash condition)
    cavitation_model.suction_pressure = Pressure::new(8_000.0).unwrap();
    cavitation_model.evaluate_cavitation(45.0, 1.0);

    assert_eq!(cavitation_model.last_regime, CavitationRegime::Cavitation);
    assert!(cavitation_model.last_cavitation_margin_m < 0.0);
    assert!(
        cavitation_model.last_head_degradation < 0.90,
        "Severe head degradation: {:.2}",
        cavitation_model.last_head_degradation
    );
    let vib_cavitating = cavitation_model.last_vibration_mm_s;
    assert!(
        vib_cavitating > vib_normal * 2.5,
        "Vibration must spike during cavitation: {vib_cavitating} vs {vib_normal}"
    );
}

// ---------------------------------------------------------------------------
// 5. Acceptance Plant C: Thermal Recycle Loop Convergence (Wegstein)
// ---------------------------------------------------------------------------

#[test]
fn test_acceptance_plant_c_thermal_recycle_loop_convergence() {
    let manager = RecycleConvergenceManager::default();

    // Initial estimate of recycle mass flow and temperature:
    // Flow: 5.0 kg/s, Temp: 350 K
    let diags = manager
        .converge_loop("PlantC_Recycle", &[5.0, 350.0], |x| {
            let x_flow = x[0];
            let x_temp = x[1];

            let feed_flow = 20.0;
            let feed_temp = 293.15; // 20 C

            // 1. Mixer
            let total_flow = feed_flow + x_flow;
            let mixed_temp = (feed_flow * feed_temp + x_flow * x_temp) / total_flow;

            // 2. Pre-heater: adds heat duty
            let hx_temp = mixed_temp + 50.0;

            // 3. Exothermic CSTR Reactor
            let rx_temp = hx_temp + 30.0;

            // 4. Separator: Bottoms recycle = 25% of total flow
            let next_recycle_flow = total_flow * 0.25;
            let next_recycle_temp = rx_temp * 0.98;

            vec![next_recycle_flow, next_recycle_temp]
        })
        .expect("Thermal recycle loop must converge");

    assert!(
        diags.converged,
        "Thermal recycle loop must converge within iteration limit"
    );

    // Theoretical analytical balance:
    // total = 20 + R, R = total * 0.25 = 5 + 0.25 R => 0.75 R = 5 => R = 6.6667 kg/s
    let x_flow = diags.final_value[0];
    let expected_flow = 20.0 / 3.0;
    assert!(
        (x_flow - expected_flow).abs() < 1e-3,
        "Recycle flow converged to {x_flow}, expected {expected_flow}"
    );
}

// ---------------------------------------------------------------------------
// 6. Acceptance Plant D: Compressor Network + Stage 5 Anti-Surge Coupling
// ---------------------------------------------------------------------------

#[test]
fn test_acceptance_plant_d_compressor_antisurge_coupling() {
    let mut comp = CompressorMapModel::new_standard(10_000.0, 15.0);

    // Initial normal operation: 15 kg/s throughput at 100% speed
    comp.evaluate_performance(15.0, 1.0);
    assert_eq!(comp.last_operating_state, CompressorOperatingState::Normal);
    assert!(comp.last_surge_margin > 0.15); // > 15% surge margin

    // Process disturbance: downstream throttle valve closes, reducing suction flow to 5.0 kg/s
    // (Below surge line flow of 6.5 kg/s)
    comp.evaluate_performance(5.0, 1.0);
    assert_eq!(
        comp.last_operating_state,
        CompressorOperatingState::Surge,
        "Low flow must enter surge"
    );
    assert!(comp.last_surge_margin < 0.0);

    // Stage 5 Anti-Surge Controller Action:
    // Anti-surge control law: opening = clamp((surge_margin_sp - margin) * Kp, 0, 1)
    let surge_margin_sp = 0.10; // 10% safety margin setpoint
    let current_margin = comp.last_surge_margin;
    let anti_surge_valve_opening = ((surge_margin_sp - current_margin) * 2.5).clamp(0.0, 1.0);

    assert!(
        anti_surge_valve_opening > 0.50,
        "Anti-surge valve must open widely to protect compressor: opening={anti_surge_valve_opening}"
    );

    // Recycle flow recycled through anti-surge valve:
    let recycle_flow = anti_surge_valve_opening * 12.0; // max recycle capacity 12 kg/s
    let protected_suction_flow = 5.0 + recycle_flow;

    // Re-evaluate compressor operating point with anti-surge recycle flow active
    comp.evaluate_performance(protected_suction_flow, 1.0);

    assert_eq!(
        comp.last_operating_state,
        CompressorOperatingState::Normal,
        "Anti-surge recycle flow must restore compressor to Normal regime"
    );
    assert!(
        comp.last_surge_margin > 0.05,
        "Surge margin restored: {:.2}%",
        comp.last_surge_margin * 100.0
    );
}

// ---------------------------------------------------------------------------
// 7. 10,000-Tick Bit-Identical Determinism Benchmark
// ---------------------------------------------------------------------------

#[test]
fn test_10000_tick_bit_identical_determinism() {
    let run_sim = || {
        let mut net = HydraulicNetwork::new();
        net.add_node(HydraulicNode::new_boundary(
            "n_in",
            "Inlet",
            Pressure::new_unchecked(250_000.0),
            0.0,
        ));
        net.add_node(HydraulicNode::new_junction("n_junc", "Junction", 0.0));
        net.add_node(HydraulicNode::new_boundary(
            "n_out",
            "Outlet",
            Pressure::new_unchecked(101_325.0),
            0.0,
        ));

        net.add_edge(HydraulicEdge::new_pipe(
            "e1", "Pipe1", "n_in", "n_junc", 15.0, 0.1, 0.000045, 0.5,
        ));
        net.add_edge(HydraulicEdge::new_piping_with_valve(
            "e2",
            "ValvePipe",
            "n_junc",
            "n_out",
            20.0,
            0.1,
            0.000045,
            1.0,
            60.0,
            0.8,
        ));

        let solver = HydraulicNetworkSolver::new();
        let mut recorded_flows = Vec::with_capacity(10_000);

        for tick in 0..10_000 {
            // Modulate valve position slightly deterministically every 100 ticks
            if tick % 100 == 0 {
                let mod_pos = 0.5 + 0.3 * ((tick as f64) * 0.001).sin();
                net.edges[1].set_opening(mod_pos);
            }

            let _ = solver.solve(&mut net);
            recorded_flows.push(net.edges[1].mass_flow.value());
        }

        recorded_flows
    };

    let run_1 = run_sim();
    let run_2 = run_sim();

    assert_eq!(run_1.len(), 10_000);
    assert_eq!(run_2.len(), 10_000);

    for (i, (f1, f2)) in run_1.iter().zip(run_2.iter()).enumerate() {
        assert_eq!(
            f1.to_bits(),
            f2.to_bits(),
            "Bit-level determinism failure at tick {i}: run1={f1}, run2={f2}"
        );
    }
}
