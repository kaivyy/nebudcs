//! Standard and User Equipment Library and Template Package Subsystem.
//!
//! Provides:
//! - Standard verified equipment templates covering generic industrial assets.
//! - User-defined custom equipment templates promoted from runtime definitions.
//! - Equipment packages representing composite machinery and subgraphs.
//! - Deterministic dependency resolution, cycle detection, and topological sorting.
//! - Immutable semantic versioning and provenance tracking.
//! - Hardened import and export with schema validation, non-finite float rejection, and size caps.
//! - Safe, isolated instantiation into the existing simulation engine.

pub mod catalog;
pub mod dependency;
pub mod import_export;
pub mod package;
pub mod standard;
pub mod template;
pub mod versioning;

pub use catalog::EquipmentLibrary;
pub use dependency::{DependencyResolver, VersionConstraint};
pub use import_export::{
    export_package, export_template, import_package, import_template, SerializationFormat,
    MAX_IMPORT_SIZE_BYTES, MAX_PACKAGE_INTERNAL_STREAMS, MAX_PACKAGE_SUB_COMPONENTS,
};
pub use package::{
    EquipmentPackage, PackageDependency, PackageInstantiationResult, PackageInternalStream,
    PackageParameterMapping, PackagePortMapping, PackageSubComponent,
};
pub use standard::{
    load_all_standard_packages, load_all_standard_templates, make_standard_centrifugal_compressor,
    make_standard_centrifugal_pump, make_standard_control_valve, make_standard_cstr_reactor,
    make_standard_fan, make_standard_fired_heater, make_standard_heat_exchanger,
    make_standard_mixer, make_standard_pump_bypass_package, make_standard_sensor,
    make_standard_separator, make_standard_splitter, make_standard_storage_tank,
    make_standard_two_stage_compressor_package,
};
pub use template::{EquipmentTemplate, TemplateMetadata, TemplateSource};
pub use versioning::{
    compute_content_fingerprint, verify_immutability, ModelProvenance, VersionLifecycle,
    VersionedKey,
};
