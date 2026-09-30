//! Hydraulic network solver using multidimensional Newton-Raphson nodal formulation.
//!
//! Solves for junction pressures and branch flows across branched, looped, and
//! recycle networks, ensuring rigorous mass conservation $\sum \dot{m}_{in} = \sum \dot{m}_{out}$
//! and thermodynamic enthalpy/composition propagation.

use std::collections::BTreeMap;

use nebudcs_units::{Density, Pressure, Temperature};

use super::diagnostics::{EdgeDiagnostic, NetworkDiagnostics, NodeDiagnostic};
use super::edge::HydraulicEdge;
use super::node::HydraulicNode;
use crate::error::PhysicsError;
use crate::numerics::{NewtonConfig, NewtonSolver};

/// A hydraulic network containing interconnected nodes and branches.
#[derive(Debug, Clone, PartialEq)]
pub struct HydraulicNetwork {
    /// Network nodes keyed by unique ID.
    pub nodes: BTreeMap<String, HydraulicNode>,
    /// Network edges / branches.
    pub edges: Vec<HydraulicEdge>,
    /// Baseline working fluid density in kg/m³.
    pub default_density: Density,
    /// Baseline dynamic viscosity in Pa·s (e.g. 0.001 for water, 1e-5 for gas).
    pub default_viscosity: f64,
}

impl Default for HydraulicNetwork {
    fn default() -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            default_density: Density::new_unchecked(1000.0),
            default_viscosity: 0.001,
        }
    }
}

impl HydraulicNetwork {
    /// Creates an empty hydraulic network.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a node to the network.
    pub fn add_node(&mut self, node: HydraulicNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Adds an edge to the network.
    pub fn add_edge(&mut self, edge: HydraulicEdge) {
        self.edges.push(edge);
    }

    /// Returns a reference to a node by ID.
    pub fn get_node(&self, id: &str) -> Option<&HydraulicNode> {
        self.nodes.get(id)
    }

    /// Returns a mutable reference to a node by ID.
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut HydraulicNode> {
        self.nodes.get_mut(id)
    }

    /// Returns a reference to an edge by ID.
    pub fn get_edge(&self, id: &str) -> Option<&HydraulicEdge> {
        self.edges.iter().find(|e| e.id == id)
    }

    /// Returns a mutable reference to an edge by ID.
    pub fn get_edge_mut(&mut self, id: &str) -> Option<&mut HydraulicEdge> {
        self.edges.iter_mut().find(|e| e.id == id)
    }

    /// Clears all nodes and edges.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
    }
}

/// Robust solver for hydraulic networks.
#[derive(Debug, Clone)]
pub struct HydraulicNetworkSolver {
    newton_config: NewtonConfig,
}

impl Default for HydraulicNetworkSolver {
    fn default() -> Self {
        Self {
            newton_config: NewtonConfig {
                max_iterations: 60,
                tolerance_abs: 1e-5, // 0.01 g/s mass balance residual
                tolerance_rel: 1e-5,
                step_tolerance: 1e-4,
                line_search: true,
                fd_epsilon: 1e-5,
            },
        }
    }
}

impl HydraulicNetworkSolver {
    /// Creates a solver with default numerical settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a solver with custom Newton solver settings.
    pub fn with_config(newton_config: NewtonConfig) -> Self {
        Self { newton_config }
    }

    /// Solves the network state, returning detailed convergence diagnostics.
    pub fn solve(
        &self,
        network: &mut HydraulicNetwork,
    ) -> Result<NetworkDiagnostics, PhysicsError> {
        // Collect internal node IDs whose pressures must be solved
        let internal_node_ids: Vec<String> = network
            .nodes
            .iter()
            .filter(|(_, n)| n.is_internal())
            .map(|(id, _)| id.clone())
            .collect();

        let n_internal = internal_node_ids.len();

        let mut iterations = 0;
        let mut max_residual = 0.0;
        let mut converged = true;
        let offending_comp: Option<String> = None;

        if n_internal > 0 {
            // Initial guess: current node pressures
            let initial_guess: Vec<f64> = internal_node_ids
                .iter()
                .map(|id| network.nodes[id].pressure.value())
                .collect();

            // Pre-collect boundary pressures and elevations
            let density = network.default_density;
            let viscosity = network.default_viscosity;

            let solver = NewtonSolver::with_config(self.newton_config);

            let node_ids_ref = &internal_node_ids;
            let edges_ref = &network.edges;
            let nodes_ref = &network.nodes;

            let residual_fn = |p_internal: &[f64]| -> Result<Vec<f64>, PhysicsError> {
                let mut residuals = vec![0.0; n_internal];

                // Temporary helper to get pressure for any node
                let get_p = |node_id: &str| -> f64 {
                    if let Some(idx) = node_ids_ref.iter().position(|id| id == node_id) {
                        p_internal[idx].max(100.0) // prevent negative absolute pressure
                    } else if let Some(node) = nodes_ref.get(node_id) {
                        node.pressure.value()
                    } else {
                        101_325.0
                    }
                };

                let get_z = |node_id: &str| -> f64 {
                    nodes_ref.get(node_id).map(|n| n.elevation).unwrap_or(0.0)
                };

                for edge in edges_ref {
                    let p_from = Pressure::new_unchecked(get_p(&edge.from_node));
                    let p_to = Pressure::new_unchecked(get_p(&edge.to_node));
                    let z_from = get_z(&edge.from_node);
                    let z_to = get_z(&edge.to_node);

                    let flow =
                        edge.calculate_flow(p_from, p_to, z_from, z_to, density, viscosity)?;

                    // If from_node is internal, flow leaving reduces mass
                    if let Some(from_idx) = node_ids_ref.iter().position(|id| id == &edge.from_node)
                    {
                        residuals[from_idx] -= flow;
                    }

                    // If to_node is internal, flow entering increases mass
                    if let Some(to_idx) = node_ids_ref.iter().position(|id| id == &edge.to_node) {
                        residuals[to_idx] += flow;
                    }
                }

                Ok(residuals)
            };

            match solver.solve(&initial_guess, residual_fn) {
                Ok(report) => {
                    iterations = report.iterations;
                    max_residual = report.final_residual;
                    converged = report.converged;

                    // Update solved internal node pressures
                    for (i, p_val) in report.solution.iter().enumerate() {
                        let node_id = &internal_node_ids[i];
                        if let Some(node) = network.nodes.get_mut(node_id) {
                            node.pressure = Pressure::new_unchecked(p_val.max(0.0));
                        }
                    }
                }
                Err(e) => {
                    return Ok(NetworkDiagnostics {
                        converged: false,
                        iterations: self.newton_config.max_iterations,
                        max_residual: f64::INFINITY,
                        residual_norm: f64::INFINITY,
                        nodes: Vec::new(),
                        edges: Vec::new(),
                        offending_component: internal_node_ids.first().cloned(),
                        error_message: Some(format!("Newton convergence failed: {e}")),
                    });
                }
            }
        }

        // Final pass: update edge flows, pressure drops, velocities, and diagnostics
        let density = network.default_density;
        let viscosity = network.default_viscosity;

        for edge in &mut network.edges {
            let p_from = network
                .nodes
                .get(&edge.from_node)
                .map(|n| n.pressure)
                .unwrap_or_else(|| Pressure::new_unchecked(101_325.0));
            let p_to = network
                .nodes
                .get(&edge.to_node)
                .map(|n| n.pressure)
                .unwrap_or_else(|| Pressure::new_unchecked(101_325.0));
            let z_from = network
                .nodes
                .get(&edge.from_node)
                .map(|n| n.elevation)
                .unwrap_or(0.0);
            let z_to = network
                .nodes
                .get(&edge.to_node)
                .map(|n| n.elevation)
                .unwrap_or(0.0);

            let flow = edge.calculate_flow(p_from, p_to, z_from, z_to, density, viscosity)?;
            edge.update_diagnostics(flow, p_from, p_to, density, viscosity);
        }

        // Thermal and enthalpy mixing at internal nodes
        for node_id in &internal_node_ids {
            let mut total_inflow = 0.0;
            let mut weighted_enthalpy = 0.0;

            for edge in &network.edges {
                if edge.to_node == *node_id && edge.mass_flow.value() > 0.0 {
                    let f = edge.mass_flow.value();
                    total_inflow += f;
                    let from_h = network
                        .nodes
                        .get(&edge.from_node)
                        .map(|n| n.enthalpy)
                        .unwrap_or(84_000.0);
                    weighted_enthalpy += f * from_h;
                } else if edge.from_node == *node_id && edge.mass_flow.value() < 0.0 {
                    let f = -edge.mass_flow.value();
                    total_inflow += f;
                    let to_h = network
                        .nodes
                        .get(&edge.to_node)
                        .map(|n| n.enthalpy)
                        .unwrap_or(84_000.0);
                    weighted_enthalpy += f * to_h;
                }
            }

            if total_inflow > 1e-6 {
                let mixed_h = weighted_enthalpy / total_inflow;
                if let Some(node) = network.nodes.get_mut(node_id) {
                    node.enthalpy = mixed_h;
                    // Standard liquid water cp approx 4184 J/(kg K)
                    let t_c = 20.0 + (mixed_h - 84_000.0) / 4184.0;
                    node.temperature = Temperature::new_unchecked((t_c + 273.15).max(1.0));
                }
            }
        }

        // Generate node diagnostics
        let mut node_diags = Vec::new();
        for node in network.nodes.values() {
            node_diags.push(NodeDiagnostic {
                id: node.id.clone(),
                name: node.name.clone(),
                pressure_pa: node.pressure.value(),
                pressure_bar: node.pressure.value() / 100_000.0,
                temperature_c: node.temperature.to_celsius(),
                density_kg_m3: node.density.value(),
                enthalpy_j_kg: node.enthalpy,
                phase: format!("{:?}", node.phase),
                elevation_m: node.elevation,
            });
        }

        // Generate edge diagnostics
        let mut edge_diags = Vec::new();
        for edge in &network.edges {
            let flow_kg_s = edge.mass_flow.value();
            let flow_m3_h = (flow_kg_s / density.value()) * 3600.0;
            let dp_kpa = edge.pressure_drop.value() / 1000.0;

            let regime = if edge.reynolds < 2300.0 {
                "Laminar".to_string()
            } else if edge.reynolds < 4000.0 {
                "Transitional".to_string()
            } else {
                "Turbulent".to_string()
            };

            edge_diags.push(EdgeDiagnostic {
                id: edge.id.clone(),
                name: edge.name.clone(),
                from_node: edge.from_node.clone(),
                to_node: edge.to_node.clone(),
                mass_flow_kg_s: flow_kg_s,
                volumetric_flow_m3_h: flow_m3_h,
                pressure_drop_kpa: dp_kpa,
                velocity_m_s: edge.velocity,
                reynolds: edge.reynolds,
                friction_factor: edge.friction_factor,
                flow_regime: regime,
            });
        }

        Ok(NetworkDiagnostics {
            converged,
            iterations,
            max_residual,
            residual_norm: max_residual,
            nodes: node_diags,
            edges: edge_diags,
            offending_component: offending_comp,
            error_message: None,
        })
    }
}
