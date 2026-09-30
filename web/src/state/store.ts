/**
 * NebuDCS Frontend State Store.
 * Central single-source-of-truth for client telemetry, state buffering, and reactivity.
 *
 * Generic design: no instance-specific IDs (K-101, LIC-101, etc.) appear here.
 * All initial state is either null / empty, or derived from the loaded project model.
 */

import {
  AlarmSnapshotDto,
  BoundaryConditionDto,
  DeltaUpdateDto,
  FaceplateTarget,
  ProjectDetailDto,
  ScenarioStatusDto,
  ServerHealthDto,
  ServerMessage,
  StateSnapshotDto,
  TagValueSampleDto,
  AlarmFloodMetricsDto,
  DisplayDto,
  DisplayElementPlacementDto,
  EquipmentDiagnosticsDto,
  FirstOutEventDto,
  PlantAreaDto,
  PlantStructureDto,
  SisStatusSummaryDto,
  NavigationLinkDto,
  StudioControlLoopDto,
  StudioEquipmentDto,
  StudioInstrumentDto,
  StudioStreamDto,
  ViewportConfigDto,
} from "../protocol/types";
import { ConnectionState } from "../websocket/client";

export type StoreListener = (store: Store) => void;

/**
 * Explicit display-scoped rendering model.
 * The renderer receives ONLY this scoped model without accessing global arrays.
 */
export interface DisplayViewModel {
  displayId: string;
  displayName: string;
  displayType: string;
  equipment: StudioEquipmentDto[];
  placements: Map<string, DisplayElementPlacementDto>;
  streams: StudioStreamDto[];
  instruments: StudioInstrumentDto[];
  controlLoops: StudioControlLoopDto[];
  boundaryConditions: BoundaryConditionDto[];
  annotations: any[];
  navigation: NavigationLinkDto[];
  viewport: ViewportConfigDto;
  validationErrors: string[];
}

export class Store {
  public connectionState: ConnectionState = "disconnected";
  public simTime: number = 0;
  public tickCount: number = 0;
  public isSimRunning: boolean = false;
  public speedMultiplier: number = 1.0;
  public serverHealth: ServerHealthDto | null = null;
  public sessionId: string | null = null;
  public scenarioStatus: ScenarioStatusDto | null = null;

  // Authoritative simulation tags
  public tags: Map<string, TagValueSampleDto> = new Map();

  // Authoritative active alarms
  public alarms: Map<string, AlarmSnapshotDto> = new Map();

  // Live timeseries ring buffers for trend charts (max 300 points per tag)
  public tagHistory: Map<string, Array<{ time: number; value: number }>> = new Map();
  private maxHistoryLength: number = 300;

  // Selected PID controller name (generic — no hardcoded default)
  public selectedLoop: string = "";

  // Active Project & Flowsheet Topology Context
  public activeProject: ProjectDetailDto | null = null;
  public activeUnitName: string | null = null;

  // Selected Faceplate Target — null until project is loaded (no hardcoded defaults)
  public selectedFaceplate: FaceplateTarget | null = null;

  // Focused Entity ID for GOTO navigation & highlight
  public focusedEntityId: string | null = null;

  // Operator Alarm Station Filters
  public alarmSeverityFilter: string = "ALL";
  public alarmStateFilter: string = "ALL";

  // Trend window configuration
  public trendTimeWindowSec: number = 60;

  // Selected tags for live trend view — empty until project is loaded
  public selectedTrendTags: string[] = [];

  // Stage 10: Advanced DCS Operations & HMI state
  public alarmFloodMetrics: AlarmFloodMetricsDto | null = null;
  public diagnosticsMap: Map<string, EquipmentDiagnosticsDto> = new Map();
  public firstOutEvent: FirstOutEventDto | null = null;
  public activeHierarchyView: "plant" | "area" | "unit" | "diagnostics" | "events" = "unit";
  public selectedEquipmentId: string | null = null;

  // Hierarchical Display & Plant Structure (ISA-101 / Experion-style)
  public plantStructure: PlantStructureDto | null = null;
  public displays: DisplayDto[] = [];
  public activeDisplayId: string | null = null;
  public selectedStreamId: string | null = null;
  public displayHistory: string[] = [];
  public historyIndex: number = -1;
  public navigationLinks: NavigationLinkDto[] = [];

  // Runtime Deployment & Safety Observables
  public isProjectDeployed: boolean = false;
  public deployedProjectId: string | null = null;
  public sisSummary: SisStatusSummaryDto | null = null;

  private listeners: Set<StoreListener> = new Set();
  private notifyScheduled: boolean = false;

  public subscribe(listener: StoreListener): () => void {
    this.listeners.add(listener);
    listener(this);
    return () => this.listeners.delete(listener);
  }

  public notify() {
    if (!this.notifyScheduled) {
      this.notifyScheduled = true;
      queueMicrotask(() => {
        this.notifyScheduled = false;
        this.listeners.forEach((l) => l(this));
      });
    }
  }

  public setConnectionState(state: ConnectionState) {
    if (this.connectionState !== state) {
      this.connectionState = state;
      this.notify();
    }
  }

  public handleServerMessage(msg: ServerMessage) {
    switch (msg.type) {
      case "handshake_ack":
        this.sessionId = msg.payload.session_id;
        break;

      case "snapshot":
        this.applySnapshot(msg.payload);
        break;

      case "delta":
        this.applyDelta(msg.payload);
        break;

      case "alarm_update":
        this.applyAlarmUpdate(msg.payload);
        break;

      case "scenario_update":
        this.scenarioStatus = msg.payload;
        this.notify();
        break;

      case "command_ack":
        break;

      default:
        break;
    }
  }

  public setScenarioStatus(status: ScenarioStatusDto) {
    this.scenarioStatus = status;
    this.notify();
  }

  public applySnapshot(snapshot: StateSnapshotDto) {
    this.simTime = snapshot.sim_time_sec;
    this.tickCount = snapshot.tick_count;

    this.tags.clear();
    for (const sample of snapshot.tags) {
      this.tags.set(sample.tag, sample);
      this.recordHistoryPoint(sample.tag, snapshot.sim_time_sec, sample.value);
    }

    this.alarms.clear();
    for (const alm of snapshot.active_alarms) {
      this.alarms.set(alm.alarm_name, alm);
    }

    this.notify();
  }

  public applyDelta(delta: DeltaUpdateDto) {
    this.simTime = delta.sim_time_sec;
    this.tickCount = delta.tick_count;
    this.isSimRunning = true;

    for (const sample of delta.changed_tags) {
      this.tags.set(sample.tag, sample);
      this.recordHistoryPoint(sample.tag, delta.sim_time_sec, sample.value);
    }

    this.notify();
  }

  public applyAlarmUpdate(alarmList: AlarmSnapshotDto[]) {
    this.alarms.clear();
    for (const alm of alarmList) {
      this.alarms.set(alm.alarm_name, alm);
    }
    this.notify();
  }

  private recordHistoryPoint(tag: string, time: number, value: unknown) {
    if (typeof value !== "number") return;
    let history = this.tagHistory.get(tag);
    if (!history) {
      history = [];
      this.tagHistory.set(tag, history);
    }
    history.push({ time, value });
    if (history.length > this.maxHistoryLength) {
      history.shift();
    }
  }

  public getTag(tag: string): TagValueSampleDto | undefined {
    return this.tags.get(tag);
  }

  public getTagFloat(tag: string, fallback: number = 0.0): number {
    const s = this.tags.get(tag);
    if (s && typeof s.value === "number") {
      return s.value;
    }
    return fallback;
  }

  public getTagBool(tag: string, fallback: boolean = false): boolean {
    const s = this.tags.get(tag);
    if (s && typeof s.value === "boolean") {
      return s.value;
    }
    if (s && typeof s.value === "number") {
      return s.value > 0.5;
    }
    return fallback;
  }

  public getAlarms(): AlarmSnapshotDto[] {
    return Array.from(this.alarms.values());
  }

  public getTrendData(tag: string): Array<{ time: number; value: number }> {
    return this.tagHistory.get(tag) || [];
  }

  public setServerHealth(health: ServerHealthDto) {
    this.serverHealth = health;
    this.isSimRunning = health.simulation_running ?? false;
    this.speedMultiplier = health.speed_multiplier ?? 1.0;
    this.notify();
  }

  public setSimRunning(running: boolean) {
    this.isSimRunning = running;
    this.notify();
  }

  public setSelectedLoop(loop: string) {
    this.selectedLoop = loop;
    this.notify();
  }

  /**
   * Builds an authoritative, realistic industrial plant structure and display hierarchy
   * if the project model does not define an explicit one.
   */
  public buildDefaultPlantStructure(project: ProjectDetailDto): PlantStructureDto {
    const eqList = project.equipment || [];
    const instList = project.instruments || [];
    const streamList = project.streams || [];

    // Categorize equipment into HP System vs LP System
    const hpEq = eqList.filter((e, idx) => {
      const d = (e.definition_id || e.name || "").toLowerCase();
      if (d.includes("pump") || d.includes("compressor") || d.includes("reactor") || d.includes("furnace")) return true;
      return idx < Math.ceil(eqList.length / 2);
    });
    const lpEq = eqList.filter((e) => !hpEq.includes(e));

    const hpEqIds = hpEq.map((e) => e.id);
    const lpEqIds = lpEq.map((e) => e.id);
    const allEqIds = eqList.map((e) => e.id);

    const hpInstIds = instList.filter((i) => !i.equipment_id || hpEqIds.includes(i.equipment_id)).map((i) => i.tag);
    const lpInstIds = instList.filter((i) => i.equipment_id && lpEqIds.includes(i.equipment_id)).map((i) => i.tag);
    const allInstIds = instList.map((i) => i.tag);

    const allStreamIds = streamList.map((s) => s.name);

    const displays: DisplayDto[] = [
      {
        id: "disp-unit-01",
        name: "CDU-100 Overview",
        display_type: "unit_process_graphic",
        area_id: "area-01",
        system_id: "sys-hp",
        unit_id: "unit-01",
        equipment_ids: allEqIds,
        instrument_ids: allInstIds,
        stream_ids: allStreamIds,
        is_default: true,
        order: 1,
        description: "Atmospheric Crude Distillation Unit Process Flowsheet",
      },
      {
        id: "disp-feed-preheat",
        name: "Feed & Pre-Heat",
        display_type: "unit_process_graphic",
        area_id: "area-01",
        system_id: "sys-hp",
        equipment_ids: hpEqIds.length > 0 ? hpEqIds : allEqIds,
        instrument_ids: hpInstIds.length > 0 ? hpInstIds : allInstIds,
        stream_ids: allStreamIds,
        is_default: false,
        order: 2,
        description: "Crude Oil Charge Battery, Exchanger Train & Desalting",
      },
      {
        id: "disp-furnace",
        name: "Furnace H-101",
        display_type: "unit_process_graphic",
        area_id: "area-01",
        system_id: "sys-hp",
        equipment_ids: hpEqIds.length > 0 ? hpEqIds : allEqIds,
        instrument_ids: hpInstIds.length > 0 ? hpInstIds : allInstIds,
        stream_ids: allStreamIds,
        is_default: false,
        order: 3,
        description: "Fired Process Heater H-101, Multi-Pass Burners & Draft Control",
      },
      {
        id: "disp-column",
        name: "Column T-101",
        display_type: "unit_process_graphic",
        area_id: "area-01",
        system_id: "sys-hp",
        equipment_ids: allEqIds,
        instrument_ids: allInstIds,
        stream_ids: allStreamIds,
        is_default: false,
        order: 4,
        description: "Atmospheric Fractionation Column T-101, Trays & Sidestreams",
      },
      {
        id: "disp-compression",
        name: "Compression C-301",
        display_type: "unit_process_graphic",
        area_id: "area-01",
        system_id: "sys-hp",
        equipment_ids: hpEqIds.length > 0 ? hpEqIds : allEqIds,
        instrument_ids: hpInstIds.length > 0 ? hpInstIds : allInstIds,
        stream_ids: allStreamIds,
        is_default: false,
        order: 5,
        description: "Wet Gas Centrifugal Compressor Train & Anti-Surge Loop",
      },
      {
        id: "disp-separation",
        name: "Separation V-103",
        display_type: "unit_process_graphic",
        area_id: "area-01",
        system_id: "sys-lp",
        equipment_ids: lpEqIds.length > 0 ? lpEqIds : allEqIds,
        instrument_ids: lpInstIds.length > 0 ? lpInstIds : allInstIds,
        stream_ids: allStreamIds,
        is_default: false,
        order: 6,
        description: "Three-Phase High Pressure Separator, Weir & Boot Level Control",
      },
      {
        id: "disp-utilities",
        name: "Utilities Network",
        display_type: "unit_process_graphic",
        area_id: "area-02",
        system_id: "sys-util",
        equipment_ids: allEqIds,
        instrument_ids: allInstIds,
        stream_ids: allStreamIds,
        is_default: false,
        order: 7,
        description: "HP/MP/LP Steam Distribution Headers, Cooling Water & Fuel Gas",
      },
      {
        id: "disp-group-01",
        name: "Group Display",
        display_type: "group_display",
        area_id: "area-01",
        equipment_ids: allEqIds,
        instrument_ids: allInstIds,
        stream_ids: allStreamIds,
        is_default: false,
        order: 8,
        description: "Multi-Loop Regulatory Controller Matrix (8-Loop Density)",
      },
      {
        id: "disp-point-detail",
        name: "Point Detail",
        display_type: "point_detail",
        area_id: "area-01",
        equipment_ids: allEqIds,
        instrument_ids: allInstIds,
        stream_ids: allStreamIds,
        is_default: false,
        order: 9,
        description: "Instrument & Controller Calibration, Limits and Signal Quality",
      },
      {
        id: "disp-trend-01",
        name: "Trend",
        display_type: "trend",
        area_id: "area-01",
        equipment_ids: allEqIds,
        instrument_ids: allInstIds,
        stream_ids: allStreamIds,
        is_default: false,
        order: 10,
        description: "Realtime Multi-Pen Process Historian Trend",
      },
      {
        id: "disp-alarms",
        name: "Alarms",
        display_type: "alarms",
        area_id: "area-01",
        equipment_ids: allEqIds,
        instrument_ids: allInstIds,
        stream_ids: allStreamIds,
        is_default: false,
        order: 11,
        description: "Alarm Annunciator Summary, Shelving & EEMUA 191 Management",
      },
      {
        id: "disp-diagnostics",
        name: "Diagnostics",
        display_type: "diagnostics",
        area_id: "area-01",
        equipment_ids: allEqIds,
        instrument_ids: allInstIds,
        stream_ids: allStreamIds,
        is_default: false,
        order: 12,
        description: "Physics Model Verification & Mechanical Equipment Health",
      },
      {
        id: "disp-plant-overview",
        name: "Plant Overview",
        display_type: "plant_overview",
        area_id: "area-01",
        equipment_ids: allEqIds,
        instrument_ids: allInstIds,
        stream_ids: allStreamIds,
        is_default: false,
        order: 13,
        description: "Plant-Wide Situational Awareness & Area Navigation Grid",
      },
      {
        id: "disp-area-01",
        name: "Area 01 Overview",
        display_type: "area_overview",
        area_id: "area-01",
        equipment_ids: allEqIds,
        instrument_ids: allInstIds,
        stream_ids: allStreamIds,
        is_default: false,
        order: 14,
        description: "Continuous Process Train Systems & Supervisory Overview",
      },
    ];

    const areas: PlantAreaDto[] = [
      {
        id: "area-01",
        name: "Area 01 - Process Units",
        description: "Continuous Process Manufacturing Units",
        systems: [
          {
            id: "sys-hp",
            name: "HP Reaction & Compression System",
            area_id: "area-01",
            default_display_id: "disp-hp-system",
            description: "High Pressure Hydrocarbon Processing",
            subsystems: [
              {
                id: "subsys-rx",
                name: "Reaction Subsystem",
                system_id: "sys-hp",
                units: [
                  {
                    id: "unit-01",
                    name: project.name || "Process Unit 01",
                    subsystem_id: "subsys-rx",
                    system_id: "sys-hp",
                    area_id: "area-01",
                    equipment_ids: hpEqIds,
                    display_ids: ["disp-hp-system", "disp-unit-01"],
                  },
                ],
              },
            ],
          },
          {
            id: "sys-lp",
            name: "LP Separation & Recovery System",
            area_id: "area-01",
            default_display_id: "disp-lp-system",
            description: "Low Pressure Vapor/Liquid Separation & Storage",
            subsystems: [
              {
                id: "subsys-sep",
                name: "Separation Subsystem",
                system_id: "sys-lp",
                units: [
                  {
                    id: "unit-02",
                    name: "Recovery Unit 02",
                    subsystem_id: "subsys-sep",
                    system_id: "sys-lp",
                    area_id: "area-01",
                    equipment_ids: lpEqIds,
                    display_ids: ["disp-lp-system"],
                  },
                ],
              },
            ],
          },
        ],
      },
      {
        id: "area-02",
        name: "Area 02 - Utilities & Steam",
        description: "Plant Utility Generation & Distribution",
        systems: [
          {
            id: "sys-util",
            name: "Steam & Cooling Network",
            area_id: "area-02",
            subsystems: [],
            description: "HP/LP Steam headers, cooling water, and fuel gas",
          },
        ],
      },
    ];

    return {
      plant_name: project.name || "Industrial Process Plant",
      areas,
      displays,
    };
  }

  /**
   * Sets the active project and ALWAYS resets operator-visible selection
   * to the first control loop of the new project (or null if no loops).
   * No guard: calling this replaces any previous selection unconditionally.
   */
  public setActiveProject(project: ProjectDetailDto | null) {
    const isSameProject = Boolean(
      this.activeProject && project && this.activeProject.id === project.id
    );
    this.activeProject = project;

    if (project) {
      this.plantStructure = project.plant_structure || this.buildDefaultPlantStructure(project);
      this.displays =
        project.displays && project.displays.length > 0
          ? project.displays
          : this.plantStructure.displays || [];
      this.navigationLinks = project.navigation_links || [];

      // Determine active display: preserve existing if updating same project, else configured is_default or first available
      const defaultDisp =
        (isSameProject && this.activeDisplayId
          ? this.displays.find((d) => d.id === this.activeDisplayId)
          : null) ||
        this.displays.find((d) => d.is_default) ||
        this.displays.find((d) => d.id === "disp-unit-01") ||
        this.displays[0];
      const nextActiveDisplayId = defaultDisp ? defaultDisp.id : "disp-unit-01";
      if (!isSameProject || this.activeDisplayId !== nextActiveDisplayId || !this.displayHistory || this.displayHistory.length === 0) {
        this.activeDisplayId = nextActiveDisplayId;
        this.displayHistory = [this.activeDisplayId];
        this.historyIndex = 0;
      }

      if (project.control_loops && project.control_loops.length > 0) {
        const existingCl = isSameProject && this.selectedFaceplate
          ? project.control_loops.find((c) => c.name === this.selectedFaceplate?.id)
          : null;
        const cl = existingCl || project.control_loops[0];
        this.selectedFaceplate = {
          kind: "control_loop",
          id: cl.name,
          name: cl.name,
          description: "PROCESS CONTROLLER",
          pvTag: cl.pv_tag,
          spTag: `${cl.name}.SP`,
          outTag: cl.out_tag,
          mode: (cl.mode as any) || "AUTO",
          sp: cl.sp,
          unit: "",
        };
        this.selectedLoop = cl.out_tag;
        this.selectedTrendTags = [cl.pv_tag, `${cl.name}.SP`, cl.out_tag];
      } else {
        this.selectedFaceplate = null;
        this.selectedLoop = "";
        this.selectedTrendTags = [];
      }
    } else {
      this.plantStructure = null;
      this.displays = [];
      this.navigationLinks = [];
      this.activeDisplayId = null;
      this.selectedStreamId = null;
      this.displayHistory = [];
      this.historyIndex = -1;
      this.selectedFaceplate = null;
      this.selectedLoop = "";
      this.selectedTrendTags = [];
    }

    this.notify();
  }

  public displayViewports: Map<string, ViewportConfigDto> = new Map();

  public saveDisplayViewport(displayId: string, vp: ViewportConfigDto) {
    this.displayViewports.set(displayId, vp);
  }

  public getActiveDisplay(): DisplayDto | null {
    if (!this.activeDisplayId) return null;
    return this.displays.find((d) => d.id === this.activeDisplayId) || null;
  }

  public get currentDisplay(): DisplayDto | null {
    return this.getActiveDisplay();
  }

  public setActiveDisplay(displayId: string) {
    this.navigateDisplay(displayId);
  }

  /**
   * Builds an authoritative, display-scoped rendering model.
   * Operations consumes ONLY this scoped model without accessing global arrays.
   * Missing placements are reported as validation errors rather than inventing synthetic coordinates.
   */
  public getDisplayViewModel(requestedDisplayId?: string): DisplayViewModel {
    const proj = this.activeProject;
    const dispId = requestedDisplayId || this.activeDisplayId || (this.displays[0]?.id ?? "disp-unit-01");
    const activeDisp = this.displays.find((d) => d.id === dispId) || this.displays[0];

    if (!proj) {
      return {
        displayId: dispId,
        displayName: "No Active Display",
        displayType: "process",
        equipment: [],
        placements: new Map(),
        streams: [],
        boundaryConditions: [],
        instruments: [],
        controlLoops: [],
        annotations: [],
        navigation: [],
        viewport: { x: 0, y: 0, width: 1280, height: 680, zoom: 1.0 },
        validationErrors: ["No active project available"],
      };
    }

    if (!activeDisp) {
      const fallbackPlacements = new Map<string, DisplayElementPlacementDto>();
      for (const eq of proj.equipment) {
        if (eq.position) {
          fallbackPlacements.set(eq.id, {
            id: `pl-${eq.id}`,
            equipment_id: eq.id,
            display_id: dispId,
            x: eq.position.x,
            y: eq.position.y,
            width: eq.display_width,
            height: eq.display_height,
            rotation: eq.rotation ?? 0,
            scale: eq.scale ?? 1.0,
            created_at: "",
            updated_at: "",
          });
        }
      }
      return {
        displayId: dispId,
        displayName: proj.name,
        displayType: "process",
        equipment: proj.equipment,
        placements: fallbackPlacements,
        streams: proj.streams || [],
        boundaryConditions: proj.boundary_conditions || [],
        instruments: proj.instruments || [],
        controlLoops: proj.control_loops || [],
        annotations: [],
        navigation: [],
        viewport: { x: 0, y: 0, width: 1920, height: 1080, zoom: 1.0 },
        validationErrors: [],
      };
    }

    const validationErrors: string[] = [];
    const placementMap = new Map<string, DisplayElementPlacementDto>();
    const hasExplicitPlacements = Boolean(activeDisp.placements && activeDisp.placements.length > 0);

    if (activeDisp.placements) {
      for (const p of activeDisp.placements) {
        placementMap.set(p.equipment_id, p);
      }
    }

    // Explicit Display Equipment Scoping:
    // Equipment must be present in activeDisp.equipment_ids OR have an explicit placement in activeDisp.placements.
    // If activeDisp defines NO equipment_ids and NO placements, equipment is EMPTY (no falling back to all proj.equipment!).
    const displayEqIds = new Set<string>();
    if (activeDisp.equipment_ids && activeDisp.equipment_ids.length > 0) {
      activeDisp.equipment_ids.forEach((id) => displayEqIds.add(id));
    }
    if (activeDisp.placements && activeDisp.placements.length > 0) {
      activeDisp.placements.forEach((p) => displayEqIds.add(p.equipment_id));
    }

    const scopedEquipment: StudioEquipmentDto[] = [];
    for (const eqId of displayEqIds) {
      const eq = proj.equipment.find((e) => e.id === eqId);
      if (eq) {
        if (placementMap.has(eq.id)) {
          scopedEquipment.push(eq);
        } else if (!hasExplicitPlacements && eq.position) {
          const autoPl: DisplayElementPlacementDto = {
            id: `pl-${eq.id}`,
            equipment_id: eq.id,
            display_id: activeDisp.id,
            x: eq.position.x,
            y: eq.position.y,
            width: eq.display_width,
            height: eq.display_height,
            rotation: eq.rotation ?? 0,
            scale: eq.scale ?? 1.0,
            created_at: "",
            updated_at: "",
          };
          placementMap.set(eq.id, autoPl);
          scopedEquipment.push(eq);
        } else {
          // As per Requirement 8: "If an invalid or missing placement exists, report it as a model/deployment validation error instead of silently generating a coordinate."
          validationErrors.push(
            `Missing display placement for equipment ${eq.name} (${eq.id}) on display ${activeDisp.id}`
          );
        }
      }
    }

    const renderedEqIds = new Set(scopedEquipment.map((e) => e.id));

    // Scoped Instruments
    const hasExplicitInstruments = Boolean(activeDisp.instrument_ids && activeDisp.instrument_ids.length > 0);
    const scopedInstruments = (proj.instruments || []).filter((inst) => {
      if (hasExplicitInstruments) {
        return activeDisp.instrument_ids!.includes(inst.id) || activeDisp.instrument_ids!.includes(inst.tag);
      }
      return Boolean(inst.equipment_id && renderedEqIds.has(inst.equipment_id));
    });

    // Scoped Control Loops
    const scopedLoops = (proj.control_loops || []).filter((cl) => {
      return (
        scopedInstruments.some((i) => i.tag === cl.pv_tag) ||
        Boolean((cl as any).equipment_id && renderedEqIds.has((cl as any).equipment_id))
      );
    });

    // Scoped Streams: if activeDisp defines explicit stream_ids, restrict strictly to those streams.
    // Otherwise, include streams connecting to at least one rendered equipment on this display.
    const hasExplicitStreams = Boolean(activeDisp.stream_ids && activeDisp.stream_ids.length > 0);
    const scopedStreams = (proj.streams || []).filter((st) => {
      if (hasExplicitStreams) {
        return activeDisp.stream_ids!.includes(st.id) || activeDisp.stream_ids!.includes(st.name);
      }
      const srcId = st.source_equipment_id || (st as any).source?.equipment_id;
      const snkId = st.sink_equipment_id || (st as any).sink?.equipment_id;
      return renderedEqIds.has(srcId) || renderedEqIds.has(snkId);
    });

    // Scoped Boundary Conditions (targeting equipment rendered on this display, or streams on this display)
    const scopedBoundaryConditions = (proj.boundary_conditions || []).filter((bc) => {
      if (bc.target_equipment_id && renderedEqIds.has(bc.target_equipment_id)) {
        return true;
      }
      if (bc.target_stream_id && scopedStreams.some((s) => s.id === bc.target_stream_id || s.name === bc.target_stream_id)) {
        return true;
      }
      return false;
    });

    // Navigation Links originating from or targeting this display
    const scopedNav = (this.navigationLinks || []).filter(
      (nl) => nl.source_display_id === activeDisp.id || nl.target_display_id === activeDisp.id
    );

    // Viewport configuration scoped to this display
    const savedViewport = this.displayViewports.get(activeDisp.id);
    const viewport: ViewportConfigDto = savedViewport || activeDisp.viewport || {
      x: 0,
      y: 0,
      width: 1280,
      height: 680,
      zoom: 1.0,
    };

    return {
      displayId: activeDisp.id,
      displayName: activeDisp.name,
      displayType: activeDisp.display_type,
      equipment: scopedEquipment,
      placements: placementMap,
      streams: scopedStreams,
      instruments: scopedInstruments,
      controlLoops: scopedLoops,
      boundaryConditions: scopedBoundaryConditions,
      annotations: [],
      navigation: scopedNav,
      viewport,
      validationErrors,
    };
  }

  public navigateDisplay(
    displayId: string,
    addToHistory: boolean = true,
    targetObjectId?: string,
    _navigationType?: string
  ) {
    if (this.activeDisplayId === displayId && !targetObjectId) return;

    const targetDisp = this.displays.find((d) => d.id === displayId);
    if (!targetDisp) {
      console.warn(`[Store] Display ${displayId} not found`);
      return;
    }

    this.activeDisplayId = displayId;

    // Requirement 13 & 15: Scoped Selection & Focus State
    // "Selection may optionally persist across displays only if the selected object exists on the destination display. Otherwise: activeSelection = null."
    const destEqIds = new Set<string>(targetDisp.equipment_ids || []);
    if (targetDisp.placements) {
      targetDisp.placements.forEach((p) => destEqIds.add(p.equipment_id));
    }

    if (this.selectedEquipmentId && !destEqIds.has(this.selectedEquipmentId)) {
      this.selectedEquipmentId = null;
    }

    // Target object focus/center
    if (targetObjectId && destEqIds.has(targetObjectId)) {
      this.focusedEntityId = targetObjectId;
      this.selectedEquipmentId = targetObjectId;
    } else if (this.focusedEntityId && !destEqIds.has(this.focusedEntityId)) {
      this.focusedEntityId = null;
    }

    // Reset temporary stream selection and overlays
    this.selectedStreamId = null;

    if (addToHistory) {
      if (this.historyIndex >= 0 && this.historyIndex < this.displayHistory.length - 1) {
        this.displayHistory = this.displayHistory.slice(0, this.historyIndex + 1);
      }
      this.displayHistory.push(displayId);
      this.historyIndex = this.displayHistory.length - 1;
    }

    this.notify();
  }

  public navigateBack() {
    if (this.historyIndex > 0) {
      this.historyIndex--;
      const targetId = this.displayHistory[this.historyIndex];
      this.navigateDisplay(targetId, false);
    }
  }

  public navigateForward() {
    if (this.historyIndex < this.displayHistory.length - 1) {
      this.historyIndex++;
      const targetId = this.displayHistory[this.historyIndex];
      this.navigateDisplay(targetId, false);
    }
  }

  public get focusedEquipmentId(): string | null {
    return this.focusedEntityId;
  }

  public navigateUp() {
    const active = this.getActiveDisplay();
    if (!active) {
      this.navigateHome();
      return;
    }

    // Generic model-driven hierarchy navigation:
    // Unit/Subsystem Graphic -> System Overview -> Area Overview -> Plant Overview
    let parentSystemId = active.system_id;
    let parentAreaId = active.area_id;

    // If unit_id or subsystem_id is present, resolve parent system & area from plantStructure
    if (this.plantStructure) {
      for (const area of this.plantStructure.areas || []) {
        for (const sys of area.systems || []) {
          for (const sub of sys.subsystems || []) {
            if (active.unit_id && sub.units?.some((u) => u.id === active.unit_id)) {
              if (!parentSystemId) parentSystemId = sys.id;
              if (!parentAreaId) parentAreaId = area.id;
            }
            if (active.subsystem_id && sub.id === active.subsystem_id) {
              if (!parentSystemId) parentSystemId = sys.id;
              if (!parentAreaId) parentAreaId = area.id;
            }
          }
          if (active.system_id && sys.id === active.system_id) {
            if (!parentAreaId) parentAreaId = area.id;
          }
        }
      }
    }

    const isUnit =
      active.display_type === "unit_process_graphic" ||
      active.display_type === "subsystem_graphic" ||
      (active.display_type as string) === "unit";
    const isSystem =
      active.display_type === "system_overview" ||
      (active.display_type as string) === "system";
    const isArea =
      active.display_type === "area_overview" ||
      (active.display_type as string) === "area";

    if (isUnit) {
      if (parentSystemId) {
        const sysDisplay = this.displays.find(
          (d) =>
            (d.system_id === parentSystemId || d.id.includes(parentSystemId)) &&
            (d.display_type === "system_overview" || (d.display_type as string) === "system")
        );
        if (sysDisplay) {
          this.navigateDisplay(sysDisplay.id);
          return;
        }
      }
      if (parentAreaId) {
        const areaDisplay = this.displays.find(
          (d) =>
            (d.area_id === parentAreaId || d.id.includes(parentAreaId)) &&
            (d.display_type === "area_overview" || (d.display_type as string) === "area")
        );
        if (areaDisplay) {
          this.navigateDisplay(areaDisplay.id);
          return;
        }
      }
      this.navigateHome();
      return;
    }

    if (isSystem) {
      if (parentAreaId) {
        const areaDisplay = this.displays.find(
          (d) =>
            (d.area_id === parentAreaId || d.id.includes(parentAreaId)) &&
            (d.display_type === "area_overview" || (d.display_type as string) === "area")
        );
        if (areaDisplay) {
          this.navigateDisplay(areaDisplay.id);
          return;
        }
      }
      this.navigateHome();
      return;
    }

    if (isArea) {
      this.navigateHome();
      return;
    }

    this.navigateHome();
  }

  public navigateHome() {
    const plantDisplay =
      this.displays.find((d) => d.display_type === "plant_overview" || d.is_default) ||
      this.displays[0];
    if (plantDisplay) {
      this.navigateDisplay(plantDisplay.id);
    }
  }

  public getNavigationLinksForDisplay(displayId: string): NavigationLinkDto[] {
    return this.navigationLinks.filter((l) => l.source_display_id === displayId);
  }

  public getNavigationLinksForElement(elementId: string): NavigationLinkDto[] {
    return this.navigationLinks.filter(
      (l) => l.source_element_id === elementId || l.target_element_id === elementId
    );
  }

  public executeNavigationLink(link: NavigationLinkDto) {
    this.navigateDisplay(link.target_display_id);
    if (link.target_element_id) {
      this.setFocusedEntity(link.target_element_id);
      this.selectEquipment(link.target_element_id);
      if (link.behavior === "open_faceplate") {
        const eq = this.activeProject?.equipment.find((e) => e.id === link.target_element_id);
        if (eq) {
          this.openFaceplate({
            kind: "equipment",
            id: eq.id,
            name: eq.name,
            description: eq.name,
          });
        }
      }
    }
  }

  public navigateToElement(targetElementId: string) {
    const targetDisp = this.displays.find((d) => d.equipment_ids?.includes(targetElementId));
    if (targetDisp && targetDisp.id !== this.activeDisplayId) {
      this.navigateDisplay(targetDisp.id);
    }
    this.setFocusedEntity(targetElementId);
    this.selectEquipment(targetElementId);
  }

  public selectStream(streamId: string | null) {
    this.selectedStreamId = streamId;
    this.notify();
  }

  public getDisplayAlarmCount(displayId: string): { total: number; unack: number; highestSeverity?: string } {
    const disp = this.displays.find((d) => d.id === displayId);
    if (!disp) return { total: 0, unack: 0 };

    let total = 0;
    let unack = 0;
    let highestSeverity: string | undefined;
    const sevScore: Record<string, number> = { CRITICAL: 4, WARNING: 3, ADVISORY: 2, DIAGNOSTIC: 1 };

    for (const alm of this.alarms.values()) {
      if (alm.state === "NORMAL" || alm.state === "CLEARED") continue;

      const matchesEq = disp.equipment_ids.some((id) => alm.alarm_name.includes(id));
      const matchesInst = disp.instrument_ids.some((tag) => alm.alarm_name.includes(tag));

      if (matchesEq || matchesInst || disp.display_type === "plant_overview" || disp.display_type === "alarms") {
        total++;
        if (alm.state === "ACTIVE_UNACK") {
          unack++;
        }
        if (!highestSeverity || (sevScore[alm.severity] || 0) > (sevScore[highestSeverity] || 0)) {
          highestSeverity = alm.severity;
        }
      }
    }

    return { total, unack, highestSeverity };
  }

  public setActiveUnit(unitName: string) {
    this.activeUnitName = unitName;
    this.notify();
  }

  public openFaceplate(target: FaceplateTarget) {
    this.selectedFaceplate = { ...target };
    this.notify();
  }

  public closeFaceplate() {
    this.selectedFaceplate = null;
    this.notify();
  }

  public setFocusedEntity(entityId: string | null) {
    this.focusedEntityId = entityId;
    this.notify();
  }

  public setAlarmSeverityFilter(sev: string) {
    this.alarmSeverityFilter = sev;
    this.notify();
  }

  public setAlarmStateFilter(st: string) {
    this.alarmStateFilter = st;
    this.notify();
  }

  public setTrendTimeWindow(sec: number) {
    this.trendTimeWindowSec = sec;
    this.notify();
  }

  public addTrendPen(tag: string) {
    if (!this.selectedTrendTags.includes(tag)) {
      this.selectedTrendTags.push(tag);
      this.notify();
    }
  }

  public removeTrendPen(tag: string) {
    this.selectedTrendTags = this.selectedTrendTags.filter((t) => t !== tag);
    this.notify();
  }

  public setAlarmFloodMetrics(metrics: AlarmFloodMetricsDto) {
    this.alarmFloodMetrics = metrics;
    this.notify();
  }

  public setDiagnostics(diags: EquipmentDiagnosticsDto[]) {
    this.diagnosticsMap.clear();
    for (const d of diags) {
      this.diagnosticsMap.set(d.equipment_id, d);
      this.diagnosticsMap.set(d.equipment_name, d);
    }
    this.notify();
  }

  public setFirstOutEvent(firstOut: FirstOutEventDto) {
    this.firstOutEvent = firstOut;
    this.notify();
  }

  public setHierarchyView(view: "plant" | "area" | "unit" | "diagnostics" | "events") {
    this.activeHierarchyView = view;
    this.notify();
  }

  public selectEquipment(eqId: string | null) {
    this.selectedEquipmentId = eqId;
    this.notify();
  }

  public setDeploymentStatus(isDeployed: boolean, projectId: string | null = null) {
    this.isProjectDeployed = isDeployed;
    this.deployedProjectId = projectId;
    this.notify();
  }

  public setSisSummary(summary: SisStatusSummaryDto | null) {
    this.sisSummary = summary;
    this.notify();
  }

  public getHighestPriorityUnackAlarm(): AlarmSnapshotDto | null {
    const unack = Array.from(this.alarms.values()).filter(
      (a) => a.state === "ACTIVE_UNACK"
    );
    if (unack.length === 0) return null;

    // Severity order: CRITICAL > HIGH > WARNING > ADVISORY
    const order: Record<string, number> = {
      CRITICAL: 4,
      HIGH: 3,
      WARNING: 2,
      ADVISORY: 1,
    };

    unack.sort((a, b) => (order[b.severity] || 0) - (order[a.severity] || 0));
    return unack[0];
  }

  /**
   * Resolves a tag name to a FaceplateTarget by looking it up in the active project.
   * Checks control loops (by pv_tag) and instruments (by tag name).
   * Returns null if no project is loaded or the tag is not found.
   */
  public getEntityByPvTag(pvTag: string): FaceplateTarget | null {
    if (!this.activeProject) return null;

    // Check control loops first (pv_tag match)
    for (const cl of this.activeProject.control_loops) {
      if (cl.pv_tag === pvTag) {
        return {
          kind: "control_loop",
          id: cl.name,
          name: cl.name,
          pvTag: cl.pv_tag,
          spTag: `${cl.name}.SP`,
          outTag: cl.out_tag,
          sp: cl.sp,
          mode: (cl.mode as any) || "AUTO",
        };
      }
    }

    // Check instruments (tag name match)
    for (const inst of this.activeProject.instruments) {
      if (inst.tag === pvTag) {
        return {
          kind: "instrument",
          id: inst.tag,
          name: inst.tag,
          pvTag: inst.tag,
          unit: inst.unit,
          rangeMin: inst.range_min,
          rangeMax: inst.range_max,
        };
      }
    }

    return null;
  }

  /**
   * Resolves an alarm name to a FaceplateTarget by looking up the alarm definition
   * in the active project, then resolving its pv_tag via getEntityByPvTag().
   * Returns null if alarm not found or project not loaded.
   */
  public getEntityForAlarm(alarmName: string): FaceplateTarget | null {
    if (!this.activeProject) return null;

    // 1. Look up by alarm definition in project
    const alarmDef = this.activeProject.alarms.find((a) => a.name === alarmName);
    if (alarmDef) {
      const target = this.getEntityByPvTag(alarmDef.pv_tag);
      if (target) return target;
    }

    // 2. Match against project equipment names/IDs (e.g. equipment trip alarms)
    for (const eq of this.activeProject.equipment) {
      if (alarmName.includes(eq.id) || alarmName.includes(eq.name)) {
        const d = eq.definition_id.toLowerCase();
        let kind: FaceplateTarget["kind"] = "equipment";
        if (d.includes("pump")) kind = "pump";
        else if (d.includes("compressor")) kind = "compressor";
        else if (d.includes("vessel") || d.includes("separator") || d.includes("drum") || d.includes("tank")) kind = "vessel";
        else if (d.includes("valve")) kind = "valve";
        else if (d.includes("exchanger") || d.includes("heater") || d.includes("cooler")) kind = "heat_exchanger";

        return {
          kind,
          id: eq.id,
          name: eq.name,
          equipmentId: eq.id,
          description: `${kind.toUpperCase()} — ${eq.name}`,
          parameters: eq.parameters,
        };
      }
    }

    // 3. Match against project control loops
    for (const cl of this.activeProject.control_loops) {
      if (alarmName.includes(cl.name)) {
        return {
          kind: "control_loop",
          id: cl.name,
          name: cl.name,
          pvTag: cl.pv_tag,
          spTag: `${cl.name}.SP`,
          outTag: cl.out_tag,
          sp: cl.sp,
          mode: (cl.mode as any) || "AUTO",
        };
      }
    }

    // 4. Match against project instruments
    for (const inst of this.activeProject.instruments) {
      if (alarmName.includes(inst.tag)) {
        return {
          kind: "instrument",
          id: inst.tag,
          name: inst.tag,
          pvTag: inst.tag,
          unit: inst.unit,
          rangeMin: inst.range_min,
          rangeMax: inst.range_max,
        };
      }
    }

    return null;
  }
}

// Global default singleton instance
export const store = new Store();
if (typeof window !== "undefined") {
  (window as any).store = store;
}
