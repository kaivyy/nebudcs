/**
 * Enterprise Backup, Validation & Restore Web Component.
 *
 * Implements Stage 14: Enterprise, High Availability & Distributed Runtime.
 * Features:
 * - Full system backup generation containing projects, scenarios, training records, cluster config, and validation evidence
 * - Dry-run validation of backup archives with SHA-256 manifest check before disk mutation
 * - Production recovery and restore execution
 */

import { client } from "../api/client";
import { RestoreValidationDto, RestoreResultDto } from "../protocol/types";

export class NebuBackupPanel extends HTMLElement {
  private validationResult: RestoreValidationDto | null = null;
  private restoreResult: RestoreResultDto | null = null;
  private stagedBundle: any = null;
  private isLoading = false;
  private statusMessage: string | null = null;
  private isError = false;

  async connectedCallback() {
    this.render();
  }

  private async generateBackup() {
    this.isLoading = true;
    this.render();
    try {
      const bundle = await client.createBackupBundle();
      const bundleStr = JSON.stringify(bundle, null, 2);
      const blob = new Blob([bundleStr], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `nebudcs-backup-bundle-${new Date().toISOString().slice(0, 10)}.json`;
      a.click();
      URL.revokeObjectURL(url);
      this.statusMessage = "Backup bundle created and downloaded successfully.";
      this.isError = false;
    } catch (err: unknown) {
      this.isError = true;
      this.statusMessage = `Backup creation failed: ${err instanceof Error ? err.message : String(err)}`;
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private async handleFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;
    const file = input.files[0];
    try {
      const text = await file.text();
      this.stagedBundle = JSON.parse(text);
      this.statusMessage = `Staged file: ${file.name} (${Math.round(file.size / 1024)} KB)`;
      this.isError = false;
      this.validationResult = null;
      this.restoreResult = null;
    } catch (err: unknown) {
      this.isError = true;
      this.statusMessage = `Invalid JSON bundle: ${err instanceof Error ? err.message : String(err)}`;
    }
    this.render();
  }

  private async validateStagedBundle() {
    if (!this.stagedBundle) return;
    this.isLoading = true;
    this.render();
    try {
      const res = await client.validateBackup(this.stagedBundle);
      this.validationResult = res;
      this.isError = !res.valid;
      this.statusMessage = res.valid
        ? `Dry-run validation PASSED: Verified ${res.manifest_items_verified} items (${res.projects_count} projects). Ready to restore.`
        : `Dry-run validation FAILED: ${res.validation_messages.join("; ")}`;
    } catch (err: unknown) {
      this.isError = true;
      this.statusMessage = `Validation request error: ${err instanceof Error ? err.message : String(err)}`;
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private async executeRestore() {
    if (!this.stagedBundle) return;
    if (!confirm("Are you sure you want to restore this backup bundle? This will merge and update project configurations.")) {
      return;
    }
    this.isLoading = true;
    this.render();
    try {
      const res = await client.restoreBackup(this.stagedBundle);
      this.restoreResult = res;
      this.isError = false;
      this.statusMessage = `Restore SUCCESS: Restored ${res.projects_restored} projects, ${res.scenarios_restored} scenarios.`;
    } catch (err: unknown) {
      this.isError = true;
      this.statusMessage = `Restore failed: ${err instanceof Error ? err.message : String(err)}`;
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  render() {
    const val = this.validationResult;
    const rest = this.restoreResult;

    this.innerHTML = `
      <style>
        nebu-backup-panel {
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
        .card {
          background: #171f2e;
          border: 1px solid #263346;
          border-radius: 8px;
          padding: 16px;
          margin-bottom: 18px;
        }
        .card-title {
          font-size: 1.05rem;
          font-weight: 600;
          color: #fff;
          margin-bottom: 8px;
          display: flex;
          align-items: center;
          gap: 8px;
        }
        .card-desc {
          font-size: 0.85rem;
          color: #94a3b8;
          margin-bottom: 14px;
          line-height: 1.4;
        }
        .btn {
          padding: 8px 16px;
          border: 1px solid #35455e;
          background: #1f2a3c;
          color: #e2e8f0;
          border-radius: 4px;
          cursor: pointer;
          font-size: 0.85rem;
          font-weight: 500;
        }
        .btn:hover { background: #28374f; }
        .btn-primary { background: #2563eb; border-color: #3b82f6; color: white; }
        .btn-primary:hover { background: #1d4ed8; }
        .btn-success { background: #15803d; border-color: #22c55e; color: white; }
        .btn-success:hover { background: #166534; }
        .btn-danger { background: #dc2626; border-color: #ef4444; color: white; }
        .status-banner {
          padding: 10px 14px;
          border-radius: 6px;
          font-size: 0.85rem;
          margin-bottom: 14px;
        }
        .status-banner.error { background: #451217; color: #f87171; border: 1px solid #7f1d1d; }
        .status-banner.info { background: #0f2d4a; color: #7dd3fc; border: 1px solid #075985; }
        .val-report {
          background: #0f141d;
          border: 1px solid #232c3d;
          border-radius: 6px;
          padding: 12px;
          margin-top: 14px;
          font-size: 0.85rem;
        }
        .grid-2 {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
          gap: 10px;
          margin-top: 10px;
        }
        .val-item {
          color: #94a3b8;
        }
        .val-item strong {
          color: #e2e8f0;
        }
      </style>

      <div class="header">
        <div class="title-area">
          <h2>
            <span>💾</span> Enterprise Backup, Integrity Verification & Restore
          </h2>
          <div class="subtitle">Cryptographic Archive Generation • Dry-Run Manifest Inspection • Zero-Data-Loss Restore</div>
        </div>
      </div>

      ${this.statusMessage ? `
        <div class="status-banner ${this.isError ? "error" : "info"}">
          ${this.statusMessage}
        </div>
      ` : ""}

      <div class="card">
        <div class="card-title">⛁ Export Full Enterprise Backup Bundle</div>
        <div class="card-desc">
          Packages all projects, plant hierarchies, equipment topologies, OTS training scenarios, cluster configuration, and scientific validation evidence into a SHA-256 signed archive.
        </div>
        <button class="btn btn-primary" id="export-backup-btn" ${this.isLoading ? "disabled" : ""}>
          📥 Create & Download Backup Bundle (.json)
        </button>
      </div>

      <div class="card">
        <div class="card-title">📤 Dry-Run Validation & Production Restore</div>
        <div class="card-desc">
          Stage a backup bundle to perform comprehensive dry-run schema and cryptographic manifest validation.
          Verifies all archive entities without modifying local state.
        </div>
        
        <div style="margin-bottom: 14px;">
          <input type="file" id="bundle-file-input" accept=".json" style="color: #94a3b8; font-size: 0.85rem;">
        </div>

        <div style="display: flex; gap: 10px; align-items: center;">
          <button class="btn" id="validate-btn" ${!this.stagedBundle || this.isLoading ? "disabled" : ""}>
            🔍 Run Dry-Run Validation
          </button>
          <button class="btn btn-danger" id="restore-btn" ${!this.validationResult?.valid || this.isLoading ? "disabled" : ""}>
            ⚡ Execute Verified Restore
          </button>
        </div>

        ${val ? `
          <div class="val-report" style="border-left: 4px solid ${val.valid ? "#22c55e" : "#ef4444"};">
            <div style="font-weight: 600; color: ${val.valid ? "#4ade80" : "#f87171"}; margin-bottom: 6px;">
              ${val.valid ? "✓ Dry-Run Validation PASSED" : "✗ Dry-Run Validation FAILED"}
            </div>
            <div class="grid-2">
              <div class="val-item">Bundle Version: <strong>${val.bundle_version}</strong></div>
              <div class="val-item">Manifest Items Verified: <strong>${val.manifest_items_verified}</strong></div>
              <div class="val-item">Projects Count: <strong>${val.projects_count}</strong></div>
              <div class="val-item">Scenarios Count: <strong>${val.scenarios_count}</strong></div>
              <div class="val-item">Cluster Config Present: <strong>${val.has_cluster_config ? "Yes" : "No"}</strong></div>
              <div class="val-item">Validation Evidence Present: <strong>${val.has_validation_evidence ? "Yes" : "No"}</strong></div>
            </div>
            ${val.validation_messages.length > 0 ? `
              <div style="margin-top: 10px; font-size: 0.8rem; color: #fca5a5;">
                <strong>Details:</strong>
                <ul style="margin: 4px 0 0 16px; padding: 0;">
                  ${val.validation_messages.map(m => `<li>${m}</li>`).join("")}
                </ul>
              </div>
            ` : ""}
          </div>
        ` : ""}

        ${rest ? `
          <div class="val-report" style="border-left: 4px solid #22c55e; margin-top: 10px;">
            <div style="font-weight: 600; color: #4ade80;">✓ Restore Completed Successfully</div>
            <div class="grid-2" style="margin-top: 6px;">
              <div class="val-item">Projects Restored: <strong>${rest.projects_restored}</strong></div>
              <div class="val-item">Scenarios Restored: <strong>${rest.scenarios_restored}</strong></div>
              <div class="val-item">Training Records: <strong>${rest.training_restored}</strong></div>
            </div>
          </div>
        ` : ""}
      </div>
    `;

    this.bindEvents();
  }

  private bindEvents() {
    this.querySelector("#export-backup-btn")?.addEventListener("click", () => this.generateBackup());
    this.querySelector("#bundle-file-input")?.addEventListener("change", (e) => this.handleFileSelect(e));
    this.querySelector("#validate-btn")?.addEventListener("click", () => this.validateStagedBundle());
    this.querySelector("#restore-btn")?.addEventListener("click", () => this.executeRestore());
  }
}

customElements.define("nebu-backup-panel", NebuBackupPanel);
