/**
 * NebuAdvancedControl Web Component.
 *
 * Implements Stage 5 Advanced Control System Engineering & Runtime Management:
 * - Cascade Control (Primary / Secondary Master-Slave loops)
 * - Ratio Control (Wild stream PV tracking with bias & bounds)
 * - Feedforward Disturbance Rejection (Lead-Lag dynamic compensation)
 * - Override / Selective Constraint Control (High/Low auctioneering with external reset feedback)
 * - Split-Range Control (Multi-valve sequenced actuation)
 * - Anti-Surge Control (SLL/SCL operating envelope, PI recycle demand & derivative kick)
 * - Valve Characterization (Linear, Equal Percentage, Quick Opening curves)
 * - Authoritative synchronization with Rust simulation host
 */

import { studioStore } from "../state/studio-store";
import { sharedApiClient } from "../api/client";
import { AdvancedControlStrategyDto, ProjectDetailDto } from "../protocol/types";

export class NebuAdvancedControl extends HTMLElement {
  private searchQuery = "";
  private filterType = "ALL";
  private selectedStrategyId: string | null = null;
  private isEditing = false;
  private editStrategy: AdvancedControlStrategyDto | null = null;
  private isDeploying = false;
  private statusMessage = "";
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

  private setStatus(msg: string) {
    this.statusMessage = msg;
    const msgEl = this.querySelector(".status-feedback");
    if (msgEl) msgEl.textContent = msg;
  }

  private getTypeBadgeClass(type: string): string {
    switch (type.toUpperCase()) {
      case "CASCADE":
        return "badge-info";
      case "RATIO":
        return "badge-good";
      case "FEEDFORWARD":
        return "bg-amber-900/60 text-amber-300 border border-amber-600/50";
      case "OVERRIDE":
        return "bg-purple-900/60 text-purple-300 border border-purple-600/50";
      case "SPLITRANGE":
        return "bg-pink-900/60 text-pink-300 border border-pink-600/50";
      case "ANTISURGE":
        return "badge-alarm";
      case "VALVECHARACTERIZATION":
      case "VALVE_CHAR":
        return "bg-cyan-900/60 text-cyan-300 border border-cyan-600/50";
      default:
        return "badge-dim";
    }
  }

  private createDefaultParameters(type: string): Record<string, string> {
    switch (type.toUpperCase()) {
      case "CASCADE":
        return {
          primary_pv: "LIC_101_PV",
          secondary_pv: "FIC_101_PV",
          primary_sp: "50.0",
          primary_kp: "1.5",
          primary_ti: "10.0",
          primary_td: "0.0",
          secondary_kp: "1.0",
          secondary_ti: "2.0",
          secondary_td: "0.1",
          secondary_sp_min: "0.0",
          secondary_sp_max: "100.0",
          output_min: "0.0",
          output_max: "100.0",
          target_actuator: "FV-101",
          output_tag: "FIC_101_OUT",
        };
      case "RATIO":
        return {
          wild_pv: "FIC_201_PV",
          controlled_pv: "FIC_202_PV",
          ratio: "0.5",
          bias: "0.0",
          ratio_min: "0.1",
          ratio_max: "5.0",
          kp: "1.2",
          ti: "5.0",
          td: "0.0",
          output_min: "0.0",
          output_max: "100.0",
          target_actuator: "FV-202",
          output_tag: "FIC_202_OUT",
        };
      case "FEEDFORWARD":
        return {
          pv_tag: "TIC_301_PV",
          sp: "75.0",
          disturbance_tag: "FIC_301_DIST",
          t_lead: "4.0",
          t_lag: "2.0",
          gain: "0.8",
          kp: "1.5",
          ti: "6.0",
          td: "0.2",
          output_min: "0.0",
          output_max: "100.0",
          target_actuator: "TV-301",
          output_tag: "TIC_301_OUT",
        };
      case "OVERRIDE":
        return {
          primary_pv: "FIC_401_PV",
          primary_sp: "60.0",
          primary_kp: "1.0",
          primary_ti: "5.0",
          constraint_pv: "PIC_401_PV",
          constraint_limit: "85.0",
          constraint_kp: "2.0",
          constraint_ti: "3.0",
          constraint_direction: "reverse",
          selector_type: "low",
          output_min: "0.0",
          output_max: "100.0",
          target_actuator: "FV-401",
          output_tag: "SEL_401_OUT",
        };
      case "SPLITRANGE":
        return {
          pv_tag: "PIC_501_PV",
          sp: "50.0",
          kp: "2.0",
          ti: "4.0",
          split_a_name: "CoolingValve",
          split_b_name: "HeatingValve",
          split_a_target_actuator: "TV-501A",
          split_b_target_actuator: "TV-501B",
          split_a_output_tag: "TV_501A_OUT",
          split_b_output_tag: "TV_501B_OUT",
        };
      case "ANTISURGE":
        return {
          flow_tag: "FT_601_FLOW",
          p_suction_tag: "PT_601_SUCTION",
          p_discharge_tag: "PT_601_DISCHARGE",
          sll_slope_a: "0.35",
          sll_intercept_b: "0.50",
          safety_margin: "0.10",
          kp: "2.0",
          ti: "1.0",
          derivative_kick_gain: "1.5",
          kick_threshold_rate: "2.0",
          target_actuator: "FV-601-RECYCLE",
          recycle_valve_tag: "ASV_601_OUT",
        };
      case "VALVECHARACTERIZATION":
        return {
          input_tag: "CV_701_IN",
          characteristic: "EqualPercentage",
          rangeability: "50.0",
          target_actuator: "FV-701",
          output_tag: "CV_701_OUT",
        };
      default:
        return {};
    }
  }

  private handleAddNew() {
    const defaultType = "Cascade";
    this.editStrategy = {
      id: `strat-${Date.now()}`,
      name: `New ${defaultType} Strategy`,
      strategy_type: defaultType,
      description: `Advanced ${defaultType} control loop`,
      enabled: true,
      parameters: this.createDefaultParameters(defaultType),
    };
    this.isEditing = true;
    this.selectedStrategyId = this.editStrategy.id;
    this.render();
  }

  private handleEdit(strat: AdvancedControlStrategyDto) {
    this.editStrategy = JSON.parse(JSON.stringify(strat));
    this.isEditing = true;
    this.selectedStrategyId = strat.id;
    this.render();
  }

  private handleDelete(id: string) {
    if (confirm(`Delete advanced control strategy '${id}'?`)) {
      studioStore.deleteAdvancedControl(id);
      if (this.selectedStrategyId === id) {
        this.selectedStrategyId = null;
        this.isEditing = false;
        this.editStrategy = null;
      }
      this.setStatus(`Deleted strategy ${id}`);
      this.render();
    }
  }

  private handleSaveEdit() {
    if (!this.editStrategy) return;
    const proj = studioStore.getProject();
    if (!proj) return;

    const existing = (proj.advanced_control || []).find((s) => s.id === this.editStrategy!.id);
    if (existing) {
      studioStore.updateAdvancedControl(this.editStrategy.id, this.editStrategy);
      this.setStatus(`Updated strategy ${this.editStrategy.name}`);
    } else {
      studioStore.addAdvancedControl(this.editStrategy);
      this.setStatus(`Added strategy ${this.editStrategy.name}`);
    }
    this.isEditing = false;
    this.editStrategy = null;
    this.render();
  }

  private async handleDeployToSim() {
    const proj = studioStore.getProject();
    if (!proj) return;

    this.isDeploying = true;
    this.setStatus("Deploying advanced control strategies into simulation host...");
    this.render();

    try {
      // First save project to persist strategies
      await sharedApiClient.saveProject(proj.id, {
        project: proj,
      });
      // Then sync to running simulation
      const res = await sharedApiClient.loadProjectSim(proj.id);
      this.setStatus(`Successfully deployed into simulation: ${res.message}`);
    } catch (err: any) {
      this.setStatus(`Deployment failed: ${err.message || err}`);
    } finally {
      this.isDeploying = false;
      this.render();
    }
  }

  render() {
    const proj: ProjectDetailDto | null = studioStore.getProject();
    if (!proj) {
      this.innerHTML = `
        <div class="p-6 text-center text-dim mono">
          No project loaded. Please open a project to configure advanced control.
        </div>
      `;
      return;
    }

    const strategies: AdvancedControlStrategyDto[] = proj.advanced_control || [];
    const q = this.searchQuery.toLowerCase().trim();

    const filtered = strategies.filter((s) => {
      const matchesType =
        this.filterType === "ALL" ||
        s.strategy_type.toUpperCase() === this.filterType.toUpperCase();
      const matchesQuery =
        !q ||
        s.name.toLowerCase().includes(q) ||
        s.id.toLowerCase().includes(q) ||
        s.strategy_type.toLowerCase().includes(q) ||
        Object.values(s.parameters || {}).some((v) =>
          v.toLowerCase().includes(q)
        );
      return matchesType && matchesQuery;
    });

    this.innerHTML = `
      <div class="advanced-control-container flex flex-col h-full bg-[#090d16] text-[#e2e8f0] text-xs font-sans">
        <!-- Top Toolbar -->
        <header class="p-3 bg-[#0f172a] border-b border-[#1e293b] flex flex-wrap items-center justify-between gap-3">
          <div class="flex items-center gap-3">
            <span class="font-bold text-sm tracking-wide text-[#38bdf8] flex items-center gap-1.5">
              <span>🎯</span> Advanced Process Control (APC)
            </span>
            <span class="badge badge-info mono">${strategies.length} Strategies</span>
            <span class="badge badge-dim mono">${filtered.length} Filtered</span>
            <span class="status-feedback text-amber mono text-xs ml-2">${this.statusMessage}</span>
          </div>

          <div class="flex items-center gap-2 flex-wrap">
            <input
              type="text"
              class="strategy-search px-2.5 py-1 bg-[#1e293b] border border-[#334155] rounded text-xs mono text-white w-48 placeholder-slate-400"
              placeholder="Filter by name, tag, type..."
              value="${this.searchQuery}"
            />
            <select class="filter-type-select px-2 py-1 bg-[#1e293b] border border-[#334155] rounded text-xs mono text-white">
              <option value="ALL" ${this.filterType === "ALL" ? "selected" : ""}>All APC Types</option>
              <option value="CASCADE" ${this.filterType === "CASCADE" ? "selected" : ""}>Cascade</option>
              <option value="RATIO" ${this.filterType === "RATIO" ? "selected" : ""}>Ratio</option>
              <option value="FEEDFORWARD" ${this.filterType === "FEEDFORWARD" ? "selected" : ""}>Feedforward</option>
              <option value="OVERRIDE" ${this.filterType === "OVERRIDE" ? "selected" : ""}>Override / Constraint</option>
              <option value="SPLITRANGE" ${this.filterType === "SPLITRANGE" ? "selected" : ""}>Split-Range</option>
              <option value="ANTISURGE" ${this.filterType === "ANTISURGE" ? "selected" : ""}>Anti-Surge</option>
              <option value="VALVECHARACTERIZATION" ${this.filterType === "VALVECHARACTERIZATION" ? "selected" : ""}>Valve Characterization</option>
            </select>
            <button class="btn-add-strategy px-3 py-1 bg-[#0284c7] hover:bg-[#0369a1] text-white font-medium rounded shadow cursor-pointer">
              + New Strategy
            </button>
            <button class="btn-deploy-sim px-3 py-1 bg-[#059669] hover:bg-[#047857] text-white font-medium rounded shadow cursor-pointer ${this.isDeploying ? "opacity-50 pointer-events-none" : ""}">
              ${this.isDeploying ? "Deploying..." : "▶ Deploy to Simulation"}
            </button>
          </div>
        </header>

        <!-- Main Content Area: Split List & Editor -->
        <div class="flex-1 flex overflow-hidden">
          <!-- Left: Strategy List Table -->
          <div class="flex-1 flex flex-col overflow-auto p-3 border-r border-[#1e293b]">
            <table class="w-full text-left border-collapse">
              <thead>
                <tr class="border-b border-[#1e293b] text-slate-400 font-mono text-xs uppercase bg-[#0f172a]">
                  <th class="p-2">Status</th>
                  <th class="p-2">Strategy Name</th>
                  <th class="p-2">Type</th>
                  <th class="p-2">Key Tags / Actuators</th>
                  <th class="p-2">Parameters Summary</th>
                  <th class="p-2 text-right">Actions</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-[#1e293b] font-mono text-xs">
                ${
                  filtered.length === 0
                    ? `
                  <tr>
                    <td colspan="6" class="p-6 text-center text-slate-500 italic">
                      No advanced control strategies configured. Click "+ New Strategy" to author APC loops.
                    </td>
                  </tr>
                `
                    : filtered
                        .map((s) => {
                          const isSelected = this.selectedStrategyId === s.id;
                          const badgeClass = this.getTypeBadgeClass(s.strategy_type);

                          let keyInfo = "";
                          let paramInfo = "";

                          switch (s.strategy_type.toUpperCase()) {
                            case "CASCADE":
                              keyInfo = `Prim: ${s.parameters.primary_pv || "-"} → Sec: ${s.parameters.secondary_pv || "-"} | Act: ${s.parameters.target_actuator || "-"}`;
                              paramInfo = `Pri Kp=${s.parameters.primary_kp || "1.5"} Ti=${s.parameters.primary_ti || "10"} | Sec Kp=${s.parameters.secondary_kp || "1.0"}`;
                              break;
                            case "RATIO":
                              keyInfo = `Wild: ${s.parameters.wild_pv || "-"} : Ctrl: ${s.parameters.controlled_pv || "-"} | Act: ${s.parameters.target_actuator || "-"}`;
                              paramInfo = `Ratio=${s.parameters.ratio || "1.0"} Bias=${s.parameters.bias || "0.0"} (Limits: ${s.parameters.ratio_min || "0.1"}-${s.parameters.ratio_max || "5.0"})`;
                              break;
                            case "FEEDFORWARD":
                              keyInfo = `PV: ${s.parameters.pv_tag || "-"} + Dist: ${s.parameters.disturbance_tag || "-"} | Act: ${s.parameters.target_actuator || "-"}`;
                              paramInfo = `Lead=${s.parameters.t_lead || "2"}s Lag=${s.parameters.t_lag || "1"}s Gain=${s.parameters.gain || "1.0"}`;
                              break;
                            case "OVERRIDE":
                              keyInfo = `Prim: ${s.parameters.primary_pv || "-"} vs Constr: ${s.parameters.constraint_pv || "-"} (${s.parameters.selector_type || "Low"})`;
                              paramInfo = `Limit=${s.parameters.constraint_limit || "80"} Kp=${s.parameters.constraint_kp || "2.0"} Ext-Reset Active`;
                              break;
                            case "SPLITRANGE":
                              keyInfo = `PV: ${s.parameters.pv_tag || "-"} → Split A: ${s.parameters.split_a_target_actuator || "-"} / Split B: ${s.parameters.split_b_target_actuator || "-"}`;
                              paramInfo = `0-50% (${s.parameters.split_a_name || "A"}) / 50-100% (${s.parameters.split_b_name || "B"})`;
                              break;
                            case "ANTISURGE":
                              keyInfo = `Flow: ${s.parameters.flow_tag || "-"} Ps: ${s.parameters.p_suction_tag || "-"} Pd: ${s.parameters.p_discharge_tag || "-"} → ${s.parameters.target_actuator || "-"}`;
                              paramInfo = `Margin=${(parseFloat(s.parameters.safety_margin || "0.1") * 100).toFixed(0)}% KickGain=${s.parameters.derivative_kick_gain || "1.5"}`;
                              break;
                            case "VALVECHARACTERIZATION":
                              keyInfo = `In: ${s.parameters.input_tag || "-"} → Act: ${s.parameters.target_actuator || "-"}`;
                              paramInfo = `Curve=${s.parameters.characteristic || "Linear"} R=${s.parameters.rangeability || "50"}`;
                              break;
                            default:
                              keyInfo = Object.entries(s.parameters || {})
                                .slice(0, 2)
                                .map(([k, v]) => `${k}:${v}`)
                                .join(" ");
                          }

                          return `
                        <tr class="hover:bg-[#1e293b]/60 ${isSelected ? "bg-[#1e293b]/80" : ""}">
                          <td class="p-2">
                            <span class="inline-block w-2.5 h-2.5 rounded-full ${s.enabled ? "bg-emerald-400" : "bg-slate-600"}" title="${s.enabled ? "Active" : "Disabled"}"></span>
                          </td>
                          <td class="p-2 font-semibold text-white">
                            <div>${s.name}</div>
                            <div class="text-[10px] text-slate-400">${s.id}</div>
                          </td>
                          <td class="p-2">
                            <span class="px-2 py-0.5 rounded text-[10px] font-bold ${badgeClass}">
                              ${s.strategy_type}
                            </span>
                          </td>
                          <td class="p-2 text-slate-300">${keyInfo}</td>
                          <td class="p-2 text-slate-400 text-[11px]">${paramInfo}</td>
                          <td class="p-2 text-right space-x-1">
                            <button class="btn-toggle-strat px-2 py-0.5 rounded bg-[#334155] hover:bg-[#475569] text-[10px] text-white cursor-pointer" data-id="${s.id}">
                              ${s.enabled ? "Disable" : "Enable"}
                            </button>
                            <button class="btn-edit-strat px-2 py-0.5 rounded bg-[#0284c7] hover:bg-[#0369a1] text-[10px] text-white cursor-pointer" data-id="${s.id}">
                              Edit
                            </button>
                            <button class="btn-del-strat px-2 py-0.5 rounded bg-red-900/60 hover:bg-red-800 text-[10px] text-red-200 cursor-pointer" data-id="${s.id}">
                              Del
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

          <!-- Right: Strategy Authoring / Editing Drawer -->
          ${
            this.isEditing && this.editStrategy
              ? `
            <div class="w-96 bg-[#0b1324] border-l border-[#1e293b] p-4 flex flex-col overflow-y-auto">
              <div class="flex items-center justify-between pb-3 mb-3 border-b border-[#1e293b]">
                <h3 class="font-bold text-sm text-white flex items-center gap-1.5">
                  <span>⚙️</span> Edit Strategy
                </h3>
                <button class="btn-close-edit text-slate-400 hover:text-white font-mono text-lg cursor-pointer">×</button>
              </div>

              <div class="space-y-3 flex-1">
                <div>
                  <label class="block text-slate-400 text-[10px] uppercase font-mono mb-1">Strategy ID</label>
                  <input type="text" class="edit-id w-full px-2 py-1 bg-[#1e293b] border border-[#334155] rounded text-white mono text-xs" value="${this.editStrategy.id}" readonly />
                </div>

                <div>
                  <label class="block text-slate-400 text-[10px] uppercase font-mono mb-1">Strategy Name</label>
                  <input type="text" class="edit-name w-full px-2 py-1 bg-[#1e293b] border border-[#334155] rounded text-white mono text-xs" value="${this.editStrategy.name}" />
                </div>

                <div>
                  <label class="block text-slate-400 text-[10px] uppercase font-mono mb-1">Strategy Type</label>
                  <select class="edit-type w-full px-2 py-1 bg-[#1e293b] border border-[#334155] rounded text-white mono text-xs">
                    <option value="Cascade" ${this.editStrategy.strategy_type === "Cascade" ? "selected" : ""}>Cascade</option>
                    <option value="Ratio" ${this.editStrategy.strategy_type === "Ratio" ? "selected" : ""}>Ratio</option>
                    <option value="FeedForward" ${this.editStrategy.strategy_type === "FeedForward" ? "selected" : ""}>FeedForward</option>
                    <option value="Override" ${this.editStrategy.strategy_type === "Override" ? "selected" : ""}>Override</option>
                    <option value="SplitRange" ${this.editStrategy.strategy_type === "SplitRange" ? "selected" : ""}>SplitRange</option>
                    <option value="AntiSurge" ${this.editStrategy.strategy_type === "AntiSurge" ? "selected" : ""}>AntiSurge</option>
                    <option value="ValveCharacterization" ${this.editStrategy.strategy_type === "ValveCharacterization" ? "selected" : ""}>ValveCharacterization</option>
                  </select>
                </div>

                <div>
                  <label class="block text-slate-400 text-[10px] uppercase font-mono mb-1">Description</label>
                  <input type="text" class="edit-desc w-full px-2 py-1 bg-[#1e293b] border border-[#334155] rounded text-white mono text-xs" value="${this.editStrategy.description}" />
                </div>

                <div class="flex items-center gap-2 py-1">
                  <input type="checkbox" id="edit-enabled" class="edit-enabled rounded bg-[#1e293b] border-[#334155]" ${this.editStrategy.enabled ? "checked" : ""} />
                  <label for="edit-enabled" class="text-white text-xs select-none">Enabled</label>
                </div>

                <!-- Dynamic Parameter Inputs -->
                <div class="pt-2 border-t border-[#1e293b]">
                  <div class="font-bold text-xs text-slate-300 mb-2">Strategy Parameters</div>
                  <div class="space-y-2">
                    ${Object.entries(this.editStrategy.parameters || {})
                      .map(
                        ([k, v]) => `
                      <div>
                        <label class="block text-slate-400 text-[10px] mono mb-0.5">${k}</label>
                        <input
                          type="text"
                          class="edit-param-field w-full px-2 py-1 bg-[#1e293b] border border-[#334155] rounded text-white mono text-xs"
                          data-key="${k}"
                          value="${v}"
                        />
                      </div>
                    `
                      )
                      .join("")}
                  </div>
                </div>
              </div>

              <div class="pt-3 border-t border-[#1e293b] flex gap-2 justify-end">
                <button class="btn-cancel-edit px-3 py-1 bg-[#334155] hover:bg-[#475569] text-white rounded text-xs cursor-pointer">
                  Cancel
                </button>
                <button class="btn-save-edit px-3 py-1 bg-[#0284c7] hover:bg-[#0369a1] text-white rounded text-xs font-semibold cursor-pointer">
                  Save Changes
                </button>
              </div>
            </div>
          `
              : ""
          }
        </div>
      </div>
    `;

    this.bindEvents();
  }

  private bindEvents() {
    // Search
    const searchEl = this.querySelector(".strategy-search") as HTMLInputElement;
    searchEl?.addEventListener("input", () => {
      this.searchQuery = searchEl.value;
      this.render();
    });

    // Filter Type
    const filterSelect = this.querySelector(".filter-type-select") as HTMLSelectElement;
    filterSelect?.addEventListener("change", () => {
      this.filterType = filterSelect.value;
      this.render();
    });

    // Add New
    this.querySelector(".btn-add-strategy")?.addEventListener("click", () => {
      this.handleAddNew();
    });

    // Deploy to Sim
    this.querySelector(".btn-deploy-sim")?.addEventListener("click", () => {
      this.handleDeployToSim();
    });

    // Toggle Enabled
    this.querySelectorAll(".btn-toggle-strat").forEach((btn) => {
      btn.addEventListener("click", () => {
        const id = btn.getAttribute("data-id");
        if (!id) return;
        const proj = studioStore.getProject();
        const strat = (proj?.advanced_control || []).find((s) => s.id === id);
        if (strat) {
          studioStore.updateAdvancedControl(id, { enabled: !strat.enabled });
          this.setStatus(`Strategy ${strat.name} ${!strat.enabled ? "enabled" : "disabled"}`);
          this.render();
        }
      });
    });

    // Edit Strategy
    this.querySelectorAll(".btn-edit-strat").forEach((btn) => {
      btn.addEventListener("click", () => {
        const id = btn.getAttribute("data-id");
        if (!id) return;
        const proj = studioStore.getProject();
        const strat = (proj?.advanced_control || []).find((s) => s.id === id);
        if (strat) {
          this.handleEdit(strat);
        }
      });
    });

    // Delete Strategy
    this.querySelectorAll(".btn-del-strat").forEach((btn) => {
      btn.addEventListener("click", () => {
        const id = btn.getAttribute("data-id");
        if (id) {
          this.handleDelete(id);
        }
      });
    });

    // Edit Form Listeners
    if (this.isEditing && this.editStrategy) {
      this.querySelector(".btn-close-edit")?.addEventListener("click", () => {
        this.isEditing = false;
        this.editStrategy = null;
        this.render();
      });

      this.querySelector(".btn-cancel-edit")?.addEventListener("click", () => {
        this.isEditing = false;
        this.editStrategy = null;
        this.render();
      });

      const nameInput = this.querySelector(".edit-name") as HTMLInputElement;
      nameInput?.addEventListener("input", () => {
        if (this.editStrategy) this.editStrategy.name = nameInput.value;
      });

      const descInput = this.querySelector(".edit-desc") as HTMLInputElement;
      descInput?.addEventListener("input", () => {
        if (this.editStrategy) this.editStrategy.description = descInput.value;
      });

      const typeSelect = this.querySelector(".edit-type") as HTMLSelectElement;
      typeSelect?.addEventListener("change", () => {
        if (this.editStrategy) {
          this.editStrategy.strategy_type = typeSelect.value;
          this.editStrategy.parameters = this.createDefaultParameters(typeSelect.value);
          this.render();
        }
      });

      const enabledCheckbox = this.querySelector(".edit-enabled") as HTMLInputElement;
      enabledCheckbox?.addEventListener("change", () => {
        if (this.editStrategy) this.editStrategy.enabled = enabledCheckbox.checked;
      });

      this.querySelectorAll(".edit-param-field").forEach((field) => {
        field.addEventListener("input", () => {
          const input = field as HTMLInputElement;
          const key = input.getAttribute("data-key");
          if (key && this.editStrategy) {
            this.editStrategy.parameters[key] = input.value;
          }
        });
      });

      this.querySelector(".btn-save-edit")?.addEventListener("click", () => {
        this.handleSaveEdit();
      });
    }
  }
}

customElements.define("nebu-advanced-control", NebuAdvancedControl);
