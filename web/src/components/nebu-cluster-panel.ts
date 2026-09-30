/**
 * Enterprise Cluster & High Availability Manager Web Component.
 *
 * Implements Stage 14: Enterprise, High Availability & Distributed Runtime.
 * Features:
 * - Active cluster node topology visualization (Primary, Standby, Historian, Communication, Gateway)
 * - Leader epoch and heartbeat monitoring with split-brain protection indicators
 * - Manual and automated failover control
 * - State replication snapshot generation and SHA-256 integrity digest inspection
 * - Controlled failure injection testing (heartbeat delay, replication lag, network partition, node crash)
 */

import { client } from "../api/client";
import {
  ClusterTopologyDto,
  HaStateDto,
  FailureInjectionStatusDto,
  ReplicationSnapshotDto,
} from "../protocol/types";

export class NebuClusterPanel extends HTMLElement {
  private topology: ClusterTopologyDto | null = null;
  private failureStatus: FailureInjectionStatusDto | null = null;
  private latestSnapshot: ReplicationSnapshotDto | null = null;
  private isLoading = false;
  private statusMessage: string | null = null;
  private isError = false;
  private refreshTimer: number | null = null;

  async connectedCallback() {
    await this.refresh();
    this.refreshTimer = window.setInterval(() => this.refresh(true), 3000);
  }

  disconnectedCallback() {
    if (this.refreshTimer !== null) {
      clearInterval(this.refreshTimer);
      this.refreshTimer = null;
    }
  }

  private async refresh(silent = false) {
    if (!silent) {
      this.isLoading = true;
      this.render();
    }
    try {
      const [topo, fail] = await Promise.all([
        client.getClusterTopology(),
        client.getClusterFailureStatus(),
      ]);
      this.topology = topo;
      this.failureStatus = fail;
      this.isError = false;
    } catch (err: unknown) {
      if (!silent) {
        this.isError = true;
        this.statusMessage = `Cluster communication error: ${err instanceof Error ? err.message : String(err)}`;
      }
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private async initiateFailover(force: boolean) {
    if (!this.topology) return;
    const standbyNode = this.topology.nodes.find(n => n.role === "standby" || !n.is_local);
    const targetLeaderId = standbyNode ? standbyNode.node_id : this.topology.local_node_id;

    try {
      this.isLoading = true;
      this.render();
      const res = await client.triggerFailover({
        target_leader_id: targetLeaderId,
        target_epoch: this.topology.leader_epoch + 1,
        force,
        reason: "Manual operator failover initiated from Web Console",
      });
      this.statusMessage = res.message;
      this.isError = !res.success;
      await this.refresh();
    } catch (err: unknown) {
      this.isError = true;
      this.statusMessage = `Failover error: ${err instanceof Error ? err.message : String(err)}`;
      this.render();
    }
  }

  private async triggerReplicationSnapshot() {
    try {
      this.isLoading = true;
      this.render();
      const snap = await client.getReplicationSnapshot();
      this.latestSnapshot = snap;
      this.statusMessage = `Snapshot created for epoch ${snap.epoch} (Tick ${snap.sim_tick}, SHA-256: ${snap.checksum_sha256.slice(0, 12)}...)`;
      this.isError = false;
    } catch (err: unknown) {
      this.isError = true;
      this.statusMessage = `Replication error: ${err instanceof Error ? err.message : String(err)}`;
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private async injectFailure(mode: string, durationMs: number) {
    try {
      this.isLoading = true;
      this.render();
      const res = await client.injectClusterFailure({
        failure_type: mode,
        duration_secs: Math.round(durationMs / 1000),
        target_node_id: this.topology?.local_node_id,
        induced_lag_ms: mode === "lag_replication" ? 450 : undefined,
      });
      this.statusMessage = `Failure injected: ${res.failure_type || mode} for ${res.remaining_secs}s`;
      this.failureStatus = res;
      this.isError = false;
    } catch (err: unknown) {
      this.isError = true;
      this.statusMessage = `Injection error: ${err instanceof Error ? err.message : String(err)}`;
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private async clearInjectedFailure() {
    try {
      this.isLoading = true;
      this.render();
      const res = await client.clearClusterFailure();
      this.failureStatus = res;
      this.statusMessage = "Active injected failure cleared. Cluster nominal.";
      this.isError = false;
    } catch (err: unknown) {
      this.isError = true;
      this.statusMessage = `Clear error: ${err instanceof Error ? err.message : String(err)}`;
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private getHaBadgeClass(state: HaStateDto): string {
    switch (state) {
      case "PRIMARY":
      case "SYNCHRONIZED":
        return "badge-good";
      case "STANDBY":
      case "SYNCHRONIZING":
        return "badge-info";
      case "DEGRADED":
      case "RECOVERING":
        return "badge-warn";
      case "FAILOVER":
      case "FAILED":
        return "badge-alarm";
      default:
        return "badge-neutral";
    }
  }

  render() {
    const topo = this.topology;
    const fail = this.failureStatus;

    this.innerHTML = `
      <style>
        nebu-cluster-panel {
          display: block;
          padding: 16px;
          color: var(--text-color, #e0e6ed);
          background: var(--bg-dark, #131822);
          font-family: var(--font-family, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif);
          box-sizing: border-box;
          height: 100%;
          overflow-y: auto;
        }
        .header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          border-bottom: 1px solid #232c3d;
          padding-bottom: 12px;
          margin-bottom: 16px;
        }
        .title-area h2 {
          margin: 0;
          font-size: 1.3rem;
          font-weight: 600;
          color: #fff;
          display: flex;
          align-items: center;
          gap: 10px;
        }
        .subtitle {
          color: #8899ac;
          font-size: 0.85rem;
          margin-top: 4px;
        }
        .kpi-grid {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
          gap: 12px;
          margin-bottom: 16px;
        }
        .kpi-card {
          background: #1a2232;
          border: 1px solid #28354a;
          border-radius: 6px;
          padding: 12px 14px;
        }
        .kpi-title {
          font-size: 0.75rem;
          color: #8292a8;
          text-transform: uppercase;
          letter-spacing: 0.5px;
          margin-bottom: 6px;
        }
        .kpi-value {
          font-size: 1.4rem;
          font-weight: bold;
          color: #fff;
        }
        .nodes-grid {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
          gap: 14px;
          margin-bottom: 20px;
        }
        .node-card {
          background: #171f2e;
          border: 1px solid #263346;
          border-radius: 8px;
          padding: 14px;
          position: relative;
        }
        .node-card.primary {
          border-color: #27ae60;
          box-shadow: 0 0 10px rgba(39, 174, 96, 0.15);
        }
        .node-card.local {
          border-left: 4px solid #3498db;
        }
        .node-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 10px;
        }
        .node-title {
          font-weight: 600;
          font-size: 1.05rem;
          color: #fff;
        }
        .node-meta {
          font-size: 0.82rem;
          color: #9cb0c6;
          display: flex;
          flex-direction: column;
          gap: 4px;
          margin-bottom: 12px;
        }
        .badge {
          display: inline-block;
          padding: 3px 8px;
          border-radius: 4px;
          font-size: 0.75rem;
          font-weight: bold;
          text-transform: uppercase;
        }
        .badge-good { background: #1b4332; color: #52b788; border: 1px solid #2d6a4f; }
        .badge-info { background: #0c3852; color: #48cae4; border: 1px solid #0077b6; }
        .badge-warn { background: #523b08; color: #ffb703; border: 1px solid #946800; }
        .badge-alarm { background: #4a151b; color: #e63946; border: 1px solid #9b2226; }
        .badge-neutral { background: #222938; color: #a0aec0; border: 1px solid #3d4a61; }
        .btn {
          padding: 8px 14px;
          border: 1px solid #35455e;
          background: #1f2a3c;
          color: #e2e8f0;
          border-radius: 5px;
          cursor: pointer;
          font-size: 0.85rem;
          font-weight: 500;
          transition: background 0.15s ease;
        }
        .btn:hover { background: #28374f; border-color: #4a5d7c; }
        .btn-primary { background: #2563eb; border-color: #3b82f6; color: white; }
        .btn-primary:hover { background: #1d4ed8; }
        .btn-danger { background: #dc2626; border-color: #ef4444; color: white; }
        .btn-danger:hover { background: #b91c1c; }
        .control-section {
          background: #171f2e;
          border: 1px solid #263346;
          border-radius: 8px;
          padding: 16px;
          margin-bottom: 18px;
        }
        .section-title {
          font-size: 1rem;
          font-weight: 600;
          margin-bottom: 12px;
          color: #fff;
          display: flex;
          align-items: center;
          gap: 8px;
        }
        .button-bar {
          display: flex;
          flex-wrap: wrap;
          gap: 10px;
          align-items: center;
        }
        .status-banner {
          padding: 10px 14px;
          border-radius: 6px;
          font-size: 0.85rem;
          margin-bottom: 14px;
        }
        .status-banner.error { background: #451217; color: #f87171; border: 1px solid #7f1d1d; }
        .status-banner.info { background: #0f2d4a; color: #7dd3fc; border: 1px solid #075985; }
        .hash-code {
          font-family: monospace;
          background: #0f141d;
          padding: 2px 6px;
          border-radius: 3px;
          font-size: 0.8rem;
          color: #a5b4fc;
        }
      </style>

      <div class="header">
        <div class="title-area">
          <h2>
            <span>⛊</span> Enterprise High Availability & Cluster Topology
          </h2>
          <div class="subtitle">Distributed Runtime Coordination • Monotonic Epoch Fencing • State Replication</div>
        </div>
        <div>
          <button class="btn" id="refresh-btn" ${this.isLoading ? "disabled" : ""}>↻ Refresh State</button>
        </div>
      </div>

      ${this.statusMessage ? `
        <div class="status-banner ${this.isError ? "error" : "info"}">
          ${this.statusMessage}
        </div>
      ` : ""}

      ${fail?.active ? `
        <div class="status-banner error" style="display: flex; justify-content: space-between; align-items: center;">
          <div>
            <strong>⚠️ Active Failure Injected:</strong> ${fail.failure_type || "Generic"} (${fail.target_node_id || "cluster"})
            — Remaining duration: ${Math.round(fail.remaining_secs)}s
          </div>
          <button class="btn btn-danger" id="clear-failure-btn">Clear Injected Failure</button>
        </div>
      ` : ""}

      <div class="kpi-grid">
        <div class="kpi-card">
          <div class="kpi-title">Cluster ID</div>
          <div class="kpi-value" style="font-size: 1.1rem; color: #60a5fa;">${topo?.cluster_id || "nebudcs-cluster"}</div>
        </div>
        <div class="kpi-card">
          <div class="kpi-title">Active Leader Node</div>
          <div class="kpi-value" style="font-size: 1.1rem; color: #34d399;">${topo?.current_leader_id || "Unassigned"}</div>
        </div>
        <div class="kpi-card">
          <div class="kpi-title">Leader Epoch</div>
          <div class="kpi-value">${topo?.leader_epoch ?? 0}</div>
        </div>
        <div class="kpi-card">
          <div class="kpi-title">HA State</div>
          <div class="kpi-value">
            <span class="badge ${this.getHaBadgeClass(topo?.local_ha_state || "STANDALONE")}">
              ${topo?.local_ha_state || "STANDALONE"}
            </span>
          </div>
        </div>
        <div class="kpi-card">
          <div class="kpi-title">Replication Lag</div>
          <div class="kpi-value" style="color: ${(topo?.nodes || []).some(n => n.replication_lag_ms > 50) ? "#f59e0b" : "#10b981"}">
            ${Math.round((topo?.nodes || []).reduce((max, n) => Math.max(max, n.replication_lag_ms), 0))} ms
          </div>
        </div>
      </div>

      <div class="section-title">Cluster Nodes (${topo?.nodes.length || 0})</div>
      <div class="nodes-grid">
        ${(topo?.nodes || []).map(node => `
          <div class="node-card ${node.ha_state === "PRIMARY" ? "primary" : ""} ${node.is_local ? "local" : ""}" data-testid="node-${node.node_id}">
            <div class="node-header">
              <span class="node-title">${node.name}</span>
              <span class="badge ${this.getHaBadgeClass(node.ha_state)}">${node.ha_state}</span>
            </div>
            <div class="node-meta">
              <div><strong>Node ID:</strong> <span class="hash-code">${node.node_id}</span> ${node.is_local ? "<span style='color: #60a5fa;'>(Local Host)</span>" : ""}</div>
              <div><strong>Role:</strong> ${node.role}</div>
              <div><strong>Endpoint:</strong> ${node.endpoint_url}</div>
              <div><strong>Epoch:</strong> ${node.epoch}</div>
              <div><strong>Last Heartbeat:</strong> ${node.last_heartbeat_secs_ago.toFixed(1)}s ago</div>
              <div><strong>Replication Lag:</strong> ${Math.round(node.replication_lag_ms)} ms</div>
              <div><strong>Health:</strong> <span style="color: ${node.health_status === "HEALTHY" ? "#34d399" : "#f87171"}">${node.health_status}</span></div>
            </div>
          </div>
        `).join("")}
      </div>

      <div class="control-section">
        <div class="section-title">High Availability & Failover Controls</div>
        <div class="button-bar">
          <button class="btn btn-primary" id="failover-btn" data-testid="failover-btn" ${this.isLoading ? "disabled" : ""}>
            ⇄ Initiate Controlled Failover
          </button>
          <button class="btn btn-danger" id="force-takeover-btn" ${this.isLoading ? "disabled" : ""}>
            ⚡ Force Leader Takeover
          </button>
          <button class="btn" id="snapshot-btn" data-testid="snapshot-btn" ${this.isLoading ? "disabled" : ""}>
            ⛁ Capture Replication Snapshot
          </button>
        </div>
        ${this.latestSnapshot ? `
          <div style="margin-top: 12px; padding: 10px; background: #111722; border-radius: 4px; font-size: 0.85rem;">
            <div><strong>Latest Snapshot:</strong> Epoch ${this.latestSnapshot.epoch} | Tick ${this.latestSnapshot.sim_tick} | Sim Time: ${this.latestSnapshot.sim_time_secs.toFixed(1)}s</div>
            <div><strong>SHA-256 Digest:</strong> <span class="hash-code">${this.latestSnapshot.checksum_sha256}</span></div>
          </div>
        ` : ""}
      </div>

      <div class="control-section">
        <div class="section-title">Controlled Infrastructure Failure Injection</div>
        <p style="color: #94a3b8; font-size: 0.85rem; margin-bottom: 12px;">
          Simulate realistic distributed failure conditions to verify split-brain fencing, recovery semantics, and HA resilience.
        </p>
        <div class="button-bar">
          <button class="btn btn-danger" id="fail-heartbeat-btn">Delay Heartbeats (3000ms)</button>
          <button class="btn" id="fail-lag-btn">Inject Replication Lag (450ms)</button>
          <button class="btn btn-danger" id="fail-partition-btn">Trigger Network Partition (Iso)</button>
          <button class="btn btn-danger" id="fail-crash-btn">Simulate Primary Node Crash</button>
        </div>
      </div>
    `;

    this.bindEvents();
  }

  private bindEvents() {
    this.querySelector("#refresh-btn")?.addEventListener("click", () => this.refresh());
    this.querySelector("#failover-btn")?.addEventListener("click", () => this.initiateFailover(false));
    this.querySelector("#force-takeover-btn")?.addEventListener("click", () => this.initiateFailover(true));
    this.querySelector("#snapshot-btn")?.addEventListener("click", () => this.triggerReplicationSnapshot());

    this.querySelector("#fail-heartbeat-btn")?.addEventListener("click", () => this.injectFailure("delay_heartbeat", 5000));
    this.querySelector("#fail-lag-btn")?.addEventListener("click", () => this.injectFailure("lag_replication", 8000));
    this.querySelector("#fail-partition-btn")?.addEventListener("click", () => this.injectFailure("network_partition", 6000));
    this.querySelector("#fail-crash-btn")?.addEventListener("click", () => this.injectFailure("node_crash", 7000));
    this.querySelector("#clear-failure-btn")?.addEventListener("click", () => this.clearInjectedFailure());
  }
}

customElements.define("nebu-cluster-panel", NebuClusterPanel);
