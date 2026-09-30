//! NebuDCS Custom Equipment & Runtime Authoring Engine.
//!
//! Provides:
//! - Level 1: Declarative parametric configuration, 1D/2D performance curves, and operating envelopes.
//! - Level 2: Composite equipment packages and sub-component wiring.
//! - Level 3: Sandboxed Rhai dynamic behavior scripting with strict 50,000 instruction limits.
//! - Comprehensive pre-execution validation and error handling.
//! - Direct integration with deterministic simulation double-buffered state and tag store.

pub mod curve;
pub mod error;
pub mod library;
pub mod model;
pub mod runtime;
pub mod sandbox;
pub mod validation;

pub use curve::{Curve1D, Curve2D, ExtrapolationMode, InterpolationMethod};
pub use error::{ScriptingError, ScriptingResult};
pub use indexmap::IndexMap;
pub use library::{
    export_package, export_template, import_package, import_template,
    standard_domain_equipment_definitions, DependencyResolver, EquipmentLibrary, EquipmentPackage,
    EquipmentTemplate, PackageDependency, PackageInstantiationResult, PackageInternalStream,
    PackageParameterMapping, PackagePortMapping, PackageSubComponent, SerializationFormat,
    TemplateMetadata, TemplateSource, VersionConstraint, VersionLifecycle, VersionedKey,
};
pub use model::{
    CompositionDefinition, CustomEquipmentDefinition, CustomizationTier, InterlockRuleDefinition,
    InternalConnectionDefinition, PortMappingDefinition, ScriptDefinition, SubComponentDefinition,
    TripConditionDefinition, TripOperator,
};
pub use runtime::{
    CompiledDefinition, CustomEquipmentInstance, CustomEquipmentManager, CustomEquipmentSystem,
};
pub use sandbox::{
    RhaiSandbox, ScriptExecutionContext, ScriptExecutionOutput, MAX_SCRIPT_OPERATIONS,
};
pub use validation::{validate_custom_equipment, validate_definition};
