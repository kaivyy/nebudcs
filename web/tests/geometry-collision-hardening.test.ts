/**
 * Geometry Collision Hardening, Transformed Bounds & Hard Display Isolation Tests
 *
 * Validates:
 * 1. Large equipment collision envelopes (columns, reactors, furnaces) vs compact items.
 * 2. Transformed rotated bounding boxes (0°, 90°, 180°, 270°) with width/height swapping and label bounds.
 * 3. Stream body penetration detection (differentiating intentional port connections from unintentional body cuts).
 * 4. Transactional group dragging (reject move atomically if any item in selection collides).
 * 5. DisplayViewModel hard display isolation (zero cross-display equipment leakage).
 * 6. Scoped navigation (selection and focus reset when target display lacks selected entity).
 * 7. Faceplate decoupling (open faceplate does not leak symbol into foreign process graphics).
 * 8. Model-to-Operations deployment parity (authoritative placement coordinates).
 */

import { describe, expect, it, beforeEach } from "vitest";
import {
  calculateEquipmentBounds,
  testEquipmentCollision,
  testLabelCollision,
  testStreamBodyIntersections,
  testGroupCollision,
} from "../src/domain/geometry-collision";
import { store } from "../src/state/store";
import { ProjectDetailDto, StudioEquipmentDto, StudioStreamDto } from "../src/protocol/types";

describe("Transformed Bounds & Multi-Scale Geometry", () => {
  it("computes accurate transformed envelopes for rotated equipment (0°, 90°, 180°, 270°)", () => {
    const colEq: StudioEquipmentDto = {
      id: "C-101",
      name: "Stabilizer Column",
      definition_id: "standard.column.distillation_detailed",
      definition_version: "1.0.0",
      position: { x: 500, y: 500 },
      display_width: 90,
      display_height: 240,
      parameters: {},
    };

    // 0 degrees: body width = 90, body height = 240
    const bounds0 = calculateEquipmentBounds(colEq, 0);
    expect(bounds0.visualBox.width).toBe(90);
    expect(bounds0.visualBox.height).toBe(240);

    // 90 degrees: rotated width = 240, rotated height = 90
    colEq.rotation = 90;
    const bounds90 = calculateEquipmentBounds(colEq, 0);
    expect(bounds90.visualBox.width).toBe(240);
    expect(bounds90.visualBox.height).toBe(90);

    // 180 degrees: rotated width = 90, rotated height = 240
    colEq.rotation = 180;
    const bounds180 = calculateEquipmentBounds(colEq, 0);
    expect(bounds180.visualBox.width).toBe(90);
    expect(bounds180.visualBox.height).toBe(240);

    // 270 degrees: rotated width = 240, rotated height = 90
    colEq.rotation = 270;
    const bounds270 = calculateEquipmentBounds(colEq, 0);
    expect(bounds270.visualBox.width).toBe(240);
    expect(bounds270.visualBox.height).toBe(90);
  });

  it("detects collision when large column rotated at 90 deg encroaches on adjacent equipment", () => {
    // Column at (500, 500), size (90, 240).
    // At 0 deg, horizontal extent is [455, 545].
    // At 90 deg, horizontal extent is [380, 620].
    const colEq: StudioEquipmentDto = {
      id: "C-101",
      name: "C-101",
      definition_id: "standard.column.distillation_detailed",
      definition_version: "1.0.0",
      position: { x: 500, y: 500 },
      display_width: 90,
      display_height: 240,
      parameters: {},
    };

    // Pump placed at (610, 500), size (80, 60).
    // Pump extent: [570, 650]
    const pumpEq: StudioEquipmentDto = {
      id: "P-101",
      name: "P-101",
      definition_id: "standard.pump.centrifugal",
      definition_version: "1.0.0",
      position: { x: 680, y: 500 },
      display_width: 80,
      display_height: 60,
      parameters: {},
    };

    // When column is upright (0 deg), distance between edges is 570 - 545 = 25px (no collision at 20px clearance)
    const test0 = testEquipmentCollision(colEq.position!, 90, 240, [pumpEq], "C-101", 20, {
      rotation: 0,
    });
    expect(test0.isColliding).toBe(false);

    // When column is rotated 90 deg, its right edge extends to 620px, directly intersecting pump at 570-650px
    const test90 = testEquipmentCollision(colEq.position!, 90, 240, [pumpEq], "C-101", 20, {
      rotation: 90,
    });
    expect(test90.isColliding).toBe(true);
    expect(test90.collidingIds).toContain("P-101");
  });

  it("detects label collisions even when visual bodies have slight clearance", () => {
    // Two vessels positioned vertically where body clearance is 30px, but vessel 1 has label below body
    const v1: StudioEquipmentDto = {
      id: "V-101",
      name: "V-101 Flash Drum",
      definition_id: "standard.vessel.vertical_separator",
      definition_version: "1.0.0",
      position: { x: 400, y: 300 },
      display_width: 80,
      display_height: 120, // y extent [240, 360], label extent around [360, 386]
      parameters: {},
    };

    const v2: StudioEquipmentDto = {
      id: "V-102",
      name: "V-102 Boot",
      definition_id: "standard.vessel.vertical_separator",
      definition_version: "1.0.0",
      position: { x: 400, y: 440 }, // y extent [380, 500]
      display_width: 80,
      display_height: 120,
      parameters: {},
    };

    const labelTest = testLabelCollision({ ...v1, position: v1.position! }, [v2]);
    expect(labelTest.isColliding).toBe(true);
    expect(labelTest.collidingIds).toContain("V-102");
  });

  it("identifies unintended stream body penetrations while ignoring intentional port connections", () => {
    const vessel: StudioEquipmentDto = {
      id: "V-101",
      name: "V-101",
      definition_id: "standard.vessel.vertical_separator",
      definition_version: "1.0.0",
      position: { x: 500, y: 500 },
      display_width: 80,
      display_height: 120, // visual box: [460, 440, 540, 560]
      parameters: {},
    };

    const intentionalStream: StudioStreamDto = {
      id: "S-FEED",
      name: "Feed",
      stream_type: "Material",
      source_equipment_id: "P-101",
      source_port_id: "out",
      sink_equipment_id: "V-101",
      sink_port_id: "in",
      composition: {},
      route_points: [
        { x: 300, y: 500 },
        { x: 460, y: 500 }, // enters V-101 port
      ],
    };

    const crossingStream: StudioStreamDto = {
      id: "S-BYPASS",
      name: "Bypass",
      stream_type: "Material",
      source_equipment_id: "P-102",
      source_port_id: "out",
      sink_equipment_id: "E-101",
      sink_port_id: "in",
      composition: {},
      route_points: [
        { x: 400, y: 500 },
        { x: 600, y: 500 }, // crosses right through center of V-101 [460..540]
      ],
    };

    // Intentional stream should NOT be flagged as penetration
    const cut1 = testStreamBodyIntersections(vessel, [intentionalStream]);
    expect(cut1).toEqual([]);

    // Unrelated crossing stream MUST be flagged as body penetration
    const cut2 = testStreamBodyIntersections(vessel, [crossingStream]);
    expect(cut2).toContain("S-BYPASS");
  });
});

describe("Transactional Group Movement", () => {
  it("rejects group movement atomically if ANY item in group collides with stationary equipment", () => {
    const stationary: StudioEquipmentDto[] = [
      {
        id: "R-101",
        name: "Reactor",
        definition_id: "standard.reactor.fixed_bed",
        definition_version: "1.0.0",
        position: { x: 700, y: 500 },
        display_width: 100,
        display_height: 180,
        parameters: {},
      },
    ];

    // Selected group to move: 2 exchangers spaced 100px apart
    const group = [
      {
        id: "E-101A",
        origPos: { x: 300, y: 500 },
        width: 100,
        height: 80,
      },
      {
        id: "E-101B",
        origPos: { x: 450, y: 500 },
        width: 100,
        height: 80,
      },
    ];

    // Valid move: delta +50px X -> E-101B ends up at 500, safely clear of R-101 and its quench port at 700
    const validMove = testGroupCollision(group, { x: 50, y: 0 }, stationary, 24);
    expect(validMove.isColliding).toBe(false);
    expect(validMove.collidingIds.length).toBe(0);

    // Invalid move: delta +200px X -> E-101B ends up at 650, colliding with R-101 quench port and body at 700
    // Even though E-101A (at 500) does not collide, group move is rejected atomically
    const invalidMove = testGroupCollision(group, { x: 200, y: 0 }, stationary, 24);
    expect(invalidMove.isColliding).toBe(true);
    expect(invalidMove.collidingIds).toContain("E-101B");
    expect(invalidMove.collidingIds).toContain("R-101");
  });
});

describe("DisplayViewModel Hard Display Isolation", () => {
  const ISOLATION_PROJECT: ProjectDetailDto = {
    id: "proj-nht-isolation",
    name: "Naphtha Hydrotreater Isolation Test",
    schema_version: "0.1.0",
    description: "Multi-display plant for isolation verification",
    author: "Test Engineer",
    created: "2026-09-09T00:00:00Z",
    modified: "2026-09-09T00:00:00Z",
    equipment: [
      { id: "TK-101", name: "Feed Surge Drum", definition_id: "std.tank", definition_version: "1.0.0", display_width: 100, display_height: 100, parameters: {} },
      { id: "P-101A", name: "Charge Pump A", definition_id: "std.pump", definition_version: "1.0.0", display_width: 80, display_height: 60, parameters: {} },
      { id: "R-101", name: "HDS Reactor", definition_id: "std.reactor", definition_version: "1.0.0", display_width: 100, display_height: 180, parameters: {} },
      { id: "C-101", name: "Stripper Column", definition_id: "std.column", definition_version: "1.0.0", display_width: 90, display_height: 240, parameters: {} },
    ],
    streams: [
      { id: "S-1", name: "Feed", stream_type: "Material", source_equipment_id: "TK-101", source_port_id: "out", sink_equipment_id: "P-101A", sink_port_id: "in", composition: {}, route_points: [] },
      { id: "S-2", name: "Rx Feed", stream_type: "Material", source_equipment_id: "P-101A", source_port_id: "out", sink_equipment_id: "R-101", sink_port_id: "in", composition: {}, route_points: [] },
      { id: "S-3", name: "Stripper Feed", stream_type: "Material", source_equipment_id: "R-101", source_port_id: "out", sink_equipment_id: "C-101", sink_port_id: "in", composition: {}, route_points: [] },
    ],
    instruments: [
      { id: "inst-1", tag: "PI-101", equipment_id: "TK-101", measurement_kind: "Pressure", range_min: 0, range_max: 10, unit: "bar" },
      { id: "inst-2", tag: "TI-101", equipment_id: "R-101", measurement_kind: "Temperature", range_min: 0, range_max: 500, unit: "°C" },
    ],
    control_loops: [],
    alarms: [],
    interlocks: [],
    plant_structure: {
      plant_name: "NHT Unit",
      areas: [],
      displays: [
        {
          id: "disp-feed",
          name: "Feed Section",
          display_type: "unit_process_graphic",
          area_id: "area-1",
          equipment_ids: ["TK-101", "P-101A"],
          placements: [
            { id: "p1", display_id: "disp-feed", equipment_id: "TK-101", x: 200, y: 300, width: 100, height: 100, rotation: 0, scale: 1, created_at: "", updated_at: "" },
            { id: "p2", display_id: "disp-feed", equipment_id: "P-101A", x: 400, y: 300, width: 80, height: 60, rotation: 0, scale: 1, created_at: "", updated_at: "" },
          ],
          instrument_ids: ["PI-101"],
          stream_ids: ["S-1"],
          is_default: true,
          order: 1,
          description: "Feed surge and pumps",
        },
        {
          id: "disp-rx",
          name: "Reactor Section",
          display_type: "unit_process_graphic",
          area_id: "area-1",
          equipment_ids: ["R-101"],
          placements: [
            { id: "p3", display_id: "disp-rx", equipment_id: "R-101", x: 600, y: 300, width: 100, height: 180, rotation: 0, scale: 1, created_at: "", updated_at: "" },
          ],
          instrument_ids: ["TI-101"],
          stream_ids: ["S-2"],
          is_default: false,
          order: 2,
          description: "Reactor and furnace",
        },
        {
          id: "disp-overview",
          name: "Plant Overview",
          display_type: "plant_overview",
          area_id: "area-1",
          equipment_ids: ["TK-101", "R-101", "C-101"],
          placements: [
            { id: "po1", display_id: "disp-overview", equipment_id: "TK-101", x: 150, y: 350, width: 100, height: 100, rotation: 0, scale: 1, created_at: "", updated_at: "" },
            { id: "po2", display_id: "disp-overview", equipment_id: "R-101", x: 550, y: 350, width: 100, height: 180, rotation: 0, scale: 1, created_at: "", updated_at: "" },
            { id: "po3", display_id: "disp-overview", equipment_id: "C-101", x: 950, y: 350, width: 90, height: 240, rotation: 0, scale: 1, created_at: "", updated_at: "" },
          ],
          instrument_ids: [],
          stream_ids: ["S-1", "S-2", "S-3"],
          is_default: false,
          order: 0,
          description: "All unit overview",
        },
      ],
    },
  };

  beforeEach(() => {
    store.setActiveProject(ISOLATION_PROJECT);
  });

  it("renders strictly scoped equipment for disp-feed without leaking R-101 or C-101", () => {
    store.navigateDisplay("disp-feed");
    const vm = store.getDisplayViewModel();

    expect(vm.displayId).toBe("disp-feed");
    expect(vm.equipment.map((e) => e.id)).toEqual(["TK-101", "P-101A"]);
    expect(vm.equipment.some((e) => e.id === "R-101")).toBe(false);
    expect(vm.equipment.some((e) => e.id === "C-101")).toBe(false);
    expect(vm.instruments.map((i) => i.tag)).toEqual(["PI-101"]);
    expect(vm.validationErrors.length).toBe(0);
  });

  it("renders strictly scoped equipment for disp-rx without leaking Feed section equipment", () => {
    store.navigateDisplay("disp-rx");
    const vm = store.getDisplayViewModel();

    expect(vm.displayId).toBe("disp-rx");
    expect(vm.equipment.map((e) => e.id)).toEqual(["R-101"]);
    expect(vm.equipment.some((e) => e.id === "TK-101")).toBe(false);
    expect(vm.equipment.some((e) => e.id === "P-101A")).toBe(false);
    expect(vm.instruments.map((i) => i.tag)).toEqual(["TI-101"]);
    expect(vm.validationErrors.length).toBe(0);
  });

  it("treats Plant Overview as an isolated display model without inheriting foreign equipment", () => {
    store.navigateDisplay("disp-overview");
    const vm = store.getDisplayViewModel();

    expect(vm.displayId).toBe("disp-overview");
    // Overview has TK-101, R-101, C-101, but NOT P-101A (pump is intermediate and omitted from high-level overview)
    expect(vm.equipment.map((e) => e.id)).toEqual(["TK-101", "R-101", "C-101"]);
    expect(vm.equipment.some((e) => e.id === "P-101A")).toBe(false);
  });

  it("resets selection and focusedEntity when navigating to a display that lacks the active entity", () => {
    // 1. On disp-feed, select P-101A
    store.navigateDisplay("disp-feed");
    store.selectEquipment("P-101A");
    store.setFocusedEntity("P-101A");
    expect(store.selectedEquipmentId).toBe("P-101A");
    expect(store.focusedEntityId).toBe("P-101A");

    // 2. Navigate to disp-rx (which only has R-101)
    store.navigateDisplay("disp-rx");

    // Scoped navigation must have cleared the selection and focus
    expect(store.selectedEquipmentId).toBeNull();
    expect(store.focusedEntityId).toBeNull();

    // 3. Navigate to disp-overview with an explicit target entity C-101
    store.navigateDisplay("disp-overview", true, "C-101");
    expect(store.selectedEquipmentId).toBe("C-101");
    expect(store.focusedEntityId).toBe("C-101");
  });

  it("decouples open faceplate state from process graphic rendering", () => {
    // 1. Open faceplate for P-101A on disp-feed
    store.navigateDisplay("disp-feed");
    store.openFaceplate({
      kind: "equipment",
      id: "P-101A",
      name: "P-101A",
      description: "Charge Pump A",
    });
    expect(store.selectedFaceplate?.id).toBe("P-101A");

    // 2. Navigate to disp-rx
    store.navigateDisplay("disp-rx");

    // Faceplate remains open in operator dock for monitoring/tuning,
    // but process view on disp-rx does NOT render P-101A symbol
    expect(store.selectedFaceplate?.id).toBe("P-101A");
    const vm = store.getDisplayViewModel();
    expect(vm.equipment.some((e) => e.id === "P-101A")).toBe(false);
  });

  it("detects and flags missing placement validation errors instead of inventing synthetic coordinates", () => {
    // Create an invalid display config with an equipment ID that has NO placement
    const brokenProj: ProjectDetailDto = {
      ...ISOLATION_PROJECT,
      plant_structure: {
        ...ISOLATION_PROJECT.plant_structure!,
        displays: [
          {
            id: "disp-broken",
            name: "Broken Display",
            display_type: "unit_process_graphic",
            area_id: "area-1",
            equipment_ids: ["TK-101", "P-101A"], // TK-101 has placement, P-101A is missing!
            placements: [
              { id: "p1", display_id: "disp-broken", equipment_id: "TK-101", x: 200, y: 300, width: 100, height: 100, rotation: 0, scale: 1, created_at: "", updated_at: "" },
            ],
            instrument_ids: [],
            stream_ids: [],
            is_default: true,
            order: 1,
            description: "Missing placement test",
          },
        ],
      },
    };

    store.setActiveProject(brokenProj);
    store.navigateDisplay("disp-broken");
    const vm = store.getDisplayViewModel();

    // Must report validation error for P-101A
    expect(vm.validationErrors.length).toBeGreaterThan(0);
    expect(vm.validationErrors[0]).toContain("Missing display placement for equipment Charge Pump A (P-101A)");
    // Must NOT render P-101A at synthetic (100, 200)
    expect(vm.equipment.map((e) => e.id)).toEqual(["TK-101"]);
  });
});
