/**
 * NebuDCS Engineering Studio Plant Structure & Display Hierarchy Editor.
 *
 * Configures the ISA-95 / ISA-101 plant hierarchy and displays:
 * Plant -> Area -> System -> Subsystem -> Unit -> Display.
 *
 * Provides full CRUD operations on:
 * - Areas
 * - Systems
 * - Units
 * - Displays (Plant Overview, Area Overview, System/Unit Graphics, Group Displays, etc.)
 * - Equipment & Instrument display assignments
 */

import { store } from "../state/store";
import { studioStore } from "../state/studio-store";
import {
  DisplayDto,
  DisplayTypeDto,
  PlantAreaDto,
  PlantStructureDto,
  PlantSystemDto,
  PlantUnitDto,
} from "../protocol/types";

export class NebuPlantStructureEditor extends HTMLElement {
  private selectedNodeId: string | null = null;
  private selectedNodeType: "plant" | "area" | "system" | "unit" | "display" | null = "plant";
  private unsubscribe?: () => void;

  connectedCallback() {
    this.render();
    this.unsubscribe = studioStore.subscribe(() => this.render());
  }

  disconnectedCallback() {
    if (this.unsubscribe) {
      this.unsubscribe();
    }
  }

  private getStructure(): PlantStructureDto {
    const proj = studioStore.getProject() || store.activeProject;
    if (proj?.plant_structure) {
      return proj.plant_structure;
    }
    if (proj) {
      const fallback = store.buildDefaultPlantStructure(proj);
      proj.plant_structure = fallback;
      return fallback;
    }
    return {
      plant_name: "Default Industrial Plant",
      areas: [],
      displays: [],
    };
  }

  private saveStructure(struct: PlantStructureDto) {
    const proj = studioStore.getProject() || store.activeProject;
    if (proj) {
      proj.plant_structure = struct;
      proj.displays = struct.displays;
      proj.areas = struct.areas;
      store.plantStructure = struct;
      store.displays = struct.displays;
      studioStore.setProject(proj, false);
      store.notify();
      this.render();
    }
  }

  private addArea() {
    const struct = this.getStructure();
    const id = `area-${Date.now().toString(36).slice(-4)}`;
    const newArea: PlantAreaDto = {
      id,
      name: `Area ${struct.areas.length + 1} - New Process Area`,
      description: "Industrial process and manufacturing area",
      systems: [],
    };
    struct.areas.push(newArea);
    this.selectedNodeType = "area";
    this.selectedNodeId = id;
    this.saveStructure(struct);
  }

  private addSystem(areaId: string) {
    const struct = this.getStructure();
    const area = struct.areas.find((a) => a.id === areaId);
    if (!area) return;

    const id = `sys-${Date.now().toString(36).slice(-4)}`;
    const newSys: PlantSystemDto = {
      id,
      name: `System ${area.systems.length + 1} - Process System`,
      area_id: areaId,
      description: "Process equipment train and regulatory subsystem",
      subsystems: [],
    };
    area.systems.push(newSys);
    this.selectedNodeType = "system";
    this.selectedNodeId = id;
    this.saveStructure(struct);
  }

  private addUnit(areaId: string, sysId: string) {
    const struct = this.getStructure();
    const area = struct.areas.find((a) => a.id === areaId);
    const sys = area?.systems.find((s) => s.id === sysId);
    if (!sys) return;

    if (sys.subsystems.length === 0) {
      sys.subsystems.push({
        id: `subsys-1`,
        name: "Default Subsystem",
        system_id: sysId,
        units: [],
      });
    }

    const id = `unit-${Date.now().toString(36).slice(-4)}`;
    const newUnit: PlantUnitDto = {
      id,
      name: `Unit ${sys.subsystems[0].units.length + 1}`,
      subsystem_id: sys.subsystems[0].id,
      system_id: sysId,
      area_id: areaId,
      equipment_ids: [],
      display_ids: [],
      description: "Continuous operating unit",
    };
    sys.subsystems[0].units.push(newUnit);
    this.selectedNodeType = "unit";
    this.selectedNodeId = id;
    this.saveStructure(struct);
  }

  private addDisplay() {
    const struct = this.getStructure();
    const proj = studioStore.getProject() || store.activeProject;
    const allEqIds = proj?.equipment.map((e) => e.id) || [];
    const allInstIds = proj?.instruments.map((i) => i.tag) || [];
    const allStreamIds = proj?.streams.map((s) => s.name) || [];

    const id = `disp-${Date.now().toString(36).slice(-4)}`;
    const newDisp: DisplayDto = {
      id,
      name: `Display ${struct.displays.length + 1}`,
      display_type: "unit_process_graphic",
      area_id: struct.areas[0]?.id || "area-01",
      equipment_ids: allEqIds,
      instrument_ids: allInstIds,
      stream_ids: allStreamIds,
      is_default: false,
      order: struct.displays.length + 1,
      description: "Custom operator process graphic display",
    };
    struct.displays.push(newDisp);
    this.selectedNodeType = "display";
    this.selectedNodeId = id;
    this.saveStructure(struct);
  }

  private deleteSelected() {
    if (!this.selectedNodeId || !this.selectedNodeType) return;
    const struct = this.getStructure();

    if (this.selectedNodeType === "area") {
      struct.areas = struct.areas.filter((a) => a.id !== this.selectedNodeId);
      this.selectedNodeType = "plant";
      this.selectedNodeId = null;
    } else if (this.selectedNodeType === "system") {
      for (const a of struct.areas) {
        a.systems = a.systems.filter((s) => s.id !== this.selectedNodeId);
      }
      this.selectedNodeType = "plant";
      this.selectedNodeId = null;
    } else if (this.selectedNodeType === "unit") {
      for (const a of struct.areas) {
        for (const s of a.systems) {
          for (const sub of s.subsystems) {
            sub.units = sub.units.filter((u) => u.id !== this.selectedNodeId);
          }
        }
      }
      this.selectedNodeType = "plant";
      this.selectedNodeId = null;
    } else if (this.selectedNodeType === "display") {
      struct.displays = struct.displays.filter((d) => d.id !== this.selectedNodeId);
      this.selectedNodeType = "plant";
      this.selectedNodeId = null;
    }

    this.saveStructure(struct);
  }

  private resetToDefault() {
    const proj = studioStore.getProject() || store.activeProject;
    if (proj) {
      const def = store.buildDefaultPlantStructure(proj);
      this.saveStructure(def);
    }
  }

  render() {
    const struct = this.getStructure();
    const proj = studioStore.getProject() || store.activeProject;
    const eqList = proj?.equipment || [];

    // Render tree nodes
    const areasHtml = struct.areas
      .map((a) => {
        const isAreaSelected = this.selectedNodeType === "area" && this.selectedNodeId === a.id;
        const systemsHtml = a.systems
          .map((s) => {
            const isSysSelected = this.selectedNodeType === "system" && this.selectedNodeId === s.id;
            const unitsHtml = s.subsystems
              .flatMap((sub) => sub.units)
              .map((u) => {
                const isUnitSelected = this.selectedNodeType === "unit" && this.selectedNodeId === u.id;
                return `
                  <li class="tree-leaf ${isUnitSelected ? "selected" : ""}" data-type="unit" data-id="${u.id}">
                    <span class="tree-icon">⛯</span>
                    <span class="tree-label">${u.name}</span>
                    <span class="badge badge-secondary mono" style="font-size:8px;">${u.equipment_ids.length} EQ</span>
                  </li>
                `;
              })
              .join("");

            return `
              <li class="tree-branch ${isSysSelected ? "selected" : ""}" data-type="system" data-id="${s.id}">
                <div class="tree-node-row">
                  <span class="tree-icon">⚙</span>
                  <span class="tree-label font-bold">${s.name}</span>
                </div>
                <ul class="tree-subgroup">
                  ${unitsHtml}
                </ul>
              </li>
            `;
          })
          .join("");

        return `
          <li class="tree-branch ${isAreaSelected ? "selected" : ""}" data-type="area" data-id="${a.id}">
            <div class="tree-node-row">
              <span class="tree-icon">🏭</span>
              <span class="tree-label font-bold text-info">${a.name}</span>
            </div>
            <ul class="tree-subgroup">
              ${systemsHtml}
            </ul>
          </li>
        `;
      })
      .join("");

    const displaysHtml = struct.displays
      .map((d) => {
        const isDispSelected = this.selectedNodeType === "display" && this.selectedNodeId === d.id;
        const typeBadge =
          d.display_type === "plant_overview"
            ? "badge-warn"
            : d.display_type === "area_overview"
            ? "badge-info"
            : d.display_type === "group_display"
            ? "badge-good"
            : "badge-secondary";

        return `
          <li class="tree-leaf ${isDispSelected ? "selected" : ""}" data-type="display" data-id="${d.id}">
            <span class="tree-icon">🖵</span>
            <span class="tree-label font-bold">${d.name}</span>
            <span class="badge ${typeBadge} mono" style="font-size:8px;">${d.display_type}</span>
            ${d.is_default ? `<span class="badge badge-accent" style="font-size:8px;">DEF</span>` : ""}
          </li>
        `;
      })
      .join("");

    // Render inspector pane for selected item
    let inspectorHtml = "";
    if (this.selectedNodeType === "plant") {
      inspectorHtml = `
        <div class="inspector-form mono">
          <div class="form-header">
            <span class="font-bold text-accent">PLANT ROOT CONFIGURATION</span>
          </div>
          <div class="form-group">
            <label>Plant Name</label>
            <input type="text" id="inpPlantName" value="${struct.plant_name}" class="form-input"/>
          </div>
          <div class="form-summary">
            <div>Configured Areas: <strong>${struct.areas.length}</strong></div>
            <div>Configured Displays: <strong>${struct.displays.length}</strong></div>
            <div>Total Flowsheet Equipment: <strong>${eqList.length}</strong></div>
          </div>
        </div>
      `;
    } else if (this.selectedNodeType === "area") {
      const area = struct.areas.find((a) => a.id === this.selectedNodeId);
      if (area) {
        inspectorHtml = `
          <div class="inspector-form mono">
            <div class="form-header">
              <span class="font-bold text-info">AREA CONFIGURATION</span>
              <span class="badge badge-info">${area.id}</span>
            </div>
            <div class="form-group">
              <label>Area Name</label>
              <input type="text" id="inpAreaName" value="${area.name}" class="form-input"/>
            </div>
            <div class="form-group">
              <label>Description</label>
              <textarea id="inpAreaDesc" class="form-input" rows="3">${area.description || ""}</textarea>
            </div>
            <div class="form-actions-inline">
              <button class="btn btn-sm btn-primary" id="btnAddSysUnderArea">+ Add System</button>
            </div>
          </div>
        `;
      }
    } else if (this.selectedNodeType === "system") {
      let foundSys: PlantSystemDto | undefined;
      let parentArea: PlantAreaDto | undefined;
      for (const a of struct.areas) {
        const s = a.systems.find((sys) => sys.id === this.selectedNodeId);
        if (s) {
          foundSys = s;
          parentArea = a;
          break;
        }
      }

      if (foundSys && parentArea) {
        inspectorHtml = `
          <div class="inspector-form mono">
            <div class="form-header">
              <span class="font-bold text-accent">PROCESS SYSTEM CONFIGURATION</span>
              <span class="badge badge-secondary">${foundSys.id}</span>
            </div>
            <div class="form-group">
              <label>System Name</label>
              <input type="text" id="inpSysName" value="${foundSys.name}" class="form-input"/>
            </div>
            <div class="form-group">
              <label>Parent Area</label>
              <input type="text" value="${parentArea.name}" disabled class="form-input text-dim"/>
            </div>
            <div class="form-group">
              <label>Description</label>
              <textarea id="inpSysDesc" class="form-input" rows="3">${foundSys.description || ""}</textarea>
            </div>
            <div class="form-actions-inline">
              <button class="btn btn-sm btn-primary" id="btnAddUnitUnderSys">+ Add Unit</button>
            </div>
          </div>
        `;
      }
    } else if (this.selectedNodeType === "unit") {
      let foundUnit: PlantUnitDto | undefined;
      for (const a of struct.areas) {
        for (const s of a.systems) {
          for (const sub of s.subsystems) {
            const u = sub.units.find((unit) => unit.id === this.selectedNodeId);
            if (u) {
              foundUnit = u;
              break;
            }
          }
        }
      }

      if (foundUnit) {
        const eqCheckboxes = eqList
          .map((eq) => {
            const isAssigned = foundUnit!.equipment_ids.includes(eq.id);
            return `
              <label class="eq-assign-label font-small">
                <input type="checkbox" class="chk-assign-eq" data-eq-id="${eq.id}" ${isAssigned ? "checked" : ""}/>
                <span>${eq.name} (${eq.id})</span>
              </label>
            `;
          })
          .join("");

        inspectorHtml = `
          <div class="inspector-form mono">
            <div class="form-header">
              <span class="font-bold text-good">UNIT CONFIGURATION</span>
              <span class="badge badge-secondary">${foundUnit.id}</span>
            </div>
            <div class="form-group">
              <label>Unit Name</label>
              <input type="text" id="inpUnitName" value="${foundUnit.name}" class="form-input"/>
            </div>
            <div class="form-group">
              <label>Description</label>
              <textarea id="inpUnitDesc" class="form-input" rows="2">${foundUnit.description || ""}</textarea>
            </div>
            <div class="form-group">
              <label>Assigned Equipment</label>
              <div class="eq-assign-grid">
                ${eqCheckboxes || `<span class="text-dim">No equipment available in project.</span>`}
              </div>
            </div>
          </div>
        `;
      }
    } else if (this.selectedNodeType === "display") {
      const disp = struct.displays.find((d) => d.id === this.selectedNodeId);
      if (disp) {
        const typeOptions: DisplayTypeDto[] = [
          "plant_overview",
          "area_overview",
          "system_overview",
          "unit_process_graphic",
          "subsystem_graphic",
          "group_display",
          "point_detail",
          "trend",
          "alarms",
          "diagnostics",
        ];

        const optsHtml = typeOptions
          .map((t) => `<option value="${t}" ${disp.display_type === t ? "selected" : ""}>${t.toUpperCase()}</option>`)
          .join("");

        const eqCheckboxes = eqList
          .map((eq) => {
            const isAssigned = disp.equipment_ids.includes(eq.id);
            return `
              <label class="eq-assign-label font-small">
                <input type="checkbox" class="chk-disp-eq" data-eq-id="${eq.id}" ${isAssigned ? "checked" : ""}/>
                <span>${eq.name} (${eq.id})</span>
              </label>
            `;
          })
          .join("");

        inspectorHtml = `
          <div class="inspector-form mono">
            <div class="form-header">
              <span class="font-bold text-accent">DCS DISPLAY CONFIGURATION</span>
              <span class="badge badge-secondary">${disp.id}</span>
            </div>
            <div class="form-group">
              <label>Display Title</label>
              <input type="text" id="inpDispName" value="${disp.name}" class="form-input"/>
            </div>
            <div class="form-group">
              <label>Display Taxonomy Type</label>
              <select id="selDispType" class="form-input mono">
                ${optsHtml}
              </select>
            </div>
            <div class="form-group">
              <label>Default Station Display</label>
              <div style="display:flex;align-items:center;gap:8px;">
                <input type="checkbox" id="chkDispDefault" ${disp.is_default ? "checked" : ""}/>
                <span class="text-dim">Auto-open this display when operator selects station</span>
              </div>
            </div>
            <div class="form-group">
              <label>Description</label>
              <textarea id="inpDispDesc" class="form-input" rows="2">${disp.description || ""}</textarea>
            </div>
            <div class="form-group">
              <label>Equipment Scope Filter</label>
              <div class="eq-assign-grid">
                ${eqCheckboxes || `<span class="text-dim">All equipment included by default.</span>`}
              </div>
            </div>
          </div>
        `;
      }
    }

    this.innerHTML = `
      <div class="plant-structure-editor-container">
        <!-- Top Toolbar -->
        <div class="pse-toolbar mono">
          <div class="pse-toolbar-left">
            <span class="font-bold text-accent">PLANT STRUCTURE &amp; DCS DISPLAY ARCHITECTURE</span>
            <span class="badge badge-info">ISA-95 / ISA-101</span>
          </div>
          <div class="pse-toolbar-right">
            <button class="btn btn-sm" id="btnAddArea">+ Area</button>
            <button class="btn btn-sm" id="btnAddDisplay">+ Display</button>
            <button class="btn btn-sm btn-warn" id="btnDeleteSelected">Delete</button>
            <button class="btn btn-sm btn-secondary" id="btnResetDefault">Reset Defaults</button>
          </div>
        </div>

        <!-- Split Body: Left Tree, Right Inspector -->
        <div class="pse-body">
          <div class="pse-tree-pane mono">
            <div class="tree-section-title">
              <span class="font-bold">PLANT TOPOLOGY TREE</span>
            </div>
            <ul class="tree-root">
              <li class="tree-branch ${this.selectedNodeType === "plant" ? "selected" : ""}" data-type="plant">
                <div class="tree-node-row">
                  <span class="tree-icon">🌐</span>
                  <span class="tree-label font-bold text-accent">${struct.plant_name}</span>
                </div>
                <ul class="tree-subgroup">
                  ${areasHtml}
                </ul>
              </li>
            </ul>

            <div class="tree-section-title" style="margin-top:16px;">
              <span class="font-bold">CONFIGURED DCS DISPLAYS (${struct.displays.length})</span>
            </div>
            <ul class="tree-displays-list">
              ${displaysHtml}
            </ul>
          </div>

          <div class="pse-inspector-pane">
            ${inspectorHtml}
          </div>
        </div>
      </div>
    `;

    this.bindEvents();
  }

  private bindEvents() {
    // Tree item selection
    this.querySelectorAll("[data-type]").forEach((el) => {
      el.addEventListener("click", (e) => {
        e.stopPropagation();
        const type = el.getAttribute("data-type") as any;
        const id = el.getAttribute("data-id");
        this.selectedNodeType = type;
        this.selectedNodeId = id || null;
        this.render();
      });
    });

    // Toolbar buttons
    this.querySelector("#btnAddArea")?.addEventListener("click", () => this.addArea());
    this.querySelector("#btnAddDisplay")?.addEventListener("click", () => this.addDisplay());
    this.querySelector("#btnDeleteSelected")?.addEventListener("click", () => this.deleteSelected());
    this.querySelector("#btnResetDefault")?.addEventListener("click", () => this.resetToDefault());

    // Inspector inputs: Plant
    const inpPlant = this.querySelector("#inpPlantName") as HTMLInputElement;
    inpPlant?.addEventListener("change", () => {
      const struct = this.getStructure();
      struct.plant_name = inpPlant.value;
      this.saveStructure(struct);
    });

    // Inspector inputs: Area
    const inpAreaName = this.querySelector("#inpAreaName") as HTMLInputElement;
    const inpAreaDesc = this.querySelector("#inpAreaDesc") as HTMLTextAreaElement;
    inpAreaName?.addEventListener("change", () => {
      const struct = this.getStructure();
      const a = struct.areas.find((ar) => ar.id === this.selectedNodeId);
      if (a) {
        a.name = inpAreaName.value;
        this.saveStructure(struct);
      }
    });
    inpAreaDesc?.addEventListener("change", () => {
      const struct = this.getStructure();
      const a = struct.areas.find((ar) => ar.id === this.selectedNodeId);
      if (a) {
        a.description = inpAreaDesc.value;
        this.saveStructure(struct);
      }
    });
    this.querySelector("#btnAddSysUnderArea")?.addEventListener("click", () => {
      if (this.selectedNodeId) this.addSystem(this.selectedNodeId);
    });

    // Inspector inputs: System
    const inpSysName = this.querySelector("#inpSysName") as HTMLInputElement;
    const inpSysDesc = this.querySelector("#inpSysDesc") as HTMLTextAreaElement;
    inpSysName?.addEventListener("change", () => {
      const struct = this.getStructure();
      for (const a of struct.areas) {
        const s = a.systems.find((sys) => sys.id === this.selectedNodeId);
        if (s) {
          s.name = inpSysName.value;
          this.saveStructure(struct);
          break;
        }
      }
    });
    inpSysDesc?.addEventListener("change", () => {
      const struct = this.getStructure();
      for (const a of struct.areas) {
        const s = a.systems.find((sys) => sys.id === this.selectedNodeId);
        if (s) {
          s.description = inpSysDesc.value;
          this.saveStructure(struct);
          break;
        }
      }
    });
    this.querySelector("#btnAddUnitUnderSys")?.addEventListener("click", () => {
      const struct = this.getStructure();
      for (const a of struct.areas) {
        const s = a.systems.find((sys) => sys.id === this.selectedNodeId);
        if (s) {
          this.addUnit(a.id, s.id);
          break;
        }
      }
    });

    // Inspector inputs: Unit
    const inpUnitName = this.querySelector("#inpUnitName") as HTMLInputElement;
    const inpUnitDesc = this.querySelector("#inpUnitDesc") as HTMLTextAreaElement;
    inpUnitName?.addEventListener("change", () => {
      const struct = this.getStructure();
      for (const a of struct.areas) {
        for (const s of a.systems) {
          for (const sub of s.subsystems) {
            const u = sub.units.find((unit) => unit.id === this.selectedNodeId);
            if (u) {
              u.name = inpUnitName.value;
              this.saveStructure(struct);
              return;
            }
          }
        }
      }
    });
    inpUnitDesc?.addEventListener("change", () => {
      const struct = this.getStructure();
      for (const a of struct.areas) {
        for (const s of a.systems) {
          for (const sub of s.subsystems) {
            const u = sub.units.find((unit) => unit.id === this.selectedNodeId);
            if (u) {
              u.description = inpUnitDesc.value;
              this.saveStructure(struct);
              return;
            }
          }
        }
      }
    });
    this.querySelectorAll(".chk-assign-eq").forEach((chk) => {
      chk.addEventListener("change", (e: any) => {
        const eqId = chk.getAttribute("data-eq-id");
        if (!eqId) return;
        const struct = this.getStructure();
        for (const a of struct.areas) {
          for (const s of a.systems) {
            for (const sub of s.subsystems) {
              const u = sub.units.find((unit) => unit.id === this.selectedNodeId);
              if (u) {
                if (e.target.checked) {
                  if (!u.equipment_ids.includes(eqId)) u.equipment_ids.push(eqId);
                } else {
                  u.equipment_ids = u.equipment_ids.filter((id) => id !== eqId);
                }
                this.saveStructure(struct);
                return;
              }
            }
          }
        }
      });
    });

    // Inspector inputs: Display
    const inpDispName = this.querySelector("#inpDispName") as HTMLInputElement;
    const selDispType = this.querySelector("#selDispType") as HTMLSelectElement;
    const chkDispDefault = this.querySelector("#chkDispDefault") as HTMLInputElement;
    const inpDispDesc = this.querySelector("#inpDispDesc") as HTMLTextAreaElement;

    inpDispName?.addEventListener("change", () => {
      const struct = this.getStructure();
      const d = struct.displays.find((disp) => disp.id === this.selectedNodeId);
      if (d) {
        d.name = inpDispName.value;
        this.saveStructure(struct);
      }
    });
    selDispType?.addEventListener("change", () => {
      const struct = this.getStructure();
      const d = struct.displays.find((disp) => disp.id === this.selectedNodeId);
      if (d) {
        d.display_type = selDispType.value as any;
        this.saveStructure(struct);
      }
    });
    chkDispDefault?.addEventListener("change", () => {
      const struct = this.getStructure();
      const d = struct.displays.find((disp) => disp.id === this.selectedNodeId);
      if (d) {
        if (chkDispDefault.checked) {
          struct.displays.forEach((other) => (other.is_default = false));
          d.is_default = true;
        } else {
          d.is_default = false;
        }
        this.saveStructure(struct);
      }
    });
    inpDispDesc?.addEventListener("change", () => {
      const struct = this.getStructure();
      const d = struct.displays.find((disp) => disp.id === this.selectedNodeId);
      if (d) {
        d.description = inpDispDesc.value;
        this.saveStructure(struct);
      }
    });
    this.querySelectorAll(".chk-disp-eq").forEach((chk) => {
      chk.addEventListener("change", (e: any) => {
        const eqId = chk.getAttribute("data-eq-id");
        if (!eqId) return;
        const struct = this.getStructure();
        const d = struct.displays.find((disp) => disp.id === this.selectedNodeId);
        if (d) {
          if (e.target.checked) {
            if (!d.equipment_ids.includes(eqId)) d.equipment_ids.push(eqId);
          } else {
            d.equipment_ids = d.equipment_ids.filter((id) => id !== eqId);
          }
          this.saveStructure(struct);
        }
      });
    });
  }
}

customElements.define("nebu-plant-structure-editor", NebuPlantStructureEditor);
