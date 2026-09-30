//! Chemical reaction kinetics, multi-reaction systems, and reactor equipment models (CSTR, PFR).

pub mod cstr;
pub mod kinetics;
pub mod pfr;

pub use cstr::{CstrConfig, CstrReactorModel, CstrState};
pub use kinetics::{ReactionDefinition, ReactionRateResult, ReactionSystem};
pub use pfr::{PfrConfig, PfrReactorModel, PfrResult, PfrSlicePoint};
