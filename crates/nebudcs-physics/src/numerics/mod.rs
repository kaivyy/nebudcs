//! Advanced numerical solvers: multidimensional Newton-Raphson and stiff ODE/DAE integrators.

pub mod nonlinear;
pub mod ode;

pub use nonlinear::{NewtonConfig, NewtonReport, NewtonSolver};
pub use ode::{Bdf2Solver, DaeSystem, ImplicitEulerSolver, OdeSystem, RadauIIASolver};
