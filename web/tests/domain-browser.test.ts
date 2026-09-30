/**
 * Stage 12 Domain Browser Web Component Test Suite.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { client } from "../src/api/client";
import { DomainPackageDetailDto, DomainPackageSummaryDto } from "../src/protocol/types";
import "../src/components/nebu-domain-browser";
import { NebuDomainBrowser } from "../src/components/nebu-domain-browser";

const MOCK_DOMAINS: DomainPackageSummaryDto[] = [
  {
    id: "domain-refinery-cdu",
    name: "Refinery Crude Distillation Unit (CDU)",
    version: "1.0.0",
    domain_kind: "Refinery",
    description: "Atmospheric crude distillation column model",
    author: "NebuDCS Process Engineering Core",
    strategy_count: 2,
    scenario_count: 2,
    template_count: 8,
    scientific_references: [
      {
        citation_key: "Nelson1958",
        title: "Petroleum Refinery Engineering",
        authors: ["W. L. Nelson"],
        publication_year: 1958,
        source: "McGraw-Hill",
        doi_or_url: undefined,
        governing_concept: "True Boiling Point (TBP) distillation curves",
        validity_domain: "Crude fractions 35-550 deg C",
      },
    ],
    model_classifications: [
      {
        subsystem: "Atmospheric Tower VLE",
        classification: "Theoretical",
        governing_equation: "y_i * P = x_i * P_i^sat (Raoult / SRK EOS)",
        primary_assumptions: ["Equilibrium stages", "Constant molar overflow"],
        conservation_verification: "Rigorous mass and thermal enthalpy balance residuals < 1e-4",
      },
    ],
  },
  {
    id: "domain-petrochemical-olefin",
    name: "Petrochemical Olefin / Steam Cracking Unit",
    version: "1.0.0",
    domain_kind: "Petrochemical",
    description: "Ethylene/propylene steam cracking and cryogenic fractionation",
    author: "NebuDCS Process Engineering Core",
    strategy_count: 2,
    scenario_count: 2,
    template_count: 6,
    scientific_references: [],
    model_classifications: [],
  },
];

const MOCK_CDU_DETAIL: DomainPackageDetailDto = {
  id: "domain-refinery-cdu",
  name: "Refinery Crude Distillation Unit (CDU)",
  version: "1.0.0",
  domain_kind: "Refinery",
  description: "Atmospheric crude distillation column model",
  author: "NebuDCS Process Engineering Core",
  operating_ranges: {
    column_pressure: {
      min_value: 101.3,
      max_value: 350.0,
      normal_operating_value: 140.0,
      unit: "kPa",
      quantity: "Pressure",
    },
    flash_zone_temp: {
      min_value: 200.0,
      max_value: 400.0,
      normal_operating_value: 360.0,
      unit: "°C",
      quantity: "Temperature",
    },
  },
  thermodynamic_requirements: [
    "Peng-Robinson EOS / SRK",
    "Hydrocarbon VLE fugacity coefficients",
  ],
  composition_defaults: {
    Naphtha: 0.20,
    Kerosene: 0.15,
    Diesel: 0.25,
    AGO: 0.15,
    Residue: 0.25,
  },
  utility_requirements: ["Cooling Water", "High Pressure Steam", "Fuel Gas"],
  control_strategies: [
    "Overhead reflux cascade temperature-to-flow controller",
    "Atmospheric bottoms level control",
  ],
  scenario_presets: [
    "Atmospheric Distillation Cold Startup",
    "Crude Charge Loss / Pre-heat Exchanger Fouling",
  ],
  equipment_templates: ["furnace_fired", "tower_atmospheric", "pump_centrifugal"],
  plant_template_ids: ["plant-cdu-flowsheet-01"],
  validation_rules: [
    "Tower top pressure must exceed 100 kPa(a) to avoid atmospheric air ingress",
  ],
  scientific_references: [
    {
      citation_key: "Nelson1958",
      title: "Petroleum Refinery Engineering",
      authors: ["W. L. Nelson"],
      publication_year: 1958,
      source: "McGraw-Hill",
      doi_or_url: undefined,
      governing_concept: "True Boiling Point (TBP) distillation curves",
      validity_domain: "Crude fractions 35-550 deg C",
    },
  ],
  model_classifications: [
    {
      subsystem: "Atmospheric Tower VLE",
      classification: "Theoretical",
      governing_equation: "y_i * P = x_i * P_i^sat (Raoult / SRK EOS)",
      primary_assumptions: ["Equilibrium stages", "Constant molar overflow"],
      conservation_verification: "Rigorous mass and thermal enthalpy balance residuals < 1e-4",
    },
  ],
};

describe("NebuDomainBrowser", () => {
  beforeEach(() => {
    vi.spyOn(client, "listDomainPackages").mockResolvedValue(MOCK_DOMAINS);
    vi.spyOn(client, "getDomainPackage").mockImplementation(async (id: string) => {
      if (id === "domain-refinery-cdu") {
        return MOCK_CDU_DETAIL;
      }
      return {
        ...MOCK_CDU_DETAIL,
        id,
        name: "Mock Domain",
      };
    });
  });

  it("renders domain browser and lists industrial domain packages", async () => {
    const el = document.createElement("nebu-domain-browser") as NebuDomainBrowser;
    document.body.appendChild(el);

    // Wait for connectedCallback promise
    await new Promise((resolve) => setTimeout(resolve, 80));

    expect(client.listDomainPackages).toHaveBeenCalled();
    expect(client.getDomainPackage).toHaveBeenCalledWith("domain-refinery-cdu");

    const text = el.textContent || "";
    expect(text).toContain("INDUSTRIAL DOMAINS & THERMODYNAMICS");
    expect(text).toContain("Refinery Crude Distillation Unit (CDU)");
    expect(text).toContain("Petrochemical Olefin / Steam Cracking Unit");

    document.body.removeChild(el);
  });

  it("renders scientific references and model classifications in provenance tab", async () => {
    const el = document.createElement("nebu-domain-browser") as NebuDomainBrowser;
    document.body.appendChild(el);
    await new Promise((resolve) => setTimeout(resolve, 80));

    // Switch to provenance tab
    const tabBtn = el.querySelector('button[data-tab="provenance"]') as HTMLButtonElement;
    expect(tabBtn).not.toBeNull();
    tabBtn.click();

    const text = el.textContent || "";
    expect(text).toContain("Petroleum Refinery Engineering");
    expect(text).toContain("W. L. Nelson");
    expect(text).toContain("Atmospheric Tower VLE");
    expect(text).toContain("Theoretical");

    document.body.removeChild(el);
  });

  it("renders safe operating envelope and thermodynamics tabs", async () => {
    const el = document.createElement("nebu-domain-browser") as NebuDomainBrowser;
    document.body.appendChild(el);
    await new Promise((resolve) => setTimeout(resolve, 80));

    // Switch to thermo tab
    const tabBtn = el.querySelector('button[data-tab="thermo"]') as HTMLButtonElement;
    expect(tabBtn).not.toBeNull();
    tabBtn.click();

    const text = el.textContent || "";
    expect(text).toContain("Peng-Robinson EOS / SRK");
    expect(text).toContain("Naphtha");
    expect(text).toContain("Diesel");

    document.body.removeChild(el);
  });
});
