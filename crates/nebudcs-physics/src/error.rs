//! Strongly typed errors for physics, property packages, conservation, and solvers.

use thiserror::Error;

/// Errors arising in physics calculations, property evaluation, and numerical solvers.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum PhysicsError {
    #[error("non-positive absolute pressure: {value} Pa (context: {context})")]
    NegativePressure { value: f64, context: String },

    #[error("non-positive thermodynamic temperature: {value} K (context: {context})")]
    NonPositiveTemperature { value: f64, context: String },

    #[error("invalid density: {value} kg/m³ (context: {context})")]
    InvalidDensity { value: f64, context: String },

    #[error("invalid vapor fraction: {value} (must be in range [0.0, 1.0])")]
    InvalidVaporFraction { value: f64 },

    #[error("inconsistent phase equilibrium: phase {phase:?} incompatible with vapor fraction {vapor_fraction}")]
    InvalidPhaseEquilibrium {
        phase: crate::state::Phase,
        vapor_fraction: f64,
    },

    #[error("invalid composition: {reason}")]
    InvalidComposition { reason: String },

    #[error("non-finite numerical value in {field}: {value}")]
    NonFiniteValue { field: String, value: f64 },

    #[error("conservation violation in {balance_type}: residual {residual} exceeds tolerance {tolerance}")]
    ConservationViolation {
        balance_type: String,
        residual: f64,
        tolerance: f64,
    },

    #[error("recycle convergence failed after {iterations}/{max_iterations} iterations (residual: {residual}, tolerance: {tolerance})")]
    RecycleConvergenceFailed {
        iterations: usize,
        max_iterations: usize,
        residual: f64,
        tolerance: f64,
    },

    #[error("hydraulic solver error: {reason}")]
    HydraulicSolverError { reason: String },

    #[error("property package error: {message}")]
    PropertyPackageError { message: String },

    #[error("operating condition out of range: {message}")]
    InvalidOperatingCondition { message: String },

    #[error("missing component or property: {name}")]
    MissingProperty { name: String },

    #[error("cubic EOS root selection error: {message}")]
    EosRootError { message: String },

    #[error("VLE flash convergence failed after {iterations}/{max_iterations} iterations (residual: {residual}, tolerance: {tolerance})")]
    FlashConvergenceFailed {
        iterations: usize,
        max_iterations: usize,
        residual: f64,
        tolerance: f64,
    },

    #[error("phase stability analysis error: {message}")]
    PhaseStabilityError { message: String },

    #[error("singular Jacobian in nonlinear solver: {detail}")]
    SingularJacobian { detail: String },

    #[error("stiff solver {method} failed: {reason}")]
    StiffSolverFailed { method: String, reason: String },

    #[error("compressor map error: {message}")]
    CompressorMapError { message: String },

    #[error("pump cavitation calculation error: {message}")]
    CavitationError { message: String },

    #[error("scientific data validation error: {detail}")]
    ScientificDataError { detail: String },

    #[error(
        "property calculation out of range: {variable}={value} outside [{min}, {max}] ({context})"
    )]
    PropertyOutOfRange {
        variable: String,
        value: f64,
        min: f64,
        max: f64,
        context: String,
    },

    #[error("unsupported capability {capability} in package {package}")]
    UnsupportedCapability { capability: String, package: String },

    #[error("thermodynamic inconsistency: {detail}")]
    ThermodynamicInconsistency { detail: String },

    #[error("reaction kinetics error: {message}")]
    ReactionKineticsError { message: String },

    #[error("heat transfer calculation error: {message}")]
    HeatTransferError { message: String },
}
