/**
 * Stage 13 Scientific Validation Browser Web Component Test Suite.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { client } from "../src/api/client";
import {
  ValidationExecutiveSummaryDto,
  ValidationCaseSummaryDto,
  ValidationCaseDetailDto,
  ReferenceDatasetDto,
  ModelValidationSummaryDto,
  ValidationManifestDto,
} from "../src/protocol/types";
import "../src/components/nebu-validation-browser";
import { NebuValidationBrowser } from "../src/components/nebu-validation-browser";

const MOCK_SUMMARY: ValidationExecutiveSummaryDto = {
  total_cases: 20,
  passed_cases: 20,
  failed_cases: 0,
  pass_rate_pct: 100.0,
  overall_status: "VALIDATED",
  domain_counts: [
    ["Thermodynamics", 4, 4],
    ["Reactions", 3, 3],
    ["Hydraulics", 4, 4],
    ["Heat Transfer", 3, 3],
    ["Regulatory Control", 1, 1],
    ["IEC 61131-3", 1, 1],
    ["Functional Safety", 1, 1],
    ["Steam & Power", 1, 1],
    ["Conservation", 2, 2],
  ],
  classification_counts: [
    ["StandardReference", 6],
    ["EngineeringReference", 4],
    ["PublishedBenchmark", 4],
    ["EngineeringConsistency", 4],
    ["AnalyticalReference", 2],
  ],
  timestamp_iso: "2026-09-08T12:00:00Z",
  engine_version: "v0.1.0",
  compliance_disclaimer:
    "All benchmark qualifications are calculated internally. Strictly no claim of external accreditation by IEC, API, ISA, or SIL certification bodies.",
};

const MOCK_CASES: ValidationCaseSummaryDto[] = [
  {
    id: "BENCH-TH-001",
    name: "Pure Methane Compressibility Factor Z (NIST Reference)",
    domain: "Thermodynamics",
    tested_feature: "Cubic EOS / Peng-Robinson Z-factor",
    classification: "PUBLIC_DATASET",
    status: "VALIDATED",
    standard_id: "NIST-NSRDS-NBS-10",
    passed: true,
    max_relative_error_pct: 0.42,
    relative_tolerance_pct: 1.5,
    last_executed_iso: "2026-09-08T12:00:00Z",
  },
  {
    id: "BENCH-HY-001",
    name: "Darcy-Weisbach Friction Factor (Crane TP-410)",
    domain: "Hydraulics",
    tested_feature: "Churchill friction factor across Re regimes",
    classification: "ENGINEERING_REFERENCE",
    status: "BENCHMARKED",
    standard_id: "CRANE-TP-410",
    passed: true,
    max_relative_error_pct: 0.85,
    relative_tolerance_pct: 2.0,
    last_executed_iso: "2026-09-08T12:00:00Z",
  },
];

const MOCK_DETAIL: ValidationCaseDetailDto = {
  id: "BENCH-TH-001",
  name: "Pure Methane Compressibility Factor Z (NIST Reference)",
  domain: "Thermodynamics",
  tested_feature: "Cubic EOS / Peng-Robinson Z-factor",
  model_name: "Peng-Robinson EOS",
  classification: "PUBLIC_DATASET",
  status: "VALIDATED",
  tolerance_provenance: "SOURCE_DEFINED",
  citation: {
    title: "Thermophysical Properties of Methane",
    authors_or_organization: "Friend, D. G., et al. (NIST)",
    publication_year: 1989,
    doi_or_identifier: "10.1063/1.555828",
    standard_id: "NIST-NSRDS-NBS-10",
    url: "https://www.nist.gov/srd",
  },
  applicability_envelope: {
    min_temperature_k: 100.0,
    max_temperature_k: 500.0,
    min_pressure_pa: 100000.0,
    max_pressure_pa: 20000000.0,
    supported_phases: ["Vapor", "Supercritical"],
    supported_fluids: ["CH4"],
    limitation_notes: "Non-polar fluid correlation; not valid for aqueous systems.",
  },
  relative_tolerance_pct: 1.5,
  absolute_tolerance: 0.01,
  metrics: {
    mean_absolute_error: 0.0028,
    root_mean_squared_error: 0.0034,
    max_absolute_deviation: 0.0041,
    max_relative_error_pct: 0.42,
    mean_bias_error: -0.0012,
    conservation_residual: 0.0,
  },
  passed: true,
  last_executed_iso: "2026-09-08T12:00:00Z",
  execution_duration_ms: 2.3,
  notes: "Peng-Robinson matches experimental NIST isobars well within industrial threshold.",
  disclaimer:
    "Evaluated for internal simulation qualification only. Not certified by NIST.",
};

const MOCK_MODELS: ModelValidationSummaryDto[] = [
  {
    model_name: "Peng-Robinson EOS",
    domain: "Thermodynamics",
    status: "VALIDATED",
    cases_count: 4,
    cases_passed: 4,
    worst_case_relative_error_pct: 1.2,
    validity_summary: "Rigorous cubic EOS for hydrocarbons and non-polar light gases.",
  },
];

const MOCK_REFERENCES: ReferenceDatasetDto[] = [
  {
    id: "REF-NIST-METHANE",
    name: "NIST NSRDS Methane Thermophysical Properties",
    domain: "Thermodynamics",
    source_citation: "NIST Standard Reference Database 69, 2023",
    standard_organization: "NIST",
    point_count: 100,
    parameters: ["T", "P", "Z", "Density"],
  },
];

const MOCK_MANIFEST: ValidationManifestDto = {
  version: "1.0.0",
  generated_at_iso: "2026-09-08T12:00:00Z",
  system_id: "nebudcs-sim-core-v0.1.0",
  summary: MOCK_SUMMARY,
  cases: [MOCK_DETAIL],
  models: MOCK_MODELS,
  references: MOCK_REFERENCES,
  disclaimers: [MOCK_SUMMARY.compliance_disclaimer],
};

describe("NebuValidationBrowser Web Component (Stage 13)", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.spyOn(client, "getValidationSummary").mockResolvedValue(MOCK_SUMMARY);
    vi.spyOn(client, "listValidationCases").mockResolvedValue(MOCK_CASES);
    vi.spyOn(client, "getValidationCase").mockResolvedValue(MOCK_DETAIL);
    vi.spyOn(client, "listValidationModels").mockResolvedValue(MOCK_MODELS);
    vi.spyOn(client, "listValidationReferences").mockResolvedValue(MOCK_REFERENCES);
    vi.spyOn(client, "getValidationManifest").mockResolvedValue(MOCK_MANIFEST);
    vi.spyOn(client, "runValidationBenchmarks").mockResolvedValue({
      executed_cases: 20,
      passed_cases: 20,
      failed_cases: 0,
      total_duration_ms: 18.5,
      cases: MOCK_CASES,
      summary: MOCK_SUMMARY,
    });
  });

  it("should mount and load executive summary and benchmark cases", async () => {
    const el = document.createElement("nebu-validation-browser") as NebuValidationBrowser;
    document.body.appendChild(el);

    // Wait for microtasks / component initialization
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(client.getValidationSummary).toHaveBeenCalled();
    expect(client.listValidationCases).toHaveBeenCalled();
    expect(client.getValidationCase).toHaveBeenCalledWith("BENCH-TH-001");

    // Check Executive KPI card contents
    expect(el.innerHTML).toContain("Scientific Validation &amp; Benchmark Framework");
    expect(el.innerHTML).toContain("INTERNALLY VALIDATED");
    expect(el.innerHTML).toContain("100.0%");
    expect(el.innerHTML).toContain("BENCH-TH-001");
    expect(el.innerHTML).toContain("Crane TP-410");

    document.body.removeChild(el);
  });

  it("should inspect detailed metrics and applicability envelope", async () => {
    const el = document.createElement("nebu-validation-browser") as NebuValidationBrowser;
    document.body.appendChild(el);
    await new Promise((resolve) => setTimeout(resolve, 50));

    // Error metrics
    expect(el.innerHTML).toContain("Max Relative Error");
    expect(el.innerHTML).toContain("0.4200%");
    expect(el.innerHTML).toContain("Mean Absolute Error");
    expect(el.innerHTML).toContain("Root Mean Squared");

    // Applicability envelope
    expect(el.innerHTML).toContain("100 K – 500 K");
    expect(el.innerHTML).toContain("Authoritative Benchmark Citation");
    expect(el.innerHTML).toContain("NIST-NSRDS-NBS-10");

    document.body.removeChild(el);
  });

  it("should switch tabs between Cases, Models, References, and Governance Disclaimer", async () => {
    const el = document.createElement("nebu-validation-browser") as NebuValidationBrowser;
    document.body.appendChild(el);
    await new Promise((resolve) => setTimeout(resolve, 50));

    // Click Models tab
    const modelsBtn = el.querySelector('[data-tab="models"]') as HTMLButtonElement;
    expect(modelsBtn).toBeTruthy();
    modelsBtn.click();
    expect(el.innerHTML).toContain("Physics &amp; Control Model Validation Profiles");
    expect(el.innerHTML).toContain("Peng-Robinson EOS");

    // Click References tab
    const refsBtn = el.querySelector('[data-tab="references"]') as HTMLButtonElement;
    expect(refsBtn).toBeTruthy();
    refsBtn.click();
    expect(el.innerHTML).toContain("Authoritative Reference Datasets &amp; Handbooks");
    expect(el.innerHTML).toContain("REF-NIST-METHANE");

    // Click Disclaimer tab
    const discBtn = el.querySelector('[data-tab="disclaimer"]') as HTMLButtonElement;
    expect(discBtn).toBeTruthy();
    discBtn.click();
    expect(el.innerHTML).toContain("MANDATORY SCIENTIFIC &amp; REGULATORY COMPLIANCE STATEMENT");
    expect(el.innerHTML).toContain("STRICTLY FORBIDDEN CLAIMS");
    expect(el.innerHTML).toContain("IEC CERTIFIED");
    expect(el.innerHTML).toContain("API CERTIFIED");

    document.body.removeChild(el);
  });

  it("should trigger Run Scientific Benchmarks action and update UI", async () => {
    const el = document.createElement("nebu-validation-browser") as NebuValidationBrowser;
    document.body.appendChild(el);
    await new Promise((resolve) => setTimeout(resolve, 50));

    const runBtn = el.querySelector("#btn-run-benchmarks") as HTMLButtonElement;
    expect(runBtn).toBeTruthy();
    runBtn.click();

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(client.runValidationBenchmarks).toHaveBeenCalled();
    expect(el.innerHTML).toContain("Benchmark suite completed: 20 executed, 20 passed");

    document.body.removeChild(el);
  });
});
