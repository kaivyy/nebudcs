//! Generic process physics, property packages, conservation laws, and hydraulic solvers for NebuDCS.

pub mod conservation;
pub mod equipment;
pub mod error;
pub mod hydraulics;
pub mod properties;
pub mod solver;
pub mod state;
pub mod system;

pub use conservation::{
    EnergyBalance, EnthalpyAccumulationInputs, MassBalance, MassBalanceResult, SpeciesBalance,
};
pub use equipment::{
    AccumulatorVolumeModel, EquipmentCapability, EquipmentExecutionContext, FlowResistanceModel,
    GenericEquipmentPhysics, HeadSourceModel, HeatExchangeModel,
};
pub use error::PhysicsError;
pub use hydraulics::{churchill_friction_factor, pipe_pressure_drop, ControlValveHydraulics};
pub use properties::{
    IdealGasMixture, IncompressibleLiquid, PhaseEquilibrium, PropertyPackage, SaturatedFluidPackage,
};
pub use solver::{RecycleSolver, SolverConfig, SolverReport};
pub use state::{
    ComponentComposition, CompositionBasis, Phase, PhysicalState, StreamPhysicalState,
};
pub use system::GenericPhysicsSystem;
