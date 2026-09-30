//! Closed-loop recycle convergence manager with bounded Wegstein acceleration.
//!
//! Solves iterative fixed-point equations $\mathbf{x} = \mathbf{g}(\mathbf{x})$ for
//! process material and energy recycles, ensuring mass and component conservation.

use serde::{Deserialize, Serialize};

use crate::error::PhysicsError;
use crate::solver::{RecycleSolver, SolverConfig, SolverReport};

/// Diagnostics report for recycle loop convergence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecycleDiagnostics {
    pub loop_name: String,
    pub converged: bool,
    pub iterations: usize,
    pub residual_norm: f64,
    pub final_value: Vec<f64>,
}

/// Manages iterative convergence for recycle loops in a process flowsheet.
#[derive(Debug, Clone)]
pub struct RecycleConvergenceManager {
    solver_config: SolverConfig,
}

impl Default for RecycleConvergenceManager {
    fn default() -> Self {
        Self::new(SolverConfig {
            max_iterations: 100,
            tol_abs: 1e-6,
            tol_rel: 1e-6,
            damping: 0.8,
            use_wegstein: true,
        })
    }
}

impl RecycleConvergenceManager {
    /// Creates a recycle convergence manager with specified solver configuration.
    pub fn new(solver_config: SolverConfig) -> Self {
        Self { solver_config }
    }

    /// Solves an n-variable recycle vector (e.g. `[flow, enthalpy, z1, z2, ...]`) using
    /// bounded Wegstein acceleration.
    pub fn converge_loop<F>(
        &self,
        loop_name: impl Into<String>,
        initial_guess: &[f64],
        loop_fn: F,
    ) -> Result<RecycleDiagnostics, PhysicsError>
    where
        F: FnMut(&[f64]) -> Vec<f64>,
    {
        let solver = RecycleSolver::new(self.solver_config);
        let report: SolverReport<Vec<f64>> = solver.solve_vector(initial_guess, loop_fn)?;

        Ok(RecycleDiagnostics {
            loop_name: loop_name.into(),
            converged: report.converged,
            iterations: report.iterations,
            residual_norm: report.residual,
            final_value: report.value,
        })
    }
}
