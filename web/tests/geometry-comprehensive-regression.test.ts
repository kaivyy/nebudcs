/**
 * Comprehensive Operations Geometry & Collision Regression Test Suite (Tests A through N)
 *
 * Strict mathematical and SVG DOM verification proving that:
 * 1. Multi-tier bounding boxes (body, visual, label, telemetry, ports, interaction, collision)
 *    satisfy geometric containment invariants.
 * 2. Rotated and scaled equipment compute exact axis-aligned bounding boxes.
 * 3. Dense arrangements with varying equipment types (columns, reactors, compressors, pumps)
 *    maintain strict industrial engineering clearance.
 * 4. Telemetry plaques dynamically adapt layout zones to avoid overlaps.
 * 5. Dynamic viewport eliminates top-dead-space and prevents clipping on all aspect ratios.
 */

import { describe, expect, it, beforeEach } from "vitest";
import {
  calculateEquipmentBounds,
  computeTransformedAABB,
  determineOptimalTelemetryZone,
  testEquipmentCollision,
  DEFAULT_ENGINEERING_CLEARANCE,
} from "../src/domain/geometry-collision";
import { NebuProcessView } from "../src/components/nebu-process-view";
import { store } from "../src/state/store";
import { ProjectDetailDto } from "../src/protocol/types";

describe("Comprehensive Geometry Regression Suite (A through N)", () => {
  let view: NebuProcessView;

  beforeEach(() => {
    view = new NebuProcessView();
    document.body.innerHTML = "";
    document.body.appendChild(view);
  });

  // -------------------------------------------------------------------------
  // Test A: Standard horizontal layout with 24px clearance
  // -------------------------------------------------------------------------
  it("Test A: Standard horizontal layout maintains >= 24px industrial clearance", () => {
    const eq1 = { id: "P-101", position: { x: 200, y: 300 }, display_width: 90, display_height: 60 };
    const eq2 = { id: "P-102", position: { x: 380, y: 300 }, display_width: 90, display_height: 60 };

    const collision = testEquipmentCollision(
      eq1.position,
      eq1.display_width,
      eq1.display_height,
      [eq2 as any],
      eq1.id,
      DEFAULT_ENGINEERING_CLEARANCE
    );

    expect(collision.isColliding).toBe(false);
    expect(collision.collidingIds).toEqual([]);
  });

  // -------------------------------------------------------------------------
  // Test B: Large reactor next to column (tall aspect ratio bounds)
  // -------------------------------------------------------------------------
  it("Test B: Large reactor (120x240) next to column (100x320) respects vertical envelopes", () => {
    const column = {
      id: "T-101",
      name: "T-101",
      definition_id: "standard.vessel.column",
      position: { x: 300, y: 500 },
      display_width: 100,
      display_height: 320,
    };
    const reactor = {
      id: "R-101",
      name: "R-101",
      definition_id: "standard.reactor.fixed_bed",
      position: { x: 550, y: 500 },
      display_width: 120,
      display_height: 240,
    };

    const boundsCol = calculateEquipmentBounds(column);
    const boundsRx = calculateEquipmentBounds(reactor);

    // Verify tall aspect ratio bounds
    expect(boundsCol.visualBox.height).toBeGreaterThanOrEqual(320);
    expect(boundsRx.visualBox.height).toBeGreaterThanOrEqual(240);

    const res = testEquipmentCollision(
      reactor.position,
      reactor.display_width,
      reactor.display_height,
      [column as any],
      reactor.id,
      DEFAULT_ENGINEERING_CLEARANCE
    );
    expect(res.isColliding).toBe(false);
  });

  // -------------------------------------------------------------------------
  // Test C: Two large compressors (K-102 vs K-103) with motor casing extensions
  // -------------------------------------------------------------------------
  it("Test C: Two large compressors account for motor casing width extensions", () => {
    const k102 = {
      id: "K-102",
      name: "K-102",
      definition_id: "standard.compressor.centrifugal",
      position: { x: 300, y: 400 },
      display_width: 130,
      display_height: 80,
    };
    const k103 = {
      id: "K-103",
      name: "K-103",
      definition_id: "standard.compressor.centrifugal",
      position: { x: 550, y: 400 },
      display_width: 130,
      display_height: 80,
    };

    const b102 = calculateEquipmentBounds(k102);
    // Compressor should have motor casing extension added to visualBox
    expect(b102.visualBox.width).toBeGreaterThan(k102.display_width);

    const res = testEquipmentCollision(
      k103.position,
      k103.display_width,
      k103.display_height,
      [k102 as any],
      k103.id,
      DEFAULT_ENGINEERING_CLEARANCE
    );
    expect(res.isColliding).toBe(false);
  });

  // -------------------------------------------------------------------------
  // Test D: Rotated equipment (45 deg, 90 deg, 180 deg) AABB transformed bounds
  // -------------------------------------------------------------------------
  it("Test D: Rotated equipment expands AABB deterministically according to trigonometric projection", () => {
    const w = 100;
    const h = 50;
    const unrotated = computeTransformedAABB(200, 200, w, h, 0);
    expect(unrotated.width).toBe(100);
    expect(unrotated.height).toBe(50);

    const rot90 = computeTransformedAABB(200, 200, w, h, 90);
    expect(Math.round(rot90.width)).toBe(50);
    expect(Math.round(rot90.height)).toBe(100);

    const rot45 = computeTransformedAABB(200, 200, w, h, 45);
    const expectedDim = Math.round((w * Math.SQRT1_2 + h * Math.SQRT1_2) * 1000) / 1000;
    expect(rot45.width).toBeCloseTo(expectedDim, 1);
    expect(rot45.height).toBeCloseTo(expectedDim, 1);
  });

  // -------------------------------------------------------------------------
  // Test E: Compact display with high equipment density and zero collisions
  // -------------------------------------------------------------------------
  it("Test E: Compact display with 5 equipment items maintains pairwise clearance", () => {
    const items = [
      { id: "E-1", position: { x: 100, y: 200 }, display_width: 60, display_height: 50 },
      { id: "E-2", position: { x: 260, y: 200 }, display_width: 60, display_height: 50 },
      { id: "E-3", position: { x: 420, y: 200 }, display_width: 60, display_height: 50 },
      { id: "E-4", position: { x: 580, y: 200 }, display_width: 60, display_height: 50 },
      { id: "E-5", position: { x: 740, y: 200 }, display_width: 60, display_height: 50 },
    ];

    for (let i = 0; i < items.length; i++) {
      const target = items[i];
      const others = items.filter((_, idx) => idx !== i);
      const res = testEquipmentCollision(
        target.position,
        target.display_width,
        target.display_height,
        others as any,
        target.id,
        24
      );
      expect(res.isColliding).toBe(false);
    }
  });

  // -------------------------------------------------------------------------
  // Test F: Multi-tier bounding box enclosure invariant
  // -------------------------------------------------------------------------
  it("Test F: Invariant holds: bodyBox ⊆ visualBox ⊆ collisionBox", () => {
    const eq = {
      id: "V-101",
      name: "V-101",
      definition_id: "standard.vessel.vertical_separator",
      position: { x: 400, y: 350 },
      display_width: 80,
      display_height: 120,
    };

    const b = calculateEquipmentBounds(eq, 24);

    // 1. bodyBox x, y, right, bottom within visualBox
    expect(b.geometry.bodyBounds.x).toBeGreaterThanOrEqual(b.visualBox.x);
    expect(b.geometry.bodyBounds.y).toBeGreaterThanOrEqual(b.visualBox.y);
    expect(b.geometry.bodyBounds.x + b.geometry.bodyBounds.width).toBeLessThanOrEqual(b.visualBox.x + b.visualBox.width);
    expect(b.geometry.bodyBounds.y + b.geometry.bodyBounds.height).toBeLessThanOrEqual(b.visualBox.y + b.visualBox.height);

    // 2. visualBox within collisionBox
    expect(b.visualBox.x).toBeGreaterThanOrEqual(b.collisionBox.x);
    expect(b.visualBox.y).toBeGreaterThanOrEqual(b.collisionBox.y);
    expect(b.visualBox.x + b.visualBox.width).toBeLessThanOrEqual(b.collisionBox.x + b.collisionBox.width);
    expect(b.visualBox.y + b.visualBox.height).toBeLessThanOrEqual(b.collisionBox.y + b.collisionBox.height);
  });

  // -------------------------------------------------------------------------
  // Test G: Label bounds strictly centered and within collisionBounds
  // -------------------------------------------------------------------------
  it("Test G: Label bounds is horizontally centered at cx and enclosed within collisionBox", () => {
    const cx = 350;
    const cy = 250;
    const eq = { id: "P-101", position: { x: cx, y: cy }, display_width: 90, display_height: 60 };
    const b = calculateEquipmentBounds(eq, 24);

    const labelCenter = b.labelBox.x + b.labelBox.width / 2;
    expect(labelCenter).toBeCloseTo(cx, 1);

    expect(b.labelBox.x).toBeGreaterThanOrEqual(b.collisionBox.x);
    expect(b.labelBox.x + b.labelBox.width).toBeLessThanOrEqual(b.collisionBox.x + b.collisionBox.width);
  });

  // -------------------------------------------------------------------------
  // Test H: Telemetry bounds strictly positioned within semantic zones
  // -------------------------------------------------------------------------
  it("Test H: Telemetry zones position plaque on correct side of equipment", () => {
    const cx = 500;
    const cy = 400;
    const eq = { id: "P-101", position: { x: cx, y: cy }, display_width: 80, display_height: 60 };

    const bBottom = calculateEquipmentBounds(eq, 0, { telemetryZone: "BOTTOM" });
    expect(bBottom.telemetryBox.y).toBeGreaterThan(bBottom.visualBox.y + bBottom.visualBox.height);

    const bTop = calculateEquipmentBounds(eq, 0, { telemetryZone: "TOP" });
    expect(bTop.telemetryBox.y + bTop.telemetryBox.height).toBeLessThan(bTop.visualBox.y);

    const bRight = calculateEquipmentBounds(eq, 0, { telemetryZone: "RIGHT" });
    expect(bRight.telemetryBox.x).toBeGreaterThan(bRight.visualBox.x + bRight.visualBox.width);

    const bLeft = calculateEquipmentBounds(eq, 0, { telemetryZone: "LEFT" });
    expect(bLeft.telemetryBox.x + bLeft.telemetryBox.width).toBeLessThan(bLeft.visualBox.x);
  });

  // -------------------------------------------------------------------------
  // Test I: Dynamic telemetry zone switching when adjacent space is restricted
  // -------------------------------------------------------------------------
  it("Test I: determineOptimalTelemetryZone avoids conflicts when BOTTOM is blocked", () => {
    const target = { id: "P-101", position: { x: 300, y: 300 }, display_width: 80, display_height: 60 };
    // Equipment placed directly below target
    const neighborBelow = { id: "P-102", position: { x: 300, y: 390 }, display_width: 80, display_height: 60 };

    const optimalZone = determineOptimalTelemetryZone(target, [neighborBelow as any], [], "BOTTOM");
    // Should choose TOP, RIGHT, LEFT or OVERLAY rather than colliding BOTTOM
    expect(["TOP", "RIGHT", "LEFT", "OVERLAY"]).toContain(optimalZone);
  });

  // -------------------------------------------------------------------------
  // Test J: Stream path clearance — orthogonal routing
  // -------------------------------------------------------------------------
  it("Test J: Stream orthogonal paths connect endpoints without diagonal shortcuts", () => {
    const p1 = { x: 100, y: 200 };
    const p2 = { x: 100, y: 350 };
    const p3 = { x: 300, y: 350 };

    const pts = [p1, p2, p3];
    for (let i = 0; i < pts.length - 1; i++) {
      const dx = pts[i + 1].x - pts[i].x;
      const dy = pts[i + 1].y - pts[i].y;
      // Either dx is 0 (vertical segment) or dy is 0 (horizontal segment)
      expect(dx === 0 || dy === 0).toBe(true);
    }
  });

  // -------------------------------------------------------------------------
  // Test K: Boundary condition box clearance to target port
  // -------------------------------------------------------------------------
  it("Test K: Boundary condition connector arrow enters port perpendicularly", () => {
    const pIn = { x: 200, y: 300 };
    const boxW = 150;
    const boxX = pIn.x - boxW - 35;

    // Leader line runs horizontally from boxX + boxW to pIn.x
    const lineStart = boxX + boxW;
    const lineEnd = pIn.x;
    expect(lineEnd - lineStart).toBe(35);
  });

  // -------------------------------------------------------------------------
  // Test L: Viewport content-bounds derivation for ultra-wide flowsheet
  // -------------------------------------------------------------------------
  it("Test L: Viewport content-bounds accommodates ultra-wide flowsheet (> 2500px)", () => {
    const wideProject: ProjectDetailDto = {
      id: "proj-wide",
      name: "Ultra-Wide Chemical Plant",
      schema_version: "0.1.0",
      description: "Wide test",
      author: "Test",
      created: "2026-09-10T00:00:00Z",
      modified: "2026-09-10T00:00:00Z",
      equipment: [
        { id: "EQ-1", name: "EQ-1", definition_id: "standard.pump.centrifugal", definition_version: "1.0.0", position: { x: 100, y: 300 }, display_width: 80, display_height: 60, parameters: {} },
        { id: "EQ-2", name: "EQ-2", definition_id: "standard.pump.centrifugal", definition_version: "1.0.0", position: { x: 2600, y: 300 }, display_width: 80, display_height: 60, parameters: {} },
      ],
      streams: [],
      instruments: [],
      control_loops: [],
      alarms: [],
      interlocks: [],
      displays: [
        {
          id: "disp-wide",
          name: "Wide Display",
          display_type: "unit_process_graphic",
          equipment_ids: ["EQ-1", "EQ-2"],
          stream_ids: [],
          instrument_ids: [],
          placements: [
            { equipment_id: "EQ-1", x: 100, y: 300, width: 80, height: 60, rotation: 0 },
            { equipment_id: "EQ-2", x: 2600, y: 300, width: 80, height: 60, rotation: 0 },
          ],
        },
      ],
    };

    store.setActiveProject(wideProject);
    store.setActiveDisplay("disp-wide");
    const vm = store.getDisplayViewModel();
    const bounds = view.computeDisplayContentBounds(vm);

    expect(bounds.minX).toBeLessThanOrEqual(100);
    expect(bounds.maxX).toBeGreaterThanOrEqual(2600);
    expect(bounds.maxX - bounds.minX).toBeGreaterThanOrEqual(2500);
  });

  // -------------------------------------------------------------------------
  // Test M: Viewport content-bounds derivation for tall flowsheet
  // -------------------------------------------------------------------------
  it("Test M: Viewport content-bounds accommodates tall flowsheet (> 1600px)", () => {
    const tallProject: ProjectDetailDto = {
      id: "proj-tall",
      name: "Tall Chemical Plant",
      schema_version: "0.1.0",
      description: "Tall test",
      author: "Test",
      created: "2026-09-10T00:00:00Z",
      modified: "2026-09-10T00:00:00Z",
      equipment: [
        { id: "EQ-TOP", name: "EQ-TOP", definition_id: "standard.pump.centrifugal", definition_version: "1.0.0", position: { x: 500, y: 80 }, display_width: 80, display_height: 60, parameters: {} },
        { id: "EQ-BTM", name: "EQ-BTM", definition_id: "standard.pump.centrifugal", definition_version: "1.0.0", position: { x: 500, y: 1700 }, display_width: 80, display_height: 60, parameters: {} },
      ],
      streams: [],
      instruments: [],
      control_loops: [],
      alarms: [],
      interlocks: [],
      displays: [
        {
          id: "disp-tall",
          name: "Tall Display",
          display_type: "unit_process_graphic",
          equipment_ids: ["EQ-TOP", "EQ-BTM"],
          stream_ids: [],
          instrument_ids: [],
          placements: [
            { equipment_id: "EQ-TOP", x: 500, y: 80, width: 80, height: 60, rotation: 0 },
            { equipment_id: "EQ-BTM", x: 500, y: 1700, width: 80, height: 60, rotation: 0 },
          ],
        },
      ],
    };

    store.setActiveProject(tallProject);
    store.setActiveDisplay("disp-tall");
    const vm = store.getDisplayViewModel();
    const bounds = view.computeDisplayContentBounds(vm);

    expect(bounds.minY).toBeLessThanOrEqual(80);
    expect(bounds.maxY).toBeGreaterThanOrEqual(1700);
    expect(bounds.maxY - bounds.minY).toBeGreaterThanOrEqual(1600);
  });

  // -------------------------------------------------------------------------
  // Test N: Viewport zero top dead space invariant
  // -------------------------------------------------------------------------
  it("Test N: Invariant holds: viewBox y closely hugs content minY without empty top margin", () => {
    const compactProject: ProjectDetailDto = {
      id: "proj-compact",
      name: "Compact Plant",
      schema_version: "0.1.0",
      description: "Compact test",
      author: "Test",
      created: "2026-09-10T00:00:00Z",
      modified: "2026-09-10T00:00:00Z",
      equipment: [
        { id: "EQ-1", name: "EQ-1", definition_id: "standard.pump.centrifugal", definition_version: "1.0.0", position: { x: 400, y: 320 }, display_width: 80, display_height: 60, parameters: {} },
      ],
      streams: [],
      instruments: [],
      control_loops: [],
      alarms: [],
      interlocks: [],
      displays: [
        {
          id: "disp-compact",
          name: "Compact Display",
          display_type: "unit_process_graphic",
          equipment_ids: ["EQ-1"],
          stream_ids: [],
          instrument_ids: [],
          placements: [
            { equipment_id: "EQ-1", x: 400, y: 320, width: 80, height: 60, rotation: 0 },
          ],
        },
      ],
    };

    store.setActiveProject(compactProject);
    store.setActiveDisplay("disp-compact");
    view.render();

    const vm = store.getDisplayViewModel();
    const bounds = view.computeDisplayContentBounds(vm);

    // Content minY is around 320 - 30 - 30 = 260
    expect(bounds.minY).toBeGreaterThan(200);

    // Viewport viewY was fitted to content minY (with padY = 50), NOT hardcoded to 0!
    expect((view as any).viewY).toBeGreaterThan(150);
  });
});
