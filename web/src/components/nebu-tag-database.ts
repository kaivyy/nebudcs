/**
 * NebuTagDatabase Web Component.
 *
 * Implements Stage 4 Canonical Tag Database Management:
 * - Tabular view of all plant and controller signal tags
 * - Filtering by tag name, description, source, and direction
 * - Channel binding and controller assignment verification
 * - Seamless cross-reference inspection link
 */

import { studioStore } from "../state/studio-store";
import { ProjectDetailDto, ProjectTagDto } from "../protocol/types";

export class NebuTagDatabase extends HTMLElement {
  private searchQuery = "";
  private filterDirection = "all";
  private filterSource = "all";
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

  private openCrossRef(tagName: string) {
    this.dispatchEvent(
      new CustomEvent("project-item-selected", {
        bubbles: true,
        composed: true,
        detail: {
          section: "cross_ref",
          id: tagName,
        },
      })
    );
  }

  render() {
    const proj: ProjectDetailDto | null = studioStore.getProject();
    if (!proj) {
      this.innerHTML = `
        <div class="p-6 text-center text-dim mono">
          No project loaded. Please open a project to inspect tag database.
        </div>
      `;
      return;
    }

    const tags: ProjectTagDto[] = proj.tags || [];
    const q = this.searchQuery.toLowerCase().trim();

    const filtered = tags.filter((t) => {
      const matchesText =
        !q ||
        t.name.toLowerCase().includes(q) ||
        t.id.toLowerCase().includes(q) ||
        (t.description ? t.description.toLowerCase().includes(q) : false) ||
        (t.process_entity && t.process_entity.toLowerCase().includes(q)) ||
        (t.channel_address && t.channel_address.toLowerCase().includes(q));

      const matchesDir =
        this.filterDirection === "all" ||
        t.direction.toLowerCase() === this.filterDirection.toLowerCase();

      const matchesSrc =
        this.filterSource === "all" ||
        t.source.toLowerCase() === this.filterSource.toLowerCase();

      return matchesText && matchesDir && matchesSrc;
    });

    this.innerHTML = `
      <div class="tag-database-container flex flex-col h-full bg-app text-main text-xs">
        <!-- Top Toolbar & Filter -->
        <header class="p-3 bg-panel border-b border-color flex flex-wrap items-center justify-between gap-3">
          <div class="flex items-center gap-3">
            <span class="font-bold text-sm tracking-wide text-main">Central Tag Database</span>
            <span class="badge badge-info mono">${tags.length} Total Tags</span>
            <span class="badge badge-dim mono">${filtered.length} Displayed</span>
          </div>

          <div class="flex items-center gap-2 flex-wrap">
            <input
              type="text"
              class="tag-search px-2.5 py-1 bg-input border border-color rounded text-xs mono text-main w-64"
              placeholder="Search tag, entity, channel..."
              value="${this.searchQuery}"
            />
            <select class="filter-dir px-2 py-1 bg-input border border-color rounded text-xs mono text-main">
              <option value="all" ${this.filterDirection === "all" ? "selected" : ""}>All Directions</option>
              <option value="input" ${this.filterDirection === "input" ? "selected" : ""}>Input (Field -> DCS)</option>
              <option value="output" ${this.filterDirection === "output" ? "selected" : ""}>Output (DCS -> Actuator)</option>
              <option value="internal" ${this.filterDirection === "internal" ? "selected" : ""}>Internal / Calculation</option>
            </select>
            <select class="filter-src px-2 py-1 bg-input border border-color rounded text-xs mono text-main">
              <option value="all" ${this.filterSource === "all" ? "selected" : ""}>All Sources</option>
              <option value="field" ${this.filterSource === "field" ? "selected" : ""}>Field Instrument</option>
              <option value="controller" ${this.filterSource === "controller" ? "selected" : ""}>Virtual Controller</option>
              <option value="manual" ${this.filterSource === "manual" ? "selected" : ""}>Manual / Operator</option>
            </select>
          </div>
        </header>

        <!-- Main Tag Grid Table -->
        <div class="flex-1 overflow-auto p-3">
          <table class="w-full text-left border-collapse">
            <thead>
              <tr class="border-b border-color text-muted font-mono text-xs uppercase bg-panel-header">
                <th class="p-2">Tag Name</th>
                <th class="p-2">Description</th>
                <th class="p-2">Type</th>
                <th class="p-2">Unit</th>
                <th class="p-2">Source</th>
                <th class="p-2">Direction</th>
                <th class="p-2">Controller</th>
                <th class="p-2">Channel Address</th>
                <th class="p-2">Entity</th>
                <th class="p-2">Alarm</th>
                <th class="p-2 text-right">Actions</th>
              </tr>
            </thead>
            <tbody>
              ${
                filtered.length === 0
                  ? `
                <tr>
                  <td colspan="11" class="p-6 text-center text-dim mono italic">
                    No matching tags found.
                  </td>
                </tr>
              `
                  : filtered
                      .map((t) => {
                        const dirBadge =
                          t.direction.toLowerCase() === "input"
                            ? "badge-good"
                            : t.direction.toLowerCase() === "output"
                            ? "badge-info"
                            : "badge-dim";

                        return `
                  <tr class="border-b border-color hover:bg-panel transition-colors">
                    <td class="p-2 font-mono font-bold text-main whitespace-nowrap">${t.name}</td>
                    <td class="p-2 text-muted max-w-xs truncate" title="${t.description}">${t.description}</td>
                    <td class="p-2 font-mono text-xs text-info">${t.data_type}</td>
                    <td class="p-2 font-mono text-xs text-dim">${t.engineering_unit || "—"}</td>
                    <td class="p-2 font-mono text-xs text-muted">${t.source}</td>
                    <td class="p-2 whitespace-nowrap">
                      <span class="badge ${dirBadge} text-xs">${t.direction}</span>
                    </td>
                    <td class="p-2 font-mono text-xs text-main">${t.controller_id || "—"}</td>
                    <td class="p-2 font-mono text-xs text-amber">${t.channel_address || "—"}</td>
                    <td class="p-2 font-mono text-xs text-main">${t.process_entity || "—"}</td>
                    <td class="p-2 font-mono text-xs ${t.alarm_id ? "text-alarm font-bold" : "text-dim"}">${t.alarm_id || "—"}</td>
                    <td class="p-2 text-right whitespace-nowrap">
                      <button class="btn-xref px-2 py-0.5 text-xs text-info hover:text-white" data-tag="${t.name}">
                        🔍 Cross-Ref
                      </button>
                    </td>
                  </tr>
                `;
                      })
                      .join("")
              }
            </tbody>
          </table>
        </div>
      </div>
    `;

    this.bindEvents();
  }

  private bindEvents() {
    const searchInput = this.querySelector(".tag-search") as HTMLInputElement;
    searchInput?.addEventListener("input", () => {
      this.searchQuery = searchInput.value;
      this.render();
    });

    const filterDir = this.querySelector(".filter-dir") as HTMLSelectElement;
    filterDir?.addEventListener("change", () => {
      this.filterDirection = filterDir.value;
      this.render();
    });

    const filterSrc = this.querySelector(".filter-src") as HTMLSelectElement;
    filterSrc?.addEventListener("change", () => {
      this.filterSource = filterSrc.value;
      this.render();
    });

    this.querySelectorAll(".btn-xref").forEach((btn) => {
      btn.addEventListener("click", () => {
        const tagName = btn.getAttribute("data-tag");
        if (tagName) {
          this.openCrossRef(tagName);
        }
      });
    });
  }
}

customElements.define("nebu-tag-database", NebuTagDatabase);
