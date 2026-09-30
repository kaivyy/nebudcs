/**
 * NebuDCS I/O Hardware Abstraction Layer (HAL) Panel.
 *
 * Implements Stage 3 I/O HAL inspection and diagnostic controls:
 * - Hardware hierarchy navigation (Rack -> Chassis -> Module -> Channel)
 * - Power supply and backplane communication bus health telemetry
 * - Live engineering values, raw ADC/DAC counts, and signal quality flags
 * - Standard NAMUR NE43 analog fault state monitoring (WireBreak, ShortCircuit, UnderRange, OverRange)
 * - Output fallback mode visualization (FailOpen, FailClosed, HoldLast, ForcedValue, Zero)
 * - Controller I/O image (%I / %Q) mapping table with direction and conflict indicators
 * - Fault injection & forced override testing (enforcing RBAC authorization)
 */

import { ApiClient, sharedApiClient } from "../api/client";
import {
  HalChannelDto,
  HalSystemSummaryDto,
} from "../protocol/types";
import { authStore } from "../state/auth-store";

export class NebuIoPanel extends HTMLElement {
  public apiClient: ApiClient = sharedApiClient;
  private systems: HalSystemSummaryDto[] = [];
  private selectedSystemId: string | null = null;
  private selectedModuleKey: string | null = null; // "rack-chassis-module"
  private refreshInterval: number | null = null;
  private isLoading = false;
  private errorMessage: string | null = null;
  private activeTab: "channels" | "mappings" = "channels";

  connectedCallback() {
    this.render();
    this.fetchData();
    this.refreshInterval = window.setInterval(() => {
      this.fetchData(true);
    }, 2000);
  }

  disconnectedCallback() {
    if (this.refreshInterval !== null) {
      clearInterval(this.refreshInterval);
      this.refreshInterval = null;
    }
  }

  public async fetchData(silent = false) {
    if (this.isLoading && !silent) return;
    if (!silent) {
      this.isLoading = true;
    }

    try {
      const list = await this.apiClient.listHalSystems();
      this.systems = Array.isArray(list) ? list : [];
      this.errorMessage = null;

      if (!this.selectedSystemId && this.systems.length > 0) {
        this.selectedSystemId = this.systems[0]?.id || null;
      }
    } catch (err: any) {
      if (!silent) {
        this.errorMessage = err?.message || "Failed to load I/O HAL systems";
      }
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private getSelectedSystem(): HalSystemSummaryDto | null {
    if (!this.selectedSystemId && this.systems.length > 0) {
      return this.systems[0] || null;
    }
    return this.systems.find((s) => s.id === this.selectedSystemId) || null;
  }

  private getAllChannels(system: HalSystemSummaryDto): HalChannelDto[] {
    const channels: HalChannelDto[] = [];
    for (const rack of system.racks || []) {
      for (const chassis of rack.chassis || []) {
        for (const module of chassis.modules || []) {
          for (const ch of module.channels || []) {
            channels.push(ch);
          }
        }
      }
    }
    return channels;
  }

  private getFilteredChannels(system: HalSystemSummaryDto): HalChannelDto[] {
    if (!this.selectedModuleKey) {
      return this.getAllChannels(system);
    }
    const [rackNum, chassisNum, modSlot] = this.selectedModuleKey.split("-").map(Number);
    for (const rack of system.racks || []) {
      if (rack.rack_number !== rackNum) continue;
      for (const chassis of rack.chassis || []) {
        if (chassis.chassis_number !== chassisNum) continue;
        for (const module of chassis.modules || []) {
          if (module.slot === modSlot) {
            return module.channels || [];
          }
        }
      }
    }
    return this.getAllChannels(system);
  }

  private async handleInjectFault(address: string, fault: string, forcedVal?: number) {
    try {
      this.isLoading = true;
      this.errorMessage = null;
      await this.apiClient.setChannelFault(address, {
        fault,
        forced_value: forcedVal,
      });
      await this.fetchData(true);
    } catch (err: any) {
      this.errorMessage = err?.message || `Failed to apply fault on ${address}`;
      this.render();
    } finally {
      this.isLoading = false;
    }
  }

  private async handleClearFault(address: string) {
    try {
      this.isLoading = true;
      this.errorMessage = null;
      await this.apiClient.setChannelFault(address, {
        fault: "clear",
        forced_value: undefined,
      });
      await this.fetchData(true);
    } catch (err: any) {
      this.errorMessage = err?.message || `Failed to clear fault on ${address}`;
      this.render();
    } finally {
      this.isLoading = false;
    }
  }

  render() {
    const role = authStore.getRole();
    const canInject = role === "admin" || role === "instructor";
    const system = this.getSelectedSystem();

    const diagnostics = system?.diagnostics || {
      channels_scanned: 0,
      faulted_channels: 0,
      last_scan_duration_us: 0,
      max_scan_duration_us: 0,
      avg_scan_duration_us: 0,
    };

    const channels = system ? this.getFilteredChannels(system) : [];
    const mappings = system?.mappings || [];

    this.innerHTML = `
      <style>
        nebu-io-panel {
          display: flex;
          flex-direction: column;
          height: 100%;
          background: var(--bg-primary, #1e1e24);
          color: var(--text-primary, #e0e0e0);
          font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace;
          font-size: 13px;
          overflow: hidden;
        }

        .io-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 10px 16px;
          background: var(--bg-secondary, #282830);
          border-bottom: 1px solid var(--border-color, #3a3a46);
        }

        .io-title {
          font-weight: 600;
          font-size: 15px;
          display: flex;
          align-items: center;
          gap: 8px;
        }

        .io-metrics {
          display: flex;
          gap: 16px;
        }

        .metric-item {
          display: flex;
          flex-direction: column;
          align-items: flex-end;
        }

        .metric-label {
          font-size: 10px;
          text-transform: uppercase;
          color: var(--text-muted, #8e8e9e);
        }

        .metric-value {
          font-weight: 700;
          font-family: monospace;
          color: var(--accent-cyan, #4ecdc4);
        }

        .metric-value.faulted {
          color: #ff6b6b;
        }

        .io-tabs {
          display: flex;
          background: var(--bg-secondary, #282830);
          border-bottom: 1px solid var(--border-color, #3a3a46);
          padding: 0 16px;
          gap: 4px;
        }

        .io-tab {
          padding: 8px 14px;
          cursor: pointer;
          border-bottom: 2px solid transparent;
          color: var(--text-muted, #8e8e9e);
          font-weight: 500;
          background: none;
          border: none;
        }

        .io-tab.active {
          color: var(--text-primary, #ffffff);
          border-bottom: 2px solid var(--accent-cyan, #4ecdc4);
          background: rgba(78, 205, 196, 0.08);
        }

        .io-body {
          display: flex;
          flex: 1;
          overflow: hidden;
        }

        .io-sidebar {
          width: 260px;
          background: var(--bg-secondary, #24242c);
          border-right: 1px solid var(--border-color, #3a3a46);
          overflow-y: auto;
          padding: 12px;
        }

        .io-content {
          flex: 1;
          overflow-y: auto;
          padding: 16px;
        }

        .tree-node {
          margin-bottom: 8px;
        }

        .rack-header, .chassis-header, .module-item {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 6px 8px;
          border-radius: 4px;
          cursor: pointer;
          user-select: none;
        }

        .rack-header {
          background: rgba(255, 255, 255, 0.04);
          font-weight: 600;
          margin-bottom: 4px;
        }

        .chassis-header {
          padding-left: 16px;
          font-weight: 500;
          color: #b0b0c0;
        }

        .module-item {
          padding-left: 28px;
          font-size: 12px;
          transition: background 0.15s;
        }

        .module-item:hover {
          background: rgba(255, 255, 255, 0.06);
        }

        .module-item.selected {
          background: rgba(78, 205, 196, 0.15);
          color: var(--accent-cyan, #4ecdc4);
          font-weight: 600;
        }

        .badge {
          font-size: 10px;
          padding: 2px 6px;
          border-radius: 3px;
          font-weight: 600;
          text-transform: uppercase;
        }

        .badge-good, .badge-normal, .badge-healthy {
          background: rgba(46, 204, 113, 0.2);
          color: #2ecc71;
          border: 1px solid rgba(46, 204, 113, 0.4);
        }

        .badge-uncertain, .badge-warning {
          background: rgba(241, 196, 15, 0.2);
          color: #f1c40f;
          border: 1px solid rgba(241, 196, 15, 0.4);
        }

        .badge-bad, .badge-faulted, .badge-danger {
          background: rgba(231, 76, 60, 0.2);
          color: #e74c3c;
          border: 1px solid rgba(231, 76, 60, 0.4);
        }

        .badge-frozen {
          background: rgba(155, 89, 182, 0.2);
          color: #9b59b6;
          border: 1px solid rgba(155, 89, 182, 0.4);
        }

        .badge-disabled {
          background: rgba(127, 140, 141, 0.2);
          color: #7f8c8d;
          border: 1px solid rgba(127, 140, 141, 0.4);
        }

        table.io-table {
          width: 100%;
          border-collapse: collapse;
          text-align: left;
        }

        table.io-table th {
          background: var(--bg-secondary, #282830);
          color: var(--text-muted, #8e8e9e);
          font-size: 11px;
          text-transform: uppercase;
          padding: 8px 10px;
          border-bottom: 1px solid var(--border-color, #3a3a46);
        }

        table.io-table td {
          padding: 8px 10px;
          border-bottom: 1px solid rgba(255, 255, 255, 0.05);
          font-family: monospace;
          vertical-align: middle;
        }

        table.io-table tr:hover td {
          background: rgba(255, 255, 255, 0.02);
        }

        .btn {
          background: var(--bg-secondary, #3a3a46);
          color: var(--text-primary, #ffffff);
          border: 1px solid rgba(255, 255, 255, 0.15);
          border-radius: 4px;
          padding: 4px 10px;
          font-size: 11px;
          cursor: pointer;
          transition: all 0.15s;
        }

        .btn:hover:not(:disabled) {
          background: var(--accent-cyan, #4ecdc4);
          color: #000;
        }

        .btn:disabled {
          opacity: 0.4;
          cursor: not-allowed;
        }

        .btn-danger {
          background: rgba(231, 76, 60, 0.2);
          color: #e74c3c;
          border-color: rgba(231, 76, 60, 0.4);
        }

        .btn-danger:hover:not(:disabled) {
          background: #e74c3c;
          color: #fff;
        }

        .fault-controls {
          display: flex;
          gap: 6px;
          align-items: center;
        }

        .alert-bar {
          background: rgba(231, 76, 60, 0.15);
          color: #e74c3c;
          border: 1px solid rgba(231, 76, 60, 0.3);
          padding: 8px 12px;
          border-radius: 4px;
          margin-bottom: 12px;
          font-size: 12px;
        }
      </style>

      <div class="io-header">
        <div class="io-title">
          <span>⚡</span>
          <span>I/O Hardware Abstraction Layer (HAL)</span>
          ${system ? `<span style="font-size: 12px; color: #8e8e9e;">[${system.name}]</span>` : ""}
        </div>
        <div class="io-metrics">
          <div class="metric-item">
            <span class="metric-label">Channels</span>
            <span class="metric-value">${diagnostics.channels_scanned}</span>
          </div>
          <div class="metric-item">
            <span class="metric-label">Faulted</span>
            <span class="metric-value ${diagnostics.faulted_channels > 0 ? "faulted" : ""}">
              ${diagnostics.faulted_channels}
            </span>
          </div>
          <div class="metric-item">
            <span class="metric-label">Scan (µs)</span>
            <span class="metric-value">${diagnostics.last_scan_duration_us}</span>
          </div>
          <div class="metric-item">
            <span class="metric-label">Max Scan</span>
            <span class="metric-value">${diagnostics.max_scan_duration_us} µs</span>
          </div>
          <button id="btn-refresh" class="btn">Refresh</button>
        </div>
      </div>

      <div class="io-tabs">
        <button class="io-tab ${this.activeTab === "channels" ? "active" : ""}" id="tab-channels">
          Channels & Hardware (${channels.length})
        </button>
        <button class="io-tab ${this.activeTab === "mappings" ? "active" : ""}" id="tab-mappings">
          Controller Mappings (${mappings.length})
        </button>
      </div>

      ${this.errorMessage ? `<div class="alert-bar">${this.errorMessage}</div>` : ""}

      <div class="io-body">
        <div class="io-sidebar">
          <div style="font-size: 11px; font-weight: 600; text-transform: uppercase; color: #8e8e9e; margin-bottom: 8px;">
            Hardware Topology
          </div>
          <div class="module-item ${this.selectedModuleKey === null ? "selected" : ""}" id="mod-all">
            <span>All Modules</span>
            <span class="badge badge-normal">${system ? this.getAllChannels(system).length : 0}</span>
          </div>

          ${(system?.racks || [])
            .map(
              (rack) => `
              <div class="tree-node">
                <div class="rack-header">
                  <span>Rack ${rack.rack_number}: ${rack.name}</span>
                  <span class="badge ${rack.power_ok ? "badge-good" : "badge-bad"}">
                    ${rack.power_ok ? "PWR OK" : "PWR FAULT"}
                  </span>
                </div>
                ${(rack.chassis || [])
                  .map(
                    (chassis) => `
                    <div class="chassis-header">
                      <span>Chassis ${chassis.chassis_number}: ${chassis.name}</span>
                      <span class="badge ${chassis.bus_ok ? "badge-good" : "badge-bad"}">
                        ${chassis.bus_ok ? "BUS OK" : "BUS FAULT"}
                      </span>
                    </div>
                    ${(chassis.modules || [])
                      .map((module) => {
                        const key = `${rack.rack_number}-${chassis.chassis_number}-${module.slot}`;
                        const isSelected = this.selectedModuleKey === key;
                        const isHealthy = module.status === "Healthy";
                        return `
                          <div class="module-item ${isSelected ? "selected" : ""}" data-modkey="${key}">
                            <span>Slot ${module.slot}: ${module.name} (${module.module_type})</span>
                            <span class="badge ${isHealthy ? "badge-good" : "badge-bad"}">
                              ${module.channels.length} ch
                            </span>
                          </div>
                        `;
                      })
                      .join("")}
                  `
                  )
                  .join("")}
              </div>
            `
            )
            .join("")}
        </div>

        <div class="io-content">
          ${
            this.activeTab === "channels"
              ? `
            <table class="io-table">
              <thead>
                <tr>
                  <th>Address</th>
                  <th>Name</th>
                  <th>Type</th>
                  <th>Engineering Value</th>
                  <th>Raw Counts</th>
                  <th>Quality</th>
                  <th>NAMUR / Fault</th>
                  <th>Fallback</th>
                  <th>Testing Actions</th>
                </tr>
              </thead>
              <tbody>
                ${
                  channels.length === 0
                    ? `<tr><td colspan="9" style="text-align: center; color: #8e8e9e; padding: 24px;">No channels found</td></tr>`
                    : channels
                        .map((ch) => {
                          const isNormal = ch.fault_state === "Normal";
                          const isWireBreak = ch.fault_state === "WireBreak";
                          const isShortCircuit = ch.fault_state === "ShortCircuit";
                          const isOverUnder =
                            ch.fault_state === "UnderRange" || ch.fault_state === "OverRange";

                          let faultBadgeClass = "badge-normal";
                          if (isWireBreak || isShortCircuit) faultBadgeClass = "badge-bad";
                          else if (isOverUnder) faultBadgeClass = "badge-uncertain";
                          else if (ch.fault_state === "Frozen") faultBadgeClass = "badge-frozen";

                          let qualBadgeClass = "badge-good";
                          if (ch.quality === "Bad") qualBadgeClass = "badge-bad";
                          else if (ch.quality === "Uncertain") qualBadgeClass = "badge-uncertain";
                          else if (ch.quality === "Disabled") qualBadgeClass = "badge-disabled";

                          const isDigital = ch.kind === "DI" || ch.kind === "DO";
                          const displayVal = isDigital
                            ? ch.value >= 0.5
                              ? "TRUE [1]"
                              : "FALSE [0]"
                            : `${ch.value.toFixed(2)} ${ch.engineering_unit}`;

                          return `
                            <tr>
                              <td style="font-weight: 600; color: #4ecdc4;">${ch.address}</td>
                              <td>${ch.name}</td>
                              <td><span class="badge badge-normal">${ch.kind}</span></td>
                              <td style="font-weight: 700;">${displayVal}</td>
                              <td style="color: #b0b0c0;">${ch.raw_counts}</td>
                              <td><span class="badge ${qualBadgeClass}">${ch.quality}</span></td>
                              <td><span class="badge ${faultBadgeClass}">${ch.fault_state}</span></td>
                              <td style="color: #8e8e9e; font-size: 11px;">${ch.fallback_behavior || "—"}</td>
                              <td>
                                <div class="fault-controls">
                                  ${
                                    !isNormal
                                      ? `<button class="btn btn-clear-fault" data-addr="${ch.address}" ${!canInject ? "disabled title='Admin/Engineer required'" : ""}>Clear</button>`
                                      : `
                                        <button class="btn btn-fault-wb" data-addr="${ch.address}" ${!canInject ? "disabled title='Admin/Engineer required'" : ""}>WireBreak</button>
                                        <button class="btn btn-fault-sc" data-addr="${ch.address}" ${!canInject ? "disabled title='Admin/Engineer required'" : ""}>Short</button>
                                      `
                                  }
                                </div>
                              </td>
                            </tr>
                          `;
                        })
                        .join("")
                }
              </tbody>
            </table>
          `
              : `
            <table class="io-table">
              <thead>
                <tr>
                  <th>Controller ID</th>
                  <th>Direction</th>
                  <th>Signal Key</th>
                  <th>I/O Channel Address</th>
                  <th>Bound Target (Tag / Actuator)</th>
                  <th>Priority</th>
                </tr>
              </thead>
              <tbody>
                ${
                  mappings.length === 0
                    ? `<tr><td colspan="6" style="text-align: center; color: #8e8e9e; padding: 24px;">No controller mappings configured</td></tr>`
                    : mappings
                        .map(
                          (m) => `
                          <tr>
                            <td style="color: #8e8e9e;">${m.controller_id}</td>
                            <td>
                              <span class="badge ${m.direction === "Input" ? "badge-good" : "badge-uncertain"}">
                                ${m.direction}
                              </span>
                            </td>
                            <td style="font-weight: 600; color: #4ecdc4;">${m.signal_key}</td>
                            <td style="font-family: monospace; font-weight: 600;">${m.channel_address}</td>
                            <td>${m.target_tag || m.target_actuator || "—"}</td>
                            <td>${m.priority !== undefined ? m.priority : "Default"}</td>
                          </tr>
                        `
                        )
                        .join("")
                }
              </tbody>
            </table>
          `
          }
        </div>
      </div>
    `;

    this.attachEvents();
  }

  private attachEvents() {
    this.querySelector("#btn-refresh")?.addEventListener("click", () => {
      this.fetchData();
    });

    this.querySelector("#tab-channels")?.addEventListener("click", () => {
      this.activeTab = "channels";
      this.render();
    });

    this.querySelector("#tab-mappings")?.addEventListener("click", () => {
      this.activeTab = "mappings";
      this.render();
    });

    this.querySelector("#mod-all")?.addEventListener("click", () => {
      this.selectedModuleKey = null;
      this.render();
    });

    this.querySelectorAll(".module-item[data-modkey]").forEach((el) => {
      el.addEventListener("click", () => {
        const key = el.getAttribute("data-modkey");
        this.selectedModuleKey = key;
        this.render();
      });
    });

    this.querySelectorAll(".btn-clear-fault").forEach((el) => {
      el.addEventListener("click", () => {
        const addr = el.getAttribute("data-addr");
        if (addr) this.handleClearFault(addr);
      });
    });

    this.querySelectorAll(".btn-fault-wb").forEach((el) => {
      el.addEventListener("click", () => {
        const addr = el.getAttribute("data-addr");
        if (addr) this.handleInjectFault(addr, "wirebreak");
      });
    });

    this.querySelectorAll(".btn-fault-sc").forEach((el) => {
      el.addEventListener("click", () => {
        const addr = el.getAttribute("data-addr");
        if (addr) this.handleInjectFault(addr, "shortcircuit");
      });
    });
  }
}

customElements.define("nebu-io-panel", NebuIoPanel);
