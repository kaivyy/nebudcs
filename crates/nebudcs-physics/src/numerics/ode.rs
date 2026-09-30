//! Reusable deterministic stiff ODE numerical integrators and future DAE abstraction.
//!
//! Provides:
//! - [`ImplicitEulerSolver`]: First-order A-stable / L-stable implicit ODE integrator
//! - [`Bdf2Solver`]: Second-order Backward Differentiation Formula for stiff ODE transients
//! - [`RadauIIASolver`]: Three-stage 5th-order Radau IIA implicit Runge-Kutta stiff ODE solver
//! - [`DaeSystem`]: Architectural interface contract for future DAE index-1 integrators
//!
//! All solvers use the robust [`super::nonlinear::NewtonSolver`] for inner nonlinear iterations.

use crate::error::PhysicsError;
use crate::numerics::nonlinear::{NewtonConfig, NewtonSolver};

/// Trait representing an Ordinary Differential Equation system $\frac{d\mathbf{x}}{dt} = \mathbf{f}(t, \mathbf{x})$.
pub trait OdeSystem {
    /// Dimension of the state vector $\mathbf{x}$.
    fn dimension(&self) -> usize;

    /// Evaluates the right-hand side time derivatives $\mathbf{f}(t, \mathbf{x}) = \dot{\mathbf{x}}$.
    fn evaluate_rhs(&self, t: f64, x: &[f64]) -> Result<Vec<f64>, PhysicsError>;
}

/// Trait representing a Differential-Algebraic Equation (DAE) system $\mathbf{F}(t, \mathbf{x}, \dot{\mathbf{x}}, \mathbf{z}) = \mathbf{0}$.
///
/// **Architectural Note**: This trait defines the problem specification contract for future DAE
/// integrators. The current stiff solvers ([`ImplicitEulerSolver`], [`Bdf2Solver`],
/// [`RadauIIASolver`]) specifically step [`OdeSystem`] implementations.
pub trait DaeSystem {
    /// Number of differential states $\mathbf{x}$.
    fn differential_dimension(&self) -> usize;

    /// Number of algebraic variables $\mathbf{z}$.
    fn algebraic_dimension(&self) -> usize;

    /// Evaluates DAE residuals $\mathbf{F}(t, \mathbf{x}, \dot{\mathbf{x}}, \mathbf{z}) = \mathbf{0}$.
    fn evaluate_residual(
        &self,
        t: f64,
        x: &[f64],
        x_dot: &[f64],
        z: &[f64],
    ) -> Result<Vec<f64>, PhysicsError>;
}

/// First-order Implicit Euler stiff integrator.
///
/// Discretization: $\mathbf{x}_{n+1} = \mathbf{x}_n + \Delta t \cdot \mathbf{f}(t_{n+1}, \mathbf{x}_{n+1})$
/// Residual: $\mathbf{R}(\mathbf{x}_{n+1}) = \mathbf{x}_{n+1} - \mathbf{x}_n - \Delta t \cdot \mathbf{f}(t_{n+1}, \mathbf{x}_{n+1}) = \mathbf{0}$
#[derive(Debug, Clone)]
pub struct ImplicitEulerSolver {
    newton: NewtonSolver,
}

impl Default for ImplicitEulerSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ImplicitEulerSolver {
    /// Creates an Implicit Euler solver with standard Newton solver settings.
    pub fn new() -> Self {
        Self {
            newton: NewtonSolver::new(),
        }
    }

    /// Creates an Implicit Euler solver with custom Newton solver settings.
    pub fn with_newton_config(config: NewtonConfig) -> Self {
        Self {
            newton: NewtonSolver::with_config(config),
        }
    }

    /// Advances state $\mathbf{x}_n$ by step $\Delta t$ to $t_{n+1} = t_n + \Delta t$.
    pub fn step<S: OdeSystem>(
        &self,
        system: &S,
        t_n: f64,
        x_n: &[f64],
        dt: f64,
    ) -> Result<Vec<f64>, PhysicsError> {
        let n = system.dimension();
        if x_n.len() != n {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("state dimension {} != system dimension {}", x_n.len(), n),
            });
        }
        if dt <= 0.0 || !dt.is_finite() {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("timestep dt must be positive: {dt}"),
            });
        }

        let t_next = t_n + dt;

        // Residual function R(x_next) = x_next - x_n - dt * f(t_next, x_next)
        let residual_fn = |x_candidate: &[f64]| -> Result<Vec<f64>, PhysicsError> {
            let f = system.evaluate_rhs(t_next, x_candidate)?;
            let mut res = vec![0.0; n];
            for i in 0..n {
                res[i] = x_candidate[i] - x_n[i] - dt * f[i];
            }
            Ok(res)
        };

        // Solve nonlinear algebraic equation using Newton-Raphson
        let report =
            self.newton
                .solve(x_n, residual_fn)
                .map_err(|e| PhysicsError::StiffSolverFailed {
                    method: "ImplicitEuler".to_string(),
                    reason: e.to_string(),
                })?;

        Ok(report.solution)
    }
}

/// Second-order Backward Differentiation Formula (BDF-2) stiff integrator.
///
/// Discretization: $\mathbf{x}_{n+1} - \frac{4}{3} \mathbf{x}_n + \frac{1}{3} \mathbf{x}_{n-1} = \frac{2}{3} \Delta t \cdot \mathbf{f}(t_{n+1}, \mathbf{x}_{n+1})$
///
/// Self-starting: uses Implicit Euler on the first step when $\mathbf{x}_{n-1}$ is not yet available.
#[derive(Debug, Clone)]
pub struct Bdf2Solver {
    newton: NewtonSolver,
}

impl Default for Bdf2Solver {
    fn default() -> Self {
        Self::new()
    }
}

impl Bdf2Solver {
    /// Creates a BDF-2 solver with standard Newton solver settings.
    pub fn new() -> Self {
        Self {
            newton: NewtonSolver::new(),
        }
    }

    /// Advances state using BDF-2 if prior step $\mathbf{x}_{n-1}$ is available, otherwise falls back to Implicit Euler.
    pub fn step<S: OdeSystem>(
        &self,
        system: &S,
        t_n: f64,
        x_n: &[f64],
        x_prev: Option<&[f64]>,
        dt: f64,
    ) -> Result<Vec<f64>, PhysicsError> {
        let n = system.dimension();
        if x_n.len() != n {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("state dimension {} != system dimension {}", x_n.len(), n),
            });
        }
        if dt <= 0.0 || !dt.is_finite() {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("timestep dt must be positive: {dt}"),
            });
        }

        let t_next = t_n + dt;

        let Some(x_prev) = x_prev else {
            // First step fallback: Implicit Euler
            let euler = ImplicitEulerSolver::new();
            return euler.step(system, t_n, x_n, dt);
        };

        if x_prev.len() != n {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!(
                    "x_prev dimension {} != system dimension {}",
                    x_prev.len(),
                    n
                ),
            });
        }

        // BDF-2 coefficients:
        // x_{n+1} - (4/3) x_n + (1/3) x_{n-1} = (2/3) dt * f(t_{n+1}, x_{n+1})
        // R(x) = x - (4/3) x_n + (1/3) x_{n-1} - (2/3) dt * f(t_{next}, x)
        let residual_fn = |x_candidate: &[f64]| -> Result<Vec<f64>, PhysicsError> {
            let f = system.evaluate_rhs(t_next, x_candidate)?;
            let mut res = vec![0.0; n];
            let dt_factor = (2.0 / 3.0) * dt;
            for i in 0..n {
                let lhs = x_candidate[i] - (4.0 / 3.0) * x_n[i] + (1.0 / 3.0) * x_prev[i];
                res[i] = lhs - dt_factor * f[i];
            }
            Ok(res)
        };

        // Linear extrapolation guess: x_guess = 2 * x_n - x_{n-1}
        let mut x_guess = vec![0.0; n];
        for i in 0..n {
            x_guess[i] = 2.0 * x_n[i] - x_prev[i];
        }

        let report = self.newton.solve(&x_guess, residual_fn).map_err(|e| {
            PhysicsError::StiffSolverFailed {
                method: "BDF-2".to_string(),
                reason: e.to_string(),
            }
        })?;

        Ok(report.solution)
    }
}

/// 3-stage, 5th-order Radau IIA implicit Runge-Kutta stiff ODE integrator.
///
/// Features:
/// - A-stable and L-stable (unconditionally stable for stiff chemical reaction/pressure dynamics)
/// - Stage order 3, global order 5
/// - End-point stage ($c_3 = 1.0$), making solution $\mathbf{x}_{n+1} = \mathbf{Y}_3$ directly
#[derive(Debug, Clone)]
pub struct RadauIIASolver {
    newton: NewtonSolver,
}

impl Default for RadauIIASolver {
    fn default() -> Self {
        Self::new()
    }
}

impl RadauIIASolver {
    /// Creates a Radau IIA solver.
    pub fn new() -> Self {
        Self {
            newton: NewtonSolver::new(),
        }
    }

    /// Advances state $\mathbf{x}_n$ by step $\Delta t$ using 3-stage Radau IIA.
    pub fn step<S: OdeSystem>(
        &self,
        system: &S,
        t_n: f64,
        x_n: &[f64],
        dt: f64,
    ) -> Result<Vec<f64>, PhysicsError> {
        let n = system.dimension();
        if x_n.len() != n {
            return Err(PhysicsError::InvalidOperatingCondition {
                message: format!("state dimension {} != system dimension {}", x_n.len(), n),
            });
        }

        let sqrt6 = 6.0_f64.sqrt();

        // Radau IIA Butcher Tableau (3-stage, order 5)
        let c = [(4.0 - sqrt6) / 10.0, (4.0 + sqrt6) / 10.0, 1.0];

        let a = [
            [
                (88.0 - 7.0 * sqrt6) / 360.0,
                (296.0 - 169.0 * sqrt6) / 1800.0,
                (-2.0 + 3.0 * sqrt6) / 225.0,
            ],
            [
                (296.0 + 169.0 * sqrt6) / 1800.0,
                (88.0 + 7.0 * sqrt6) / 360.0,
                (-2.0 - 3.0 * sqrt6) / 225.0,
            ],
            [(16.0 - sqrt6) / 36.0, (16.0 + sqrt6) / 36.0, 1.0 / 9.0],
        ];

        // Total stage vector length: 3 * n
        // Layout: [Y_1 (n values), Y_2 (n values), Y_3 (n values)]
        let mut y_guess = vec![0.0; 3 * n];
        for s in 0..3 {
            for i in 0..n {
                y_guess[s * n + i] = x_n[i];
            }
        }

        let residual_fn = |y_vec: &[f64]| -> Result<Vec<f64>, PhysicsError> {
            let mut f_stages = Vec::with_capacity(3);
            for s in 0..3 {
                let y_s = &y_vec[(s * n)..((s + 1) * n)];
                let t_s = t_n + c[s] * dt;
                let f_s = system.evaluate_rhs(t_s, y_s)?;
                f_stages.push(f_s);
            }

            let mut res = vec![0.0; 3 * n];
            for i_stage in 0..3 {
                for i in 0..n {
                    let mut sum_a_f = 0.0;
                    for j_stage in 0..3 {
                        sum_a_f += a[i_stage][j_stage] * f_stages[j_stage][i];
                    }
                    let y_curr = y_vec[i_stage * n + i];
                    res[i_stage * n + i] = y_curr - x_n[i] - dt * sum_a_f;
                }
            }

            Ok(res)
        };

        let report = self.newton.solve(&y_guess, residual_fn).map_err(|e| {
            PhysicsError::StiffSolverFailed {
                method: "RadauIIA".to_string(),
                reason: e.to_string(),
            }
        })?;

        // In Radau IIA, c_3 = 1.0 and b_j = a_{3,j}, so Y_3 is exactly x_{n+1}!
        let solution = report.solution[(2 * n)..(3 * n)].to_vec();
        Ok(solution)
    }
}
