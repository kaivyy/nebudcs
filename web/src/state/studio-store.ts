/**
 * Studio State Store & Command-Based Undo/Redo Engine.
 *
 * Implements authoritative client-side project state management,
 * transactional canvas mutations, and command history.
 */

import {
  AdvancedControlStrategyDto,
  BoundaryConditionDto,
  NavigationLinkDto,
  Position2dDto,
  ProjectDetailDto,
  ProjectValidationResponseDto,
  StudioAlarmDto,
  StudioControlLoopDto,
  StudioEquipmentDto,
  StudioInstrumentDto,
  StudioInterlockDto,
  StudioStreamDto,
  TemplateSummaryDto,
} from "../protocol/types";
import { getUniversalTemplateSummaries } from "../domain/equipment-template-catalog";
import { generateAutoLayout, validateLayoutGeometry, GeometryValidationIssue } from "../domain/geometry-collision";

export type SelectableType =
  | "equipment"
  | "stream"
  | "instrument"
  | "control_loop"
  | "alarm"
  | "interlock"
  | "boundary_condition";

export interface SelectedEntity {
  type: SelectableType;
  id: string;
}

export interface ConnectionDraft {
  sourceEquipmentId: string;
  sourcePortId: string;
  portName?: string;
}

export interface StudioCommand {
  description: string;
  execute: (project: ProjectDetailDto) => ProjectDetailDto;
  undo: (project: ProjectDetailDto) => ProjectDetailDto;
}

export class StudioStore {
  private project: ProjectDetailDto | null = null;
  private isDirty = false;
  private selected: SelectedEntity | null = null;
  private selectedIds: Set<string> = new Set();
  private connectionDraft: ConnectionDraft | null = null;
  private validation: ProjectValidationResponseDto | null = null;
  private templates: TemplateSummaryDto[] = getUniversalTemplateSummaries();

  private undoStack: StudioCommand[] = [];
  private redoStack: StudioCommand[] = [];
  private listeners: Set<() => void> = new Set();

  constructor() {}

  // -------------------------------------------------------------------------
  // Subscriptions & Change Notifications
  // -------------------------------------------------------------------------

  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify(): void {
    for (const listener of this.listeners) {
      listener();
    }
  }

  // -------------------------------------------------------------------------
  // State Accessors
  // -------------------------------------------------------------------------

  getProject(): ProjectDetailDto | null {
    return this.project;
  }

  getIsDirty(): boolean {
    return this.isDirty;
  }

  getSelected(): SelectedEntity | null {
    return this.selected;
  }

  getSelectedIds(): Set<string> {
    return this.selectedIds;
  }

  isSelected(id: string): boolean {
    return this.selectedIds.has(id);
  }

  getConnectionDraft(): ConnectionDraft | null {
    return this.connectionDraft;
  }

  getValidation(): ProjectValidationResponseDto | null {
    return this.validation;
  }

  getTemplates(): TemplateSummaryDto[] {
    return this.templates;
  }

  canUndo(): boolean {
    return this.undoStack.length > 0;
  }

  canRedo(): boolean {
    return this.redoStack.length > 0;
  }

  // -------------------------------------------------------------------------
  // Project Lifecycle Mutators
  // -------------------------------------------------------------------------

  setProject(project: ProjectDetailDto, markClean = true): void {
    this.project = JSON.parse(JSON.stringify(project));
    if (markClean) {
      this.isDirty = false;
      this.undoStack = [];
      this.redoStack = [];
    }
    this.selected = null;
    this.selectedIds.clear();
    this.connectionDraft = null;
    this.notify();
  }

  setTemplates(templates: TemplateSummaryDto[], mergeWithUniversal = false): void {
    if (mergeWithUniversal) {
      const universal = getUniversalTemplateSummaries();
      const mergedMap = new Map<string, TemplateSummaryDto>();
      for (const u of universal) {
        mergedMap.set(u.id, u);
      }
      for (const s of (templates || [])) {
        mergedMap.set(s.id, s);
      }
      this.templates = Array.from(mergedMap.values());
    } else {
      this.templates = templates;
    }
    this.notify();
  }

  addCustomTemplate(tmpl: TemplateSummaryDto): void {
    const idx = this.templates.findIndex((t) => t.id === tmpl.id);
    if (idx >= 0) {
      this.templates[idx] = tmpl;
    } else {
      this.templates.push(tmpl);
    }
    this.notify();
  }

  setValidation(validation: ProjectValidationResponseDto | null): void {
    this.validation = validation;
    this.notify();
  }

  selectItem(type: SelectableType, id: string, multi = false, toggle = false): void {
    if (toggle) {
      if (this.selectedIds.has(id)) {
        this.selectedIds.delete(id);
        if (this.selected?.id === id) {
          const nextId = Array.from(this.selectedIds)[0];
          this.selected = nextId ? { type, id: nextId } : null;
        }
      } else {
        this.selectedIds.add(id);
        this.selected = { type, id };
      }
    } else if (multi) {
      this.selectedIds.add(id);
      this.selected = { type, id };
    } else {
      this.selectedIds.clear();
      this.selectedIds.add(id);
      this.selected = { type, id };
    }
    this.notify();
  }

  selectMultiple(items: Array<{ type: SelectableType; id: string }>): void {
    this.selectedIds.clear();
    for (const item of items) {
      this.selectedIds.add(item.id);
    }
    this.selected = items[0] ? { ...items[0] } : null;
    this.notify();
  }

  selectAllEquipment(): void {
    if (!this.project) return;
    this.selectedIds.clear();
    for (const eq of this.project.equipment) {
      this.selectedIds.add(eq.id);
    }
    const first = this.project.equipment[0];
    this.selected = first ? { type: "equipment", id: first.id } : null;
    this.notify();
  }

  clearSelection(): void {
    this.selected = null;
    this.selectedIds.clear();
    this.notify();
  }

  startConnection(sourceEquipmentId: string, sourcePortId: string, portName?: string): void {
    this.connectionDraft = { sourceEquipmentId, sourcePortId, portName };
    this.notify();
  }

  cancelConnection(): void {
    this.connectionDraft = null;
    this.notify();
  }

  // -------------------------------------------------------------------------
  // Transactional Command Execution (Undo / Redo)
  // -------------------------------------------------------------------------

  execute(command: StudioCommand): void {
    if (!this.project) return;
    this.project = command.execute(this.project);
    this.undoStack.push(command);
    this.redoStack = [];
    this.isDirty = true;
    this.notify();
  }

  undo(): boolean {
    if (!this.project || this.undoStack.length === 0) return false;
    const cmd = this.undoStack.pop()!;
    this.project = cmd.undo(this.project);
    this.redoStack.push(cmd);
    this.isDirty = true;
    this.notify();
    return true;
  }

  redo(): boolean {
    if (!this.project || this.redoStack.length === 0) return false;
    const cmd = this.redoStack.pop()!;
    this.project = cmd.execute(this.project);
    this.undoStack.push(cmd);
    this.isDirty = true;
    this.notify();
    return true;
  }

  // -------------------------------------------------------------------------
  // High-Level Studio Actions
  // -------------------------------------------------------------------------

  addEquipment(equipment: StudioEquipmentDto): void {
    const cmd: StudioCommand = {
      description: `Add Equipment ${equipment.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.equipment.push(equipment);
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.equipment = next.equipment.filter((e) => e.id !== equipment.id);
        return next;
      },
    };
    this.execute(cmd);
    this.selectItem("equipment", equipment.id);
  }

  updateEquipment(id: string, updates: Partial<StudioEquipmentDto>): void {
    if (!this.project) return;
    const eq = this.project.equipment.find((e) => e.id === id);
    if (!eq) return;
    const oldState = { ...eq };

    const cmd: StudioCommand = {
      description: `Update Equipment ${eq.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const target = next.equipment.find((e) => e.id === id);
        if (target) {
          Object.assign(target, updates);
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const target = next.equipment.find((e) => e.id === id);
        if (target) {
          Object.assign(target, oldState);
        }
        return next;
      },
    };
    this.execute(cmd);
  }

  moveEquipment(id: string, newPosition: Position2dDto, targetDisplayId?: string): void {
    this.moveEntities([{ id, type: "equipment", pos: newPosition }], targetDisplayId);
  }

  moveInstrument(id: string, newPosition: Position2dDto): void {
    if (!this.project) return;
    const inst = this.project.instruments.find((i) => i.id === id);
    if (!inst) return;
    const oldPosition = inst.position ? { ...inst.position } : undefined;

    const cmd: StudioCommand = {
      description: `Move ${inst.tag}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const target = next.instruments.find((i) => i.id === id);
        if (target) {
          target.position = { ...newPosition };
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const target = next.instruments.find((i) => i.id === id);
        if (target) {
          target.position = oldPosition ? { ...oldPosition } : undefined;
        }
        return next;
      },
    };
    this.execute(cmd);
  }

  moveEntities(
    moves: Array<{ id: string; type: SelectableType; pos: Position2dDto }>,
    targetDisplayId?: string
  ): void {
    if (!this.project || moves.length === 0) return;
    const oldPositions: Map<string, Position2dDto | undefined> = new Map();
    const affectedStreamOldRoutes: Array<{ id: string; points: Position2dDto[] }> = [];

    const isDisplayScoped = Boolean(targetDisplayId && targetDisplayId !== "all");
    const movedEqIds = new Set(moves.filter((m) => m.type === "equipment").map((m) => m.id));

    const allDisplays = (this.project.displays && this.project.displays.length > 0
      ? this.project.displays
      : this.project.plant_structure?.displays) || [];

    for (const m of moves) {
      if (m.type === "equipment") {
        if (isDisplayScoped) {
          const disp = allDisplays.find((d) => d.id === targetDisplayId);
          const pl = disp?.placements?.find((p: any) => p.equipment_id === m.id);
          if (pl) {
            oldPositions.set(m.id, { x: pl.x, y: pl.y });
          } else {
            const eq = this.project.equipment.find((e) => e.id === m.id);
            oldPositions.set(m.id, eq?.position ? { ...eq.position } : undefined);
          }
        } else {
          const eq = this.project.equipment.find((e) => e.id === m.id);
          oldPositions.set(m.id, eq?.position ? { ...eq.position } : undefined);
        }
      } else if (m.type === "instrument") {
        const inst = this.project.instruments.find((i) => i.id === m.id);
        oldPositions.set(m.id, inst?.position ? { ...inst.position } : undefined);
      }
    }

    for (const st of this.project.streams) {
      if (movedEqIds.has(st.source_equipment_id) || movedEqIds.has(st.sink_equipment_id)) {
        affectedStreamOldRoutes.push({
          id: st.id,
          points: st.route_points ? st.route_points.map((p) => ({ ...p })) : [],
        });
      }
    }

    const cmd: StudioCommand = {
      description: `Move ${moves.length} items${isDisplayScoped ? ` in ${targetDisplayId}` : ""}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const currentDisplays = (next.displays && next.displays.length > 0
          ? next.displays
          : next.plant_structure?.displays) || [];

        for (const m of moves) {
          if (m.type === "equipment") {
            if (isDisplayScoped) {
              const updateDisplayPlacement = (displays?: any[]) => {
                if (!displays) return;
                const disp = displays.find((d) => d.id === targetDisplayId);
                if (disp) {
                  if (!disp.placements) disp.placements = [];
                  const pl = disp.placements.find((p: any) => p.equipment_id === m.id);
                  if (pl) {
                    pl.x = m.pos.x;
                    pl.y = m.pos.y;
                  } else {
                    disp.placements.push({
                      id: `pl-${disp.id}-${m.id}`,
                      equipment_id: m.id,
                      display_id: disp.id,
                      x: m.pos.x,
                      y: m.pos.y,
                      created_at: new Date().toISOString(),
                      updated_at: new Date().toISOString(),
                    });
                  }
                  if (disp.equipment_ids && !disp.equipment_ids.includes(m.id)) {
                    disp.equipment_ids.push(m.id);
                  }
                }
              };
              updateDisplayPlacement(next.displays);
              updateDisplayPlacement(next.plant_structure?.displays);
            } else {
              const target = next.equipment.find((e) => e.id === m.id);
              if (target) target.position = { ...m.pos };

              // Two-Way Sync: Update any displays containing this equipment's placement
              const syncPlacements = (displays?: any[]) => {
                if (!displays) return;
                for (const disp of displays) {
                  if (disp.placements) {
                    const pl = disp.placements.find((p: any) => p.equipment_id === m.id);
                    if (pl) {
                      pl.x = m.pos.x;
                      pl.y = m.pos.y;
                    }
                  }
                }
              };
              syncPlacements(next.displays);
              syncPlacements(next.plant_structure?.displays);
            }
          } else if (m.type === "instrument") {
            const target = next.instruments.find((i) => i.id === m.id);
            if (target) target.position = { ...m.pos };
          }
        }

        for (const st of next.streams) {
          if (movedEqIds.has(st.source_equipment_id) || movedEqIds.has(st.sink_equipment_id)) {
            const src = next.equipment.find((e) => e.id === st.source_equipment_id);
            const snk = next.equipment.find((e) => e.id === st.sink_equipment_id);
            if (src && snk) {
              let srcPos = src.position;
              let snkPos = snk.position;
              if (isDisplayScoped) {
                const disp = currentDisplays.find((d) => d.id === targetDisplayId);
                const srcPl = disp?.placements?.find((p: any) => p.equipment_id === src.id);
                const snkPl = disp?.placements?.find((p: any) => p.equipment_id === snk.id);
                if (srcPl) srcPos = { x: srcPl.x, y: srcPl.y };
                if (snkPl) snkPos = { x: snkPl.x, y: snkPl.y };
              }
              if (srcPos && snkPos) {
                st.route_points = computeOrthogonalWaypoints(srcPos, snkPos);
              }
            }
          }
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        for (const m of moves) {
          const oldPos = oldPositions.get(m.id);
          if (m.type === "equipment") {
            if (isDisplayScoped) {
              const restoreDisplayPlacement = (displays?: any[]) => {
                if (!displays || !oldPos) return;
                const disp = displays.find((d) => d.id === targetDisplayId);
                if (disp && disp.placements) {
                  const pl = disp.placements.find((p: any) => p.equipment_id === m.id);
                  if (pl) {
                    pl.x = oldPos.x;
                    pl.y = oldPos.y;
                  }
                }
              };
              restoreDisplayPlacement(next.displays);
              restoreDisplayPlacement(next.plant_structure?.displays);
            } else {
              const target = next.equipment.find((e) => e.id === m.id);
              if (target) target.position = oldPos ? { ...oldPos } : undefined;

              const restorePlacements = (displays?: any[]) => {
                if (!displays || !oldPos) return;
                for (const disp of displays) {
                  if (disp.placements) {
                    const pl = disp.placements.find((p: any) => p.equipment_id === m.id);
                    if (pl) {
                      pl.x = oldPos.x;
                      pl.y = oldPos.y;
                    }
                  }
                }
              };
              restorePlacements(next.displays);
              restorePlacements(next.plant_structure?.displays);
            }
          } else if (m.type === "instrument") {
            const target = next.instruments.find((i) => i.id === m.id);
            if (target) target.position = oldPos ? { ...oldPos } : undefined;
          }
        }
        for (const oldRoute of affectedStreamOldRoutes) {
          const st = next.streams.find((s) => s.id === oldRoute.id);
          if (st) {
            st.route_points = oldRoute.points.map((p) => ({ ...p }));
          }
        }
        return next;
      },
    };
    this.execute(cmd);
  }

  resizeEquipment(id: string, width: number, height: number, targetDisplayId?: string): void {
    if (!this.project) return;
    const eq = this.project.equipment.find((e) => e.id === id);
    if (!eq) return;
    const oldW = eq.display_width;
    const oldH = eq.display_height;
    const clampedW = Math.max(40, Math.min(300, Math.round(width / 5) * 5));
    const clampedH = Math.max(40, Math.min(300, Math.round(height / 5) * 5));

    const isDisplayScoped = Boolean(targetDisplayId && targetDisplayId !== "all");

    const cmd: StudioCommand = {
      description: `Resize ${eq.name} to ${clampedW}x${clampedH}${isDisplayScoped ? ` in ${targetDisplayId}` : ""}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (isDisplayScoped) {
          const updateDisplayDims = (displays?: any[]) => {
            if (!displays) return;
            const disp = displays.find((d) => d.id === targetDisplayId);
            if (disp) {
              if (!disp.placements) disp.placements = [];
              const pl = disp.placements.find((p: any) => p.equipment_id === id);
              if (pl) {
                pl.width = clampedW;
                pl.height = clampedH;
              }
            }
          };
          updateDisplayDims(next.displays);
          updateDisplayDims(next.plant_structure?.displays);
        } else {
          const target = next.equipment.find((e) => e.id === id);
          if (target) {
            target.display_width = clampedW;
            target.display_height = clampedH;
          }
          const syncDims = (displays?: any[]) => {
            if (!displays) return;
            for (const disp of displays) {
              if (disp.placements) {
                const pl = disp.placements.find((p: any) => p.equipment_id === id);
                if (pl) {
                  pl.width = clampedW;
                  pl.height = clampedH;
                }
              }
            }
          };
          syncDims(next.displays);
          syncDims(next.plant_structure?.displays);
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (isDisplayScoped) {
          const restoreDisplayDims = (displays?: any[]) => {
            if (!displays) return;
            const disp = displays.find((d) => d.id === targetDisplayId);
            if (disp && disp.placements) {
              const pl = disp.placements.find((p: any) => p.equipment_id === id);
              if (pl) {
                pl.width = oldW;
                pl.height = oldH;
              }
            }
          };
          restoreDisplayDims(next.displays);
          restoreDisplayDims(next.plant_structure?.displays);
        } else {
          const target = next.equipment.find((e) => e.id === id);
          if (target) {
            target.display_width = oldW;
            target.display_height = oldH;
          }
          const restoreDims = (displays?: any[]) => {
            if (!displays) return;
            for (const disp of displays) {
              if (disp.placements) {
                const pl = disp.placements.find((p: any) => p.equipment_id === id);
                if (pl) {
                  pl.width = oldW;
                  pl.height = oldH;
                }
              }
            }
          };
          restoreDims(next.displays);
          restoreDims(next.plant_structure?.displays);
        }
        return next;
      },
    };
    this.execute(cmd);
  }

  rotateEquipment(id: string, rotationDeg: number, targetDisplayId?: string): void {
    if (!this.project) return;
    const eq = this.project.equipment.find((e) => e.id === id);
    if (!eq) return;
    const oldRot = eq.rotation ?? 0;
    const normRot = ((Math.round(rotationDeg / 90) * 90) % 360 + 360) % 360;
    const isDisplayScoped = Boolean(targetDisplayId && targetDisplayId !== "all");

    const cmd: StudioCommand = {
      description: `Rotate ${eq.name} to ${normRot}°${isDisplayScoped ? ` in ${targetDisplayId}` : ""}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (isDisplayScoped) {
          const updateDisplayRot = (displays?: any[]) => {
            if (!displays) return;
            const disp = displays.find((d) => d.id === targetDisplayId);
            if (disp) {
              if (!disp.placements) disp.placements = [];
              const pl = disp.placements.find((p: any) => p.equipment_id === id);
              if (pl) {
                pl.rotation = normRot;
              }
            }
          };
          updateDisplayRot(next.displays);
          updateDisplayRot(next.plant_structure?.displays);
        } else {
          const target = next.equipment.find((e) => e.id === id);
          if (target) {
            target.rotation = normRot;
          }
          const syncRot = (displays?: any[]) => {
            if (!displays) return;
            for (const disp of displays) {
              if (disp.placements) {
                const pl = disp.placements.find((p: any) => p.equipment_id === id);
                if (pl) {
                  pl.rotation = normRot;
                }
              }
            }
          };
          syncRot(next.displays);
          syncRot(next.plant_structure?.displays);
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (isDisplayScoped) {
          const restoreDisplayRot = (displays?: any[]) => {
            if (!displays) return;
            const disp = displays.find((d) => d.id === targetDisplayId);
            if (disp && disp.placements) {
              const pl = disp.placements.find((p: any) => p.equipment_id === id);
              if (pl) {
                pl.rotation = oldRot;
              }
            }
          };
          restoreDisplayRot(next.displays);
          restoreDisplayRot(next.plant_structure?.displays);
        } else {
          const target = next.equipment.find((e) => e.id === id);
          if (target) {
            target.rotation = oldRot;
          }
          const restoreRot = (displays?: any[]) => {
            if (!displays) return;
            for (const disp of displays) {
              if (disp.placements) {
                const pl = disp.placements.find((p: any) => p.equipment_id === id);
                if (pl) {
                  pl.rotation = oldRot;
                }
              }
            }
          };
          restoreRot(next.displays);
          restoreRot(next.plant_structure?.displays);
        }
        return next;
      },
    };
    this.execute(cmd);
  }

  alignSelected(mode: "left" | "center" | "right" | "top" | "middle" | "bottom", targetDisplayId?: string): void {
    if (!this.project || this.selectedIds.size < 2) return;
    const isDisplayScoped = Boolean(targetDisplayId && targetDisplayId !== "all");
    const allDisplays = (this.project.displays && this.project.displays.length > 0
      ? this.project.displays
      : this.project.plant_structure?.displays) || [];
    const activeDisp = isDisplayScoped ? allDisplays.find((d) => d.id === targetDisplayId) : null;

    const getPos = (eq: StudioEquipmentDto) => {
      if (activeDisp && activeDisp.placements) {
        const pl = activeDisp.placements.find((p: any) => p.equipment_id === eq.id);
        if (pl) return { x: pl.x, y: pl.y };
      }
      return eq.position;
    };

    const selectedEq = this.project.equipment.filter(
      (e) => this.selectedIds.has(e.id) && getPos(e)
    );
    if (selectedEq.length < 2) return;

    let targetVal = 0;
    const xs = selectedEq.map((e) => getPos(e)!.x);
    const ys = selectedEq.map((e) => getPos(e)!.y);

    switch (mode) {
      case "left":
        targetVal = Math.min(...xs);
        break;
      case "center":
        targetVal = Math.round(xs.reduce((a, b) => a + b, 0) / xs.length / 10) * 10;
        break;
      case "right":
        targetVal = Math.max(...xs);
        break;
      case "top":
        targetVal = Math.min(...ys);
        break;
      case "middle":
        targetVal = Math.round(ys.reduce((a, b) => a + b, 0) / ys.length / 10) * 10;
        break;
      case "bottom":
        targetVal = Math.max(...ys);
        break;
    }

    const moves: Array<{ id: string; type: SelectableType; pos: Position2dDto }> = [];
    for (const eq of selectedEq) {
      const currentPos = getPos(eq)!;
      let newX = currentPos.x;
      let newY = currentPos.y;
      if (mode === "left" || mode === "center" || mode === "right") {
        newX = targetVal;
      } else {
        newY = targetVal;
      }
      moves.push({ id: eq.id, type: "equipment", pos: { x: newX, y: newY } });
    }

    this.moveEntities(moves, targetDisplayId);
  }

  distributeSelected(axis: "horizontal" | "vertical", targetDisplayId?: string): void {
    if (!this.project || this.selectedIds.size < 3) return;
    const isDisplayScoped = Boolean(targetDisplayId && targetDisplayId !== "all");
    const allDisplays = (this.project.displays && this.project.displays.length > 0
      ? this.project.displays
      : this.project.plant_structure?.displays) || [];
    const activeDisp = isDisplayScoped ? allDisplays.find((d) => d.id === targetDisplayId) : null;

    const getPos = (eq: StudioEquipmentDto) => {
      if (activeDisp && activeDisp.placements) {
        const pl = activeDisp.placements.find((p: any) => p.equipment_id === eq.id);
        if (pl) return { x: pl.x, y: pl.y };
      }
      return eq.position;
    };

    const selectedEq = this.project.equipment
      .filter((e) => this.selectedIds.has(e.id) && getPos(e))
      .slice();
    if (selectedEq.length < 3) return;

    if (axis === "horizontal") {
      selectedEq.sort((a, b) => getPos(a)!.x - getPos(b)!.x);
      const minX = getPos(selectedEq[0])!.x;
      const maxX = getPos(selectedEq[selectedEq.length - 1])!.x;
      const step = (maxX - minX) / (selectedEq.length - 1);

      const moves: Array<{ id: string; type: SelectableType; pos: Position2dDto }> = [];
      selectedEq.forEach((eq, idx) => {
        const newX = Math.round((minX + idx * step) / 10) * 10;
        moves.push({ id: eq.id, type: "equipment", pos: { x: newX, y: getPos(eq)!.y } });
      });
      this.moveEntities(moves, targetDisplayId);
    } else {
      selectedEq.sort((a, b) => getPos(a)!.y - getPos(b)!.y);
      const minY = getPos(selectedEq[0])!.y;
      const maxY = getPos(selectedEq[selectedEq.length - 1])!.y;
      const step = (maxY - minY) / (selectedEq.length - 1);

      const moves: Array<{ id: string; type: SelectableType; pos: Position2dDto }> = [];
      selectedEq.forEach((eq, idx) => {
        const newY = Math.round((minY + idx * step) / 10) * 10;
        moves.push({ id: eq.id, type: "equipment", pos: { x: getPos(eq)!.x, y: newY } });
      });
      this.moveEntities(moves, targetDisplayId);
    }
  }

  duplicateSelected(): void {
    if (!this.project || this.selectedIds.size === 0) return;
    const selectedEq = this.project.equipment.filter((e) => this.selectedIds.has(e.id));
    if (selectedEq.length === 0) return;

    const newEqs: StudioEquipmentDto[] = [];
    const allEq = [...this.project.equipment];
    for (const eq of selectedEq) {
      const copyId = `eq-${crypto.randomUUID ? crypto.randomUUID().slice(0, 8) : Date.now()}`;
      const copyName = `${eq.name}_COPY`;
      const copyPos = eq.position
        ? { x: eq.position.x + 30, y: eq.position.y + 30 }
        : { x: 140, y: 140 };
      const duplicated: StudioEquipmentDto = {
        ...JSON.parse(JSON.stringify(eq)),
        id: copyId,
        name: copyName,
        position: copyPos,
      };
      newEqs.push(duplicated);
      allEq.push(duplicated);
    }

    const cmd: StudioCommand = {
      description: `Duplicate ${newEqs.length} equipment`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.equipment.push(...newEqs);
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const newIds = new Set(newEqs.map((e) => e.id));
        next.equipment = next.equipment.filter((e) => !newIds.has(e.id));
        return next;
      },
    };
    this.execute(cmd);
    this.selectMultiple(newEqs.map((e) => ({ type: "equipment", id: e.id })));
  }

  deleteEquipment(id: string): void {
    if (!this.project) return;
    const eq = this.project.equipment.find((e) => e.id === id);
    if (!eq) return;

    // Also delete connected streams, instruments, and interlocks referring to this equipment
    const affectedStreams = this.project.streams.filter(
      (s) => s.source_equipment_id === id || s.sink_equipment_id === id
    );
    const affectedInstruments = this.project.instruments.filter((i) => i.equipment_id === id);

    const cmd: StudioCommand = {
      description: `Delete Equipment ${eq.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.equipment = next.equipment.filter((e) => e.id !== id);
        next.streams = next.streams.filter(
          (s) => s.source_equipment_id !== id && s.sink_equipment_id !== id
        );
        next.instruments = next.instruments.filter((i) => i.equipment_id !== id);

        const cleanDisplays = (displays?: any[]) => {
          if (!displays) return;
          for (const disp of displays) {
            if (disp.equipment_ids) {
              disp.equipment_ids = disp.equipment_ids.filter((eid: string) => eid !== id);
            }
            if (disp.placements) {
              disp.placements = disp.placements.filter((p: any) => p.equipment_id !== id);
            }
          }
        };
        cleanDisplays(next.displays);
        cleanDisplays(next.plant_structure?.displays);
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.equipment.push(eq);
        next.streams.push(...affectedStreams);
        next.instruments.push(...affectedInstruments);
        return next;
      },
    };
    this.execute(cmd);
    if (this.selected?.id === id) {
      this.clearSelection();
    }
  }

  updateEquipmentParameters(id: string, params: Record<string, number>): void {
    if (!this.project) return;
    const eq = this.project.equipment.find((e) => e.id === id);
    if (!eq) return;
    const oldParams = { ...eq.parameters };

    const cmd: StudioCommand = {
      description: `Update parameters for ${eq.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const target = next.equipment.find((e) => e.id === id);
        if (target) {
          target.parameters = { ...params };
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const target = next.equipment.find((e) => e.id === id);
        if (target) {
          target.parameters = { ...oldParams };
        }
        return next;
      },
    };
    this.execute(cmd);
  }

  addStream(stream: StudioStreamDto): void {
    const cmd: StudioCommand = {
      description: `Add Stream ${stream.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.streams.push(stream);
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.streams = next.streams.filter((s) => s.id !== stream.id);
        return next;
      },
    };
    this.execute(cmd);
    this.selectItem("stream", stream.id);
    this.connectionDraft = null;
  }

  deleteStream(id: string): void {
    if (!this.project) return;
    const st = this.project.streams.find((s) => s.id === id);
    if (!st) return;

    const cmd: StudioCommand = {
      description: `Delete Stream ${st.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.streams = next.streams.filter((s) => s.id !== id);
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.streams.push(st);
        return next;
      },
    };
    this.execute(cmd);
    if (this.selected?.id === id) {
      this.clearSelection();
    }
  }

  updateStream(id: string, update: Partial<StudioStreamDto>): void {
    if (!this.project) return;
    const st = this.project.streams.find((s) => s.id === id);
    if (!st) return;
    const oldStream = { ...st };

    const cmd: StudioCommand = {
      description: `Update Stream ${st.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const target = next.streams.find((s) => s.id === id);
        if (target) {
          Object.assign(target, update);
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const target = next.streams.find((s) => s.id === id);
        if (target) {
          Object.assign(target, oldStream);
        }
        return next;
      },
    };
    this.execute(cmd);
  }

  addInstrument(instrument: StudioInstrumentDto): void {
    const cmd: StudioCommand = {
      description: `Add Instrument ${instrument.tag}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.instruments.push(instrument);
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.instruments = next.instruments.filter((i) => i.id !== instrument.id);
        return next;
      },
    };
    this.execute(cmd);
    this.selectItem("instrument", instrument.id);
  }

  updateInstrument(id: string, update: Partial<StudioInstrumentDto>): void {
    if (!this.project) return;
    const inst = this.project.instruments.find((i) => i.id === id);
    if (!inst) return;
    const oldInst = { ...inst };

    const cmd: StudioCommand = {
      description: `Update Instrument ${inst.tag}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const target = next.instruments.find((i) => i.id === id);
        if (target) {
          Object.assign(target, update);
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const target = next.instruments.find((i) => i.id === id);
        if (target) {
          Object.assign(target, oldInst);
        }
        return next;
      },
    };
    this.execute(cmd);
  }

  addInlineInstrumentToStream(
    streamId: string,
    kind: "Flow" | "Pressure" | "Temperature" = "Flow",
    tagPrefix: string = "FT"
  ): StudioInstrumentDto | null {
    if (!this.project) return null;
    const st = this.project.streams.find((s) => s.id === streamId);
    if (!st) return null;

    const existingCount =
      this.project.instruments.filter((i) => i.tag.startsWith(tagPrefix)).length + 1;
    const tag = `${tagPrefix}-${100 + existingCount}`;
    const id = `inst-${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`;

    let posX = 250;
    let posY = 150;
    if (st.route_points && st.route_points.length >= 2) {
      const mid = Math.floor(st.route_points.length / 2);
      const p1 = st.route_points[mid - 1];
      const p2 = st.route_points[mid];
      posX = Math.round((p1.x + p2.x) / 2);
      posY = Math.round((p1.y + p2.y) / 2) - 35;
    } else {
      const srcEq = this.project.equipment.find((e) => e.id === st.source_equipment_id);
      const snkEq = this.project.equipment.find((e) => e.id === st.sink_equipment_id);
      if (srcEq?.position && snkEq?.position) {
        posX = Math.round((srcEq.position.x + snkEq.position.x) / 2);
        posY = Math.round((srcEq.position.y + snkEq.position.y) / 2) - 35;
      }
    }

    const unitMap: Record<string, string> = {
      Flow: "m3/h",
      Pressure: "bar",
      Temperature: "degC",
      Level: "%",
    };

    const inst: StudioInstrumentDto = {
      id,
      tag,
      equipment_id: st.source_equipment_id,
      port_id: st.source_port_id,
      measurement_kind: kind,
      range_min: 0,
      range_max: kind === "Pressure" ? 50 : kind === "Temperature" ? 200 : 150,
      unit: unitMap[kind] || "units",
      position: { x: posX, y: posY },
    };

    this.addInstrument(inst);
    return inst;
  }

  deleteInstrument(id: string): void {
    if (!this.project) return;
    const inst = this.project.instruments.find((i) => i.id === id);
    if (!inst) return;

    const cmd: StudioCommand = {
      description: `Delete Instrument ${inst.tag}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.instruments = next.instruments.filter((i) => i.id !== id);
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.instruments.push(inst);
        return next;
      },
    };
    this.execute(cmd);
    if (this.selected?.id === id) {
      this.clearSelection();
    }
  }

  addControlLoop(loop: StudioControlLoopDto): void {
    const cmd: StudioCommand = {
      description: `Add Control Loop ${loop.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.control_loops.push(loop);
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.control_loops = next.control_loops.filter((c) => c.id !== loop.id);
        return next;
      },
    };
    this.execute(cmd);
    this.selectItem("control_loop", loop.id);
  }

  updateControlLoop(id: string, update: Partial<StudioControlLoopDto>): void {
    if (!this.project) return;
    const loop = this.project.control_loops.find((c) => c.id === id);
    if (!loop) return;
    const oldLoop = { ...loop };

    const cmd: StudioCommand = {
      description: `Update Control Loop ${loop.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const target = next.control_loops.find((c) => c.id === id);
        if (target) {
          Object.assign(target, update);
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const target = next.control_loops.find((c) => c.id === id);
        if (target) {
          Object.assign(target, oldLoop);
        }
        return next;
      },
    };
    this.execute(cmd);
  }

  deleteControlLoop(id: string): void {
    if (!this.project) return;
    const loop = this.project.control_loops.find((c) => c.id === id);
    if (!loop) return;
    const oldLoop = { ...loop };

    const cmd: StudioCommand = {
      description: `Delete Control Loop ${loop.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.control_loops = next.control_loops.filter((c) => c.id !== id);
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.control_loops.push(oldLoop);
        return next;
      },
    };
    this.execute(cmd);
    if (this.selected?.id === id) {
      this.clearSelection();
    }
  }

  addAlarm(alarm: StudioAlarmDto): void {
    const cmd: StudioCommand = {
      description: `Add Alarm ${alarm.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.alarms.push(alarm);
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.alarms = next.alarms.filter((a) => a.id !== alarm.id);
        return next;
      },
    };
    this.execute(cmd);
    this.selectItem("alarm", alarm.id);
  }

  updateAlarm(id: string, update: Partial<StudioAlarmDto>): void {
    if (!this.project) return;
    const alarm = this.project.alarms.find((a) => a.id === id);
    if (!alarm) return;
    const oldAlarm = { ...alarm };

    const cmd: StudioCommand = {
      description: `Update Alarm ${alarm.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const target = next.alarms.find((a) => a.id === id);
        if (target) {
          Object.assign(target, update);
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const target = next.alarms.find((a) => a.id === id);
        if (target) {
          Object.assign(target, oldAlarm);
        }
        return next;
      },
    };
    this.execute(cmd);
  }

  deleteAlarm(id: string): void {
    if (!this.project) return;
    const alarm = this.project.alarms.find((a) => a.id === id);
    if (!alarm) return;
    const oldAlarm = { ...alarm };

    const cmd: StudioCommand = {
      description: `Delete Alarm ${alarm.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.alarms = next.alarms.filter((a) => a.id !== id);
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.alarms.push(oldAlarm);
        return next;
      },
    };
    this.execute(cmd);
    if (this.selected?.id === id) {
      this.clearSelection();
    }
  }

  addInterlock(interlock: StudioInterlockDto): void {
    const cmd: StudioCommand = {
      description: `Add Interlock ${interlock.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.interlocks.push(interlock);
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.interlocks = next.interlocks.filter((i) => i.id !== interlock.id);
        return next;
      },
    };
    this.execute(cmd);
    this.selectItem("interlock", interlock.id);
  }

  addAdvancedControl(strategy: AdvancedControlStrategyDto): void {
    const cmd: StudioCommand = {
      description: `Add Advanced Control Strategy ${strategy.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (!next.advanced_control) next.advanced_control = [];
        next.advanced_control.push(strategy);
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (next.advanced_control) {
          next.advanced_control = next.advanced_control.filter((s) => s.id !== strategy.id);
        }
        return next;
      },
    };
    this.execute(cmd);
  }

  updateAdvancedControl(id: string, update: Partial<AdvancedControlStrategyDto>): void {
    if (!this.project || !this.project.advanced_control) return;
    const strat = this.project.advanced_control.find((s) => s.id === id);
    if (!strat) return;
    const oldStrat = { ...strat };

    const cmd: StudioCommand = {
      description: `Update Advanced Control Strategy ${strat.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (next.advanced_control) {
          const target = next.advanced_control.find((s) => s.id === id);
          if (target) {
            Object.assign(target, update);
          }
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (next.advanced_control) {
          const target = next.advanced_control.find((s) => s.id === id);
          if (target) {
            Object.assign(target, oldStrat);
          }
        }
        return next;
      },
    };
    this.execute(cmd);
  }

  deleteAdvancedControl(id: string): void {
    if (!this.project || !this.project.advanced_control) return;
    const strat = this.project.advanced_control.find((s) => s.id === id);
    if (!strat) return;
    const oldStrat = { ...strat };

    const cmd: StudioCommand = {
      description: `Delete Advanced Control Strategy ${strat.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (next.advanced_control) {
          next.advanced_control = next.advanced_control.filter((s) => s.id !== id);
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (!next.advanced_control) next.advanced_control = [];
        next.advanced_control.push(oldStrat);
        return next;
      },
    };
    this.execute(cmd);
  }

  // -------------------------------------------------------------------------
  // Process Boundary Conditions (Feeds, Utilities, Sinks)
  // -------------------------------------------------------------------------

  updateBoundaryCondition(id: string, update: Partial<BoundaryConditionDto>): void {
    if (!this.project) return;
    const list = this.project.boundary_conditions || [];
    const bc = list.find((b) => b.id === id);
    if (!bc) return;
    const oldBc = { ...bc };

    const cmd: StudioCommand = {
      description: `Update Boundary Condition ${bc.name}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (next.boundary_conditions) {
          const target = next.boundary_conditions.find((b) => b.id === id);
          if (target) {
            Object.assign(target, update);
          }
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (next.boundary_conditions) {
          const target = next.boundary_conditions.find((b) => b.id === id);
          if (target) {
            Object.assign(target, oldBc);
          }
        }
        return next;
      },
    };
    this.execute(cmd);
  }

  // -------------------------------------------------------------------------
  // Cross-Display Navigation Links & Operational Mapping
  // -------------------------------------------------------------------------

  addNavigationLink(link: NavigationLinkDto): void {
    if (!this.project) return;
    const cmd: StudioCommand = {
      description: `Add Navigation Link ${link.label}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (!next.navigation_links) next.navigation_links = [];
        next.navigation_links.push({ ...link });
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (next.navigation_links) {
          next.navigation_links = next.navigation_links.filter((l) => l.id !== link.id);
        }
        return next;
      },
    };
    this.execute(cmd);
  }

  deleteNavigationLink(linkId: string): void {
    if (!this.project) return;
    const oldLink = (this.project.navigation_links || []).find((l) => l.id === linkId);
    if (!oldLink) return;

    const cmd: StudioCommand = {
      description: `Delete Navigation Link ${oldLink.label}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (next.navigation_links) {
          next.navigation_links = next.navigation_links.filter((l) => l.id !== linkId);
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (!next.navigation_links) next.navigation_links = [];
        next.navigation_links.push({ ...oldLink });
        return next;
      },
    };
    this.execute(cmd);
  }

  setDisplayPlacement(
    displayId: string,
    equipmentId: string,
    pos: { x: number; y: number; width?: number; height?: number; rotation?: number }
  ): void {
    if (!this.project) return;
    const oldDisplays = JSON.parse(JSON.stringify(this.project.displays || []));

    const cmd: StudioCommand = {
      description: `Set display placement for equipment ${equipmentId} on ${displayId}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        const disp = next.displays?.find((d) => d.id === displayId);
        if (disp) {
          if (!disp.placements) disp.placements = [];
          const idx = disp.placements.findIndex((p) => p.equipment_id === equipmentId);
          if (idx >= 0) {
            disp.placements[idx].x = pos.x;
            disp.placements[idx].y = pos.y;
            if (pos.width !== undefined) disp.placements[idx].width = pos.width;
            if (pos.height !== undefined) disp.placements[idx].height = pos.height;
            if (pos.rotation !== undefined) disp.placements[idx].rotation = pos.rotation;
          } else {
            disp.placements.push({
              equipment_id: equipmentId,
              x: pos.x,
              y: pos.y,
              width: pos.width,
              height: pos.height,
              rotation: pos.rotation,
            });
          }
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.displays = JSON.parse(JSON.stringify(oldDisplays));
        return next;
      },
    };
    this.execute(cmd);
  }

  setEquipmentDisplays(equipmentId: string, displayIds: string[]): void {
    if (!this.project) return;
    const oldDisplays = JSON.parse(JSON.stringify(this.project.displays || []));
    const oldPlantStruct = this.project.plant_structure
      ? JSON.parse(JSON.stringify(this.project.plant_structure))
      : undefined;

    const cmd: StudioCommand = {
      description: `Update display memberships for ${equipmentId}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (!next.displays) next.displays = [];
        for (const disp of next.displays) {
          if (!disp.equipment_ids) disp.equipment_ids = [];
          const shouldHave = displayIds.includes(disp.id);
          const has = disp.equipment_ids.includes(equipmentId);
          if (shouldHave && !has) {
            disp.equipment_ids.push(equipmentId);
          } else if (!shouldHave && has) {
            disp.equipment_ids = disp.equipment_ids.filter((id) => id !== equipmentId);
          }
        }
        if (next.plant_structure?.displays) {
          for (const disp of next.plant_structure.displays) {
            if (!disp.equipment_ids) disp.equipment_ids = [];
            const shouldHave = displayIds.includes(disp.id);
            const has = disp.equipment_ids.includes(equipmentId);
            if (shouldHave && !has) {
              disp.equipment_ids.push(equipmentId);
            } else if (!shouldHave && has) {
              disp.equipment_ids = disp.equipment_ids.filter((id) => id !== equipmentId);
            }
          }
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.displays = JSON.parse(JSON.stringify(oldDisplays));
        if (oldPlantStruct) {
          next.plant_structure = JSON.parse(JSON.stringify(oldPlantStruct));
        }
        return next;
      },
    };
    this.execute(cmd);
  }

  setEquipmentOperationalMapping(
    equipmentId: string,
    unitId?: string,
    _systemId?: string,
    _areaId?: string
  ): void {
    if (!this.project || !this.project.plant_structure) return;
    const oldPlantStruct = JSON.parse(JSON.stringify(this.project.plant_structure));

    const cmd: StudioCommand = {
      description: `Update operational hierarchy mapping for ${equipmentId}`,
      execute: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        if (!next.plant_structure?.areas) return next;

        // Remove equipment from all existing units
        for (const area of next.plant_structure.areas) {
          for (const sys of area.systems || []) {
            for (const sub of sys.subsystems || []) {
              for (const u of sub.units || []) {
                if (u.equipment_ids) {
                  u.equipment_ids = u.equipment_ids.filter((id) => id !== equipmentId);
                }
              }
            }
          }
        }

        // Add to selected unit if specified
        if (unitId) {
          for (const area of next.plant_structure.areas) {
            for (const sys of area.systems || []) {
              for (const sub of sys.subsystems || []) {
                for (const u of sub.units || []) {
                  if (u.id === unitId) {
                    if (!u.equipment_ids) u.equipment_ids = [];
                    if (!u.equipment_ids.includes(equipmentId)) {
                      u.equipment_ids.push(equipmentId);
                    }
                  }
                }
              }
            }
          }
        }
        return next;
      },
      undo: (proj) => {
        const next = JSON.parse(JSON.stringify(proj)) as ProjectDetailDto;
        next.plant_structure = JSON.parse(JSON.stringify(oldPlantStruct));
        return next;
      },
    };
    this.execute(cmd);
  }

  applyAutoLayout(mode: "flow-lr" | "flow-tb" | "hierarchical" | "compact" = "flow-lr"): void {
    if (!this.project || this.project.equipment.length === 0) return;
    const positions = generateAutoLayout(this.project.equipment, this.project.streams, mode);
    const moves: Array<{ id: string; type: SelectableType; pos: Position2dDto }> = [];
    for (const [id, pos] of positions.entries()) {
      moves.push({ id, type: "equipment", pos });
    }
    this.moveEntities(moves);
  }

  validateLayout(clearance = 24): GeometryValidationIssue[] {
    if (!this.project) return [];
    return validateLayoutGeometry(this.project.equipment, this.project.streams, 1920, 1080, clearance);
  }
}

export const studioStore = new StudioStore();
if (typeof window !== "undefined") {
  (window as any).studioStore = studioStore;
}

export function computeOrthogonalWaypoints(
  p1: Position2dDto,
  p2: Position2dDto
): Position2dDto[] {
  if (p2.x > p1.x + 30) {
    const midX = Math.round((p1.x + p2.x) / 2);
    return [
      { x: p1.x, y: p1.y },
      { x: midX, y: p1.y },
      { x: midX, y: p2.y },
      { x: p2.x, y: p2.y },
    ];
  }

  const detourY = Math.max(p1.y, p2.y) + 40;
  return [
    { x: p1.x, y: p1.y },
    { x: p1.x + 25, y: p1.y },
    { x: p1.x + 25, y: detourY },
    { x: p2.x - 25, y: detourY },
    { x: p2.x - 25, y: p2.y },
    { x: p2.x, y: p2.y },
  ];
}
