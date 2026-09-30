/**
 * Unified Global Diagnostics View Web Component.
 *
 * Implements Stage 14: Enterprise, High Availability & Distributed Runtime.
 * Correlates faults and diagnostic advisories across all 10 NebuDCS subsystems:
 * - System, Sim, Control, I/O, Protocol, SIS, Historian, OTS, Validation, Cluster
 * - Clickable links to entity IDs (tags, controllers, channels, nodes, SIFs)
 * - Suggested corrective operator/engineer actions
 */

import { client } from "../api/client";
import { DiagnosticItemDto, DiagnosticsSummaryDto } from "../protocol/types";

export class NebuDiagnosticsView extends HTMLElement {
  private items: DiagnosticItemDto[] = [];
  private summary: DiagnosticsSummaryDto | null = null;
  private selectedDomain = "";
  private selectedSeverity = "";
  private isLoading = false;
  private statusMessage: string | null = null;

  async connectedCallback() {
    await this.fetchDiagnostics();
  }

  private async fetchDiagnostics() {
    this.isLoading = true;
    this.render();
    try {
      const [list, sum] = await Promise.all([
        client.getDiagnostics({
          domain: this.selectedDomain || undefined,
          severity: this.selectedSeverity || undefined,
        }),
        client.getDiagnosticsSummary(),
      ]);
      this.items = list;
      this.summary = sum;
    } catch (err: unknown) {
      this.statusMessage = `Failed to fetch diagnostics: ${err instanceof Error ? err.message : String(err)}`;
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private getSeverityBadge(sev: string): string {
    switch (sev.toUpperCase()) {
      case "CRITICAL": return "badge-crit";
      case "ERROR": return "badge-err";
      case "WARNING": return "badge-warn";
      default: return "badge-info";
    }
  }

  render() {
    this.innerHTML = `
      <style>
        nebu-diagnostics-view {
          display: block;
          padding: 16px;
          color: #e2e8f0;
          background: #131822;
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
          grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
          gap: 12px;
          margin-bottom: 16px;
        }
        .kpi-card {
          background: #1a2232;
          border: 1px solid #28354a;
          border-radius: 6px;
          padding: 10px 12px;
        }
        .kpi-title {
          font-size: 0.72rem;
          color: #8292a8;
          text-transform: uppercase;
          margin-bottom: 4px;
        }
        .kpi-value {
          font-size: 1.3rem;
          font-weight: bold;
        }
        .filter-bar {
          display: flex;
          flex-wrap: wrap;
          gap: 10px;
          align-items: center;
          background: #171f2e;
          border: 1px solid #263346;
          border-radius: 6px;
          padding: 12px;
          margin-bottom: 16px;
        }
        .filter-group {
          display: flex;
          align-items: center;
          gap: 8px;
        }
        .filter-group label {
          font-size: 0.8rem;
          color: #94a3b8;
        }
        .select-input {
          background: #0f141d;
          border: 1px solid #35455e;
          color: #fff;
          padding: 6px 10px;
          border-radius: 4px;
          font-size: 0.85rem;
        }
        .btn {
          padding: 6px 14px;
          border: 1px solid #35455e;
          background: #1f2a3c;
          color: #e2e8f0;
          border-radius: 4px;
          cursor: pointer;
          font-size: 0.85rem;
        }
        .btn:hover { background: #28374f; }
        .badge {
          display: inline-block;
          padding: 2px 6px;
          border-radius: 3px;
          font-size: 0.72rem;
          font-weight: bold;
          text-transform: uppercase;
        }
        .badge-crit { background: #5a0b14; color: #ff8585; border: 1px solid #991b1b; }
        .badge-err { background: #4a151b; color: #f87171; border: 1px solid #7f1d1d; }
        .badge-warn { background: #523b08; color: #fbbf24; border: 1px solid #78350f; }
        .badge-info { background: #0c3852; color: #38bdf8; border: 1px solid #0369a1; }
        .item-card {
          background: #171f2e;
          border: 1px solid #263346;
          border-radius: 6px;
          padding: 14px;
          margin-bottom: 12px;
        }
        .item-card.crit { border-left: 4px solid #ef4444; }
        .item-card.warn { border-left: 4px solid #f59e0b; }
        .item-card.info { border-left: 4px solid #0ea5e9; }
        .item-head {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 6px;
        }
        .item-title {
          font-weight: 600;
          font-size: 0.95rem;
          color: #fff;
        }
        .item-domain {
          color: #94a3b8;
          font-size: 0.8rem;
          font-weight: 500;
        }
        .item-desc {
          font-size: 0.85rem;
          color: #cbd5e1;
          margin-bottom: 8px;
        }
        .item-footer {
          display: flex;
          flex-wrap: wrap;
          justify-content: space-between;
          align-items: center;
          font-size: 0.8rem;
          color: #8292a8;
          border-top: 1px solid #232c3d;
          padding-top: 8px;
          margin-top: 6px;
        }
        .entity-tag {
          font-family: monospace;
          background: #0f141d;
          padding: 2px 6px;
          border-radius: 3px;
          color: #60a5fa;
          cursor: pointer;
        }
        .action-text {
          color: #34d399;
          font-weight: 500;
        }
      </style>

      <div class="header">
        <div class="title-area">
          <h2>
            <span>🔍</span> Global Subsystem Diagnostics & Issue Correlation
          </h2>
          <div class="subtitle">Unified Cross-Subsystem Telemetry, Health Status & Remediation Guidance</div>
        </div>
        <div>
          <button class="btn" id="refresh-diag-btn" ${this.isLoading ? "disabled" : ""}>↻ Refresh</button>
        </div>
      </div>

      ${this.statusMessage ? `
        <div style="padding: 10px; background: #451217; color: #f87171; border-radius: 4px; margin-bottom: 14px;">
          ${this.statusMessage}
        </div>
      ` : ""}

      <div class="kpi-grid">
        <div class="kpi-card">
          <div class="kpi-title">Total Active</div>
          <div class="kpi-value" style="color: #60a5fa;">${this.summary?.total_active ?? this.items.length}</div>
        </div>
        <div class="kpi-card">
          <div class="kpi-title">Critical</div>
          <div class="kpi-value" style="color: #ef4444;">${this.summary?.critical_count ?? 0}</div>
        </div>
        <div class="kpi-card">
          <div class="kpi-title">Errors</div>
          <div class="kpi-value" style="color: #f87171;">${this.summary?.error_count ?? 0}</div>
        </div>
        <div class="kpi-card">
          <div class="kpi-title">Warnings</div>
          <div class="kpi-value" style="color: #fbbf24;">${this.summary?.warning_count ?? 0}</div>
        </div>
        <div class="kpi-card">
          <div class="kpi-title">Info / Advisory</div>
          <div class="kpi-value" style="color: #38bdf8;">${this.summary?.info_count ?? 0}</div>
        </div>
      </div>

      <div class="filter-bar">
        <div class="filter-group">
          <label>Domain:</label>
          <select class="select-input" id="diag-domain">
            <option value="">All Domains</option>
            <option value="Cluster" ${this.selectedDomain === "Cluster" ? "selected" : ""}>Cluster & HA</option>
            <option value="Simulation" ${this.selectedDomain === "Simulation" ? "selected" : ""}>Simulation Core</option>
            <option value="Control" ${this.selectedDomain === "Control" ? "selected" : ""}>Control System</option>
            <option value="IO" ${this.selectedDomain === "IO" ? "selected" : ""}>I/O HAL</option>
            <option value="Communications" ${this.selectedDomain === "Communications" ? "selected" : ""}>Industrial Protocol</option>
            <option value="Safety" ${this.selectedDomain === "Safety" ? "selected" : ""}>SIS Functional Safety</option>
            <option value="Historian" ${this.selectedDomain === "Historian" ? "selected" : ""}>Historian</option>
            <option value="Validation" ${this.selectedDomain === "Validation" ? "selected" : ""}>Scientific Validation</option>
          </select>
        </div>
        <div class="filter-group">
          <label>Severity:</label>
          <select class="select-input" id="diag-sev">
            <option value="">All Severities</option>
            <option value="CRITICAL" ${this.selectedSeverity === "CRITICAL" ? "selected" : ""}>CRITICAL</option>
            <option value="ERROR" ${this.selectedSeverity === "ERROR" ? "selected" : ""}>ERROR</option>
            <option value="WARNING" ${this.selectedSeverity === "WARNING" ? "selected" : ""}>WARNING</option>
            <option value="INFO" ${this.selectedSeverity === "INFO" ? "selected" : ""}>INFO</option>
          </select>
        </div>
        <button class="btn" id="apply-diag-filter">Filter</button>
      </div>

      <div class="items-list">
        ${this.items.length === 0 ? `
          <div style="text-align: center; padding: 32px; color: #94a3b8; background: #171f2e; border-radius: 6px;">
            ✓ All subsystems operating nominally. Zero active diagnostics in selected scope.
          </div>
        ` : this.items.map(item => `
          <div class="item-card ${item.severity === "CRITICAL" ? "crit" : item.severity === "WARNING" ? "warn" : "info"}">
            <div class="item-head">
              <div style="display: flex; align-items: center; gap: 8px;">
                <span class="badge ${this.getSeverityBadge(item.severity)}">${item.severity}</span>
                <span class="item-title">${item.title}</span>
              </div>
              <span class="item-domain">[${item.domain}]</span>
            </div>
            <div class="item-desc">${item.description}</div>
            <div class="item-footer">
              <div>
                ${item.entity_id ? `
                  <span>Entity (${item.entity_type || "Object"}): <span class="entity-tag" title="Click to navigate" data-entity="${item.entity_id}">${item.entity_id}</span></span>
                ` : ""}
              </div>
              <div>
                <span class="action-text">Action:</span> ${item.suggested_action}
              </div>
            </div>
          </div>
        `).join("")}
      </div>
    `;

    this.bindEvents();
  }

  private bindEvents() {
    this.querySelector("#refresh-diag-btn")?.addEventListener("click", () => this.fetchDiagnostics());
    this.querySelector("#apply-diag-filter")?.addEventListener("click", () => {
      const dom = this.querySelector("#diag-domain") as HTMLSelectElement;
      const sev = this.querySelector("#diag-sev") as HTMLSelectElement;
      this.selectedDomain = dom.value;
      this.selectedSeverity = sev.value;
      this.fetchDiagnostics();
    });

    this.querySelectorAll(".entity-tag").forEach(el => {
      el.addEventListener("click", (e) => {
        const entityId = (e.currentTarget as HTMLElement).getAttribute("data-entity");
        if (entityId) {
          window.dispatchEvent(new CustomEvent("nebu-navigate-entity", {
            detail: { entityId },
            bubbles: true,
            composed: true,
          }));
        }
      });
    });
  }
}

customElements.define("nebu-diagnostics-view", NebuDiagnosticsView);
