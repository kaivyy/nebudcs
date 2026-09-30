//! I/O Hardware Abstraction Layer (HAL) REST management and diagnostic endpoints.

use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

use nebudcs_protocol::{HalChannelDto, HalFaultRequestDto, HalMappingDto, HalSystemSummaryDto};

use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;
use crate::security::{audit_log, UserRole};

/// Lists all configured I/O HAL systems including diagnostic counters.
pub async fn list_hal_systems(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<HalSystemSummaryDto>>> {
    Ok(Json(host.get_hal_systems().await))
}

/// Retrieves detailed state, ADC/DAC counts, NAMUR NE43 status, and quality for a channel.
pub async fn get_hal_channel(
    State(host): State<HostHandle>,
    Path(address): Path<String>,
) -> ServerResult<Json<HalChannelDto>> {
    if let Some(ch) = host.get_hal_channel(&address).await {
        Ok(Json(ch))
    } else {
        Err(ServerError::NotFound(format!(
            "Channel address '{address}' not found"
        )))
    }
}

/// Injects or clears hardware faults / forced value overrides on an I/O channel.
///
/// Requires Engineer or Admin privileges per ISA-62443 / RBAC.
pub async fn set_channel_fault(
    State(host): State<HostHandle>,
    role: UserRole,
    Path(address): Path<String>,
    Json(req): Json<HalFaultRequestDto>,
) -> ServerResult<Json<Value>> {
    if !role.can_modify_topology() {
        audit_log(
            "HAL_FAULT_INJECTION",
            &format!("{role}"),
            &format!("/api/v1/hal/channels/{address}/fault"),
            "Unauthorized attempt to inject or clear hardware channel fault",
            false,
        );
        return Err(ServerError::Forbidden(
            "Engineer or Admin role required to inject or mutate hardware channel faults"
                .to_string(),
        ));
    }

    let cmd = nebudcs_protocol::RuntimeCommand::HalChannelFault {
        channel_address: address.clone(),
        fault: req.fault.clone(),
        forced_value: req.forced_value,
    };

    host.dispatch_command(cmd).await?;

    audit_log(
        "HAL_FAULT_INJECTION",
        &format!("{role}"),
        &format!("/api/v1/hal/channels/{address}/fault"),
        &format!(
            "Fault '{:?}' / forced value '{:?}' applied",
            req.fault, req.forced_value
        ),
        true,
    );

    Ok(Json(json!({
        "status": "ok",
        "address": address,
        "fault": req.fault,
        "forced_value": req.forced_value,
    })))
}

/// Lists all active controller-to-I/O mappings.
pub async fn list_hal_mappings(
    State(host): State<HostHandle>,
) -> ServerResult<Json<Vec<HalMappingDto>>> {
    Ok(Json(host.get_hal_mappings().await))
}
