/**
 * Operations Process View Authoritative Geometry Parity & Display Isolation Tests
 *
 * Verifies:
 * 1. Operations renders strictly the equipment belonging to activeDisplay (e.g. 7 on disp-nht-feed, NOT 41).
 * 2. Equipment placement coordinates (x, y, rotation) in Operations match the authoritative model.
 * 3. No two rendered equipment items overlap/collide (clearance >= 24px maintained).
 * 4. In-display streams connect directly without redundant off-page badges.
 * 5. Cross-display boundary streams render off-page continuation badges (TO ... / FROM ...).
 * 6. Off-page continuation badges trigger cross-display navigation to target display.
 */

import { describe, expect, it, beforeEach } from "vitest";
import { NebuProcessView } from "../src/components/nebu-process-view";
import { store } from "../src/state/store";
import { ProjectDetailDto } from "../src/protocol/types";
import { testEquipmentCollision } from "../src/domain/geometry-collision";

const NHT_MOCK_PROJECT: ProjectDetailDto = {
  id: "proj-domain-nht-reference",
  name: "Naphtha Hydrotreater (NHT) Reference Plant",
  schema_version: "0.1.0",
  description: "Forensic validation project with authoritative placements",
  author: "EIL Benchmark",
  created: "2026-09-09T00:00:00Z",
  modified: "2026-09-09T00:00:00Z",
  equipment: [
    // Feed System (7 equipment items)
    { id: "TK-101", name: "TK-101", definition_id: "standard.tank.atmospheric", definition_version: "1.0.0", position: { x: 80, y: 360 }, display_width: 100, display_height: 80, parameters: {} },
    { id: "V-101", name: "V-101", definition_id: "standard.vessel.vertical_separator", definition_version: "1.0.0", position: { x: 220, y: 360 }, display_width: 80, display_height: 120, parameters: {} },
    { id: "P-101A", name: "P-101A", definition_id: "standard.pump.centrifugal", definition_version: "1.0.0", position: { x: 360, y: 360 }, display_width: 90, display_height: 60, parameters: {} },
    { id: "P-101B", name: "P-101B", definition_id: "standard.pump.centrifugal", definition_version: "1.0.0", position: { x: 360, y: 540 }, display_width: 90, display_height: 60, parameters: {} },
    { id: "FV-101", name: "FV-101", definition_id: "standard.valve.control_globe", definition_version: "1.0.0", position: { x: 500, y: 360 }, display_width: 50, display_height: 40, parameters: {} },
    { id: "X-101", name: "X-101", definition_id: "standard.vessel.vertical_separator", definition_version: "1.0.0", position: { x: 640, y: 360 }, display_width: 70, display_height: 100, parameters: {} },
    { id: "MIX-101", name: "MIX-101", definition_id: "standard.mixer.static", definition_version: "1.0.0", position: { x: 780, y: 360 }, display_width: 50, display_height: 35, parameters: {} },

    // Reactor System (Equipment on disp-nht-rx)
    { id: "E-101A", name: "E-101A", definition_id: "standard.exchanger.shell_tube", definition_version: "1.0.0", position: { x: 200, y: 300 }, display_width: 100, display_height: 60, parameters: {} },
    { id: "F-101", name: "F-101", definition_id: "standard.furnace.fired_heater", definition_version: "1.0.0", position: { x: 450, y: 300 }, display_width: 120, display_height: 140, parameters: {} },
    { id: "R-101", name: "R-101", definition_id: "standard.reactor.fixed_bed", definition_version: "1.0.0", position: { x: 700, y: 300 }, display_width: 100, display_height: 180, parameters: {} },
  ],
  streams: [
    // In-display feed streams
    { id: "STR-NHT-01", name: "STR-NHT-01", stream_type: "Material", source_equipment_id: "TK-101", source_port_id: "outlet", sink_equipment_id: "P-101A", sink_port_id: "suction", composition: {}, route_points: [] },
    { id: "STR-NHT-02", name: "STR-NHT-02", stream_type: "Material", source_equipment_id: "P-101A", source_port_id: "discharge", sink_equipment_id: "FV-101", sink_port_id: "inlet", composition: {}, route_points: [] },
    { id: "STR-NHT-03", name: "STR-NHT-03", stream_type: "Material", source_equipment_id: "FV-101", source_port_id: "outlet", sink_equipment_id: "X-101", sink_port_id: "feed", composition: {}, route_points: [] },
    { id: "STR-NHT-04", name: "STR-NHT-04", stream_type: "Material", source_equipment_id: "X-101", source_port_id: "liquid_out", sink_equipment_id: "MIX-101", sink_port_id: "primary_in", composition: {}, route_points: [] },

    // Cross-display boundary stream: MIX-101 (feed) -> E-101A (reactor)
    { id: "STR-NHT-05", name: "STR-NHT-05", stream_type: "Material", source_equipment_id: "MIX-101", source_port_id: "mixture_out", sink_equipment_id: "E-101A", sink_port_id: "tube_in", composition: {}, route_points: [] },

    // In-display reactor streams
    { id: "STR-NHT-12", name: "STR-NHT-12", stream_type: "Material", source_equipment_id: "E-101A", source_port_id: "tube_out", sink_equipment_id: "F-101", sink_port_id: "process_in", composition: {}, route_points: [] },
    { id: "STR-NHT-13", name: "STR-NHT-13", stream_type: "Material", source_equipment_id: "F-101", source_port_id: "process_out", sink_equipment_id: "R-101", sink_port_id: "feed", composition: {}, route_points: [] },
  ],
  instruments: [
    { id: "inst-1", tag: "FT-101", measurement_kind: "Flow", equipment_id: "FV-101", position: { x: 500, y: 300 }, range_min: 0, range_max: 250, unit: "m3/h" },
    { id: "inst-2", tag: "TT-101", measurement_kind: "Temperature", equipment_id: "F-101", position: { x: 450, y: 220 }, range_min: 0, range_max: 450, unit: "°C" },
  ],
  control_loops: [
    { id: "loop-1", name: "FIC-101", pv_tag: "FT-101", sp: 199.0, out_tag: "FV-101.OPENING", kp: 0.8, ti: 6.0, td: 0.0, reverse_acting: false, mode: "AUTO", output_min: 0, output_max: 100, position: { x: 500, y: 240 } },
  ],
  alarms: [],
  interlocks: [],
  displays: [
    {
      id: "disp-nht-feed",
      name: "NHT Feed & Surge System",
      display_type: "unit_process_graphic",
      equipment_ids: ["TK-101", "V-101", "P-101A", "P-101B", "FV-101", "X-101", "MIX-101"],
      placements: [
        { equipment_id: "TK-101", x: 80, y: 360, width: 100, height: 80, rotation: 0 },
        { equipment_id: "V-101", x: 220, y: 360, width: 80, height: 120, rotation: 0 },
        { equipment_id: "P-101A", x: 360, y: 360, width: 90, height: 60, rotation: 0 },
        { equipment_id: "P-101B", x: 360, y: 540, width: 90, height: 60, rotation: 0 },
        { equipment_id: "FV-101", x: 500, y: 360, width: 50, height: 40, rotation: 0 },
        { equipment_id: "X-101", x: 640, y: 360, width: 70, height: 100, rotation: 0 },
        { equipment_id: "MIX-101", x: 780, y: 360, width: 50, height: 35, rotation: 0 },
      ],
      stream_ids: ["STR-NHT-01", "STR-NHT-02", "STR-NHT-03", "STR-NHT-04", "STR-NHT-05"],
      instrument_ids: ["inst-1"],
    },
    {
      id: "disp-nht-rx",
      name: "Combined Feed Exchanger & Reactor System",
      display_type: "unit_process_graphic",
      equipment_ids: ["E-101A", "F-101", "R-101"],
      placements: [
        { equipment_id: "E-101A", x: 200, y: 300, width: 100, height: 60, rotation: 0 },
        { equipment_id: "F-101", x: 450, y: 300, width: 120, height: 140, rotation: 0 },
        { equipment_id: "R-101", x: 700, y: 300, width: 100, height: 180, rotation: 0 },
      ],
      stream_ids: ["STR-NHT-05", "STR-NHT-12", "STR-NHT-13"],
      instrument_ids: ["inst-2"],
    },
  ],
  navigation_links: [
    {
      id: "nav-feed-to-rx",
      source_display_id: "disp-nht-feed",
      target_display_id: "disp-nht-rx",
      source_element_id: "STR-NHT-05",
      label: "To Reactor Section",
    },
    {
      id: "nav-rx-to-feed",
      source_display_id: "disp-nht-rx",
      target_display_id: "disp-nht-feed",
      source_element_id: "STR-NHT-05",
      label: "From Feed Section",
    },
  ],
};

describe("Operations Authoritative Geometry & Display Isolation Parity", () => {
  let view: NebuProcessView;

  beforeEach(() => {
    store.setActiveProject(NHT_MOCK_PROJECT);
    store.setActiveDisplay("disp-nht-feed");

    if (!window.customElements.get("nebu-process-view")) {
      window.customElements.define("nebu-process-view", NebuProcessView);
    }
    view = new NebuProcessView();
    document.body.appendChild(view);
    view.render();
  });

  it("isolates rendering to exactly the 7 equipment items on disp-nht-feed (not all 10)", () => {
    const renderedNodes = view.querySelectorAll(".equipment-node[data-eq-id]");
    expect(renderedNodes.length).toBe(7);

    // Verify all 7 feed items are rendered
    const renderedIds = Array.from(renderedNodes).map((node) => node.getAttribute("data-eq-id"));
    expect(renderedIds).toContain("TK-101");
    expect(renderedIds).toContain("V-101");
    expect(renderedIds).toContain("P-101A");
    expect(renderedIds).toContain("P-101B");
    expect(renderedIds).toContain("FV-101");
    expect(renderedIds).toContain("X-101");
    expect(renderedIds).toContain("MIX-101");

    // Equipment from other displays must NOT be rendered
    expect(renderedIds).not.toContain("E-101A");
    expect(renderedIds).not.toContain("F-101");
    expect(renderedIds).not.toContain("R-101");
  });

  it("renders equipment at exact authoritative placement coordinates", () => {
    const feedPlacements = NHT_MOCK_PROJECT.displays![0].placements!;

    for (const pl of feedPlacements) {
      const node = view.querySelector(`.equipment-node[data-eq-id="${pl.equipment_id}"]`);
      expect(node).not.toBeNull();
      expect(node?.getAttribute("data-cx")).toBe(String(pl.x));
      expect(node?.getAttribute("data-cy")).toBe(String(pl.y));
    }
  });

  it("guarantees zero collisions and strict clearance between rendered equipment", () => {
    const feedPlacements = NHT_MOCK_PROJECT.displays![0].placements!;
    const placedEqList = feedPlacements.map((pl) => ({
      id: pl.equipment_id,
      name: pl.equipment_id,
      definition_id: "test",
      definition_version: "1.0.0",
      position: { x: pl.x, y: pl.y },
      display_width: pl.width || 90,
      display_height: pl.height || 60,
      parameters: {},
    }));

    for (let i = 0; i < placedEqList.length; i++) {
      const target = placedEqList[i];
      const others = placedEqList.filter((_, idx) => idx !== i);
      const collisionResult = testEquipmentCollision(
        target.position,
        target.display_width,
        target.display_height,
        others,
        target.id,
        24 // 24px industrial clearance
      );
      expect(collisionResult.isColliding).toBe(false);
      expect(collisionResult.collidingIds.length).toBe(0);
    }
  });

  it("renders off-page continuation badge only for cross-display boundary streams", () => {
    const badges = view.querySelectorAll(".off-page-badge");
    // Only STR-NHT-05 leaves the display towards E-101A
    expect(badges.length).toBe(1);

    const badgeText = badges[0].querySelector("text")?.textContent;
    expect(badgeText).toContain("TO E-101A");
  });

  it("switches active display to disp-nht-rx and isolates reactor equipment cleanly", () => {
    store.setActiveDisplay("disp-nht-rx");
    view.render();

    const renderedNodes = view.querySelectorAll(".equipment-node[data-eq-id]");
    expect(renderedNodes.length).toBe(3);

    const renderedIds = Array.from(renderedNodes).map((node) => node.getAttribute("data-eq-id"));
    expect(renderedIds).toContain("E-101A");
    expect(renderedIds).toContain("F-101");
    expect(renderedIds).toContain("R-101");
    expect(renderedIds).not.toContain("TK-101");

    // Incoming boundary stream STR-NHT-05 should display FROM MIX-101
    const badges = view.querySelectorAll(".off-page-badge");
    expect(badges.length).toBe(1);
    const badgeText = badges[0].querySelector("text")?.textContent;
    expect(badgeText).toContain("FROM MIX-101");
  });

  it("clicking off-page continuation badge navigates to connected display", () => {
    store.setActiveDisplay("disp-nht-feed");
    view.render();

    const badge = view.querySelector(".off-page-badge") as SVGGElement;
    expect(badge).not.toBeNull();

    // Trigger click on off-page badge
    badge.dispatchEvent(new MouseEvent("click", { bubbles: true }));

    // Store should now have activeDisplay changed to disp-nht-rx
    expect(store.activeDisplayId).toBe("disp-nht-rx");
  });

  it("splits tracks vertically for opposing TO and FROM badges to prevent collision in tight layouts", () => {
    // Construct a project with two adjacent equipment on the same horizontal line (dist = 180px < 320px)
    // where eq1 has an outgoing boundary stream and eq2 has an incoming boundary stream
    const tightProject: ProjectDetailDto = {
      ...NHT_MOCK_PROJECT,
      id: "proj-tight-layout",
      displays: [
        {
          id: "disp-tight",
          name: "Tight Layout Test",
          display_type: "unit_process_graphic",
          equipment_ids: ["FV-101", "E-101A"],
          placements: [
            { equipment_id: "FV-101", x: 400, y: 360, width: 50, height: 40, rotation: 0 },
            { equipment_id: "E-101A", x: 580, y: 360, width: 100, height: 60, rotation: 0 },
          ],
          stream_ids: ["STR-OUT", "STR-IN"],
          instrument_ids: [],
        },
      ],
      streams: [
        // Outgoing boundary stream from FV-101 to foreign equipment
        { id: "STR-OUT", name: "STR-OUT", stream_type: "Material", source_equipment_id: "FV-101", source_port_id: "outlet", sink_equipment_id: "X-101", sink_port_id: "feed", composition: {}, route_points: [] },
        // Incoming boundary stream to E-101A from foreign equipment
        { id: "STR-IN", name: "STR-IN", stream_type: "Material", source_equipment_id: "TK-101", source_port_id: "outlet", sink_equipment_id: "E-101A", sink_port_id: "tube_in", composition: {}, route_points: [] },
      ],
    };

    store.setActiveProject(tightProject);
    store.setActiveDisplay("disp-tight");
    view.render();

    const badges = Array.from(view.querySelectorAll(".off-page-badge")) as SVGGElement[];
    expect(badges.length).toBe(2);

    const toBadge = badges.find((b) => b.textContent?.includes("TO X-101"));
    const fromBadge = badges.find((b) => b.textContent?.includes("FROM TK-101"));
    expect(toBadge).toBeDefined();
    expect(fromBadge).toBeDefined();

    // Extract Y coordinates of polygons
    const getPolygonPoints = (badge: SVGGElement) => {
      const poly = badge.querySelector("polygon");
      const ptsStr = poly?.getAttribute("points") || "";
      return ptsStr.trim().split(/\s+/).map((pt) => {
        const [x, y] = pt.split(",").map(Number);
        return { x, y };
      });
    };

    const toPts = getPolygonPoints(toBadge!);
    const fromPts = getPolygonPoints(fromBadge!);

    const toYCenter = toPts.reduce((acc, p) => acc + p.y, 0) / toPts.length;
    const fromYCenter = fromPts.reduce((acc, p) => acc + p.y, 0) / fromPts.length;

    // Outgoing TO badge should be routed to upper track (y < 360)
    expect(toYCenter).toBeLessThan(360);
    // Incoming FROM badge should be routed to lower track (y > 360)
    expect(fromYCenter).toBeGreaterThan(360);

    // Vertical clearance between the two tracks must be >= 40px
    const verticalClearance = fromYCenter - toYCenter;
    expect(verticalClearance).toBeGreaterThanOrEqual(40);
  });

  it("staggers multiple boundary badges on the same equipment to avoid stacking", () => {
    // Construct a project where an equipment item has two outgoing boundary streams
    const multiBadgeProject: ProjectDetailDto = {
      ...NHT_MOCK_PROJECT,
      id: "proj-multi-badge",
      displays: [
        {
          id: "disp-multi",
          name: "Multi Badge Test",
          display_type: "unit_process_graphic",
          equipment_ids: ["V-101"],
          placements: [
            { equipment_id: "V-101", x: 300, y: 360, width: 80, height: 120, rotation: 0 },
          ],
          stream_ids: ["STR-OUT-1", "STR-OUT-2"],
          instrument_ids: [],
        },
      ],
      streams: [
        { id: "STR-OUT-1", name: "STR-OUT-1", stream_type: "Material", source_equipment_id: "V-101", source_port_id: "outlet", sink_equipment_id: "P-101A", sink_port_id: "suction", composition: {}, route_points: [] },
        { id: "STR-OUT-2", name: "STR-OUT-2", stream_type: "Material", source_equipment_id: "V-101", source_port_id: "outlet", sink_equipment_id: "P-101B", sink_port_id: "suction", composition: {}, route_points: [] },
      ],
    };

    store.setActiveProject(multiBadgeProject);
    store.setActiveDisplay("disp-multi");
    view.render();

    const badges = Array.from(view.querySelectorAll(".off-page-badge")) as SVGGElement[];
    expect(badges.length).toBe(2);

    const getPolyYCenter = (badge: SVGGElement) => {
      const poly = badge.querySelector("polygon");
      const ptsStr = poly?.getAttribute("points") || "";
      const pts = ptsStr.trim().split(/\s+/).map((pt) => Number(pt.split(",")[1]));
      return pts.reduce((a, b) => a + b, 0) / pts.length;
    };

    const y1 = getPolyYCenter(badges[0]);
    const y2 = getPolyYCenter(badges[1]);

    // Badges must not have identical Y coordinates (staggered by 24px)
    expect(Math.abs(y1 - y2)).toBeGreaterThanOrEqual(20);
  });
});

