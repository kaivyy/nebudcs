/**
 * NebuCrossReference Web Component.
 *
 * Implements Stage 4 Tag and Signal Cross-Reference Inspector:
 * - Query cross-references across P&ID, Controllers, Logic Routines, HAL Channels, and Alarms
 * - Filter by usage direction (Read / Write / Bounded) and entity category
 * - Live inspection of tag dependencies to prevent unmapped signals or orphan loops
 */

import { ApiClient, sharedApiClient } from "../api/client";
import { CrossReferenceEntryDto, CrossReferenceResultDto } from "../protocol/types";
import { studioStore } from "../state/studio-store";

export class NebuCrossReference extends HTMLElement {
  public apiClient: ApiClient = sharedApiClient;
  private currentQuery = "*";
  private result: CrossReferenceResultDto | null = null;
  private isLoading = false;
  private errorMessage: string | null = null;
  private typeFilter = "all";

  connectedCallback() {
    this.render();
    this.runQuery(this.currentQuery);
  }

  public setQuery(query: string) {
    this.currentQuery = query;
    const input = this.querySelector(".xref-query-input") as HTMLInputElement;
    if (input) input.value = query;
    this.runQuery(query);
  }

  public async runQuery(target: string) {
    const proj = studioStore.getProject();
    if (!proj) return;

    this.isLoading = true;
    this.errorMessage = null;
    this.render();

    try {
      this.result = await this.apiClient.getCrossReference(proj.id, target || "*");
    } catch (err: any) {
      this.errorMessage = err.message || "Failed to query cross-reference database";
      this.result = null;
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  render() {
    const proj = studioStore.getProject();
    const entries: CrossReferenceEntryDto[] = this.result?.entries || [];

    const filtered = entries.filter((e) => {
      if (this.typeFilter === "all") return true;
      return e.entity_type.toLowerCase() === this.typeFilter.toLowerCase();
    });

    this.innerHTML = `
      <div class="cross-reference-container flex flex-col h-full bg-app text-main text-xs">
        <!-- Search Toolbar -->
        <header class="p-3 bg-panel border-b border-color flex flex-wrap items-center justify-between gap-3">
          <div class="flex items-center gap-2">
            <span class="font-bold text-sm tracking-wide text-main">Signal & Tag Cross-Reference</span>
            ${
              this.result
                ? `<span class="badge badge-info mono">${this.result.entries.length} References Found</span>`
                : ""
            }
          </div>

          <div class="flex items-center gap-2 flex-wrap">
            <div class="flex items-center gap-1">
              <input
                type="text"
                class="xref-query-input px-2.5 py-1 bg-input border border-color rounded text-xs mono text-main w-64"
                placeholder="Enter tag name, e.g. V-101.LEVEL or *"
                value="${this.currentQuery}"
              />
              <button class="btn-query px-3 py-1 bg-info text-white font-medium rounded hover:bg-blue-600">
                ${this.isLoading ? "Searching..." : "Search"}
              </button>
            </div>

            <select class="filter-type px-2 py-1 bg-input border border-color rounded text-xs mono text-main">
              <option value="all" ${this.typeFilter === "all" ? "selected" : ""}>All Entity Types</option>
              <option value="Tag" ${this.typeFilter === "Tag" ? "selected" : ""}>Tag Database</option>
              <option value="ControlLoop" ${this.typeFilter === "ControlLoop" ? "selected" : ""}>Control Loops</option>
              <option value="Alarm" ${this.typeFilter === "Alarm" ? "selected" : ""}>Alarms</option>
              <option value="ChannelMapping" ${this.typeFilter === "ChannelMapping" ? "selected" : ""}>I/O Mappings</option>
              <option value="LogicRoutine" ${this.typeFilter === "LogicRoutine" ? "selected" : ""}>Logic Routines</option>
            </select>
          </div>
        </header>

        <!-- Body Content -->
        <div class="flex-1 overflow-auto p-3">
          ${
            !proj
              ? `<div class="p-6 text-center text-dim mono">No project active.</div>`
              : this.errorMessage
              ? `<div class="p-4 bg-red-950/40 border border-alarm rounded text-alarm mono">${this.errorMessage}</div>`
              : this.isLoading
              ? `<div class="p-8 text-center text-muted mono">Querying cross-reference engine...</div>`
              : filtered.length === 0
              ? `<div class="p-8 text-center text-dim mono italic">No cross-references found for '${this.currentQuery}'.</div>`
              : `
            <table class="w-full text-left border-collapse">
              <thead>
                <tr class="border-b border-color text-muted font-mono text-xs uppercase bg-panel-header">
                  <th class="p-2">Target Query</th>
                  <th class="p-2">Entity Name</th>
                  <th class="p-2">Entity Type</th>
                  <th class="p-2">Location</th>
                  <th class="p-2">Usage / Direction</th>
                  <th class="p-2">Context / Description</th>
                </tr>
              </thead>
              <tbody>
                ${filtered
                  .map((e) => {
                    const roleBadge =
                      e.role === "Read"
                        ? "badge-info"
                        : e.role === "Write"
                        ? "badge-good"
                        : "badge-warn";

                    return `
                  <tr class="border-b border-color hover:bg-panel transition-colors">
                    <td class="p-2 font-mono text-dim whitespace-nowrap">${this.result?.target || "*"}</td>
                    <td class="p-2 font-mono font-bold text-main whitespace-nowrap">${e.tag}</td>
                    <td class="p-2 font-mono text-xs text-info whitespace-nowrap">${e.entity_type}</td>
                    <td class="p-2 font-mono text-xs text-amber max-w-xs truncate" title="${e.location}">${e.location}</td>
                    <td class="p-2 whitespace-nowrap">
                      <span class="badge ${roleBadge} text-xs mono">${e.role}</span>
                    </td>
                    <td class="p-2 text-muted text-xs max-w-md truncate" title="${e.details}">${e.details}</td>
                  </tr>
                `;
                  })
                  .join("")}
              </tbody>
            </table>
          `
          }
        </div>
      </div>
    `;

    this.bindEvents();
  }

  private bindEvents() {
    const input = this.querySelector(".xref-query-input") as HTMLInputElement;
    input?.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        this.currentQuery = input.value;
        this.runQuery(this.currentQuery);
      }
    });

    const btn = this.querySelector(".btn-query");
    btn?.addEventListener("click", () => {
      if (input) {
        this.currentQuery = input.value;
        this.runQuery(this.currentQuery);
      }
    });

    const filterSelect = this.querySelector(".filter-type") as HTMLSelectElement;
    filterSelect?.addEventListener("change", () => {
      this.typeFilter = filterSelect.value;
      this.render();
    });
  }
}

customElements.define("nebu-cross-reference", NebuCrossReference);
