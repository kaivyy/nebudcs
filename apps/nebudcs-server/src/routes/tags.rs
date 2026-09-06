//! Runtime tag inspection and mutation endpoints.

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use nebudcs_protocol::{RuntimeCommand, TagValueDto, TagValueSampleDto};

use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagListResponse {
    pub count: usize,
    pub tags: Vec<TagValueSampleDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteTagRequest {
    #[serde(alias = "tag_id")]
    pub tag: String,
    pub value: TagValueDto,
}

/// Retrieves all current tags from the latest coherent snapshot.
pub async fn list_tags(State(host): State<HostHandle>) -> Json<TagListResponse> {
    let snapshot = host.get_snapshot().await;
    let count = snapshot.tags.len();
    Json(TagListResponse {
        count,
        tags: snapshot.tags,
    })
}

/// Retrieves a specific tag by tag name/ID.
pub async fn get_tag(
    State(host): State<HostHandle>,
    Path(tag_id): Path<String>,
) -> ServerResult<Json<TagValueSampleDto>> {
    let snapshot = host.get_snapshot().await;
    let tag = snapshot
        .tags
        .into_iter()
        .find(|t| t.tag == tag_id)
        .ok_or_else(|| ServerError::NotFound(format!("Tag '{tag_id}' not found")))?;
    Ok(Json(tag))
}

/// Writes an operator or external setpoint/value to a runtime tag.
pub async fn write_tag(
    State(host): State<HostHandle>,
    Json(req): Json<WriteTagRequest>,
) -> ServerResult<Json<Value>> {
    let cmd = RuntimeCommand::WriteTag {
        tag: req.tag,
        value: req.value,
    };
    host.dispatch_command(cmd).await?;
    Ok(Json(json!({ "status": "ok" })))
}
