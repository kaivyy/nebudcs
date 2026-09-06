//! Simulation run isolation and session metadata.

use nebudcs_types::ProjectId;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Generates a new, globally unique, sortable simulation run ID using ULID.
pub fn generate_run_id() -> String {
    Ulid::new().to_string()
}

/// Metadata describing a specific simulation run session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistorianRunMeta {
    /// Unique 26-character Base32 ULID identifying this run.
    pub run_id: String,
    /// Parent project identifier.
    pub project_id: ProjectId,
    /// Plant model version or git hash at the time of run execution.
    pub model_version: String,
    /// Real-world start time in UTC milliseconds since epoch.
    pub started_wall_clock: i64,
    /// Real-world completion/termination time in UTC milliseconds.
    pub ended_wall_clock: Option<i64>,
    /// Nominal fixed simulation timestep ($\Delta t$).
    pub simulation_dt: f64,
    /// Nominal time scale factor (e.g. 1.0 = 1x real time).
    pub time_scale: f64,
    /// Optional user or scenario description.
    pub comments: Option<String>,
}

impl HistorianRunMeta {
    /// Creates a new run metadata entry with the current wall-clock timestamp.
    pub fn new(
        project_id: ProjectId,
        model_version: impl Into<String>,
        simulation_dt: f64,
        time_scale: f64,
    ) -> Self {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        Self {
            run_id: generate_run_id(),
            project_id,
            model_version: model_version.into(),
            started_wall_clock: now_ms,
            ended_wall_clock: None,
            simulation_dt,
            time_scale,
            comments: None,
        }
    }
}
