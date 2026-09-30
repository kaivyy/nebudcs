/**
 * Enterprise Audit Trail & Compliance Journal Web Component.
 *
 * Implements Stage 14: Enterprise, High Availability & Distributed Runtime.
 * Features:
 * - Realtime and historical audit event inspection
 * - Multi-role filtering (Admin, Engineer, Operator, Instructor, Trainee)
 * - Action and entity search (setpoint changes, SIS bypasses, trips, failover, backup/restore)
 * - JSON compliance journal export
 */

import { client } from "../api/client";
import { AuditEventDto, AuditSummaryDto } from "../protocol/types";

export class NebuAuditPanel extends HTMLElement {
  private events: AuditEventDto[] = [];
  private summary: AuditSummaryDto | null = null;
  private selectedRole = "";
  private selectedAction = "";
  private searchActor = "";
  private isLoading = false;
  private statusMessage: string | null = null;

  async connectedCallback() {
    await this.fetchData();
  }

  private async fetchData() {
    this.isLoading = true;
    this.render();
    try {
      const [evList, sum] = await Promise.all([
        client.getAuditEvents({
          role: this.selectedRole || undefined,
          action: this.selectedAction || undefined,
          actor: this.searchActor || undefined,
          limit: 100,
        }),
        client.getAuditSummary(),
      ]);
      this.events = evList;
      this.summary = sum;
    } catch (err: unknown) {
      this.statusMessage = `Failed to load audit events: ${err instanceof Error ? err.message : String(err)}`;
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private exportJson() {
    const dataStr = JSON.stringify(this.events, null, 2);
    const blob = new Blob([dataStr], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `nebudcs-audit-journal-${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  render() {
    this.innerHTML = `
      <style>
        nebu-audit-panel {
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
          margin-bottom: 6px;
        }
        .kpi-value {
          font-size: 1.4rem;
          font-weight: bold;
          color: #fff;
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
        .select-input, .text-input {
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
        .btn-primary { background: #2563eb; border-color: #3b82f6; color: white; }
        .table-container {
          background: #171f2e;
          border: 1px solid #263346;
          border-radius: 6px;
          overflow-x: auto;
        }
        table {
          width: 100%;
          border-collapse: collapse;
          font-size: 0.85rem;
          text-align: left;
        }
        th {
          background: #1c2637;
          color: #94a3b8;
          font-weight: 600;
          padding: 10px 12px;
          border-bottom: 1px solid #28354a;
        }
        td {
          padding: 8px 12px;
          border-bottom: 1px solid #1f2937;
          vertical-align: middle;
        }
        tr:hover { background: #1b2434; }
        .badge {
          display: inline-block;
          padding: 2px 6px;
          border-radius: 3px;
          font-size: 0.75rem;
          font-weight: bold;
        }
        .role-admin { background: #4a151b; color: #f87171; }
        .role-engineer { background: #523b08; color: #fbbf24; }
        .role-operator { background: #0c3852; color: #38bdf8; }
        .role-instructor { background: #3b1d5c; color: #c084fc; }
        .role-trainee { background: #133926; color: #4ade80; }
        .role-system { background: #262c3a; color: #94a3b8; }
        .mono { font-family: monospace; font-size: 0.8rem; }
      </style>

      <div class="header">
        <div class="title-area">
          <h2>
            <span>📜</span> Unified Enterprise Audit Log & Compliance Journal
          </h2>
          <div class="subtitle">Complete Tamper-Resistant Regulatory & Operational Action Logging</div>
        </div>
        <div>
          <button class="btn" id="export-json-btn">📥 Export Audit Journal (JSON)</button>
        </div>
      </div>

      ${this.statusMessage ? `
        <div style="padding: 10px; background: #451217; color: #f87171; border-radius: 4px; margin-bottom: 14px;">
          ${this.statusMessage}
        </div>
      ` : ""}

      <div class="kpi-grid">
        <div class="kpi-card">
          <div class="kpi-title">Total Logged Events</div>
          <div class="kpi-value">${this.summary?.total_events_recorded ?? this.events.length}</div>
        </div>
        <div class="kpi-card">
          <div class="kpi-title">Buffer Capacity</div>
          <div class="kpi-value">${this.summary?.current_buffer_size ?? 5000}</div>
        </div>
        <div class="kpi-card">
          <div class="kpi-title">Unique Actors</div>
          <div class="kpi-value mono" style="font-size: 1.1rem; color: #60a5fa;">
            ${this.summary?.unique_actors_count ?? 1}
          </div>
        </div>
        <div class="kpi-card">
          <div class="kpi-title">Latest Timestamp</div>
          <div class="kpi-value mono" style="font-size: 0.95rem; color: #34d399;">
            ${this.summary?.latest_event_timestamp?.slice(0, 19) || "N/A"}
          </div>
        </div>
      </div>

      <div class="filter-bar">
        <div class="filter-group">
          <label>Role:</label>
          <select class="select-input" id="filter-role">
            <option value="">All Roles</option>
            <option value="Admin" ${this.selectedRole === "Admin" ? "selected" : ""}>Admin</option>
            <option value="Engineer" ${this.selectedRole === "Engineer" ? "selected" : ""}>Engineer</option>
            <option value="Operator" ${this.selectedRole === "Operator" ? "selected" : ""}>Operator</option>
            <option value="Instructor" ${this.selectedRole === "Instructor" ? "selected" : ""}>Instructor</option>
            <option value="Trainee" ${this.selectedRole === "Trainee" ? "selected" : ""}>Trainee</option>
            <option value="System" ${this.selectedRole === "System" ? "selected" : ""}>System</option>
          </select>
        </div>
        <div class="filter-group">
          <label>Action:</label>
          <input type="text" class="text-input" id="filter-action" placeholder="e.g. Setpoint, Failover, Trip" value="${this.selectedAction}">
        </div>
        <div class="filter-group">
          <label>Actor:</label>
          <input type="text" class="text-input" id="filter-actor" placeholder="Filter by username" value="${this.searchActor}">
        </div>
        <button class="btn btn-primary" id="apply-filters-btn" ${this.isLoading ? "disabled" : ""}>Filter</button>
        <button class="btn" id="clear-filters-btn">Reset</button>
      </div>

      <div class="table-container">
        <table>
          <thead>
            <tr>
              <th>Timestamp</th>
              <th>Actor</th>
              <th>Role</th>
              <th>Action</th>
              <th>Target Entity</th>
              <th>Details</th>
              <th>Outcome</th>
              <th>Correlation ID</th>
            </tr>
          </thead>
          <tbody>
            ${this.events.length === 0 ? `
              <tr><td colspan="8" style="text-align: center; padding: 24px; color: #94a3b8;">No audit events found matching query criteria.</td></tr>
            ` : this.events.map(ev => `
              <tr>
                <td class="mono">${ev.timestamp_iso.replace("T", " ").slice(0, 19)}</td>
                <td><strong>${ev.actor}</strong></td>
                <td>
                  <span class="badge role-${ev.role.toLowerCase()}">${ev.role}</span>
                </td>
                <td><strong>${ev.action}</strong></td>
                <td class="mono">${ev.target || "—"}</td>
                <td>${ev.reason || ev.action}</td>
                <td>
                  <span style="color: ${ev.status === "SUCCESS" ? "#34d399" : "#f87171"}">
                    ${ev.status}
                  </span>
                </td>
                <td class="mono" style="color: #818cf8;">${ev.correlation_id?.slice(0, 8) || "—"}</td>
              </tr>
            `).join("")}
          </tbody>
        </table>
      </div>
    `;

    this.bindEvents();
  }

  private bindEvents() {
    this.querySelector("#apply-filters-btn")?.addEventListener("click", () => {
      const roleSel = this.querySelector("#filter-role") as HTMLSelectElement;
      const actInput = this.querySelector("#filter-action") as HTMLInputElement;
      const actorInput = this.querySelector("#filter-actor") as HTMLInputElement;
      this.selectedRole = roleSel.value;
      this.selectedAction = actInput.value;
      this.searchActor = actorInput.value;
      this.fetchData();
    });

    this.querySelector("#clear-filters-btn")?.addEventListener("click", () => {
      this.selectedRole = "";
      this.selectedAction = "";
      this.searchActor = "";
      this.fetchData();
    });

    this.querySelector("#export-json-btn")?.addEventListener("click", () => this.exportJson());
  }
}

customElements.define("nebu-audit-panel", NebuAuditPanel);
