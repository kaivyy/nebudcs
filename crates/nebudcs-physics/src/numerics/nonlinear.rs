//! Robust multidimensional Newton-Raphson nonlinear equation solver with analytical and numerical Jacobians.
//!
//! Implements:
//! - Analytical and finite-difference Jacobian evaluation
//! - Gaussian elimination with partial pivoting (LU decomposition)
//! - Armijo backtracking line search / damping
//! - Strict convergence criteria on residual norm and step size
//! - Singular Jacobian detection and divergence safeguards

use serde::{Deserialize, Serialize};

use crate::error::PhysicsError;

/// Configuration options for the multidimensional Newton-Raphson solver.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NewtonConfig {
    /// Maximum allowed Newton iterations (default: 50).
    pub max_iterations: usize,
    /// Absolute residual convergence tolerance $\|\mathbf{F}(\mathbf{x})\|_\infty$ (default: 1e-8).
    pub tolerance_abs: f64,
    /// Relative residual convergence tolerance (default: 1e-8).
    pub tolerance_rel: f64,
    /// Step size tolerance $\|\Delta \mathbf{x}\|_\infty$ (default: 1e-10).
    pub step_tolerance: f64,
    /// Whether to enable Armijo backtracking line search (default: true).
    pub line_search: bool,
    /// Finite difference step size $\epsilon$ for numerical Jacobian (default: 1e-7).
    pub fd_epsilon: f64,
}

impl Default for NewtonConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            tolerance_abs: 1e-8,
            tolerance_rel: 1e-8,
            step_tolerance: 1e-10,
            line_search: true,
            fd_epsilon: 1e-7,
        }
    }
}

/// Convergence report returned by the Newton-Raphson solver.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewtonReport {
    /// Final solution vector.
    pub solution: Vec<f64>,
    /// Number of iterations executed.
    pub iterations: usize,
    /// Final residual $\ell_\infty$ norm.
    pub final_residual: f64,
    /// Final step size $\ell_\infty$ norm.
    pub final_step: f64,
    /// Whether convergence criteria were satisfied.
    pub converged: bool,
}

/// Multidimensional Newton-Raphson solver.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NewtonSolver {
    config: NewtonConfig,
}

impl Default for NewtonSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl NewtonSolver {
    /// Creates a Newton solver with default configuration.
    pub fn new() -> Self {
        Self {
            config: NewtonConfig::default(),
        }
    }

    /// Creates a Newton solver with custom configuration.
    pub fn with_config(config: NewtonConfig) -> Self {
        Self { config }
    }

    /// Solves $\mathbf{F}(\mathbf{x}) = \mathbf{0}$ using numerical finite-difference Jacobian.
    pub fn solve<F>(
        &self,
        initial_guess: &[f64],
        mut residual_fn: F,
    ) -> Result<NewtonReport, PhysicsError>
    where
        F: FnMut(&[f64]) -> Result<Vec<f64>, PhysicsError>,
    {
        let n = initial_guess.len();
        if n == 0 {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: "cannot solve empty system".to_string(),
            });
        }

        let mut x = initial_guess.to_vec();
        let mut f_val = residual_fn(&x)?;
        if f_val.len() != n {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!(
                    "residual dimension {} != state dimension {}",
                    f_val.len(),
                    n
                ),
            });
        }

        let initial_norm = Self::norm_inf(&f_val);
        if initial_norm <= self.config.tolerance_abs {
            return Ok(NewtonReport {
                solution: x,
                iterations: 0,
                final_residual: initial_norm,
                final_step: 0.0,
                converged: true,
            });
        }

        let mut last_step_norm = 0.0;
        let mut last_res_norm = initial_norm;
        let _ = last_step_norm;

        for iter in 0..self.config.max_iterations {
            // Evaluate numerical Jacobian directly
            let jac =
                Self::numerical_jacobian(&x, &f_val, &mut residual_fn, self.config.fd_epsilon)?;

            // Solve linear system: J * delta_x = -F(x)
            let neg_f: Vec<f64> = f_val.iter().map(|&v| -v).collect();
            let delta_x = Self::solve_linear_system(&jac, &neg_f)?;

            last_step_norm = Self::norm_inf(&delta_x);

            // Step with optional Armijo backtracking line search
            let mut alpha = 1.0;
            let mut x_trial = vec![0.0; n];
            let mut f_trial = Vec::new();
            let mut trial_norm = f64::INFINITY;

            let min_alpha = if self.config.line_search {
                0.03125
            } else {
                1.0
            };

            while alpha >= min_alpha {
                for i in 0..n {
                    x_trial[i] = x[i] + alpha * delta_x[i];
                }

                if let Ok(res) = residual_fn(&x_trial) {
                    let norm = Self::norm_inf(&res);
                    if norm < last_res_norm || !self.config.line_search {
                        f_trial = res;
                        trial_norm = norm;
                        break;
                    }
                }
                alpha *= 0.5;
            }

            if f_trial.is_empty() {
                // If line search failed to find decrease, take undamped step
                for i in 0..n {
                    x_trial[i] = x[i] + delta_x[i];
                }
                f_trial = residual_fn(&x_trial)?;
                trial_norm = Self::norm_inf(&f_trial);
            }

            x = x_trial;
            f_val = f_trial;
            last_res_norm = trial_norm;

            let max_allowed = self.config.tolerance_abs + self.config.tolerance_rel * initial_norm;
            if last_res_norm <= max_allowed
                || (last_step_norm <= self.config.step_tolerance
                    && last_res_norm <= 10.0 * max_allowed)
            {
                return Ok(NewtonReport {
                    solution: x,
                    iterations: iter + 1,
                    final_residual: last_res_norm,
                    final_step: last_step_norm,
                    converged: true,
                });
            }
        }

        Err(PhysicsError::RecycleConvergenceFailed {
            iterations: self.config.max_iterations,
            max_iterations: self.config.max_iterations,
            residual: last_res_norm,
            tolerance: self.config.tolerance_abs,
        })
    }

    /// Solves $\mathbf{F}(\mathbf{x}) = \mathbf{0}$ using user-provided analytical Jacobian.
    pub fn solve_with_jacobian<F, J>(
        &self,
        initial_guess: &[f64],
        mut residual_fn: F,
        mut jacobian_fn: J,
    ) -> Result<NewtonReport, PhysicsError>
    where
        F: FnMut(&[f64]) -> Result<Vec<f64>, PhysicsError>,
        J: FnMut(&[f64], &[f64]) -> Result<Vec<Vec<f64>>, PhysicsError>,
    {
        let n = initial_guess.len();
        if n == 0 {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: "cannot solve empty system".to_string(),
            });
        }

        let mut x = initial_guess.to_vec();
        let mut f_val = residual_fn(&x)?;
        if f_val.len() != n {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!(
                    "residual dimension {} != state dimension {}",
                    f_val.len(),
                    n
                ),
            });
        }

        let initial_norm = Self::norm_inf(&f_val);
        if initial_norm <= self.config.tolerance_abs {
            return Ok(NewtonReport {
                solution: x,
                iterations: 0,
                final_residual: initial_norm,
                final_step: 0.0,
                converged: true,
            });
        }

        let mut last_step_norm = 0.0;
        let mut last_res_norm = initial_norm;
        let _ = last_step_norm;

        for iter in 0..self.config.max_iterations {
            // Evaluate Jacobian J(x)
            let jac = jacobian_fn(&x, &f_val)?;
            if jac.len() != n || jac.iter().any(|row| row.len() != n) {
                return Err(PhysicsError::InvalidOperatingCondition {
                    message: "Jacobian dimension mismatch".to_string(),
                });
            }

            // Solve linear system: J * delta_x = -F(x)
            let neg_f: Vec<f64> = f_val.iter().map(|&v| -v).collect();
            let delta_x = Self::solve_linear_system(&jac, &neg_f)?;

            last_step_norm = Self::norm_inf(&delta_x);

            // Step with optional Armijo backtracking line search
            let mut alpha = 1.0;
            let mut x_trial = vec![0.0; n];
            let mut f_trial = Vec::new();
            let mut trial_norm = f64::INFINITY;

            let min_alpha = if self.config.line_search {
                0.03125
            } else {
                1.0
            };

            while alpha >= min_alpha {
                for i in 0..n {
                    x_trial[i] = x[i] + alpha * delta_x[i];
                }

                if let Ok(res) = residual_fn(&x_trial) {
                    let norm = Self::norm_inf(&res);
                    if norm < last_res_norm || !self.config.line_search {
                        f_trial = res;
                        trial_norm = norm;
                        break;
                    }
                }
                alpha *= 0.5;
            }

            if f_trial.is_empty() {
                for i in 0..n {
                    x_trial[i] = x[i] + delta_x[i];
                }
                f_trial = residual_fn(&x_trial)?;
                trial_norm = Self::norm_inf(&f_trial);
            }

            x = x_trial;
            f_val = f_trial;
            last_res_norm = trial_norm;

            let max_allowed = self.config.tolerance_abs + self.config.tolerance_rel * initial_norm;
            if last_res_norm <= max_allowed
                || (last_step_norm <= self.config.step_tolerance
                    && last_res_norm <= 10.0 * max_allowed)
            {
                return Ok(NewtonReport {
                    solution: x,
                    iterations: iter + 1,
                    final_residual: last_res_norm,
                    final_step: last_step_norm,
                    converged: true,
                });
            }
        }

        Err(PhysicsError::RecycleConvergenceFailed {
            iterations: self.config.max_iterations,
            max_iterations: self.config.max_iterations,
            residual: last_res_norm,
            tolerance: self.config.tolerance_abs,
        })
    }

    /// Evaluates central finite-difference Jacobian matrix $\mathbf{J}_{ij} = \frac{\partial F_i}{\partial x_j}$.
    pub fn numerical_jacobian<F>(
        x: &[f64],
        _f_x: &[f64],
        mut residual_fn: F,
        eps: f64,
    ) -> Result<Vec<Vec<f64>>, PhysicsError>
    where
        F: FnMut(&[f64]) -> Result<Vec<f64>, PhysicsError>,
    {
        let n = x.len();
        let mut jac = vec![vec![0.0; n]; n];
        let mut x_plus = x.to_vec();
        let mut x_minus = x.to_vec();

        for j in 0..n {
            let h = eps * (1.0 + x[j].abs());
            x_plus[j] = x[j] + h;
            x_minus[j] = x[j] - h;

            let f_plus = residual_fn(&x_plus)?;
            let f_minus = residual_fn(&x_minus)?;

            let inv_2h = 0.5 / h;
            for i in 0..n {
                jac[i][j] = (f_plus[i] - f_minus[i]) * inv_2h;
            }

            x_plus[j] = x[j];
            x_minus[j] = x[j];
        }

        Ok(jac)
    }

    /// Solves linear system $\mathbf{A} \mathbf{x} = \mathbf{b}$ via Gaussian elimination with partial pivoting.
    #[allow(clippy::needless_range_loop)]
    pub fn solve_linear_system(a: &[Vec<f64>], b: &[f64]) -> Result<Vec<f64>, PhysicsError> {
        let n = a.len();
        let mut aug: Vec<Vec<f64>> = Vec::with_capacity(n);
        for i in 0..n {
            let mut row = a[i].clone();
            row.push(b[i]);
            aug.push(row);
        }

        // Forward elimination
        for col in 0..n {
            // Find pivot
            let mut max_row = col;
            let mut max_val = aug[col][col].abs();
            for r in (col + 1)..n {
                let v = aug[r][col].abs();
                if v > max_val {
                    max_val = v;
                    max_row = r;
                }
            }

            if max_val < 1e-15 || !max_val.is_finite() {
                return Err(PhysicsError::SingularJacobian {
                    detail: format!("pivot column {col} is singular (max pivot: {max_val})"),
                });
            }

            // Swap rows
            if max_row != col {
                aug.swap(col, max_row);
            }

            // Eliminate lower column
            let pivot = aug[col][col];
            for r in (col + 1)..n {
                let factor = aug[r][col] / pivot;
                for c in col..=n {
                    aug[r][c] -= factor * aug[col][c];
                }
            }
        }

        // Back substitution
        let mut x = vec![0.0; n];
        for r in (0..n).rev() {
            let mut sum = aug[r][n];
            for c in (r + 1)..n {
                sum -= aug[r][c] * x[c];
            }
            let diag = aug[r][r];
            if diag.abs() < 1e-15 {
                return Err(PhysicsError::SingularJacobian {
                    detail: format!("zero diagonal at row {r} during back substitution"),
                });
            }
            x[r] = sum / diag;
        }

        Ok(x)
    }

    /// Evaluates $\ell_\infty$ norm of a vector: $\max_i |x_i|$.
    #[inline]
    pub fn norm_inf(v: &[f64]) -> f64 {
        v.iter().fold(0.0_f64, |acc, &val| acc.max(val.abs()))
    }
}
