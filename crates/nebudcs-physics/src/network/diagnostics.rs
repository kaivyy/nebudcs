//! Hydraulic network solver diagnostics and inspection structures.
//!
//! Provides detailed convergence statistics, per-node thermodynamic states,
//! and per-edge hydraulic metrics (velocities, Reynolds numbers, friction factors)
//! for Engineering Studio telemetry and operator monitoring.

use serde::{Deserialize, Serialize};

/// Detailed diagnostic state of an individual hydraulic node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeDiagnostic {
    pub id: String,
    pub name: String,
    pub pressure_pa: f64,
    pub pressure_bar: f64,
    pub temperature_c: f64,
    pub density_kg_m3: f64,
    pub enthalpy_j_kg: f64,
    pub phase: String,
    pub elevation_m: f64,
}

/// Classification of fluid flow regime based on Reynolds number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowRegime {
    Laminar,
    Transitional,
    Turbulent,
}

impl FlowRegime {
    pub fn from_reynolds(reynolds: f64) -> Self {
        if reynolds < 2000.0 {
            Self::Laminar
        } else if reynolds <= 4000.0 {
            Self::Transitional
        } else {
            Self::Turbulent
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Laminar => "Laminar",
            Self::Transitional => "Transitional",
            Self::Turbulent => "Turbulent",
        }
    }
}

/// Detailed diagnostic state of an individual hydraulic edge / branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeDiagnostic {
    pub id: String,
    pub name: String,
    pub from_node: String,
    pub to_node: String,
    pub mass_flow_kg_s: f64,
    pub volumetric_flow_m3_h: f64,
    pub pressure_drop_kpa: f64,
    pub velocity_m_s: f64,
    pub reynolds: f64,
    pub friction_factor: f64,
    pub flow_regime: String,
}

/// Full network diagnostics returned by [`super::solver::HydraulicNetworkSolver`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkDiagnostics {
    /// Whether the network solver converged within specified tolerances.
    pub converged: bool,
    /// Number of Newton / fixed-point iterations executed.
    pub iterations: usize,
    /// Maximum mass balance residual at any junction in kg/s: $\max_j |R_j|$.
    pub max_residual: f64,
    /// Euclidean $\ell_2$ or $\ell_\infty$ residual norm.
    pub residual_norm: f64,
    /// Diagnostics for every node in the network.
    pub nodes: Vec<NodeDiagnostic>,
    /// Diagnostics for every branch/edge in the network.
    pub edges: Vec<EdgeDiagnostic>,
    /// Offending node or edge ID with highest residual if non-convergent.
    pub offending_component: Option<String>,
    /// Detailed error explanation if solving failed.
    pub error_message: Option<String>,
}

impl Default for NetworkDiagnostics {
    fn default() -> Self {
        Self {
            converged: true,
            iterations: 0,
            max_residual: 0.0,
            residual_norm: 0.0,
            nodes: Vec::new(),
            edges: Vec::new(),
            offending_component: None,
            error_message: None,
        }
    }
}
