//! Custom equipment authoring, validation, instantiation, and runtime control endpoints.

use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

use nebudcs_protocol::{
    EquipmentCommand, EquipmentControlRequestDto, EquipmentDefinitionSummaryDto,
    EquipmentInstanceSummaryDto, InstantiateEquipmentRequestDto, RuntimeCommand,
    ValidateEquipmentResponseDto,
};
use nebudcs_scripting::{validation::validate_definition, CustomEquipmentDefinition};
use nebudcs_types::EquipmentId;

use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;

/// Validates a custom equipment definition without mutating runtime state.
pub async fn validate_equipment(
    Json(def): Json<CustomEquipmentDefinition>,
) -> Json<ValidateEquipmentResponseDto> {
    let def_id = def.id.clone();
    match validate_definition(&def) {
        Ok(()) => Json(ValidateEquipmentResponseDto {
            valid: true,
            definition_id: def_id,
            errors: Vec::new(),
            warnings: Vec::new(),
        }),
        Err(e) => Json(ValidateEquipmentResponseDto {
            valid: false,
            definition_id: def_id,
            errors: vec![e.to_string()],
            warnings: Vec::new(),
        }),
    }
}

/// Registers a new custom equipment definition.
pub async fn define_equipment(
    State(host): State<HostHandle>,
    Json(def): Json<CustomEquipmentDefinition>,
) -> ServerResult<Json<Value>> {
    // Validate first
    validate_definition(&def).map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
    let def_id = def.id.clone();
    let def_json =
        serde_json::to_string(&def).map_err(|e| ServerError::InvalidRequest(e.to_string()))?;

    let cmd = RuntimeCommand::Equipment(EquipmentCommand::Define {
        definition_json: def_json,
    });
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok", "definition_id": def_id })))
}

/// Lists all registered custom equipment definitions.
pub async fn list_definitions(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<EquipmentDefinitionSummaryDto>>> {
    let eq_mgr = host
        .equipment_manager()
        .read()
        .map_err(|_| ServerError::Internal("Equipment manager lock poisoned".to_string()))?;

    let summaries = eq_mgr
        .list_definition_objects()
        .into_iter()
        .map(|d| EquipmentDefinitionSummaryDto {
            id: d.id.clone(),
            name: d.name.clone(),
            version: d.version.to_string(),
            category: format!("{:?}", d.category),
            tier: format!("{:?}", d.tier),
            description: d.description.clone(),
            parameter_count: d.parameters.len(),
            port_count: d.ports.len(),
            curve_count: d.curves.len() + d.curves_2d.len(),
            trip_count: d.trips.len(),
            interlock_count: d.interlocks.len(),
            has_script: d.script.is_some(),
            has_composition: d.composition.is_some(),
        })
        .collect();

    Ok(Json(summaries))
}

/// Retrieves a specific custom equipment definition by ID.
pub async fn get_definition(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<CustomEquipmentDefinition>> {
    let eq_mgr = host
        .equipment_manager()
        .read()
        .map_err(|_| ServerError::Internal("Equipment manager lock poisoned".to_string()))?;

    let def = eq_mgr
        .get_definition(&id)
        .ok_or_else(|| ServerError::NotFound(format!("Equipment definition '{id}' not found")))?;

    Ok(Json(def.clone()))
}

/// Instantiates a custom equipment definition into the runtime simulation.
pub async fn instantiate_equipment(
    State(host): State<HostHandle>,
    Json(req): Json<InstantiateEquipmentRequestDto>,
) -> ServerResult<Json<Value>> {
    let inst_id_str = req
        .instance_id
        .unwrap_or_else(|| EquipmentId::new().to_string());
    let cmd = RuntimeCommand::Equipment(EquipmentCommand::Instantiate {
        definition_id: req.definition_id,
        name: req.name,
        parameter_overrides: req.parameter_overrides,
        instance_id: Some(inst_id_str.clone()),
    });
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok", "instance_id": inst_id_str })))
}

/// Lists all active custom equipment instances.
pub async fn list_instances(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<EquipmentInstanceSummaryDto>>> {
    let eq_mgr = host
        .equipment_manager()
        .read()
        .map_err(|_| ServerError::Internal("Equipment manager lock poisoned".to_string()))?;

    let instances = eq_mgr
        .list_instances()
        .into_iter()
        .map(|inst| EquipmentInstanceSummaryDto {
            id: inst.id.to_string(),
            definition_id: inst.definition_id.clone(),
            name: inst.name.clone(),
            active_state: inst.active_state.clone(),
            is_running: inst.is_running(),
            is_tripped: inst.is_tripped(),
            latched_trips: inst.latched_trips.clone(),
            parameters: inst.parameters.clone(),
            variables: inst.variables.clone(),
            flags: inst.flags.clone(),
        })
        .collect();

    Ok(Json(instances))
}

/// Retrieves a specific custom equipment instance by ID.
pub async fn get_instance(
    State(host): State<HostHandle>,
    Path(id): Path<String>,
) -> ServerResult<Json<EquipmentInstanceSummaryDto>> {
    let eq_id = id
        .parse::<EquipmentId>()
        .map_err(|e| ServerError::InvalidRequest(format!("Invalid EquipmentId: {e}")))?;

    let eq_mgr = host
        .equipment_manager()
        .read()
        .map_err(|_| ServerError::Internal("Equipment manager lock poisoned".to_string()))?;

    let inst = eq_mgr
        .get_instance(eq_id)
        .ok_or_else(|| ServerError::NotFound(format!("Equipment instance '{id}' not found")))?;

    Ok(Json(EquipmentInstanceSummaryDto {
        id: inst.id.to_string(),
        definition_id: inst.definition_id.clone(),
        name: inst.name.clone(),
        active_state: inst.active_state.clone(),
        is_running: inst.is_running(),
        is_tripped: inst.is_tripped(),
        latched_trips: inst.latched_trips.clone(),
        parameters: inst.parameters.clone(),
        variables: inst.variables.clone(),
        flags: inst.flags.clone(),
    }))
}

/// Dispatches runtime control actions (start, stop, reset_trip, set_var) to an equipment instance.
pub async fn control_equipment(
    State(host): State<HostHandle>,
    Json(req): Json<EquipmentControlRequestDto>,
) -> ServerResult<Json<Value>> {
    let cmd = match req.action.to_lowercase().as_str() {
        "start" => EquipmentCommand::Start {
            instance_id: req.instance_id.clone(),
        },
        "stop" => EquipmentCommand::Stop {
            instance_id: req.instance_id.clone(),
        },
        "reset_trip" | "reset" => EquipmentCommand::ResetTrip {
            instance_id: req.instance_id.clone(),
        },
        "set_var" | "set_variable" => {
            let var_name = req.variable_name.ok_or_else(|| {
                ServerError::InvalidRequest(
                    "Missing 'variable_name' for set_var action".to_string(),
                )
            })?;
            let value = req.value.ok_or_else(|| {
                ServerError::InvalidRequest("Missing 'value' for set_var action".to_string())
            })?;
            EquipmentCommand::SetVariable {
                instance_id: req.instance_id.clone(),
                var_name,
                value,
            }
        }
        other => {
            return Err(ServerError::InvalidRequest(format!(
                "Unknown equipment control action: '{other}'"
            )))
        }
    };

    host.dispatch_command(RuntimeCommand::Equipment(cmd))
        .await?;
    Ok(Json(json!({
        "status": "ok",
        "instance_id": req.instance_id,
        "action": req.action
    })))
}
