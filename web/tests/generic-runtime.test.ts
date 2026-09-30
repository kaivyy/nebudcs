/**
 * Generic Runtime Proof Tests
 *
 * Proves that NebuDCS Operator Station works with ANY valid project,
 * not just the baseline K-101/FV-101/V-101/LIC-101 demonstration project.
 * A custom project (TK-900/P-ALPHA/E-HEATER-7/V-SEP-X) is used as the
 * primary test fixture.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { Store } from "../src/state/store";
import { ProjectDetailDto } from "../src/protocol/types";

// ─── CUSTOM Project Fixture (TK-900 / P-ALPHA / E-HEATER-7 / V-SEP-X) ─────
// Deliberately uses IDs that are completely different from the baseline project
const CUSTOM_PROJECT: ProjectDetailDto = {
  id: "proj-custom-alpha-001",
  name: "Alpha Separation Train",
  schema_version: "1.0.0",
  description: "Custom test project — no K-101, LIC-101, FV-101, or V-101 instances",
  author: "Test Engineer",
  created: "2026-01-01T00:00:00Z",
  modified: "2026-01-01T00:00:00Z",
  equipment: [
    {
      id: "eq-palpha",
      definition_id: "standard.pump.centrifugal",
      definition_version: "1.0.0",
      name: "P-ALPHA",
      parameters: { design_flow: 3000, rated_head: 80 },
      position: { x: 100, y: 200 },
    },
    {
      id: "eq-eheater7",
      definition_id: "standard.heat_exchanger.shell_tube",
      definition_version: "1.0.0",
      name: "E-HEATER-7",
      parameters: { duty: 2000 },
      position: { x: 300, y: 200 },
    },
    {
      id: "eq-vsepx",
      definition_id: "standard.vessel.vertical_separator",
      definition_version: "1.0.0",
      name: "V-SEP-X",
      parameters: { volume: 40 },
      position: { x: 550, y: 180 },
    },
  ],
  streams: [
    {
      id: "s-01",
      name: "FEED",
      stream_type: "Material",
      source_equipment_id: "eq-palpha",
      source_port_id: "discharge",
      sink_equipment_id: "eq-eheater7",
      sink_port_id: "shell_inlet",
      composition: {},
      route_points: [
        { x: 140, y: 200 },
        { x: 260, y: 200 },
      ],
    },
  ],
  instruments: [
    {
      id: "i-ti900",
      tag: "TI-900",
      equipment_id: "eq-palpha",
      measurement_kind: "Temperature",
      range_min: 0,
      range_max: 300,
      unit: "°C",
      position: { x: 200, y: 140 },
    },
    {
      id: "i-pi900",
      tag: "PI-900",
      equipment_id: "eq-palpha",
      measurement_kind: "Pressure",
      range_min: 0,
      range_max: 50,
      unit: "bar",
      position: { x: 260, y: 140 },
    },
    {
      id: "i-li900",
      tag: "LI-900",
      equipment_id: "eq-vsepx",
      measurement_kind: "Level",
      range_min: 0,
      range_max: 100,
      unit: "%",
      position: { x: 580, y: 120 },
    },
  ],
  control_loops: [
    {
      id: "cl-lic900",
      name: "LIC-900",
      pv_tag: "LI-900",
      sp: 50,
      out_tag: "LCV-900.POS",
      kp: 1.5,
      ti: 20,
      td: 0.5,
      reverse_acting: false,
      mode: "AUTO",
      output_min: 0,
      output_max: 100,
      position: { x: 700, y: 200 },
    },
    {
      id: "cl-tic900",
      name: "TIC-900",
      pv_tag: "TI-900",
      sp: 180,
      out_tag: "HCV-900.POS",
      kp: 0.8,
      ti: 30,
      td: 0,
      reverse_acting: false,
      mode: "AUTO",
      output_min: 0,
      output_max: 100,
      position: { x: 700, y: 280 },
    },
  ],
  alarms: [
    {
      id: "a-lah900",
      name: "LAH-900",
      pv_tag: "LI-900",
      alarm_type: "Hi",
      severity: "Critical",
      trip_limit: 80,
      deadband: 1,
      description: "Separator level high",
    },
    {
      id: "a-tah900",
      name: "TAH-900",
      pv_tag: "TI-900",
      alarm_type: "Hi",
      severity: "Warning",
      trip_limit: 250,
      deadband: 5,
      description: "Feed temperature high",
    },
  ],
  interlocks: [],
};

// ─── BASELINE Project Fixture (K-101 / FV-101 / V-101 / LIC-101) ─────────
const BASELINE_PROJECT: ProjectDetailDto = {
  id: "proj-vertical-slice-01",
  name: "NebuDCS Closed-Loop Refinery Unit",
  schema_version: "1.0.0",
  description: "Standard Level-Controlled Surge Loop (baseline demo)",
  author: "NebuDCS Engineering Studio",
  created: "2026-09-06T00:00:00Z",
  modified: "2026-09-06T00:00:00Z",
  equipment: [
    {
      id: "eq-k101",
      definition_id: "standard.pump.centrifugal",
      definition_version: "1.0.0",
      name: "K-101",
      parameters: { design_flow: 5000 },
      position: { x: 100, y: 180 },
    },
    {
      id: "eq-fv101",
      definition_id: "standard.valve.control_globe",
      definition_version: "1.0.0",
      name: "FV-101",
      parameters: { cv: 85 },
      position: { x: 360, y: 95 },
    },
    {
      id: "eq-v101",
      definition_id: "standard.vessel.vertical_separator",
      definition_version: "1.0.0",
      name: "V-101",
      parameters: { volume: 25 },
      position: { x: 520, y: 140 },
    },
  ],
  streams: [],
  instruments: [
    {
      id: "i-ti101",
      tag: "TI-101",
      equipment_id: "eq-k101",
      measurement_kind: "Temperature",
      range_min: 0,
      range_max: 250,
      unit: "°C",
      position: { x: 190, y: 70 },
    },
  ],
  control_loops: [
    {
      id: "cl-lic101",
      name: "LIC-101",
      pv_tag: "V-101.LEVEL",
      sp: 50,
      out_tag: "FV-101.POS",
      kp: 1.5,
      ti: 20,
      td: 2,
      reverse_acting: false,
      mode: "AUTO",
      output_min: 0,
      output_max: 100,
      position: { x: 450, y: 20 },
    },
  ],
  alarms: [
    {
      id: "a-lah101",
      name: "LAH-101",
      pv_tag: "V-101.LEVEL",
      alarm_type: "Hi",
      severity: "Critical",
      trip_limit: 80,
      deadband: 1,
      description: "Level high",
    },
  ],
  interlocks: [],
};

// ────────────────────────────────────────────────────────────────────────────
describe("Generic Runtime — Custom Project Proof", () => {
  let s: Store;

  beforeEach(() => {
    s = new Store();
  });

  it("initializes with null faceplate and empty trend tags before project load", () => {
    expect(s.selectedFaceplate).toBeNull();
    expect(s.selectedTrendTags).toHaveLength(0);
    expect(s.selectedLoop).toBe("");
  });

  it("baseline project: loads and resolves LIC-101 loop correctly", () => {
    s.setActiveProject(BASELINE_PROJECT);
    expect(s.selectedFaceplate?.id).toBe("LIC-101");
    expect(s.selectedFaceplate?.pvTag).toBe("V-101.LEVEL");
    expect(s.selectedTrendTags.some((t) => t.includes("V-101.LEVEL") || t.includes("LIC-101"))).toBe(true);
  });

  it("custom project: loads and resolves LIC-900 (no K-101 reference)", () => {
    s.setActiveProject(CUSTOM_PROJECT);
    expect(s.selectedFaceplate?.id).toBe("LIC-900");
    expect(s.selectedFaceplate?.pvTag).toBe("LI-900");
    expect(s.selectedFaceplate?.id).not.toBe("LIC-101");
    expect(s.selectedTrendTags.some((t) => t.includes("K-101"))).toBe(false);
    expect(s.selectedTrendTags.some((t) => t.includes("LIC-101"))).toBe(false);
  });

  it("custom project: selectedTrendTags contain LIC-900 pv/sp/out", () => {
    s.setActiveProject(CUSTOM_PROJECT);
    // Should include LI-900 (pv_tag), LIC-900.SP, LCV-900.POS
    expect(s.selectedTrendTags).toContain("LI-900");
    expect(s.selectedTrendTags).toContain("LCV-900.POS");
  });

  it("custom project: alarm LAH-900 resolves to LIC-900 entity via pv_tag=LI-900", () => {
    s.setActiveProject(CUSTOM_PROJECT);
    const target = s.getEntityForAlarm("LAH-900");
    expect(target).not.toBeNull();
    expect(target?.kind).toBe("control_loop");
    expect(target?.id).toBe("LIC-900");
    expect(target?.pvTag).toBe("LI-900");
  });

  it("custom project: alarm TAH-900 resolves to TIC-900 entity", () => {
    s.setActiveProject(CUSTOM_PROJECT);
    const target = s.getEntityForAlarm("TAH-900");
    expect(target?.id).toBe("TIC-900");
  });

  it("custom project: unknown alarm returns null (graceful unavailable state)", () => {
    s.setActiveProject(CUSTOM_PROJECT);
    expect(s.getEntityForAlarm("ALM_NONEXISTENT")).toBeNull();
    expect(s.getEntityForAlarm("K-101")).toBeNull();
    expect(s.getEntityForAlarm("LIC-101")).toBeNull();
  });

  it("custom project: getEntityByPvTag resolves PI-900 to instrument entity", () => {
    s.setActiveProject(CUSTOM_PROJECT);
    const e = s.getEntityByPvTag("PI-900");
    expect(e?.kind).toBe("instrument");
    expect(e?.name).toBe("PI-900");
    expect(e?.rangeMax).toBe(50);
  });

  it("custom project: getEntityByPvTag resolves LI-900 to control_loop (LIC-900.pv_tag)", () => {
    s.setActiveProject(CUSTOM_PROJECT);
    const e = s.getEntityByPvTag("LI-900");
    expect(e?.kind).toBe("control_loop");
    expect(e?.id).toBe("LIC-900");
  });

  it("derived store state never references K-101, FV-101, or LIC-101", () => {
    s.setActiveProject(CUSTOM_PROJECT);
    // Serializing the faceplate and trend tags must not contain baseline IDs
    const faceplateStr = JSON.stringify(s.selectedFaceplate);
    const trendStr = JSON.stringify(s.selectedTrendTags);
    expect(faceplateStr).not.toContain("K-101");
    expect(faceplateStr).not.toContain("LIC-101");
    expect(faceplateStr).not.toContain("FV-101");
    expect(trendStr).not.toContain("K-101");
    expect(trendStr).not.toContain("LIC-101");
  });

  it("baseline project also works correctly (regression guard)", () => {
    s.setActiveProject(BASELINE_PROJECT);
    const target = s.getEntityForAlarm("LAH-101");
    expect(target?.kind).toBe("control_loop");
    expect(target?.id).toBe("LIC-101");
    expect(target?.pvTag).toBe("V-101.LEVEL");
  });
});
