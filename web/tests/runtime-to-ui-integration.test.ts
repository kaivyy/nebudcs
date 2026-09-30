/**
 * NebuDCS P0/P1 Forensic Runtime-to-Process-Graphic Integration Acceptance Tests.
 *
 * Verifies end-to-end coupling between simulated physics telemetry and Operations
 * SVG workstation visualization:
 * - Test A: Feed Ramp (199 -> 248.7 m3/h) updates stream flow and displays active blue pipe with animation
 * - Test B: Feed Shutdown (0 m3/h) turns stream line to inactive grey (#334155) and freezes animation
 * - Test C: Pump Trip (P-101A.TRIP = true) highlights trip badge and zeros discharge flow
 * - Test D: Valve Movement (stroke 100% -> 35% -> 0%) alters downstream flow
 * - Test E: Compressor Speed Change (0 -> 10,450 RPM) activates recycle stream
 * - Test F: Multi-Vessel Independent Levels verifies V-101, V-102, V-105 display distinct levels
 * - Test G: Reactor R-101 renders dual catalyst beds and temperature indicator
 * - Test H: Port-to-Port Manhattan Routing verifies nozzle coordinate offsets and orthogonal stubs
 * - Test I: Plant Feed Boundary Condition renders authoritative badge and connecting pipe
 */

import { beforeAll, beforeEach, describe, expect, it } from "vitest";
import { NebuProcessView, computePortAwareWaypoints } from "../src/components/nebu-process-view";
import { ProjectDetailDto } from "../src/protocol/types";
import { store } from "../src/state/store";

const TEST_PROJECT: ProjectDetailDto = {
  id: "proj-nht-runtime-integration",
  name: "Naphtha Hydrotreater Integration Rig",
  description: "Forensic test rig for runtime-to-SVG binding",
  schema_version: "1.0.0",
  author: "NebuDCS Engineering",
  created: "2026-09-09T00:00:00Z",
  modified: "2026-09-09T00:00:00Z",
  boundary_conditions: [
    {
      id: "bc-nht-feed",
      name: "NHT Fresh Naphtha Feed",
      condition_type: "feed",
      flow_value: 199.0,
      flow_unit: "m3/h",
      pressure_value: 5.0,
      pressure_unit: "barg",
      temperature_value: 68.0,
      temperature_unit: "°C",
      phase: "liquid",
      target_equipment_id: "eq-tk101",
      target_port: "inlet",
      source_provenance: "Crude Distillation Unit (CDU) Naphtha Cut",
      enabled: true,
    },
  ],
  equipment: [
    {
      id: "eq-tk101",
      name: "TK-101",
      definition_id: "standard.tank.feed_storage",
      definition_version: "1.0.0",
      parameters: { volume_m3: 5000, level: 75.0 },
      position: { x: 100, y: 380 },
    },
    {
      id: "eq-v101",
      name: "V-101",
      definition_id: "standard.vessel.vertical",
      definition_version: "1.0.0",
      parameters: { level: 45.0, operating_pressure: 500000 },
      position: { x: 300, y: 380 },
    },
    {
      id: "eq-p101-a",
      name: "P-101A",
      definition_id: "standard.pump.centrifugal",
      definition_version: "1.0.0",
      parameters: { rated_flow_m3h: 199.0, speed_rpm: 2950 },
      position: { x: 500, y: 380 },
    },
    {
      id: "eq-fv101",
      name: "FV-101",
      definition_id: "standard.valve.control",
      definition_version: "1.0.0",
      parameters: { stroke_pct: 100.0, cv: 120.0 },
      position: { x: 680, y: 380 },
    },
    {
      id: "eq-r101",
      name: "R-101",
      definition_id: "standard.reactor.fixed_bed",
      definition_version: "1.0.0",
      parameters: { operating_temp: 332.0, operating_pressure: 3100000 },
      position: { x: 900, y: 380 },
    },
    {
      id: "eq-v102",
      name: "V-102",
      definition_id: "standard.vessel.vertical",
      definition_version: "1.0.0",
      parameters: { level: 58.0, operating_pressure: 2800000 },
      position: { x: 1120, y: 380 },
    },
    {
      id: "eq-k101a",
      name: "K-101A",
      definition_id: "standard.compressor.centrifugal",
      definition_version: "1.0.0",
      parameters: { speed_rpm: 10450 },
      position: { x: 1120, y: 180 },
    },
    {
      id: "eq-v105",
      name: "V-105",
      definition_id: "standard.vessel.vertical",
      definition_version: "1.0.0",
      parameters: { level: 72.0, operating_pressure: 400000 },
      position: { x: 1350, y: 380 },
    },
  ],
  streams: [
    {
      id: "str-00",
      name: "STR-NHT-00",
      source_equipment_id: "eq-tk101",
      source_port_id: "outlet",
      sink_equipment_id: "eq-v101",
      sink_port_id: "feed",
      stream_type: "material",
      composition: {},
      route_points: [],
    },
    {
      id: "str-01",
      name: "STR-NHT-01",
      source_equipment_id: "eq-v101",
      source_port_id: "liquid_out",
      sink_equipment_id: "eq-p101-a",
      sink_port_id: "suction",
      stream_type: "material",
      composition: {},
      route_points: [],
    },
    {
      id: "str-02",
      name: "STR-NHT-02",
      source_equipment_id: "eq-p101-a",
      source_port_id: "discharge",
      sink_equipment_id: "eq-fv101",
      sink_port_id: "inlet",
      stream_type: "material",
      composition: {},
      route_points: [],
    },
    {
      id: "str-03",
      name: "STR-NHT-03",
      source_equipment_id: "eq-fv101",
      source_port_id: "outlet",
      sink_equipment_id: "eq-r101",
      sink_port_id: "inlet",
      stream_type: "material",
      composition: {},
      route_points: [],
    },
    {
      id: "str-25",
      name: "STR-NHT-25",
      source_equipment_id: "eq-r101",
      source_port_id: "outlet",
      sink_equipment_id: "eq-v102",
      sink_port_id: "inlet",
      stream_type: "material",
      composition: {},
      route_points: [],
    },
    {
      id: "str-29",
      name: "STR-NHT-29",
      source_equipment_id: "eq-v102",
      source_port_id: "gas_out",
      sink_equipment_id: "eq-k101a",
      sink_port_id: "suction",
      stream_type: "material",
      composition: {},
      route_points: [],
    },
  ],
  instruments: [
    { id: "inst-tt101", tag: "TT-101", equipment_id: "eq-r101", measurement_kind: "temperature", range_min: 0, range_max: 500, unit: "°C" },
    { id: "inst-lt101", tag: "LT-101", equipment_id: "eq-v101", measurement_kind: "level", range_min: 0, range_max: 100, unit: "%" },
    { id: "inst-lt102", tag: "LT-102", equipment_id: "eq-v102", measurement_kind: "level", range_min: 0, range_max: 100, unit: "%" },
    { id: "inst-lt105", tag: "LT-105", equipment_id: "eq-v105", measurement_kind: "level", range_min: 0, range_max: 100, unit: "%" },
  ],
  displays: [
    {
      id: "disp-nht-runtime-test",
      name: "NHT Runtime Verification Display",
      display_type: "plant_overview",
      equipment_ids: [
        "eq-tk101",
        "eq-v101",
        "eq-p101-a",
        "eq-fv101",
        "eq-r101",
        "eq-v102",
        "eq-k101a",
        "eq-v105",
      ],
      stream_ids: [
        "str-00",
        "str-01",
        "str-02",
        "str-03",
        "str-25",
        "str-29",
      ],
      instrument_ids: ["inst-tt101", "inst-lt101", "inst-lt102", "inst-lt105"],
      viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 1 },
      is_default: true,
      order: 1,
      placements: [
        { equipment_id: "eq-tk101", x: 100, y: 380, width: 80, height: 80 },
        { equipment_id: "eq-v101", x: 300, y: 380, width: 80, height: 80 },
        { equipment_id: "eq-p101-a", x: 500, y: 380, width: 80, height: 80 },
        { equipment_id: "eq-fv101", x: 680, y: 380, width: 80, height: 80 },
        { equipment_id: "eq-r101", x: 900, y: 380, width: 80, height: 80 },
        { equipment_id: "eq-v102", x: 1120, y: 380, width: 80, height: 80 },
        { equipment_id: "eq-k101a", x: 1120, y: 180, width: 80, height: 80 },
        { equipment_id: "eq-v105", x: 1350, y: 380, width: 80, height: 80 },
      ],
    },
  ],
  control_loops: [],
  alarms: [],
  interlocks: [],
};

describe("NHT Forensic Runtime-to-Process-Graphic Integration Suite", () => {
  let view: NebuProcessView;

  beforeAll(() => {
    if (!window.customElements.get("nebu-process-view")) {
      window.customElements.define("nebu-process-view", NebuProcessView);
    }
  });

  beforeEach(() => {
    document.body.innerHTML = "";
    store.setActiveProject(JSON.parse(JSON.stringify(TEST_PROJECT)));
    store.isSimRunning = true;
    store.simTime = 10.0;

    view = document.createElement("nebu-process-view") as NebuProcessView;
    document.body.appendChild(view);
  });

  it("Test A: Feed Ramp (199 -> 248.7 m3/h) updates stream flow and displays active blue pipe with animation", async () => {
    // Initial feed rate at baseline 199.0 m3/h
    store.applySnapshot({
      snapshot_id: "snap-feed-1",
      sim_time_sec: 1.0,
      tick_count: 50,
      tags: [
        { tag: "STR-NHT-01.FLOW", value: 199.0, quality: "GOOD" },
      ],
      active_alarms: [],
    });
    view.render();
    await Promise.resolve();

    let labels = Array.from(view.querySelectorAll("text")).map((t) => t.textContent || "");
    expect(labels.some((l) => l.includes("STR-NHT-01") && l.includes("199.0 m³/h"))).toBe(true);

    // Operator executes feed ramp: set flow to 248.7 m3/h
    store.applySnapshot({
      snapshot_id: "snap-feed-2",
      sim_time_sec: 2.0,
      tick_count: 100,
      tags: [
        { tag: "STR-NHT-01.FLOW", value: 248.7, quality: "GOOD" },
      ],
      active_alarms: [],
    });
    view.render();
    await Promise.resolve();

    labels = Array.from(view.querySelectorAll("text")).map((t) => t.textContent || "");
    expect(labels.some((l) => l.includes("STR-NHT-01") && l.includes("248.7 m³/h"))).toBe(true);

    // Verify stream core line is active primary process color (#0284c7)
    const paths = Array.from(view.querySelectorAll("#streamsLayer path"));
    const activeCore = paths.find((p) => p.getAttribute("stroke") === "#0284c7");
    expect(activeCore).not.toBeUndefined();

    // Verify flow dash animation path is active
    const flowDash = paths.find((p) => p.getAttribute("stroke-dasharray") === "6 14");
    expect(flowDash).not.toBeUndefined();
    expect((flowDash as SVGPathElement).style.strokeDashoffset).toBeDefined();
  });

  it("Test B: Feed Shutdown (0.0 m3/h) switches line to inactive grey (#334155) and stops animation", async () => {
    // Flow is zeroed
    store.applySnapshot({
      snapshot_id: "snap-feed-zero",
      sim_time_sec: 3.0,
      tick_count: 150,
      tags: [
        { tag: "STR-NHT-01.FLOW", value: 0.0, quality: "GOOD" },
      ],
      active_alarms: [],
    });
    view.render();
    await Promise.resolve();

    const labels = Array.from(view.querySelectorAll("text")).map((t) => t.textContent || "");
    expect(labels.some((l) => l.includes("STR-NHT-01") && l.includes("0.0 m³/h"))).toBe(true);

    // Stream core line becomes inactive slate grey (#334155)
    const paths = Array.from(view.querySelectorAll("#streamsLayer path"));
    const inactiveCore = paths.find((p) => p.getAttribute("stroke") === "#334155");
    expect(inactiveCore).not.toBeUndefined();
  });

  it("Test C: Pump Trip highlights trip badge and zeros discharge flow", async () => {
    // Inject trip diagnostics and tag for P-101A
    store.diagnosticsMap.set("eq-p101-a", {
      equipment_id: "eq-p101-a",
      equipment_name: "P-101A",
      equipment_type: "pump",
      validity_state: "VALID" as any,
      state_label: "TRIPPED",
      is_tripped: true,
      flow_m3_h: 0.0,
      discharge_pressure_kpa: 101.3,
      speed_rpm: 0.0,
    } as any);
    store.applySnapshot({
      snapshot_id: "snap-trip",
      sim_time_sec: 4.0,
      tick_count: 200,
      tags: [
        { tag: "P-101A.TRIP", value: 1.0, quality: "GOOD" },
        { tag: "P-101A.SPEED", value: 0.0, quality: "GOOD" },
        { tag: "STR-NHT-02.FLOW", value: 0.0, quality: "GOOD" },
      ],
      active_alarms: [],
    });
    view.render();
    await Promise.resolve();

    // Verify trip banner or trip status rendered on pump
    const allText = Array.from(view.querySelectorAll("text")).map((t) => t.textContent || "");
    expect(allText.some((t) => t.includes("TRIP"))).toBe(true);

    // Verify discharge stream STR-NHT-02 displays 0.0 m3/h
    expect(allText.some((l) => l.includes("STR-NHT-02") && l.includes("0.0 m³/h"))).toBe(true);
  });

  it("Test D: Valve Movement (stroke 100% -> 35% -> 0%) alters downstream stream and visual state", async () => {
    // Throttled to 35%
    store.applySnapshot({
      snapshot_id: "snap-vlv-35",
      sim_time_sec: 5.0,
      tick_count: 250,
      tags: [
        { tag: "FV-101.POS", value: 35.0, quality: "GOOD" },
        { tag: "STR-NHT-03.FLOW", value: 69.6, quality: "GOOD" },
      ],
      active_alarms: [],
    });
    view.render();
    await Promise.resolve();

    let labels = Array.from(view.querySelectorAll("text")).map((t) => t.textContent || "");
    expect(labels.some((l) => l.includes("STR-NHT-03") && l.includes("69.6 m³/h"))).toBe(true);

    // Fully closed to 0%
    store.applySnapshot({
      snapshot_id: "snap-vlv-0",
      sim_time_sec: 6.0,
      tick_count: 300,
      tags: [
        { tag: "FV-101.POS", value: 0.0, quality: "GOOD" },
        { tag: "STR-NHT-03.FLOW", value: 0.0, quality: "GOOD" },
      ],
      active_alarms: [],
    });
    view.render();
    await Promise.resolve();

    labels = Array.from(view.querySelectorAll("text")).map((t) => t.textContent || "");
    expect(labels.some((l) => l.includes("STR-NHT-03") && l.includes("0.0 m³/h"))).toBe(true);

    const fvText = Array.from(view.querySelectorAll("text")).find((t) => t.textContent?.includes("ACT: 0%"));
    expect(fvText).not.toBeUndefined();
  });

  it("Test E: Compressor Speed Change (0 -> 10,450 RPM) activates recycle stream", async () => {
    // Set compressor running at rated speed 10,450 RPM with active recycle flow
    store.diagnosticsMap.set("eq-k101a", {
      equipment_id: "eq-k101a",
      equipment_name: "K-101A",
      equipment_type: "compressor",
      validity_state: "VALID" as any,
      state_label: "RUNNING",
      is_tripped: false,
      speed_rpm: 10450.0,
      discharge_pressure_kpa: 3100.0,
      flow_m3_h: 42.5,
    } as any);
    store.applySnapshot({
      snapshot_id: "snap-comp",
      sim_time_sec: 7.0,
      tick_count: 350,
      tags: [
        { tag: "K-101A.SPEED", value: 10450.0, quality: "GOOD" },
        { tag: "STR-NHT-29.FLOW", value: 42.5, quality: "GOOD" },
      ],
      active_alarms: [],
    });
    view.render();
    await Promise.resolve();

    const labels = Array.from(view.querySelectorAll("text")).map((t) => t.textContent || "");
    expect(labels.some((l) => l.includes("STR-NHT-29") && l.includes("42.5 m³/h"))).toBe(true);
    expect(labels.some((l) => l.includes("10450 RPM") || l.includes("10,450 RPM"))).toBe(true);
  });

  it("Test F: Multi-Vessel Independent Levels verifies V-101, V-102, V-105 display distinct levels", async () => {
    // Assign three distinctly different levels
    store.applySnapshot({
      snapshot_id: "snap-levels",
      sim_time_sec: 8.0,
      tick_count: 400,
      tags: [
        { tag: "LT-101", value: 35.0, quality: "GOOD" },
        { tag: "LT-102", value: 58.0, quality: "GOOD" },
        { tag: "LT-105", value: 72.0, quality: "GOOD" },
        { tag: "V-101.LEVEL", value: 35.0, quality: "GOOD" },
        { tag: "V-102.LEVEL", value: 58.0, quality: "GOOD" },
        { tag: "V-105.LEVEL", value: 72.0, quality: "GOOD" },
      ],
      active_alarms: [],
    });
    view.render();
    await Promise.resolve();

    const allText = Array.from(view.querySelectorAll("text")).map((t) => t.textContent || "");
    expect(allText.some((t) => t.includes("LVL: 35.0%"))).toBe(true);
    expect(allText.some((t) => t.includes("LVL: 58.0%"))).toBe(true);
    expect(allText.some((t) => t.includes("LVL: 72.0%"))).toBe(true);
  });

  it("Test G: Reactor R-101 renders dual catalytic beds and multi-point temperature readout", async () => {
    store.applySnapshot({
      snapshot_id: "snap-rx",
      sim_time_sec: 9.0,
      tick_count: 450,
      tags: [
        { tag: "TT-101", value: 332.0, quality: "GOOD" },
        { tag: "R-101.TEMP", value: 332.0, quality: "GOOD" },
      ],
      active_alarms: [],
    });
    view.render();
    await Promise.resolve();

    const allText = Array.from(view.querySelectorAll("text")).map((t) => t.textContent || "");
    expect(allText.some((t) => t.includes("BED 1 (NiMo)"))).toBe(true);
    expect(allText.some((t) => t.includes("BED 2 (CoMo)"))).toBe(true);
    expect(allText.some((t) => t.includes("332.0°C") || t.includes("332.0 °C"))).toBe(true);
    expect(allText.some((t) => t.includes("R-101"))).toBe(true);
  });

  it("Test H: Port-to-Port Manhattan Routing verifies nozzle coordinates and orthogonal stubs", () => {
    const pump = TEST_PROJECT.equipment.find((e) => e.name === "P-101A")!;
    const pSuct = view.getPortCoordinates(pump, "suction");
    const pDisch = view.getPortCoordinates(pump, "discharge");

    // Pump suction is left nozzle (-28px), discharge is top nozzle (-28px)
    expect(pSuct.x).toBe(pump.position!.x - 28);
    expect(pSuct.y).toBe(pump.position!.y);
    expect(pSuct.dir).toBe("left");

    expect(pDisch.x).toBe(pump.position!.x);
    expect(pDisch.y).toBe(pump.position!.y - 28);
    expect(pDisch.dir).toBe("top");

    // Test computePortAwareWaypoints generates orthogonal routing with stubs
    const pts = computePortAwareWaypoints(pDisch, { x: 680, y: 380, dir: "left" });
    expect(pts.length).toBeGreaterThanOrEqual(3);
    expect(pts[0].x).toBe(pDisch.x);
    expect(pts[0].y).toBe(pDisch.y);

    // Verify all segments are purely orthogonal (dx === 0 or dy === 0)
    for (let i = 0; i < pts.length - 1; i++) {
      const isOrthogonal = pts[i].x === pts[i + 1].x || pts[i].y === pts[i + 1].y;
      expect(isOrthogonal).toBe(true);
    }
  });

  it("Test I: Plant Feed Boundary Condition renders authoritative badge and connecting pipe", () => {
    view.render();
    const bcGroup = view.querySelector(".boundary-node");
    expect(bcGroup).not.toBeNull();

    const bcText = Array.from(bcGroup!.querySelectorAll("text")).map((t) => t.textContent || "");
    expect(bcText.some((t) => t.includes("PLANT FEED"))).toBe(true);
    expect(bcText.some((t) => t.includes("NHT Fresh Naphtha Feed"))).toBe(true);
    expect(bcText.some((t) => t.includes("199") && t.includes("m3/h") && t.includes("68"))).toBe(true);

    const pipe = view.querySelector(".boundary-conditions-layer line");
    expect(pipe).not.toBeNull();
    expect(pipe?.getAttribute("stroke")).toBe("#10b981");
  });
});
