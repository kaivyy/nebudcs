/**
 * Engineering Studio Geometry, Collision & Docking Acceptance Tests
 *
 * Validates:
 * 1. findCollisionFreePosition avoids overlaps and respects clearance.
 * 2. testEquipmentCollision detects clearance violations.
 * 3. calculateMagneticPortDock snaps compatible ports within tolerance.
 * 4. generateAutoLayout computes deterministic flow-based coordinates.
 * 5. Authoritative geometry: studioStore moveEntities updates project model
 *    and propagates to Operations process graphic rendering.
 */

import { describe, expect, it, beforeEach } from "vitest";
import {
  findCollisionFreePosition,
  testEquipmentCollision,
  calculateMagneticPortDock,
  generateAutoLayout,
  validateLayoutGeometry,
} from "../src/domain/geometry-collision";
import { studioStore } from "../src/state/studio-store";
import { store } from "../src/state/store";
import { ProjectDetailDto, StudioEquipmentDto, StudioStreamDto } from "../src/protocol/types";
import { NebuProcessView } from "../src/components/nebu-process-view";

const TEST_PROJECT: ProjectDetailDto = {
  id: "proj-geometry-test",
  name: "Geometry Acceptance Test Plant",
  schema_version: "0.1.0",
  description: "Test flowsheet for collision and authoritative geometry",
  author: "Test Engineer",
  created: "2026-09-09T00:00:00Z",
  modified: "2026-09-09T00:00:00Z",
  equipment: [
    {
      id: "P-101A",
      name: "P-101A",
      definition_id: "standard.pump.centrifugal",
      definition_version: "1.0.0",
      position: { x: 300, y: 300 },
      display_width: 100,
      display_height: 70,
      parameters: {},
    },
    {
      id: "V-101",
      name: "V-101",
      definition_id: "standard.vessel.vertical_separator",
      definition_version: "1.0.0",
      position: { x: 500, y: 300 },
      display_width: 90,
      display_height: 140,
      parameters: {},
    },
  ],
  streams: [],
  instruments: [],
  control_loops: [],
  alarms: [],
  interlocks: [],
};

describe("Deterministic Collision & Clearance Engine", () => {
  it("detects equipment overlap when clearance is violated", () => {
    const existing = TEST_PROJECT.equipment;
    // Overlap candidate at (320, 310) which is inside P-101A (300, 300) box [250, 350]
    const test1 = testEquipmentCollision(
      { x: 320, y: 310 },
      100,
      70,
      existing,
      undefined,
      24
    );
    expect(test1.isColliding).toBe(true);
    expect(test1.collidingIds).toContain("P-101A");

    // Clear candidate at (100, 100) well away from (300, 300) and (500, 300)
    const test2 = testEquipmentCollision(
      { x: 100, y: 100 },
      100,
      70,
      existing,
      undefined,
      24
    );
    expect(test2.isColliding).toBe(false);
    expect(test2.collidingIds.length).toBe(0);
  });

  it("finds collision-free position via spiral search respecting clearance", () => {
    const existing = TEST_PROJECT.equipment;
    // Request position directly on top of P-101A (300, 300)
    const freePos = findCollisionFreePosition(
      { x: 300, y: 300 },
      100,
      70,
      existing,
      { gridSize: 20, clearance: 24 }
    );

    // Returned position must not collide with P-101A or V-101
    const verify = testEquipmentCollision(freePos, 100, 70, existing, undefined, 24);
    expect(verify.isColliding).toBe(false);
    expect(freePos.x % 20).toBe(0);
    expect(freePos.y % 20).toBe(0);
    expect(freePos.x !== 300 || freePos.y !== 300).toBe(true);
  });

  it("validates entire flowsheet and reports clearance warnings or boundary errors", () => {
    const crowdedEq: StudioEquipmentDto[] = [
      {
        id: "E1",
        definition_id: "standard.test.eq",
        definition_version: "1.0.0",
        name: "E1",
        position: { x: 200, y: 200 },
        display_width: 100,
        display_height: 70,
        parameters: {},
      },
      {
        id: "E2",
        definition_id: "standard.test.eq",
        definition_version: "1.0.0",
        name: "E2",
        position: { x: 230, y: 220 }, // Direct clearance violation
        display_width: 100,
        display_height: 70,
        parameters: {},
      },
      {
        id: "E3_OUT_OF_BOUNDS",
        definition_id: "standard.test.eq",
        definition_version: "1.0.0",
        name: "E3",
        position: { x: 5, y: 5 }, // Near margin out of bounds
        display_width: 100,
        display_height: 70,
        parameters: {},
      },
    ];

    const issues = validateLayoutGeometry(crowdedEq, [], 1920, 1080, 24);
    expect(issues.length).toBeGreaterThanOrEqual(2);
    const hasCollision = issues.some((i) => i.severity === "ERROR" || i.severity === "WARNING");
    expect(hasCollision).toBe(true);
  });
});

describe("Magnetic Port Docking", () => {
  it("snaps dragged equipment port to compatible nearby equipment port", () => {
    const existing = TEST_PROJECT.equipment;
    // Candidate template with OUTLET port at xOffset +50, yOffset 0
    const candidateTmpl = {
      id: "CANDIDATE",
      ports: [
        { id: "OUT", name: "Outlet", domain: "Material" as const, direction: "OUTLET" as const, xOffset: 50, yOffset: 0 },
      ],
    };

    // V-101 at (500, 300) has FEED_IN at (-40, -10) -> world pos (460, 290)
    // When candidate is near (415, 292), candidate outlet (465, 292) is 5.3px from (460, 290)
    const dock = calculateMagneticPortDock(
      candidateTmpl,
      { x: 415, y: 292 },
      existing,
      30
    );

    expect(dock).not.toBeNull();
    expect(dock?.isDocked).toBe(true);
    expect(dock?.targetEquipment?.id).toBe("V-101");
    expect(dock?.targetPort?.id).toBe("FEED_IN");
    expect(dock?.snappedEquipmentPos).toEqual({ x: 410, y: 290 });
  });
});

describe("Non-Destructive Flow Auto-Layout", () => {
  it("computes left-to-right flow coordinates based on stream topology", () => {
    const eq: StudioEquipmentDto[] = [
      { id: "EQ-1", definition_id: "std.tank", definition_version: "1.0.0", name: "Tank", position: { x: 0, y: 0 }, parameters: {} },
      { id: "EQ-2", definition_id: "std.pump", definition_version: "1.0.0", name: "Pump", position: { x: 0, y: 0 }, parameters: {} },
      { id: "EQ-3", definition_id: "std.reactor", definition_version: "1.0.0", name: "Reactor", position: { x: 0, y: 0 }, parameters: {} },
    ];
    const streams: StudioStreamDto[] = [
      {
        id: "S1",
        name: "S1",
        stream_type: "Material",
        source_equipment_id: "EQ-1",
        source_port_id: "out",
        sink_equipment_id: "EQ-2",
        sink_port_id: "in",
        composition: {},
        route_points: [],
      },
      {
        id: "S2",
        name: "S2",
        stream_type: "Material",
        source_equipment_id: "EQ-2",
        source_port_id: "out",
        sink_equipment_id: "EQ-3",
        sink_port_id: "in",
        composition: {},
        route_points: [],
      },
    ];

    const layout = generateAutoLayout(eq, streams, "flow-lr", 20, 40);
    expect(layout.size).toBe(3);

    const pos1 = layout.get("EQ-1")!;
    const pos2 = layout.get("EQ-2")!;
    const pos3 = layout.get("EQ-3")!;

    // Stream progression EQ-1 -> EQ-2 -> EQ-3 must yield strictly increasing X coordinates
    expect(pos1.x).toBeLessThan(pos2.x);
    expect(pos2.x).toBeLessThan(pos3.x);
  });
});

describe("Authoritative Geometry: Studio to Operations Propagation", () => {
  beforeEach(() => {
    studioStore.setProject(TEST_PROJECT, true);
  });

  it("updates model coordinates on moveEntities and propagates to Operations rendering", () => {
    // 1. Move P-101A in Engineering Studio to (850, 450)
    studioStore.moveEntities([
      { id: "P-101A", type: "equipment", pos: { x: 850, y: 450 } },
    ]);

    const updatedProj = studioStore.getProject()!;
    const movedEq = updatedProj.equipment.find((e) => e.id === "P-101A");
    expect(movedEq?.position).toEqual({ x: 850, y: 450 });

    // 2. Deploy updated project into live simulation store
    store.setActiveProject(updatedProj);
    expect(store.activeProject?.equipment.find((e) => e.id === "P-101A")?.position).toEqual({
      x: 850,
      y: 450,
    });

    // 3. Render in Operations nebu-process-view
    if (!window.customElements.get("nebu-process-view")) {
      window.customElements.define("nebu-process-view", NebuProcessView);
    }
    const view = new NebuProcessView();
    document.body.appendChild(view);
    view.render();
    const pumpGroup = view.querySelector("#pumpGroup") || view.querySelector(".equipment-node");
    expect(pumpGroup).not.toBeNull();
    const ring = pumpGroup?.querySelector(".focus-ring");
    expect(ring?.getAttribute("cx")).toBe("850");
    expect(ring?.getAttribute("cy")).toBe("450");

    view.remove();
  });
});
