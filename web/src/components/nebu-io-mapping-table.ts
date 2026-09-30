/**
 * NebuIoMappingTable Web Component.
 *
 * Implements Stage 4 Controller-to-I/O Mapping Grid:
 * - Direct mapping between controller variables (%I/%Q or named signals) and physical HAL channels
 * - Multi-controller multicast input support and single-owner output validation
 * - Priority level visualization (Esd, Interlock, Permissive, Manual, Auto)
 * - Target tag and actuator resolution
 */

import { studioStore } from "../state/studio-store";
import { HalMappingDto, ProjectDetailDto } from "../protocol/types";

export class NebuIoMappingTable extends HTMLElement {
  private searchQuery = "";
  private filterDirection = "all";
  private filterController = "all";
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

  private openCrossRef(signalOrTag: string) {
    this.dispatchEvent(
      new CustomEvent("project-item-selected", {
        bubbles: true,
        composed: true,
        detail: {
          section: "cross_ref",
          id: signalOrTag,
        },
      })
    );
  }

  render() {
    const proj: ProjectDetailDto | null = studioStore.getProject();
    if (!proj) {
      this.innerHTML = `
        <div class="p-6 text-center text-dim mono">
          No project loaded. Please open a project to inspect I/O mappings.
        </div>
      `;
      return;
    }

    const mappings: HalMappingDto[] = proj.io_mappings || [];
    const q = this.searchQuery.toLowerCase().trim();

    const filtered = mappings.filter((m) => {
      const matchesSearch =
        !q ||
        m.signal_key.toLowerCase().includes(q) ||
        m.channel_address.toLowerCase().includes(q) ||
        m.controller_id.toLowerCase().includes(q) ||
        (m.target_tag && m.target_tag.toLowerCase().includes(q));

      const matchesDir =
        this.filterDirection === "all" ||
        m.direction.toLowerCase() === this.filterDirection.toLowerCase();

      const matchesCtrl =
        this.filterController === "all" ||
        m.controller_id.toLowerCase() === this.filterController.toLowerCase();

      return matchesSearch && matchesDir && matchesCtrl;
    });

    const controllers = Array.from(new Set(mappings.map((m) => m.controller_id)));

    this.innerHTML = `
      <div class="io-mapping-container flex flex-col h-full bg-app text-main text-xs">
        <!-- Top Filter Bar -->
        <header class="p-3 bg-panel border-b border-color flex flex-wrap items-center justify-between gap-3">
          <div class="flex items-center gap-3">
            <span class="font-bold text-sm tracking-wide text-main">Hardware I/O Mappings</span>
            <span class="badge badge-info mono">${mappings.length} Configured</span>
            <span class="badge badge-dim mono">${filtered.length} Displayed</span>
          </div>

          <div class="flex items-center gap-2 flex-wrap">
            <input
              type="text"
              class="mapping-search px-2.5 py-1 bg-input border border-color rounded text-xs mono text-main w-64"
              placeholder="Search signal, address, tag..."
              value="${this.searchQuery}"
            />
            <select class="filter-dir px-2 py-1 bg-input border border-color rounded text-xs mono text-main">
              <option value="all" ${this.filterDirection === "all" ? "selected" : ""}>All Directions</option>
              <option value="input" ${this.filterDirection === "input" ? "selected" : ""}>Input (%I)</option>
              <option value="output" ${this.filterDirection === "output" ? "selected" : ""}>Output (%Q)</option>
            </select>
            <select class="filter-ctrl px-2 py-1 bg-input border border-color rounded text-xs mono text-main">
              <option value="all" ${this.filterController === "all" ? "selected" : ""}>All Controllers</option>
              ${controllers.map((c) => `<option value="${c}" ${this.filterController === c ? "selected" : ""}>${c}</option>`).join("")}
            </select>
          </div>
        </header>

        <!-- Mapping Grid Table -->
        <div class="flex-1 overflow-auto p-3">
          <table class="w-full text-left border-collapse">
            <thead>
              <tr class="border-b border-color text-muted font-mono text-xs uppercase bg-panel-header">
                <th class="p-2">Controller ID</th>
                <th class="p-2">Signal Key</th>
                <th class="p-2">Direction</th>
                <th class="p-2">Physical Channel Address</th>
                <th class="p-2">Bound Process Tag</th>
                <th class="p-2">Bound Actuator</th>
                <th class="p-2">Command Priority</th>
                <th class="p-2 text-right">Actions</th>
              </tr>
            </thead>
            <tbody>
              ${
                filtered.length === 0
                  ? `
                <tr>
                  <td colspan="8" class="p-6 text-center text-dim mono italic">
                    No matching I/O mappings found.
                  </td>
                </tr>
              `
                  : filtered
                      .map((m) => {
                        const dirBadge =
                          m.direction.toLowerCase() === "input" ? "badge-good" : "badge-info";
                        const prioLabel =
                          m.priority === 1
                            ? "1 (ESD)"
                            : m.priority === 2
                            ? "2 (Interlock)"
                            : m.priority === 3
                            ? "3 (Permissive)"
                            : m.priority === 4
                            ? "4 (Manual)"
                            : "5 (Auto)";

                        return `
                  <tr class="border-b border-color hover:bg-panel transition-colors">
                    <td class="p-2 font-mono font-bold text-main whitespace-nowrap">${m.controller_id}</td>
                    <td class="p-2 font-mono text-xs text-info font-medium whitespace-nowrap">${m.signal_key}</td>
                    <td class="p-2 whitespace-nowrap">
                      <span class="badge ${dirBadge} text-xs">${m.direction}</span>
                    </td>
                    <td class="p-2 font-mono text-xs text-amber font-semibold whitespace-nowrap">${m.channel_address}</td>
                    <td class="p-2 font-mono text-xs text-main whitespace-nowrap">${m.target_tag || "—"}</td>
                    <td class="p-2 font-mono text-xs text-dim whitespace-nowrap">${m.target_actuator || "—"}</td>
                    <td class="p-2 font-mono text-xs text-muted whitespace-nowrap">${m.priority !== undefined ? prioLabel : "Default"}</td>
                    <td class="p-2 text-right whitespace-nowrap">
                      <button class="btn-xref px-2 py-0.5 text-xs text-info hover:text-white" data-target="${m.target_tag || m.signal_key}">
                        🔍 Trace
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
    const searchInput = this.querySelector(".mapping-search") as HTMLInputElement;
    searchInput?.addEventListener("input", () => {
      this.searchQuery = searchInput.value;
      this.render();
    });

    const filterDir = this.querySelector(".filter-dir") as HTMLSelectElement;
    filterDir?.addEventListener("change", () => {
      this.filterDirection = filterDir.value;
      this.render();
    });

    const filterCtrl = this.querySelector(".filter-ctrl") as HTMLSelectElement;
    filterCtrl?.addEventListener("change", () => {
      this.filterController = filterCtrl.value;
      this.render();
    });

    this.querySelectorAll(".btn-xref").forEach((btn) => {
      btn.addEventListener("click", () => {
        const target = btn.getAttribute("data-target");
        if (target) {
          this.openCrossRef(target);
        }
      });
    });
  }
}

customElements.define("nebu-io-mapping-table", NebuIoMappingTable);
