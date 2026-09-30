//! Heat transfer primitives, LMTD evaluation, $\varepsilon$-NTU solvers, and exchanger rating.

pub mod exchanger;

pub use exchanger::{
    EpsilonNtuSolver, FlowArrangement, FoulingParameters, HeatExchangerRatingResult,
    HeatExchangerUnit, LmtdCalculator, LmtdResult,
};
