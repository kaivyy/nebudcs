import { describe, it, expect } from "vitest";
import { Store } from "../src/state/store";
import { ProjectDetailDto } from "../src/protocol/types";

// Minimal projects
const PROJECT_A: ProjectDetailDto = {
  id: "proj-a",
  name: "Project A",
  schema_version: "1.0.0",
  description: "Test Project A",
  author: "Author A",
  created: "2026-01-01T00:00:00Z",
  modified: "2026-01-01T00:00:00Z",
  equipment: [],
  streams: [],
  instruments: [],
  control_loops: [
    {
      id: "cl-a1",
      name: "FIC-STEAM",
      pv_tag: "FIC-STEAM.PV",
      sp: 100,
      out_tag: "FIC-STEAM.OUT",
      kp: 1,
      ti: 10,
      td: 0,
      reverse_acting: false,
      mode: "AUTO",
      output_min: 0,
      output_max: 100,
    },
  ],
  alarms: [
    {
      id: "a1",
      name: "FAH-STEAM",
      pv_tag: "FIC-STEAM.PV",
      alarm_type: "Hi",
      severity: "Warning",
      trip_limit: 150,
      deadband: 5,
      description: "High flow",
    },
  ],
  interlocks: [],
};

const PROJECT_B: ProjectDetailDto = {
  id: "proj-b",
  name: "Project B",
  schema_version: "1.0.0",
  description: "Test Project B",
  author: "Author B",
  created: "2026-01-01T00:00:00Z",
  modified: "2026-01-01T00:00:00Z",
  equipment: [],
  streams: [],
  instruments: [],
  control_loops: [
    {
      id: "cl-b1",
      name: "PIC-DRUM",
      pv_tag: "PIC-DRUM.PV",
      sp: 15,
      out_tag: "PIC-DRUM.OUT",
      kp: 0.5,
      ti: 8,
      td: 0,
      reverse_acting: false,
      mode: "AUTO",
      output_min: 0,
      output_max: 100,
    },
  ],
  alarms: [
    {
      id: "b1",
      name: "PAH-DRUM",
      pv_tag: "PIC-DRUM.PV",
      alarm_type: "Hi",
      severity: "Critical",
      trip_limit: 25,
      deadband: 1,
      description: "Drum overpressure",
    },
  ],
  interlocks: [],
};

describe("State Isolation — Project Switching", () => {
  it("switching from Project A to Project B clears old faceplate", () => {
    const s = new Store();
    s.setActiveProject(PROJECT_A);
    expect(s.selectedFaceplate?.id).toBe("FIC-STEAM");

    s.setActiveProject(PROJECT_B);
    expect(s.selectedFaceplate?.id).toBe("PIC-DRUM");
    expect(s.selectedFaceplate?.id).not.toBe("FIC-STEAM");
  });

  it("selectedTrendTags update when project changes", () => {
    const s = new Store();
    s.setActiveProject(PROJECT_A);
    expect(s.selectedTrendTags.some((t) => t.includes("FIC-STEAM"))).toBe(true);

    s.setActiveProject(PROJECT_B);
    expect(s.selectedTrendTags.some((t) => t.includes("FIC-STEAM"))).toBe(false);
    expect(s.selectedTrendTags.some((t) => t.includes("PIC-DRUM"))).toBe(true);
  });

  it("alarm navigation resolves to Project B entities after switch", () => {
    const s = new Store();
    s.setActiveProject(PROJECT_A);
    expect(s.getEntityForAlarm("PAH-DRUM")).toBeNull(); // not in Project A

    s.setActiveProject(PROJECT_B);
    const target = s.getEntityForAlarm("PAH-DRUM");
    expect(target?.id).toBe("PIC-DRUM");
  });

  it("K-101/LIC-101 are not required by any generic system code", () => {
    const s = new Store();
    // Neither initial state nor project B has K-101/LIC-101
    expect(s.selectedFaceplate).toBeNull();
    s.setActiveProject(PROJECT_B);
    expect(JSON.stringify(s)).not.toContain("K-101");
    expect(JSON.stringify(s)).not.toContain("LIC-101");
  });

  it("setting project to null clears faceplate and trend tags", () => {
    const s = new Store();
    s.setActiveProject(PROJECT_A);
    s.setActiveProject(null);
    expect(s.selectedFaceplate).toBeNull();
    expect(s.selectedTrendTags).toHaveLength(0);
  });
});
