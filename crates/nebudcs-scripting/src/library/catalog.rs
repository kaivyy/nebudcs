//! Central Equipment Library Catalog.
//!
//! Maintains registered standard and user equipment templates and packages,
//! manages immutable semantic versions, provides fast resolution, and guarantees
//! isolated, safe instantiation.

use std::collections::{BTreeMap, HashMap};

use nebudcs_model::equipment::EquipmentCategory;
use nebudcs_types::{EquipmentId, SchemaVersion};

use crate::error::{ScriptingError, ScriptingResult};
use crate::library::dependency::DependencyResolver;
use crate::library::import_export::{
    export_package, export_template, import_package, import_template, SerializationFormat,
};
use crate::library::package::{EquipmentPackage, PackageInstantiationResult};
use crate::library::standard::{load_all_standard_packages, load_all_standard_templates};
use crate::library::template::{EquipmentTemplate, TemplateMetadata, TemplateSource};
use crate::library::versioning::{compute_content_fingerprint, verify_immutability};
use crate::model::CustomEquipmentDefinition;
use crate::runtime::CustomEquipmentInstance;

/// The central repository of standard and user equipment templates and packages.
#[derive(Default, Clone)]
pub struct EquipmentLibrary {
    /// Templates keyed by (template_id, version).
    templates: HashMap<(String, SchemaVersion), EquipmentTemplate>,
    /// Latest stable version for each template ID.
    latest_template: HashMap<String, SchemaVersion>,
    /// Packages keyed by (package_id, version).
    packages: HashMap<(String, SchemaVersion), EquipmentPackage>,
    /// Latest stable version for each package ID.
    latest_package: HashMap<String, SchemaVersion>,
    /// Content fingerprints to enforce immutability of published versions.
    fingerprints: HashMap<(String, SchemaVersion), String>,
}

impl EquipmentLibrary {
    /// Creates a new empty equipment library.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an equipment library pre-loaded with the verified standard library.
    pub fn with_standard_library() -> ScriptingResult<Self> {
        let mut library = Self::new();
        library.load_standard_catalog()?;
        Ok(library)
    }

    /// Loads all standard industrial equipment templates and packages into the library.
    pub fn load_standard_catalog(&mut self) -> ScriptingResult<usize> {
        let templates = load_all_standard_templates()?;
        let count = templates.len();
        for tmpl in templates {
            self.register_template(tmpl)?;
        }

        let packages = load_all_standard_packages();
        for pkg in packages {
            self.register_package(pkg)?;
        }

        Ok(count)
    }

    /// Registers an equipment template in the library.
    ///
    /// Enforces immutability: if the exact `(id, version)` already exists, the definition
    /// content must be identical or registration fails with a version conflict error.
    pub fn register_template(&mut self, template: EquipmentTemplate) -> ScriptingResult<()> {
        let key = (template.id.clone(), template.version);
        let serialized = serde_json::to_string(&template.definition)
            .map_err(|e| ScriptingError::Serialization(e.to_string()))?;
        let fingerprint = compute_content_fingerprint(&serialized);

        if let Some(existing_fingerprint) = self.fingerprints.get(&key) {
            verify_immutability(
                &template.id,
                &template.version,
                existing_fingerprint,
                &fingerprint,
            )?;
            return Ok(());
        }

        let id = template.id.clone();
        let version = template.version;

        self.fingerprints.insert(key.clone(), fingerprint);
        self.templates.insert(key, template);

        // Update latest version pointer if newer or not present
        match self.latest_template.get(&id) {
            Some(curr) if *curr >= version => {}
            _ => {
                self.latest_template.insert(id, version);
            }
        }

        Ok(())
    }

    /// Promotes a validated Phase 10 `CustomEquipmentDefinition` into a reusable user template.
    pub fn register_user_template(
        &mut self,
        definition: CustomEquipmentDefinition,
        metadata: Option<TemplateMetadata>,
    ) -> ScriptingResult<EquipmentTemplate> {
        let template = EquipmentTemplate::from_user_definition(definition, metadata)?;
        self.register_template(template.clone())?;
        Ok(template)
    }

    /// Retrieves an equipment template by ID and optional version (defaults to latest).
    pub fn get_template(
        &self,
        id: &str,
        version: Option<&SchemaVersion>,
    ) -> Option<&EquipmentTemplate> {
        match version {
            Some(v) => self.templates.get(&(id.to_string(), *v)),
            None => {
                let latest = self.latest_template.get(id)?;
                self.templates.get(&(id.to_string(), *latest))
            }
        }
    }

    /// Lists all registered templates, with optional category and source filters.
    pub fn list_templates(
        &self,
        filter_category: Option<EquipmentCategory>,
        filter_source: Option<TemplateSource>,
    ) -> Vec<&EquipmentTemplate> {
        let mut list: Vec<&EquipmentTemplate> = self
            .templates
            .values()
            .filter(|t| {
                if let Some(cat) = filter_category {
                    if t.category != cat {
                        return false;
                    }
                }
                if let Some(src) = filter_source {
                    if t.source != src {
                        return false;
                    }
                }
                true
            })
            .collect();
        list.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.version.cmp(&b.version)));
        list
    }

    /// Registers an equipment package in the library.
    ///
    /// Validates internal connections and references against registered templates.
    pub fn register_package(&mut self, package: EquipmentPackage) -> ScriptingResult<()> {
        let key = (package.id.clone(), package.version);
        let serialized = serde_json::to_string(&package)
            .map_err(|e| ScriptingError::Serialization(e.to_string()))?;
        let fingerprint = compute_content_fingerprint(&serialized);

        if let Some(existing_fingerprint) = self.fingerprints.get(&key) {
            verify_immutability(
                &package.id,
                &package.version,
                existing_fingerprint,
                &fingerprint,
            )?;
            return Ok(());
        }

        // Validate package connections and child templates
        package.validate(|id, ver| self.get_template(id, ver))?;

        let id = package.id.clone();
        let version = package.version;

        self.fingerprints.insert(key.clone(), fingerprint);
        self.packages.insert(key, package);

        match self.latest_package.get(&id) {
            Some(curr) if *curr >= version => {}
            _ => {
                self.latest_package.insert(id, version);
            }
        }

        Ok(())
    }

    /// Retrieves an equipment package by ID and optional version (defaults to latest).
    pub fn get_package(
        &self,
        id: &str,
        version: Option<&SchemaVersion>,
    ) -> Option<&EquipmentPackage> {
        match version {
            Some(v) => self.packages.get(&(id.to_string(), *v)),
            None => {
                let latest = self.latest_package.get(id)?;
                self.packages.get(&(id.to_string(), *latest))
            }
        }
    }

    /// Lists all registered packages.
    pub fn list_packages(&self) -> Vec<&EquipmentPackage> {
        let mut list: Vec<&EquipmentPackage> = self.packages.values().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.version.cmp(&b.version)));
        list
    }

    /// Validates an equipment package without mutating catalog state.
    pub fn validate_package(&self, package: &EquipmentPackage) -> ScriptingResult<()> {
        package.validate(|id, ver| self.get_template(id, ver))
    }

    /// Resolves dependencies for a package in deterministic topological order.
    pub fn resolve_package_dependencies(
        &self,
        package: &EquipmentPackage,
    ) -> ScriptingResult<Vec<String>> {
        let mut resolver = DependencyResolver::new();
        resolver.add_dependencies(package.id.clone(), package.dependencies.clone());

        // Gather all available template versions in catalog
        let mut available: HashMap<String, Vec<SchemaVersion>> = HashMap::new();
        for (id, ver) in self.templates.keys() {
            available.entry(id.clone()).or_default().push(*ver);
        }
        for (id, ver) in self.packages.keys() {
            available.entry(id.clone()).or_default().push(*ver);
        }

        resolver.resolve(&available)
    }

    /// Instantiates an equipment template into an independent `CustomEquipmentInstance`.
    pub fn instantiate_template(
        &self,
        template_id: &str,
        version: Option<&SchemaVersion>,
        instance_id: EquipmentId,
        name: impl Into<String>,
        param_overrides: &BTreeMap<String, f64>,
    ) -> ScriptingResult<CustomEquipmentInstance> {
        let tmpl = self.get_template(template_id, version).ok_or_else(|| {
            ScriptingError::NotFound(format!("Template '{template_id}' not found"))
        })?;
        tmpl.instantiate(instance_id, name, param_overrides)
    }

    /// Instantiates an equipment package into a collection of child instances with internal wiring.
    pub fn instantiate_package(
        &self,
        package_id: &str,
        version: Option<&SchemaVersion>,
        instance_prefix: &str,
        param_overrides: &BTreeMap<String, f64>,
    ) -> ScriptingResult<PackageInstantiationResult> {
        let pkg = self
            .get_package(package_id, version)
            .ok_or_else(|| ScriptingError::NotFound(format!("Package '{package_id}' not found")))?;
        pkg.instantiate_subgraph(instance_prefix, param_overrides, |id, ver| {
            self.get_template(id, ver)
        })
    }

    /// Imports, validates, and registers an equipment template from raw serialized data.
    pub fn import_and_register_template(
        &mut self,
        content: &str,
        format: SerializationFormat,
    ) -> ScriptingResult<EquipmentTemplate> {
        let template = import_template(content, format)?;
        self.register_template(template.clone())?;
        Ok(template)
    }

    /// Exports an equipment template to serialized data string.
    pub fn export_template(
        &self,
        id: &str,
        version: Option<&SchemaVersion>,
        format: SerializationFormat,
    ) -> ScriptingResult<String> {
        let tmpl = self
            .get_template(id, version)
            .ok_or_else(|| ScriptingError::NotFound(format!("Template '{id}' not found")))?;
        export_template(tmpl, format)
    }

    /// Imports, validates, and registers an equipment package from raw serialized data.
    pub fn import_and_register_package(
        &mut self,
        content: &str,
        format: SerializationFormat,
    ) -> ScriptingResult<EquipmentPackage> {
        let package = import_package(content, format)?;
        self.register_package(package.clone())?;
        Ok(package)
    }

    /// Exports an equipment package to serialized data string.
    pub fn export_package(
        &self,
        id: &str,
        version: Option<&SchemaVersion>,
        format: SerializationFormat,
    ) -> ScriptingResult<String> {
        let pkg = self
            .get_package(id, version)
            .ok_or_else(|| ScriptingError::NotFound(format!("Package '{id}' not found")))?;
        export_package(pkg, format)
    }
}
