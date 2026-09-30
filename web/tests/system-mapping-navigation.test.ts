/**
 * Stage 14 System Mapping & Cross-Display Navigation Test Suite.
 *
 * Verifies the 5-tier architecture:
 * 1. Physical Plant Model (independent equipment & topology)
 * 2. Operational Organization (Plant -> Area -> System -> Subsystem -> Unit)
 * 3. Display Model (DisplayDto with equipment_ids, viewport, display_type)
 * 4. Navigation Model (NavigationLinkDto and continuation badges)
 * 5. Deployed DCS Operations (execution of cross-display navigation & hierarchy traversal)
 */

import { beforeEach, describe, expect, it } from "vitest";
import {
  DisplayDto,
  NavigationLinkDto,
  PlantStructureDto,
  ProjectDetailDto,
} from "../src/protocol/types";
import { store } from "../src/state/store";
import { studioStore } from "../src/state/studio-store";

const MOCK_PLANT_STRUCTURE: PlantStructureDto = {
  plant_name: "Integrated Energy & Chemical Complex",
  displays: [],
  areas: [
    {
      id: "area-hydro",
      name: "Area 100 - Hydroprocessing",
      systems: [
        {
          id: "sys-hp-react",
          name: "HP Reaction & Separation",
          subsystems: [
            {
              id: "subsys-hp-loop",
              name: "High Pressure Recycle Loop",
              units: [
                {
                  id: "unit-sep-100",
                  name: "Separator Unit 100",
                  equipment_ids: ["eq-vessel-01", "eq-valve-01"],
                  display_ids: ["disp-sep-unit"],
                },
                {
                  id: "unit-comp-100",
                  name: "Recycle Compression Unit",
                  equipment_ids: ["eq-pump-01"],
                  display_ids: ["disp-comp-unit"],
                },
              ],
            },
          ],
        },
      ],
    },
  ],
};

const MOCK_DISPLAYS: DisplayDto[] = [
  {
    id: "disp-plant-overview",
    name: "Plant Overview",
    display_type: "plant_overview",
    equipment_ids: ["eq-pump-01", "eq-valve-01", "eq-vessel-01"],
    instrument_ids: [],
    stream_ids: [],
    viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 0.8 },
    is_default: true,
  },
  {
    id: "disp-hp-system",
    name: "HP Reaction System Overview",
    display_type: "system_overview",
    system_id: "sys-hp-react",
    equipment_ids: ["eq-pump-01", "eq-valve-01", "eq-vessel-01"],
    instrument_ids: [],
    stream_ids: [],
    viewport: { x: 100, y: 50, width: 1920, height: 1080, zoom: 1.0 },
  },
  {
    id: "disp-sep-unit",
    name: "Separator Unit Graphic",
    display_type: "unit_process_graphic",
    unit_id: "unit-sep-100",
    equipment_ids: ["eq-vessel-01", "eq-valve-01"],
    instrument_ids: [],
    stream_ids: [],
    viewport: { x: 200, y: 100, width: 1920, height: 1080, zoom: 1.2 },
  },
  {
    id: "disp-comp-unit",
    name: "Recycle Compression Graphic",
    display_type: "unit_process_graphic",
    unit_id: "unit-comp-100",
    equipment_ids: ["eq-pump-01"],
    instrument_ids: [],
    stream_ids: [],
    viewport: { x: 50, y: 50, width: 1920, height: 1080, zoom: 1.2 },
  },
];

const MOCK_NAV_LINKS: NavigationLinkDto[] = [
  {
    id: "nav-sep-to-comp",
    source_display_id: "disp-sep-unit",
    source_element_id: "eq-valve-01",
    target_display_id: "disp-comp-unit",
    target_element_id: "eq-pump-01",
    label: "To Recycle Compressor C-101",
    behavior: "focus_and_center",
  },
];

const MOCK_PROJECT: ProjectDetailDto = {
  id: "proj-plant-nav-01",
  name: "Integrated Process & Navigation Model",
  schema_version: "0.1.0",
  description: "Test project for cross-display navigation and plant mapping",
  author: "NebuDCS Verification Suite",
  created: "2026-09-09T00:00:00Z",
  modified: "2026-09-09T00:00:00Z",
  equipment: [
    {
      id: "eq-pump-01",
      definition_id: "standard.pump.centrifugal",
      definition_version: "1.0.0",
      name: "C-101",
      parameters: { design_flow: 5000 },
      position: { x: 100, y: 150 },
    },
    {
      id: "eq-valve-01",
      definition_id: "standard.valve.control_globe",
      definition_version: "1.0.0",
      name: "FV-101",
      parameters: { cv: 80 },
      position: { x: 300, y: 150 },
    },
    {
      id: "eq-vessel-01",
      definition_id: "standard.vessel.vertical_separator",
      definition_version: "1.0.0",
      name: "V-101",
      parameters: { volume: 20 },
      position: { x: 500, y: 150 },
    },
  ],
  streams: [
    {
      id: "st-01",
      name: "Stream 01",
      stream_type: "material",
      source_equipment_id: "eq-vessel-01",
      source_port_id: "out-gas",
      sink_equipment_id: "eq-valve-01",
      sink_port_id: "inlet",
      composition: {},
      route_points: [],
    },
    {
      id: "st-02",
      name: "Stream 02",
      stream_type: "material",
      source_equipment_id: "eq-valve-01",
      source_port_id: "outlet",
      sink_equipment_id: "eq-pump-01",
      sink_port_id: "suction",
      composition: {},
      route_points: [],
    },
  ],
  instruments: [],
  control_loops: [],
  alarms: [],
  interlocks: [],
  plant_structure: MOCK_PLANT_STRUCTURE,
  displays: MOCK_DISPLAYS,
  navigation_links: MOCK_NAV_LINKS,
};

describe("Stage 14: Plant Modeling, System Mapping & Navigation", () => {
  beforeEach(() => {
    // Reset stores with mock project
    store.setActiveProject(JSON.parse(JSON.stringify(MOCK_PROJECT)));
    studioStore.setProject(JSON.parse(JSON.stringify(MOCK_PROJECT)));
  });

  describe("Engineering Studio: Operational Mapping & Navigation Authoring", () => {
    it("assigns equipment to operational units non-destructively", () => {
      const proj = studioStore.getProject();
      expect(proj).toBeDefined();

      // Move eq-pump-01 to unit-sep-100
      studioStore.setEquipmentOperationalMapping("eq-pump-01", "unit-sep-100");

      const updated = studioStore.getProject();
      const unitSep = updated?.plant_structure?.areas[0].systems[0].subsystems[0].units.find(
        (u) => u.id === "unit-sep-100"
      );
      expect(unitSep?.equipment_ids).toContain("eq-pump-01");

      // Verify undo restores previous unit mapping
      studioStore.undo();
      const afterUndo = studioStore.getProject();
      const unitSepAfterUndo = afterUndo?.plant_structure?.areas[0].systems[0].subsystems[0].units.find(
        (u) => u.id === "unit-sep-100"
      );
      expect(unitSepAfterUndo?.equipment_ids).not.toContain("eq-pump-01");
    });

    it("manages multi-display membership for physical equipment", () => {
      // Assign eq-vessel-01 to disp-comp-unit as well
      studioStore.setEquipmentDisplays("eq-vessel-01", [
        "disp-plant-overview",
        "disp-hp-system",
        "disp-sep-unit",
        "disp-comp-unit",
      ]);

      const updated = studioStore.getProject();
      const compDisp = updated?.displays?.find((d) => d.id === "disp-comp-unit");
      expect(compDisp?.equipment_ids).toContain("eq-vessel-01");

      // Undo removes it
      studioStore.undo();
      const afterUndo = studioStore.getProject();
      const compDispAfterUndo = afterUndo?.displays?.find((d) => d.id === "disp-comp-unit");
      expect(compDispAfterUndo?.equipment_ids).not.toContain("eq-vessel-01");
    });

    it("creates and removes cross-display navigation links with undo/redo", () => {
      const newLink: NavigationLinkDto = {
        id: "nav-vessel-to-plant",
        source_display_id: "disp-sep-unit",
        source_element_id: "eq-vessel-01",
        target_display_id: "disp-plant-overview",
        target_element_id: "eq-vessel-01",
        label: "Overview",
        behavior: "focus_and_center",
      };

      studioStore.addNavigationLink(newLink);
      let proj = studioStore.getProject();
      expect(proj?.navigation_links?.some((l) => l.id === "nav-vessel-to-plant")).toBe(true);

      // Delete link
      studioStore.deleteNavigationLink("nav-vessel-to-plant");
      proj = studioStore.getProject();
      expect(proj?.navigation_links?.some((l) => l.id === "nav-vessel-to-plant")).toBe(false);

      // Undo deletion
      studioStore.undo();
      proj = studioStore.getProject();
      expect(proj?.navigation_links?.some((l) => l.id === "nav-vessel-to-plant")).toBe(true);
    });
  });

  describe("Operations DCS: Model-Driven Hierarchy & Cross-Display Navigation", () => {
    it("initializes active display to the default overview display", () => {
      expect(store.currentDisplay?.id).toBe("disp-plant-overview");
    });

    it("executes cross-display navigation link and focuses target element", () => {
      // Start in separator unit display
      store.navigateDisplay("disp-sep-unit");
      expect(store.currentDisplay?.id).toBe("disp-sep-unit");

      // Execute navigation link to compression unit
      const link = MOCK_NAV_LINKS[0];
      store.executeNavigationLink(link);

      expect(store.currentDisplay?.id).toBe("disp-comp-unit");
      expect(store.focusedEquipmentId).toBe("eq-pump-01");
      expect(store.selectedEquipmentId).toBe("eq-pump-01");
    });

    it("navigates up hierarchically using authoritative plant structure", () => {
      // Set display to separator unit graphic
      store.navigateDisplay("disp-sep-unit");
      expect(store.currentDisplay?.id).toBe("disp-sep-unit");

      // Navigate up -> should reach HP Reaction System Overview
      store.navigateUp();
      expect(store.currentDisplay?.id).toBe("disp-hp-system");

      // Navigate up again -> should reach Plant Overview
      store.navigateUp();
      expect(store.currentDisplay?.id).toBe("disp-plant-overview");
    });

    it("navigates home directly to default plant overview", () => {
      store.navigateDisplay("disp-comp-unit");
      expect(store.currentDisplay?.id).toBe("disp-comp-unit");

      store.navigateHome();
      expect(store.currentDisplay?.id).toBe("disp-plant-overview");
    });
  });
});
