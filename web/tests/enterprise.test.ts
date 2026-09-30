/**
 * Stage 14 Enterprise, High Availability & Distributed Runtime Test Suite.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { client } from "../src/api/client";
import {
  ClusterTopologyDto,
  FailureInjectionStatusDto,
  AuditEventDto,
  AuditSummaryDto,
  DiagnosticItemDto,
  DiagnosticsSummaryDto,
  SubsystemObservabilityDto,
} from "../src/protocol/types";
import "../src/components/nebu-cluster-panel";
import { NebuClusterPanel } from "../src/components/nebu-cluster-panel";
import "../src/components/nebu-audit-panel";
import { NebuAuditPanel } from "../src/components/nebu-audit-panel";
import "../src/components/nebu-diagnostics-view";
import { NebuDiagnosticsView } from "../src/components/nebu-diagnostics-view";
import "../src/components/nebu-backup-panel";
import { NebuBackupPanel } from "../src/components/nebu-backup-panel";
import "../src/components/nebu-system-observability";
import { NebuSystemObservability } from "../src/components/nebu-system-observability";
import "../src/components/nebu-app";
import { NebuApp } from "../src/components/nebu-app";

const MOCK_TOPOLOGY: ClusterTopologyDto = {
  cluster_id: "nebudcs-cluster-alpha",
  local_node_id: "node-01",
  current_leader_id: "node-01",
  leader_epoch: 1,
  lease_remaining_secs: 4.8,
  local_ha_state: "PRIMARY",
  nodes: [
    {
      node_id: "node-01",
      name: "Primary Simulation Host",
      role: "control_simulation",
      ha_state: "PRIMARY",
      endpoint_url: "http://127.0.0.1:8088",
      is_local: true,
      epoch: 1,
      last_heartbeat_secs_ago: 0.2,
      health_status: "HEALTHY",
      replication_lag_ms: 0.0,
    },
    {
      node_id: "node-02",
      name: "Hot Standby Replica",
      role: "standby",
      ha_state: "SYNCHRONIZED",
      endpoint_url: "http://127.0.0.1:8089",
      is_local: false,
      epoch: 1,
      last_heartbeat_secs_ago: 0.3,
      health_status: "HEALTHY",
      replication_lag_ms: 12.5,
    },
  ],
  split_brain_fencing_active: false,
  total_nodes_count: 2,
  active_nodes_count: 2,
  replication_health: "SYNCHRONIZED",
};

const MOCK_FAILURE: FailureInjectionStatusDto = {
  active: false,
  remaining_secs: 0,
  message: "Nominal",
};

const MOCK_AUDIT_EVENTS: AuditEventDto[] = [
  {
    event_id: "aud-001",
    timestamp_iso: "2026-09-08T12:00:00Z",
    sim_time_secs: 10.0,
    actor: "lead_operator",
    role: "Operator",
    source: "Console",
    target: "FIC-101",
    action: "SetpointAdjustment",
    reason: "Adjusted setpoint from 45.0 to 50.0 kg/s",
    status: "SUCCESS",
    correlation_id: "corr-12345",
  },
];

const MOCK_AUDIT_SUMMARY: AuditSummaryDto = {
  total_events_recorded: 1,
  current_buffer_size: 5000,
  unique_actors_count: 1,
  unique_actions_count: 1,
  latest_event_timestamp: "2026-09-08T12:00:00Z",
};

const MOCK_DIAGNOSTICS: DiagnosticItemDto[] = [
  {
    id: "DIAG-CLUSTER-001",
    domain: "Cluster",
    severity: "INFO",
    title: "Cluster Topology Synchronized",
    description: "All registered nodes responding within heartbeat timeout lease.",
    entity_id: "node-01",
    entity_type: "ClusterNode",
    timestamp_iso: "2026-09-08T12:00:00Z",
    suggested_action: "None required.",
  },
];

const MOCK_DIAGNOSTICS_SUMMARY: DiagnosticsSummaryDto = {
  total_active: 1,
  critical_count: 0,
  error_count: 0,
  warning_count: 0,
  info_count: 1,
};

const MOCK_OBSERVABILITY: SubsystemObservabilityDto = {
  system_cpu_pct: 4.2,
  system_memory_rss_bytes: 256 * 1024 * 1024,
  system_uptime_secs: 3600.0,
  sim_freq_hz: 50.0,
  sim_dt: 0.02,
  sim_tick: 2500,
  sim_convergence_failures: 0,
  sim_is_deterministic: true,
  control_tasks_active: 4,
  control_overruns_count: 0,
  control_faults_active: 0,
  io_channels_total: 128,
  io_bad_quality_count: 0,
  io_stale_data_count: 0,
  protocol_connections_active: 3,
  protocol_packet_rate: 150.0,
  protocol_error_count: 0,
  sis_active_trips: 0,
  sis_bypasses_active: 0,
  sis_proof_test_active: false,
  historian_write_rate_sps: 250.0,
  historian_queue_depth: 2,
  historian_total_samples: 45000,
  ots_active_sessions: 1,
  ots_scenarios_running: 1,
  validation_cases_passed: 20,
  validation_cases_total: 20,
  validation_coverage_pct: 100.0,
  cluster_ha_state: "PRIMARY",
  cluster_leader_id: "node-01",
  cluster_epoch: 1,
  cluster_replication_lag_ms: 12.5,
};

describe("NebuClusterPanel", () => {
  beforeEach(() => {
    vi.spyOn(client, "getClusterTopology").mockResolvedValue(MOCK_TOPOLOGY);
    vi.spyOn(client, "getClusterFailureStatus").mockResolvedValue(MOCK_FAILURE);
  });

  it("renders cluster topology with node cards and KPIs", async () => {
    const el = document.createElement("nebu-cluster-panel") as NebuClusterPanel;
    document.body.appendChild(el);
    await vi.waitFor(() => {
      expect(el.innerHTML).toContain("nebudcs-cluster-alpha");
      expect(el.innerHTML).toContain("Primary Simulation Host");
      expect(el.innerHTML).toContain("Hot Standby Replica");
      expect(el.innerHTML).toContain("Initiate Controlled Failover");
    });
    el.remove();
  });

  it("calls failover API when failover button is clicked", async () => {
    const spy = vi.spyOn(client, "triggerFailover").mockResolvedValue({
      success: true,
      message: "Failover successful",
      promoted_node_id: "node-02",
      new_epoch: 2,
    });

    const el = document.createElement("nebu-cluster-panel") as NebuClusterPanel;
    document.body.appendChild(el);
    await vi.waitFor(() => {
      const btn = el.querySelector("#failover-btn") as HTMLButtonElement | null;
      expect(btn).toBeTruthy();
      expect(btn?.disabled).toBe(false);
    });

    const btn = el.querySelector("#failover-btn") as HTMLButtonElement;
    btn.click();

    await vi.waitFor(() => {
      expect(spy).toHaveBeenCalled();
    });
    el.remove();
  });
});

describe("NebuAuditPanel", () => {
  beforeEach(() => {
    vi.spyOn(client, "getAuditEvents").mockResolvedValue(MOCK_AUDIT_EVENTS);
    vi.spyOn(client, "getAuditSummary").mockResolvedValue(MOCK_AUDIT_SUMMARY);
  });

  it("renders audit journal entries and summary counts", async () => {
    const el = document.createElement("nebu-audit-panel") as NebuAuditPanel;
    document.body.appendChild(el);
    await vi.waitFor(() => {
      expect(el.innerHTML).toContain("lead_operator");
      expect(el.innerHTML).toContain("SetpointAdjustment");
      expect(el.innerHTML).toContain("FIC-101");
      expect(el.innerHTML).toContain("Total Logged Events");
    });
    el.remove();
  });
});

describe("NebuDiagnosticsView", () => {
  beforeEach(() => {
    vi.spyOn(client, "getDiagnostics").mockResolvedValue(MOCK_DIAGNOSTICS);
    vi.spyOn(client, "getDiagnosticsSummary").mockResolvedValue(MOCK_DIAGNOSTICS_SUMMARY);
  });

  it("renders diagnostics items and domain filters", async () => {
    const el = document.createElement("nebu-diagnostics-view") as NebuDiagnosticsView;
    document.body.appendChild(el);
    await vi.waitFor(() => {
      expect(el.innerHTML).toContain("Cluster Topology Synchronized");
      expect(el.innerHTML).toContain("node-01");
      expect(el.innerHTML).toContain("Global Subsystem Diagnostics");
    });
    el.remove();
  });
});

describe("NebuSystemObservability", () => {
  beforeEach(() => {
    vi.spyOn(client, "getSubsystemObservability").mockResolvedValue(MOCK_OBSERVABILITY);
  });

  it("renders all 10 subsystem telemetry sections", async () => {
    const el = document.createElement("nebu-system-observability") as NebuSystemObservability;
    document.body.appendChild(el);
    await vi.waitFor(() => {
      expect(el.innerHTML).toContain("Host &amp; Process Runtime");
      expect(el.innerHTML).toContain("Simulation Engine");
      expect(el.innerHTML).toContain("Virtual DCS &amp; Controllers");
      expect(el.innerHTML).toContain("Industrial I/O HAL");
      expect(el.innerHTML).toContain("Industrial Protocols");
      expect(el.innerHTML).toContain("SIS Functional Safety");
      expect(el.innerHTML).toContain("Industrial Historian");
      expect(el.innerHTML).toContain("Training Simulator (OTS)");
      expect(el.innerHTML).toContain("Scientific Validation");
      expect(el.innerHTML).toContain("High Availability Cluster");
    });
    el.remove();
  });
});

describe("NebuBackupPanel", () => {
  it("renders backup and restore controls", () => {
    const el = document.createElement("nebu-backup-panel") as NebuBackupPanel;
    document.body.appendChild(el);
    expect(el.innerHTML).toContain("Export Full Enterprise Backup Bundle");
    expect(el.innerHTML).toContain("Dry-Run Validation &amp; Production Restore");
    el.remove();
  });
});

describe("NebuApp Enterprise Integration", () => {
  beforeEach(() => {
    vi.spyOn(client, "getClusterTopology").mockResolvedValue(MOCK_TOPOLOGY);
    vi.spyOn(client, "getClusterFailureStatus").mockResolvedValue(MOCK_FAILURE);
    vi.spyOn(client, "getSubsystemObservability").mockResolvedValue(MOCK_OBSERVABILITY);
  });

  it("renders global status bar and switches to enterprise mode", async () => {
    const app = document.createElement("nebu-app") as NebuApp;
    document.body.appendChild(app);

    await vi.waitFor(() => {
      expect(app.querySelector("#globalStatusBar")).toBeTruthy();
    });

    const entTab = app.querySelector('.mode-tab[data-mode="enterprise"]') as HTMLButtonElement;
    expect(entTab).toBeTruthy();
    entTab.click();

    await vi.waitFor(() => {
      expect(app.querySelector("nebu-cluster-panel")).toBeTruthy();
      expect(app.innerHTML).toContain("Cluster &amp; HA Topology");
      expect(app.innerHTML).toContain("Subsystem Observability");
    });

    app.remove();
  });
});
