/**
 * NebuSafetyPanel Web Component.
 *
 * Implements Stage 9: Safety Instrumented System (SIS) & Functional Safety Architecture Workbench:
 * - Real-time SIS Executive Status Summary & Active SIF Monitoring
 * - Dynamic Cause & Effect Matrix Visualization (IEC 61511-2 / ISA-84)
 * - Sensor Channel Voting & Diagnostics Telemetry (1oo1, 1oo2, 2oo2, 2oo3, 2oo4)
 * - Administrative Bypass & Maintenance Inhibit Management with Expiration Tracking
 * - Simulated Proof Testing Runner & Historical Verification Journal
 */

import { sharedApiClient } from "../api/client";
import {
  CauseEffectMatrixDto,
  SifDetailDto,
  SisStatusSummaryDto,
} from "../protocol/types";

export class NebuSafetyPanel extends HTMLElement {
  private activeTab: "sifs" | "cause_effect" | "voting" | "bypasses" | "proof_test" = "sifs";
  private summary: SisStatusSummaryDto | null = null;
  private sifs: SifDetailDto[] = [];
  private causeEffect: CauseEffectMatrixDto | null = null;
  private selectedSifId: string | null = null;
  private isLoading = false;
  private autoRefresh = true;
  private timer: number | null = null;
  private statusMessage = "";
  private actionMessage = "";

  // Bypass form state
  private bypassReason = "Scheduled maintenance & sensor inspection";
  private bypassAuthorizedBy = "Lead Safety Engineer";
  private bypassDurationMinutes = 60;

  // Proof test form state
  private proofTechnician = "Lead Instrument Technician";

  connectedCallback() {
    this.fetchData();
    if (this.autoRefresh) {
      this.startPolling();
    }
  }

  disconnectedCallback() {
    this.stopPolling();
  }

  private startPolling() {
    this.stopPolling();
    this.timer = window.setInterval(() => {
      if (this.autoRefresh && !this.isLoading) {
        this.fetchDataQuietly();
      }
    }, 2000);
  }

  private stopPolling() {
    if (this.timer !== null) {
      window.clearInterval(this.timer);
      this.timer = null;
    }
  }

  private async fetchData() {
    this.isLoading = true;
    try {
      const [sum, sifList, ce] = await Promise.all([
        sharedApiClient.getSisSummary(),
        sharedApiClient.listSifs(),
        sharedApiClient.getCauseEffectMatrix(),
      ]);
      this.summary = sum;
      this.sifs = sifList;
      this.causeEffect = ce;
      if (!this.selectedSifId && this.sifs.length > 0) {
        this.selectedSifId = this.sifs[0].id;
      }
      this.statusMessage = `Updated at ${new Date().toLocaleTimeString()}`;
    } catch (err: any) {
      console.error("[nebu-safety-panel] Fetch error:", err);
      this.statusMessage = `Error: ${err.message || "Failed to load safety system data"}`;
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private async fetchDataQuietly() {
    try {
      const [sum, sifList, ce] = await Promise.all([
        sharedApiClient.getSisSummary(),
        sharedApiClient.listSifs(),
        sharedApiClient.getCauseEffectMatrix(),
      ]);
      this.summary = sum;
      this.sifs = sifList;
      this.causeEffect = ce;
      this.renderDynamicOnly();
    } catch (err: any) {
      console.warn("[nebu-safety-panel] Polling error:", err);
    }
  }

  private async handleReset(sifId: string) {
    this.actionMessage = `Resetting SIF ${sifId}...`;
    this.render();
    try {
      await sharedApiClient.resetSif(sifId, "Shift Supervisor");
      this.actionMessage = `SIF ${sifId} reset successfully.`;
      await this.fetchData();
    } catch (err: any) {
      this.actionMessage = `Reset rejected: ${err.message || "Process condition still active or permissive missing"}`;
      this.render();
    }
  }

  private async handleSetBypass(sifId: string, active: boolean) {
    this.actionMessage = `${active ? "Enabling" : "Clearing"} bypass on ${sifId}...`;
    this.render();
    try {
      await sharedApiClient.setSifBypass(sifId, {
        bypass_type: "Maintenance",
        authorized_by: this.bypassAuthorizedBy,
        reason: this.bypassReason,
        duration_seconds: active ? this.bypassDurationMinutes * 60 : undefined,
        active,
      });
      this.actionMessage = `Bypass ${active ? "enabled" : "cleared"} on ${sifId}.`;
      await this.fetchData();
    } catch (err: any) {
      this.actionMessage = `Bypass mutation failed: ${err.message || "Unauthorized"}`;
      this.render();
    }
  }

  private async handleRunProofTest(sifId: string) {
    this.actionMessage = `Executing proof test on ${sifId}...`;
    this.render();
    try {
      const res = await sharedApiClient.runProofTest(sifId, this.proofTechnician);
      this.actionMessage = `Proof test completed for ${sifId}: ${res.message || "PASS"}`;
      await this.fetchData();
    } catch (err: any) {
      this.actionMessage = `Proof test failed: ${err.message || "Execution error"}`;
      this.render();
    }
  }

  private renderDynamicOnly() {
    // Quick update for live state badges
    const statusBadges = this.querySelectorAll(".live-sif-state");
    statusBadges.forEach((badge) => {
      const id = badge.getAttribute("data-sif-id");
      const sif = this.sifs.find((s) => s.id === id);
      if (sif) {
        badge.textContent = sif.state;
        badge.className = `live-sif-state badge badge-${sif.state.toLowerCase()}`;
      }
    });

    // Update summary counters
    if (this.summary) {
      const armedEl = this.querySelector(".counter-armed");
      if (armedEl) armedEl.textContent = String(this.summary.armed_count);
      const trippedEl = this.querySelector(".counter-tripped");
      if (trippedEl) trippedEl.textContent = String(this.summary.tripped_count);
      const bypassedEl = this.querySelector(".counter-bypassed");
      if (bypassedEl) bypassedEl.textContent = String(this.summary.bypassed_count);
      const overridesEl = this.querySelector(".counter-overrides");
      if (overridesEl) overridesEl.textContent = String(this.summary.active_overrides_count);
    }
  }

  render() {
    this.innerHTML = `
      <style>
        .safety-container {
          display: flex;
          flex-direction: column;
          height: 100%;
          background: #0f141c;
          color: #e2e8f0;
          font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace;
        }
        .safety-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 10px 16px;
          background: #171e2e;
          border-bottom: 1px solid #2d3748;
        }
        .safety-title {
          font-size: 15px;
          font-weight: 700;
          color: #f6ad55;
          display: flex;
          align-items: center;
          gap: 8px;
        }
        .safety-disclaimer {
          font-size: 11px;
          color: #a0aec0;
          background: rgba(237, 137, 54, 0.1);
          border: 1px solid rgba(237, 137, 54, 0.3);
          padding: 3px 8px;
          border-radius: 4px;
        }
        .summary-bar {
          display: flex;
          gap: 16px;
          padding: 8px 16px;
          background: #131a26;
          border-bottom: 1px solid #232d3f;
          font-size: 12px;
        }
        .summary-card {
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 4px 10px;
          border-radius: 4px;
          background: #1c2433;
          border: 1px solid #2d3748;
        }
        .summary-num {
          font-weight: 700;
          font-size: 13px;
        }
        .tab-bar {
          display: flex;
          background: #151b27;
          border-bottom: 1px solid #2d3748;
          padding: 0 16px;
          gap: 4px;
        }
        .tab-btn {
          padding: 8px 14px;
          font-size: 12px;
          font-weight: 600;
          color: #a0aec0;
          background: transparent;
          border: none;
          border-bottom: 2px solid transparent;
          cursor: pointer;
        }
        .tab-btn:hover {
          color: #edf2f7;
        }
        .tab-btn.active {
          color: #f6ad55;
          border-bottom-color: #f6ad55;
        }
        .safety-body {
          flex: 1;
          overflow: auto;
          padding: 16px;
        }
        .badge {
          display: inline-block;
          padding: 2px 8px;
          border-radius: 10px;
          font-size: 11px;
          font-weight: 700;
        }
        .badge-armed { background: #22543d; color: #9ae6b4; }
        .badge-tripped { background: #742a2a; color: #feb2b2; animation: pulse-red 1s infinite; }
        .badge-bypassed { background: #744210; color: #fbd38d; }
        .badge-degraded { background: #5c3c10; color: #f6ad55; }
        .badge-sil { background: #2b6cb0; color: #bee3f8; }
        @keyframes pulse-red {
          0% { opacity: 1; }
          50% { opacity: 0.6; }
          100% { opacity: 1; }
        }
        table.industrial-table {
          width: 100%;
          border-collapse: collapse;
          font-size: 12px;
          background: #141b26;
          border: 1px solid #2d3748;
        }
        table.industrial-table th {
          background: #1a2233;
          color: #a0aec0;
          text-align: left;
          padding: 8px 10px;
          border-bottom: 1px solid #2d3748;
          font-weight: 600;
        }
        table.industrial-table td {
          padding: 8px 10px;
          border-bottom: 1px solid #1f2937;
        }
        table.industrial-table tr:hover {
          background: #1c2638;
        }
        .btn {
          padding: 4px 10px;
          border-radius: 4px;
          font-size: 11px;
          font-weight: 600;
          cursor: pointer;
          border: 1px solid transparent;
        }
        .btn-reset { background: #c53030; color: white; }
        .btn-reset:hover { background: #9b2c2c; }
        .btn-bypass { background: #d69e2e; color: black; }
        .btn-bypass:hover { background: #b7791f; }
        .btn-clear { background: #4a5568; color: white; }
        .btn-clear:hover { background: #2d3748; }
        .btn-proof { background: #3182ce; color: white; }
        .btn-proof:hover { background: #2b6cb0; }
        .ce-matrix-table {
          border-collapse: collapse;
          font-size: 11px;
        }
        .ce-matrix-table th, .ce-matrix-table td {
          border: 1px solid #334155;
          padding: 6px 10px;
          text-align: center;
        }
        .ce-matrix-table th.cause-hdr {
          text-align: left;
          background: #1e293b;
        }
        .ce-matrix-table th.effect-hdr {
          background: #1e293b;
          writing-mode: vertical-lr;
          transform: rotate(180deg);
          padding: 12px 6px;
        }
        .ce-cell-active {
          background: rgba(237, 137, 54, 0.2);
          color: #f6ad55;
          font-weight: bold;
        }
        .ce-cell-tripped {
          background: rgba(229, 62, 62, 0.4);
          color: #feb2b2;
          font-weight: bold;
          animation: pulse-red 1s infinite;
        }
        .action-banner {
          padding: 6px 12px;
          background: #2d3748;
          border-radius: 4px;
          margin-bottom: 12px;
          font-size: 12px;
          color: #f7fafc;
        }
      </style>

      <div class="safety-container">
        <div class="safety-header">
          <div class="safety-title">
            <span>🛡️</span>
            <span>Safety Instrumented System (SIS) & Functional Safety</span>
            <span class="badge badge-sil">IEC 61511 / ISA-84</span>
          </div>
          <div class="safety-disclaimer">
            ⚠️ Simulation & Training Model — Not Real Safety Approval
          </div>
        </div>

        <div class="summary-bar">
          <div class="summary-card">
            <span>Total SIFs:</span>
            <span class="summary-num">${this.summary?.sif_count ?? this.sifs.length}</span>
          </div>
          <div class="summary-card" style="border-left: 3px solid #48bb78;">
            <span>Armed:</span>
            <span class="summary-num counter-armed" style="color: #68d391;">${this.summary?.armed_count ?? 0}</span>
          </div>
          <div class="summary-card" style="border-left: 3px solid #f56565;">
            <span>Tripped:</span>
            <span class="summary-num counter-tripped" style="color: #fc8181;">${this.summary?.tripped_count ?? 0}</span>
          </div>
          <div class="summary-card" style="border-left: 3px solid #ecc94b;">
            <span>Bypassed:</span>
            <span class="summary-num counter-bypassed" style="color: #f6e05e;">${this.summary?.bypassed_count ?? 0}</span>
          </div>
          <div class="summary-card" style="border-left: 3px solid #e53e3e;">
            <span>Safety ESD Overrides:</span>
            <span class="summary-num counter-overrides" style="color: #feb2b2;">${this.summary?.active_overrides_count ?? 0}</span>
          </div>
          <div style="margin-left: auto; display: flex; align-items: center; gap: 8px;">
            <button class="btn btn-clear btn-refresh">🔄 Refresh</button>
            <span style="font-size: 11px; color: #718096;">${this.statusMessage}</span>
          </div>
        </div>

        <div class="tab-bar">
          <button class="tab-btn ${this.activeTab === "sifs" ? "active" : ""}" data-tab="sifs">SIF Overview</button>
          <button class="tab-btn ${this.activeTab === "cause_effect" ? "active" : ""}" data-tab="cause_effect">Cause & Effect Matrix</button>
          <button class="tab-btn ${this.activeTab === "voting" ? "active" : ""}" data-tab="voting">Voting & Telemetry</button>
          <button class="tab-btn ${this.activeTab === "bypasses" ? "active" : ""}" data-tab="bypasses">Bypass Management</button>
          <button class="tab-btn ${this.activeTab === "proof_test" ? "active" : ""}" data-tab="proof_test">Proof Testing</button>
        </div>

        <div class="safety-body">
          ${this.actionMessage ? `<div class="action-banner">${this.actionMessage}</div>` : ""}
          ${this.renderActiveTab()}
        </div>
      </div>
    `;

    this.setupListeners();
  }

  private renderActiveTab(): string {
    switch (this.activeTab) {
      case "sifs":
        return this.renderSifsTab();
      case "cause_effect":
        return this.renderCauseEffectTab();
      case "voting":
        return this.renderVotingTab();
      case "bypasses":
        return this.renderBypassesTab();
      case "proof_test":
        return this.renderProofTestTab();
    }
  }

  private renderSifsTab(): string {
    if (this.sifs.length === 0) {
      return `<div style="color: #a0aec0; text-align: center; padding: 40px;">No Safety Instrumented Functions (SIFs) configured in this project.</div>`;
    }

    return `
      <table class="industrial-table">
        <thead>
          <tr>
            <th>SIF ID</th>
            <th>Name & Scenario</th>
            <th>Target SIL</th>
            <th>Architecture</th>
            <th>Setpoint</th>
            <th>State</th>
            <th>Sensors</th>
            <th>Final Elements</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          ${this.sifs
            .map((sif) => {
              const stateClass = `badge-${sif.state.toLowerCase()}`;
              return `
                <tr>
                  <td style="font-weight: 700; color: #cbd5e0;">${sif.id}</td>
                  <td>
                    <div style="font-weight: 600;">${sif.name}</div>
                    <div style="font-size: 11px; color: #718096;">${sif.description}</div>
                  </td>
                  <td><span class="badge badge-sil">${sif.target_sil || "SIL-2"}</span></td>
                  <td><code>${sif.architecture}</code></td>
                  <td>${sif.direction} &gt; ${sif.threshold}</td>
                  <td><span class="live-sif-state badge ${stateClass}" data-sif-id="${sif.id}">${sif.state}</span></td>
                  <td>${sif.sensor_channels.length} ch (${sif.sensor_channels.map((c) => c.tag).join(", ")})</td>
                  <td>${sif.final_elements.map((fe) => `${fe.target_equipment} [${fe.safe_state}]`).join(", ")}</td>
                  <td>
                    <div style="display: flex; gap: 4px;">
                      ${
                        sif.state === "Tripped"
                          ? `<button class="btn btn-reset btn-action-reset" data-sif-id="${sif.id}">Reset Trip</button>`
                          : ""
                      }
                      ${
                        sif.state === "Bypassed"
                          ? `<button class="btn btn-clear btn-action-unbypass" data-sif-id="${sif.id}">Clear Bypass</button>`
                          : `<button class="btn btn-bypass btn-action-bypass" data-sif-id="${sif.id}">Bypass</button>`
                      }
                      <button class="btn btn-proof btn-action-proof" data-sif-id="${sif.id}">Proof Test</button>
                    </div>
                  </td>
                </tr>
              `;
            })
            .join("")}
        </tbody>
      </table>
    `;
  }

  private renderCauseEffectTab(): string {
    if (!this.causeEffect || this.causeEffect.causes.length === 0) {
      return `<div style="color: #a0aec0; text-align: center; padding: 40px;">No Cause & Effect relationships available.</div>`;
    }

    const { causes, effects, cells } = this.causeEffect;

    return `
      <div>
        <div style="margin-bottom: 12px; font-size: 12px; color: #a0aec0;">
          Dynamic Cause & Effect Matrix (ISA-84 / IEC 61511-2). Intersections with 'X' command protective isolation.
        </div>
        <div style="overflow-x: auto;">
          <table class="ce-matrix-table">
            <thead>
              <tr>
                <th class="cause-hdr" style="min-width: 260px;">Cause (Initiating Event / Hazard)</th>
                <th class="cause-hdr" style="width: 80px;">Voting</th>
                <th class="cause-hdr" style="width: 80px;">Status</th>
                ${effects
                  .map(
                    (eff) =>
                      `<th class="effect-hdr">${eff.equipment_id} (${eff.variable} &rarr; ${eff.safe_state})</th>`
                  )
                  .join("")}
              </tr>
            </thead>
            <tbody>
              ${causes
                .map((cause, cIdx) => {
                  return `
                    <tr>
                      <td class="cause-hdr">
                        <div style="font-weight: 700; color: #f6ad55;">${cause.sif_id}</div>
                        <div style="font-size: 10px; color: #94a3b8;">${cause.description}</div>
                      </td>
                      <td><code>${cause.voting}</code></td>
                      <td><span class="badge badge-${cause.status.toLowerCase()}">${cause.status}</span></td>
                      ${effects
                        .map((_, eIdx) => {
                          const cell = cells.find(
                            (cl) => cl.cause_index === cIdx && cl.effect_index === eIdx
                          );
                          if (!cell || !cell.active_link) {
                            return `<td>-</td>`;
                          }
                          const isTripped = cell.is_asserted;
                          return `
                            <td class="${isTripped ? "ce-cell-tripped" : "ce-cell-active"}">
                              ${isTripped ? "⚡ TRIP" : "X"}
                            </td>
                          `;
                        })
                        .join("")}
                    </tr>
                  `;
                })
                .join("")}
            </tbody>
          </table>
        </div>
      </div>
    `;
  }

  private renderVotingTab(): string {
    return `
      <div style="display: flex; flex-direction: column; gap: 16px;">
        <div style="display: flex; align-items: center; gap: 8px;">
          <span style="font-size: 12px; color: #a0aec0;">Select SIF to inspect:</span>
          <select class="sif-selector" style="background: #1c2433; color: white; border: 1px solid #2d3748; padding: 4px 8px; border-radius: 4px;">
            ${this.sifs
              .map(
                (s) =>
                  `<option value="${s.id}" ${s.id === this.selectedSifId ? "selected" : ""}>${s.id} — ${s.name}</option>`
              )
              .join("")}
          </select>
        </div>

        ${(() => {
          const sif = this.sifs.find((s) => s.id === this.selectedSifId) || this.sifs[0];
          if (!sif) return "<div>No SIF selected.</div>";

          return `
            <div style="display: grid; grid-template-columns: 2fr 1fr; gap: 16px;">
              <div>
                <h4 style="margin: 0 0 8px 0; font-size: 13px; color: #f6ad55;">Sensor Channels & Voting Logic (${sif.architecture})</h4>
                <table class="industrial-table">
                  <thead>
                    <tr>
                      <th>Channel</th>
                      <th>Tag</th>
                      <th>Live PV</th>
                      <th>Quality</th>
                      <th>Fault Status</th>
                      <th>Trip Condition</th>
                    </tr>
                  </thead>
                  <tbody>
                    ${sif.sensor_channels
                      .map((ch) => {
                        const isTripping =
                          sif.direction.toLowerCase() === "high"
                            ? ch.value >= sif.threshold
                            : ch.value <= sif.threshold;
                        return `
                          <tr>
                            <td><code>${ch.id}</code></td>
                            <td style="font-weight: 600;">${ch.tag}</td>
                            <td style="font-weight: 700; color: ${isTripping ? "#fc8181" : "#68d391"};">${ch.value.toFixed(2)}</td>
                            <td><span class="badge" style="background: ${ch.quality === "Good" ? "#22543d" : "#742a2a"};">${ch.quality}</span></td>
                            <td>${ch.faulted ? "⚠️ FAULTED" : "HEALTHY"}</td>
                            <td>${isTripping ? "🔴 EXCEEDED" : "🟢 OK"}</td>
                          </tr>
                        `;
                      })
                      .join("")}
                  </tbody>
                </table>
              </div>

              <div style="background: #141b26; border: 1px solid #2d3748; border-radius: 6px; padding: 12px;">
                <h4 style="margin: 0 0 8px 0; font-size: 13px; color: #90cdf4;">Diagnostics & Statistics</h4>
                <div style="font-size: 12px; display: flex; flex-direction: column; gap: 6px;">
                  <div><strong>Healthy Channels:</strong> ${sif.diagnostics.healthy_sensors} / ${sif.diagnostics.total_sensors}</div>
                  <div><strong>Faulted Channels:</strong> ${sif.diagnostics.faulted_sensors}</div>
                  <div><strong>Lifetime Trips:</strong> ${sif.diagnostics.trip_count}</div>
                  <div><strong>Last Trip Cause:</strong> ${sif.diagnostics.last_trip_cause || "None"}</div>
                  <div><strong>Degradation:</strong> <code>${sif.degradation_policy}</code></div>
                  <div><strong>Estimated PFDavg:</strong> ${sif.pfd_avg_estimate ? sif.pfd_avg_estimate.toExponential(2) : "3.50e-3"}</div>
                </div>
              </div>
            </div>
          `;
        })()}
      </div>
    `;
  }

  private renderBypassesTab(): string {
    return `
      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px;">
        <div style="background: #141b26; border: 1px solid #2d3748; border-radius: 6px; padding: 16px;">
          <h4 style="margin: 0 0 12px 0; font-size: 14px; color: #f6ad55;">Administrative Bypass Request</h4>
          <div style="font-size: 12px; display: flex; flex-direction: column; gap: 10px;">
            <div>
              <label style="display: block; margin-bottom: 4px; color: #a0aec0;">Target SIF:</label>
              <select class="bypass-sif-select" style="width: 100%; background: #1c2433; color: white; border: 1px solid #2d3748; padding: 6px; border-radius: 4px;">
                ${this.sifs.map((s) => `<option value="${s.id}">${s.id} — ${s.name} (${s.state})</option>`).join("")}
              </select>
            </div>
            <div>
              <label style="display: block; margin-bottom: 4px; color: #a0aec0;">Authorized By (Engineer / Supervisor):</label>
              <input type="text" class="bypass-authorizer-input" value="${this.bypassAuthorizedBy}" style="width: 100%; background: #1c2433; color: white; border: 1px solid #2d3748; padding: 6px; border-radius: 4px;" />
            </div>
            <div>
              <label style="display: block; margin-bottom: 4px; color: #a0aec0;">Justification Reason (Mandatory Audit):</label>
              <input type="text" class="bypass-reason-input" value="${this.bypassReason}" style="width: 100%; background: #1c2433; color: white; border: 1px solid #2d3748; padding: 6px; border-radius: 4px;" />
            </div>
            <div>
              <label style="display: block; margin-bottom: 4px; color: #a0aec0;">Duration Limit (Minutes):</label>
              <input type="number" class="bypass-duration-input" value="${this.bypassDurationMinutes}" style="width: 100%; background: #1c2433; color: white; border: 1px solid #2d3748; padding: 6px; border-radius: 4px;" />
            </div>
            <div style="display: flex; gap: 8px; margin-top: 6px;">
              <button class="btn btn-bypass btn-submit-bypass" style="flex: 1; padding: 8px;">Apply Administrative Bypass</button>
              <button class="btn btn-clear btn-submit-unbypass" style="flex: 1; padding: 8px;">Clear / Restore SIF</button>
            </div>
          </div>
        </div>

        <div style="background: #141b26; border: 1px solid #2d3748; border-radius: 6px; padding: 16px;">
          <h4 style="margin: 0 0 12px 0; font-size: 14px; color: #e2e8f0;">Active Bypasses & Inhibits</h4>
          ${(() => {
            const bypassed = this.sifs.filter((s) => s.state === "Bypassed");
            if (bypassed.length === 0) {
              return `<div style="color: #718096; font-size: 12px;">No functions are currently bypassed. All SIFs active.</div>`;
            }
            return bypassed
              .map(
                (s) => `
              <div style="background: #1c2433; border-left: 3px solid #ecc94b; padding: 10px; border-radius: 4px; margin-bottom: 8px; font-size: 12px;">
                <div style="font-weight: 700; color: #f6ad55;">${s.id} — ${s.name}</div>
                <div>Reason: ${s.bypass?.reason || "Maintenance"}</div>
                <div>Authorized by: ${s.bypass?.authorized_by || "Unknown"}</div>
                <div style="margin-top: 6px;">
                  <button class="btn btn-clear btn-action-unbypass" data-sif-id="${s.id}">Remove Bypass</button>
                </div>
              </div>
            `
              )
              .join("");
          })()}
        </div>
      </div>
    `;
  }

  private renderProofTestTab(): string {
    return `
      <div style="display: flex; flex-direction: column; gap: 16px;">
        <div style="background: #141b26; border: 1px solid #2d3748; border-radius: 6px; padding: 16px;">
          <h4 style="margin: 0 0 12px 0; font-size: 14px; color: #63b3ed;">Simulated Proof Test Execution (IEC 61511-1 Clause 16)</h4>
          <div style="display: flex; gap: 12px; align-items: flex-end; font-size: 12px;">
            <div style="flex: 1;">
              <label style="display: block; margin-bottom: 4px; color: #a0aec0;">Select SIF to Test:</label>
              <select class="proof-sif-select" style="width: 100%; background: #1c2433; color: white; border: 1px solid #2d3748; padding: 6px; border-radius: 4px;">
                ${this.sifs.map((s) => `<option value="${s.id}">${s.id} — ${s.name}</option>`).join("")}
              </select>
            </div>
            <div style="flex: 1;">
              <label style="display: block; margin-bottom: 4px; color: #a0aec0;">Technician / Inspector Name:</label>
              <input type="text" class="proof-technician-input" value="${this.proofTechnician}" style="width: 100%; background: #1c2433; color: white; border: 1px solid #2d3748; padding: 6px; border-radius: 4px;" />
            </div>
            <button class="btn btn-proof btn-submit-proof-test" style="padding: 8px 16px;">Execute Automated Proof Test</button>
          </div>
        </div>

        <div style="background: #141b26; border: 1px solid #2d3748; border-radius: 6px; padding: 16px;">
          <h4 style="margin: 0 0 8px 0; font-size: 13px; color: #e2e8f0;">Proof Test Protocol Steps Verified</h4>
          <div style="font-size: 12px; color: #a0aec0; display: flex; flex-direction: column; gap: 6px;">
            <div>1. Sensor Channel Diagnostics: Check 4-20mA calibration and NAMUR NE43 thresholds.</div>
            <div>2. Voting Logic Solver: Verify truth table execution under simulated out-of-spec demand.</div>
            <div>3. Logic Solver Trip Assert: Verify ESD Priority 1 command generation.</div>
            <div>4. Final Control Element Travel: Stroke valve to safe position (0.0% closed) within stroke time.</div>
            <div>5. Reset & Permissive Interlock: Verify latched state and requirement for permissive clearance.</div>
          </div>
        </div>
      </div>
    `;
  }

  private setupListeners() {
    // Tab switching
    this.querySelectorAll(".tab-btn").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        const target = (e.currentTarget as HTMLElement).getAttribute("data-tab");
        if (target) {
          this.activeTab = target as any;
          this.render();
        }
      });
    });

    // Refresh button
    const refreshBtn = this.querySelector(".btn-refresh");
    if (refreshBtn) {
      refreshBtn.addEventListener("click", () => this.fetchData());
    }

    // SIF Selector
    const selector = this.querySelector(".sif-selector") as HTMLSelectElement;
    if (selector) {
      selector.addEventListener("change", () => {
        this.selectedSifId = selector.value;
        this.render();
      });
    }

    // Reset buttons
    this.querySelectorAll(".btn-action-reset").forEach((btn) => {
      btn.addEventListener("click", () => {
        const sifId = btn.getAttribute("data-sif-id");
        if (sifId) this.handleReset(sifId);
      });
    });

    // Bypass buttons
    this.querySelectorAll(".btn-action-bypass").forEach((btn) => {
      btn.addEventListener("click", () => {
        const sifId = btn.getAttribute("data-sif-id");
        if (sifId) this.handleSetBypass(sifId, true);
      });
    });

    this.querySelectorAll(".btn-action-unbypass").forEach((btn) => {
      btn.addEventListener("click", () => {
        const sifId = btn.getAttribute("data-sif-id");
        if (sifId) this.handleSetBypass(sifId, false);
      });
    });

    // Proof test buttons
    this.querySelectorAll(".btn-action-proof").forEach((btn) => {
      btn.addEventListener("click", () => {
        const sifId = btn.getAttribute("data-sif-id");
        if (sifId) this.handleRunProofTest(sifId);
      });
    });

    // Submit bypass form
    const submitBypass = this.querySelector(".btn-submit-bypass");
    if (submitBypass) {
      submitBypass.addEventListener("click", () => {
        const select = this.querySelector(".bypass-sif-select") as HTMLSelectElement;
        const authorizer = this.querySelector(".bypass-authorizer-input") as HTMLInputElement;
        const reason = this.querySelector(".bypass-reason-input") as HTMLInputElement;
        const duration = this.querySelector(".bypass-duration-input") as HTMLInputElement;
        if (select) {
          this.bypassAuthorizedBy = authorizer?.value || this.bypassAuthorizedBy;
          this.bypassReason = reason?.value || this.bypassReason;
          this.bypassDurationMinutes = parseInt(duration?.value || "60", 10);
          this.handleSetBypass(select.value, true);
        }
      });
    }

    const submitUnbypass = this.querySelector(".btn-submit-unbypass");
    if (submitUnbypass) {
      submitUnbypass.addEventListener("click", () => {
        const select = this.querySelector(".bypass-sif-select") as HTMLSelectElement;
        if (select) {
          this.handleSetBypass(select.value, false);
        }
      });
    }

    // Submit proof test form
    const submitProof = this.querySelector(".btn-submit-proof-test");
    if (submitProof) {
      submitProof.addEventListener("click", () => {
        const select = this.querySelector(".proof-sif-select") as HTMLSelectElement;
        const tech = this.querySelector(".proof-technician-input") as HTMLInputElement;
        if (select) {
          this.proofTechnician = tech?.value || this.proofTechnician;
          this.handleRunProofTest(select.value);
        }
      });
    }
  }
}

customElements.define("nebu-safety-panel", NebuSafetyPanel);
