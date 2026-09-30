//! REST API endpoints for the Phase 11 Standard & User Equipment Library and Template Packages.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use nebudcs_model::equipment::EquipmentCategory;
use nebudcs_protocol::{
    DomainPackageDetailDto, DomainPackageSummaryDto, ExportLibraryItemResponseDto,
    ImportLibraryItemRequestDto, InstantiatePackageRequestDto, InstantiateTemplateRequestDto,
    LibraryCommand, OperatingRangeDto, PackageDetailDto, PackageSummaryDto, RuntimeCommand,
    TemplateDetailDto, TemplatePortDto, TemplateSummaryDto, ValidatePackageResponseDto,
};
use nebudcs_scripting::{
    CustomEquipmentDefinition, EquipmentPackage, EquipmentTemplate, SerializationFormat,
    TemplateSource,
};
use nebudcs_types::{EquipmentId, SchemaVersion};

use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;

#[derive(Debug, Deserialize)]
pub struct TemplateFilterParams {
    pub category: Option<String>,
    pub source: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PackageFilterParams {
    pub version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExportParams {
    pub format: Option<String>,
    pub version: Option<String>,
}

/// Lists all registered equipment templates.
pub async fn list_templates(
    State(host): State<HostHandle>,
    Query(params): Query<TemplateFilterParams>,
) -> ServerResult<Json<Vec<TemplateSummaryDto>>> {
    let eq_mgr = host
        .equipment_manager()
        .read()
        .map_err(|_| ServerError::Internal("Equipment manager lock poisoned".to_string()))?;

    let cat_filter = match params.category.as_deref() {
        Some("rotating") => Some(EquipmentCategory::Rotating),
        Some("heat_transfer") => Some(EquipmentCategory::HeatTransfer),
        Some("process") => Some(EquipmentCategory::Process),
        Some("valve") => Some(EquipmentCategory::Valve),
        Some("instrumentation") => Some(EquipmentCategory::Instrumentation),
        Some("utility") => Some(EquipmentCategory::Utility),
        _ => None,
    };

    let src_filter = match params.source.as_deref() {
        Some("standard") => Some(TemplateSource::Standard),
        Some("user") => Some(TemplateSource::User),
        _ => None,
    };

    let templates = eq_mgr.library().list_templates(cat_filter, src_filter);
    let summaries = templates
        .into_iter()
        .map(|t| TemplateSummaryDto {
            id: t.id.clone(),
            name: t.name.clone(),
            version: t.version.to_string(),
            category: format!("{:?}", t.category),
            source: match t.source {
                TemplateSource::Standard => "standard".to_string(),
                TemplateSource::User => "user".to_string(),
            },
            description: t.metadata.description.clone(),
            parameter_count: t.definition.parameters.len(),
            port_count: t.definition.ports.len(),
            curve_count: t.definition.curves.len() + t.definition.curves_2d.len(),
            trip_count: t.definition.trips.len(),
            author: t.metadata.author.clone(),
            ports: t
                .definition
                .ports
                .iter()
                .map(|p| TemplatePortDto {
                    id: p.id.to_string(),
                    name: p.name.clone(),
                    domain: format!("{:?}", p.domain).to_lowercase(),
                    direction: format!("{:?}", p.direction).to_lowercase(),
                })
                .collect(),
        })
        .collect();

    Ok(Json(summaries))
}

/// Retrieves detailed information for a specific equipment template.
pub async fn get_template(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
    Query(params): Query<TemplateFilterParams>,
) -> ServerResult<Json<TemplateDetailDto>> {
    let eq_mgr = host
        .equipment_manager()
        .read()
        .map_err(|_| ServerError::Internal("Equipment manager lock poisoned".to_string()))?;

    let version_req = match params.version.as_deref() {
        Some(v) => Some(
            v.parse::<SchemaVersion>()
                .map_err(|e| ServerError::InvalidRequest(format!("Invalid version string: {e}")))?,
        ),
        None => None,
    };

    let tmpl = eq_mgr
        .library()
        .get_template(&id, version_req.as_ref())
        .ok_or_else(|| ServerError::NotFound(format!("Template '{id}' not found")))?;

    let def_json = serde_json::to_string_pretty(&tmpl.definition)
        .map_err(|e| ServerError::Internal(format!("Failed to serialize definition: {e}")))?;

    Ok(Json(TemplateDetailDto {
        id: tmpl.id.clone(),
        name: tmpl.name.clone(),
        version: tmpl.version.to_string(),
        category: format!("{:?}", tmpl.category),
        source: match tmpl.source {
            TemplateSource::Standard => "standard".to_string(),
            TemplateSource::User => "user".to_string(),
        },
        description: tmpl.metadata.description.clone(),
        author: tmpl.metadata.author.clone(),
        tags: tmpl.metadata.tags.clone(),
        icon_hint: tmpl.metadata.icon_hint.clone(),
        definition_json: def_json,
    }))
}

/// Registers a user template promoted from a CustomEquipmentDefinition.
pub async fn register_template(
    State(host): State<HostHandle>,
    Json(def): Json<CustomEquipmentDefinition>,
) -> ServerResult<Json<Value>> {
    let template = EquipmentTemplate::from_user_definition(def, None)
        .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;

    let tmpl_id = template.id.clone();
    let tmpl_ver = template.version.to_string();
    let tmpl_json = serde_json::to_string(&template)
        .map_err(|e| ServerError::Internal(format!("Serialization error: {e}")))?;

    let cmd = RuntimeCommand::Library(LibraryCommand::RegisterTemplate {
        template_json: tmpl_json,
    });
    host.dispatch_command(cmd).await?;

    Ok(Json(json!({
        "status": "ok",
        "template_id": tmpl_id,
        "version": tmpl_ver,
    })))
}

/// Exports an equipment template to serialized string (JSON or TOML).
pub async fn export_template_endpoint(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
    Query(params): Query<ExportParams>,
) -> ServerResult<Json<ExportLibraryItemResponseDto>> {
    let eq_mgr = host
        .equipment_manager()
        .read()
        .map_err(|_| ServerError::Internal("Equipment manager lock poisoned".to_string()))?;

    let format = match params.format.as_deref() {
        Some("toml") => SerializationFormat::Toml,
        _ => SerializationFormat::Json,
    };

    let version_req = match params.version.as_deref() {
        Some(v) => Some(
            v.parse::<SchemaVersion>()
                .map_err(|e| ServerError::InvalidRequest(format!("Invalid version string: {e}")))?,
        ),
        None => None,
    };

    let tmpl = eq_mgr
        .library()
        .get_template(&id, version_req.as_ref())
        .ok_or_else(|| ServerError::NotFound(format!("Template '{id}' not found")))?;

    let content = eq_mgr
        .library()
        .export_template(&id, version_req.as_ref(), format)
        .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;

    Ok(Json(ExportLibraryItemResponseDto {
        id: tmpl.id.clone(),
        version: tmpl.version.to_string(),
        format: match format {
            SerializationFormat::Json => "json".to_string(),
            SerializationFormat::Toml => "toml".to_string(),
        },
        content,
    }))
}

/// Imports and registers an equipment template from data.
pub async fn import_template_endpoint(
    State(host): State<HostHandle>,
    Json(req): Json<ImportLibraryItemRequestDto>,
) -> ServerResult<Json<TemplateSummaryDto>> {
    let format = match req.format.to_lowercase().as_str() {
        "toml" => SerializationFormat::Toml,
        _ => SerializationFormat::Json,
    };

    let template = nebudcs_scripting::import_template(&req.content, format)
        .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;

    let summary = TemplateSummaryDto {
        id: template.id.clone(),
        name: template.name.clone(),
        version: template.version.to_string(),
        category: format!("{:?}", template.category),
        source: match template.source {
            TemplateSource::Standard => "standard".to_string(),
            TemplateSource::User => "user".to_string(),
        },
        description: template.metadata.description.clone(),
        parameter_count: template.definition.parameters.len(),
        port_count: template.definition.ports.len(),
        curve_count: template.definition.curves.len() + template.definition.curves_2d.len(),
        trip_count: template.definition.trips.len(),
        author: template.metadata.author.clone(),
        ports: template
            .definition
            .ports
            .iter()
            .map(|p| TemplatePortDto {
                id: p.id.to_string(),
                name: p.name.clone(),
                domain: format!("{:?}", p.domain).to_lowercase(),
                direction: format!("{:?}", p.direction).to_lowercase(),
            })
            .collect(),
    };

    let tmpl_json = serde_json::to_string(&template)
        .map_err(|e| ServerError::Internal(format!("Serialization error: {e}")))?;

    let cmd = RuntimeCommand::Library(LibraryCommand::RegisterTemplate {
        template_json: tmpl_json,
    });
    host.dispatch_command(cmd).await?;

    Ok(Json(summary))
}

/// Instantiates an equipment template into the active simulation state.
pub async fn instantiate_template_endpoint(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
    Json(req): Json<InstantiateTemplateRequestDto>,
) -> ServerResult<Json<Value>> {
    let inst_id_str = req
        .instance_id
        .unwrap_or_else(|| EquipmentId::new().to_string());

    let cmd = RuntimeCommand::Library(LibraryCommand::InstantiateTemplate {
        template_id: id,
        version: req.version,
        name: req.name,
        instance_id: Some(inst_id_str.clone()),
        parameter_overrides: req.parameter_overrides,
    });
    host.dispatch_command(cmd).await?;

    Ok(Json(json!({
        "status": "ok",
        "instance_id": inst_id_str,
    })))
}

/// Lists all registered equipment packages.
pub async fn list_packages(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<PackageSummaryDto>>> {
    let eq_mgr = host
        .equipment_manager()
        .read()
        .map_err(|_| ServerError::Internal("Equipment manager lock poisoned".to_string()))?;

    let packages = eq_mgr.library().list_packages();
    let summaries = packages
        .into_iter()
        .map(|p| PackageSummaryDto {
            id: p.id.clone(),
            name: p.name.clone(),
            version: p.version.to_string(),
            description: p.description.clone(),
            author: p.author.clone(),
            sub_component_count: p.sub_components.len(),
            internal_stream_count: p.internal_streams.len(),
            external_port_count: p.external_ports.len(),
            dependency_count: p.dependencies.len(),
        })
        .collect();

    Ok(Json(summaries))
}

/// Retrieves detailed information for a specific equipment package.
pub async fn get_package(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
    Query(params): Query<PackageFilterParams>,
) -> ServerResult<Json<PackageDetailDto>> {
    let eq_mgr = host
        .equipment_manager()
        .read()
        .map_err(|_| ServerError::Internal("Equipment manager lock poisoned".to_string()))?;

    let version_req = match params.version.as_deref() {
        Some(v) => Some(
            v.parse::<SchemaVersion>()
                .map_err(|e| ServerError::InvalidRequest(format!("Invalid version string: {e}")))?,
        ),
        None => None,
    };

    let pkg = eq_mgr
        .library()
        .get_package(&id, version_req.as_ref())
        .ok_or_else(|| ServerError::NotFound(format!("Package '{id}' not found")))?;

    let pkg_json = serde_json::to_string_pretty(pkg)
        .map_err(|e| ServerError::Internal(format!("Serialization error: {e}")))?;

    Ok(Json(PackageDetailDto {
        id: pkg.id.clone(),
        name: pkg.name.clone(),
        version: pkg.version.to_string(),
        description: pkg.description.clone(),
        author: pkg.author.clone(),
        package_json: pkg_json,
    }))
}

/// Validates an equipment package without modifying catalog or runtime state.
pub async fn validate_package_endpoint(
    State(host): State<HostHandle>,
    Json(pkg): Json<EquipmentPackage>,
) -> ServerResult<Json<ValidatePackageResponseDto>> {
    let eq_mgr = host
        .equipment_manager()
        .read()
        .map_err(|_| ServerError::Internal("Equipment manager lock poisoned".to_string()))?;

    let pkg_id = pkg.id.clone();
    match eq_mgr.library().validate_package(&pkg) {
        Ok(()) => {
            let deps = eq_mgr
                .library()
                .resolve_package_dependencies(&pkg)
                .unwrap_or_default();
            Ok(Json(ValidatePackageResponseDto {
                valid: true,
                package_id: pkg_id,
                resolved_dependencies: deps,
                errors: Vec::new(),
                warnings: Vec::new(),
            }))
        }
        Err(e) => Ok(Json(ValidatePackageResponseDto {
            valid: false,
            package_id: pkg_id,
            resolved_dependencies: Vec::new(),
            errors: vec![e.to_string()],
            warnings: Vec::new(),
        })),
    }
}

/// Registers an equipment package into the catalog.
pub async fn register_package(
    State(host): State<HostHandle>,
    Json(pkg): Json<EquipmentPackage>,
) -> ServerResult<Json<Value>> {
    let pkg_id = pkg.id.clone();
    let pkg_ver = pkg.version.to_string();
    let pkg_json = serde_json::to_string(&pkg)
        .map_err(|e| ServerError::Internal(format!("Serialization error: {e}")))?;

    let cmd = RuntimeCommand::Library(LibraryCommand::RegisterPackage {
        package_json: pkg_json,
    });
    host.dispatch_command(cmd).await?;

    Ok(Json(json!({
        "status": "ok",
        "package_id": pkg_id,
        "version": pkg_ver,
    })))
}

/// Exports an equipment package to serialized data string.
pub async fn export_package_endpoint(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
    Query(params): Query<ExportParams>,
) -> ServerResult<Json<ExportLibraryItemResponseDto>> {
    let eq_mgr = host
        .equipment_manager()
        .read()
        .map_err(|_| ServerError::Internal("Equipment manager lock poisoned".to_string()))?;

    let format = match params.format.as_deref() {
        Some("toml") => SerializationFormat::Toml,
        _ => SerializationFormat::Json,
    };

    let version_req = match params.version.as_deref() {
        Some(v) => Some(
            v.parse::<SchemaVersion>()
                .map_err(|e| ServerError::InvalidRequest(format!("Invalid version string: {e}")))?,
        ),
        None => None,
    };

    let pkg = eq_mgr
        .library()
        .get_package(&id, version_req.as_ref())
        .ok_or_else(|| ServerError::NotFound(format!("Package '{id}' not found")))?;

    let content = eq_mgr
        .library()
        .export_package(&id, version_req.as_ref(), format)
        .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;

    Ok(Json(ExportLibraryItemResponseDto {
        id: pkg.id.clone(),
        version: pkg.version.to_string(),
        format: match format {
            SerializationFormat::Json => "json".to_string(),
            SerializationFormat::Toml => "toml".to_string(),
        },
        content,
    }))
}

/// Imports and registers an equipment package from raw data.
pub async fn import_package_endpoint(
    State(host): State<HostHandle>,
    Json(req): Json<ImportLibraryItemRequestDto>,
) -> ServerResult<Json<PackageSummaryDto>> {
    let format = match req.format.to_lowercase().as_str() {
        "toml" => SerializationFormat::Toml,
        _ => SerializationFormat::Json,
    };

    let package = nebudcs_scripting::import_package(&req.content, format)
        .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;

    let summary = PackageSummaryDto {
        id: package.id.clone(),
        name: package.name.clone(),
        version: package.version.to_string(),
        description: package.description.clone(),
        author: package.author.clone(),
        sub_component_count: package.sub_components.len(),
        internal_stream_count: package.internal_streams.len(),
        external_port_count: package.external_ports.len(),
        dependency_count: package.dependencies.len(),
    };

    let pkg_json = serde_json::to_string(&package)
        .map_err(|e| ServerError::Internal(format!("Serialization error: {e}")))?;

    let cmd = RuntimeCommand::Library(LibraryCommand::RegisterPackage {
        package_json: pkg_json,
    });
    host.dispatch_command(cmd).await?;

    Ok(Json(summary))
}

/// Instantiates an equipment package into child instances in the simulation.
pub async fn instantiate_package_endpoint(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
    Json(req): Json<InstantiatePackageRequestDto>,
) -> ServerResult<Json<Value>> {
    let cmd = RuntimeCommand::Library(LibraryCommand::InstantiatePackage {
        package_id: id,
        version: req.version,
        instance_prefix: req.instance_prefix.clone(),
        parameter_overrides: req.parameter_overrides,
    });
    host.dispatch_command(cmd).await?;

    Ok(Json(json!({
        "status": "ok",
        "instance_prefix": req.instance_prefix,
    })))
}

/// Lists all registered industrial domain packages.
pub async fn list_domains(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<DomainPackageSummaryDto>>> {
    let eq_mgr = host
        .equipment_manager()
        .read()
        .map_err(|_| ServerError::Internal("Equipment manager lock poisoned".to_string()))?;

    let domains = eq_mgr.library().list_domain_packages();
    let summaries = domains
        .into_iter()
        .map(|d| DomainPackageSummaryDto {
            id: d.id.clone(),
            name: d.name.clone(),
            version: d.version.to_string(),
            domain_kind: format!("{:?}", d.domain_kind),
            description: d.description.clone(),
            author: d.author.clone(),
            plant_template_ids: d.plant_template_ids.clone(),
            control_strategy_count: d.control_strategies.len(),
            scenario_preset_count: d.scenario_presets.len(),
            equipment_template_count: d.equipment_templates.len(),
            scientific_references: d.scientific_references.clone(),
            model_classifications: d.model_classifications.clone(),
        })
        .collect();

    Ok(Json(summaries))
}

/// Retrieves detailed information for a specific industrial domain package.
pub async fn get_domain(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<DomainPackageDetailDto>> {
    let eq_mgr = host
        .equipment_manager()
        .read()
        .map_err(|_| ServerError::Internal("Equipment manager lock poisoned".to_string()))?;

    let d = eq_mgr
        .library()
        .get_domain_package(&id)
        .ok_or_else(|| ServerError::NotFound(format!("Domain package '{id}' not found")))?;

    let mut ranges = std::collections::BTreeMap::new();
    for (k, r) in &d.operating_ranges {
        ranges.insert(
            k.clone(),
            OperatingRangeDto {
                quantity: r.quantity.clone(),
                min_value: r.min_value,
                max_value: r.max_value,
                unit: r.unit.clone(),
                normal_operating_value: r.normal_operating_value,
            },
        );
    }

    Ok(Json(DomainPackageDetailDto {
        id: d.id.clone(),
        name: d.name.clone(),
        version: d.version.to_string(),
        domain_kind: format!("{:?}", d.domain_kind),
        description: d.description.clone(),
        author: d.author.clone(),
        thermodynamic_requirements: d.thermodynamic_requirements.clone(),
        utility_requirements: d.utility_requirements.clone(),
        composition_defaults: d.composition_defaults.clone(),
        operating_ranges: ranges,
        control_strategies: d.control_strategies.clone(),
        scenario_presets: d.scenario_presets.clone(),
        equipment_templates: d.equipment_templates.clone(),
        plant_template_ids: d.plant_template_ids.clone(),
        validation_rules: d.validation_rules.clone(),
        scientific_references: d.scientific_references.clone(),
        model_classifications: d.model_classifications.clone(),
    }))
}
