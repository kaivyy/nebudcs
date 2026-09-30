/**
 * Validation Diagnostics Panel Web Component.
 *
 * Displays topology, port compatibility, physical parameter consistency,
 * and cyclic recycle diagnostics with direct clickable links to offending entities.
 */

import { studioStore } from "../state/studio-store";

export class NebuValidationPanel extends HTMLElement {
  private userExplicitCollapse: boolean | null = null;
  private unsubscribe: (() => void) | null = null;

  connectedCallback() {
    this.unsubscribe = studioStore.subscribe(() => this.render());
    this.render();
  }

  disconnectedCallback() {
    if (this.unsubscribe) {
      this.unsubscribe();
      this.unsubscribe = null;
    }
  }

  render() {
    const val = studioStore.getValidation();

    if (!val) {
      this.innerHTML = `
        <div class="validation-panel-container collapsed">
          <div class="validation-header">
            <span class="val-title">Validation Diagnostics</span>
            <span class="val-status-badge status-idle">Not Validated</span>
          </div>
        </div>
      `;
      return;
    }

    const isSuccess = val.valid && val.error_count === 0;
    const hasIssues = val.error_count > 0 || val.warning_count > 0;
    const isCollapsed = this.userExplicitCollapse !== null ? this.userExplicitCollapse : !hasIssues;

    this.innerHTML = `
      <div class="validation-panel-container ${isCollapsed ? "collapsed" : ""}">
        <div class="validation-header">
          <div class="val-header-left">
            <button class="btn-toggle-val">${isCollapsed ? "▲" : "▼"}</button>
            <span class="val-title">Validation Diagnostics</span>
            <span class="val-status-badge ${isSuccess ? "status-valid" : "status-invalid"}">
              ${isSuccess ? "VALID" : "INVALID"}
            </span>
          </div>
          <div class="val-counts">
            <span class="val-count-badge count-error">${val.error_count} Errors</span>
            <span class="val-count-badge count-warning">${val.warning_count} Warnings</span>
            <span class="val-count-badge count-info">${val.info_count} Info</span>
          </div>
        </div>

        ${
          !isCollapsed
            ? `
          <div class="validation-results-list">
            ${
              val.results.length === 0
                ? `<div class="val-empty">✓ Flowsheet topology and equipment configuration are fully valid.</div>`
                : val.results
                    .map(
                      (r) => `
                  <div class="val-result-item severity-${r.severity}">
                    <span class="val-code">${r.code}</span>
                    <span class="val-msg">${r.message}</span>
                    ${
                      r.entity_id
                        ? `<button class="btn-goto-entity" data-entity-id="${r.entity_id}">Locate</button>`
                        : ""
                    }
                  </div>
                `
                    )
                    .join("")
            }
          </div>
        `
            : ""
        }
      </div>
    `;

    const toggleBtn = this.querySelector(".btn-toggle-val");
    toggleBtn?.addEventListener("click", () => {
      this.userExplicitCollapse = !isCollapsed;
      this.render();
    });

    this.querySelectorAll(".btn-goto-entity").forEach((btn) => {
      btn.addEventListener("click", () => {
        const entityId = btn.getAttribute("data-entity-id");
        if (!entityId) return;

        const proj = studioStore.getProject();
        if (!proj) return;

        // Find entity type
        if (proj.equipment.some((e) => e.id === entityId)) {
          studioStore.selectItem("equipment", entityId);
        } else if (proj.streams.some((s) => s.id === entityId)) {
          studioStore.selectItem("stream", entityId);
        } else if (proj.instruments.some((i) => i.id === entityId)) {
          studioStore.selectItem("instrument", entityId);
        } else if (proj.control_loops.some((c) => c.id === entityId)) {
          studioStore.selectItem("control_loop", entityId);
        }
      });
    });
  }
}

customElements.define("nebu-validation-panel", NebuValidationPanel);
