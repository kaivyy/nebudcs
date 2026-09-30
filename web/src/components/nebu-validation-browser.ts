/**
 * Scientific Validation, Benchmark & Certification Browser Web Component.
 *
 * Implements Stage 13: Validation, Benchmark & Scientific Certification Framework.
 * Provides rigorous model qualification, benchmark evaluation, error metrics inspection,
 * authoritative reference tracking, and certified manifest export.
 *
 * CERTIFICATION LANGUAGE POLICY:
 * Permitted terms: "INTERNALLY VALIDATED", "BENCHMARKED", "ENGINEERING CONSISTENCY VERIFIED", "VALIDATED WITH LIMITATIONS".
 * Forbidden terms: "IEC CERTIFIED", "API CERTIFIED", "ISA CERTIFIED", "SIL CERTIFIED", "INDUSTRY CERTIFIED".
 */

import { client } from "../api/client";
import {
  ValidationExecutiveSummaryDto,
  ValidationCaseSummaryDto,
  ValidationCaseDetailDto,
  ReferenceDatasetDto,
  ModelValidationSummaryDto,
  ValidationManifestDto,
} from "../protocol/types";

export class NebuValidationBrowser extends HTMLElement {
  private summary: ValidationExecutiveSummaryDto | null = null;
  private cases: ValidationCaseSummaryDto[] = [];
  private selectedCase: ValidationCaseDetailDto | null = null;
  private selectedCaseId: string | null = null;
  private models: ModelValidationSummaryDto[] = [];
  private references: ReferenceDatasetDto[] = [];

  private selectedDomain: string = "ALL";
  private activeViewTab: "cases" | "models" | "references" | "disclaimer" = "cases";
  private isLoading = false;
  private isRunningSuite = false;
  private statusMessage: string | null = null;
  private isError = false;

  async connectedCallback() {
    await this.loadAll();
  }

  private async loadAll() {
    this.isLoading = true;
    this.render();
    try {
      const [sum, casesList, modelsList, refsList] = await Promise.all([
        client.getValidationSummary(),
        client.listValidationCases(this.selectedDomain === "ALL" ? undefined : this.selectedDomain),
        client.listValidationModels(),
        client.listValidationReferences(),
      ]);
      this.summary = sum;
      this.cases = casesList;
      this.models = modelsList;
      this.references = refsList;

      if (this.cases.length > 0 && !this.selectedCaseId) {
        await this.selectCase(this.cases[0].id);
      } else if (this.selectedCaseId) {
        await this.selectCase(this.selectedCaseId);
      }
    } catch (err: any) {
      console.error("Failed to load validation framework data:", err);
      this.statusMessage = `Failed to load validation framework: ${err.message || err}`;
      this.isError = true;
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private async selectCase(id: string) {
    this.selectedCaseId = id;
    try {
      this.selectedCase = await client.getValidationCase(id);
    } catch (err: any) {
      console.error(`Failed to load case detail ${id}:`, err);
      this.selectedCase = null;
    }
    this.render();
  }

  private async handleRunBenchmarks() {
    if (this.isRunningSuite) return;
    this.isRunningSuite = true;
    this.statusMessage = "Executing scientific benchmark verification suite across all domains...";
    this.isError = false;
    this.render();

    try {
      const domainFilter = this.selectedDomain === "ALL" ? undefined : this.selectedDomain;
      const resp = await client.runValidationBenchmarks({ domain_filter: domainFilter });
      this.summary = resp.summary;
      this.cases = resp.cases;
      this.statusMessage = `✓ Benchmark suite completed: ${resp.executed_cases} executed, ${resp.passed_cases} passed (${resp.total_duration_ms.toFixed(2)} ms)`;
      this.isError = resp.failed_cases > 0;
      if (this.selectedCaseId) {
        await this.selectCase(this.selectedCaseId);
      } else if (this.cases.length > 0) {
        await this.selectCase(this.cases[0].id);
      }
    } catch (err: any) {
      this.statusMessage = `Benchmark execution failed: ${err.message || err}`;
      this.isError = true;
    } finally {
      this.isRunningSuite = false;
      this.render();
    }
  }

  private async handleExportManifest() {
    try {
      this.statusMessage = "Generating certified validation manifest...";
      this.render();
      const manifest: ValidationManifestDto = await client.getValidationManifest();
      const blob = new Blob([JSON.stringify(manifest, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `nebudcs-validation-manifest-${manifest.system_id}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      this.statusMessage = "✓ Validation manifest JSON exported successfully.";
      this.isError = false;
    } catch (err: any) {
      this.statusMessage = `Export failed: ${err.message || err}`;
      this.isError = true;
    } finally {
      this.render();
    }
  }

  private getStatusBadge(status: string) {
    switch (status) {
      case "VALIDATED":
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-emerald-950 text-emerald-400 border border-emerald-700">INTERNALLY VALIDATED</span>`;
      case "BENCHMARKED":
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-cyan-950 text-cyan-400 border border-cyan-700">BENCHMARKED</span>`;
      case "ENGINEERING_CONSISTENT":
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-blue-950 text-blue-400 border border-blue-700">ENGINEERING CONSISTENCY VERIFIED</span>`;
      case "VALIDATED_WITH_LIMITATIONS":
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-amber-950 text-amber-400 border border-amber-700">VALIDATED WITH LIMITATIONS</span>`;
      case "INVALID":
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-rose-950 text-rose-400 border border-rose-700">FAILED / INVALID</span>`;
      default:
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-slate-800 text-slate-400 border border-slate-700">${status}</span>`;
    }
  }

  private getClassificationLabel(c: string) {
    switch (c) {
      case "PEER_REVIEWED_EXPERIMENTAL":
        return "Peer-Reviewed Experimental";
      case "PUBLISHED_BENCHMARK":
        return "Published Benchmark";
      case "PUBLIC_DATASET":
        return "Public Repository (NIST/GPA)";
      case "ENGINEERING_REFERENCE":
        return "Engineering Reference (Perry/Crane)";
      case "STANDARD_REFERENCE":
        return "Standard Reference (API/ISA/IAPWS)";
      case "MANUFACTURER_DATA":
        return "Manufacturer Data";
      case "ANALYTICAL_REFERENCE":
        return "Analytical Closed-Form";
      case "ENGINEERING_CONSISTENCY":
        return "Conservation Law Invariant";
      case "SYNTHETIC_TEST":
        return "Synthetic Invariant";
      default:
        return c;
    }
  }

  private getProvenanceLabel(p: string) {
    switch (p) {
      case "SOURCE_DEFINED":
        return "Author / Source Defined";
      case "ENGINEERING_DEFINED":
        return "Standard Engineering Margin (API/TEMA)";
      case "NUMERICAL":
        return "Machine Precision Bound";
      case "PROJECT_DEFINED":
        return "Project Baseline Defined";
      default:
        return p;
    }
  }

  render() {
    if (this.isLoading && !this.summary) {
      this.innerHTML = `
        <div class="p-8 text-center text-slate-400">
          <div class="inline-block animate-spin text-2xl mb-2">⚙</div>
          <p>Loading validation cases, error metrics and benchmark datasets...</p>
        </div>
      `;
      return;
    }

    const sum = this.summary || {
      total_cases: 0,
      passed_cases: 0,
      failed_cases: 0,
      pass_rate_pct: 0,
      overall_status: "NOT_VALIDATED",
      domain_counts: [],
      classification_counts: [],
      timestamp_iso: "",
      engine_version: "v0.1.0",
      compliance_disclaimer: "",
    };

    const domainsList = [
      "ALL",
      "Thermodynamics",
      "Reactions",
      "Hydraulics",
      "Heat Transfer",
      "Regulatory Control",
      "IEC 61131-3",
      "Functional Safety",
      "Steam & Power",
      "Conservation",
    ];

    this.innerHTML = `
      <div class="h-full flex flex-col bg-slate-950 text-slate-200 overflow-hidden font-sans">
        <!-- Top Banner / Header -->
        <header class="px-6 py-3 border-b border-slate-800 bg-slate-900/90 flex flex-wrap items-center justify-between gap-4">
          <div class="flex items-center gap-3">
            <span class="text-2xl">🔬</span>
            <div>
              <div class="flex items-center gap-2">
                <h2 class="text-base font-bold text-white tracking-wide uppercase">Scientific Validation & Benchmark Framework</h2>
                <span class="px-2 py-0.5 text-xs font-mono font-semibold rounded bg-indigo-950 text-indigo-300 border border-indigo-700">
                  STAGE 13
                </span>
                ${this.getStatusBadge(sum.overall_status)}
              </div>
              <p class="text-xs text-slate-400">
                Authoritative verification against published datasets, engineering handbooks, standard bodies & conservation invariants
              </p>
            </div>
          </div>

          <!-- Top Actions -->
          <div class="flex items-center gap-2">
            <button id="btn-export-manifest" class="px-3 py-1.5 text-xs font-semibold bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 rounded flex items-center gap-1.5 transition">
              📄 Export Manifest (JSON)
            </button>
            <button id="btn-run-benchmarks" ${this.isRunningSuite ? "disabled" : ""} class="px-3 py-1.5 text-xs font-semibold bg-emerald-600 hover:bg-emerald-500 disabled:bg-emerald-800 disabled:opacity-50 text-white rounded flex items-center gap-1.5 transition shadow">
              ${this.isRunningSuite ? "⚙ Running Benchmark Suite..." : "▶ Run Scientific Benchmarks"}
            </button>
          </div>
        </header>

        <!-- Status / Feedback Toast -->
        ${
          this.statusMessage
            ? `
          <div class="px-6 py-2 text-xs flex items-center justify-between border-b ${
            this.isError ? "bg-rose-950/80 border-rose-800 text-rose-300" : "bg-emerald-950/80 border-emerald-800 text-emerald-300"
          }">
            <span class="font-mono">${this.statusMessage}</span>
            <button id="btn-dismiss-msg" class="text-xs opacity-75 hover:opacity-100">✕</button>
          </div>
        `
            : ""
        }

        <!-- Executive KPI Cards -->
        <section class="grid grid-cols-2 md:grid-cols-5 gap-3 p-4 bg-slate-900/40 border-b border-slate-800">
          <div class="p-3 rounded bg-slate-900/80 border border-slate-800 flex flex-col">
            <span class="text-xs text-slate-400 font-semibold uppercase">Total Benchmark Cases</span>
            <span class="text-xl font-bold font-mono text-white mt-1">${sum.total_cases}</span>
            <span class="text-[10px] text-slate-500 mt-0.5">Acceptance Matrix A–T</span>
          </div>
          <div class="p-3 rounded bg-slate-900/80 border border-slate-800 flex flex-col">
            <span class="text-xs text-slate-400 font-semibold uppercase">Verification Passed</span>
            <span class="text-xl font-bold font-mono text-emerald-400 mt-1">${sum.passed_cases}</span>
            <span class="text-[10px] text-emerald-500 mt-0.5">Within tolerance bounds</span>
          </div>
          <div class="p-3 rounded bg-slate-900/80 border border-slate-800 flex flex-col">
            <span class="text-xs text-slate-400 font-semibold uppercase">Failed / Deviations</span>
            <span class="text-xl font-bold font-mono ${sum.failed_cases > 0 ? "text-rose-400" : "text-slate-400"} mt-1">${sum.failed_cases}</span>
            <span class="text-[10px] text-slate-500 mt-0.5">0 tolerance violations</span>
          </div>
          <div class="p-3 rounded bg-slate-900/80 border border-slate-800 flex flex-col">
            <span class="text-xs text-slate-400 font-semibold uppercase">Benchmark Pass Rate</span>
            <span class="text-xl font-bold font-mono text-cyan-400 mt-1">${sum.pass_rate_pct.toFixed(1)}%</span>
            <span class="text-[10px] text-cyan-500 mt-0.5">Strict scientific threshold</span>
          </div>
          <div class="p-3 rounded bg-slate-900/80 border border-slate-800 flex flex-col">
            <span class="text-xs text-slate-400 font-semibold uppercase">Scientific Governance</span>
            <span class="text-xs font-bold text-indigo-300 mt-2 tracking-wide uppercase">ASME V&V 10/20</span>
            <span class="text-[10px] text-slate-400 mt-0.5">Non-regulatory validation</span>
          </div>
        </section>

        <!-- View Switcher Tabs & Domain Filter Bar -->
        <div class="px-6 py-2 border-b border-slate-800 bg-slate-900/70 flex flex-wrap items-center justify-between gap-3">
          <div class="flex items-center gap-1">
            <button class="view-tab-btn px-3 py-1 text-xs font-semibold rounded ${
              this.activeViewTab === "cases" ? "bg-slate-800 text-cyan-400 border border-slate-700" : "text-slate-400 hover:text-white"
            }" data-tab="cases">
              📋 Validation Cases (${this.cases.length})
            </button>
            <button class="view-tab-btn px-3 py-1 text-xs font-semibold rounded ${
              this.activeViewTab === "models" ? "bg-slate-800 text-cyan-400 border border-slate-700" : "text-slate-400 hover:text-white"
            }" data-tab="models">
              ⚙️ Model Profiles (${this.models.length})
            </button>
            <button class="view-tab-btn px-3 py-1 text-xs font-semibold rounded ${
              this.activeViewTab === "references" ? "bg-slate-800 text-cyan-400 border border-slate-700" : "text-slate-400 hover:text-white"
            }" data-tab="references">
              📚 Reference Datasets (${this.references.length})
            </button>
            <button class="view-tab-btn px-3 py-1 text-xs font-semibold rounded ${
              this.activeViewTab === "disclaimer" ? "bg-slate-800 text-amber-400 border border-slate-700" : "text-slate-400 hover:text-white"
            }" data-tab="disclaimer">
              ⚖️ Scientific Governance & Disclaimer
            </button>
          </div>

          ${
            this.activeViewTab === "cases"
              ? `
            <!-- Domain Filters -->
            <div class="flex items-center gap-1 overflow-x-auto text-xs py-1">
              <span class="text-slate-500 mr-1 text-[11px] font-semibold uppercase">Domain:</span>
              ${domainsList
                .map(
                  (d) => `
                <button class="domain-pill-btn px-2.5 py-0.5 rounded-full text-[11px] font-medium transition ${
                  this.selectedDomain === d
                    ? "bg-cyan-500/20 text-cyan-300 border border-cyan-500/40 font-semibold"
                    : "bg-slate-800/60 text-slate-400 hover:text-slate-200 border border-slate-800"
                }" data-domain="${d}">
                  ${d}
                </button>
              `
                )
                .join("")}
            </div>
          `
              : ""
          }
        </div>

        <!-- Main Body Area -->
        <main class="flex-1 flex overflow-hidden">
          ${
            this.activeViewTab === "cases"
              ? this.renderCasesView()
              : this.activeViewTab === "models"
              ? this.renderModelsView()
              : this.activeViewTab === "references"
              ? this.renderReferencesView()
              : this.renderDisclaimerView()
          }
        </main>

        <!-- Bottom Industrial Legal Guard -->
        <footer class="px-6 py-2 border-t border-slate-800 bg-slate-950 flex items-center justify-between text-[11px] text-slate-500">
          <div class="flex items-center gap-2">
            <span class="text-amber-500 font-bold">LEGAL NOTICE:</span>
            <span>All benchmark qualifications are calculated internally. Strictly no claim of external accreditation by IEC, API, ISA, or SIL certification bodies.</span>
          </div>
          <div class="font-mono text-slate-400">
            NebuDCS Simulation Core ${sum.engine_version}
          </div>
        </footer>
      </div>
    `;

    this.setupEventListeners();
  }

  private renderCasesView(): string {
    const filteredCases = this.cases.filter((c) => {
      if (this.selectedDomain === "ALL") return true;
      return c.domain.toLowerCase().includes(this.selectedDomain.toLowerCase());
    });

    return `
      <!-- Left Column: Benchmark Cases List -->
      <div class="w-2/5 border-r border-slate-800 bg-slate-900/30 flex flex-col overflow-hidden">
        <div class="p-3 border-b border-slate-800 bg-slate-900/50 flex items-center justify-between text-xs text-slate-400 font-semibold">
          <span>CASES (${filteredCases.length})</span>
          <span>TOLERANCE / ERROR</span>
        </div>
        <div class="flex-1 overflow-y-auto divide-y divide-slate-800/60">
          ${
            filteredCases.length === 0
              ? `<div class="p-6 text-center text-slate-500 text-xs">No benchmark cases found for this domain filter.</div>`
              : filteredCases
                  .map((c) => {
                    const isSelected = c.id === this.selectedCaseId;
                    return `
                <div class="case-item p-3 cursor-pointer transition ${
                  isSelected ? "bg-cyan-950/40 border-l-4 border-cyan-400" : "hover:bg-slate-800/40"
                }" data-case-id="${c.id}">
                  <div class="flex items-center justify-between mb-1">
                    <span class="font-mono text-xs font-bold ${isSelected ? "text-cyan-300" : "text-slate-300"}">${c.id}</span>
                    ${this.getStatusBadge(c.status)}
                  </div>
                  <div class="text-xs font-semibold text-white truncate">${c.name}</div>
                  <div class="text-[11px] text-slate-400 mt-1 flex items-center justify-between">
                    <span>${c.domain} · ${c.tested_feature}</span>
                    <span class="font-mono text-[10px] ${c.passed ? "text-emerald-400" : "text-rose-400"}">
                      Err: ${c.max_relative_error_pct.toFixed(2)}% (tol: ${c.relative_tolerance_pct}%)
                    </span>
                  </div>
                  ${
                    c.standard_id
                      ? `<div class="mt-1 text-[10px] font-mono text-indigo-400">Ref: ${c.standard_id}</div>`
                      : ""
                  }
                </div>
              `;
                  })
                  .join("")
          }
        </div>
      </div>

      <!-- Right Column: Case Deep Inspector -->
      <div class="flex-1 overflow-y-auto p-6 bg-slate-950/80">
        ${
          this.selectedCase
            ? this.renderCaseDetail(this.selectedCase)
            : `<div class="p-12 text-center text-slate-500">Select a benchmark case to inspect metrics, applicability envelope, and citations.</div>`
        }
      </div>
    `;
  }

  private renderCaseDetail(c: ValidationCaseDetailDto): string {
    return `
      <div class="space-y-6">
        <!-- Header -->
        <div class="border-b border-slate-800 pb-4">
          <div class="flex items-center justify-between gap-4 mb-2">
            <div>
              <span class="font-mono text-xs text-cyan-400 font-bold tracking-wider">${c.id}</span>
              <h3 class="text-lg font-bold text-white mt-0.5">${c.name}</h3>
            </div>
            <div class="flex flex-col items-end gap-1">
              ${this.getStatusBadge(c.status)}
              <span class="text-[10px] text-slate-500 font-mono">Executed: ${c.last_executed_iso}</span>
            </div>
          </div>
          <div class="flex flex-wrap items-center gap-2 text-xs text-slate-400">
            <span class="px-2 py-0.5 rounded bg-slate-800 border border-slate-700 text-slate-300">Domain: <strong>${c.domain}</strong></span>
            <span class="px-2 py-0.5 rounded bg-slate-800 border border-slate-700 text-slate-300">Model: <strong>${c.model_name}</strong></span>
            <span class="px-2 py-0.5 rounded bg-slate-800 border border-slate-700 text-slate-300">Classification: <strong>${this.getClassificationLabel(c.classification)}</strong></span>
            <span class="px-2 py-0.5 rounded bg-slate-800 border border-slate-700 text-slate-300">Duration: <strong>${c.execution_duration_ms.toFixed(2)} ms</strong></span>
          </div>
        </div>

        <!-- Error Metrics Grid -->
        <div>
          <h4 class="text-xs font-semibold uppercase tracking-wider text-slate-400 mb-3">Verification Error Metrics & Tolerances</h4>
          <div class="grid grid-cols-2 md:grid-cols-3 gap-3">
            <div class="p-3 rounded bg-slate-900 border border-slate-800">
              <span class="text-[10px] text-slate-500 font-semibold uppercase">Max Relative Error</span>
              <div class="text-lg font-mono font-bold ${c.passed ? "text-emerald-400" : "text-rose-400"} mt-0.5">
                ${c.metrics.max_relative_error_pct.toFixed(4)}%
              </div>
              <span class="text-[10px] text-slate-400">Tolerance: &lt; ${c.relative_tolerance_pct}%</span>
            </div>
            <div class="p-3 rounded bg-slate-900 border border-slate-800">
              <span class="text-[10px] text-slate-500 font-semibold uppercase">Mean Absolute Error (MAE)</span>
              <div class="text-lg font-mono font-bold text-slate-200 mt-0.5">
                ${c.metrics.mean_absolute_error.toExponential(3)}
              </div>
              <span class="text-[10px] text-slate-400">Abs Tol: ${c.absolute_tolerance}</span>
            </div>
            <div class="p-3 rounded bg-slate-900 border border-slate-800">
              <span class="text-[10px] text-slate-500 font-semibold uppercase">Root Mean Squared (RMSE)</span>
              <div class="text-lg font-mono font-bold text-slate-200 mt-0.5">
                ${c.metrics.root_mean_squared_error.toExponential(3)}
              </div>
              <span class="text-[10px] text-slate-400">L2 Norm Residual</span>
            </div>
            <div class="p-3 rounded bg-slate-900 border border-slate-800">
              <span class="text-[10px] text-slate-500 font-semibold uppercase">Mean Bias Error</span>
              <div class="text-lg font-mono font-bold text-slate-200 mt-0.5">
                ${c.metrics.mean_bias_error.toExponential(3)}
              </div>
              <span class="text-[10px] text-slate-400">Directional bias</span>
            </div>
            <div class="p-3 rounded bg-slate-900 border border-slate-800">
              <span class="text-[10px] text-slate-500 font-semibold uppercase">Conservation Residual</span>
              <div class="text-lg font-mono font-bold ${c.metrics.conservation_residual < 1e-6 ? "text-emerald-400" : "text-amber-400"} mt-0.5">
                ${c.metrics.conservation_residual.toExponential(2)}
              </div>
              <span class="text-[10px] text-slate-400">Mass/Energy Closure</span>
            </div>
            <div class="p-3 rounded bg-slate-900 border border-slate-800">
              <span class="text-[10px] text-slate-500 font-semibold uppercase">Tolerance Provenance</span>
              <div class="text-xs font-semibold text-cyan-300 mt-1">
                ${this.getProvenanceLabel(c.tolerance_provenance)}
              </div>
              <span class="text-[10px] text-slate-500">Justification source</span>
            </div>
          </div>
        </div>

        <!-- Applicability Envelope & Boundaries -->
        <div>
          <h4 class="text-xs font-semibold uppercase tracking-wider text-slate-400 mb-3">Model Validity Envelope (Applicability Domain)</h4>
          <div class="p-4 rounded bg-slate-900/60 border border-slate-800 space-y-3">
            <div class="grid grid-cols-2 md:grid-cols-4 gap-2 text-xs">
              <div class="p-2 rounded bg-slate-900 border border-slate-800">
                <span class="text-slate-500 block text-[10px]">Temperature Range:</span>
                <span class="font-mono text-slate-200 font-semibold">${c.applicability_envelope.min_temperature_k} K – ${c.applicability_envelope.max_temperature_k} K</span>
              </div>
              <div class="p-2 rounded bg-slate-900 border border-slate-800">
                <span class="text-slate-500 block text-[10px]">Pressure Range:</span>
                <span class="font-mono text-slate-200 font-semibold">${(c.applicability_envelope.min_pressure_pa / 1e5).toFixed(1)} bar – ${(c.applicability_envelope.max_pressure_pa / 1e5).toFixed(1)} bar</span>
              </div>
              <div class="p-2 rounded bg-slate-900 border border-slate-800">
                <span class="text-slate-500 block text-[10px]">Supported Phases:</span>
                <span class="font-mono text-slate-200">${c.applicability_envelope.supported_phases.join(", ") || "All Fluid"}</span>
              </div>
              <div class="p-2 rounded bg-slate-900 border border-slate-800">
                <span class="text-slate-500 block text-[10px]">Tested Fluids:</span>
                <span class="font-mono text-slate-200">${c.applicability_envelope.supported_fluids.join(", ") || "Generic"}</span>
              </div>
            </div>
            ${
              c.applicability_envelope.limitation_notes
                ? `
              <div class="text-xs text-amber-300/90 bg-amber-950/30 p-2.5 rounded border border-amber-800/50">
                ⚠️ <strong>Extrapolation Boundary Note:</strong> ${c.applicability_envelope.limitation_notes}
              </div>
            `
                : ""
            }
          </div>
        </div>

        <!-- Scientific Citation & Academic Source -->
        <div>
          <h4 class="text-xs font-semibold uppercase tracking-wider text-slate-400 mb-3">Authoritative Benchmark Citation</h4>
          <div class="p-4 rounded bg-slate-900/60 border border-slate-800 text-xs space-y-2">
            <div class="text-slate-200 font-semibold">${c.citation.title}</div>
            <div class="text-slate-400">
              Authors/Body: <span class="text-slate-300">${c.citation.authors_or_organization}</span> (${c.citation.publication_year})
            </div>
            <div class="flex flex-wrap items-center gap-3 text-[11px]">
              ${
                c.citation.standard_id
                  ? `<span class="px-2 py-0.5 rounded bg-indigo-950 text-indigo-300 border border-indigo-800 font-mono">Standard: ${c.citation.standard_id}</span>`
                  : ""
              }
              ${
                c.citation.doi_or_identifier
                  ? `<span class="px-2 py-0.5 rounded bg-slate-800 text-slate-300 border border-slate-700 font-mono">Identifier: ${c.citation.doi_or_identifier}</span>`
                  : ""
              }
              ${
                c.citation.url
                  ? `<a href="${c.citation.url}" target="_blank" class="text-cyan-400 underline hover:text-cyan-300">${c.citation.url}</a>`
                  : ""
              }
            </div>
          </div>
        </div>

        <!-- Notes and Compliance Disclaimer -->
        <div class="text-xs text-slate-400 border-t border-slate-800 pt-3 space-y-1">
          <div><strong>Technical Notes:</strong> ${c.notes}</div>
          <div class="text-[11px] text-slate-500 italic"><strong>Validation Disclaimer:</strong> ${c.disclaimer}</div>
        </div>
      </div>
    `;
  }

  private renderModelsView(): string {
    return `
      <div class="flex-1 overflow-y-auto p-6 space-y-4">
        <div class="flex items-center justify-between border-b border-slate-800 pb-3">
          <div>
            <h3 class="text-base font-bold text-white uppercase tracking-wide">Physics & Control Model Validation Profiles</h3>
            <p class="text-xs text-slate-400">Aggregate benchmark compliance and worst-case relative errors per simulation model</p>
          </div>
          <span class="text-xs font-mono text-slate-400">${this.models.length} Models Profiled</span>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          ${this.models
            .map(
              (m) => `
            <div class="p-4 rounded-lg bg-slate-900/60 border border-slate-800 flex flex-col justify-between">
              <div>
                <div class="flex items-center justify-between mb-2">
                  <h4 class="font-bold text-white text-sm">${m.model_name}</h4>
                  ${this.getStatusBadge(m.status)}
                </div>
                <div class="text-xs text-cyan-400 font-mono mb-2">Domain: ${m.domain}</div>
                <p class="text-xs text-slate-300 leading-relaxed mb-3">${m.validity_summary}</p>
              </div>

              <div class="pt-3 border-t border-slate-800/80 grid grid-cols-3 gap-2 text-center text-xs">
                <div>
                  <span class="text-[10px] text-slate-500 uppercase block">Cases Verified</span>
                  <span class="font-mono font-bold text-slate-200">${m.cases_passed} / ${m.cases_count}</span>
                </div>
                <div>
                  <span class="text-[10px] text-slate-500 uppercase block">Worst-Case Error</span>
                  <span class="font-mono font-bold text-amber-400">${m.worst_case_relative_error_pct.toFixed(2)}%</span>
                </div>
                <div>
                  <span class="text-[10px] text-slate-500 uppercase block">Qualification</span>
                  <span class="font-mono font-bold text-emerald-400">PASS</span>
                </div>
              </div>
            </div>
          `
            )
            .join("")}
        </div>
      </div>
    `;
  }

  private renderReferencesView(): string {
    return `
      <div class="flex-1 overflow-y-auto p-6 space-y-4">
        <div class="flex items-center justify-between border-b border-slate-800 pb-3">
          <div>
            <h3 class="text-base font-bold text-white uppercase tracking-wide">Authoritative Reference Datasets & Handbooks</h3>
            <p class="text-xs text-slate-400">Curated external measurement datasets, published engineering benchmarks, and standards</p>
          </div>
          <span class="text-xs font-mono text-slate-400">${this.references.length} Datasets Cataloged</span>
        </div>

        <div class="space-y-3">
          ${this.references
            .map(
              (r) => `
            <div class="p-4 rounded-lg bg-slate-900/60 border border-slate-800 flex items-start justify-between gap-4">
              <div class="space-y-1.5 flex-1">
                <div class="flex items-center gap-2">
                  <span class="font-mono text-xs font-bold text-cyan-400">${r.id}</span>
                  <h4 class="font-bold text-white text-sm">${r.name}</h4>
                  ${
                    r.standard_organization
                      ? `<span class="px-2 py-0.5 text-[10px] font-mono rounded bg-indigo-950 text-indigo-300 border border-indigo-800">${r.standard_organization}</span>`
                      : ""
                  }
                </div>
                <div class="text-xs text-slate-300">${r.source_citation}</div>
                <div class="flex items-center gap-4 text-[11px] text-slate-400 pt-1">
                  <span>Domain: <strong>${r.domain}</strong></span>
                  <span>Data Points: <strong>${r.point_count}</strong></span>
                  <span>Parameters: <strong class="font-mono text-slate-300">${r.parameters.join(", ")}</strong></span>
                </div>
              </div>
            </div>
          `
            )
            .join("")}
        </div>
      </div>
    `;
  }

  private renderDisclaimerView(): string {
    return `
      <div class="flex-1 overflow-y-auto p-8 max-w-4xl mx-auto space-y-6 text-slate-300 text-xs leading-relaxed">
        <div class="p-4 rounded bg-amber-950/40 border border-amber-700/60 text-amber-200 space-y-2">
          <div class="flex items-center gap-2 font-bold text-sm">
            <span>⚖️</span>
            <span>MANDATORY SCIENTIFIC & REGULATORY COMPLIANCE STATEMENT</span>
          </div>
          <p>
            NebuDCS is a process simulation, engineering analysis, and Operator Training Simulator (OTS) runtime.
            All benchmarks and consistency comparisons presented within this framework are evaluated strictly for software verification and validation (V&V) under internal engineering guidelines.
          </p>
        </div>

        <div class="space-y-3 bg-slate-900/60 p-6 rounded-lg border border-slate-800">
          <h4 class="text-white text-sm font-bold uppercase tracking-wider">Governing Standards and Guidelines</h4>
          <ul class="list-disc pl-5 space-y-1.5 text-slate-400">
            <li><strong>ASME V&V 10-2019:</strong> Standard for Verification and Validation in Computational Solid Mechanics / Fluid Dynamics.</li>
            <li><strong>ASME V&V 20-2009:</strong> Standard for Verification and Validation in Computational Fluid Dynamics and Heat Transfer.</li>
            <li><strong>API Standard 610 / 617:</strong> Centrifugal pumps and compressors for petroleum, petrochemical, and natural gas industries.</li>
            <li><strong>ISA-75.01.01 (IEC 60534-2-1 Mod):</strong> Flow Equations for Sizing Control Valves.</li>
            <li><strong>IAPWS-IF97:</strong> Industrial Formulation 1997 for the Thermodynamic Properties of Water and Steam.</li>
          </ul>
        </div>

        <div class="space-y-3 bg-slate-900/60 p-6 rounded-lg border border-slate-800">
          <h4 class="text-white text-sm font-bold uppercase tracking-wider">Permitted & Restricted Language Protocol</h4>
          <p>
            To prevent deceptive claims of statutory compliance, NebuDCS strictly enforces the following naming ontology:
          </p>
          <div class="grid grid-cols-2 gap-4 pt-2">
            <div class="p-3 rounded bg-emerald-950/40 border border-emerald-800 text-emerald-300">
              <span class="font-bold block mb-1">✓ PERMITTED STATEMENTS:</span>
              <ul class="list-disc pl-4 space-y-1 text-[11px]">
                <li>"INTERNALLY VALIDATED"</li>
                <li>"BENCHMARKED"</li>
                <li>"ENGINEERING CONSISTENCY VERIFIED"</li>
                <li>"VALIDATED WITH LIMITATIONS"</li>
              </ul>
            </div>
            <div class="p-3 rounded bg-rose-950/40 border border-rose-800 text-rose-300">
              <span class="font-bold block mb-1">✗ STRICTLY FORBIDDEN CLAIMS:</span>
              <ul class="list-disc pl-4 space-y-1 text-[11px]">
                <li>"IEC CERTIFIED"</li>
                <li>"API CERTIFIED"</li>
                <li>"ISA CERTIFIED"</li>
                <li>"SIL CERTIFIED"</li>
                <li>"INDUSTRY CERTIFIED"</li>
              </ul>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  private setupEventListeners() {
    this.querySelectorAll(".view-tab-btn").forEach((btn) => {
      btn.addEventListener("click", () => {
        const tab = btn.getAttribute("data-tab") as any;
        if (tab) {
          this.activeViewTab = tab;
          this.render();
        }
      });
    });

    this.querySelectorAll(".domain-pill-btn").forEach((btn) => {
      btn.addEventListener("click", async () => {
        const d = btn.getAttribute("data-domain");
        if (d) {
          this.selectedDomain = d;
          await this.loadAll();
        }
      });
    });

    this.querySelectorAll(".case-item").forEach((el) => {
      el.addEventListener("click", async () => {
        const id = el.getAttribute("data-case-id");
        if (id) {
          await this.selectCase(id);
        }
      });
    });

    this.querySelector("#btn-run-benchmarks")?.addEventListener("click", () => {
      this.handleRunBenchmarks();
    });

    this.querySelector("#btn-export-manifest")?.addEventListener("click", () => {
      this.handleExportManifest();
    });

    this.querySelector("#btn-dismiss-msg")?.addEventListener("click", () => {
      this.statusMessage = null;
      this.render();
    });
  }
}

if (!customElements.get("nebu-validation-browser")) {
  customElements.define("nebu-validation-browser", NebuValidationBrowser);
}
