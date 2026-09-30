//! Generic process physics, property packages, conservation laws, and hydraulic solvers for NebuDCS.

pub mod conservation;
pub mod equipment;
pub mod error;
pub mod heat_transfer;
pub mod hydraulics;
pub mod network;
pub mod numerics;
pub mod properties;
pub mod reaction;
pub mod rotating;
pub mod solver;
pub mod state;
pub mod system;
pub mod thermo;
pub mod validation;

pub use conservation::{
    EnergyBalance, EnthalpyAccumulationInputs, MassBalance, MassBalanceResult, SpeciesBalance,
};
pub use equipment::{
    AccumulatorVolumeModel, CheckValveModel, DistillationColumnModel, EquipmentCapability,
    EquipmentExecutionContext, FlowResistanceModel, GenericEquipmentPhysics, HeadSourceModel,
    HeatExchangeModel, ProcessThermalModel, TurbineExpansionModel,
};
pub use error::PhysicsError;
pub use hydraulics::{churchill_friction_factor, pipe_pressure_drop, ControlValveHydraulics};
pub use network::{
    EdgeComponent, EdgeDiagnostic, HydraulicEdge, HydraulicNetwork, HydraulicNetworkSolver,
    HydraulicNode, NetworkDiagnostics, NodeDiagnostic, NodeKind, RecycleConvergenceManager,
    RecycleDiagnostics,
};
pub use numerics::{
    Bdf2Solver, DaeSystem, ImplicitEulerSolver, NewtonConfig, NewtonReport, NewtonSolver,
    OdeSystem, RadauIIASolver,
};
pub use properties::{
    IdealGasMixture, IncompressibleLiquid, PhaseEquilibrium, PropertyPackage, SaturatedFluidPackage,
};
pub use heat_transfer::{
    EpsilonNtuSolver, FlowArrangement, FoulingParameters, HeatExchangerRatingResult,
    HeatExchangerUnit, LmtdCalculator, LmtdResult,
};
pub use reaction::{
    CstrConfig, CstrReactorModel, CstrState, PfrConfig, PfrReactorModel, PfrResult, PfrSlicePoint,
    ReactionDefinition, ReactionRateResult, ReactionSystem,
};
pub use rotating::{
    CavitationRegime, CompressorMapModel, CompressorMapPoint, CompressorOperatingState,
    CompressorSpeedLine, PumpCavitationModel,
};
pub use solver::{RecycleSolver, SolverConfig, SolverReport};
pub use state::{
    ComponentComposition, CompositionBasis, Phase, PhysicalState, StreamPhysicalState,
};
pub use system::{CouplingKind, GenericPhysicsSystem, ProcessBoundaryCondition, ProcessCoupling};
pub use thermo::{
    BinaryInteractionMatrix, ComponentDatabase, CoolPropAdapter, CubicEos, CubicEosState,
    CubicEosType, CubicPropertyPackage, ExternalPropertyPackage, ExternalPropertyProvider,
    FlashConfig, FlashResult, PhaseStabilityState, PhaseTarget, PureComponentParameters,
    StabilityAnalysisResult, ThermodynamicComponent, VleFlashSolver, R_GAS_CONSTANT,
    STANDARD_REFERENCE_PRESSURE_PA, STANDARD_REFERENCE_TEMPERATURE_K,
};
