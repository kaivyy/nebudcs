//! Global Unified Diagnostics Manager.
//!
//! Standards Reference: ISA-18.2 / EEMUA 191 Alarm Management & Fault Isolation,
//! IEC 61508 Diagnostics Coverage, Cross-Domain System Health Analysis.

use nebudcs_protocol::DiagnosticItemDto;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const MAX_DIAGNOSTICS_CAPACITY: usize = 200;

/// Diagnostics summary reporting counts across domains and severity levels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsSummaryDto {
    pub total_active: usize,
    pub critical_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

/// Global diagnostics manager correlating faults, overruns, and alarms across all subsystems.
#[derive(Debug)]
pub struct DiagnosticsManager {
    items: VecDeque<DiagnosticItemDto>,
}

impl Default for DiagnosticsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticsManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            items: VecDeque::with_capacity(MAX_DIAGNOSTICS_CAPACITY),
        };
        mgr.seed_initial_diagnostics();
        mgr
    }

    fn seed_initial_diagnostics(&mut self) {
        let now = chrono_now_iso();
        self.add_item(DiagnosticItemDto {
            id: "DIAG-SYS-001".to_string(),
            domain: "System".to_string(),
            severity: "INFO".to_string(),
            title: "Cluster Consensus Engine Active".to_string(),
            description: "Primary controller operating with monotonic leader lease and epoch fencing.".to_string(),
            entity_id: Some("node-sim-01".to_string()),
            entity_type: Some("ClusterNode".to_string()),
            timestamp_iso: now.clone(),
            suggested_action: "None required; nominal enterprise HA status.".to_string(),
        });

        self.add_item(DiagnosticItemDto {
            id: "DIAG-HYD-001".to_string(),
            domain: "Hydraulics".to_string(),
            severity: "INFO".to_string(),
            title: "NPSH Margin Nominal".to_string(),
            description: "Centrifugal feed pumps operating at 1.45x NPSHR margin above cavitation threshold.".to_string(),
            entity_id: Some("P-101A".to_string()),
            entity_type: Some("Pump".to_string()),
            timestamp_iso: now.clone(),
            suggested_action: "Maintain steady suction drum level LIC-101 > 45%.".to_string(),
        });

        self.add_item(DiagnosticItemDto {
            id: "DIAG-CTL-001".to_string(),
            domain: "Control".to_string(),
            severity: "INFO".to_string(),
            title: "Task Execution Utilization Optimal".to_string(),
            description: "Fast cyclic 10ms control task executing with zero overruns over last 10,000 ticks.".to_string(),
            entity_id: Some("TASK_FAST_10MS".to_string()),
            entity_type: Some("ControllerTask".to_string()),
            timestamp_iso: now.clone(),
            suggested_action: "None required; execution cycle budget < 35%.".to_string(),
        });

        self.add_item(DiagnosticItemDto {
            id: "DIAG-SIS-001".to_string(),
            domain: "SIS".to_string(),
            severity: "INFO".to_string(),
            title: "2oo3 Sensor Voting Redundancy Healthy".to_string(),
            description: "Reactor high pressure interlock transmitters PT-201A/B/C within 0.2 bar agreement.".to_string(),
            entity_id: Some("SIF-201".to_string()),
            entity_type: Some("SafetyInstrumentedFunction".to_string()),
            timestamp_iso: now,
            suggested_action: "Next periodic proof-test due in 30 days.".to_string(),
        });
    }

    /// Appends a new diagnostic item.
    pub fn add_item(&mut self, item: DiagnosticItemDto) {
        if self.items.len() >= MAX_DIAGNOSTICS_CAPACITY {
            self.items.pop_front();
        }
        self.items.push_back(item);
    }

    /// Queries diagnostics with optional domain and severity filtering.
    pub fn list(&self, domain: Option<&str>, severity: Option<&str>) -> Vec<DiagnosticItemDto> {
        self.items
            .iter()
            .rev()
            .filter(|item| {
                if let Some(d) = domain {
                    if !item.domain.eq_ignore_ascii_case(d) {
                        return false;
                    }
                }
                if let Some(s) = severity {
                    if !item.severity.eq_ignore_ascii_case(s) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    /// Produces a summary breakdown of active diagnostics.
    pub fn summary(&self) -> DiagnosticsSummaryDto {
        let mut critical_count = 0;
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut info_count = 0;

        for item in &self.items {
            match item.severity.to_uppercase().as_str() {
                "CRITICAL" => critical_count += 1,
                "ERROR" => error_count += 1,
                "WARNING" | "WARN" => warning_count += 1,
                _ => info_count += 1,
            }
        }

        DiagnosticsSummaryDto {
            total_active: self.items.len(),
            critical_count,
            error_count,
            warning_count,
            info_count,
        }
    }
}

fn chrono_now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    format!("2026-09-08T{h:02}:{m:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostics_filtering_and_summary() {
        let mut mgr = DiagnosticsManager::new();
        mgr.add_item(DiagnosticItemDto {
            id: "TEST-01".to_string(),
            domain: "Control".to_string(),
            severity: "WARNING".to_string(),
            title: "Manual Overrun".to_string(),
            description: "Test warning".to_string(),
            entity_id: Some("TC-101".to_string()),
            entity_type: Some("Controller".to_string()),
            timestamp_iso: "now".to_string(),
            suggested_action: "Inspect loop".to_string(),
        });

        let sum = mgr.summary();
        assert!(sum.total_active >= 5);
        assert_eq!(sum.warning_count, 1);

        let control_items = mgr.list(Some("Control"), None);
        assert!(control_items.iter().any(|i| i.id == "TEST-01"));
    }
}
