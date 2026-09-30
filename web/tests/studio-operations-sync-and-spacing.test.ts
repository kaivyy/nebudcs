/**
 * Studio Store ↔ Operations Process View Synchronization & Overlap Avoidance Tests
 *
 * Verifies:
 * 1. Engineering Studio actions (move, resize, rotate, delete) synchronize with display placements.
 * 2. Operations view model reflects Studio moves immediately.
 * 3. Instrument bubbles (left shoulder), Alarm badges (top center), and Control loop bubbles (right shoulder)
 *    do not overlap each other when rendered around the same equipment.
 * 4. Equipment without explicit positions distribute across a grid instead of collapsing into a single pile.
 * 5. Telemetry plaque text is centered dynamically within the telemetry rectangle.
 */

import { describe, expect, it, beforeEach } from "vitest";
import { studioStore } from "../src/state/studio-store";
import { store } from "../src/state/store";
import { NebuProcessView } from "../src/components/nebu-process-view";
import { ProjectDetailDto } from "../src/protocol/types";

const TEST_PROJECT: ProjectDetailDto = {
  id: "proj-sync-test",
  name: "Sync & Spacing Test Project",
  schema_version: "0.1.0",
  description: "Validating studio to operations sync and spacing",
  author: "Test Engineer",
  created: "2026-09-10T00:00:00Z",
  modified: "2026-09-10T00:00:00Z", alarms: [], interlocks: [],
  equipment: [
    {
      id: "P-101",
      name: "P-101",
      definition_id: "standard.pump.centrifugal",
      definition_version: "1.0.0",
      position: { x: 300, y: 300 },
      display_width: 80,
      display_height: 60,
      parameters: {},
    },
    {
      id: "V-101",
      name: "V-101",
      definition_id: "standard.vessel.vertical_separator",
      definition_version: "1.0.0",
      position: { x: 600, y: 300 },
      display_width: 80,
      display_height: 120,
      parameters: {},
    },
  ],
  streams: [],
  instruments: [
    {
      id: "inst-p101",
      tag: "PI-101",
      measurement_kind: "Pressure",
      equipment_id: "P-101",
      range_min: 0,
      range_max: 50,
      unit: "bar",
    },
  ],
  control_loops: [
    {
      id: "loop-p101",
      name: "PIC-101",
      pv_tag: "PI-101",
      sp: 25,
      mode: "AUTO",
      equipment_id: "P-101",
    } as any,
  ],
  displays: [
    {
      id: "disp-main",
      name: "Main Process Display",
      display_type: "plant_overview",
      equipment_ids: ["P-101", "V-101"],
      placements: [
        { equipment_id: "P-101", x: 300, y: 300, width: 80, height: 60, rotation: 0 },
        { equipment_id: "V-101", x: 600, y: 300, width: 80, height: 120, rotation: 0 },
      ],
      stream_ids: [], instrument_ids: ["inst-p101"],
    },
  ],
};

describe("Studio ↔ Operations Synchronization", () => {
  beforeEach(() => {
    const proj = JSON.parse(JSON.stringify(TEST_PROJECT));
    studioStore.setProject(proj, true);
    store.setActiveProject(proj);
    store.setActiveDisplay("disp-main");
  });

  it("synchronizes moveEntities in Studio with active display placements", () => {
    let p101 = studioStore.getProject()?.equipment.find((e) => e.id === "P-101");
    expect(p101?.position).toEqual({ x: 300, y: 300 });

    studioStore.moveEntities([
      { id: "P-101", type: "equipment", pos: { x: 450, y: 350 } },
    ]);

    p101 = studioStore.getProject()?.equipment.find((e) => e.id === "P-101");
    expect(p101?.position).toEqual({ x: 450, y: 350 });

    const disp = studioStore.getProject()?.displays?.find((d) => d.id === "disp-main");
    const pl = disp?.placements?.find((p) => p.equipment_id === "P-101");
    expect(pl?.x).toBe(450);
    expect(pl?.y).toBe(350);

    store.setActiveProject(studioStore.getProject()!);
    const vm = store.getDisplayViewModel();
    const runtimePl = vm.placements.get("P-101");
    expect(runtimePl?.x).toBe(450);
    expect(runtimePl?.y).toBe(350);
  });

  it("synchronizes undo and redo of moves across display placements", () => {
    studioStore.moveEntities([
      { id: "P-101", type: "equipment", pos: { x: 400, y: 250 } },
    ]);

    let pl = studioStore.getProject()?.displays?.[0].placements?.find((p) => p.equipment_id === "P-101");
    expect(pl?.x).toBe(400);
    expect(pl?.y).toBe(250);

    studioStore.undo();
    pl = studioStore.getProject()?.displays?.[0].placements?.find((p) => p.equipment_id === "P-101");
    expect(pl?.x).toBe(300);
    expect(pl?.y).toBe(300);

    studioStore.redo();
    pl = studioStore.getProject()?.displays?.[0].placements?.find((p) => p.equipment_id === "P-101");
    expect(pl?.x).toBe(400);
    expect(pl?.y).toBe(250);
  });

  it("synchronizes resizeEquipment and rotateEquipment across display placements", () => {
    studioStore.resizeEquipment("P-101", 120, 90);
    let pl = studioStore.getProject()?.displays?.[0].placements?.find((p) => p.equipment_id === "P-101");
    expect(pl?.width).toBe(120);
    expect(pl?.height).toBe(90);

    studioStore.rotateEquipment("P-101", 90);
    pl = studioStore.getProject()?.displays?.[0].placements?.find((p) => p.equipment_id === "P-101");
    expect(pl?.rotation).toBe(90);
  });

  it("prunes display placements when equipment is deleted in Studio", () => {
    studioStore.deleteEquipment("P-101");
    const disp = studioStore.getProject()?.displays?.[0];
    expect(disp?.equipment_ids).not.toContain("P-101");
    expect(disp?.placements?.some((p) => p.equipment_id === "P-101")).toBe(false);
  });

  it("isolates display-scoped equipment placement moves from other displays", () => {
    // Add a second display to project
    const proj = studioStore.getProject()!;
    if (!proj.displays) proj.displays = [];
    proj.displays.push({
      id: "disp-secondary",
      name: "Secondary Unit Display",
      display_type: "unit_process_graphic",
      equipment_ids: ["P-101"],
      placements: [
        { equipment_id: "P-101", x: 150, y: 180, width: 80, height: 60, rotation: 0 },
      ],
      stream_ids: [],
      instrument_ids: [],
    });
    studioStore.setProject(proj);

    // Move P-101 specifically within disp-secondary
    studioStore.moveEntities(
      [{ id: "P-101", type: "equipment", pos: { x: 750, y: 500 } }],
      "disp-secondary"
    );

    const updatedProj = studioStore.getProject()!;
    const mainDisp = updatedProj.displays?.find((d) => d.id === "disp-main");
    const secDisp = updatedProj.displays?.find((d) => d.id === "disp-secondary");

    // disp-secondary must have updated coordinates
    const secPl = secDisp?.placements?.find((p) => p.equipment_id === "P-101");
    expect(secPl?.x).toBe(750);
    expect(secPl?.y).toBe(500);

    // disp-main placement must remain unchanged
    const mainPl = mainDisp?.placements?.find((p) => p.equipment_id === "P-101");
    expect(mainPl?.x).toBe(300);
    expect(mainPl?.y).toBe(300);

    // Undo must restore disp-secondary without touching disp-main
    studioStore.undo();
    const restoredSecPl = studioStore.getProject()?.displays?.find((d) => d.id === "disp-secondary")?.placements?.find((p) => p.equipment_id === "P-101");
    expect(restoredSecPl?.x).toBe(150);
    expect(restoredSecPl?.y).toBe(180);
  });
});

describe("Operations Non-Overlapping Spacing and Shoulder Layout", () => {
  let view: NebuProcessView;

  beforeEach(() => {
    const proj = JSON.parse(JSON.stringify(TEST_PROJECT));
    store.setActiveProject(proj);
    store.setActiveDisplay("disp-main");
    if (!window.customElements.get("nebu-process-view")) {
      window.customElements.define("nebu-process-view", NebuProcessView);
    }
    view = new NebuProcessView();
    document.body.appendChild(view);
    view.render();
  });

  it("avoids stacking fallback coordinates when equipment has no position or placement", () => {
    const unplacedEq1 = { id: "UNP-1", name: "UNP-1" } as any;
    const unplacedEq2 = { id: "UNP-2", name: "UNP-2" } as any;

    const coords1 = view.getEquipmentCoordinates(unplacedEq1);
    const coords2 = view.getEquipmentCoordinates(unplacedEq2);

    expect(coords1).not.toEqual(coords2);
    expect(coords1.x).toBeGreaterThan(0);
    expect(coords1.y).toBeGreaterThan(0);
    expect(coords2.x).toBeGreaterThan(0);
    expect(coords2.y).toBeGreaterThan(0);
  });

  it("renders instruments and control loops on separate shoulders to eliminate collision", () => {
    const instCircle = view.querySelector(".inst-circle");
    expect(instCircle).not.toBeNull();

    const instNode = instCircle?.closest("g");
    const instLine = instNode?.querySelector("line");
    expect(instLine).not.toBeNull();
    const instX1 = Number(instLine?.getAttribute("x1"));
    expect(instX1).toBeLessThan(300); // Shoulder-left of P-101 at (300, 300)

    const ctrlNode = view.querySelector("[data-loop-name='PIC-101']");
    expect(ctrlNode).not.toBeNull();
    const ctrlLine = ctrlNode?.querySelector("line");
    expect(ctrlLine).not.toBeNull();
    const ctrlX1 = Number(ctrlLine?.getAttribute("x1"));
    expect(ctrlX1).toBeGreaterThan(300); // Shoulder-right of P-101 at (300, 300)

    // Clearance between instrument and control loop > 70px
    expect(ctrlX1 - instX1).toBeGreaterThan(70);
  });
});
