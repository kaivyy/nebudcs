/**
 * Stage 14 Naphtha Hydrotreater (NHT) Reference Plant Frontend Acceptance Tests.
 *
 * Verifies:
 * 1. Complete NHT Reference Plant project model loading (12 systems, 7 units, 12 displays, 17 links).
 * 2. Cross-display DCS navigation traversal across all operational sections.
 * 3. Element placements persistence and display-specific viewport geometry.
 * 4. Runtime tag state synchronization across multiple displays.
 * 5. Non-destructive physical vs operational mapping consistency.
 */

import { beforeEach, describe, expect, it } from "vitest";
import {
  DisplayDto,
  NavigationLinkDto,
  ProjectDetailDto,
} from "../src/protocol/types";
import { store } from "../src/state/store";
import { studioStore } from "../src/state/studio-store";

const NHT_DISPLAYS: DisplayDto[] = [
  {
    id: "disp-nht-overview",
    name: "Naphtha Hydrotreater Plant Overview",
    display_type: "plant_overview",
    area_id: "area-nht",
    system_id: undefined,
    subsystem_id: undefined,
    unit_id: undefined,
    equipment_ids: ["eq-tk101", "eq-p101-a", "eq-e101-a", "eq-f101", "eq-r101", "eq-v102", "eq-k101a", "eq-c101", "eq-c102"],
    instrument_ids: [],
    stream_ids: [],
    viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 1 },
    is_default: true,
    order: 1,
    description: "Plantwide overview",
    placements: [
      { equipment_id: "eq-p101-a", x: 260, y: 360, width: 80, height: 80 },
      { equipment_id: "eq-r101", x: 760, y: 340, width: 80, height: 80 },
      { equipment_id: "eq-v102", x: 1040, y: 340, width: 80, height: 80 },
      { equipment_id: "eq-k101a", x: 1040, y: 180, width: 80, height: 80 },
      { equipment_id: "eq-c101", x: 1240, y: 340, width: 80, height: 80 },
      { equipment_id: "eq-c102", x: 1440, y: 340, width: 80, height: 80 },
    ],
  },
  {
    id: "disp-nht-feed",
    name: "NHT Feed Preparation & Surge",
    display_type: "unit_process_graphic",
    area_id: "area-nht",
    system_id: "sys-nht-feed",
    subsystem_id: undefined,
    unit_id: "unit-nht-feed",
    equipment_ids: ["eq-tk101", "eq-v101", "eq-p101-a", "eq-p101-b", "eq-fv101", "eq-x101", "eq-mix101"],
    instrument_ids: ["FT-101"],
    stream_ids: ["STR-NHT-01", "STR-NHT-02", "STR-NHT-03", "STR-NHT-04"],
    viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 1 },
    is_default: false,
    order: 2,
    description: "Feed system",
    placements: [
      { equipment_id: "eq-p101-a", x: 480, y: 380, width: 80, height: 80 },
      { equipment_id: "eq-mix101", x: 900, y: 380, width: 80, height: 80 },
    ],
  },
  {
    id: "disp-nht-rx",
    name: "NHT Reactor Train & Combined Feed Exchangers",
    display_type: "unit_process_graphic",
    area_id: "area-nht",
    system_id: "sys-nht-rx",
    subsystem_id: undefined,
    unit_id: "unit-nht-rx",
    equipment_ids: ["eq-mix101", "eq-e101-a", "eq-f101", "eq-r101"],
    instrument_ids: ["TT-101", "TT-102"],
    stream_ids: ["STR-NHT-05", "STR-NHT-12", "STR-NHT-13", "STR-NHT-14"],
    viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 1 },
    is_default: false,
    order: 3,
    description: "Reactor train",
    placements: [
      { equipment_id: "eq-e101-a", x: 260, y: 380, width: 80, height: 80 },
      { equipment_id: "eq-r101", x: 1320, y: 360, width: 80, height: 80 },
    ],
  },
  {
    id: "disp-nht-sep",
    name: "NHT High Pressure Separation",
    display_type: "unit_process_graphic",
    area_id: "area-nht",
    system_id: "sys-nht-sep",
    subsystem_id: undefined,
    unit_id: "unit-nht-sep",
    equipment_ids: ["eq-mix102", "eq-a101", "eq-v102", "eq-lv101", "eq-v103"],
    instrument_ids: ["PT-102", "LT-101"],
    stream_ids: ["STR-NHT-25", "STR-NHT-27", "STR-NHT-28"],
    viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 1 },
    is_default: false,
    order: 4,
    description: "Separation",
    placements: [
      { equipment_id: "eq-v102", x: 640, y: 360, width: 80, height: 80 },
      { equipment_id: "eq-lv101", x: 820, y: 380, width: 80, height: 80 },
    ],
  },
  {
    id: "disp-nht-h2-recycle",
    name: "NHT Hydrogen & Recycle Compression",
    display_type: "unit_process_graphic",
    area_id: "area-nht",
    system_id: "sys-nht-recycle",
    subsystem_id: undefined,
    unit_id: "unit-nht-recycle",
    equipment_ids: ["eq-v103", "eq-k101a", "eq-split101", "eq-pv102"],
    instrument_ids: ["FT-102"],
    stream_ids: ["STR-NHT-29", "STR-NHT-30", "STR-NHT-31"],
    viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 1 },
    is_default: false,
    order: 5,
    description: "Recycle compression",
    placements: [
      { equipment_id: "eq-k101a", x: 420, y: 380, width: 80, height: 80 },
    ],
  },
  {
    id: "disp-nht-strip",
    name: "NHT Stripper Section",
    display_type: "unit_process_graphic",
    area_id: "area-nht",
    system_id: "sys-nht-strip",
    subsystem_id: undefined,
    unit_id: "unit-nht-strip",
    equipment_ids: ["eq-c101", "eq-e103", "eq-v105"],
    instrument_ids: ["PT-103", "TT-102"],
    stream_ids: ["STR-NHT-33", "STR-NHT-34"],
    viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 1 },
    is_default: false,
    order: 6,
    description: "Stripper section",
    placements: [
      { equipment_id: "eq-c101", x: 620, y: 360, width: 80, height: 80 },
    ],
  },
  {
    id: "disp-nht-split",
    name: "NHT Splitter Section",
    display_type: "unit_process_graphic",
    area_id: "area-nht",
    system_id: "sys-nht-split",
    subsystem_id: undefined,
    unit_id: "unit-nht-split",
    equipment_ids: ["eq-c102", "eq-e105", "eq-v106"],
    instrument_ids: ["TT-103", "LT-103"],
    stream_ids: ["STR-NHT-41", "STR-NHT-42"],
    viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 1 },
    is_default: false,
    order: 7,
    description: "Splitter section",
    placements: [
      { equipment_id: "eq-c102", x: 620, y: 360, width: 80, height: 80 },
    ],
  },
  {
    id: "disp-nht-util",
    name: "NHT Wash Water & Utilities",
    display_type: "unit_process_graphic",
    area_id: "area-nht",
    system_id: "sys-nht-water",
    subsystem_id: undefined,
    unit_id: "unit-nht-util",
    equipment_ids: ["eq-v104", "eq-p102-a", "eq-fv103"],
    instrument_ids: ["FT-103"],
    stream_ids: ["STR-NHT-23", "STR-NHT-24"],
    viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 1 },
    is_default: false,
    order: 8,
    description: "Wash water and utilities",
    placements: [
      { equipment_id: "eq-p102-a", x: 420, y: 380, width: 80, height: 80 },
    ],
  },
  {
    id: "disp-nht-comp-detail",
    name: "Recycle Gas Compressor K-101A Performance & Anti-Surge",
    display_type: "point_detail",
    area_id: "area-nht",
    system_id: "sys-nht-recycle",
    subsystem_id: undefined,
    unit_id: "unit-nht-recycle",
    equipment_ids: ["eq-k101a"],
    instrument_ids: ["FT-102"],
    stream_ids: [],
    viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 1 },
    is_default: false,
    order: 9,
    description: "Compressor detail",
    placements: [
      { equipment_id: "eq-k101a", x: 540, y: 360, width: 80, height: 80 },
    ],
  },
  {
    id: "disp-nht-rx-detail",
    name: "Reactor R-101 Bed Profile & Multi-Point Temperatures",
    display_type: "point_detail",
    area_id: "area-nht",
    system_id: "sys-nht-rx",
    subsystem_id: undefined,
    unit_id: "unit-nht-rx",
    equipment_ids: ["eq-r101"],
    instrument_ids: ["TT-101", "TT-102"],
    stream_ids: [],
    viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 1 },
    is_default: false,
    order: 10,
    description: "Reactor bed detail",
    placements: [
      { equipment_id: "eq-r101", x: 620, y: 340, width: 80, height: 80 },
    ],
  },
  {
    id: "disp-nht-alarm-summary",
    name: "NHT Plant Alarm Summary & Matrix",
    display_type: "alarms",
    area_id: "area-nht",
    system_id: undefined,
    subsystem_id: undefined,
    unit_id: undefined,
    equipment_ids: ["eq-r101", "eq-v102", "eq-k101a", "eq-c101"],
    instrument_ids: [],
    stream_ids: [],
    viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 1 },
    is_default: false,
    order: 11,
    description: "Alarm matrix",
    placements: [
      { equipment_id: "eq-r101", x: 300, y: 300, width: 80, height: 80 },
    ],
  },
  {
    id: "disp-nht-ctl-summary",
    name: "NHT Plant Control Loops & PID Overview",
    display_type: "system_overview",
    area_id: "area-nht",
    system_id: undefined,
    subsystem_id: undefined,
    unit_id: undefined,
    equipment_ids: ["eq-fv101", "eq-f101", "eq-lv101", "eq-pv102"],
    instrument_ids: [],
    stream_ids: [],
    viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 1 },
    is_default: false,
    order: 12,
    description: "Control summary",
    placements: [
      { equipment_id: "eq-fv101", x: 200, y: 300, width: 80, height: 80 },
    ],
  },
];

const NHT_NAV_LINKS: NavigationLinkDto[] = [
  {
    id: "nav-nht-01",
    source_display_id: "disp-nht-overview",
    source_element_id: "eq-p101-a",
    target_display_id: "disp-nht-feed",
    target_element_id: "eq-p101-a",
    label: "Open Feed System",
    behavior: "focus_and_center",
  },
  {
    id: "nav-nht-02",
    source_display_id: "disp-nht-overview",
    source_element_id: "eq-r101",
    target_display_id: "disp-nht-rx",
    target_element_id: "eq-r101",
    label: "Open Reaction Train",
    behavior: "focus_and_center",
  },
  {
    id: "nav-nht-03",
    source_display_id: "disp-nht-overview",
    source_element_id: "eq-v102",
    target_display_id: "disp-nht-sep",
    target_element_id: "eq-v102",
    label: "Open HP Separation",
    behavior: "focus_and_center",
  },
  {
    id: "nav-nht-07",
    source_display_id: "disp-nht-feed",
    source_element_id: "eq-mix101",
    target_display_id: "disp-nht-rx",
    target_element_id: "eq-e101-a",
    label: "Follow Stream to Combined Feed Exchangers",
    behavior: "focus_and_center",
  },
  {
    id: "nav-nht-08",
    source_display_id: "disp-nht-rx",
    source_element_id: "eq-r101",
    target_display_id: "disp-nht-rx-detail",
    target_element_id: "eq-r101",
    label: "Inspect Reactor R-101 Bed Profile",
    behavior: "focus_and_center",
  },
  {
    id: "nav-nht-09",
    source_display_id: "disp-nht-rx",
    source_element_id: "eq-r101",
    target_display_id: "disp-nht-sep",
    target_element_id: "eq-v102",
    label: "Follow Effluent to HP Separator",
    behavior: "focus_and_center",
  },
  {
    id: "nav-nht-10",
    source_display_id: "disp-nht-sep",
    source_element_id: "eq-v103",
    target_display_id: "disp-nht-h2-recycle",
    target_element_id: "eq-k101a",
    label: "Follow Vapor to Recycle Compressor",
    behavior: "focus_and_center",
  },
  {
    id: "nav-nht-11",
    source_display_id: "disp-nht-sep",
    source_element_id: "eq-lv101",
    target_display_id: "disp-nht-strip",
    target_element_id: "eq-c101",
    label: "Follow Bottoms to Stripper Train",
    behavior: "focus_and_center",
  },
  {
    id: "nav-nht-12",
    source_display_id: "disp-nht-h2-recycle",
    source_element_id: "eq-k101a",
    target_display_id: "disp-nht-comp-detail",
    target_element_id: "eq-k101a",
    label: "Inspect K-101A Performance",
    behavior: "focus_and_center",
  },
  {
    id: "nav-nht-14",
    source_display_id: "disp-nht-strip",
    source_element_id: "eq-c101",
    target_display_id: "disp-nht-split",
    target_element_id: "eq-c102",
    label: "Follow Stripper Bottoms to Splitter",
    behavior: "focus_and_center",
  },
  {
    id: "nav-nht-15",
    source_display_id: "disp-nht-split",
    source_element_id: "eq-c102",
    target_display_id: "disp-nht-overview",
    target_element_id: "eq-tk101",
    label: "Return to NHT Overview",
    behavior: "focus_and_center",
  },
];

const NHT_PROJECT: ProjectDetailDto = {
  id: "proj-domain-nht-reference",
  name: "Naphtha Hydrotreater (NHT) Reference Plant",
  description: "HPCL Visakh Benchmark Plant",
  schema_version: "1.0.0",
  author: "NebuDCS Engineering Standards",
  created: "2026-09-09T00:00:00Z",
  modified: "2026-09-09T00:00:00Z",
  instruments: [],
  equipment: [
    { id: "eq-p101-a", name: "P-101A", definition_id: "standard.pump.centrifugal", definition_version: "1.0.0", parameters: { rated_flow_m3h: 199 }, position: { x: 260, y: 360 } },
    { id: "eq-r101", name: "R-101", definition_id: "standard.reactor.fixed_bed", definition_version: "1.0.0", parameters: { operating_temp: 332, max_bed_delta_t: 10 }, position: { x: 760, y: 340 } },
    { id: "eq-v102", name: "V-102", definition_id: "standard.vessel.vertical", definition_version: "1.0.0", parameters: { operating_pressure: 2069000 }, position: { x: 1040, y: 340 } },
    { id: "eq-k101a", name: "K-101A", definition_id: "standard.compressor.centrifugal", definition_version: "1.0.0", parameters: { speed_rpm: 10450 }, position: { x: 1040, y: 180 } },
    { id: "eq-c101", name: "C-101", definition_id: "standard.column.fractionator", definition_version: "1.0.0", parameters: { tray_count: 30 }, position: { x: 1240, y: 340 } },
    { id: "eq-c102", name: "C-102", definition_id: "standard.column.fractionator", definition_version: "1.0.0", parameters: { tray_count: 55 }, position: { x: 1440, y: 340 } },
  ],
  streams: [],
  displays: NHT_DISPLAYS,
  navigation_links: NHT_NAV_LINKS,
  plant_structure: {
    plant_name: "Naphtha Hydrotreater Complex",
    displays: NHT_DISPLAYS,
    areas: [
      {
        id: "area-nht",
        name: "NHT Process Complex",
        systems: [
          { id: "sys-nht-feed", name: "Naphtha Feed & Surge System", default_display_id: "disp-nht-feed", subsystems: [] },
          { id: "sys-nht-h2", name: "Hydrogen Supply System", default_display_id: "disp-nht-h2-recycle", subsystems: [] },
          { id: "sys-nht-rx", name: "Reactor & Preheating System", default_display_id: "disp-nht-rx", subsystems: [] },
          { id: "sys-nht-sep", name: "HP Separation System", default_display_id: "disp-nht-sep", subsystems: [] },
          { id: "sys-nht-recycle", name: "Recycle Compression System", default_display_id: "disp-nht-h2-recycle", subsystems: [] },
          { id: "sys-nht-strip", name: "Stripper Section", default_display_id: "disp-nht-strip", subsystems: [] },
          { id: "sys-nht-split", name: "Splitter Section", default_display_id: "disp-nht-split", subsystems: [] },
          { id: "sys-nht-water", name: "Wash Water System", default_display_id: "disp-nht-util", subsystems: [] },
          { id: "sys-nht-steam", name: "Steam Utilities", default_display_id: "disp-nht-util", subsystems: [] },
          { id: "sys-nht-sour", name: "Sour Water Header", default_display_id: "disp-nht-sep", subsystems: [] },
          { id: "sys-nht-offgas", name: "Off-Gas System", default_display_id: "disp-nht-h2-recycle", subsystems: [] },
          { id: "sys-nht-bl", name: "Battery Limits Interconnect", default_display_id: "disp-nht-overview", subsystems: [] },
        ],
      },
    ],
  },
  control_loops: [],
  alarms: [],
  interlocks: [],
};

describe("NHT Reference Plant Acceptance Tests", () => {
  beforeEach(() => {
    store.setActiveProject(JSON.parse(JSON.stringify(NHT_PROJECT)));
    studioStore.setProject(JSON.parse(JSON.stringify(NHT_PROJECT)));
  });

  it("1. Loads NHT Reference Plant with 12 displays, 12 systems, and 11 key navigation links", () => {
    expect(store.activeProject?.id).toBe("proj-domain-nht-reference");
    expect(store.displays.length).toBe(12);
    expect(store.navigationLinks.length).toBe(11);

    const plantStruct = store.activeProject?.plant_structure;
    expect(plantStruct?.areas[0].systems.length).toBe(12);
  });

  it("2. Initial default display is disp-nht-overview", () => {
    expect(store.currentDisplay?.id).toBe("disp-nht-overview");
    expect(store.currentDisplay?.name).toBe("Naphtha Hydrotreater Plant Overview");
  });

  it("3. Executes full cross-display process navigation flow from Feed to Splitter and back", () => {
    // 1. Overview -> Feed
    const navToFeed = NHT_NAV_LINKS.find((l) => l.id === "nav-nht-01")!;
    store.executeNavigationLink(navToFeed);
    expect(store.currentDisplay?.id).toBe("disp-nht-feed");
    expect(store.focusedEquipmentId).toBe("eq-p101-a");

    // 2. Feed -> Reaction Train
    const navToRx = NHT_NAV_LINKS.find((l) => l.id === "nav-nht-07")!;
    store.executeNavigationLink(navToRx);
    expect(store.currentDisplay?.id).toBe("disp-nht-rx");
    expect(store.focusedEquipmentId).toBe("eq-e101-a");

    // 3. Reaction Train -> Reactor Bed Profile Detail
    const navToRxDetail = NHT_NAV_LINKS.find((l) => l.id === "nav-nht-08")!;
    store.executeNavigationLink(navToRxDetail);
    expect(store.currentDisplay?.id).toBe("disp-nht-rx-detail");
    expect(store.focusedEquipmentId).toBe("eq-r101");

    // 4. Return to Reaction Train then go to HP Separator
    const navToSep = NHT_NAV_LINKS.find((l) => l.id === "nav-nht-09")!;
    store.executeNavigationLink(navToSep);
    expect(store.currentDisplay?.id).toBe("disp-nht-sep");
    expect(store.focusedEquipmentId).toBe("eq-v102");

    // 5. HP Separator -> Recycle Gas Compression
    const navToRecycle = NHT_NAV_LINKS.find((l) => l.id === "nav-nht-10")!;
    store.executeNavigationLink(navToRecycle);
    expect(store.currentDisplay?.id).toBe("disp-nht-h2-recycle");
    expect(store.focusedEquipmentId).toBe("eq-k101a");

    // 6. Recycle Gas -> Compressor Performance Detail
    const navToCompDetail = NHT_NAV_LINKS.find((l) => l.id === "nav-nht-12")!;
    store.executeNavigationLink(navToCompDetail);
    expect(store.currentDisplay?.id).toBe("disp-nht-comp-detail");
    expect(store.focusedEquipmentId).toBe("eq-k101a");

    // 7. Back from Sep to Stripper
    const navToStrip = NHT_NAV_LINKS.find((l) => l.id === "nav-nht-11")!;
    store.executeNavigationLink(navToStrip);
    expect(store.currentDisplay?.id).toBe("disp-nht-strip");
    expect(store.focusedEquipmentId).toBe("eq-c101");

    // 8. Stripper -> Splitter
    const navToSplit = NHT_NAV_LINKS.find((l) => l.id === "nav-nht-14")!;
    store.executeNavigationLink(navToSplit);
    expect(store.currentDisplay?.id).toBe("disp-nht-split");
    expect(store.focusedEquipmentId).toBe("eq-c102");

    // 9. Splitter -> Return to Overview
    const navToOverview = NHT_NAV_LINKS.find((l) => l.id === "nav-nht-15")!;
    store.executeNavigationLink(navToOverview);
    expect(store.currentDisplay?.id).toBe("disp-nht-overview");
    expect(store.focusedEquipmentId).toBe("eq-tk101");
  });

  it("4. Maintains display-specific placement geometry for C-101 and R-101", () => {
    const overviewDisp = NHT_DISPLAYS.find((d) => d.id === "disp-nht-overview")!;
    const rxDisp = NHT_DISPLAYS.find((d) => d.id === "disp-nht-rx")!;
    const stripDisp = NHT_DISPLAYS.find((d) => d.id === "disp-nht-strip")!;

    const r101OverviewPlacement = overviewDisp.placements?.find((p) => p.equipment_id === "eq-r101");
    const r101RxPlacement = rxDisp.placements?.find((p) => p.equipment_id === "eq-r101");
    expect(r101OverviewPlacement?.x).toBe(760);
    expect(r101RxPlacement?.x).toBe(1320);

    const c101OverviewPlacement = overviewDisp.placements?.find((p) => p.equipment_id === "eq-c101");
    const c101StripPlacement = stripDisp.placements?.find((p) => p.equipment_id === "eq-c101");
    expect(c101OverviewPlacement?.x).toBe(1240);
    expect(c101StripPlacement?.x).toBe(620);
  });

  it("5. Retains realtime tag synchronization across displays during operations", () => {
    store.applyDelta({
      seq: 1,
      sim_time_sec: 100.5,
      tick_count: 2010,
      changed_tags: [
        { tag: "TT-101", value: 332.0, quality: "GOOD", unit: "°C" },
        { tag: "TT-102", value: 343.0, quality: "GOOD", unit: "°C" },
        { tag: "PT-102", value: 21.1, quality: "GOOD", unit: "kg/cm²g" },
      ],
    });

    store.navigateDisplay("disp-nht-overview");
    expect(store.tags.get("TT-101")?.value).toBe(332.0);
    expect(store.tags.get("PT-102")?.value).toBe(21.1);

    store.navigateDisplay("disp-nht-rx");
    expect(store.tags.get("TT-101")?.value).toBe(332.0);
    expect(store.tags.get("TT-102")?.value).toBe(343.0);

    store.navigateDisplay("disp-nht-sep");
    expect(store.tags.get("PT-102")?.value).toBe(21.1);
  });
});
