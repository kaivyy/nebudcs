/**
 * Industrial Domains & Thermodynamics Inspector Web Component.
 *
 * Provides model-driven engineering inspection for:
 * - Registered industrial domain packages (Refinery, Petrochemical, Chemical, Gas, LNG, Power)
 * - Thermodynamic property requirements and cubic EOS formulations
 * - Arrhenius chemical reaction kinetics and CSTR/PFR reactor models
 * - Heat transfer models (LMTD, epsilon-NTU, fouling resistances, TEMA standards)
 * - Safe operating ranges and applicability envelopes
 * - Scientific provenance, authoritative citations, and model classifications
 * - Regulatory control strategies and scenario presets
 */

import { client } from "../api/client";
import { DomainPackageDetailDto, DomainPackageSummaryDto } from "../protocol/types";

export class NebuDomainBrowser extends HTMLElement {
  private domains: DomainPackageSummaryDto[] = [];
  private selectedDomain: DomainPackageDetailDto | null = null;
  private selectedDomainId: string | null = null;
  private isLoading = false;
  private activeTab: "overview" | "thermo" | "reactions" | "heat_transfer" | "controls" | "provenance" = "overview";

  async connectedCallback() {
    await this.loadDomains();
  }

  private async loadDomains() {
    this.isLoading = true;
    this.render();
    try {
      this.domains = await client.listDomainPackages();
      if (this.domains.length > 0 && !this.selectedDomainId) {
        await this.selectDomain(this.domains[0].id);
      } else if (this.selectedDomainId) {
        await this.selectDomain(this.selectedDomainId);
      }
    } catch (err) {
      console.error("Failed to load industrial domain packages:", err);
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private async selectDomain(id: string) {
    this.selectedDomainId = id;
    try {
      this.selectedDomain = await client.getDomainPackage(id);
    } catch (err) {
      console.error(`Failed to load domain package details for ${id}:`, err);
      this.selectedDomain = null;
    }
    this.render();
  }

  render() {
    if (this.isLoading && this.domains.length === 0) {
      this.innerHTML = `
        <div class="p-8 text-center text-slate-400">
          <div class="inline-block animate-spin text-2xl mb-2">⚙</div>
          <p>Loading industrial domain packages and thermodynamics...</p>
        </div>
      `;
      return;
    }

    this.innerHTML = `
      <div class="h-full flex flex-col bg-slate-950 text-slate-200 overflow-hidden">
        <!-- Header Banner -->
        <div class="px-6 py-4 border-b border-slate-800 bg-slate-900/80 flex items-center justify-between">
          <div>
            <div class="flex items-center gap-3">
              <span class="text-xl">🌐</span>
              <h2 class="text-lg font-bold text-white tracking-wide">INDUSTRIAL DOMAINS & THERMODYNAMICS</h2>
              <span class="px-2 py-0.5 text-xs font-semibold rounded bg-cyan-900/60 text-cyan-300 border border-cyan-700">
                ${this.domains.length} PACKAGES LOADED
              </span>
            </div>
            <p class="text-xs text-slate-400 mt-1">
              Authoritative process simulation models, thermodynamic property envelopes, and scientific provenance
            </p>
          </div>
          <button id="refresh-domains-btn" class="px-3 py-1.5 text-xs bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 rounded flex items-center gap-1.5 transition">
            🔄 Refresh Registry
          </button>
        </div>

        <!-- Main Layout: Sidebar & Content Area -->
        <div class="flex-1 flex overflow-hidden">
          <!-- Sidebar: Domain Package Selector -->
          <div class="w-80 border-r border-slate-800 bg-slate-900/40 flex flex-col">
            <div class="p-3 border-b border-slate-800 bg-slate-900/60 text-xs font-semibold uppercase tracking-wider text-slate-400">
              Standard Industrial Domains
            </div>
            <div class="flex-1 overflow-y-auto divide-y divide-slate-800/60">
              ${this.domains
                .map((d) => {
                  const isSelected = d.id === this.selectedDomainId;
                  const kindColor = this.getDomainColor(d.domain_kind);
                  return `
                    <button 
                      class="domain-item-btn w-full text-left p-3 hover:bg-slate-800/60 transition flex flex-col gap-1.5 ${
                        isSelected ? "bg-slate-800/90 border-l-4 border-cyan-400" : ""
                      }"
                      data-id="${d.id}"
                    >
                      <div class="flex items-center justify-between">
                        <span class="font-semibold text-xs text-white truncate">${d.name}</span>
                        <span class="text-[10px] px-1.5 py-0.5 rounded font-mono ${kindColor}">
                          ${d.domain_kind}
                        </span>
                      </div>
                      <div class="text-[11px] text-slate-400 line-clamp-2 leading-relaxed">
                        ${d.description}
                      </div>
                      <div class="flex items-center gap-3 text-[10px] text-slate-500 font-mono mt-0.5">
                        <span>v${d.version}</span>
                        <span>•</span>
                        <span>${d.strategy_count} Controls</span>
                        <span>•</span>
                        <span>${d.scenario_count} Scenarios</span>
                      </div>
                    </button>
                  `;
                })
                .join("")}
            </div>
          </div>

          <!-- Content Panel -->
          <div class="flex-1 flex flex-col bg-slate-950 overflow-hidden">
            ${
              this.selectedDomain
                ? this.renderDomainDetail(this.selectedDomain)
                : `<div class="p-12 text-center text-slate-500">Select an industrial domain package to view technical details.</div>`
            }
          </div>
        </div>
      </div>
    `;

    this.attachEventListeners();
  }

  private renderDomainDetail(domain: DomainPackageDetailDto): string {
    return `
      <!-- Detail Header -->
      <div class="px-6 py-4 border-b border-slate-800 bg-slate-900/30 flex items-center justify-between">
        <div>
          <div class="flex items-center gap-2">
            <h3 class="text-base font-bold text-white">${domain.name}</h3>
            <span class="text-xs px-2 py-0.5 rounded font-mono ${this.getDomainColor(domain.domain_kind)}">
              ${domain.domain_kind}
            </span>
            <span class="text-xs text-slate-400 font-mono">(${domain.id} v${domain.version})</span>
          </div>
          <p class="text-xs text-slate-300 mt-1 max-w-4xl">${domain.description}</p>
        </div>
      </div>

      <!-- Navigation Tabs -->
      <div class="px-6 border-b border-slate-800 bg-slate-900/50 flex gap-2 pt-2">
        <button class="detail-tab-btn px-3 py-2 text-xs font-semibold border-b-2 transition ${
          this.activeTab === "overview"
            ? "border-cyan-400 text-cyan-300"
            : "border-transparent text-slate-400 hover:text-slate-200"
        }" data-tab="overview">
          📊 Operating Envelope
        </button>
        <button class="detail-tab-btn px-3 py-2 text-xs font-semibold border-b-2 transition ${
          this.activeTab === "thermo"
            ? "border-cyan-400 text-cyan-300"
            : "border-transparent text-slate-400 hover:text-slate-200"
        }" data-tab="thermo">
          🌡️ Thermodynamics & Fluid
        </button>
        <button class="detail-tab-btn px-3 py-2 text-xs font-semibold border-b-2 transition ${
          this.activeTab === "reactions"
            ? "border-cyan-400 text-cyan-300"
            : "border-transparent text-slate-400 hover:text-slate-200"
        }" data-tab="reactions">
          ⚗️ Reaction Kinetics & CSTR/PFR
        </button>
        <button class="detail-tab-btn px-3 py-2 text-xs font-semibold border-b-2 transition ${
          this.activeTab === "heat_transfer"
            ? "border-cyan-400 text-cyan-300"
            : "border-transparent text-slate-400 hover:text-slate-200"
        }" data-tab="heat_transfer">
          🔥 Heat Exchangers & LMTD
        </button>
        <button class="detail-tab-btn px-3 py-2 text-xs font-semibold border-b-2 transition ${
          this.activeTab === "controls"
            ? "border-cyan-400 text-cyan-300"
            : "border-transparent text-slate-400 hover:text-slate-200"
        }" data-tab="controls">
          🕹️ Control Strategies & Scenarios
        </button>
        <button class="detail-tab-btn px-3 py-2 text-xs font-semibold border-b-2 transition ${
          this.activeTab === "provenance"
            ? "border-cyan-400 text-cyan-300"
            : "border-transparent text-slate-400 hover:text-slate-200"
        }" data-tab="provenance">
          📚 Scientific Governance
        </button>
      </div>

      <!-- Tab Content Area -->
      <div class="flex-1 overflow-y-auto p-6">
        ${this.renderTabContent(domain)}
      </div>
    `;
  }

  private renderTabContent(domain: DomainPackageDetailDto): string {
    switch (this.activeTab) {
      case "overview":
        return this.renderOperatingEnvelope(domain);
      case "thermo":
        return this.renderThermodynamics(domain);
      case "reactions":
        return this.renderReactions(domain);
      case "heat_transfer":
        return this.renderHeatTransfer(domain);
      case "controls":
        return this.renderControlsAndScenarios(domain);
      case "provenance":
        return this.renderScientificProvenance(domain);
    }
  }

  private renderOperatingEnvelope(domain: DomainPackageDetailDto): string {
    const ranges = Object.entries(domain.operating_ranges || {});
    return `
      <div class="space-y-6">
        <div>
          <h4 class="text-xs font-bold uppercase tracking-wider text-slate-400 mb-3">
            Safe Operating Envelopes & Physical Bounds
          </h4>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            ${ranges
              .map(([key, r]) => `
                <div class="p-4 rounded border border-slate-800 bg-slate-900/60 flex flex-col gap-2">
                  <div class="flex items-center justify-between">
                    <span class="text-xs font-mono font-semibold text-cyan-300">${key}</span>
                    <span class="text-[11px] px-2 py-0.5 rounded bg-slate-800 text-slate-300 font-mono">
                      ${r.quantity}
                    </span>
                  </div>
                  <div class="flex items-baseline justify-between mt-1">
                    <span class="text-2xl font-bold text-white font-mono">
                      ${r.normal_operating_value} <span class="text-xs text-slate-400 font-normal">${r.unit}</span>
                    </span>
                    <span class="text-xs text-emerald-400 font-mono">Nominal Operating Point</span>
                  </div>
                  <div class="w-full bg-slate-800 rounded-full h-2 mt-2 relative overflow-hidden">
                    <div class="bg-cyan-500 h-2 rounded-full" style="width: 60%"></div>
                  </div>
                  <div class="flex justify-between text-[11px] text-slate-500 font-mono">
                    <span>Min: ${r.min_value} ${r.unit}</span>
                    <span>Max: ${r.max_value} ${r.unit}</span>
                  </div>
                </div>
              `)
              .join("")}
          </div>
        </div>

        <!-- Validation Heuristics -->
        <div>
          <h4 class="text-xs font-bold uppercase tracking-wider text-slate-400 mb-3">
            Process Validation Heuristics & Protection Rules
          </h4>
          <div class="border border-slate-800 rounded divide-y divide-slate-800/60 bg-slate-900/30">
            ${(domain.validation_rules || [])
              .map(
                (rule) => `
                <div class="p-3 text-xs flex items-start gap-2.5">
                  <span class="text-amber-400 mt-0.5">⚠️</span>
                  <span class="text-slate-300 font-mono">${rule}</span>
                </div>
              `
              )
              .join("")}
          </div>
        </div>
      </div>
    `;
  }

  private renderThermodynamics(domain: DomainPackageDetailDto): string {
    const compEntries = Object.entries(domain.composition_defaults || {});
    return `
      <div class="space-y-6">
        <div>
          <h4 class="text-xs font-bold uppercase tracking-wider text-slate-400 mb-3">
            Thermodynamic Engine Requirements
          </h4>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            ${(domain.thermodynamic_requirements || [])
              .map(
                (req) => `
                <div class="p-3 rounded border border-slate-800 bg-slate-900/40 flex items-center gap-2.5">
                  <span class="text-cyan-400">⚡</span>
                  <span class="text-xs font-mono text-slate-200">${req}</span>
                </div>
              `
              )
              .join("")}
          </div>
        </div>

        <div>
          <h4 class="text-xs font-bold uppercase tracking-wider text-slate-400 mb-3">
            Default Chemical Composition (Mole / Mass Fraction)
          </h4>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
            ${compEntries
              .map(
                ([comp, val]) => `
                <div class="p-3 rounded border border-slate-800 bg-slate-900/40 flex flex-col">
                  <span class="text-xs font-mono text-slate-400">${comp}</span>
                  <span class="text-base font-bold text-white font-mono mt-1">
                    ${(val * 100).toFixed(1)}%
                  </span>
                </div>
              `
              )
              .join("")}
          </div>
        </div>

        <div>
          <h4 class="text-xs font-bold uppercase tracking-wider text-slate-400 mb-3">
            Required Industrial Utilities
          </h4>
          <div class="flex flex-wrap gap-2">
            ${(domain.utility_requirements || [])
              .map(
                (u) => `
                <span class="px-2.5 py-1 text-xs rounded border border-slate-700 bg-slate-800 text-slate-300 font-mono">
                  🔧 ${u}
                </span>
              `
              )
              .join("")}
          </div>
        </div>
      </div>
    `;
  }

  private renderReactions(domain: DomainPackageDetailDto): string {
    const isChemical = domain.domain_kind.toLowerCase().includes("chem");
    return `
      <div class="space-y-6">
        <div class="p-4 rounded border border-cyan-900/40 bg-cyan-950/20 text-xs text-cyan-200 flex flex-col gap-2">
          <div class="flex items-center gap-2 font-bold text-cyan-300">
            <span>⚗️</span>
            <span>Arrhenius Reaction Engine & Mathematical Formulations</span>
          </div>
          <p class="leading-relaxed text-slate-300">
            NebuDCS solves generalized multi-species Arrhenius rate equations coupled with conservation balances:
          </p>
          <div class="p-3 bg-slate-900/90 rounded border border-slate-800 font-mono text-emerald-400 text-xs">
            k(T) = A · (T / T_ref)^n · exp(-E_a / (R · T))<br/>
            r_net = η_cat · [ k_f ∏ C_i^α_i - k_r ∏ C_j^β_j ]<br/>
            q_rxn = ∑ r_k · (-ΔH_r,k)
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="p-4 rounded border border-slate-800 bg-slate-900/40">
            <h5 class="text-xs font-bold text-white mb-1">CSTR Dynamic Model</h5>
            <p class="text-xs text-slate-400 mb-3">
              V · dC_i/dt = F_in·C_in,i - F_out·C_i + V · R_i
            </p>
            <div class="space-y-1 text-xs font-mono text-slate-300">
              <div>• Perfectly mixed assumption</div>
              <div>• Jacketed cooling / heating balance: Q = UA(T - T_j)</div>
              <div>• Semenov thermal runaway detection: dQ_gen/dT > dQ_rem/dT</div>
            </div>
          </div>

          <div class="p-4 rounded border border-slate-800 bg-slate-900/40">
            <h5 class="text-xs font-bold text-white mb-1">PFR 1D Discretized Model</h5>
            <p class="text-xs text-slate-400 mb-3">
              dC_i/dz = (A_c / v_dot) · R_i
            </p>
            <div class="space-y-1 text-xs font-mono text-slate-300">
              <div>• 1D spatial axial finite slices</div>
              <div>• Wall heat flux: dq/dz = U·P·(T_wall - T)</div>
              <div>• Ergun equation pressure drop option</div>
            </div>
          </div>
        </div>

        ${
          isChemical
            ? `
          <div class="p-3 bg-emerald-950/20 border border-emerald-800/40 rounded text-xs text-emerald-300 flex items-center gap-2">
            <span>✅</span>
            <span>This domain package actively enables dynamic Arrhenius reaction evaluations and thermal runaway monitors.</span>
          </div>
        `
            : `
          <div class="p-3 bg-slate-900/40 border border-slate-800 rounded text-xs text-slate-400 flex items-center gap-2">
            <span>ℹ️</span>
            <span>Non-reactive separation/transport flowsheet. Reaction rates default to inert (r = 0).</span>
          </div>
        `
        }
      </div>
    `;
  }

  private renderHeatTransfer(_domain: DomainPackageDetailDto): string {
    return `
      <div class="space-y-6">
        <div class="p-4 rounded border border-cyan-900/40 bg-cyan-950/20 text-xs text-cyan-200 flex flex-col gap-2">
          <div class="flex items-center gap-2 font-bold text-cyan-300">
            <span>🔥</span>
            <span>Rigorous Heat Transfer & TEMA Standard Solvers</span>
          </div>
          <p class="leading-relaxed text-slate-300">
            Equipment models implement Logarithmic Mean Temperature Difference (LMTD) and Epsilon-NTU methods:
          </p>
          <div class="p-3 bg-slate-900/90 rounded border border-slate-800 font-mono text-emerald-400 text-xs">
            Q = U · A · F · ΔT_lm<br/>
            ΔT_lm = (ΔT_1 - ΔT_2) / ln(ΔT_1 / ΔT_2)<br/>
            1 / (U·A) = 1/(h_h·A_h) + R_f,h/A_h + R_wall + R_f,c/A_c + 1/(h_c·A_c)
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div class="p-4 rounded border border-slate-800 bg-slate-900/40">
            <h5 class="text-xs font-bold text-white mb-2">Flow Configurations</h5>
            <div class="space-y-1.5 text-xs text-slate-300 font-mono">
              <div>• Counter-Current (F = 1.0)</div>
              <div>• Co-Current Parallel</div>
              <div>• Shell & Tube 1-2 (Bowman F-corr)</div>
              <div>• Shell & Tube 2-4 Pass</div>
              <div>• Cross-Flow Unmixed</div>
            </div>
          </div>

          <div class="p-4 rounded border border-slate-800 bg-slate-900/40">
            <h5 class="text-xs font-bold text-white mb-2">TEMA Fouling Factors</h5>
            <div class="space-y-1.5 text-xs text-slate-300 font-mono">
              <div>• Cooling Water: 0.0002 m²K/W</div>
              <div>• Crude Oil: 0.0007 m²K/W</div>
              <div>• Steam / Condensate: 0.0001 m²K/W</div>
              <div>• Light Hydrocarbons: 0.0002 m²K/W</div>
              <div>• Heavy Fuel Oil: 0.0009 m²K/W</div>
            </div>
          </div>

          <div class="p-4 rounded border border-slate-800 bg-slate-900/40">
            <h5 class="text-xs font-bold text-white mb-2">Thermodynamic Bounds</h5>
            <div class="space-y-1.5 text-xs text-slate-300 font-mono">
              <div>• 2nd Law Enforced: Q &le; C_min·ΔT_max</div>
              <div>• Smooth Taylor expansion for ΔT_1 &approx; ΔT_2</div>
              <div>• No temperature cross violation</div>
              <div>• Strict energy balance residual &lt; 1e-4</div>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  private renderControlsAndScenarios(domain: DomainPackageDetailDto): string {
    return `
      <div class="space-y-6">
        <div>
          <h4 class="text-xs font-bold uppercase tracking-wider text-slate-400 mb-3">
            Authoritative Regulatory & Advanced Control Strategies
          </h4>
          <div class="border border-slate-800 rounded divide-y divide-slate-800 bg-slate-900/30">
            ${(domain.control_strategies || [])
              .map(
                (ctrl) => `
                <div class="p-3 text-xs flex items-center gap-2.5">
                  <span class="text-cyan-400">🕹️</span>
                  <span class="text-slate-200 font-mono">${ctrl}</span>
                </div>
              `
              )
              .join("")}
          </div>
        </div>

        <div>
          <h4 class="text-xs font-bold uppercase tracking-wider text-slate-400 mb-3">
            Pre-configured Training Scenarios & Upset Conditions
          </h4>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            ${(domain.scenario_presets || [])
              .map(
                (scn) => `
                <div class="p-3 rounded border border-slate-800 bg-slate-900/40 flex items-center gap-2.5">
                  <span class="text-amber-400">⚡</span>
                  <span class="text-xs font-mono text-slate-300">${scn}</span>
                </div>
              `
              )
              .join("")}
          </div>
        </div>
      </div>
    `;
  }

  private renderScientificProvenance(domain: DomainPackageDetailDto): string {
    const refs = domain.scientific_references ?? [];
    const models = domain.model_classifications ?? [];

    return `
      <div class="space-y-6">
        <div>
          <h4 class="text-xs font-bold uppercase tracking-wider text-slate-400 mb-3">
            Authoritative Scientific References & Standards Citations
          </h4>
          ${
            refs.length > 0
              ? `
              <div class="space-y-3">
                ${refs
                  .map(
                    (ref) => {
                      if (typeof ref === "string") {
                        return `
                          <div class="p-3 text-xs flex items-start gap-2.5 rounded border border-slate-800 bg-slate-900/30">
                            <span class="text-emerald-400 mt-0.5">📖</span>
                            <span class="text-slate-200 font-mono">${ref}</span>
                          </div>
                        `;
                      }
                      return `
                        <div class="p-4 rounded border border-slate-800 bg-slate-900/40 flex flex-col gap-2">
                          <div class="flex items-center justify-between">
                            <span class="text-xs font-bold text-white">${ref.title}</span>
                            <span class="text-[11px] px-2 py-0.5 rounded bg-emerald-950 text-emerald-300 font-mono border border-emerald-800">
                              [${ref.citation_key}] (${ref.publication_year})
                            </span>
                          </div>
                          <div class="text-xs text-slate-300">
                            <span class="text-slate-500">Authors:</span> ${ref.authors.join(", ")} &bull; <span class="text-slate-500">Source:</span> ${ref.source}
                          </div>
                          <div class="p-2.5 bg-slate-950/80 rounded border border-slate-800 text-xs font-mono space-y-1">
                            <div class="text-cyan-300"><span class="text-slate-500 font-sans">Governing Concept:</span> ${ref.governing_concept}</div>
                            <div class="text-slate-400"><span class="text-slate-500 font-sans">Validity Domain:</span> ${ref.validity_domain}</div>
                          </div>
                        </div>
                      `;
                    }
                  )
                  .join("")}
              </div>
            `
              : `<div class="p-4 text-xs text-slate-500 italic border border-slate-800 rounded bg-slate-900/20">Standard thermodynamic package library references apply.</div>`
          }
        </div>

        <div>
          <h4 class="text-xs font-bold uppercase tracking-wider text-slate-400 mb-3">
            Mathematical & Physical Model Classifications
          </h4>
          ${
            models.length > 0
              ? `
              <div class="space-y-3">
                ${models
                  .map(
                    (m) => {
                      if (typeof m === "string") {
                        return `
                          <div class="p-3 text-xs flex items-start gap-2.5 rounded border border-slate-800 bg-slate-900/30">
                            <span class="text-cyan-400 mt-0.5">📐</span>
                            <span class="text-slate-300 font-mono">${m}</span>
                          </div>
                        `;
                      }
                      return `
                        <div class="p-4 rounded border border-slate-800 bg-slate-900/40 flex flex-col gap-2">
                          <div class="flex items-center justify-between">
                            <span class="text-xs font-bold text-white">${m.subsystem}</span>
                            <span class="text-[11px] px-2 py-0.5 rounded bg-cyan-950 text-cyan-300 font-mono border border-cyan-800">
                              ${m.classification}
                            </span>
                          </div>
                          <div class="p-2.5 bg-slate-950/80 rounded border border-slate-800 text-xs font-mono text-emerald-400">
                            ${m.governing_equation}
                          </div>
                          <div class="text-xs text-slate-400">
                            <span class="text-slate-500">Assumptions:</span> ${m.primary_assumptions.join(", ")}
                          </div>
                          <div class="text-xs text-slate-300 flex items-center gap-1.5">
                            <span class="text-cyan-400">✓</span>
                            <span>${m.conservation_verification}</span>
                          </div>
                        </div>
                      `;
                    }
                  )
                  .join("")}
              </div>
            `
              : `<div class="p-4 text-xs text-slate-500 italic border border-slate-800 rounded bg-slate-900/20">Model classifications documented in system architecture.</div>`
          }
        </div>

        <div class="p-4 rounded border border-amber-900/40 bg-amber-950/20 text-xs text-amber-200">
          <span class="font-bold">⚠️ Scientific Governance Disclaimer:</span>
          All process physical models, equations of state, and kinetics in NebuDCS are rigorous mathematical representations intended for engineering simulation, validation, and training. They do not constitute formal real-world regulatory safety or SIL certifications.
        </div>
      </div>
    `;
  }

  private getDomainColor(domainKind: string): string {
    const k = domainKind.toLowerCase();
    if (k.includes("refin")) return "bg-amber-950 text-amber-300 border border-amber-800";
    if (k.includes("petro")) return "bg-purple-950 text-purple-300 border border-purple-800";
    if (k.includes("chem")) return "bg-emerald-950 text-emerald-300 border border-emerald-800";
    if (k.includes("gas")) return "bg-cyan-950 text-cyan-300 border border-cyan-800";
    if (k.includes("power")) return "bg-rose-950 text-rose-300 border border-rose-800";
    return "bg-slate-800 text-slate-300 border border-slate-700";
  }

  private attachEventListeners() {
    this.querySelectorAll<HTMLButtonElement>(".domain-item-btn").forEach((btn) => {
      btn.addEventListener("click", () => {
        const id = btn.dataset.id;
        if (id) {
          this.selectDomain(id);
        }
      });
    });

    this.querySelectorAll<HTMLButtonElement>(".detail-tab-btn").forEach((btn) => {
      btn.addEventListener("click", () => {
        const tab = btn.dataset.tab as any;
        if (tab) {
          this.activeTab = tab;
          this.render();
        }
      });
    });

    this.querySelector("#refresh-domains-btn")?.addEventListener("click", () => {
      this.loadDomains();
    });
  }
}

customElements.define("nebu-domain-browser", NebuDomainBrowser);
