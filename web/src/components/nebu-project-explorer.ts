/**
 * NebuProjectExplorer Web Component.
 *
 * Implements Stage 14 Authoritative DCS/SCADA & PLC Engineering Project Explorer:
 * - Authoritative Plant -> Area -> System -> Unit -> Display -> Equipment -> Streams -> Instruments -> Loops -> Alarms hierarchy
 * - Direct CRUD actions for Areas, Systems, Units, and Displays
 * - Canvas auto-focus and navigation when clicking Displays and Equipment
 * - Virtual Controllers, Tasks, Programs, and IEC 61131-3 Routines
 * - I/O Hardware Hierarchy (Racks, Chassis, Modules, Channels)
 * - Tag Database & Cross-Reference exploration
 * - Collapsible folders and real-time filtering
 */

import { studioStore } from "../state/studio-store";
import { store } from "../state/store";
import {
  DisplayDto,
  PlantStructureDto,
  PlantSystemDto,
  ProjectDetailDto,
} from "../protocol/types";

export interface ProjectItemSelectedDetail {
  section:
    | "flowsheet"
    | "controller"
    | "task"
    | "routine"
    | "io_system"
    | "io_module"
    | "io_mapping"
    | "tag"
    | "alarm"
    | "cross_ref"
    | "advanced_control"
    | "plant_structure"
    | "control_loop";
  id?: string;
  subId?: string;
  extra?: any;
  displayId?: string;
}

export class NebuProjectExplorer extends HTMLElement {
  private searchQuery = "";
  private collapsedNodes: Set<string> = new Set();
  private unsubscribe: (() => void) | null = null;

  connectedCallback() {
    this.unsubscribe = studioStore.subscribe(() => this.render());
    this.render();
  }

  disconnectedCallback() {
    if (this.unsubscribe) {
      this.unsubscribe();
      this.unsubscribe = null;
    }
  }

  private toggleNode(nodeId: string, e: Event) {
    e.stopPropagation();
    if (this.collapsedNodes.has(nodeId)) {
      this.collapsedNodes.delete(nodeId);
    } else {
      this.collapsedNodes.add(nodeId);
    }
    this.render();
  }

  private selectItem(detail: ProjectItemSelectedDetail) {
    this.dispatchEvent(
      new CustomEvent<ProjectItemSelectedDetail>("project-item-selected", {
        bubbles: true,
        composed: true,
        detail,
      })
    );
  }

  private getStructure(proj: ProjectDetailDto): PlantStructureDto {
    if (proj.plant_structure) {
      return proj.plant_structure;
    }
    const fallback = store.buildDefaultPlantStructure(proj);
    proj.plant_structure = fallback;
    return fallback;
  }

  private saveStructure(proj: ProjectDetailDto, struct: PlantStructureDto) {
    proj.plant_structure = struct;
    proj.displays = struct.displays;
    proj.areas = struct.areas;
    store.plantStructure = struct;
    store.displays = struct.displays;
    studioStore.setProject(proj, false);
    store.notify();
  }

  private addAreaPrompt(proj: ProjectDetailDto) {
    const struct = this.getStructure(proj);
    const name = prompt("Enter new Process Area name:", `Area ${struct.areas.length + 1}`);
    if (!name) return;
    const id = `area-${Date.now().toString(36).slice(-4)}`;
    struct.areas.push({
      id,
      name,
      description: "Industrial process area",
      systems: [],
    });
    this.saveStructure(proj, struct);
    this.collapsedNodes.delete(`area-${id}`);
  }

  private addSystemPrompt(proj: ProjectDetailDto, areaId?: string) {
    const struct = this.getStructure(proj);
    if (struct.areas.length === 0) {
      this.addAreaPrompt(proj);
      return;
    }
    const targetArea = areaId ? struct.areas.find((a) => a.id === areaId) : struct.areas[0];
    if (!targetArea) return;
    const name = prompt(`Enter new Process System name for ${targetArea.name}:`, `System ${targetArea.systems.length + 1}`);
    if (!name) return;
    const id = `sys-${Date.now().toString(36).slice(-4)}`;
    targetArea.systems.push({
      id,
      name,
      area_id: targetArea.id,
      description: "Process system train",
      subsystems: [],
    });
    this.saveStructure(proj, struct);
    this.collapsedNodes.delete(`area-${targetArea.id}`);
  }

  private addUnitPrompt(proj: ProjectDetailDto, systemId?: string) {
    const struct = this.getStructure(proj);
    let targetSys: PlantSystemDto | undefined;
    for (const a of struct.areas) {
      const s = a.systems.find((sys) => sys.id === systemId);
      if (s) {
        targetSys = s;
        break;
      }
    }
    if (!targetSys && struct.areas[0]?.systems[0]) {
      targetSys = struct.areas[0].systems[0];
    }
    if (!targetSys) {
      alert("Please create an Area and System before adding a Unit.");
      return;
    }

    const name = prompt(`Enter new Process Unit name for ${targetSys.name}:`, "New Unit");
    if (!name) return;
    const id = `unit-${Date.now().toString(36).slice(-4)}`;

    if (!targetSys.subsystems) targetSys.subsystems = [];
    let sub = targetSys.subsystems[0];
    if (!sub) {
      sub = {
        id: `sub-${Date.now().toString(36).slice(-4)}`,
        name: "Main Section",
        system_id: targetSys.id,
        units: [],
      };
      targetSys.subsystems.push(sub);
    }

    sub.units.push({
      id,
      name,
      system_id: targetSys.id,
      equipment_ids: [],
      display_ids: [],
      description: "Operating unit",
    });

    this.saveStructure(proj, struct);
    this.collapsedNodes.delete(`sys-${targetSys.id}`);
  }

  private addDisplayPrompt(proj: ProjectDetailDto, unitId?: string) {
    const struct = this.getStructure(proj);
    const name = prompt("Enter new DCS Process Graphic Display name:", "Graphic Display");
    if (!name) return;
    const id = `disp-${Date.now().toString(36).slice(-4)}`;
    const newDisp: DisplayDto = {
      id,
      name,
      display_type: "unit_process_graphic",
      unit_id: unitId,
      equipment_ids: [],
      stream_ids: [],
      instrument_ids: [],
    };
    struct.displays.push(newDisp);
    this.saveStructure(proj, struct);
  }

  render() {
    const proj: ProjectDetailDto | null = studioStore.getProject();
    if (!proj) {
      this.innerHTML = `
        <div class="p-3 text-dim mono text-xs">
          No project loaded. Open or create a project to inspect hierarchy.
        </div>
      `;
      return;
    }

    const struct = this.getStructure(proj);
    const q = this.searchQuery.toLowerCase().trim();

    // Filtering helpers
    const matches = (str?: string) => !q || (str ? str.toLowerCase().includes(q) : false);

    const isNodeOpen = (id: string) => !this.collapsedNodes.has(id);

    // Collect all assigned equipment IDs
    const assignedEqIds = new Set<string>();
    for (const a of struct.areas) {
      for (const s of a.systems) {
        for (const sub of s.subsystems || []) {
          for (const u of sub.units) {
            (u.equipment_ids || []).forEach((id) => assignedEqIds.add(id));
          }
        }
      }
    }

    const unassignedEquipment = (proj.equipment || []).filter((e) => !assignedEqIds.has(e.id));

    // Secondary filters
    const controllers = (proj.controllers || []).filter(
      (c) => !q || c.name.toLowerCase().includes(q) || c.id.toLowerCase().includes(q)
    );
    const ioSystems = (proj.io_systems || []).filter(
      (s) => !q || s.name.toLowerCase().includes(q) || s.id.toLowerCase().includes(q)
    );
    const tags = (proj.tags || []).filter(
      (t) =>
        !q ||
        t.name.toLowerCase().includes(q) ||
        t.id.toLowerCase().includes(q) ||
        (t.description ? t.description.toLowerCase().includes(q) : false)
    );
    const mappings = (proj.io_mappings || []).filter(
      (m) => !q || m.signal_key.toLowerCase().includes(q) || m.channel_address.toLowerCase().includes(q)
    );
    const advancedControl = (proj.advanced_control || []).filter(
      (s) => !q || s.name.toLowerCase().includes(q) || s.id.toLowerCase().includes(q) || s.strategy_type.toLowerCase().includes(q)
    );

    this.innerHTML = `
      <div class="project-explorer-shell flex flex-col h-full min-h-0 bg-panel text-main text-xs select-none">
        <!-- Explorer Header & Actions -->
        <div class="p-2 border-b border-color bg-panel-header">
          <div class="flex items-center justify-between mb-1.5">
            <span class="font-bold tracking-wider uppercase text-sky-400 text-xs flex items-center gap-1">
              <span>🏛️</span> PLANT EXPLORER
            </span>
            <span class="badge badge-dim mono text-[10px]">${proj.id}</span>
          </div>

          <!-- Quick Structure Action Bar -->
          <div class="flex items-center gap-1 mb-2">
            <button class="btn-explorer-action px-1.5 py-0.5 bg-[#1e293b] hover:bg-[#334155] text-sky-300 rounded text-[10px] mono" data-action="add-area" title="Add Process Area">+ Area</button>
            <button class="btn-explorer-action px-1.5 py-0.5 bg-[#1e293b] hover:bg-[#334155] text-sky-300 rounded text-[10px] mono" data-action="add-system" title="Add System to Area">+ Sys</button>
            <button class="btn-explorer-action px-1.5 py-0.5 bg-[#1e293b] hover:bg-[#334155] text-sky-300 rounded text-[10px] mono" data-action="add-unit" title="Add Operating Unit">+ Unit</button>
            <button class="btn-explorer-action px-1.5 py-0.5 bg-[#1e293b] hover:bg-[#334155] text-emerald-300 rounded text-[10px] mono" data-action="add-display" title="Add Graphic Display">+ Disp</button>
            <button class="btn-explorer-action ml-auto px-1.5 py-0.5 bg-slate-800 hover:bg-slate-700 text-amber-300 rounded text-[10px] mono" data-action="manage-structure" title="Open Plant Structure Manager">⚙ Struct</button>
          </div>

          <input
            type="text"
            class="explorer-search w-full px-2 py-1 bg-input border border-color rounded text-xs mono text-main"
            placeholder="Filter plant hierarchy, tags..."
            value="${this.searchQuery}"
          />
        </div>

        <!-- Tree View Container -->
        <div class="explorer-tree flex-1 overflow-y-auto min-h-0 p-1 space-y-1">
          <!-- 1. AUTHORITATIVE PLANT HIERARCHY ROOT -->
          <div class="tree-group">
            <div class="tree-header flex items-center justify-between px-2 py-1 bg-[#131b2a] hover:bg-slate-800 rounded cursor-pointer group" data-node="plant-root">
              <div class="flex items-center gap-1.5 flex-1 min-w-0">
                <span class="tree-toggle text-dim font-bold">${isNodeOpen("plant-root") ? "▼" : "▶"}</span>
                <span class="text-amber-400 font-bold">🏭</span>
                <span class="font-bold text-sky-200 truncate">${struct.plant_name || proj.name}</span>
              </div>
              <span class="badge badge-dim text-[10px]">${struct.areas.length} Areas</span>
            </div>

            ${
              isNodeOpen("plant-root")
                ? `
              <div class="tree-children pl-2 py-0.5 space-y-1">
                ${
                  struct.areas.length === 0
                    ? `<div class="px-3 py-1 text-dim italic">No areas configured. Click "+ Area" above.</div>`
                    : struct.areas
                        .filter((a) => matches(a.name) || matches(a.id) || (a.systems && a.systems.some((s) => matches(s.name) || matches(s.id))))
                        .map((area) => {
                          const aNode = `area-${area.id}`;
                          return `
                        <!-- Area Node -->
                        <div class="tree-subgroup border-l border-[#1e293b] pl-2 my-0.5">
                          <div class="tree-header flex items-center justify-between px-1.5 py-0.5 hover:bg-slate-800 rounded cursor-pointer" data-node="${aNode}">
                            <div class="flex items-center gap-1 flex-1 min-w-0">
                              <span class="tree-toggle text-dim">${isNodeOpen(aNode) ? "▼" : "▶"}</span>
                              <span class="text-sky-400 font-semibold">📁</span>
                              <span class="mono font-semibold text-sky-300 truncate">${area.name}</span>
                            </div>
                            <span class="badge badge-dim text-[10px]">${area.systems.length} Sys</span>
                          </div>

                          ${
                            isNodeOpen(aNode)
                              ? `
                            <div class="tree-children pl-2 py-0.5 space-y-0.5">
                              ${area.systems
                                .map((sys) => {
                                  const sNode = `sys-${sys.id}`;
                                  const allUnits = (sys.subsystems || []).flatMap((sub) => sub.units);
                                  return `
                                <!-- System Node -->
                                <div class="tree-subgroup border-l border-[#1e293b] pl-2 my-0.5">
                                  <div class="tree-header flex items-center justify-between px-1 py-0.5 hover:bg-slate-800 rounded cursor-pointer" data-node="${sNode}">
                                    <div class="flex items-center gap-1 flex-1 min-w-0">
                                      <span class="tree-toggle text-dim">${isNodeOpen(sNode) ? "▼" : "▶"}</span>
                                      <span class="text-emerald-400">⚙</span>
                                      <span class="mono font-medium text-emerald-300 truncate">${sys.name}</span>
                                    </div>
                                    <span class="badge badge-dim text-[10px]">${allUnits.length} Units</span>
                                  </div>

                                  ${
                                    isNodeOpen(sNode)
                                      ? `
                                    <div class="tree-children pl-2 py-0.5 space-y-0.5">
                                      ${allUnits
                                        .map((unit) => {
                                          const uNode = `unit-${unit.id}`;
                                          const unitEq = (proj.equipment || []).filter(
                                            (e) => (unit.equipment_ids || []).includes(e.id)
                                          );
                                          const unitDisplays = (struct.displays || []).filter(
                                            (d) => d.unit_id === unit.id || (unit.display_ids || []).includes(d.id)
                                          );
                                          return `
                                        <!-- Unit Node -->
                                        <div class="tree-subgroup border-l border-[#1e293b] pl-2 my-0.5">
                                          <div class="tree-header flex items-center justify-between px-1 py-0.5 hover:bg-slate-800 rounded cursor-pointer" data-node="${uNode}">
                                            <div class="flex items-center gap-1 flex-1 min-w-0">
                                              <span class="tree-toggle text-dim">${isNodeOpen(uNode) ? "▼" : "▶"}</span>
                                              <span class="text-amber-300">📦</span>
                                              <span class="mono font-medium text-amber-200 truncate">${unit.name}</span>
                                            </div>
                                            <span class="badge badge-dim text-[10px]">${unitEq.length} Eq</span>
                                          </div>

                                          ${
                                            isNodeOpen(uNode)
                                              ? `
                                            <div class="tree-children pl-2 py-0.5 space-y-1">
                                              <!-- Displays Subfolder -->
                                              <div class="display-folder">
                                                <div class="flex items-center justify-between px-1 text-[11px] text-sky-400 font-semibold">
                                                  <span>📺 Displays (${unitDisplays.length})</span>
                                                  <button class="btn-add-disp-to-unit text-[10px] text-sky-300 hover:text-white" data-unit-id="${unit.id}">+ Add</button>
                                                </div>
                                                ${
                                                  unitDisplays.length === 0
                                                    ? `<div class="pl-2 text-dim text-[10px] italic">No displays assigned</div>`
                                                    : unitDisplays
                                                        .map(
                                                          (disp) => `
                                                      <div class="tree-item flex items-center justify-between px-1.5 py-0.5 hover:bg-slate-800 rounded cursor-pointer" data-select-display="${disp.id}">
                                                        <div class="flex items-center gap-1 min-w-0">
                                                          <span class="text-info">📐</span>
                                                          <span class="mono text-main truncate">${disp.name}</span>
                                                        </div>
                                                        <span class="badge badge-info text-[9px] mono">${disp.display_type}</span>
                                                      </div>
                                                    `
                                                        )
                                                        .join("")
                                                }
                                              </div>

                                              <!-- Equipment & Sub-elements Subfolder -->
                                              <div class="equipment-folder">
                                                <div class="px-1 text-[11px] text-slate-400 font-semibold">
                                                  🔧 Equipment (${unitEq.length})
                                                </div>
                                                ${
                                                  unitEq.length === 0
                                                    ? `<div class="pl-2 text-dim text-[10px] italic">No equipment in this unit</div>`
                                                    : unitEq
                                                        .map((eq) => {
                                                          const eqNode = `eq-${eq.id}`;
                                                          const connectedStreams = (proj.streams || []).filter(
                                                            (s) => s.source_equipment_id === eq.id || s.sink_equipment_id === eq.id
                                                          );
                                                          const attachedInsts = (proj.instruments || []).filter(
                                                            (i) => i.equipment_id === eq.id
                                                          );
                                                          const assocLoops = (proj.control_loops || []).filter(
                                                            (l) => l.pv_tag?.includes(eq.name) || l.out_tag?.includes(eq.name) || l.name?.includes(eq.name)
                                                          );
                                                          const assocAlarms = (proj.alarms || []).filter(
                                                            (a) => a.pv_tag?.includes(eq.name) || a.name?.includes(eq.name)
                                                          );

                                                          return `
                                                        <div class="tree-subgroup pl-1">
                                                          <div class="tree-header flex items-center justify-between px-1 py-0.5 hover:bg-slate-800 rounded cursor-pointer" data-node="${eqNode}">
                                                            <div class="flex items-center gap-1 min-w-0" data-select-eq="${eq.id}">
                                                              <span class="tree-toggle text-dim">${isNodeOpen(eqNode) ? "▼" : "▶"}</span>
                                                              <span class="text-sky-400">▪</span>
                                                              <span class="mono font-medium text-main truncate">${eq.name}</span>
                                                            </div>
                                                            <span class="text-dim mono text-[10px] truncate max-w-[80px]">${eq.definition_id?.split(".").pop() || "Unit"}</span>
                                                          </div>

                                                          ${
                                                            isNodeOpen(eqNode)
                                                              ? `
                                                            <div class="tree-children pl-3 py-0.5 space-y-0.5 text-[11px]">
                                                              <!-- Streams -->
                                                              ${connectedStreams
                                                                .map(
                                                                  (st) => `
                                                                <div class="tree-item flex items-center justify-between px-1 py-0.5 hover:bg-slate-800 rounded cursor-pointer" data-select-stream="${st.id}">
                                                                  <div class="flex items-center gap-1 min-w-0">
                                                                    <span class="text-sky-500">〰</span>
                                                                    <span class="mono text-slate-300 truncate">${st.name}</span>
                                                                  </div>
                                                                  <span class="text-[9px] mono text-dim">${st.stream_type}</span>
                                                                </div>
                                                              `
                                                                )
                                                                .join("")}

                                                              <!-- Instruments -->
                                                              ${attachedInsts
                                                                .map(
                                                                  (inst) => `
                                                                <div class="tree-item flex items-center justify-between px-1 py-0.5 hover:bg-slate-800 rounded cursor-pointer" data-select-inst="${inst.id}">
                                                                  <div class="flex items-center gap-1 min-w-0">
                                                                    <span class="text-amber-400">🏷️</span>
                                                                    <span class="mono text-amber-300 truncate">${inst.tag}</span>
                                                                  </div>
                                                                  <span class="text-[9px] mono text-dim">${inst.measurement_kind}</span>
                                                                </div>
                                                              `
                                                                )
                                                                .join("")}

                                                              <!-- Control Loops -->
                                                              ${assocLoops
                                                                .map(
                                                                  (loop) => `
                                                                <div class="tree-item flex items-center justify-between px-1 py-0.5 hover:bg-slate-800 rounded cursor-pointer" data-select-loop="${loop.id}">
                                                                  <div class="flex items-center gap-1 min-w-0">
                                                                    <span class="text-emerald-400">🎮</span>
                                                                    <span class="mono text-emerald-300 truncate">${loop.name}</span>
                                                                  </div>
                                                                  <span class="text-[9px] mono text-dim">${loop.mode}</span>
                                                                </div>
                                                              `
                                                                )
                                                                .join("")}

                                                              <!-- Alarms -->
                                                              ${assocAlarms
                                                                .map(
                                                                  (alm) => `
                                                                <div class="tree-item flex items-center justify-between px-1 py-0.5 hover:bg-slate-800 rounded cursor-pointer" data-select-alarm="${alm.id}">
                                                                  <div class="flex items-center gap-1 min-w-0">
                                                                    <span class="text-rose-400">🔔</span>
                                                                    <span class="mono text-rose-300 truncate">${alm.name}</span>
                                                                  </div>
                                                                  <span class="text-[9px] mono text-dim">${alm.alarm_type}</span>
                                                                </div>
                                                              `
                                                                )
                                                                .join("")}
                                                            </div>
                                                          `
                                                              : ""
                                                          }
                                                        </div>
                                                      `;
                                                        })
                                                        .join("")
                                                }
                                              </div>
                                            </div>
                                          `
                                              : ""
                                          }
                                        </div>
                                      `;
                                        })
                                        .join("")}
                                    </div>
                                  `
                                      : ""
                                  }
                                </div>
                              `;
                                })
                                .join("")}
                            </div>
                          `
                              : ""
                          }
                        </div>
                      `;
                        })
                        .join("")
                }
              </div>
            `
                : ""
            }
          </div>

          <!-- 2. UNASSIGNED / FLOWSHEET OVERVIEW ELEMENTS -->
          ${
            unassignedEquipment.length > 0
              ? `
            <div class="tree-group border-t border-[#1e293b] pt-1">
              <div class="tree-header flex items-center justify-between px-2 py-1 hover:bg-slate-800 rounded cursor-pointer group" data-node="unassigned-flowsheet">
                <div class="flex items-center gap-1.5 flex-1 min-w-0">
                  <span class="tree-toggle text-dim font-bold">${isNodeOpen("unassigned-flowsheet") ? "▼" : "▶"}</span>
                  <span class="text-sky-400">📐</span>
                  <span class="font-semibold text-main truncate">Global Canvas Flowsheet</span>
                </div>
                <span class="badge badge-dim">${unassignedEquipment.length} Unassigned</span>
              </div>
              ${
                isNodeOpen("unassigned-flowsheet")
                  ? `
                <div class="tree-children pl-4 py-0.5 space-y-0.5">
                  <div class="tree-item flex items-center justify-between px-2 py-1 hover:bg-slate-800 rounded cursor-pointer" data-action="view-canvas">
                    <span class="text-dim">Focus All Flowsheet</span>
                    <span class="text-xs text-info">Fit</span>
                  </div>
                  ${unassignedEquipment
                    .map(
                      (eq) => `
                    <div class="tree-item flex items-center justify-between px-2 py-0.5 hover:bg-slate-800 rounded cursor-pointer" data-select-eq="${eq.id}">
                      <div class="flex items-center gap-1.5">
                        <span class="text-dim">▪</span>
                        <span class="mono font-medium text-main">${eq.id}</span>
                      </div>
                      <span class="text-dim mono text-xs">${eq.name}</span>
                    </div>
                  `
                    )
                    .join("")}
                </div>
              `
                  : ""
              }
            </div>
          `
              : ""
          }

          <!-- 3. VIRTUAL CONTROLLERS & IEC ROUTINES -->
          <div class="tree-group border-t border-[#1e293b] pt-1">
            <div class="tree-header flex items-center justify-between px-2 py-1 hover:bg-slate-800 rounded cursor-pointer group" data-node="controllers">
              <div class="flex items-center gap-1.5 flex-1">
                <span class="tree-toggle text-dim font-bold">${isNodeOpen("controllers") ? "▼" : "▶"}</span>
                <span class="tree-icon text-good">🎮</span>
                <span class="font-semibold text-main">Controllers</span>
              </div>
              <span class="badge badge-dim">${controllers.length}</span>
            </div>
            ${
              isNodeOpen("controllers")
                ? `
              <div class="tree-children pl-4 py-0.5 space-y-0.5">
                ${
                  controllers.length === 0
                    ? `<div class="px-2 py-1 text-dim italic">No controllers configured</div>`
                    : controllers
                        .map((c) => {
                          const cNode = `ctrl-${c.id}`;
                          return `
                    <div class="tree-subgroup">
                      <div class="tree-header flex items-center justify-between px-2 py-0.5 hover:bg-slate-800 rounded cursor-pointer" data-node="${cNode}">
                        <div class="flex items-center gap-1.5 flex-1">
                          <span class="tree-toggle text-dim">${isNodeOpen(cNode) ? "▼" : "▶"}</span>
                          <span class="mono font-semibold text-good">${c.id}</span>
                          <span class="text-dim text-xs truncate">(${c.name})</span>
                        </div>
                        <span class="badge ${c.enabled ? "badge-good" : "badge-dim"}">${c.tasks.length} Tasks</span>
                      </div>
                      ${
                        isNodeOpen(cNode)
                          ? `
                        <div class="tree-children pl-4 py-0.5 space-y-0.5">
                          ${c.tasks
                            .map((task) => `
                            <div class="tree-item flex items-center justify-between px-2 py-0.5 hover:bg-slate-800 rounded cursor-pointer" data-select-task="${task.id}" data-ctrl="${c.id}">
                              <div class="flex items-center gap-1">
                                <span class="text-dim">⏱</span>
                                <span class="mono text-main">${task.id}</span>
                              </div>
                              <span class="mono text-dim text-xs">${Math.round(task.period_seconds * 1000)}ms (P${task.priority})</span>
                            </div>
                            ${task.programs
                              .map((prog) => `
                              <div class="tree-children pl-3 space-y-0.5">
                                ${prog.routines
                                  .map((r) => `
                                  <div class="tree-item flex items-center justify-between px-2 py-0.5 hover:bg-slate-800 rounded cursor-pointer text-xs" data-select-routine="${r.name}" data-ctrl="${c.id}" data-task="${task.id}">
                                    <div class="flex items-center gap-1">
                                      <span class="text-amber-400">⚡</span>
                                      <span class="mono text-muted">${r.name}</span>
                                    </div>
                                    <span class="badge badge-info mono text-[10px]">${r.language}</span>
                                  </div>
                                `)
                                  .join("")}
                              </div>
                            `)
                              .join("")}
                          `)
                            .join("")}
                        </div>
                      `
                          : ""
                      }
                    </div>
                  `;
                        })
                        .join("")
                }
              </div>
            `
                : ""
            }
          </div>

          <!-- 4. ADVANCED PROCESS CONTROL -->
          <div class="tree-group">
            <div class="tree-header flex items-center justify-between px-2 py-1 hover:bg-slate-800 rounded cursor-pointer group" data-node="advanced_control">
              <div class="flex items-center gap-1.5 flex-1">
                <span class="tree-toggle text-dim font-bold">${isNodeOpen("advanced_control") ? "▼" : "▶"}</span>
                <span class="tree-icon text-info">🎯</span>
                <span class="font-semibold text-main">Advanced Control</span>
              </div>
              <span class="badge badge-dim">${advancedControl.length}</span>
            </div>
            ${
              isNodeOpen("advanced_control")
                ? `
              <div class="tree-children pl-4 py-0.5 space-y-0.5">
                <div class="tree-item flex items-center justify-between px-2 py-1 hover:bg-slate-800 rounded cursor-pointer" data-action="view-advanced-control">
                  <span class="text-dim">APC Overview</span>
                  <span class="text-xs text-info">Open</span>
                </div>
                ${advancedControl
                  .map(
                    (strat) => `
                  <div class="tree-item flex items-center justify-between px-2 py-0.5 hover:bg-slate-800 rounded cursor-pointer" data-select-strat="${strat.id}">
                    <div class="flex items-center gap-1.5">
                      <span class="text-dim">▪</span>
                      <span class="mono font-medium text-main truncate">${strat.name}</span>
                    </div>
                    <span class="badge badge-info text-[10px] mono">${strat.strategy_type}</span>
                  </div>
                `
                  )
                  .join("")}
              </div>
            `
                : ""
            }
          </div>

          <!-- 5. I/O HARDWARE SYSTEMS (HAL) -->
          <div class="tree-group">
            <div class="tree-header flex items-center justify-between px-2 py-1 hover:bg-slate-800 rounded cursor-pointer group" data-node="io_systems">
              <div class="flex items-center gap-1.5 flex-1">
                <span class="tree-toggle text-dim font-bold">${isNodeOpen("io_systems") ? "▼" : "▶"}</span>
                <span class="tree-icon text-amber-400">🔌</span>
                <span class="font-semibold text-main">I/O Systems (HAL)</span>
              </div>
              <span class="badge badge-dim">${ioSystems.length}</span>
            </div>
            ${
              isNodeOpen("io_systems")
                ? `
              <div class="tree-children pl-4 py-0.5 space-y-0.5">
                ${
                  ioSystems.length === 0
                    ? `<div class="px-2 py-1 text-dim italic">No I/O systems configured</div>`
                    : ioSystems
                        .map((sys) => {
                          const sNode = `io-${sys.id}`;
                          return `
                    <div class="tree-subgroup">
                      <div class="tree-header flex items-center justify-between px-2 py-0.5 hover:bg-slate-800 rounded cursor-pointer" data-node="${sNode}">
                        <div class="flex items-center gap-1.5 flex-1">
                          <span class="tree-toggle text-dim">${isNodeOpen(sNode) ? "▼" : "▶"}</span>
                          <span class="mono font-semibold text-amber-400">${sys.id}</span>
                        </div>
                        <span class="badge badge-dim">${sys.racks.length} Racks</span>
                      </div>
                      ${
                        isNodeOpen(sNode)
                          ? `
                        <div class="tree-children pl-4 py-0.5 space-y-0.5">
                          ${sys.racks
                            .map((r) => `
                            <div class="tree-item text-muted font-medium px-2 py-0.5">
                              Rack ${r.rack_number}: ${r.name}
                            </div>
                            ${r.chassis
                              .map((ch) => `
                              <div class="pl-2 space-y-0.5">
                                ${ch.modules
                                  .map((m) => `
                                  <div class="tree-item flex items-center justify-between px-2 py-0.5 hover:bg-slate-800 rounded cursor-pointer" data-select-module="${m.slot}" data-sys="${sys.id}">
                                    <div class="flex items-center gap-1">
                                      <span class="text-dim">📦</span>
                                      <span class="mono text-main">M${m.slot}: ${m.name}</span>
                                    </div>
                                    <span class="badge badge-dim mono">${m.module_type}</span>
                                  </div>
                                `)
                                  .join("")}
                              </div>
                            `)
                              .join("")}
                          `)
                            .join("")}
                        </div>
                      `
                          : ""
                      }
                    </div>
                  `;
                        })
                        .join("")
                }
              </div>
            `
                : ""
            }
          </div>

          <!-- 6. I/O MAPPINGS -->
          <div class="tree-group">
            <div class="tree-header flex items-center justify-between px-2 py-1 hover:bg-slate-800 rounded cursor-pointer group" data-action="view-mappings">
              <div class="flex items-center gap-1.5 flex-1">
                <span class="tree-icon text-info">🔀</span>
                <span class="font-semibold text-main">I/O Mappings</span>
              </div>
              <span class="badge badge-dim">${mappings.length} Active</span>
            </div>
          </div>

          <!-- 7. TAG DATABASE -->
          <div class="tree-group">
            <div class="tree-header flex items-center justify-between px-2 py-1 hover:bg-slate-800 rounded cursor-pointer group" data-action="view-tags">
              <div class="flex items-center gap-1.5 flex-1">
                <span class="tree-icon text-amber-400">🏷️</span>
                <span class="font-semibold text-main">Tag Database</span>
              </div>
              <span class="badge badge-dim">${tags.length} Tags</span>
            </div>
          </div>

          <!-- 8. CROSS-REFERENCE -->
          <div class="tree-group">
            <div class="tree-header flex items-center justify-between px-2 py-1 hover:bg-slate-800 rounded cursor-pointer group" data-action="view-cross-ref">
              <div class="flex items-center gap-1.5 flex-1">
                <span class="tree-icon text-info">🔍</span>
                <span class="font-semibold text-main">Cross-Reference</span>
              </div>
              <span class="text-dim text-xs mono">Query</span>
            </div>
          </div>
        </div>
      </div>
    `;

    this.bindEvents(proj);
  }

  private bindEvents(proj: ProjectDetailDto) {
    // Search input
    const searchInput = this.querySelector(".explorer-search") as HTMLInputElement;
    searchInput?.addEventListener("input", () => {
      this.searchQuery = searchInput.value;
      this.render();
    });

    // Structure Action buttons
    this.querySelector("[data-action='add-area']")?.addEventListener("click", () => this.addAreaPrompt(proj));
    this.querySelector("[data-action='add-system']")?.addEventListener("click", () => this.addSystemPrompt(proj));
    this.querySelector("[data-action='add-unit']")?.addEventListener("click", () => this.addUnitPrompt(proj));
    this.querySelector("[data-action='add-display']")?.addEventListener("click", () => this.addDisplayPrompt(proj));
    this.querySelector("[data-action='manage-structure']")?.addEventListener("click", () => {
      this.selectItem({ section: "plant_structure" });
    });

    this.querySelectorAll(".btn-add-disp-to-unit").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        e.stopPropagation();
        const unitId = btn.getAttribute("data-unit-id");
        this.addDisplayPrompt(proj, unitId || undefined);
      });
    });

    // Group expand/collapse toggles
    this.querySelectorAll(".tree-header[data-node]").forEach((el) => {
      el.addEventListener("click", (e) => {
        const nodeId = el.getAttribute("data-node");
        if (nodeId) this.toggleNode(nodeId, e);
      });
    });

    // Display selection & navigation
    this.querySelectorAll("[data-select-display]").forEach((el) => {
      el.addEventListener("click", () => {
        const dispId = el.getAttribute("data-select-display");
        if (!dispId) return;
        const struct = this.getStructure(proj);
        const disp = struct.displays.find((d) => d.id === dispId);
        this.selectItem({ section: "flowsheet", displayId: dispId });
        if (disp && disp.equipment_ids && disp.equipment_ids.length > 0) {
          const canvas = document.querySelector("nebu-pid-canvas") as any;
          if (canvas?.focusEntities) {
            canvas.focusEntities(disp.equipment_ids);
          }
        }
      });
    });

    // Action button clicks
    this.querySelector("[data-action='view-canvas']")?.addEventListener("click", () => {
      this.selectItem({ section: "flowsheet" });
      const canvas = document.querySelector("nebu-pid-canvas") as any;
      if (canvas?.fitToView) canvas.fitToView();
    });

    this.querySelector("[data-action='view-mappings']")?.addEventListener("click", () => {
      this.selectItem({ section: "io_mapping" });
    });

    this.querySelector("[data-action='view-tags']")?.addEventListener("click", () => {
      this.selectItem({ section: "tag" });
    });

    this.querySelector("[data-action='view-cross-ref']")?.addEventListener("click", () => {
      this.selectItem({ section: "cross_ref" });
    });

    this.querySelector("[data-action='view-advanced-control']")?.addEventListener("click", () => {
      this.selectItem({ section: "advanced_control" });
    });

    this.querySelectorAll("[data-select-strat]").forEach((el) => {
      el.addEventListener("click", () => {
        const id = el.getAttribute("data-select-strat");
        this.selectItem({ section: "advanced_control", id: id || undefined });
      });
    });

    // Equipment node click
    this.querySelectorAll("[data-select-eq]").forEach((el) => {
      el.addEventListener("click", (e) => {
        e.stopPropagation();
        const id = el.getAttribute("data-select-eq");
        if (id) {
          studioStore.selectItem("equipment", id);
          this.selectItem({ section: "flowsheet", id });
          const canvas = document.querySelector("nebu-pid-canvas") as any;
          if (canvas?.focusEntities) {
            canvas.focusEntities([id]);
          }
        }
      });
    });

    // Stream click
    this.querySelectorAll("[data-select-stream]").forEach((el) => {
      el.addEventListener("click", (e) => {
        e.stopPropagation();
        const id = el.getAttribute("data-select-stream");
        if (id) {
          studioStore.selectItem("stream", id);
          this.selectItem({ section: "flowsheet", id });
        }
      });
    });

    // Instrument click
    this.querySelectorAll("[data-select-inst]").forEach((el) => {
      el.addEventListener("click", (e) => {
        e.stopPropagation();
        const id = el.getAttribute("data-select-inst");
        if (id) {
          studioStore.selectItem("instrument", id);
          this.selectItem({ section: "flowsheet", id });
        }
      });
    });

    // Control Loop click
    this.querySelectorAll("[data-select-loop]").forEach((el) => {
      el.addEventListener("click", (e) => {
        e.stopPropagation();
        const id = el.getAttribute("data-select-loop");
        if (id) {
          studioStore.selectItem("control_loop", id);
          this.selectItem({ section: "control_loop", id });
        }
      });
    });

    // Alarm click
    this.querySelectorAll("[data-select-alarm]").forEach((el) => {
      el.addEventListener("click", (e) => {
        e.stopPropagation();
        const id = el.getAttribute("data-select-alarm");
        if (id) {
          studioStore.selectItem("alarm", id);
          this.selectItem({ section: "alarm", id });
        }
      });
    });

    // Task click
    this.querySelectorAll("[data-select-task]").forEach((el) => {
      el.addEventListener("click", () => {
        const taskId = el.getAttribute("data-select-task");
        const ctrlId = el.getAttribute("data-ctrl");
        this.selectItem({ section: "task", id: ctrlId || undefined, subId: taskId || undefined });
      });
    });

    // Routine click
    this.querySelectorAll("[data-select-routine]").forEach((el) => {
      el.addEventListener("click", () => {
        const rId = el.getAttribute("data-select-routine");
        const ctrlId = el.getAttribute("data-ctrl");
        const taskId = el.getAttribute("data-task");
        this.selectItem({ section: "routine", id: ctrlId || undefined, subId: rId || undefined, extra: { taskId } });
      });
    });

    // I/O Module click
    this.querySelectorAll("[data-select-module]").forEach((el) => {
      el.addEventListener("click", () => {
        const slot = el.getAttribute("data-select-module");
        const sysId = el.getAttribute("data-sys");
        this.selectItem({ section: "io_module", id: sysId || undefined, subId: slot || undefined });
      });
    });
  }
}

customElements.define("nebu-project-explorer", NebuProjectExplorer);
