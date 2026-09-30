/**
 * Phase 13 Engineering Studio Test Suite.
 *
 * Tests studio state store, command-based undo/redo, REST API integration,
 * P&ID canvas rendering, equipment palette, and property inspection.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { ApiClient } from "../src/api/client";
import {
  ProjectDetailDto,
  ProjectSummaryDto,
  ProjectValidationResponseDto,
  StudioEquipmentDto,
  StudioStreamDto,
  TemplateSummaryDto,
} from "../src/protocol/types";
import { studioStore } from "../src/state/studio-store";
import "../src/components/nebu-equipment-palette";
import "../src/components/nebu-pid-canvas";
import { resolveEquipmentArchetype } from "../src/components/nebu-property-inspector";
import "../src/components/nebu-project-explorer";
import "../src/components/nebu-validation-panel";
import "../src/components/nebu-studio";

const SAMPLE_PROJECT: ProjectDetailDto = {
  id: "proj-vertical-slice-01",
  name: "NebuDCS Process Engineering Vertical Slice",
  schema_version: "0.1.0",
  description: "Baseline surge loop with pump, valve, and vessel",
  author: "NebuDCS Engineering Studio Core",
  created: "2026-09-06T00:00:00Z",
  modified: "2026-09-06T00:00:00Z",
  equipment: [
    {
      id: "eq-pump-01",
      definition_id: "standard.pump.centrifugal",
      definition_version: "1.0.0",
      name: "K-101",
      parameters: { design_flow: 5000, rated_head: 120, speed_rpm: 2980 },
      position: { x: 100, y: 180 },
    },
    {
      id: "eq-valve-01",
      definition_id: "standard.valve.control_globe",
      definition_version: "1.0.0",
      name: "FV-101",
      parameters: { cv: 85, position: 55.2 },
      position: { x: 360, y: 95 },
    },
    {
      id: "eq-vessel-01",
      definition_id: "standard.vessel.vertical_separator",
      definition_version: "1.0.0",
      name: "V-101",
      parameters: { volume: 25, operating_pressure: 101325 },
      position: { x: 520, y: 140 },
    },
  ],
  streams: [
    {
      id: "st-01",
      name: "S-01",
      stream_type: "Material",
      source_equipment_id: "eq-pump-01",
      source_port_id: "discharge",
      sink_equipment_id: "eq-valve-01",
      sink_port_id: "inlet",
      composition: { H2O: 1.0 },
      route_points: [
        { x: 100, y: 180 },
        { x: 100, y: 95 },
        { x: 360, y: 95 },
      ],
    },
    {
      id: "st-02",
      name: "S-02",
      stream_type: "Material",
      source_equipment_id: "eq-valve-01",
      source_port_id: "outlet",
      sink_equipment_id: "eq-vessel-01",
      sink_port_id: "feed",
      composition: { H2O: 1.0 },
      route_points: [
        { x: 360, y: 95 },
        { x: 520, y: 95 },
        { x: 520, y: 140 },
      ],
    },
  ],
  instruments: [
    {
      id: "inst-01",
      tag: "PT-101",
      equipment_id: "eq-pump-01",
      port_id: "discharge",
      measurement_kind: "Pressure",
      range_min: 0,
      range_max: 50,
      unit: "bar",
      position: { x: 200, y: 60 },
    },
  ],
  control_loops: [
    {
      id: "loop-01",
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
      id: "alm-01",
      name: "LAH-101",
      pv_tag: "V-101.LEVEL",
      alarm_type: "HI",
      severity: "CRITICAL",
      trip_limit: 80,
      deadband: 1,
      description: "Level High",
    },
  ],
  interlocks: [
    {
      id: "ilk-01",
      name: "TRIP-K101",
      source_tag: "V-101.LEVEL",
      condition: "<",
      threshold: 10,
      target_action: "TRIP",
      target_equipment: "eq-pump-01",
      latched: true,
      description: "Low level pump trip",
    },
  ],
};

const SAMPLE_TEMPLATES: TemplateSummaryDto[] = [
  {
    id: "standard.pump.centrifugal",
    name: "Centrifugal Pump",
    version: "1.0.0",
    category: "Rotating",
    source: "standard",
    description: "Centrifugal process pump with head and power curves",
    parameter_count: 3,
    port_count: 3,
    curve_count: 2,
    trip_count: 1,
    author: "NebuDCS Standard Library",
  },
  {
    id: "standard.valve.control_globe",
    name: "Control Globe Valve",
    version: "1.0.0",
    category: "Valve",
    source: "standard",
    description: "Equal percentage / linear globe control valve",
    parameter_count: 2,
    port_count: 3,
    curve_count: 1,
    trip_count: 0,
    author: "NebuDCS Standard Library",
  },
];

describe("Studio State Store & Commands", () => {
  beforeEach(() => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    studioStore.setTemplates(SAMPLE_TEMPLATES);
  });

  it("loads and clones project cleanly", () => {
    const proj = studioStore.getProject();
    expect(proj).toBeDefined();
    expect(proj?.id).toBe("proj-vertical-slice-01");
    expect(proj?.equipment.length).toBe(3);
    expect(proj?.streams.length).toBe(2);
    expect(studioStore.getIsDirty()).toBe(false);
    expect(studioStore.canUndo()).toBe(false);
    expect(studioStore.canRedo()).toBe(false);
  });

  it("executes AddEquipmentCommand and supports Undo/Redo", () => {
    const newEq: StudioEquipmentDto = {
      id: "eq-compressor-01",
      definition_id: "standard.compressor.centrifugal",
      definition_version: "1.0.0",
      name: "C-201",
      parameters: { speed_rpm: 9000 },
      position: { x: 600, y: 200 },
    };

    studioStore.addEquipment(newEq);
    expect(studioStore.getProject()?.equipment.length).toBe(4);
    expect(studioStore.getIsDirty()).toBe(true);
    expect(studioStore.canUndo()).toBe(true);

    // Undo
    expect(studioStore.undo()).toBe(true);
    expect(studioStore.getProject()?.equipment.length).toBe(3);
    expect(studioStore.canRedo()).toBe(true);

    // Redo
    expect(studioStore.redo()).toBe(true);
    expect(studioStore.getProject()?.equipment.length).toBe(4);
  });

  it("moves equipment and supports undo/redo", () => {
    studioStore.moveEquipment("eq-pump-01", { x: 150, y: 220 });
    let pump = studioStore.getProject()?.equipment.find((e) => e.id === "eq-pump-01");
    expect(pump?.position).toEqual({ x: 150, y: 220 });
    expect(studioStore.canUndo()).toBe(true);

    // Undo move
    studioStore.undo();
    pump = studioStore.getProject()?.equipment.find((e) => e.id === "eq-pump-01");
    expect(pump?.position).toEqual({ x: 100, y: 180 });

    // Redo move
    studioStore.redo();
    pump = studioStore.getProject()?.equipment.find((e) => e.id === "eq-pump-01");
    expect(pump?.position).toEqual({ x: 150, y: 220 });
  });

  it("updates equipment parameters and cascades undo", () => {
    studioStore.updateEquipmentParameters("eq-pump-01", { design_flow: 7500, rated_head: 140 });
    let pump = studioStore.getProject()?.equipment.find((e) => e.id === "eq-pump-01");
    expect(pump?.parameters.design_flow).toBe(7500);

    studioStore.undo();
    pump = studioStore.getProject()?.equipment.find((e) => e.id === "eq-pump-01");
    expect(pump?.parameters.design_flow).toBe(5000);
  });

  it("wires and deletes streams with connected port state", () => {
    const newStream: StudioStreamDto = {
      id: "st-03",
      name: "S-03",
      stream_type: "Material",
      source_equipment_id: "eq-vessel-01",
      source_port_id: "liquid_out",
      sink_equipment_id: "eq-pump-01",
      sink_port_id: "suction",
      composition: {},
      route_points: [
        { x: 520, y: 140 },
        { x: 100, y: 180 },
      ],
    };

    studioStore.addStream(newStream);
    expect(studioStore.getProject()?.streams.length).toBe(3);

    // Delete stream
    studioStore.deleteStream("st-03");
    expect(studioStore.getProject()?.streams.length).toBe(2);

    // Undo delete
    studioStore.undo();
    expect(studioStore.getProject()?.streams.length).toBe(3);
  });

  it("deleting equipment cascades to connected streams and instruments", () => {
    studioStore.deleteEquipment("eq-pump-01");
    const proj = studioStore.getProject();
    expect(proj?.equipment.some((e) => e.id === "eq-pump-01")).toBe(false);
    // S-01 connected to eq-pump-01 should be removed
    expect(proj?.streams.some((s) => s.id === "st-01")).toBe(false);
    // PT-101 attached to eq-pump-01 should be removed
    expect(proj?.instruments.some((i) => i.id === "inst-01")).toBe(false);

    // Undo cascades back
    studioStore.undo();
    const restored = studioStore.getProject();
    expect(restored?.equipment.some((e) => e.id === "eq-pump-01")).toBe(true);
    expect(restored?.streams.some((s) => s.id === "st-01")).toBe(true);
    expect(restored?.instruments.some((i) => i.id === "inst-01")).toBe(true);
  });

  it("handles entity selection and port connection draft lifecycle", () => {
    studioStore.selectItem("equipment", "eq-pump-01");
    expect(studioStore.getSelected()).toEqual({ type: "equipment", id: "eq-pump-01" });

    studioStore.startConnection("eq-pump-01", "discharge", "Discharge Port");
    expect(studioStore.getConnectionDraft()?.sourcePortId).toBe("discharge");

    studioStore.cancelConnection();
    expect(studioStore.getConnectionDraft()).toBeNull();

    studioStore.clearSelection();
    expect(studioStore.getSelected()).toBeNull();
  });
});

describe("ApiClient Studio & Project Endpoints", () => {
  it("fetches project list", async () => {
    const client = new ApiClient();
    const mockList: ProjectSummaryDto[] = [
      {
        id: "proj-vertical-slice-01",
        name: "Baseline Plant",
        description: "Vertical slice",
        author: "NebuDCS",
        schema_version: "0.1.0",
        plant_count: 1,
        equipment_count: 3,
        stream_count: 2,
        created: "2026-09-06T00:00:00Z",
        modified: "2026-09-06T00:00:00Z",
      },
    ];

    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => mockList,
    });

    const projects = await client.listProjects();
    expect(projects.length).toBe(1);
    expect(projects[0].id).toBe("proj-vertical-slice-01");
    expect(global.fetch).toHaveBeenCalledWith("/api/v1/projects", expect.any(Object));
  });

  it("validates project and parses diagnostics response", async () => {
    const client = new ApiClient();
    const mockVal: ProjectValidationResponseDto = {
      valid: true,
      error_count: 0,
      warning_count: 1,
      info_count: 2,
      review_count: 0,
      results: [
        {
          severity: "warning",
          code: "VLD-STR04",
          message: "Composition fractions missing",
        },
      ],
    };

    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => mockVal,
    });

    const val = await client.validateProject("proj-vertical-slice-01");
    expect(val.valid).toBe(true);
    expect(val.error_count).toBe(0);
    expect(val.results.length).toBe(1);
  });

  it("synchronizes project into live simulation engine", async () => {
    const client = new ApiClient();
    const mockResp = {
      status: "ok",
      message: "Project synchronized",
      project_id: "proj-vertical-slice-01",
    };

    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => mockResp,
    });

    const res = await client.loadProjectSim("proj-vertical-slice-01");
    expect(res.status).toBe("ok");
    expect(res.project_id).toBe("proj-vertical-slice-01");
  });
});

describe("Web Components Rendering", () => {
  beforeEach(() => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    studioStore.setTemplates(SAMPLE_TEMPLATES);
  });

  it("renders equipment palette with category filtering and template cards", () => {
    const el = document.createElement("nebu-equipment-palette");
    document.body.appendChild(el);

    const cards = el.querySelectorAll(".template-card");
    expect(cards.length).toBe(2);

    const chips = el.querySelectorAll(".cat-chip");
    expect(chips.length).toBe(6);

    el.remove();
  });

  it("renders SVG P&ID canvas with equipment nodes, ports, and pipes", () => {
    const el = document.createElement("nebu-pid-canvas");
    document.body.appendChild(el);

    const svg = el.querySelector(".pid-svg");
    expect(svg).toBeDefined();

    const eqNodes = el.querySelectorAll(".eq-node");
    expect(eqNodes.length).toBe(3);

    const portPins = el.querySelectorAll(".port-pin");
    expect(portPins.length).toBeGreaterThanOrEqual(6);

    const pipes = el.querySelectorAll(".process-pipe");
    expect(pipes.length).toBe(2);

    const instNodes = el.querySelectorAll(".instrument-node");
    expect(instNodes.length).toBe(1);

    el.remove();
  });

  it("renders property inspector with editable fields", () => {
    const el = document.createElement("nebu-property-inspector");
    document.body.appendChild(el);

    // Initial empty state
    expect(el.querySelector(".inspector-empty")).toBeDefined();

    // Select pump equipment
    studioStore.selectItem("equipment", "eq-pump-01");

    const nameInput = el.querySelector(".input-eq-name") as HTMLInputElement;
    expect(nameInput?.value).toBe("K-101");

    const paramRows = el.querySelectorAll(".params-table tbody tr");
    expect(paramRows.length).toBe(3);

    el.remove();
  });

  it("renders validation panel diagnostics", () => {
    const el = document.createElement("nebu-validation-panel");
    document.body.appendChild(el);

    studioStore.setValidation({
      valid: true,
      error_count: 0,
      warning_count: 1,
      info_count: 0,
      review_count: 0,
      results: [
        {
          severity: "warning",
          code: "VLD-TEST01",
          message: "Advisory test diagnostic",
          entity_id: "eq-pump-01",
        },
      ],
    });

    const statusBadge = el.querySelector(".val-status-badge");
    expect(statusBadge?.textContent?.trim()).toBe("VALID");

    const locateBtn = el.querySelector(".btn-goto-entity");
    expect(locateBtn).toBeDefined();

    el.remove();
  });

  it("adds inline instrument to stream and updates instrument properties", () => {
    const inst = studioStore.addInlineInstrumentToStream("st-01", "Flow", "FT");
    expect(inst).toBeDefined();
    expect(inst?.measurement_kind).toBe("Flow");
    expect(inst?.unit).toBe("m3/h");
    expect(inst?.equipment_id).toBe("eq-pump-01");

    const proj = studioStore.getProject();
    expect(proj?.instruments.some((i) => i.id === inst?.id)).toBe(true);

    // Update instrument properties
    studioStore.updateInstrument(inst!.id, {
      tag: "FT-999",
      range_max: 200,
      unit: "kg/s",
    });

    const updated = studioStore.getProject()?.instruments.find((i) => i.id === inst!.id);
    expect(updated?.tag).toBe("FT-999");
    expect(updated?.range_max).toBe(200);
    expect(updated?.unit).toBe("kg/s");
  });

  it("constructs and wires an arbitrary non-refinery plant", () => {
    // Arbitrary plant: Vessel-A -> Valve-C3 -> Pump-Z9 -> Tank-B7
    const vesselA: StudioEquipmentDto = {
      id: "eq-vessel-a",
      definition_id: "standard.vessel.vertical_separator",
      definition_version: "1.0.0",
      name: "Vessel-A",
      parameters: { volume: 20 },
      position: { x: 100, y: 150 },
    };
    const valveC3: StudioEquipmentDto = {
      id: "eq-valve-c3",
      definition_id: "standard.valve.control_globe",
      definition_version: "1.0.0",
      name: "Valve-C3",
      parameters: { cv: 75 },
      position: { x: 280, y: 150 },
    };
    const pumpZ9: StudioEquipmentDto = {
      id: "eq-pump-z9",
      definition_id: "standard.pump.centrifugal",
      definition_version: "1.0.0",
      name: "Pump-Z9",
      parameters: { rated_flow: 80, rated_head: 60 },
      position: { x: 460, y: 150 },
    };
    const tankB7: StudioEquipmentDto = {
      id: "eq-tank-b7",
      definition_id: "standard.tank.atmospheric",
      definition_version: "1.0.0",
      name: "Tank-B7",
      parameters: { capacity_m3: 300 },
      position: { x: 640, y: 150 },
    };

    studioStore.addEquipment(vesselA);
    studioStore.addEquipment(valveC3);
    studioStore.addEquipment(pumpZ9);
    studioStore.addEquipment(tankB7);

    // Stream 1: Vessel-A (liquid_out) -> Valve-C3 (inlet)
    studioStore.addStream({
      id: "st-arb-01",
      name: "S-101",
      stream_type: "Material",
      source_equipment_id: "eq-vessel-a",
      source_port_id: "liquid_out",
      sink_equipment_id: "eq-valve-c3",
      sink_port_id: "inlet",
      composition: {},
      route_points: [
        { x: 100, y: 202 },
        { x: 190, y: 202 },
        { x: 190, y: 150 },
        { x: 252, y: 150 },
      ],
    });

    // Add inline flow instrument FT-X1 on stream 1
    const ftX1 = studioStore.addInlineInstrumentToStream("st-arb-01", "Flow", "FT");
    expect(ftX1).toBeDefined();
    studioStore.updateInstrument(ftX1!.id, { tag: "FT-X1" });

    // Stream 2: Valve-C3 (outlet) -> Pump-Z9 (suction)
    studioStore.addStream({
      id: "st-arb-02",
      name: "S-102",
      stream_type: "Material",
      source_equipment_id: "eq-valve-c3",
      source_port_id: "outlet",
      sink_equipment_id: "eq-pump-z9",
      sink_port_id: "suction",
      composition: {},
      route_points: [],
    });

    // Stream 3: Pump-Z9 (discharge) -> Tank-B7 (inlet)
    studioStore.addStream({
      id: "st-arb-03",
      name: "S-103",
      stream_type: "Material",
      source_equipment_id: "eq-pump-z9",
      source_port_id: "discharge",
      sink_equipment_id: "eq-tank-b7",
      sink_port_id: "inlet",
      composition: {},
      route_points: [],
    });

    const proj = studioStore.getProject();
    expect(proj?.equipment.length).toBe(7); // 3 original + 4 new
    expect(proj?.streams.length).toBe(5); // 2 original + 3 new
    expect(proj?.instruments.some((i) => i.tag === "FT-X1")).toBe(true);

    // Mount canvas to ensure proper rendering without exceptions
    const canvas = document.createElement("nebu-pid-canvas");
    document.body.appendChild(canvas);
    expect(canvas.querySelectorAll(".eq-node").length).toBe(7);
    expect(canvas.querySelectorAll(".stream-group").length).toBe(5);
    canvas.remove();
  });

  it("supports instrument movement with undo/redo", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    studioStore.moveInstrument("inst-01", { x: 280, y: 120 });
    let inst = studioStore.getProject()?.instruments.find((i) => i.id === "inst-01");
    expect(inst?.position).toEqual({ x: 280, y: 120 });
    expect(studioStore.canUndo()).toBe(true);

    studioStore.undo();
    inst = studioStore.getProject()?.instruments.find((i) => i.id === "inst-01");
    expect(inst?.position).toEqual({ x: 200, y: 60 });

    studioStore.redo();
    inst = studioStore.getProject()?.instruments.find((i) => i.id === "inst-01");
    expect(inst?.position).toEqual({ x: 280, y: 120 });
  });

  it("supports equipment resizing and undo/redo", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    studioStore.resizeEquipment("eq-pump-01", 120, 100);
    let pump = studioStore.getProject()?.equipment.find((e) => e.id === "eq-pump-01");
    expect(pump?.display_width).toBe(120);
    expect(pump?.display_height).toBe(100);
    expect(studioStore.canUndo()).toBe(true);

    studioStore.undo();
    pump = studioStore.getProject()?.equipment.find((e) => e.id === "eq-pump-01");
    expect(pump?.display_width).toBeUndefined();

    studioStore.redo();
    pump = studioStore.getProject()?.equipment.find((e) => e.id === "eq-pump-01");
    expect(pump?.display_width).toBe(120);
  });

  it("supports equipment rotation and undo/redo", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    studioStore.rotateEquipment("eq-pump-01", 90);
    let pump = studioStore.getProject()?.equipment.find((e) => e.id === "eq-pump-01");
    expect(pump?.rotation).toBe(90);

    studioStore.undo();
    pump = studioStore.getProject()?.equipment.find((e) => e.id === "eq-pump-01");
    expect(pump?.rotation).toBe(0);

    studioStore.redo();
    pump = studioStore.getProject()?.equipment.find((e) => e.id === "eq-pump-01");
    expect(pump?.rotation).toBe(90);
  });

  it("supports multi-selection and atomic moveEntities", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    studioStore.selectItem("equipment", "eq-pump-01");
    studioStore.selectItem("equipment", "eq-valve-01", true);

    expect(studioStore.getSelectedIds().size).toBe(2);
    expect(studioStore.isSelected("eq-pump-01")).toBe(true);
    expect(studioStore.isSelected("eq-valve-01")).toBe(true);

    studioStore.moveEntities([
      { id: "eq-pump-01", type: "equipment", pos: { x: 150, y: 200 } },
      { id: "eq-valve-01", type: "equipment", pos: { x: 400, y: 120 } },
    ]);

    let proj = studioStore.getProject();
    expect(proj?.equipment.find((e) => e.id === "eq-pump-01")?.position).toEqual({ x: 150, y: 200 });
    expect(proj?.equipment.find((e) => e.id === "eq-valve-01")?.position).toEqual({ x: 400, y: 120 });

    studioStore.undo();
    proj = studioStore.getProject();
    expect(proj?.equipment.find((e) => e.id === "eq-pump-01")?.position).toEqual({ x: 100, y: 180 });
    expect(proj?.equipment.find((e) => e.id === "eq-valve-01")?.position).toEqual({ x: 360, y: 95 });
  });

  it("supports alignment and distribution of selected equipment", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    studioStore.selectAllEquipment();
    expect(studioStore.getSelectedIds().size).toBe(3);

    // Align Left
    studioStore.alignSelected("left");
    let proj = studioStore.getProject();
    const minX = 100;
    expect(proj?.equipment.every((e) => e.position?.x === minX)).toBe(true);

    // Undo alignment
    studioStore.undo();
    proj = studioStore.getProject();
    expect(proj?.equipment.find((e) => e.id === "eq-valve-01")?.position?.x).toBe(360);

    // Align Top
    studioStore.alignSelected("top");
    proj = studioStore.getProject();
    const minY = 95;
    expect(proj?.equipment.every((e) => e.position?.y === minY)).toBe(true);

    // Distribute Vertically
    studioStore.undo();
    studioStore.distributeSelected("vertical");
    proj = studioStore.getProject();
    expect(proj).toBeDefined();
  });

  it("duplicates selected equipment with clean IDs and positions", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    studioStore.selectItem("equipment", "eq-pump-01");
    studioStore.duplicateSelected();

    const proj = studioStore.getProject();
    expect(proj?.equipment.length).toBe(4);
    const copy = proj?.equipment.find((e) => e.name === "K-101_COPY");
    expect(copy).toBeDefined();
    expect(copy?.position).toEqual({ x: 130, y: 210 });

    studioStore.undo();
    expect(studioStore.getProject()?.equipment.length).toBe(3);
  });

  it("strictly resolves equipment archetype from definition_id without name guessing", () => {
    expect(resolveEquipmentArchetype({ definition_id: "standard.pump.centrifugal", name: "arbitrary-tag-1" })).toBe("pump");
    expect(resolveEquipmentArchetype({ definition_id: "standard.compressor.centrifugal", name: "vessel-like-name" })).toBe("compressor");
    expect(resolveEquipmentArchetype({ definition_id: "standard.valve.control_globe", name: "unit_99" })).toBe("valve");
    expect(resolveEquipmentArchetype({ definition_id: "standard.column.fractionator", name: "box_01" })).toBe("column");
    expect(resolveEquipmentArchetype({ definition_id: "standard.vessel.vertical_separator", name: "he-101" })).toBe("vessel");
    expect(resolveEquipmentArchetype({ definition_id: "standard.heat_exchanger.shell_and_tube", name: "pump-tagged" })).toBe("heat_exchanger");
    expect(resolveEquipmentArchetype({ definition_id: "standard.heater.fired_process", name: "c-101" })).toBe("heater");
    expect(resolveEquipmentArchetype({ definition_id: "standard.reactor.cstr", name: "v-101" })).toBe("reactor");
    expect(resolveEquipmentArchetype({ definition_id: "custom.equipment.unknown", name: "P-101" })).toBe("generic");
  });

  it("renders authoritative plant structure tree in project explorer", () => {
    const explorer = document.createElement("nebu-project-explorer");
    document.body.appendChild(explorer);

    studioStore.setProject(SAMPLE_PROJECT, true);

    const plantRoot = explorer.querySelector("[data-node='plant-root']");
    expect(plantRoot).toBeDefined();

    const actionButtons = explorer.querySelectorAll(".btn-explorer-action");
    expect(actionButtons.length).toBeGreaterThanOrEqual(4);

    explorer.remove();
  });

  it("supports control loop and alarm deletion with transactional undo/redo in studioStore", () => {
    const projWithLoops: ProjectDetailDto = {
      ...SAMPLE_PROJECT,
      control_loops: [
        {
          id: "loop-01",
          name: "FIC-101",
          pv_tag: "FT-101.pv",
          sp: 100,
          out_tag: "FV-101.out",
          kp: 1.5,
          ti: 20,
          td: 0,
          reverse_acting: false,
          mode: "AUTO",
          output_min: 0,
          output_max: 100,
        },
      ],
      alarms: [
        {
          id: "alm-01",
          name: "PAH-101",
          pv_tag: "PT-101.pv",
          alarm_type: "HI",
          severity: "High",
          trip_limit: 45.0,
          deadband: 1.0,
          description: "High Pressure Alarm",
        },
      ],
    };

    studioStore.setProject(projWithLoops, true);
    expect(studioStore.getProject()?.control_loops.length).toBe(1);
    expect(studioStore.getProject()?.alarms.length).toBe(1);

    // Delete control loop
    studioStore.deleteControlLoop("loop-01");
    expect(studioStore.getProject()?.control_loops.length).toBe(0);

    // Undo delete loop
    studioStore.undo();
    expect(studioStore.getProject()?.control_loops.length).toBe(1);

    // Delete alarm
    studioStore.deleteAlarm("alm-01");
    expect(studioStore.getProject()?.alarms.length).toBe(0);

    // Undo delete alarm
    studioStore.undo();
    expect(studioStore.getProject()?.alarms.length).toBe(1);
  });

  it("initiates viewport panning on single left-click drag on canvas background", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    const canvas = document.createElement("nebu-pid-canvas") as any;
    document.body.appendChild(canvas);

    const bg = canvas.querySelector(".canvas-bg");
    expect(bg).not.toBeNull();

    // Normal left-click (button 0, no shift) on background initiates panning
    const downEvt = new MouseEvent("mousedown", {
      button: 0,
      clientX: 200,
      clientY: 150,
      bubbles: true,
      cancelable: true,
    });
    bg.dispatchEvent(downEvt);

    expect(canvas.panState).not.toBeNull();
    expect(canvas.panState.startX).toBe(200);
    expect(canvas.panState.startY).toBe(150);
    expect(canvas.style.cursor).toBe("grabbing");

    // Mousemove pans viewport transform
    const moveEvt = new MouseEvent("mousemove", {
      clientX: 250,
      clientY: 180,
      bubbles: true,
    });
    window.dispatchEvent(moveEvt);

    expect(canvas.panX).toBe(50);
    expect(canvas.panY).toBe(30);

    // Mouseup concludes panning
    const upEvt = new MouseEvent("mouseup", { bubbles: true });
    window.dispatchEvent(upEvt);
    expect(canvas.panState).toBeNull();
    canvas.remove();
  });

  it("initiates marquee selection on Shift + left-click drag on canvas background", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    const canvas = document.createElement("nebu-pid-canvas") as any;
    document.body.appendChild(canvas);

    const bg = canvas.querySelector(".canvas-bg");

    // Shift + left-click on background starts marquee
    const shiftDownEvt = new MouseEvent("mousedown", {
      button: 0,
      shiftKey: true,
      clientX: 100,
      clientY: 100,
      bubbles: true,
      cancelable: true,
    });
    bg.dispatchEvent(shiftDownEvt);

    expect(canvas.marqueeState).not.toBeNull();
    expect(canvas.panState).toBeNull();

    // Mouseup completes marquee
    const upEvt = new MouseEvent("mouseup", { bubbles: true });
    window.dispatchEvent(upEvt);
    expect(canvas.marqueeState).toBeNull();
    canvas.remove();
  });

  it("rotates selected equipment by 90 degrees on pressing R", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    const canvas = document.createElement("nebu-pid-canvas") as any;
    document.body.appendChild(canvas);

    studioStore.selectItem("equipment", "eq-pump-01");
    let pump = studioStore.getProject()?.equipment.find((e) => e.id === "eq-pump-01");
    expect(pump?.rotation ?? 0).toBe(0);

    // Press 'r'
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "r", bubbles: true }));
    pump = studioStore.getProject()?.equipment.find((e) => e.id === "eq-pump-01");
    expect(pump?.rotation).toBe(90);

    // Press 'r' again
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "r", bubbles: true }));
    pump = studioStore.getProject()?.equipment.find((e) => e.id === "eq-pump-01");
    expect(pump?.rotation).toBe(180);

    canvas.remove();
  });

  it("intercepts Ctrl+S to save active project", async () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    const studio = document.createElement("nebu-studio") as any;
    document.body.appendChild(studio);

    const saveSpy = vi.spyOn(studio, "saveActiveProject").mockImplementation(() => Promise.resolve());

    const ctrlSEvt = new KeyboardEvent("keydown", {
      key: "s",
      ctrlKey: true,
      bubbles: true,
      cancelable: true,
    });
    window.dispatchEvent(ctrlSEvt);

    expect(ctrlSEvt.defaultPrevented).toBe(true);
    expect(saveSpy).toHaveBeenCalled();

    studio.remove();
  });

  it("pans canvas vertically on standard mouse wheel scroll without modifiers", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    const canvas = document.createElement("nebu-pid-canvas") as any;
    document.body.appendChild(canvas);

    const initialPanY = canvas.panY;
    const svg = canvas.querySelector(".pid-svg");

    const wheelEvt = new WheelEvent("wheel", {
      deltaY: 50,
      deltaX: 0,
      ctrlKey: false,
      metaKey: false,
      shiftKey: false,
      bubbles: true,
      cancelable: true,
    });
    svg.dispatchEvent(wheelEvt);

    expect(wheelEvt.defaultPrevented).toBe(true);
    expect(canvas.panY).toBe(initialPanY - 50);

    canvas.remove();
  });

  it("pans canvas horizontally on Shift + mouse wheel scroll", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    const canvas = document.createElement("nebu-pid-canvas") as any;
    document.body.appendChild(canvas);

    const initialPanX = canvas.panX;
    const svg = canvas.querySelector(".pid-svg");

    const shiftWheelEvt = new WheelEvent("wheel", {
      deltaY: 40,
      deltaX: 0,
      bubbles: true,
      cancelable: true,
    });
    Object.defineProperty(shiftWheelEvt, "shiftKey", { value: true });
    svg.dispatchEvent(shiftWheelEvt);

    expect(shiftWheelEvt.defaultPrevented).toBe(true);
    expect(canvas.panX).toBe(initialPanX - 40);

    canvas.remove();
  });

  it("zooms canvas on Ctrl + mouse wheel scroll", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    const canvas = document.createElement("nebu-pid-canvas") as any;
    document.body.appendChild(canvas);

    const initialScale = canvas.scale;
    const svg = canvas.querySelector(".pid-svg");

    // Mock getBoundingClientRect
    svg.getBoundingClientRect = () => ({
      left: 0,
      top: 0,
      width: 1000,
      height: 700,
      right: 1000,
      bottom: 700,
      x: 0,
      y: 0,
      toJSON: () => {},
    });

    const zoomWheelEvt = new WheelEvent("wheel", {
      deltaY: -100, // scroll up = zoom in
      bubbles: true,
      cancelable: true,
    });
    Object.defineProperty(zoomWheelEvt, "ctrlKey", { value: true });
    svg.dispatchEvent(zoomWheelEvt);

    expect(zoomWheelEvt.defaultPrevented).toBe(true);
    expect(canvas.scale).toBeGreaterThan(initialScale);

    canvas.remove();
  });

  it("renders project explorer tree with flex containment and overflow-y-auto for desktop scrolling", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    const explorer = document.createElement("nebu-project-explorer") as any;
    document.body.appendChild(explorer);

    const shell = explorer.querySelector(".project-explorer-shell");
    expect(shell).not.toBeNull();
    expect(shell.classList.contains("min-h-0")).toBe(true);

    const tree = explorer.querySelector(".explorer-tree");
    expect(tree).not.toBeNull();
    expect(tree.classList.contains("overflow-y-auto")).toBe(true);
    expect(tree.classList.contains("min-h-0")).toBe(true);

    explorer.remove();
  });

  it("toggles fullscreen mode on studio shell and updates button label", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    const studio = document.createElement("nebu-studio") as any;
    document.body.appendChild(studio);
    studio.render();

    const fsBtn = studio.querySelector(".btn-fullscreen-toggle");
    expect(fsBtn).not.toBeNull();
    expect(fsBtn.textContent).toContain("Fullscreen");

    // Click to enter fullscreen
    fsBtn.click();
    const shell = studio.querySelector(".studio-shell");
    expect(shell.classList.contains("fullscreen-mode")).toBe(true);
    expect(fsBtn.textContent).toContain("Exit Fullscreen");

    // Click again to exit
    fsBtn.click();
    expect(shell.classList.contains("fullscreen-mode")).toBe(false);
    expect(fsBtn.textContent).toContain("Fullscreen");

    studio.remove();
  });

  it("provides side-edge show/hide toggles for inspector and palette sidebars", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    const studio = document.createElement("nebu-studio") as any;
    document.body.appendChild(studio);
    studio.render();

    const inspectorEdgeToggle = studio.querySelector(".inspector-edge-toggle");
    expect(inspectorEdgeToggle).not.toBeNull();
    const inspectorAside = studio.querySelector(".workbench-inspector");
    expect(inspectorAside.classList.contains("collapsed")).toBe(false);

    // Click edge toggle to hide inspector
    inspectorEdgeToggle.click();
    const collapsedAside = studio.querySelector(".workbench-inspector");
    expect(collapsedAside.classList.contains("collapsed")).toBe(true);

    // Edge toggle should now have collapsed class and left arrow to re-open
    const updatedInspectorToggle = studio.querySelector(".inspector-edge-toggle");
    expect(updatedInspectorToggle.classList.contains("collapsed")).toBe(true);
    expect(updatedInspectorToggle.textContent).toContain("◀");

    // Click edge toggle again to re-open inspector
    updatedInspectorToggle.click();
    const expandedAside = studio.querySelector(".workbench-inspector");
    expect(expandedAside.classList.contains("collapsed")).toBe(false);

    // Also test header strip side toggle button
    const sideHideBtn = studio.querySelector(".btn-side-toggle-inspector");
    expect(sideHideBtn).not.toBeNull();
    sideHideBtn.click();
    expect(studio.querySelector(".workbench-inspector").classList.contains("collapsed")).toBe(true);

    // Also test palette edge toggle
    const paletteEdgeToggle = studio.querySelector(".palette-edge-toggle");
    expect(paletteEdgeToggle).not.toBeNull();
    paletteEdgeToggle.click();
    expect(studio.querySelector(".workbench-palette").classList.contains("collapsed")).toBe(true);

    studio.remove();
  });

  it("renders floating canvas controls with fullscreen button that triggers studio fullscreen", () => {
    studioStore.setProject(SAMPLE_PROJECT, true);
    const studio = document.createElement("nebu-studio") as any;
    document.body.appendChild(studio);
    studio.render();

    const canvas = studio.querySelector("nebu-pid-canvas") as any;
    expect(canvas).not.toBeNull();
    canvas.render();

    const floatControls = canvas.querySelector(".canvas-floating-controls");
    expect(floatControls).not.toBeNull();

    const fsFloatingBtn = canvas.querySelector(".btn-canvas-fullscreen");
    expect(fsFloatingBtn).not.toBeNull();

    // Clicking floating fullscreen button activates fullscreen on studio
    fsFloatingBtn.click();
    const shell = studio.querySelector(".studio-shell");
    expect(shell.classList.contains("fullscreen-mode")).toBe(true);

    studio.remove();
  });
});

