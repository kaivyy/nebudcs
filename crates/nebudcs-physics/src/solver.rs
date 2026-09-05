//! Numerical solver routines for recycle loop convergence and nonlinear equations.
//!
//! Implements:
//! - Successive substitution
//! - Bounded Wegstein acceleration
//! - Convergence tracking, tolerance checks, and divergence guards

use serde::{Deserialize, Serialize};

use crate::error::PhysicsError;

/// Configuration options for iterative nonlinear solvers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SolverConfig {
    /// Maximum allowed iterations before declaring convergence failure.
    pub max_iterations: usize,
    /// Absolute residual convergence tolerance.
    pub tol_abs: f64,
    /// Relative convergence tolerance.
    pub tol_rel: f64,
    /// Under-relaxation damping factor $\lambda \in (0, 1.0]$.
    /// When Wegstein is disabled, $x_{k+1} = (1 - \lambda) x_k + \lambda y_k$.
    pub damping: f64,
    /// Whether to enable Wegstein acceleration (false = successive substitution with damping).
    pub use_wegstein: bool,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tol_abs: 1e-6,
            tol_rel: 1e-6,
            damping: 1.0,
            use_wegstein: true,
        }
    }
}

impl SolverConfig {
    /// Creates a solver configuration with mixed absolute and relative tolerances.
    pub fn new(max_iterations: usize, tol_abs: f64, tol_rel: f64, use_wegstein: bool) -> Self {
        Self {
            max_iterations,
            tol_abs,
            tol_rel,
            damping: 1.0,
            use_wegstein,
        }
    }

    /// Configures under-relaxation damping factor $\lambda \in (0, 1]$.
    pub fn with_damping(mut self, damping: f64) -> Self {
        self.damping = damping.clamp(0.001, 1.0);
        self
    }
}

/// Convergence report returned by the solver.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SolverReport<T> {
    pub value: T,
    pub iterations: usize,
    pub residual: f64,
    pub converged: bool,
}

/// Iterative solver for process recycle loops and nonlinear algebraic equations.
pub struct RecycleSolver {
    config: SolverConfig,
}

impl RecycleSolver {
    /// Creates a new recycle solver with the specified configuration.
    pub fn new(config: SolverConfig) -> Self {
        Self { config }
    }

    /// Solves a 1D scalar fixed-point equation $x = g(x)$ using successive substitution
    /// or bounded Wegstein acceleration.
    pub fn solve_scalar<F>(
        &self,
        initial_guess: f64,
        mut g: F,
    ) -> Result<SolverReport<f64>, PhysicsError>
    where
        F: FnMut(f64) -> f64,
    {
        let mut x_k = initial_guess;
        let mut x_prev = initial_guess;
        let mut y_prev = initial_guess;

        for k in 0..self.config.max_iterations {
            let y_k = g(x_k);

            if !y_k.is_finite() {
                return Err(PhysicsError::NonFiniteValue {
                    field: "solver_iterate".to_string(),
                    value: y_k,
                });
            }

            let residual = (y_k - x_k).abs();
            if residual > 1e12 {
                return Err(PhysicsError::RecycleConvergenceFailed {
                    iterations: k + 1,
                    max_iterations: self.config.max_iterations,
                    residual,
                    tolerance: self.config.tol_abs,
                });
            }

            let max_allowed = self.config.tol_abs + self.config.tol_rel * x_k.abs();
            if residual <= max_allowed {
                return Ok(SolverReport {
                    value: y_k,
                    iterations: k + 1,
                    residual,
                    converged: true,
                });
            }

            let x_next = if self.config.use_wegstein && k > 0 {
                let dx = x_k - x_prev;
                let dy = y_k - y_prev;

                if dx.abs() > 1e-12 {
                    let s = dy / dx;
                    if (s - 1.0).abs() > 1e-6 {
                        let q = (s / (s - 1.0)).clamp(-5.0, 0.0);
                        q * x_k + (1.0 - q) * y_k
                    } else {
                        y_k
                    }
                } else {
                    y_k
                }
            } else {
                (1.0 - self.config.damping) * x_k + self.config.damping * y_k
            };

            x_prev = x_k;
            y_prev = y_k;
            x_k = x_next;
        }

        let final_residual = (g(x_k) - x_k).abs();
        Err(PhysicsError::RecycleConvergenceFailed {
            iterations: self.config.max_iterations,
            max_iterations: self.config.max_iterations,
            residual: final_residual,
            tolerance: self.config.tol_abs,
        })
    }

    /// Solves an n-dimensional vector fixed-point equation $\mathbf{x} = \mathbf{g}(\mathbf{x})$.
    pub fn solve_vector<F>(
        &self,
        initial_guess: &[f64],
        mut g: F,
    ) -> Result<SolverReport<Vec<f64>>, PhysicsError>
    where
        F: FnMut(&[f64]) -> Vec<f64>,
    {
        let n = initial_guess.len();
        let mut x_k = initial_guess.to_vec();
        let mut x_prev = x_k.clone();
        let mut y_prev = x_k.clone();

        for k in 0..self.config.max_iterations {
            let y_k = g(&x_k);
            if y_k.len() != n {
                return Err(PhysicsError::InvalidOperatingCondition {
                    message: format!(
                        "solver function output dimension {} != input dimension {}",
                        y_k.len(),
                        n
                    ),
                });
            }

            let mut max_residual: f64 = 0.0;
            let mut converged = true;
            for i in 0..n {
                if !y_k[i].is_finite() {
                    return Err(PhysicsError::NonFiniteValue {
                        field: format!("solver_iterate_{i}"),
                        value: y_k[i],
                    });
                }
                let r = (y_k[i] - x_k[i]).abs();
                if r > max_residual {
                    max_residual = r;
                }
                let max_allowed = self.config.tol_abs + self.config.tol_rel * x_k[i].abs();
                if r > max_allowed {
                    converged = false;
                }
            }

            if max_residual > 1e12 {
                return Err(PhysicsError::RecycleConvergenceFailed {
                    iterations: k + 1,
                    max_iterations: self.config.max_iterations,
                    residual: max_residual,
                    tolerance: self.config.tol_abs,
                });
            }

            if converged {
                return Ok(SolverReport {
                    value: y_k,
                    iterations: k + 1,
                    residual: max_residual,
                    converged: true,
                });
            }

            let mut x_next = vec![0.0; n];
            for i in 0..n {
                if self.config.use_wegstein && k > 0 {
                    let dx = x_k[i] - x_prev[i];
                    let dy = y_k[i] - y_prev[i];
                    if dx.abs() > 1e-12 {
                        let s = dy / dx;
                        if (s - 1.0).abs() > 1e-6 {
                            let q = (s / (s - 1.0)).clamp(-5.0, 0.0);
                            x_next[i] = q * x_k[i] + (1.0 - q) * y_k[i];
                        } else {
                            x_next[i] = y_k[i];
                        }
                    } else {
                        x_next[i] = y_k[i];
                    }
                } else {
                    x_next[i] = (1.0 - self.config.damping) * x_k[i] + self.config.damping * y_k[i];
                }
            }

            x_prev = x_k;
            y_prev = y_k;
            x_k = x_next;
        }

        let final_y = g(&x_k);
        let mut final_residual: f64 = 0.0;
        for i in 0..n {
            let r = (final_y[i] - x_k[i]).abs();
            if r > final_residual {
                final_residual = r;
            }
        }

        Err(PhysicsError::RecycleConvergenceFailed {
            iterations: self.config.max_iterations,
            max_iterations: self.config.max_iterations,
            residual: final_residual,
            tolerance: self.config.tol_abs,
        })
    }
}
