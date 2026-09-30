/**
 * Virtual Controller Runtime Diagnostics & Management Panel.
 *
 * Implements Stage 1 & Stage 8 Virtual PLC/DCS Controller Architecture:
 * - 7-state controller lifecycle management (PowerOff, Booting, Initializing, Running, Paused, Faulted, ShuttingDown)
 * - Hardware Profiles: Virtual CPU budget, Memory allocations (Program/Data/Retentive), Firmware profile
 * - DCS Control Modules: MAN / AUTO / CAS modes, Bumpless transfer, SP / PV / OUT tracking, Permissives & Safety Interlocks
 * - High-Availability Redundancy: Primary / Standby role synchronization, heartbeat timeout, manual switchover
 * - Controlled Fault Injection Workbench: Watchdog timeout, Task overrun, CPU overload, Memory exhaustion, Power loss
 * - Deterministic scan cycle telemetry and IEC 61131-3 logic editor integration
 */

import { ApiClient, sharedApiClient } from "../api/client";
import { ControllerDetailDto, ControllerSummaryDto, DcsControlModuleDto } from "../protocol/types";
import { authStore } from "../state/auth-store";
import "./nebu-iec-editor";

type ControllerTab = "overview" | "profiles" | "modules" | "redundancy" | "faults";

export class NebuControllerPanel extends HTMLElement {
  public apiClient: ApiClient = sharedApiClient;
  private controllers: ControllerSummaryDto[] = [];
  private selectedControllerId: string | null = null;
  private selectedDetail: ControllerDetailDto | null = null;
  private activeTab: ControllerTab = "overview";
  private refreshInterval: number | null = null;
  private isLoading = false;
  private errorMessage: string | null = null;

  connectedCallback() {
    this.render();
    this.fetchControllers();
    this.refreshInterval = window.setInterval(() => {
      this.fetchControllers(true);
    }, 2000);
  }

  disconnectedCallback() {
    if (this.refreshInterval !== null) {
      clearInterval(this.refreshInterval);
      this.refreshInterval = null;
    }
  }

  public async fetchControllers(silent = false) {
    if (this.isLoading && !silent) return;
    if (!silent) {
      this.isLoading = true;
    }

    try {
      const list = await this.apiClient.listControllers();
      this.controllers = Array.isArray(list) ? list : [];
      this.errorMessage = null;

      if (!this.selectedControllerId && this.controllers.length > 0) {
        this.selectedControllerId = this.controllers[0]?.id || null;
      }

      if (this.selectedControllerId) {
        try {
          this.selectedDetail = await this.apiClient.getController(this.selectedControllerId);
        } catch {
          this.selectedDetail = null;
        }
      } else {
        this.selectedDetail = null;
      }
    } catch (err: any) {
      if (!silent) {
        this.errorMessage = err?.message || "Failed to load controllers";
      }
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private async handleAction(action: string) {
    if (!this.selectedControllerId) return;
    try {
      this.isLoading = true;
      this.errorMessage = null;
      this.selectedDetail = await this.apiClient.updateControllerState(this.selectedControllerId, action);
      await this.fetchControllers(true);
    } catch (err: any) {
      this.errorMessage = err?.message || `Action '${action}' failed`;
      this.render();
    } finally {
      this.isLoading = false;
    }
  }

  private async handleModuleMode(moduleId: string, mode: string) {
    if (!this.selectedControllerId) return;
    try {
      this.isLoading = true;
      this.errorMessage = null;
      await this.apiClient.updateControlModule(this.selectedControllerId, moduleId, { mode });
      await this.fetchControllers(true);
    } catch (err: any) {
      this.errorMessage = err?.message || `Mode change failed: ${err}`;
      this.render();
    } finally {
      this.isLoading = false;
    }
  }

  private async handleModuleSp(moduleId: string, sp: number) {
    if (!this.selectedControllerId) return;
    try {
      await this.apiClient.updateControlModule(this.selectedControllerId, moduleId, { sp });
      await this.fetchControllers(true);
    } catch (err: any) {
      this.errorMessage = err?.message || `SP update failed: ${err}`;
      this.render();
    }
  }

  private async handleModuleManualOut(moduleId: string, manual_out: number) {
    if (!this.selectedControllerId) return;
    try {
      await this.apiClient.updateControlModule(this.selectedControllerId, moduleId, { manual_out });
      await this.fetchControllers(true);
    } catch (err: any) {
      this.errorMessage = err?.message || `Manual output update failed: ${err}`;
      this.render();
    }
  }

  private async handleInjectFault(fault_type: string) {
    if (!this.selectedControllerId) return;
    try {
      this.isLoading = true;
      this.errorMessage = null;
      await this.apiClient.injectControllerFault(this.selectedControllerId, {
        fault_type,
        description: `Controlled fault test: ${fault_type}`,
        recoverable: fault_type !== "memory_exhaustion",
      });
      await this.fetchControllers(true);
    } catch (err: any) {
      this.errorMessage = err?.message || `Fault injection failed: ${err}`;
      this.render();
    } finally {
      this.isLoading = false;
    }
  }

  private async handleSwitchover() {
    if (!this.selectedControllerId) return;
    try {
      this.isLoading = true;
      this.errorMessage = null;
      await this.apiClient.triggerRedundancySwitchover(this.selectedControllerId);
      await this.fetchControllers(true);
    } catch (err: any) {
      this.errorMessage = err?.message || `Redundancy switchover failed: ${err}`;
      this.render();
    } finally {
      this.isLoading = false;
    }
  }

  render() {
    const role = authStore.getRole();
    const canControl = role === "admin" || role === "instructor" || role === "operator";

    const controllers = Array.isArray(this.controllers) ? this.controllers : [];
    const detail = this.selectedDetail;
    const tasks = Array.isArray(detail?.tasks) ? detail!.tasks : [];
    const modules: DcsControlModuleDto[] = Array.isArray(detail?.control_modules) ? detail!.control_modules : [];
    const cpu = detail?.cpu_profile;
    const mem = detail?.memory_profile;
    const fw = detail?.firmware_profile;
    const red = detail?.redundancy;

    const lifecycleClass = detail?.lifecycle === "running"
      ? "badge-success"
      : detail?.lifecycle === "faulted"
      ? "badge-danger"
      : detail?.lifecycle === "paused"
      ? "badge-warning"
      : detail?.lifecycle === "poweroff"
      ? "badge-neutral"
      : "badge-info";

    this.innerHTML = `
      <div class="controller-panel-card">
        <div class="panel-header">
          <div class="panel-title-group">
            <span class="panel-icon">🎮</span>
            <span class="panel-title">Virtual Controller Runtime & Architecture</span>
            <span class="badge badge-success">STAGE 8</span>
          </div>
          <div class="panel-actions">
            <button class="btn-refresh btn-tool-xs" title="Refresh Diagnostics">🔄 Refresh</button>
          </div>
        </div>

        ${this.errorMessage ? `<div class="alert-box alert-danger mono text-xs">${this.errorMessage}</div>` : ""}

        <div class="controller-selector-row">
          <label class="text-dim text-xs mono">CONTROLLER:</label>
          <select class="sel-controller mono text-sm" ${controllers.length === 0 ? "disabled" : ""}>
            ${controllers.length === 0 ? `<option value="">(No active controllers)</option>` : ""}
            ${controllers
              .map(
                (c) =>
                  `<option value="${c.id}" ${c.id === this.selectedControllerId ? "selected" : ""}>
                    ${c.name} [${c.lifecycle.toUpperCase()}]
                  </option>`
              )
              .join("")}
          </select>
        </div>

        ${
          detail
            ? `
          <div class="controller-status-bar">
            <div class="status-badge-group">
              <span class="badge ${lifecycleClass} mono text-xs">${detail.lifecycle.toUpperCase()}</span>
              <span class="text-dim text-xs mono">ID: ${detail.id}</span>
              ${red ? `<span class="badge badge-info mono text-xs">${red.role.toUpperCase()} (${red.status})</span>` : ""}
            </div>
            ${
              canControl
                ? `
              <div class="lifecycle-controls">
                <button class="btn-ctrl btn-play btn-tool-xs" data-action="start" title="Start / Run" ${detail.lifecycle === "running" ? "disabled" : ""}>▶ Start</button>
                <button class="btn-ctrl btn-pause btn-tool-xs" data-action="pause" title="Pause Scan" ${detail.lifecycle !== "running" ? "disabled" : ""}>⏸ Pause</button>
                <button class="btn-ctrl btn-stop btn-tool-xs" data-action="stop" title="Stop Controller" ${detail.lifecycle === "stopped" ? "disabled" : ""}>⏹ Stop</button>
                <button class="btn-ctrl btn-reset btn-tool-xs" data-action="reset" title="Reset Controller & Faults">⟲ Reset</button>
                <button class="btn-ctrl btn-tool-xs" data-action="boot" title="Boot Controller" ${detail.lifecycle !== "poweroff" ? "disabled" : ""}>⚡ Boot</button>
                <button class="btn-ctrl btn-tool-xs btn-danger" data-action="power_off" title="Cut Power">🔌 Power Off</button>
              </div>
            `
                : `<span class="text-dim text-xs mono" title="Engineer/Admin role required">(Read-Only)</span>`
            }
          </div>

          ${
            detail.fault
              ? `
            <div class="alert-box alert-danger mono text-xs" style="display: flex; justify-content: space-between; align-items: center;">
              <div>
                <strong>FAULT DETECTED:</strong> [${detail.fault.code}] ${detail.fault.message}
                (Sim: ${detail.fault.timestamp_sim.toFixed(2)}s, Recoverable: ${detail.fault.recoverable})
              </div>
              ${canControl && detail.fault.recoverable ? `<button class="btn-tool-xs btn-fault-clear">Clear Fault</button>` : ""}
            </div>
          `
              : ""
          }

          <!-- Stage 8 Sub-Navigation Tabs -->
          <div class="panel-tabs-bar" style="display: flex; gap: 6px; border-bottom: 1px solid #30363d; margin: 10px 0 12px 0;">
            <button class="tab-btn btn-tool-xs ${this.activeTab === "overview" ? "btn-primary" : ""}" data-tab="overview">Overview & Tasks</button>
            <button class="tab-btn btn-tool-xs ${this.activeTab === "profiles" ? "btn-primary" : ""}" data-tab="profiles">Hardware Profiles</button>
            <button class="tab-btn btn-tool-xs ${this.activeTab === "modules" ? "btn-primary" : ""}" data-tab="modules">Control Modules (${modules.length})</button>
            <button class="tab-btn btn-tool-xs ${this.activeTab === "redundancy" ? "btn-primary" : ""}" data-tab="redundancy">Redundancy & HA</button>
            <button class="tab-btn btn-tool-xs ${this.activeTab === "faults" ? "btn-primary" : ""}" data-tab="faults">Fault Injection</button>
          </div>

          <!-- TAB 1: OVERVIEW & TASKS -->
          ${
            this.activeTab === "overview"
              ? `
            <div class="telemetry-grid">
              <div class="telemetry-stat">
                <div class="stat-label text-dim text-xs mono">SCANS</div>
                <div class="stat-value mono">${detail.scan_count}</div>
              </div>
              <div class="telemetry-stat">
                <div class="stat-label text-dim text-xs mono">AVG CYCLE</div>
                <div class="stat-value mono">${detail.avg_scan_duration_us.toFixed(1)} µs</div>
              </div>
              <div class="telemetry-stat">
                <div class="stat-label text-dim text-xs mono">MAX CYCLE</div>
                <div class="stat-value mono">${detail.max_scan_duration_us} µs</div>
              </div>
              <div class="telemetry-stat">
                <div class="stat-label text-dim text-xs mono">WATCHDOG</div>
                <div class="stat-value mono ${detail.watchdog_trips > 0 ? "text-danger" : ""}">${detail.watchdog_trips} trips</div>
              </div>
              <div class="telemetry-stat">
                <div class="stat-label text-dim text-xs mono">OVERRUNS</div>
                <div class="stat-value mono ${detail.overrun_count > 0 ? "text-warning" : ""}">${detail.overrun_count}</div>
              </div>
              <div class="telemetry-stat">
                <div class="stat-label text-dim text-xs mono">I/O IMAGE</div>
                <div class="stat-value mono">%I: ${detail.input_points.length} | %Q: ${detail.output_points.length}</div>
              </div>
            </div>

            <div class="tasks-table-section">
              <div class="section-subtitle text-xs mono text-dim">SCHEDULED TASKS (${tasks.length})</div>
              <div class="table-scroll-wrapper">
                <table class="tasks-table mono text-xs">
                  <thead>
                    <tr>
                      <th>TASK</th>
                      <th>TYPE</th>
                      <th>PERIOD</th>
                      <th>PRIO</th>
                      <th>EXECS</th>
                      <th>OVERRUNS</th>
                      <th>LAST DURATION</th>
                    </tr>
                  </thead>
                  <tbody>
                    ${
                      tasks.length === 0
                        ? `<tr><td colspan="7" class="text-center text-dim">(No tasks configured)</td></tr>`
                        : tasks
                            .map(
                              (t) => `
                          <tr class="${t.overrun_count > 0 ? "row-warn" : ""}">
                            <td><strong>${t.name}</strong></td>
                            <td>${t.task_type}</td>
                            <td>${t.period_seconds !== null && t.period_seconds !== undefined ? `${(t.period_seconds * 1000).toFixed(0)} ms` : "Continuous"}</td>
                            <td>${t.priority}</td>
                            <td>${t.execution_count}</td>
                            <td>${t.overrun_count}</td>
                            <td>${t.last_duration_us} µs</td>
                          </tr>
                        `
                            )
                            .join("")
                    }
                  </tbody>
                </table>
              </div>

              <div class="iec-workbench-section" style="margin-top: 16px; border-top: 1px solid #30363d; padding-top: 12px;">
                <nebu-iec-editor></nebu-iec-editor>
              </div>
            </div>
          `
              : ""
          }

          <!-- TAB 2: HARDWARE PROFILES -->
          ${
            this.activeTab === "profiles"
              ? `
            <div class="profiles-container" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 12px;">
              <div class="card" style="background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 12px;">
                <div class="text-xs mono text-primary" style="font-weight: bold; margin-bottom: 8px;">VIRTUAL CPU PROFILE</div>
                <div class="mono text-xs" style="line-height: 1.8;">
                  <div><span class="text-dim">Architecture:</span> <strong>${cpu?.architecture || "VendorNeutral-VirtualVCPU-64"}</strong></div>
                  <div><span class="text-dim">Execution Budget:</span> <strong>${cpu?.execution_budget_us || 50000} µs</strong></div>
                  <div><span class="text-dim">Max Scan Capacity:</span> <strong>${cpu?.max_scan_frequency_hz || 50} Hz</strong></div>
                  <div><span class="text-dim">Warning Threshold:</span> <strong>${cpu?.utilization_threshold_pct || 80}%</strong></div>
                  <div><span class="text-dim">Current Last Scan:</span> <strong>${detail.last_scan_duration_us} µs</strong></div>
                </div>
              </div>

              <div class="card" style="background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 12px;">
                <div class="text-xs mono text-primary" style="font-weight: bold; margin-bottom: 8px;">VIRTUAL MEMORY PROFILE</div>
                <div class="mono text-xs" style="line-height: 1.8;">
                  <div><span class="text-dim">Program Memory:</span> <strong>${((mem?.program_memory_bytes || 16777216) / (1024 * 1024)).toFixed(1)} MB</strong></div>
                  <div><span class="text-dim">Data Memory (RAM):</span> <strong>${((mem?.data_memory_bytes || 33554432) / (1024 * 1024)).toFixed(1)} MB</strong></div>
                  <div><span class="text-dim">Retentive Memory (NVRAM):</span> <strong>${((mem?.retentive_memory_bytes || 4194304) / (1024 * 1024)).toFixed(1)} MB</strong></div>
                  <div class="text-dim text-xs" style="margin-top: 4px; color: #3fb950;">✓ NVRAM survives controller reboot and power interruption</div>
                </div>
              </div>

              <div class="card" style="background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 12px;">
                <div class="text-xs mono text-primary" style="font-weight: bold; margin-bottom: 8px;">FIRMWARE PROFILE</div>
                <div class="mono text-xs" style="line-height: 1.8;">
                  <div><span class="text-dim">Firmware Version:</span> <strong>${fw?.version || "v8.1.0"}</strong></div>
                  <div><span class="text-dim">Build Tag:</span> <strong>${fw?.build || "2026.09-LTS"}</strong></div>
                  <div><span class="text-dim">IEC Standard:</span> <strong>${fw?.iec_standard || "IEC 61131-3:2013 Ed.3"}</strong></div>
                  <div><span class="text-dim">Causality Invariant:</span> <strong>Single-threaded 50 Hz sole authority</strong></div>
                </div>
              </div>
            </div>
          `
              : ""
          }

          <!-- TAB 3: DCS CONTROL MODULES -->
          ${
            this.activeTab === "modules"
              ? `
            <div class="modules-container">
              ${
                modules.length === 0
                  ? `<div class="empty-state mono text-xs text-dim" style="padding: 20px; text-align: center;">No DCS Control Modules configured on this controller.</div>`
                  : modules
                      .map(
                        (cm) => `
                    <div class="card module-card" style="background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 14px; margin-bottom: 10px;">
                      <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
                        <div>
                          <strong class="mono text-sm">${cm.name}</strong>
                          <span class="mono text-xs text-dim">(${cm.id})</span>
                          <span class="badge badge-neutral mono text-xs" style="margin-left: 6px;">${cm.strategy_type}</span>
                        </div>
                        <div class="mode-selector-group" style="display: flex; gap: 4px;">
                          <button class="btn-tool-xs btn-mode ${cm.mode === "MAN" ? "btn-primary" : ""}" data-mod="${cm.id}" data-mode="MAN">MAN</button>
                          <button class="btn-tool-xs btn-mode ${cm.mode === "AUTO" ? "btn-success" : ""}" data-mod="${cm.id}" data-mode="AUTO" ${!cm.permissives_ok ? "disabled title='Permissives not satisfied'" : ""}>AUTO</button>
                          <button class="btn-tool-xs btn-mode ${cm.mode === "CAS" ? "btn-info" : ""}" data-mod="${cm.id}" data-mode="CAS" ${!cm.permissives_ok ? "disabled title='Permissives not satisfied'" : ""}>CAS</button>
                        </div>
                      </div>

                      <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 12px; margin-bottom: 8px;">
                        <div>
                          <div class="mono text-xs text-dim">PV: ${cm.pv_signal}</div>
                          <div class="mono text-lg" style="font-weight: bold; color: #58a6ff;">${cm.current_pv.toFixed(2)}</div>
                        </div>

                        <div>
                          <div class="mono text-xs text-dim">SP: Setpoint</div>
                          <div style="display: flex; align-items: center; gap: 6px;">
                            <input type="number" class="mono text-sm input-sp" data-mod="${cm.id}" value="${cm.sp.toFixed(1)}" step="1.0" style="width: 80px; background: #0d1117; border: 1px solid #30363d; color: #fff; padding: 2px 6px; border-radius: 4px;" ${cm.mode !== "AUTO" ? "disabled" : ""} />
                            <span class="mono text-xs text-dim">SP Target</span>
                          </div>
                        </div>

                        <div>
                          <div class="mono text-xs text-dim">OUT: Demand (${cm.out_signal})</div>
                          <div style="display: flex; align-items: center; gap: 6px;">
                            <input type="range" class="range-out" data-mod="${cm.id}" min="${cm.output_min}" max="${cm.output_max}" step="0.5" value="${cm.current_out.toFixed(1)}" ${cm.mode !== "MAN" ? "disabled" : ""} style="flex: 1;" />
                            <strong class="mono text-sm" style="color: #3fb950; min-width: 45px;">${cm.current_out.toFixed(1)}%</strong>
                          </div>
                        </div>
                      </div>

                      <div style="display: flex; gap: 12px; align-items: center; padding-top: 6px; border-top: 1px solid #21262d;" class="mono text-xs">
                        <div>
                          <span class="text-dim">Permissives:</span>
                          <span class="badge ${cm.permissives_ok ? "badge-success" : "badge-danger"}">${cm.permissives_ok ? "OK" : "INHIBITED"}</span>
                          ${cm.permissives.length > 0 ? `<span class="text-dim">(${cm.permissives.join(", ")})</span>` : ""}
                        </div>

                        <div>
                          <span class="text-dim">Interlock:</span>
                          <span class="badge ${cm.is_interlocked ? "badge-danger" : "badge-success"}">${cm.is_interlocked ? "TRIPPED" : "CLEAR"}</span>
                          ${cm.is_interlocked ? `<span class="text-danger">(Clamped to Safe Pos: ${cm.safe_position}%)</span>` : ""}
                        </div>

                        ${cm.tracking_active ? `<span class="badge badge-warning">TRACKING ACTIVE</span>` : ""}
                        <div class="text-dim" style="margin-left: auto;">Kp=${cm.kp} Ti=${cm.ti}s</div>
                      </div>
                    </div>
                  `
                      )
                      .join("")
              }
            </div>
          `
              : ""
          }

          <!-- TAB 4: REDUNDANCY & HIGH AVAILABILITY -->
          ${
            this.activeTab === "redundancy"
              ? `
            <div class="card" style="background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 16px;">
              <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;">
                <div>
                  <strong class="mono text-sm">HIGH-AVAILABILITY CONTROLLER REDUNDANCY</strong>
                  <div class="mono text-xs text-dim">1-out-of-2 (1oo2) Bumpless Warm/Hot Standby Replication</div>
                </div>
                ${
                  canControl
                    ? `<button class="btn-tool-xs btn-primary btn-switchover" title="Initiate Controlled Redundancy Switchover">🔄 Trigger Switchover</button>`
                    : ""
                }
              </div>

              <div class="telemetry-grid">
                <div class="telemetry-stat">
                  <div class="stat-label text-dim text-xs mono">ROLE</div>
                  <div class="stat-value mono ${red?.role === "Primary" ? "text-success" : "text-info"}">${red?.role.toUpperCase() || "STANDALONE"}</div>
                </div>
                <div class="telemetry-stat">
                  <div class="stat-label text-dim text-xs mono">STATUS</div>
                  <div class="stat-value mono">${red?.status || "Synchronized"}</div>
                </div>
                <div class="telemetry-stat">
                  <div class="stat-label text-dim text-xs mono">PARTNER CONTROLLER</div>
                  <div class="stat-value mono">${red?.partner_id || "None"}</div>
                </div>
                <div class="telemetry-stat">
                  <div class="stat-label text-dim text-xs mono">SYNC FREQUENCY</div>
                  <div class="stat-value mono">${red?.sync_frequency_hz || 20} Hz</div>
                </div>
                <div class="telemetry-stat">
                  <div class="stat-label text-dim text-xs mono">SWITCHOVER TIMEOUT</div>
                  <div class="stat-value mono">${((red?.switchover_timeout_seconds || 0.1) * 1000).toFixed(0)} ms</div>
                </div>
                <div class="telemetry-stat">
                  <div class="stat-label text-dim text-xs mono">SYNC CYCLES</div>
                  <div class="stat-value mono">${red?.sync_cycle_count || 0}</div>
                </div>
              </div>

              <div class="mono text-xs text-dim" style="margin-top: 14px; background: #0d1117; padding: 10px; border-radius: 4px; border-left: 3px solid #58a6ff;">
                <strong>Replication Mechanics:</strong> In hot standby mode, internal memory images, retentive storage variables, and active PID accumulators are serialized and synchronized over the virtual backplane bus each scan cycle without interrupting process physics.
              </div>
            </div>
          `
              : ""
          }

          <!-- TAB 5: CONTROLLED FAULT INJECTION -->
          ${
            this.activeTab === "faults"
              ? `
            <div class="card" style="background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 16px;">
              <div class="mono text-sm" style="font-weight: bold; margin-bottom: 4px;">CONTROLLED FAULT INJECTION WORKBENCH</div>
              <div class="mono text-xs text-dim" style="margin-bottom: 14px;">Inject deterministic controller hardware, software, or memory failure states to test system safety and DCS fail-over.</div>

              <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 10px;">
                <button class="btn-tool-xs btn-fault btn-danger mono" data-fault="watchdog" ${!canControl ? "disabled" : ""}>
                  ⚠️ Watchdog Timeout (1.0s)
                </button>
                <button class="btn-tool-xs btn-fault btn-danger mono" data-fault="overrun" ${!canControl ? "disabled" : ""}>
                  ⏱️ Scan Task Overrun
                </button>
                <button class="btn-tool-xs btn-fault btn-danger mono" data-fault="cpu_overload" ${!canControl ? "disabled" : ""}>
                  🔥 CPU Overload (>98%)
                </button>
                <button class="btn-tool-xs btn-fault btn-danger mono" data-fault="memory_exhaustion" ${!canControl ? "disabled" : ""}>
                  💾 Memory Exhaustion (Fatal)
                </button>
                <button class="btn-tool-xs btn-fault btn-warning mono" data-fault="power_loss" ${!canControl ? "disabled" : ""}>
                  🔌 Power Loss / Outage
                </button>
                <button class="btn-tool-xs btn-success mono btn-fault-clear" ${!canControl ? "disabled" : ""}>
                  ✅ Clear Recoverable Fault
                </button>
              </div>
            </div>
          `
              : ""
          }
        `
            : `
          <div class="empty-state text-dim mono text-xs">
            ${controllers.length === 0 ? "No active virtual controllers in current plant." : "Select a controller above to inspect telemetry."}
          </div>
        `
        }
      </div>
    `;

    this.bindEvents();
  }

  private bindEvents() {
    const sel = this.querySelector(".sel-controller") as HTMLSelectElement | null;
    if (sel) {
      sel.addEventListener("change", (e) => {
        const val = (e.target as HTMLSelectElement).value;
        this.selectedControllerId = val || null;
        this.fetchControllers(false);
      });
    }

    const refreshBtn = this.querySelector(".btn-refresh");
    if (refreshBtn) {
      refreshBtn.addEventListener("click", () => this.fetchControllers(false));
    }

    // Tabs
    const tabBtns = this.querySelectorAll(".tab-btn");
    tabBtns.forEach((btn) => {
      btn.addEventListener("click", (e) => {
        const tab = (e.currentTarget as HTMLElement).getAttribute("data-tab") as ControllerTab;
        if (tab) {
          this.activeTab = tab;
          this.render();
        }
      });
    });

    // Lifecycle actions
    const ctrlBtns = this.querySelectorAll(".btn-ctrl");
    ctrlBtns.forEach((btn) => {
      btn.addEventListener("click", (e) => {
        const action = (e.currentTarget as HTMLElement).getAttribute("data-action");
        if (action) {
          this.handleAction(action);
        }
      });
    });

    // Control Module Mode
    const modeBtns = this.querySelectorAll(".btn-mode");
    modeBtns.forEach((btn) => {
      btn.addEventListener("click", (e) => {
        const target = e.currentTarget as HTMLElement;
        const modId = target.getAttribute("data-mod");
        const mode = target.getAttribute("data-mode");
        if (modId && mode) {
          this.handleModuleMode(modId, mode);
        }
      });
    });

    // Control Module SP
    const spInputs = this.querySelectorAll(".input-sp");
    spInputs.forEach((input) => {
      input.addEventListener("change", (e) => {
        const target = e.target as HTMLInputElement;
        const modId = target.getAttribute("data-mod");
        const val = parseFloat(target.value);
        if (modId && !isNaN(val)) {
          this.handleModuleSp(modId, val);
        }
      });
    });

    // Control Module Manual Output
    const outRanges = this.querySelectorAll(".range-out");
    outRanges.forEach((range) => {
      range.addEventListener("change", (e) => {
        const target = e.target as HTMLInputElement;
        const modId = target.getAttribute("data-mod");
        const val = parseFloat(target.value);
        if (modId && !isNaN(val)) {
          this.handleModuleManualOut(modId, val);
        }
      });
    });

    // Fault injection
    const faultBtns = this.querySelectorAll(".btn-fault");
    faultBtns.forEach((btn) => {
      btn.addEventListener("click", (e) => {
        const fault = (e.currentTarget as HTMLElement).getAttribute("data-fault");
        if (fault) {
          this.handleInjectFault(fault);
        }
      });
    });

    // Clear Fault
    const clearFaultBtns = this.querySelectorAll(".btn-fault-clear");
    clearFaultBtns.forEach((btn) => {
      btn.addEventListener("click", () => {
        this.handleInjectFault("clear");
      });
    });

    // Redundancy Switchover
    const switchoverBtn = this.querySelector(".btn-switchover");
    if (switchoverBtn) {
      switchoverBtn.addEventListener("click", () => {
        this.handleSwitchover();
      });
    }
  }
}

if (!customElements.get("nebu-controller-panel")) {
  customElements.define("nebu-controller-panel", NebuControllerPanel);
}
