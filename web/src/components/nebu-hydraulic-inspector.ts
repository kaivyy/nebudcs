/**
 * NebuHydraulicInspector Web Component.
 *
 * Implements Stage 6 Hydraulic Network Engineering & Telemetry Inspection:
 * - Real-time Nodal Pressure & Mass Conservation Diagnostics
 * - Branch Darcy-Weisbach Friction, Velocities & Reynolds Numbers
 * - Flow Regime Classifications (Laminar, Transitional, Turbulent)
 * - Convergence KPIs, Residual Norms & Offending Component Isolation
 */

import { sharedApiClient } from "../api/client";
import { NetworkDiagnosticsDto } from "../protocol/types";

export class NebuHydraulicInspector extends HTMLElement {
  private diagnostics: NetworkDiagnosticsDto | null = null;
  private isLoading = false;
  private autoRefresh = true;
  private timer: number | null = null;
  private filter = "ALL";
  private searchQuery = "";
  private statusMessage = "";

  connectedCallback() {
    this.fetchDiagnostics();
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
        this.fetchDiagnostics(true);
      }
    }, 1500);
  }

  private stopPolling() {
    if (this.timer !== null) {
      window.clearInterval(this.timer);
      this.timer = null;
    }
  }

  private async fetchDiagnostics(silent = false) {
    if (!silent) this.isLoading = true;
    try {
      this.diagnostics = await sharedApiClient.getNetworkDiagnostics();
      this.statusMessage = `Updated at ${new Date().toLocaleTimeString()}`;
    } catch (err: any) {
      console.error("[nebudcs-hydraulic-inspector] Fetch error:", err);
      this.statusMessage = `Error: ${err.message || "Failed to fetch diagnostics"}`;
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private getRegimeBadge(regime: string): string {
    switch (regime.toLowerCase()) {
      case "laminar":
        return `<span class="badge bg-blue-900/60 text-blue-300 border border-blue-600/50">Laminar</span>`;
      case "transitional":
        return `<span class="badge bg-amber-900/60 text-amber-300 border border-amber-600/50">Transitional</span>`;
      case "turbulent":
        return `<span class="badge bg-purple-900/60 text-purple-300 border border-purple-600/50">Turbulent</span>`;
      default:
        return `<span class="badge bg-slate-800 text-slate-300">${regime}</span>`;
    }
  }

  render() {
    const d = this.diagnostics;
    const isConverged = d?.converged ?? false;
    const nodes = d?.nodes || [];
    const edges = d?.edges || [];

    const filteredEdges = edges.filter((e) => {
      if (this.filter === "LAMINAR" && e.flow_regime.toLowerCase() !== "laminar") return false;
      if (this.filter === "TURBULENT" && e.flow_regime.toLowerCase() !== "turbulent") return false;
      if (this.filter === "TRANSITIONAL" && e.flow_regime.toLowerCase() !== "transitional") return false;
      if (this.searchQuery) {
        const q = this.searchQuery.toLowerCase();
        return (
          e.id.toLowerCase().includes(q) ||
          e.name.toLowerCase().includes(q) ||
          e.from_node.toLowerCase().includes(q) ||
          e.to_node.toLowerCase().includes(q)
        );
      }
      return true;
    });

    const filteredNodes = nodes.filter((n) => {
      if (this.searchQuery) {
        const q = this.searchQuery.toLowerCase();
        return n.id.toLowerCase().includes(q) || n.name.toLowerCase().includes(q);
      }
      return true;
    });

    this.innerHTML = `
      <div class="h-full flex flex-col bg-[#0b0f17] text-slate-200 overflow-hidden font-sans">
        <!-- Top Title & Action Bar -->
        <div class="p-4 border-b border-[#1e293b] flex flex-wrap items-center justify-between gap-3 bg-[#0d131f]">
          <div>
            <div class="flex items-center gap-3">
              <h2 class="text-base font-bold tracking-wide text-white flex items-center gap-2">
                <span class="text-cyan-400">🌊</span> Hydraulic Network Solver Diagnostics
              </h2>
              <span class="badge ${isConverged ? "bg-emerald-950 text-emerald-400 border border-emerald-700" : "bg-rose-950 text-rose-400 border border-rose-700"}">
                ${isConverged ? "● CONVERGED" : "⚠ NON-CONVERGENT"}
              </span>
            </div>
            <p class="text-xs text-slate-400 mt-0.5">
              Nodal pressure formulation with Darcy-Weisbach friction, Churchill factor & Wegstein recycle closure
            </p>
          </div>

          <div class="flex items-center gap-2">
            <label class="flex items-center gap-1.5 text-xs text-slate-300 cursor-pointer bg-[#151d2d] px-2.5 py-1 rounded border border-[#243048]">
              <input type="checkbox" id="chkAutoRefresh" ${this.autoRefresh ? "checked" : ""} class="cursor-pointer">
              Auto-poll (1.5s)
            </label>
            <button id="btnRefresh" class="px-3 py-1 bg-cyan-700 hover:bg-cyan-600 text-white text-xs font-semibold rounded shadow transition flex items-center gap-1.5">
              <span>↻</span> Refresh
            </button>
          </div>
        </div>

        <!-- KPI Cards Bar -->
        <div class="grid grid-cols-2 md:grid-cols-5 gap-3 p-4 border-b border-[#1e293b] bg-[#0e1626]">
          <div class="bg-[#141d30] border border-[#22314e] p-3 rounded shadow-sm">
            <div class="text-[11px] font-semibold text-slate-400 uppercase tracking-wider">Solver Status</div>
            <div class="text-lg font-bold mt-1 ${isConverged ? "text-emerald-400" : "text-rose-400"}">
              ${isConverged ? "Converged" : "Diverged"}
            </div>
            <div class="text-[11px] text-slate-400 mt-0.5">${d?.iterations ?? 0} Newton iters</div>
          </div>

          <div class="bg-[#141d30] border border-[#22314e] p-3 rounded shadow-sm">
            <div class="text-[11px] font-semibold text-slate-400 uppercase tracking-wider">Max Residual</div>
            <div class="text-lg font-bold mt-1 text-cyan-300 mono">
              ${d ? d.max_residual.toExponential(3) : "0.000e+0"}
            </div>
            <div class="text-[11px] text-slate-400 mt-0.5">kg/s mass imbalance</div>
          </div>

          <div class="bg-[#141d30] border border-[#22314e] p-3 rounded shadow-sm">
            <div class="text-[11px] font-semibold text-slate-400 uppercase tracking-wider">Residual L2 Norm</div>
            <div class="text-lg font-bold mt-1 text-sky-300 mono">
              ${d ? d.residual_norm.toExponential(3) : "0.000e+0"}
            </div>
            <div class="text-[11px] text-slate-400 mt-0.5">Euclidean norm</div>
          </div>

          <div class="bg-[#141d30] border border-[#22314e] p-3 rounded shadow-sm">
            <div class="text-[11px] font-semibold text-slate-400 uppercase tracking-wider">Network Nodes</div>
            <div class="text-lg font-bold mt-1 text-indigo-300 mono">
              ${nodes.length}
            </div>
            <div class="text-[11px] text-slate-400 mt-0.5">Pressures & Elevations</div>
          </div>

          <div class="bg-[#141d30] border border-[#22314e] p-3 rounded shadow-sm">
            <div class="text-[11px] font-semibold text-slate-400 uppercase tracking-wider">Piping Branches</div>
            <div class="text-lg font-bold mt-1 text-purple-300 mono">
              ${edges.length}
            </div>
            <div class="text-[11px] text-slate-400 mt-0.5">Pipes, Valves & Pumps</div>
          </div>
        </div>

        <!-- Filter & Search Controls -->
        <div class="px-4 py-2 bg-[#0d131f] border-b border-[#1e293b] flex flex-wrap items-center justify-between gap-2">
          <div class="flex items-center gap-2">
            <input type="text" id="searchInput" placeholder="Search node or branch ID..." value="${this.searchQuery}"
              class="px-2.5 py-1 text-xs bg-[#151d2d] border border-[#22314e] rounded text-slate-200 placeholder-slate-500 focus:outline-none focus:border-cyan-500 w-56">
            <div class="flex items-center bg-[#151d2d] border border-[#22314e] rounded p-0.5 text-xs">
              <button class="filter-btn px-2.5 py-0.5 rounded ${this.filter === "ALL" ? "bg-cyan-700 text-white font-semibold" : "text-slate-400"}" data-filter="ALL">All Regimes</button>
              <button class="filter-btn px-2.5 py-0.5 rounded ${this.filter === "TURBULENT" ? "bg-cyan-700 text-white font-semibold" : "text-slate-400"}" data-filter="TURBULENT">Turbulent</button>
              <button class="filter-btn px-2.5 py-0.5 rounded ${this.filter === "TRANSITIONAL" ? "bg-cyan-700 text-white font-semibold" : "text-slate-400"}" data-filter="TRANSITIONAL">Transitional</button>
              <button class="filter-btn px-2.5 py-0.5 rounded ${this.filter === "LAMINAR" ? "bg-cyan-700 text-white font-semibold" : "text-slate-400"}" data-filter="LAMINAR">Laminar</button>
            </div>
          </div>
          <span class="text-[11px] text-slate-400">${this.statusMessage}</span>
        </div>

        <!-- Tables Split View -->
        <div class="flex-1 overflow-y-auto p-4 space-y-6">
          <!-- Hydraulic Edges / Branches Table -->
          <div class="bg-[#101726] border border-[#1e293b] rounded shadow-sm overflow-hidden">
            <div class="px-4 py-2.5 bg-[#141e30] border-b border-[#1e293b] flex items-center justify-between">
              <h3 class="text-xs font-bold uppercase tracking-wider text-cyan-300 flex items-center gap-1.5">
                <span>⚡</span> Piping Branches & Equipment Edges (${filteredEdges.length})
              </h3>
            </div>
            <div class="overflow-x-auto">
              <table class="w-full text-xs text-left text-slate-300">
                <thead class="bg-[#0b101b] text-slate-400 uppercase font-semibold border-b border-[#1e293b]">
                  <tr>
                    <th class="p-2.5">Branch ID / Name</th>
                    <th class="p-2.5">Topology (From → To)</th>
                    <th class="p-2.5 text-right">Mass Flow (kg/s)</th>
                    <th class="p-2.5 text-right">Vol Flow (m³/h)</th>
                    <th class="p-2.5 text-right">ΔP (kPa)</th>
                    <th class="p-2.5 text-right">Velocity (m/s)</th>
                    <th class="p-2.5 text-right">Reynolds (Re)</th>
                    <th class="p-2.5 text-right">Friction (f)</th>
                    <th class="p-2.5 text-center">Regime</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-[#1e293b]">
                  ${
                    filteredEdges.length === 0
                      ? `<tr><td colspan="9" class="p-4 text-center text-slate-500">No hydraulic edges match filter or no active network loaded.</td></tr>`
                      : filteredEdges
                          .map(
                            (e) => `
                        <tr class="hover:bg-[#152033] transition">
                          <td class="p-2.5 font-medium text-white">${e.id} <span class="text-slate-400 font-normal">(${e.name})</span></td>
                          <td class="p-2.5 mono text-slate-300">${e.from_node} → ${e.to_node}</td>
                          <td class="p-2.5 text-right mono font-semibold text-cyan-300">${e.mass_flow_kg_s.toFixed(2)}</td>
                          <td class="p-2.5 text-right mono text-slate-300">${e.volumetric_flow_m3_h.toFixed(2)}</td>
                          <td class="p-2.5 text-right mono text-amber-300">${e.pressure_drop_kpa.toFixed(2)}</td>
                          <td class="p-2.5 text-right mono text-slate-300">${e.velocity_m_s.toFixed(2)}</td>
                          <td class="p-2.5 text-right mono text-sky-300">${Math.round(e.reynolds).toLocaleString()}</td>
                          <td class="p-2.5 text-right mono text-slate-400">${e.friction_factor.toFixed(4)}</td>
                          <td class="p-2.5 text-center">${this.getRegimeBadge(e.flow_regime)}</td>
                        </tr>
                      `
                          )
                          .join("")
                  }
                </tbody>
              </table>
            </div>
          </div>

          <!-- Hydraulic Nodes Table -->
          <div class="bg-[#101726] border border-[#1e293b] rounded shadow-sm overflow-hidden">
            <div class="px-4 py-2.5 bg-[#141e30] border-b border-[#1e293b] flex items-center justify-between">
              <h3 class="text-xs font-bold uppercase tracking-wider text-indigo-300 flex items-center gap-1.5">
                <span>📍</span> Hydraulic Network Nodes (${filteredNodes.length})
              </h3>
            </div>
            <div class="overflow-x-auto">
              <table class="w-full text-xs text-left text-slate-300">
                <thead class="bg-[#0b101b] text-slate-400 uppercase font-semibold border-b border-[#1e293b]">
                  <tr>
                    <th class="p-2.5">Node ID / Tag</th>
                    <th class="p-2.5 text-right">Pressure (kPa)</th>
                    <th class="p-2.5 text-right">Pressure (bar)</th>
                    <th class="p-2.5 text-right">Temp (°C)</th>
                    <th class="p-2.5 text-right">Density (kg/m³)</th>
                    <th class="p-2.5 text-right">Enthalpy (kJ/kg)</th>
                    <th class="p-2.5 text-center">Phase</th>
                    <th class="p-2.5 text-right">Elevation (m)</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-[#1e293b]">
                  ${
                    filteredNodes.length === 0
                      ? `<tr><td colspan="8" class="p-4 text-center text-slate-500">No hydraulic nodes match filter.</td></tr>`
                      : filteredNodes
                          .map(
                            (n) => `
                        <tr class="hover:bg-[#152033] transition">
                          <td class="p-2.5 font-medium text-white">${n.id} <span class="text-slate-400 font-normal">(${n.name})</span></td>
                          <td class="p-2.5 text-right mono font-semibold text-emerald-300">${(n.pressure_pa / 1000.0).toFixed(2)}</td>
                          <td class="p-2.5 text-right mono text-slate-300">${n.pressure_bar.toFixed(3)}</td>
                          <td class="p-2.5 text-right mono text-amber-300">${n.temperature_c.toFixed(1)}</td>
                          <td class="p-2.5 text-right mono text-slate-300">${n.density_kg_m3.toFixed(1)}</td>
                          <td class="p-2.5 text-right mono text-slate-300">${(n.enthalpy_j_kg / 1000.0).toFixed(1)}</td>
                          <td class="p-2.5 text-center"><span class="badge bg-slate-800 text-cyan-300">${n.phase}</span></td>
                          <td class="p-2.5 text-right mono text-slate-400">${n.elevation_m.toFixed(1)}</td>
                        </tr>
                      `
                          )
                          .join("")
                  }
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>
    `;

    this.bindEvents();
  }

  private bindEvents() {
    const btnRefresh = this.querySelector("#btnRefresh");
    if (btnRefresh) {
      btnRefresh.addEventListener("click", () => this.fetchDiagnostics());
    }

    const chkAuto = this.querySelector("#chkAutoRefresh") as HTMLInputElement;
    if (chkAuto) {
      chkAuto.addEventListener("change", () => {
        this.autoRefresh = chkAuto.checked;
        if (this.autoRefresh) {
          this.startPolling();
        } else {
          this.stopPolling();
        }
      });
    }

    const searchInput = this.querySelector("#searchInput") as HTMLInputElement;
    if (searchInput) {
      searchInput.addEventListener("input", () => {
        this.searchQuery = searchInput.value;
        this.render();
      });
    }

    const filterBtns = this.querySelectorAll(".filter-btn");
    filterBtns.forEach((btn) => {
      btn.addEventListener("click", () => {
        this.filter = (btn as HTMLElement).dataset.filter || "ALL";
        this.render();
      });
    });
  }
}

if (!customElements.get("nebu-hydraulic-inspector")) {
  customElements.define("nebu-hydraulic-inspector", NebuHydraulicInspector);
}
