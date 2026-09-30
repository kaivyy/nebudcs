/**
 * Stage 14 Full Plant Reconstruction & Engineering Studio Architecture Acceptance Tests.
 *
 * Implements and verifies Section 20 & 21 Acceptance Scenario:
 * - Physical Topology: TK-101 -> P-101 -> C-101 -> E-101 -> V-101 -> C-201
 * - Operational Structure: Process Area -> LP System (P-101), HP System (C-101, E-101, C-201), Separation System (V-101)
 * - Displays: Plant Overview, Area Overview, LP Overview, HP Overview, Compression Overview, Separation Overview, C-101 Detail
 * - Multi-display memberships, cross-display placements, stream continuity, and model-driven navigation
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

const RECON_PLANT_STRUCTURE: PlantStructureDto = {
  plant_name: "Integrated Process Complex 2026",
  displays: [],
  areas: [
    {
      id: "area-proc",
      name: "Process Area",
      systems: [
        {
          id: "sys-lp",
          name: "LP System",
          area_id: "area-proc",
          subsystems: [
            {
              id: "subsys-lp-feed",
              name: "LP Feed Subsystem",
              system_id: "sys-lp",
              units: [
                {
                  id: "unit-p101",
                  name: "Feed Pumping Unit",
                  subsystem_id: "subsys-lp-feed",
                  system_id: "sys-lp",
                  area_id: "area-proc",
                  equipment_ids: ["eq-p101", "eq-tk101"],
                  display_ids: ["disp-lp-overview"],
                },
              ],
            },
          ],
        },
        {
          id: "sys-hp",
          name: "HP System",
          area_id: "area-proc",
          subsystems: [
            {
              id: "subsys-hp-comp",
              name: "HP Compression & Treatment Subsystem",
              system_id: "sys-hp",
              units: [
                {
                  id: "unit-comp-train",
                  name: "Compression Train",
                  subsystem_id: "subsys-hp-comp",
                  system_id: "sys-hp",
                  area_id: "area-proc",
                  equipment_ids: ["eq-c101", "eq-e101", "eq-c201"],
                  display_ids: ["disp-hp-overview", "disp-comp-overview", "disp-c101-detail"],
                },
              ],
            },
          ],
        },
        {
          id: "sys-sep",
          name: "Separation System",
          area_id: "area-proc",
          subsystems: [
            {
              id: "subsys-sep-vessel",
              name: "Separation Drum Subsystem",
              system_id: "sys-sep",
              units: [
                {
                  id: "unit-v101",
                  name: "Separator Unit",
                  subsystem_id: "subsys-sep-vessel",
                  system_id: "sys-sep",
                  area_id: "area-proc",
                  equipment_ids: ["eq-v101"],
                  display_ids: ["disp-sep-overview"],
                },
              ],
            },
          ],
        },
      ],
    },
  ],
};

const RECON_DISPLAYS: DisplayDto[] = [
  {
    id: "disp-plant-overview",
    name: "Plant Overview",
    display_type: "plant_overview",
    equipment_ids: ["eq-tk101", "eq-p101", "eq-c101", "eq-e101", "eq-v101", "eq-c201"],
    instrument_ids: [],
    stream_ids: ["st-01", "st-02", "st-03", "st-04", "st-05"],
    viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 0.7 },
    is_default: true,
  },
  {
    id: "disp-area-overview",
    name: "Area Overview",
    display_type: "area_overview",
    area_id: "area-proc",
    equipment_ids: ["eq-tk101", "eq-p101", "eq-c101", "eq-e101", "eq-v101", "eq-c201"],
    instrument_ids: [],
    stream_ids: ["st-01", "st-02", "st-03", "st-04", "st-05"],
    viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 0.8 },
  },
  {
    id: "disp-lp-overview",
    name: "LP Overview",
    display_type: "system_overview",
    system_id: "sys-lp",
    area_id: "area-proc",
    equipment_ids: ["eq-tk101", "eq-p101", "eq-c101"],
    instrument_ids: [],
    stream_ids: ["st-01", "st-02"],
    viewport: { x: 50, y: 50, width: 1920, height: 1080, zoom: 1.0 },
    placements: [
      { equipment_id: "eq-c101", x: 600, y: 250 },
    ],
  },
  {
    id: "disp-hp-overview",
    name: "HP Overview",
    display_type: "system_overview",
    system_id: "sys-hp",
    area_id: "area-proc",
    equipment_ids: ["eq-c101", "eq-e101", "eq-c201"],
    instrument_ids: [],
    stream_ids: ["st-03", "st-05"],
    viewport: { x: 200, y: 100, width: 1920, height: 1080, zoom: 1.0 },
    placements: [
      { equipment_id: "eq-c101", x: 220, y: 300 },
      { equipment_id: "eq-e101", x: 520, y: 300 },
      { equipment_id: "eq-c201", x: 820, y: 300 },
    ],
  },
  {
    id: "disp-comp-overview",
    name: "Compression Overview",
    display_type: "system_overview",
    system_id: "sys-hp",
    area_id: "area-proc",
    equipment_ids: ["eq-p101", "eq-c101", "eq-e101", "eq-c201"],
    instrument_ids: [],
    stream_ids: ["st-02", "st-03", "st-05"],
    viewport: { x: 100, y: 100, width: 1920, height: 1080, zoom: 0.9 },
  },
  {
    id: "disp-sep-overview",
    name: "Separation Overview",
    display_type: "system_overview",
    system_id: "sys-sep",
    area_id: "area-proc",
    equipment_ids: ["eq-v101"],
    instrument_ids: [],
    stream_ids: ["st-04", "st-05"],
    viewport: { x: 300, y: 200, width: 1920, height: 1080, zoom: 1.2 },
  },
  {
    id: "disp-c101-detail",
    name: "C-101 Detail",
    display_type: "unit_process_graphic",
    unit_id: "unit-comp-train",
    system_id: "sys-hp",
    area_id: "area-proc",
    equipment_ids: ["eq-c101"],
    instrument_ids: [],
    stream_ids: ["st-02", "st-03"],
    viewport: { x: 200, y: 200, width: 1920, height: 1080, zoom: 1.4 },
  },
];

const RECON_NAV_LINKS: NavigationLinkDto[] = [
  {
    id: "nav-lp-to-hp",
    source_display_id: "disp-lp-overview",
    source_element_id: "eq-p101",
    target_display_id: "disp-hp-overview",
    target_element_id: "eq-c101",
    label: "TO HP OVERVIEW",
    behavior: "focus_and_center",
  },
  {
    id: "nav-c101-to-detail",
    source_display_id: "disp-hp-overview",
    source_element_id: "eq-c101",
    target_display_id: "disp-c101-detail",
    target_element_id: "eq-c101",
    label: "C-101 DETAIL",
    behavior: "open_faceplate",
  },
  {
    id: "nav-hp-to-comp",
    source_display_id: "disp-hp-overview",
    source_element_id: "eq-c101",
    target_display_id: "disp-comp-overview",
    target_element_id: "eq-c201",
    label: "COMPRESSION TRAIN OVERVIEW",
    behavior: "focus_and_center",
  },
  {
    id: "nav-e101-to-sep",
    source_display_id: "disp-hp-overview",
    source_element_id: "eq-e101",
    target_display_id: "disp-sep-overview",
    target_element_id: "eq-v101",
    label: "TO SEPARATION OVERVIEW",
    behavior: "focus_and_center",
  },
];

const RECON_PROJECT: ProjectDetailDto = {
  id: "proj-recon-acceptance-01",
  name: "Full Plant Reconstruction Acceptance Model",
  schema_version: "0.1.0",
  description: "TK-101 -> P-101 -> C-101 -> E-101 -> V-101 -> C-201",
  author: "NebuDCS Acceptance Suite",
  created: "2026-09-09T00:00:00Z",
  modified: "2026-09-09T00:00:00Z",
  equipment: [
    {
      id: "eq-tk101",
      definition_id: "standard.tank.storage",
      definition_version: "1.0.0",
      name: "TK-101",
      parameters: { capacity: 1000 },
      position: { x: 80, y: 220 },
    },
    {
      id: "eq-p101",
      definition_id: "standard.pump.centrifugal",
      definition_version: "1.0.0",
      name: "P-101",
      parameters: { design_flow: 3000 },
      position: { x: 260, y: 220 },
    },
    {
      id: "eq-c101",
      definition_id: "standard.compressor.centrifugal",
      definition_version: "1.0.0",
      name: "C-101",
      parameters: { rated_power: 450 },
      position: { x: 480, y: 220 },
    },
    {
      id: "eq-e101",
      definition_id: "standard.heat_exchanger.shell_tube",
      definition_version: "1.0.0",
      name: "E-101",
      parameters: { area: 120 },
      position: { x: 700, y: 220 },
    },
    {
      id: "eq-v101",
      definition_id: "standard.vessel.vertical_separator",
      definition_version: "1.0.0",
      name: "V-101",
      parameters: { volume: 35 },
      position: { x: 920, y: 220 },
    },
    {
      id: "eq-c201",
      definition_id: "standard.compressor.centrifugal",
      definition_version: "1.0.0",
      name: "C-201",
      parameters: { rated_power: 600 },
      position: { x: 1140, y: 220 },
    },
  ],
  streams: [
    {
      id: "st-01",
      name: "Feed Draw",
      stream_type: "material",
      source_equipment_id: "eq-tk101",
      source_port_id: "outlet",
      sink_equipment_id: "eq-p101",
      sink_port_id: "suction",
      composition: {},
      route_points: [{ x: 130, y: 220 }, { x: 260, y: 220 }],
    },
    {
      id: "st-02",
      name: "LP to HP Transfer",
      stream_type: "material",
      source_equipment_id: "eq-p101",
      source_port_id: "discharge",
      sink_equipment_id: "eq-c101",
      sink_port_id: "suction",
      composition: {},
      route_points: [{ x: 310, y: 220 }, { x: 480, y: 220 }],
    },
    {
      id: "st-03",
      name: "HP Discharge to Cooler",
      stream_type: "material",
      source_equipment_id: "eq-c101",
      source_port_id: "discharge",
      sink_equipment_id: "eq-e101",
      sink_port_id: "tube_inlet",
      composition: {},
      route_points: [{ x: 530, y: 220 }, { x: 700, y: 220 }],
    },
    {
      id: "st-04",
      name: "Cooler Outlet to Separator",
      stream_type: "material",
      source_equipment_id: "eq-e101",
      source_port_id: "tube_outlet",
      sink_equipment_id: "eq-v101",
      sink_port_id: "feed_inlet",
      composition: {},
      route_points: [{ x: 750, y: 220 }, { x: 920, y: 220 }],
    },
    {
      id: "st-05",
      name: "Separator Overhead to Booster",
      stream_type: "material",
      source_equipment_id: "eq-v101",
      source_port_id: "vapor_outlet",
      sink_equipment_id: "eq-c201",
      sink_port_id: "suction",
      composition: {},
      route_points: [{ x: 970, y: 220 }, { x: 1140, y: 220 }],
    },
  ],
  instruments: [],
  control_loops: [],
  alarms: [],
  interlocks: [],
  plant_structure: RECON_PLANT_STRUCTURE,
  displays: RECON_DISPLAYS,
  navigation_links: RECON_NAV_LINKS,
};

describe("Stage 14 Full Plant Reconstruction Behavioral Verification", () => {
  beforeEach(() => {
    store.setActiveProject(JSON.parse(JSON.stringify(RECON_PROJECT)));
    studioStore.setProject(JSON.parse(JSON.stringify(RECON_PROJECT)));
  });

  it("1. C-101 exists exactly once in the physical model", () => {
    const proj = studioStore.getProject();
    const c101Instances = proj?.equipment.filter((e) => e.id === "eq-c101");
    expect(c101Instances?.length).toBe(1);
  });

  it("2 & 3. C-101 belongs to HP System and P-101 belongs to LP System operationally", () => {
    const proj = studioStore.getProject();
    const lpUnit = proj?.plant_structure?.areas[0].systems[0].subsystems[0].units[0];
    const hpUnit = proj?.plant_structure?.areas[0].systems[1].subsystems[0].units[0];

    expect(lpUnit?.equipment_ids).toContain("eq-p101");
    expect(hpUnit?.equipment_ids).toContain("eq-c101");
  });

  it("4 & 5. Physical stream P-101 -> C-101 is singular and crosses the LP/HP operational boundary", () => {
    const proj = studioStore.getProject();
    const crossStreams = proj?.streams.filter(
      (s) => s.source_equipment_id === "eq-p101" && s.sink_equipment_id === "eq-c101"
    );
    expect(crossStreams?.length).toBe(1);
    expect(crossStreams![0].name).toBe("LP to HP Transfer");
  });

  it("6. C-101 appears on multiple displays simultaneously (LP, HP, Comp, Detail)", () => {
    const proj = studioStore.getProject();
    const displaysWithC101 = proj?.displays?.filter((d) => d.equipment_ids.includes("eq-c101"));
    const displayIds = displaysWithC101?.map((d) => d.id);

    expect(displayIds).toContain("disp-lp-overview");
    expect(displayIds).toContain("disp-hp-overview");
    expect(displayIds).toContain("disp-comp-overview");
    expect(displayIds).toContain("disp-c101-detail");
  });

  it("7. Every display observes the same runtime state for C-101", () => {
    // In store, runtime tags for C-101 are global and shared across any active display view
    store.applyDelta({
      seq: 1,
      sim_time_sec: 10.0,
      tick_count: 200,
      changed_tags: [
        { tag: "C-101.DISCH_PRESS", value: 38.5, quality: "GOOD", unit: "barg" },
      ],
    });

    store.navigateDisplay("disp-lp-overview");
    expect(store.tags.get("C-101.DISCH_PRESS")?.value).toBe(38.5);

    store.navigateDisplay("disp-hp-overview");
    expect(store.tags.get("C-101.DISCH_PRESS")?.value).toBe(38.5);

    store.navigateDisplay("disp-c101-detail");
    expect(store.tags.get("C-101.DISCH_PRESS")?.value).toBe(38.5);
  });

  it("8. C-101 physical position can move without breaking operational mappings or navigation links", () => {
    studioStore.moveEquipment("eq-c101", { x: 600, y: 350 });
    const updated = studioStore.getProject();
    const c101 = updated?.equipment.find((e) => e.id === "eq-c101");
    expect(c101?.position?.x).toBe(600);
    expect(c101?.position?.y).toBe(350);

    // Navigation link still references stable ID eq-c101
    const link = updated?.navigation_links?.find((l) => l.id === "nav-lp-to-hp");
    expect(link?.target_element_id).toBe("eq-c101");

    // Operational mapping still references eq-c101
    const hpUnit = updated?.plant_structure?.areas[0].systems[1].subsystems[0].units[0];
    expect(hpUnit?.equipment_ids).toContain("eq-c101");
  });

  it("9. Renaming HP Overview display does not break navigation links referencing it", () => {
    const proj = studioStore.getProject();
    const hpDisp = proj?.displays?.find((d) => d.id === "disp-hp-overview");
    if (hpDisp) {
      hpDisp.name = "High Pressure Compression & Treatment Center";
    }

    // Navigation link references stable ID disp-hp-overview, unaffected by name change
    const link = proj?.navigation_links?.find((l) => l.id === "nav-lp-to-hp");
    expect(link?.target_display_id).toBe("disp-hp-overview");
  });

  it("10. Renaming C-101 does not break physical topology streams", () => {
    studioStore.updateEquipment("eq-c101", { name: "C-101-REBUILT" });
    const updated = studioStore.getProject();
    const st2 = updated?.streams.find((s) => s.id === "st-02");
    expect(st2?.source_equipment_id).toBe("eq-p101");
    expect(st2?.sink_equipment_id).toBe("eq-c101");
  });

  it("11. Supports display-specific placement geometry for C-101 across different displays", () => {
    const proj = studioStore.getProject();
    const lpDisp = proj?.displays?.find((d) => d.id === "disp-lp-overview");
    const hpDisp = proj?.displays?.find((d) => d.id === "disp-hp-overview");

    const lpPlacement = lpDisp?.placements?.find((p) => p.equipment_id === "eq-c101");
    const hpPlacement = hpDisp?.placements?.find((p) => p.equipment_id === "eq-c101");

    expect(lpPlacement?.x).toBe(600);
    expect(lpPlacement?.y).toBe(250);

    expect(hpPlacement?.x).toBe(220);
    expect(hpPlacement?.y).toBe(300);

    // Both reference the singular physical equipment entity eq-c101
    expect(lpPlacement?.equipment_id).toBe(hpPlacement?.equipment_id);
  });

  it("12. Executes cross-display navigation LP -> HP and focuses C-101", () => {
    store.navigateDisplay("disp-lp-overview");
    expect(store.currentDisplay?.id).toBe("disp-lp-overview");

    const link = RECON_NAV_LINKS[0];
    store.executeNavigationLink(link);

    expect(store.currentDisplay?.id).toBe("disp-hp-overview");
    expect(store.focusedEquipmentId).toBe("eq-c101");
    expect(store.selectedEquipmentId).toBe("eq-c101");
  });

  it("13. Executes HP -> Separation Overview and focuses V-101", () => {
    store.navigateDisplay("disp-hp-overview");
    const link = RECON_NAV_LINKS[3]; // nav-e101-to-sep
    store.executeNavigationLink(link);

    expect(store.currentDisplay?.id).toBe("disp-sep-overview");
    expect(store.focusedEquipmentId).toBe("eq-v101");
    expect(store.selectedEquipmentId).toBe("eq-v101");
  });

  it("14. Seamlessly supports non-refinery custom equipment without schema alteration", () => {
    const customReactor = {
      id: "eq-custom-react-001",
      definition_id: "custom.biotech.fermenter_batch",
      definition_version: "2.1.0",
      name: "FERM-501",
      parameters: { agitation_rpm: 120, ph_sp: 6.8 },
      position: { x: 300, y: 300 },
    };

    studioStore.addEquipment(customReactor);
    const updated = studioStore.getProject();
    expect(updated?.equipment.some((e) => e.id === "eq-custom-react-001")).toBe(true);

    // Can be mapped into operational hierarchy and displays identically
    studioStore.setEquipmentOperationalMapping("eq-custom-react-001", "unit-v101");
    studioStore.setEquipmentDisplays("eq-custom-react-001", ["disp-sep-overview"]);

    const afterMap = studioStore.getProject();
    const sepDisp = afterMap?.displays?.find((d) => d.id === "disp-sep-overview");
    expect(sepDisp?.equipment_ids).toContain("eq-custom-react-001");
  });
});
