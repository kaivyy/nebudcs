//! Advanced thermodynamic calculations, cubic equations of state, and multicomponent flash.

pub mod benchmark;
pub mod component;
pub mod correlation;
pub mod cubic;
pub mod envelope;
pub mod external;
pub mod flash;
pub mod package;
pub mod provenance;

pub use benchmark::{
    BenchmarkClass, BenchmarkVerificationResult, ReferenceBenchmarkCase, ReferenceBenchmarkDatabase,
};
pub use component::{
    BinaryInteractionMatrix, ComponentDatabase, ReferenceStateConvention, ThermodynamicComponent,
    TransportPropertiesSchema, R_GAS_CONSTANT, STANDARD_REFERENCE_PRESSURE_PA,
    STANDARD_REFERENCE_TEMPERATURE_K,
};
pub use correlation::{
    AntoineVaporPressureCorrelation, IdealGasCpCorrelation, RackettLiquidDensityCorrelation,
    ScientificCorrelation, WatsonHeatOfVaporizationCorrelation,
};
pub use cubic::{CubicEos, CubicEosState, CubicEosType, PhaseTarget, PureComponentParameters};
pub use envelope::{PhaseEnvelope, PhaseEnvelopePoint, PhaseEnvelopeSolver};
pub use external::{CoolPropAdapter, ExternalPropertyPackage, ExternalPropertyProvider};
pub use flash::{
    FlashConfig, FlashResult, PhaseStabilityState, StabilityAnalysisResult, VleFlashSolver,
};
pub use package::CubicPropertyPackage;
pub use provenance::{
    PropertyPackageCapabilities, ScientificParameter, ScientificProvenance, ToleranceClass,
    ValidityDomain, ValidityState,
};
