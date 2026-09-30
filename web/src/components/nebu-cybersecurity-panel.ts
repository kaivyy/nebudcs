/**
 * NebuCybersecurityPanel Web Component.
 *
 * Implements Stage 15: Industrial Cyber Security & IEC 62443 Alignment.
 * Standards Reference:
 * - IEC 62443-3-2: Security Risk Assessment and System Design (Zones & Conduits).
 * - IEC 62443-3-3: System Security Requirements and Security Levels (SL-1 to SL-4).
 * - IEC 62443-4-2: Technical Security Requirements for IACS Components.
 * - NIST SP 800-82: Guide to Industrial Control Systems (ICS) Security.
 * - 21 CFR Part 11 / SR 2.8: Cryptographic Tamper-Evident Audit Trails.
 */

import { client } from "../api/client";
import {
  AuditVerificationResultDto,
  PurdueConduitDto,
  PurdueZoneDto,
  SecurityIncidentDto,
  SecurityStatusDto,
} from "../protocol/types";

export class NebuCybersecurityPanel extends HTMLElement {
  private status: SecurityStatusDto | null = null;
  private zones: PurdueZoneDto[] = [];
  private conduits: PurdueConduitDto[] = [];
  private incidents: SecurityIncidentDto[] = [];
  private verificationResult: AuditVerificationResultDto | null = null;

  private selectedZone: PurdueZoneDto | null = null;
  private isVerifyingAudit = false;
  private isLoading = false;
  private errorMessage: string | null = null;
  private successMessage: string | null = null;
  private refreshIntervalId?: number;

  async connectedCallback() {
    await this.fetchData();
    // Auto-refresh security posture every 5 seconds
    this.refreshIntervalId = window.setInterval(() => {
      this.fetchData(true);
    }, 5000);
  }

  disconnectedCallback() {
    if (this.refreshIntervalId) {
      clearInterval(this.refreshIntervalId);
    }
  }

  private async fetchData(silent = false) {
    if (!silent) {
      this.isLoading = true;
      this.render();
    }
    try {
      const [secStatus, zoneList, conduitList, incidentList] = await Promise.all([
        client.getSecurityStatus(),
        client.getSecurityZones(),
        client.getSecurityConduits(),
        client.getSecurityIncidents(30),
      ]);
      this.status = secStatus;
      this.zones = zoneList;
      this.conduits = conduitList;
      this.incidents = incidentList;
      this.errorMessage = null;
    } catch (err: unknown) {
      if (!silent) {
        this.errorMessage = `Failed to load cybersecurity status: ${
          err instanceof Error ? err.message : String(err)
        }`;
      }
    } finally {
      if (!silent) {
        this.isLoading = false;
      }
      this.render();
    }
  }

  private async verifyAuditTrail() {
    this.isVerifyingAudit = true;
    this.errorMessage = null;
    this.successMessage = null;
    this.render();

    try {
      const res = await client.verifyAuditIntegrity();
      this.verificationResult = res;
      if (res.is_valid && !res.tampering_detected) {
        this.successMessage = `Cryptographic integrity verified: ${res.total_events_checked} audit events unbroken. Root SHA-256 valid.`;
      } else {
        this.errorMessage = `CRITICAL: Tamper detection alert! Chain unbroken verification failed at event ${
          res.first_invalid_event_id || "unknown"
        }.`;
      }
      // Re-fetch status to refresh badges
      const statusRes = await client.getSecurityStatus();
      this.status = statusRes;
    } catch (err: unknown) {
      this.errorMessage = `Audit verification error: ${
        err instanceof Error ? err.message : String(err)
      }`;
    } finally {
      this.isVerifyingAudit = false;
      this.render();
    }
  }

  private async toggleConduitEncryption(conduit: PurdueConduitDto) {
    try {
      const updated = await client.updateConduitPolicy({
        conduit_id: conduit.conduit_id,
        allowed_protocols: conduit.allowed_protocols,
        is_encrypted: !conduit.is_encrypted,
        status: conduit.status,
      });
      this.successMessage = `Conduit ${conduit.conduit_id} encryption updated to ${
        updated.is_encrypted ? "TLS/mTLS Active" : "Disabled"
      }`;
      await this.fetchData(true);
    } catch (err: unknown) {
      this.errorMessage = `Failed to update conduit policy: ${
        err instanceof Error ? err.message : String(err)
      }`;
      this.render();
    }
  }

  render() {
    const s = this.status;
    const threatLevel = s?.active_threat_level || "LOW";
    const threatColor =
      threatLevel === "CRITICAL"
        ? "#ef4444"
        : threatLevel === "HIGH"
        ? "#f97316"
        : threatLevel === "ELEVATED"
        ? "#eab308"
        : "#10b981";

    this.innerHTML = `
      <style>
        nebu-cybersecurity-panel {
          display: block;
          padding: 16px;
          color: #e2e8f0;
          background: #0b0f17;
          font-family: var(--font-family, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace);
          box-sizing: border-box;
          height: 100%;
          overflow-y: auto;
        }

        .header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          border-bottom: 1px solid #1e293b;
          padding-bottom: 14px;
          margin-bottom: 16px;
        }

        .title-area h2 {
          margin: 0;
          font-size: 1.35rem;
          font-weight: 700;
          color: #f8fafc;
          display: flex;
          align-items: center;
          gap: 10px;
          letter-spacing: -0.02em;
        }

        .standards-badge {
          font-size: 0.7rem;
          padding: 2px 8px;
          border-radius: 4px;
          background: #1e293b;
          color: #38bdf8;
          border: 1px solid #0284c7;
          font-weight: 600;
        }

        .subtitle {
          color: #94a3b8;
          font-size: 0.85rem;
          margin-top: 4px;
        }

        .header-actions {
          display: flex;
          gap: 10px;
          align-items: center;
        }

        .btn-sec {
          background: #1e293b;
          color: #f8fafc;
          border: 1px solid #334155;
          border-radius: 6px;
          padding: 8px 14px;
          font-size: 0.82rem;
          font-weight: 600;
          cursor: pointer;
          transition: all 0.15s ease;
          display: inline-flex;
          align-items: center;
          gap: 6px;
        }

        .btn-sec:hover {
          background: #334155;
          border-color: #475569;
        }

        .btn-verify {
          background: #065f46;
          border-color: #059669;
          color: #a7f3d0;
        }

        .btn-verify:hover {
          background: #047857;
          border-color: #10b981;
        }

        .btn-verify:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .posture-cards {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
          gap: 12px;
          margin-bottom: 20px;
        }

        .kpi-card {
          background: #131822;
          border: 1px solid #1e293b;
          border-radius: 8px;
          padding: 12px 14px;
          position: relative;
        }

        .kpi-label {
          font-size: 0.75rem;
          color: #64748b;
          text-transform: uppercase;
          font-weight: 600;
          letter-spacing: 0.05em;
        }

        .kpi-val {
          font-size: 1.3rem;
          font-weight: 700;
          color: #f1f5f9;
          margin-top: 4px;
          display: flex;
          align-items: center;
          gap: 8px;
        }

        .kpi-sub {
          font-size: 0.75rem;
          color: #94a3b8;
          margin-top: 4px;
        }

        .pulse-dot {
          width: 10px;
          height: 10px;
          border-radius: 50%;
          display: inline-block;
        }

        .pulse-crit {
          animation: pulse 1.5s infinite;
        }

        @keyframes pulse {
          0% { transform: scale(0.95); opacity: 0.8; }
          50% { transform: scale(1.2); opacity: 1; box-shadow: 0 0 10px #ef4444; }
          100% { transform: scale(0.95); opacity: 0.8; }
        }

        .banner-msg {
          padding: 10px 14px;
          border-radius: 6px;
          font-size: 0.85rem;
          margin-bottom: 16px;
          display: flex;
          align-items: center;
          justify-content: space-between;
        }

        .banner-err {
          background: rgba(239, 68, 68, 0.15);
          border: 1px solid #ef4444;
          color: #fca5a5;
        }

        .banner-ok {
          background: rgba(16, 185, 129, 0.15);
          border: 1px solid #10b981;
          color: #6ee7b7;
        }

        .main-layout {
          display: grid;
          grid-template-columns: 1.4fr 1fr;
          gap: 16px;
        }

        @media (max-width: 1200px) {
          .main-layout {
            grid-template-columns: 1fr;
          }
        }

        .section-box {
          background: #131822;
          border: 1px solid #1e293b;
          border-radius: 8px;
          padding: 14px;
          margin-bottom: 16px;
        }

        .section-title {
          font-size: 0.95rem;
          font-weight: 600;
          color: #cbd5e1;
          margin: 0 0 12px 0;
          display: flex;
          align-items: center;
          justify-content: space-between;
        }

        /* Purdue Zones Visual Stack */
        .purdue-stack {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }

        .purdue-zone-row {
          background: #0f172a;
          border: 1px solid #1e293b;
          border-left: 4px solid #38bdf8;
          border-radius: 6px;
          padding: 10px 12px;
          cursor: pointer;
          transition: all 0.15s ease;
        }

        .purdue-zone-row:hover, .purdue-zone-row.selected {
          border-color: #38bdf8;
          background: #1e293b;
        }

        .zone-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
        }

        .zone-title {
          font-weight: 600;
          font-size: 0.88rem;
          color: #f8fafc;
          display: flex;
          align-items: center;
          gap: 8px;
        }

        .zone-badge {
          font-size: 0.7rem;
          font-weight: 700;
          padding: 2px 6px;
          border-radius: 3px;
        }

        .zone-desc {
          font-size: 0.78rem;
          color: #94a3b8;
          margin-top: 4px;
        }

        .zone-meta {
          display: flex;
          gap: 12px;
          font-size: 0.75rem;
          color: #64748b;
          margin-top: 6px;
        }

        /* Conduits Table */
        .conduit-table {
          width: 100%;
          border-collapse: collapse;
          font-size: 0.8rem;
        }

        .conduit-table th {
          text-align: left;
          padding: 8px;
          background: #0f172a;
          color: #64748b;
          font-weight: 600;
          border-bottom: 1px solid #1e293b;
        }

        .conduit-table td {
          padding: 8px;
          border-bottom: 1px solid #1e293b;
          color: #cbd5e1;
        }

        .conduit-row:hover {
          background: #1e293b;
        }

        .conduit-badge {
          display: inline-block;
          font-size: 0.7rem;
          padding: 1px 5px;
          border-radius: 3px;
          background: #0284c7;
          color: #fff;
          margin-right: 4px;
        }

        .diode-badge {
          background: #b91c1c;
          color: #fef2f2;
          font-size: 0.7rem;
          font-weight: 700;
          padding: 2px 6px;
          border-radius: 4px;
        }

        /* Incidents Table */
        .incident-item {
          padding: 8px 10px;
          border-bottom: 1px solid #1e293b;
          font-size: 0.8rem;
          display: flex;
          flex-direction: column;
          gap: 3px;
        }

        .incident-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
        }

        .sev-pill {
          font-size: 0.68rem;
          font-weight: 700;
          padding: 1px 6px;
          border-radius: 3px;
        }

        .sev-CRITICAL {
          background: #ef4444;
          color: #fff;
        }

        .sev-HIGH {
          background: #f97316;
          color: #fff;
        }

        .sev-WARNING {
          background: #eab308;
          color: #000;
        }

        .sev-INFO {
          background: #3b82f6;
          color: #fff;
        }

        .hash-code {
          font-family: monospace;
          background: #0b0f17;
          padding: 2px 6px;
          border-radius: 4px;
          font-size: 0.72rem;
          color: #38bdf8;
          word-break: break-all;
        }
      </style>

      <!-- Panel Header -->
      <div class="header">
        <div class="title-area">
          <h2>
            🛡️ IEC 62443 Industrial Cyber Security & PERA Architecture
            <span class="standards-badge">IEC 62443-3-2 / 3-3</span>
          </h2>
          <div class="subtitle">
            Zero-Trust Industrial Demilitarized Zones, Deep Protocol Conduits & Tamper-Evident SHA-256 Audit Integrity
          </div>
        </div>
        <div class="header-actions">
          <button id="btnVerifyAudit" class="btn-sec btn-verify" ${
            this.isVerifyingAudit ? "disabled" : ""
          }>
            ${this.isVerifyingAudit ? "⏳ Verifying Hashes..." : "🔐 Verify Audit Hash Chain"}
          </button>
          <button id="btnRefresh" class="btn-sec" ${this.isLoading ? "disabled" : ""}>
            ${this.isLoading ? "⏳ Loading..." : "🔄 Refresh"}
          </button>
        </div>
      </div>

      <!-- Messages / Alerts -->
      ${
        this.errorMessage
          ? `<div class="banner-msg banner-err"><span>⚠️ ${this.errorMessage}</span><button class="btn-sec" style="padding:2px 6px;" id="btnCloseErr">✕</button></div>`
          : ""
      }
      ${
        this.successMessage
          ? `<div class="banner-msg banner-ok"><span>✅ ${this.successMessage}</span><button class="btn-sec" style="padding:2px 6px;" id="btnCloseOk">✕</button></div>`
          : ""
      }

      <!-- Top Level Security Posture Indicators -->
      <div class="posture-cards">
        <div class="kpi-card">
          <div class="kpi-label">System Security Level</div>
          <div class="kpi-val" style="color: #38bdf8;">
            ${s?.system_security_level || "SL3_SOPHISTICATED"}
          </div>
          <div class="kpi-sub">Target: SL-3 / IEC 62443-3-3</div>
        </div>

        <div class="kpi-card">
          <div class="kpi-label">Active Threat Posture</div>
          <div class="kpi-val" style="color: ${threatColor};">
            <span class="pulse-dot ${threatLevel === "CRITICAL" ? "pulse-crit" : ""}" style="background: ${threatColor};"></span>
            ${threatLevel}
          </div>
          <div class="kpi-sub">${s?.active_incidents || 0} active anomaly detections</div>
        </div>

        <div class="kpi-card">
          <div class="kpi-label">Cryptographic Audit Chain</div>
          <div class="kpi-val" style="color: ${s?.audit_chain_valid !== false ? "#10b981" : "#ef4444"};">
            ${s?.audit_chain_valid !== false ? "✓ SECURE & VALID" : "⚠️ CHAIN BROKEN"}
          </div>
          <div class="kpi-sub">${s?.audit_events_count || 0} SHA-256 linked records</div>
        </div>

        <div class="kpi-card">
          <div class="kpi-label">Defense-in-Depth Model</div>
          <div class="kpi-val" style="color: #a78bfa;">
            ${this.zones.length} Zones / ${this.conduits.length} Conduits
          </div>
          <div class="kpi-sub">PERA L0 through L5 Segmented</div>
        </div>
      </div>

      <!-- Main Layout: Zones/Conduits on Left, Audit & IDS on Right -->
      <div class="main-layout">
        <!-- Left Column: Zones & Conduits -->
        <div class="left-col">
          <!-- Purdue Model Zones Stack -->
          <div class="section-box">
            <div class="section-title">
              <span>🏛️ Purdue Model (PERA) Industrial Zones</span>
              <span style="font-size: 0.75rem; color: #64748b;">Click zone to inspect policy</span>
            </div>
            <div class="purdue-stack">
              ${this.zones
                .slice()
                .sort((a, b) => b.purdue_level - a.purdue_level)
                .map((z) => {
                  const borderCol =
                    z.purdue_level >= 4
                      ? "#60a5fa"
                      : z.purdue_level === 3
                      ? "#a78bfa"
                      : z.purdue_level === 2
                      ? "#34d399"
                      : z.purdue_level === 1
                      ? "#f59e0b"
                      : "#ef4444";
                  return `
                    <div class="purdue-zone-row ${this.selectedZone?.zone_id === z.zone_id ? "selected" : ""}" data-zone-id="${z.zone_id}" style="border-left-color: ${borderCol};">
                      <div class="zone-header">
                        <span class="zone-title">
                          <span style="color:${borderCol}; font-weight:700;">L${z.purdue_level}</span>
                          ${z.name}
                        </span>
                        <span class="zone-badge" style="background:${borderCol}22; color:${borderCol}; border:1px solid ${borderCol}44;">
                          ${z.target_security_level}
                        </span>
                      </div>
                      <div class="zone-desc">${z.description}</div>
                      <div class="zone-meta">
                        <span>Assets: <strong style="color:#e2e8f0;">${z.assets_count}</strong></span>
                        <span>Boundary: <strong style="color:#e2e8f0;">${z.trust_boundary ? "Strict Isolated" : "Standard"}</strong></span>
                        <span>Roles: <strong style="color:#e2e8f0;">${z.assigned_roles.join(", ")}</strong></span>
                      </div>
                    </div>
                  `;
                })
                .join("")}
            </div>
          </div>

          <!-- Conduits and Traffic Inspection -->
          <div class="section-box">
            <div class="section-title">
              <span>⚡ Industrial Conduits & Deep Protocol Inspection</span>
              <span style="font-size: 0.75rem; color: #64748b;">IEC 62443-3-2</span>
            </div>
            <table class="conduit-table">
              <thead>
                <tr>
                  <th>Conduit</th>
                  <th>Route</th>
                  <th>Allowed Protocols</th>
                  <th>Inspected</th>
                  <th>Blocked</th>
                  <th>Action</th>
                </tr>
              </thead>
              <tbody>
                ${this.conduits
                  .map((c) => {
                    const isDiode = c.conduit_id === "cnd-l3-l4";
                    return `
                      <tr class="conduit-row">
                        <td>
                          <strong>${c.name}</strong><br/>
                          <span style="font-size:0.7rem; color:#64748b;">${c.conduit_id}</span>
                        </td>
                        <td>
                          <span style="font-size:0.75rem;">${c.source_zone_id} ➔ ${c.destination_zone_id}</span>
                          ${isDiode ? `<br/><span class="diode-badge">DATA DIODE: WRITE-BLOCKED</span>` : ""}
                        </td>
                        <td>
                          ${c.allowed_protocols
                            .map((p) => `<span class="conduit-badge">${p}</span>`)
                            .join("")}
                        </td>
                        <td><span style="color:#38bdf8;">${c.packets_inspected}</span></td>
                        <td><span style="color:${c.violations_blocked > 0 ? "#ef4444" : "#64748b"}; font-weight:700;">${c.violations_blocked}</span></td>
                        <td>
                          <button class="btn-sec btn-toggle-enc" data-conduit-id="${c.conduit_id}" style="padding:2px 8px; font-size:0.72rem;">
                            ${c.is_encrypted ? "🔒 TLS Active" : "🔓 Plaintext"}
                          </button>
                        </td>
                      </tr>
                    `;
                  })
                  .join("")}
              </tbody>
            </table>
          </div>
        </div>

        <!-- Right Column: Cryptographic Verification & IDS Alerts -->
        <div class="right-col">
          <!-- Cryptographic Audit Verification Card -->
          <div class="section-box">
            <div class="section-title">
              <span>🔐 Cryptographic Audit Hash Verification</span>
              <span style="font-size: 0.75rem; color: #10b981;">21 CFR Part 11 / SR 2.8</span>
            </div>
            ${
              this.verificationResult
                ? `
                  <div style="background: #0f172a; padding: 12px; border-radius: 6px; border: 1px solid #1e293b; margin-bottom: 10px;">
                    <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:8px;">
                      <span style="font-weight:600; font-size:0.85rem; color:${this.verificationResult.is_valid ? "#10b981" : "#ef4444"};">
                        ${this.verificationResult.is_valid ? "✓ ALL HASHES MATHEMATICALLY VERIFIED" : "⚠️ INTEGRITY TAMPERING DETECTED"}
                      </span>
                      <span style="font-size:0.72rem; color:#64748b;">${this.verificationResult.verification_timestamp_iso}</span>
                    </div>
                    <div style="font-size:0.78rem; color:#94a3b8; margin-bottom:6px;">
                      Events Checked: <strong style="color:#f8fafc;">${this.verificationResult.total_events_checked}</strong> |
                      Tampering Detected: <strong style="color:${this.verificationResult.tampering_detected ? "#ef4444" : "#10b981"};">${this.verificationResult.tampering_detected ? "YES" : "NO"}</strong>
                    </div>
                    <div style="font-size:0.72rem; color:#64748b; margin-bottom:2px;">Merkle Root SHA-256:</div>
                    <div class="hash-code">${this.verificationResult.root_hash}</div>
                    <div style="font-size:0.72rem; color:#64748b; margin-top:6px; margin-bottom:2px;">Latest Event Hash:</div>
                    <div class="hash-code">${this.verificationResult.latest_hash}</div>
                  </div>
                `
                : `
                  <div style="background: #0f172a; padding: 14px; border-radius: 6px; border: 1px dashed #334155; text-align:center; color:#94a3b8; font-size:0.82rem;">
                    Click <strong>"Verify Audit Hash Chain"</strong> to mathematically trace and confirm unbroken SHA-256 linkage across all stored operational events.
                  </div>
                `
            }
            <div style="margin-top:10px; font-size:0.75rem; color:#64748b; line-height: 1.4;">
              Each operational audit record stores <code>SHA-256(previous_hash + event_id + timestamp + actor + action + target + status)</code> ensuring complete tamper-evidence.
            </div>
          </div>

          <!-- Intrusion Detection & Anomaly Log -->
          <div class="section-box">
            <div class="section-title">
              <span>🚨 Intrusion Detection System (IDS) Anomalies</span>
              <span style="font-size: 0.72rem; color: #64748b;">Live Buffer: ${this.incidents.length} events</span>
            </div>
            <div style="max-height: 380px; overflow-y: auto;">
              ${
                this.incidents.length === 0
                  ? `<div style="text-align:center; padding:24px; color:#64748b; font-size:0.82rem;">No active security anomalies detected. System operating within baseline.</div>`
                  : this.incidents
                      .map((inc) => `
                        <div class="incident-item">
                          <div class="incident-header">
                            <span style="font-weight:600; color:#f8fafc;">${inc.category}</span>
                            <span class="sev-pill sev-${inc.severity}">${inc.severity}</span>
                          </div>
                          <div style="color:#cbd5e1; font-size:0.78rem;">${inc.description}</div>
                          <div style="display:flex; justify-content:space-between; font-size:0.72rem; color:#64748b; margin-top:2px;">
                            <span>Source: <code>${inc.source_ip}</code> | Actor: <code>${inc.actor}</code></span>
                            <span>Action: <strong style="color:#38bdf8;">${inc.mitigation_action}</strong></span>
                          </div>
                        </div>
                      `)
                      .join("")
              }
            </div>
          </div>
        </div>
      </div>
    `;

    this.bindEvents();
  }

  private bindEvents() {
    this.querySelector("#btnRefresh")?.addEventListener("click", () => {
      this.fetchData();
    });

    this.querySelector("#btnVerifyAudit")?.addEventListener("click", () => {
      this.verifyAuditTrail();
    });

    this.querySelector("#btnCloseErr")?.addEventListener("click", () => {
      this.errorMessage = null;
      this.render();
    });

    this.querySelector("#btnCloseOk")?.addEventListener("click", () => {
      this.successMessage = null;
      this.render();
    });

    this.querySelectorAll(".purdue-zone-row").forEach((el) => {
      el.addEventListener("click", () => {
        const zoneId = el.getAttribute("data-zone-id");
        this.selectedZone = this.zones.find((z) => z.zone_id === zoneId) || null;
        this.render();
      });
    });

    this.querySelectorAll(".btn-toggle-enc").forEach((el) => {
      el.addEventListener("click", () => {
        const conduitId = el.getAttribute("data-conduit-id");
        const conduit = this.conduits.find((c) => c.conduit_id === conduitId);
        if (conduit) {
          this.toggleConduitEncryption(conduit);
        }
      });
    });
  }
}

customElements.define("nebu-cybersecurity-panel", NebuCybersecurityPanel);
