/**
 * Interactive P&ID SVG Flowsheet Canvas Web Component.
 *
 * Implements a professional, model-driven industrial process schematic editor:
 * - Precision vector equipment symbols driven by canonical templates & port signatures
 * - Exact, zoom-invariant coordinate transformation via svg.getScreenCTM().inverse()
 * - Tightly bounded hitboxes preventing overlap and event shadowing
 * - Multi-selection and rectangular marquee selection box
 * - 4-corner resize handles with custom width/height and dynamic port perimeter calculation
 * - Rotation (0°, 90°, 180°, 270°) with trigonometric port rotation and upright tag labels
 * - Leak-free stable window event listeners and 60fps targeted DOM dragging
 * - Deterministic Manhattan orthogonal pipe routing with live waypoint re-routing on move
 * - Movable inline field instrumentation
 * - Magnetic port snapping & validity feedback for interactive stream drafting
 */

import {
  Position2dDto,
  StudioEquipmentDto,
  StudioStreamDto,
  TemplatePortDto,
} from "../protocol/types";
import { computeOrthogonalWaypoints, studioStore } from "../state/studio-store";
import { testEquipmentCollision, testGroupCollision, calculateMagneticPortDock } from "../domain/geometry-collision";
import { findTemplateById } from "../domain/equipment-template-catalog";

interface DragItemState {
  id: string;
  type: "equipment" | "instrument";
  origX: number;
  origY: number;
}

interface MultiDragState {
  startMouseWorld: Position2dDto;
  items: DragItemState[];
  hasMoved: boolean;
}

interface ResizeState {
  equipmentId: string;
  handle: "nw" | "ne" | "sw" | "se";
  startMouseWorld: Position2dDto;
  origWidth: number;
  origHeight: number;
  origX: number;
  origY: number;
}

interface MarqueeState {
  startMouseWorld: Position2dDto;
  currentMouseWorld: Position2dDto;
}

interface PanState {
  startX: number;
  startY: number;
  origPanX: number;
  origPanY: number;
}

interface PortLayout {
  id: string;
  name: string;
  domain: string;
  direction: string;
  relX: number;
  relY: number;
  stubX: number;
  stubY: number;
}

const ARCHETYPE_DIMS: Record<string, { w: number; h: number }> = {
  pump: { w: 60, h: 60 },
  valve: { w: 52, h: 40 },
  tank: { w: 56, h: 80 },
  vessel: { w: 52, h: 96 },
  reactor: { w: 52, h: 96 },
  compressor: { w: 64, h: 72 },
  exchanger: { w: 64, h: 64 },
  column: { w: 64, h: 140 },
  furnace: { w: 120, h: 110 },
};

export class NebuPidCanvas extends HTMLElement {
  private unsubscribe: (() => void) | null = null;
  private dragState: MultiDragState | null = null;
  private resizeState: ResizeState | null = null;
  private marqueeState: MarqueeState | null = null;
  private panState: PanState | null = null;
  private isSpaceDown = false;

  // Viewport navigation state
  private scale = 1.0;
  private panX = 0;
  private panY = 0;

  // Snapping settings
  public snapGrid = true;
  public gridSize = 10;
  public activeDisplayFilter: string = "all";

  private cursorPreviewPos: Position2dDto | null = null;
  private activeSnapTarget: {
    eqId: string;
    portId: string;
    portName: string;
    absX: number;
    absY: number;
    isValid: boolean;
  } | null = null;
  private lastCollisionStatus: { isColliding: boolean; collidingIds: string[] } | null = null;

  // Bound stable event references
  private boundWindowMouseMove = this.onWindowMouseMove.bind(this);
  private boundWindowMouseUp = this.onWindowMouseUp.bind(this);
  private boundWindowKeyDown = this.onWindowKeyDown.bind(this);
  private boundWindowKeyUp = this.onWindowKeyUp.bind(this);

  connectedCallback() {
    this.unsubscribe = studioStore.subscribe(() => this.render());
    this.render();

    window.addEventListener("mousemove", this.boundWindowMouseMove);
    window.addEventListener("mouseup", this.boundWindowMouseUp);
    window.addEventListener("keydown", this.boundWindowKeyDown);
    window.addEventListener("keyup", this.boundWindowKeyUp);
  }

  disconnectedCallback() {
    if (this.unsubscribe) {
      this.unsubscribe();
      this.unsubscribe = null;
    }
    window.removeEventListener("mousemove", this.boundWindowMouseMove);
    window.removeEventListener("mouseup", this.boundWindowMouseUp);
    window.removeEventListener("keydown", this.boundWindowKeyDown);
    window.removeEventListener("keyup", this.boundWindowKeyUp);
  }

  private onWindowKeyDown(e: KeyboardEvent) {
    if (e.target && typeof (e.target as HTMLElement).matches === "function") {
      if ((e.target as HTMLElement).matches("input, select, textarea")) return;
    }

    if (e.code === "Space" && !this.isSpaceDown) {
      this.isSpaceDown = true;
      this.style.cursor = "grab";
      return;
    }

    // Keyboard Arrow Nudging
    if (e.key === "ArrowLeft" || e.key === "ArrowRight" || e.key === "ArrowUp" || e.key === "ArrowDown") {
      const selectedIds = studioStore.getSelectedIds();
      const proj = studioStore.getProject();
      if (proj && selectedIds.size > 0) {
        e.preventDefault();
        const step = e.shiftKey ? 100 : (this.snapGrid ? this.gridSize : 10);
        let dx = 0;
        let dy = 0;
        if (e.key === "ArrowLeft") dx = -step;
        if (e.key === "ArrowRight") dx = step;
        if (e.key === "ArrowUp") dy = -step;
        if (e.key === "ArrowDown") dy = step;

        const moves: Array<{ id: string; type: any; pos: Position2dDto }> = [];
        for (const id of selectedIds) {
          const eq = proj.equipment.find((i) => i.id === id);
          if (eq) {
            const coords = this.getEquipmentCoordinates(eq);
            moves.push({
              id: eq.id,
              type: "equipment",
              pos: { x: Math.max(30, coords.x + dx), y: Math.max(30, coords.y + dy) },
            });
          }
        }
        if (moves.length > 0) {
          studioStore.moveEntities(
            moves,
            this.activeDisplayFilter !== "all" ? this.activeDisplayFilter : undefined
          );
        }
      }
    }

    // Rotation shortcut (R key)
    if ((e.key === "r" || e.key === "R") && !e.ctrlKey && !e.metaKey) {
      const selected = studioStore.getSelected();
      if (selected && selected.type === "equipment") {
        e.preventDefault();
        const proj = studioStore.getProject();
        const eq = proj?.equipment.find((i) => i.id === selected.id);
        if (eq) {
          let currentRot = eq.rotation ?? 0;
          if (this.activeDisplayFilter !== "all") {
            const allDisplays = (proj?.displays && proj?.displays.length > 0 ? proj.displays : proj?.plant_structure?.displays) || [];
            const disp = allDisplays.find((d) => d.id === this.activeDisplayFilter);
            const pl = disp?.placements?.find((p: any) => p.equipment_id === eq.id);
            if (pl && pl.rotation !== undefined) currentRot = pl.rotation;
          }
          studioStore.rotateEquipment(
            eq.id,
            (currentRot + 90) % 360,
            this.activeDisplayFilter !== "all" ? this.activeDisplayFilter : undefined
          );
        }
      }
    }

    // Undo / Redo shortcuts
    if ((e.ctrlKey || e.metaKey) && e.key === "z" && !e.shiftKey) {
      e.preventDefault();
      studioStore.undo();
    } else if ((e.ctrlKey || e.metaKey) && (e.key === "y" || (e.key === "z" && e.shiftKey))) {
      e.preventDefault();
      studioStore.redo();
    }
  }

  private onWindowKeyUp(e: KeyboardEvent) {
    if (e.code === "Space") {
      this.isSpaceDown = false;
      this.style.cursor = "default";
    }
  }

  public zoomIn() {
    this.setScale(this.scale * 1.2);
  }

  public zoomOut() {
    this.setScale(this.scale / 1.2);
  }

  public resetZoom() {
    this.scale = 1.0;
    this.panX = 0;
    this.panY = 0;
    this.updateTransform();
  }

  public fitToView() {
    const proj = studioStore.getProject();
    if (!proj || proj.equipment.length === 0) {
      this.resetZoom();
      return;
    }

    const xs = proj.equipment.map((e) => e.position?.x ?? 100);
    const ys = proj.equipment.map((e) => e.position?.y ?? 100);
    const minX = Math.min(...xs) - 80;
    const maxX = Math.max(...xs) + 80;
    const minY = Math.min(...ys) - 80;
    const maxY = Math.max(...ys) + 80;

    const rect = this.getBoundingClientRect();
    const w = rect.width || 800;
    const h = rect.height || 600;

    const spanX = Math.max(maxX - minX, 200);
    const spanY = Math.max(maxY - minY, 200);

    const s = Math.min(w / spanX, h / spanY, 1.5);
    this.scale = Math.max(0.4, Math.min(s, 1.2));
    this.panX = Math.round((w - (minX + maxX) * this.scale) / 2);
    this.panY = Math.round((h - (minY + maxY) * this.scale) / 2);

    this.updateTransform();
  }

  public focusEntities(ids: string[]) {
    const proj = studioStore.getProject();
    if (!proj || ids.length === 0) return;
    const targets = proj.equipment.filter((e) => ids.includes(e.id));
    if (targets.length === 0) return;

    const xs = targets.map((e) => e.position?.x ?? 100);
    const ys = targets.map((e) => e.position?.y ?? 100);
    const minX = Math.min(...xs) - 80;
    const maxX = Math.max(...xs) + 80;
    const minY = Math.min(...ys) - 80;
    const maxY = Math.max(...ys) + 80;

    const rect = this.getBoundingClientRect();
    const w = rect.width || 800;
    const h = rect.height || 600;

    const spanX = Math.max(maxX - minX, 200);
    const spanY = Math.max(maxY - minY, 200);

    const s = Math.min(w / spanX, h / spanY, 1.4);
    this.scale = Math.max(0.5, Math.min(s, 1.3));
    this.panX = Math.round((w - (minX + maxX) * this.scale) / 2);
    this.panY = Math.round((h - (minY + maxY) * this.scale) / 2);

    this.updateTransform();
  }

  private setScale(newScale: number, pivotX?: number, pivotY?: number) {
    const clamped = Math.max(0.3, Math.min(newScale, 2.5));
    if (pivotX !== undefined && pivotY !== undefined) {
      const ratio = clamped / this.scale;
      this.panX = pivotX - (pivotX - this.panX) * ratio;
      this.panY = pivotY - (pivotY - this.panY) * ratio;
    }
    this.scale = clamped;
    this.updateTransform();
  }

  private updateTransform() {
    const vp = this.querySelector(".viewport-transform") as SVGGElement;
    if (vp) {
      vp.setAttribute("transform", `translate(${this.panX}, ${this.panY}) scale(${this.scale})`);
    }
    this.dispatchEvent(
      new CustomEvent("canvas-view-change", {
        bubbles: true,
        detail: { scale: this.scale, panX: this.panX, panY: this.panY },
      })
    );
  }

  /**
   * Convert screen client coordinates to SVG world coordinates taking pan and zoom into account.
   * Uses svg.getScreenCTM().inverse() for exact, zoom-invariant sub-pixel precision.
   */
  public getSvgCoordinates(e: MouseEvent): Position2dDto {
    const svg = this.querySelector(".pid-svg") as SVGSVGElement;
    if (!svg) return { x: e.clientX, y: e.clientY };
    if (typeof svg.createSVGPoint === "function") {
      const pt = svg.createSVGPoint();
      const ctm = typeof svg.getScreenCTM === "function" ? svg.getScreenCTM() : null;
      if (pt && ctm && typeof pt.matrixTransform === "function") {
        pt.x = e.clientX;
        pt.y = e.clientY;
        const svgP = pt.matrixTransform(ctm.inverse());
        const worldX = Math.round((svgP.x - this.panX) / this.scale);
        const worldY = Math.round((svgP.y - this.panY) / this.scale);
        return { x: worldX, y: worldY };
      }
    }
    const rect = typeof svg.getBoundingClientRect === "function" ? svg.getBoundingClientRect() : { left: 0, top: 0 };
    const worldX = Math.round((e.clientX - rect.left - this.panX) / this.scale);
    const worldY = Math.round((e.clientY - rect.top - this.panY) / this.scale);
    return { x: worldX, y: worldY };
  }

  private getEquipmentArchetype(eq: StudioEquipmentDto): string {
    const defId = eq.definition_id.toLowerCase();
    const name = eq.name.toUpperCase();

    if (defId.includes("pump") || name.startsWith("P-") || name.startsWith("K-")) return "pump";
    if (
      defId.includes("valve") ||
      name.startsWith("FV-") ||
      name.startsWith("VLV-") ||
      name.startsWith("CV-") ||
      name.startsWith("XV-") ||
      name.startsWith("PV-") ||
      name.startsWith("LV-")
    )
      return "valve";
    if (
      defId.includes("furnace") ||
      defId.includes("boiler") ||
      defId.includes("fired") ||
      name.startsWith("F-") ||
      name.startsWith("H-")
    )
      return "furnace";
    if (defId.includes("column") || defId.includes("fractionat") || (name.startsWith("T-") && !name.startsWith("TK-"))) return "column";
    if (defId.includes("tank") || name.startsWith("TK-")) return "tank";
    if (defId.includes("compressor") || name.startsWith("C-")) return "compressor";
    if (
      defId.includes("heat") ||
      defId.includes("exchanger") ||
      defId.includes("cooler") ||
      name.startsWith("E-")
    )
      return "exchanger";
    if (defId.includes("reactor") || name.startsWith("R-")) return "reactor";
    return "vessel";
  }

  private getEquipmentPorts(eq: StudioEquipmentDto): TemplatePortDto[] {
    const templates = studioStore.getTemplates();
    const tmpl = templates.find((t) => t.id === eq.definition_id);
    if (tmpl && tmpl.ports && tmpl.ports.length > 0) {
      return tmpl.ports;
    }

    const arch = this.getEquipmentArchetype(eq);
    if (arch === "pump") {
      return [
        { id: "suction", name: "suction", domain: "material", direction: "inlet" },
        { id: "discharge", name: "discharge", domain: "material", direction: "outlet" },
        { id: "motor_feeder", name: "motor_feeder", domain: "electrical", direction: "inlet" },
      ];
    } else if (arch === "valve") {
      return [
        { id: "inlet", name: "inlet", domain: "material", direction: "inlet" },
        { id: "outlet", name: "outlet", domain: "material", direction: "outlet" },
        { id: "demand_signal", name: "demand_signal", domain: "signal", direction: "inlet" },
      ];
    } else if (arch === "furnace") {
      return [
        { id: "process_in", name: "process_in", domain: "material", direction: "inlet" },
        { id: "process_out", name: "process_out", domain: "material", direction: "outlet" },
        { id: "fuel_gas", name: "fuel_gas", domain: "utility", direction: "inlet" },
        { id: "combustion_air", name: "combustion_air", domain: "utility", direction: "inlet" },
        { id: "flue_gas", name: "flue_gas", domain: "utility", direction: "outlet" },
        { id: "atomizing_steam", name: "atomizing_steam", domain: "utility", direction: "inlet" },
      ];
    } else if (arch === "column") {
      return [
        { id: "feed", name: "feed", domain: "material", direction: "inlet" },
        { id: "overhead_vapor", name: "overhead_vapor", domain: "material", direction: "outlet" },
        { id: "bottoms", name: "bottoms", domain: "material", direction: "outlet" },
        { id: "reflux_in", name: "reflux_in", domain: "material", direction: "inlet" },
        { id: "reboiler_vapor", name: "reboiler_vapor", domain: "material", direction: "inlet" },
        { id: "reboiler_liquid", name: "reboiler_liquid", domain: "material", direction: "outlet" },
      ];
    } else if (arch === "tank") {
      return [
        { id: "inlet", name: "inlet", domain: "material", direction: "inlet" },
        { id: "outlet", name: "outlet", domain: "material", direction: "outlet" },
        { id: "drain_overflow", name: "drain_overflow", domain: "utility", direction: "outlet" },
      ];
    } else if (arch === "compressor") {
      return [
        { id: "suction", name: "suction", domain: "material", direction: "inlet" },
        { id: "discharge", name: "discharge", domain: "material", direction: "outlet" },
      ];
    } else if (arch === "exchanger") {
      return [
        { id: "shell_in", name: "shell_in", domain: "material", direction: "inlet" },
        { id: "shell_out", name: "shell_out", domain: "material", direction: "outlet" },
        { id: "tube_in", name: "tube_in", domain: "material", direction: "inlet" },
        { id: "tube_out", name: "tube_out", domain: "material", direction: "outlet" },
      ];
    } else if (arch === "reactor") {
      return [
        { id: "feed", name: "feed", domain: "material", direction: "inlet" },
        { id: "effluent", name: "effluent", domain: "material", direction: "outlet" },
        { id: "coolant_in", name: "coolant_in", domain: "utility", direction: "inlet" },
        { id: "coolant_out", name: "coolant_out", domain: "utility", direction: "outlet" },
      ];
    }

    return [
      { id: "feed", name: "feed", domain: "material", direction: "inlet" },
      { id: "gas_out", name: "gas_out", domain: "material", direction: "outlet" },
      { id: "liquid_out", name: "liquid_out", domain: "material", direction: "outlet" },
      { id: "psv_relief", name: "psv_relief", domain: "utility", direction: "outlet" },
    ];
  }

  private getEquipmentPortLayouts(eq: StudioEquipmentDto): PortLayout[] {
    const ports = this.getEquipmentPorts(eq);
    const arch = this.getEquipmentArchetype(eq);
    const defaultDims = ARCHETYPE_DIMS[arch] || { w: 60, h: 80 };
    const w = eq.display_width ?? defaultDims.w;
    const h = eq.display_height ?? defaultDims.h;
    const hw = Math.round(w / 2);
    const hh = Math.round(h / 2);

    return ports.map((p) => {
      let relX = 0;
      let relY = 0;
      let stubX = 0;
      let stubY = 0;
      const pName = p.name.toLowerCase();

      if (arch === "pump") {
        if (pName.includes("suction") || (p.direction === "inlet" && p.domain === "material")) {
          relX = -hw - 8;
          relY = 0;
          stubX = -hw;
          stubY = 0;
        } else if (
          pName.includes("discharge") ||
          (p.direction === "outlet" && p.domain === "material")
        ) {
          relX = 0;
          relY = -hh - 8;
          stubX = 0;
          stubY = -hh;
        } else {
          relX = 0;
          relY = hh + 8;
          stubX = 0;
          stubY = hh;
        }
      } else if (arch === "valve") {
        if (pName.includes("inlet") || p.direction === "inlet") {
          if (p.domain === "signal" || pName.includes("signal")) {
            relX = 0;
            relY = -hh - 8;
            stubX = 0;
            stubY = -hh;
          } else {
            relX = -hw - 6;
            relY = 0;
            stubX = -hw;
            stubY = 0;
          }
        } else {
          relX = hw + 6;
          relY = 0;
          stubX = hw;
          stubY = 0;
        }
      } else if (arch === "tank") {
        if (pName.includes("inlet") || p.direction === "inlet") {
          relX = -hw - 8;
          relY = Math.round(-hh * 0.4);
          stubX = -hw;
          stubY = Math.round(-hh * 0.4);
        } else if (pName.includes("outlet")) {
          relX = hw + 8;
          relY = Math.round(hh * 0.5);
          stubX = hw;
          stubY = Math.round(hh * 0.5);
        } else {
          relX = 0;
          relY = hh + 8;
          stubX = 0;
          stubY = hh;
        }
      } else if (arch === "compressor") {
        if (pName.includes("suction") || p.direction === "inlet") {
          relX = -hw - 8;
          relY = 0;
          stubX = -hw;
          stubY = 0;
        } else {
          relX = hw + 8;
          relY = 0;
          stubX = hw;
          stubY = 0;
        }
      } else if (arch === "exchanger") {
        if (pName.includes("shell_in")) {
          relX = -hw - 8;
          relY = Math.round(-hh * 0.35);
          stubX = -hw;
          stubY = Math.round(-hh * 0.35);
        } else if (pName.includes("shell_out")) {
          relX = hw + 8;
          relY = Math.round(-hh * 0.35);
          stubX = hw;
          stubY = Math.round(-hh * 0.35);
        } else if (pName.includes("tube_in")) {
          relX = -hw - 8;
          relY = Math.round(hh * 0.35);
          stubX = -hw;
          stubY = Math.round(hh * 0.35);
        } else if (pName.includes("tube_out")) {
          relX = hw + 8;
          relY = Math.round(hh * 0.35);
          stubX = hw;
          stubY = Math.round(hh * 0.35);
        } else {
          relX = p.direction === "inlet" ? -hw - 8 : hw + 8;
          relY = 0;
          stubX = p.direction === "inlet" ? -hw : hw;
          stubY = 0;
        }
      } else if (arch === "furnace") {
        if (pName.includes("process_in") || pName.includes("feed") || (p.direction === "inlet" && p.domain === "material")) {
          relX = -hw - 8;
          relY = Math.round(-hh * 0.4);
          stubX = -hw;
          stubY = Math.round(-hh * 0.4);
        } else if (pName.includes("process_out") || pName.includes("effluent") || (p.direction === "outlet" && p.domain === "material")) {
          relX = hw + 8;
          relY = Math.round(hh * 0.3);
          stubX = hw;
          stubY = Math.round(hh * 0.3);
        } else if (pName.includes("fuel_gas") || pName.includes("fg")) {
          relX = -hw - 8;
          relY = Math.round(hh * 0.7);
          stubX = -hw;
          stubY = Math.round(hh * 0.7);
        } else if (pName.includes("combustion_air") || pName.includes("air")) {
          relX = -Math.round(hw * 0.5);
          relY = hh + 8;
          stubX = -Math.round(hw * 0.5);
          stubY = hh;
        } else if (pName.includes("flue_gas") || pName.includes("stack")) {
          relX = 0;
          relY = -hh - 8;
          stubX = 0;
          stubY = -hh;
        } else {
          relX = hw + 8;
          relY = Math.round(hh * 0.7);
          stubX = hw;
          stubY = Math.round(hh * 0.7);
        }
      } else if (arch === "column") {
        if (pName.includes("feed")) {
          relX = -hw - 8;
          relY = 0;
          stubX = -hw;
          stubY = 0;
        } else if (pName.includes("overhead") || pName.includes("vapor")) {
          relX = 0;
          relY = -hh - 8;
          stubX = 0;
          stubY = -hh;
        } else if (pName.includes("bottoms")) {
          relX = 0;
          relY = hh + 8;
          stubX = 0;
          stubY = hh;
        } else if (pName.includes("reflux")) {
          relX = -hw - 8;
          relY = -Math.round(hh * 0.65);
          stubX = -hw;
          stubY = -Math.round(hh * 0.65);
        } else if (pName.includes("reboiler") && p.direction === "inlet") {
          relX = -hw - 8;
          relY = Math.round(hh * 0.65);
          stubX = -hw;
          stubY = Math.round(hh * 0.65);
        } else {
          relX = hw + 8;
          relY = Math.round(hh * 0.65);
          stubX = hw;
          stubY = Math.round(hh * 0.65);
        }
      } else {
        // Vessel / Separator / Reactor
        if (pName.includes("feed") || p.direction === "inlet") {
          relX = -hw - 8;
          relY = Math.round(-hh * 0.2);
          stubX = -hw;
          stubY = Math.round(-hh * 0.2);
        } else if (pName.includes("gas") || pName.includes("vapor")) {
          relX = 0;
          relY = -hh - 8;
          stubX = 0;
          stubY = -hh;
        } else if (pName.includes("liquid") || pName.includes("effluent")) {
          relX = 0;
          relY = hh + 8;
          stubX = 0;
          stubY = hh;
        } else if (pName.includes("psv") || pName.includes("relief")) {
          relX = hw + 8;
          relY = Math.round(-hh * 0.7);
          stubX = hw;
          stubY = Math.round(-hh * 0.7);
        } else {
          relX = hw + 8;
          relY = Math.round(hh * 0.2);
          stubX = hw;
          stubY = Math.round(hh * 0.2);
        }
      }

      return {
        id: p.id,
        name: p.name,
        domain: p.domain,
        direction: p.direction,
        relX,
        relY,
        stubX,
        stubY,
      };
    });
  }

  public getPortAbsoluteCoords(
    eqId: string,
    portNameOrId: string
  ): { x: number; y: number } {
    const proj = studioStore.getProject();
    const eq = proj?.equipment.find((e) => e.id === eqId);
    if (!eq) return { x: 100, y: 100 };

    const coords = this.getEquipmentCoordinates(eq);
    const eqX = coords.x;
    const eqY = coords.y;
    const rot = eq.rotation ?? 0;

    const layouts = this.getEquipmentPortLayouts(eq);
    const target = layouts.find(
      (l) => l.name === portNameOrId || l.id === portNameOrId
    );

    const rawRelX = target ? target.relX : 30;
    const rawRelY = target ? target.relY : 0;

    // Apply equipment rotation to port coordinate offset
    if (rot !== 0) {
      const rad = (rot * Math.PI) / 180;
      const cos = Math.cos(rad);
      const sin = Math.sin(rad);
      const rx = rawRelX * cos - rawRelY * sin;
      const ry = rawRelX * sin + rawRelY * cos;
      return { x: Math.round(eqX + rx), y: Math.round(eqY + ry) };
    }

    return { x: Math.round(eqX + rawRelX), y: Math.round(eqY + rawRelY) };
  }

  public getEquipmentCoordinates(eq: StudioEquipmentDto): { x: number; y: number } {
    if (this.activeDisplayFilter !== "all") {
      const proj = studioStore.getProject();
      const allDisplays = (proj?.displays && proj?.displays.length > 0 ? proj.displays : proj?.plant_structure?.displays) || [];
      const disp = allDisplays.find((d) => d.id === this.activeDisplayFilter);
      const pl = disp?.placements?.find((p) => p.equipment_id === eq.id);
      if (pl) {
        return { x: pl.x, y: pl.y };
      }
    }
    return { x: eq.position?.x ?? 100, y: eq.position?.y ?? 100 };
  }

  private renderEquipmentSymbol(
    eq: StudioEquipmentDto,
    isSelected: boolean,
    isDimmed: boolean = false
  ): string {
    const coords = this.getEquipmentCoordinates(eq);
    const x = coords.x;
    const y = coords.y;
    const name = eq.name;
    const arch = this.getEquipmentArchetype(eq);
    const defaultDims = ARCHETYPE_DIMS[arch] || { w: 60, h: 80 };
    let w = eq.display_width ?? defaultDims.w;
    let h = eq.display_height ?? defaultDims.h;
    let rot = eq.rotation ?? 0;

    if (this.activeDisplayFilter !== "all") {
      const proj = studioStore.getProject();
      const allDisplays = (proj?.displays && proj?.displays.length > 0 ? proj.displays : proj?.plant_structure?.displays) || [];
      const disp = allDisplays.find((d) => d.id === this.activeDisplayFilter);
      const pl = disp?.placements?.find((p) => p.equipment_id === eq.id);
      if (pl) {
        if (pl.width) w = pl.width;
        if (pl.height) h = pl.height;
        if (pl.rotation !== undefined) rot = pl.rotation;
      }
    }

    const hw = Math.round(w / 2);
    const hh = Math.round(h / 2);

    const portLayouts = this.getEquipmentPortLayouts(eq);

    let bodySvg = "";
    if (arch === "pump") {
      const r = Math.min(hw, hh) - 2;
      bodySvg = `
        <circle cx="0" cy="0" r="${r}" class="eq-body ${isSelected ? "selected" : ""}" />
        <circle cx="0" cy="0" r="${Math.round(r * 0.28)}" fill="none" stroke="#64748b" stroke-width="1.5" />
        <path d="M 0 ${-r} L ${Math.round(r * 0.8)} ${-Math.round(r * 0.5)} L 0 0 Z" fill="#38bdf8" opacity="0.65" />
        <line x1="${-r}" y1="0" x2="${-hw}" y2="0" stroke="#64748b" stroke-width="2" />
        <line x1="0" y1="${-r}" x2="0" y2="${-hh}" stroke="#64748b" stroke-width="2" />
      `;
    } else if (arch === "valve") {
      bodySvg = `
        <polygon points="${-hw},-14 0,0 ${-hw},14" class="eq-body ${isSelected ? "selected" : ""}" />
        <polygon points="${hw},-14 0,0 ${hw},14" class="eq-body ${isSelected ? "selected" : ""}" />
        <line x1="0" y1="0" x2="0" y2="${-hh + 6}" stroke="#94a3b8" stroke-width="2" />
        <ellipse cx="0" cy="${-hh}" rx="14" ry="6" fill="#1e293b" stroke="#94a3b8" stroke-width="1.5" />
        <circle cx="0" cy="${-hh}" r="2" fill="#38bdf8" />
      `;
    } else if (arch === "compressor") {
      bodySvg = `
        <polygon points="${-hw},${-Math.round(hh * 0.6)} ${hw},${-hh} ${hw},${hh} ${-hw},${Math.round(hh * 0.6)}" class="eq-body ${isSelected ? "selected" : ""}" />
        <line x1="${-Math.round(hw * 0.33)}" y1="${-Math.round(hh * 0.7)}" x2="${-Math.round(hw * 0.33)}" y2="${Math.round(hh * 0.7)}" stroke="#64748b" stroke-width="1" stroke-dasharray="2 2" />
        <line x1="${Math.round(hw * 0.33)}" y1="${-Math.round(hh * 0.85)}" x2="${Math.round(hw * 0.33)}" y2="${Math.round(hh * 0.85)}" stroke="#64748b" stroke-width="1" stroke-dasharray="2 2" />
      `;
    } else if (arch === "exchanger") {
      const r = Math.min(hw, hh) - 2;
      bodySvg = `
        <circle cx="0" cy="0" r="${r}" class="eq-body ${isSelected ? "selected" : ""}" />
        <path d="M ${-Math.round(r * 0.7)} -10 Q 0 -22 ${Math.round(r * 0.7)} -10 Q 0 10 ${-Math.round(r * 0.7)} 10 Q 0 22 ${Math.round(r * 0.7)} 10" fill="none" stroke="#fbbf24" stroke-width="2.2" />
      `;
    } else if (arch === "tank") {
      bodySvg = `
        <rect x="${-hw}" y="${-hh}" width="${w}" height="${h}" rx="3" class="eq-body ${isSelected ? "selected" : ""}" />
        <path d="M ${-hw} ${-hh} L 0 ${-hh - 12} L ${hw} ${-hh}" fill="none" stroke="#94a3b8" stroke-width="2" />
        <line x1="${-hw + 6}" y1="${Math.round(hh * 0.5)}" x2="${hw - 6}" y2="${Math.round(hh * 0.5)}" stroke="#38bdf8" stroke-width="1" stroke-dasharray="3 3" opacity="0.6" />
      `;
    } else if (arch === "reactor") {
      bodySvg = `
        <rect x="${-hw}" y="${-hh}" width="${w}" height="${h}" rx="14" class="eq-body ${isSelected ? "selected" : ""}" />
        <line x1="0" y1="${-hh + 15}" x2="0" y2="${hh - 25}" stroke="#fbbf24" stroke-width="2" />
        <ellipse cx="0" cy="${hh - 25}" rx="${Math.min(hw - 8, 14)}" ry="4" fill="#fbbf24" opacity="0.7" />
        <circle cx="0" cy="${-hh - 3}" r="4" fill="#64748b" />
      `;
    } else if (arch === "furnace") {
      bodySvg = `
        <!-- Main Furnace Firebox Casing -->
        <rect x="${-hw}" y="${-hh + 20}" width="${w}" height="${h - 20}" rx="3" class="eq-body ${isSelected ? "selected" : ""}" />
        <!-- Stack Transition & Stack -->
        <polygon points="${-Math.round(hw * 0.4)},${-hh + 20} ${-14},${-hh} ${14},${-hh} ${Math.round(hw * 0.4)},${-hh + 20}" fill="#1e293b" stroke="#64748b" stroke-width="1.5" />
        <rect x="-14" y="${-hh - 18}" width="28" height="18" fill="#1e293b" stroke="#64748b" stroke-width="1.5" />
        <!-- Stack Damper -->
        <line x1="-10" y1="${-hh - 9}" x2="10" y2="${-hh - 9}" stroke="#f59e0b" stroke-width="2" />
        <!-- Convection / Radiant Divider -->
        <line x1="${-hw + 4}" y1="${-Math.round(hh * 0.15)}" x2="${hw - 4}" y2="${-Math.round(hh * 0.15)}" stroke="#475569" stroke-width="1" stroke-dasharray="3 2" />
        <!-- Convection Coil Pass -->
        <path d="M ${-hw + 10} ${-Math.round(hh * 0.4)} L ${hw - 10} ${-Math.round(hh * 0.4)} C ${hw - 4} ${-Math.round(hh * 0.4)} ${hw - 4} ${-Math.round(hh * 0.25)} ${hw - 10} ${-Math.round(hh * 0.25)} L ${-hw + 10} ${-Math.round(hh * 0.25)}" fill="none" stroke="#38bdf8" stroke-width="1.6" />
        <!-- Radiant Coil Pass -->
        <path d="M ${-hw + 10} ${Math.round(hh * 0.05)} L ${hw - 10} ${Math.round(hh * 0.05)} C ${hw - 4} ${Math.round(hh * 0.05)} ${hw - 4} ${Math.round(hh * 0.25)} ${hw - 10} ${Math.round(hh * 0.25)} L ${-hw + 10} ${Math.round(hh * 0.25)}" fill="none" stroke="#f97316" stroke-width="1.8" />
        <!-- Floor Burners & Flame indicators -->
        <rect x="${-Math.round(hw * 0.6)}" y="${hh - 8}" width="14" height="8" rx="1" fill="#334155" stroke="#64748b" />
        <polygon points="${-Math.round(hw * 0.6) + 3},${hh - 8} ${-Math.round(hw * 0.6) + 7},${hh - 22} ${-Math.round(hw * 0.6) + 11},${hh - 8}" fill="#f59e0b" opacity="0.85" />
        <rect x="-7" y="${hh - 8}" width="14" height="8" rx="1" fill="#334155" stroke="#64748b" />
        <polygon points="-4,${hh - 8} 0,${hh - 24} 4,${hh - 8}" fill="#f59e0b" opacity="0.85" />
        <rect x="${Math.round(hw * 0.6) - 14}" y="${hh - 8}" width="14" height="8" rx="1" fill="#334155" stroke="#64748b" />
        <polygon points="${Math.round(hw * 0.6) - 11},${hh - 8} ${Math.round(hw * 0.6) - 7},${hh - 22} ${Math.round(hw * 0.6) - 3},${hh - 8}" fill="#f59e0b" opacity="0.85" />
        <!-- Fuel Gas Manifold line -->
        <line x1="${-hw - 6}" y1="${hh + 4}" x2="${hw + 6}" y2="${hh + 4}" stroke="#eab308" stroke-width="2" />
      `;
    } else if (arch === "column") {
      bodySvg = `
        <!-- Column Shell -->
        <rect x="${-hw}" y="${-hh}" width="${w}" height="${h}" rx="14" class="eq-body ${isSelected ? "selected" : ""}" />
        <!-- Trays -->
        <line x1="${-hw + 6}" y1="${-Math.round(hh * 0.55)}" x2="${hw - 6}" y2="${-Math.round(hh * 0.55)}" stroke="#475569" stroke-width="1.2" stroke-dasharray="4 2" />
        <line x1="${-hw + 6}" y1="${-Math.round(hh * 0.35)}" x2="${hw - 6}" y2="${-Math.round(hh * 0.35)}" stroke="#475569" stroke-width="1.2" stroke-dasharray="4 2" />
        <line x1="${-hw + 6}" y1="${-Math.round(hh * 0.15)}" x2="${hw - 6}" y2="${-Math.round(hh * 0.15)}" stroke="#475569" stroke-width="1.2" stroke-dasharray="4 2" />
        <line x1="${-hw + 6}" y1="${Math.round(hh * 0.05)}" x2="${hw - 6}" y2="${Math.round(hh * 0.05)}" stroke="#475569" stroke-width="1.2" stroke-dasharray="4 2" />
        <line x1="${-hw + 6}" y1="${Math.round(hh * 0.25)}" x2="${hw - 6}" y2="${Math.round(hh * 0.25)}" stroke="#475569" stroke-width="1.2" stroke-dasharray="4 2" />
        <line x1="${-hw + 6}" y1="${Math.round(hh * 0.45)}" x2="${hw - 6}" y2="${Math.round(hh * 0.45)}" stroke="#475569" stroke-width="1.2" stroke-dasharray="4 2" />
        <!-- Liquid Level -->
        <line x1="${-hw + 4}" y1="${Math.round(hh * 0.7)}" x2="${hw - 4}" y2="${Math.round(hh * 0.7)}" stroke="#38bdf8" stroke-width="1.5" stroke-dasharray="3 3" opacity="0.8" />
        <!-- Skirt Support -->
        <polygon points="${-hw},${hh} ${-hw - 4},${hh + 12} ${-hw + 8},${hh + 12} ${-hw + 6},${hh}" fill="#1e293b" stroke="#64748b" />
        <polygon points="${hw},${hh} ${hw + 4},${hh + 12} ${hw - 8},${hh + 12} ${hw - 6},${hh}" fill="#1e293b" stroke="#64748b" />
      `;
    } else {
      // Vertical Vessel
      bodySvg = `
        <rect x="${-hw}" y="${-hh}" width="${w}" height="${h}" rx="16" class="eq-body ${isSelected ? "selected" : ""}" />
        <line x1="${-hw + 4}" y1="${-Math.round(hh * 0.3)}" x2="${hw - 4}" y2="${-Math.round(hh * 0.3)}" stroke="#475569" stroke-width="1" stroke-dasharray="2 2" />
        <line x1="${-hw + 4}" y1="${Math.round(hh * 0.3)}" x2="${hw - 4}" y2="${Math.round(hh * 0.3)}" stroke="#38bdf8" stroke-width="1" stroke-dasharray="3 3" opacity="0.6" />
      `;
    }

    const isDrafting = studioStore.getConnectionDraft() !== null;
    const portsSvg = portLayouts
      .map((pl) => {
        const pinClass =
          pl.direction === "inlet"
            ? "port-inlet"
            : pl.direction === "outlet"
            ? "port-outlet"
            : "port-bidirectional";

        const isSnapTarget =
          this.activeSnapTarget?.eqId === eq.id &&
          this.activeSnapTarget?.portName === pl.name;

        const snapClass = isSnapTarget
          ? this.activeSnapTarget?.isValid
            ? "snap-valid"
            : "snap-invalid"
          : "";

        return `
          <line x1="${pl.stubX}" y1="${pl.stubY}" x2="${pl.relX}" y2="${pl.relY}" class="port-stub" />
          <circle
            cx="${pl.relX}"
            cy="${pl.relY}"
            r="${isSnapTarget ? 8.5 : 5.5}"
            class="port-pin ${pinClass} ${snapClass}"
            data-eq-id="${eq.id}"
            data-port-id="${pl.name}"
            data-port-name="${pl.name}"
            data-port-direction="${pl.direction}"
            data-port-domain="${pl.domain}"
            title="${pl.name} (${pl.direction}, ${pl.domain})"
          />
        `;
      })
      .join("");

    // Resize handles rendered only when selected
    const resizeHandlesSvg = isSelected
      ? `
        <g class="resize-handles-group">
          <rect x="${-hw - 5}" y="${-hh - 5}" width="8" height="8" class="resize-handle handle-nw" data-handle="nw" data-eq-id="${eq.id}" />
          <rect x="${hw - 3}" y="${-hh - 5}" width="8" height="8" class="resize-handle handle-ne" data-handle="ne" data-eq-id="${eq.id}" />
          <rect x="${-hw - 5}" y="${hh - 3}" width="8" height="8" class="resize-handle handle-sw" data-handle="sw" data-eq-id="${eq.id}" />
          <rect x="${hw - 3}" y="${hh - 3}" width="8" height="8" class="resize-handle handle-se" data-handle="se" data-eq-id="${eq.id}" />
        </g>
      `
      : "";

    return `
      <g class="eq-node ${isSelected ? "selected" : ""} ${isDrafting ? "drafting-mode" : ""}"
         transform="translate(${x}, ${y}) rotate(${rot})"
         data-eq-id="${eq.id}"
         data-rotation="${rot}"
         opacity="${isDimmed ? "0.32" : "1.0"}">
        <!-- Tight, dimension-matched hitarea prevents click shadowing -->
        <rect x="${-hw}" y="${-hh}" width="${w}" height="${h}" fill="transparent" class="eq-hitarea" pointer-events="all" />
        ${isSelected ? `<rect x="${-hw - 4}" y="${-hh - 4}" width="${w + 8}" height="${h + 8}" class="eq-selection-halo" rx="6" />` : ""}
        ${bodySvg}
        <g class="ports-group">${portsSvg}</g>
        ${resizeHandlesSvg}
        <!-- Counter-rotated tag badge keeps text upright and readable -->
        <g class="eq-tag-badge" transform="translate(0, ${hh + 16}) rotate(${-rot})" pointer-events="none">
          <text x="0" y="0" class="eq-tag-label" text-anchor="middle" pointer-events="none">${name}</text>
        </g>
      </g>
    `;
  }

  private renderStream(
    stream: StudioStreamDto,
    equipmentList: StudioEquipmentDto[],
    isSelected: boolean
  ): string {
    const srcEq = equipmentList.find((e) => e.id === stream.source_equipment_id);
    const snkEq = equipmentList.find((e) => e.id === stream.sink_equipment_id);
    if (!srcEq || !snkEq) return "";

    const p1 = this.getPortAbsoluteCoords(stream.source_equipment_id, stream.source_port_id);
    const p2 = this.getPortAbsoluteCoords(stream.sink_equipment_id, stream.sink_port_id);

    let points = stream.route_points;
    if (this.activeDisplayFilter !== "all" || !points || points.length < 2) {
      points = computeOrthogonalWaypoints(p1, p2);
    } else {
      points[0] = { x: p1.x, y: p1.y };
      points[points.length - 1] = { x: p2.x, y: p2.y };
    }

    const pathD = points
      .map((p, idx) => (idx === 0 ? `M ${p.x} ${p.y}` : `L ${p.x} ${p.y}`))
      .join(" ");

    const midIdx = Math.floor(points.length / 2);
    const pPrev = points[midIdx - 1] || points[0];
    const pCur = points[midIdx] || points[1];
    const labelX = Math.round((pPrev.x + pCur.x) / 2);
    const labelY = Math.round((pPrev.y + pCur.y) / 2);

    return `
      <g class="stream-group ${isSelected ? "selected" : ""}" data-stream-id="${stream.id}">
        <path
          d="${pathD}"
          class="process-pipe ${isSelected ? "selected" : ""}"
          data-stream-id="${stream.id}"
          marker-mid="url(#pipe-arrow)"
        />
        <path
          d="${pathD}"
          class="process-pipe-hitarea"
          data-stream-id="${stream.id}"
        />
        <g transform="translate(${labelX}, ${labelY})" pointer-events="none">
          <rect x="-18" y="-9" width="36" height="18" class="stream-label-bg" />
          <text x="0" y="3.5" class="stream-label" text-anchor="middle">
            ${stream.name}
          </text>
        </g>
      </g>
    `;
  }

  render() {
    const proj = studioStore.getProject();
    if (!proj) {
      this.innerHTML = `
        <div class="canvas-empty-state">
          <div class="empty-state-card">
            <span class="empty-icon">⛶</span>
            <h4>No Flowsheet Active</h4>
            <p>Select an existing project from the toolbar or create a new flowsheet to begin engineering modeling.</p>
          </div>
        </div>
      `;
      return;
    }

    const selected = studioStore.getSelected();
    const draft = studioStore.getConnectionDraft();

    const allDisplays = proj.displays || proj.plant_structure?.displays || [];
    const activeDisp = allDisplays.find((d) => d.id === this.activeDisplayFilter);
    const filterEquipmentIds = activeDisp?.equipment_ids;

    const equipmentSvg = proj.equipment
      .map((eq) => {
        const isDimmed =
          this.activeDisplayFilter !== "all" &&
          Boolean(filterEquipmentIds && !filterEquipmentIds.includes(eq.id));
        return this.renderEquipmentSymbol(
          eq,
          studioStore.isSelected(eq.id),
          isDimmed
        );
      })
      .join("");

    const streamsSvg = proj.streams
      .map((st) =>
        this.renderStream(
          st,
          proj.equipment,
          selected?.type === "stream" && selected.id === st.id
        )
      )
      .join("");

    const instrumentsSvg = proj.instruments
      .map((inst) => {
        const x = inst.position?.x ?? 200;
        const y = inst.position?.y ?? 60;
        const isSelected = selected?.type === "instrument" && selected.id === inst.id;

        let linkLine = "";
        if (inst.equipment_id) {
          const parent = proj.equipment.find((e) => e.id === inst.equipment_id);
          if (parent?.position) {
            const targetPos = inst.port_id
              ? this.getPortAbsoluteCoords(inst.equipment_id, inst.port_id)
              : parent.position;

            linkLine = `<line x1="${x}" y1="${y + 16}" x2="${targetPos.x}" y2="${
              targetPos.y
            }" class="signal-line" />`;
          }
        }

        const tagParts = inst.tag.split("-");
        const topLetters = tagParts[0] || inst.tag.slice(0, 2);
        const bottomNum = tagParts.slice(1).join("-") || inst.tag.slice(2);

        return `
          <g class="instrument-node ${isSelected ? "selected" : ""}" transform="translate(${x}, ${y})" data-inst-id="${inst.id}">
            ${linkLine}
            <circle cx="0" cy="0" r="16" class="instrument-bubble ${
              isSelected ? "selected" : ""
            }" data-inst-id="${inst.id}" />
            <line x1="-16" y1="0" x2="16" y2="0" stroke="#64748b" stroke-width="1.2" />
            <text x="0" y="-3" class="inst-tag-text" text-anchor="middle">${topLetters}</text>
            <text x="0" y="10" class="inst-tag-text" text-anchor="middle">${bottomNum}</text>
          </g>
        `;
      })
      .join("");

    let draftPreviewSvg = "";
    if (draft && this.cursorPreviewPos) {
      const srcPos = this.getPortAbsoluteCoords(
        draft.sourceEquipmentId,
        draft.sourcePortId
      );
      const destPos = this.activeSnapTarget
        ? { x: this.activeSnapTarget.absX, y: this.activeSnapTarget.absY }
        : this.cursorPreviewPos;

      const previewWaypoints = computeOrthogonalWaypoints(srcPos, destPos);
      const draftPathD = previewWaypoints
        .map((p, idx) => (idx === 0 ? `M ${p.x} ${p.y}` : `L ${p.x} ${p.y}`))
        .join(" ");

      draftPreviewSvg = `
        <path d="${draftPathD}" class="pipe-draft-preview ${
        this.activeSnapTarget?.isValid ? "valid-snap" : ""
      }" />
      `;
    }

    this.innerHTML = `
      <div class="pid-canvas-wrapper">
        <div class="canvas-display-scope-bar flex items-center justify-between px-3 py-1.5 bg-[#0b0f17]/95 border-b border-[#1e293b] text-xs z-10">
          <div class="flex items-center gap-2">
            <span class="text-dim font-medium">Display Scope:</span>
            <select class="select-display-filter bg-[#1e293b] text-slate-200 border border-slate-700 rounded px-2 py-0.5 text-xs">
              <option value="all" ${this.activeDisplayFilter === "all" ? "selected" : ""}>Full Physical Process Model (All Equipment)</option>
              ${allDisplays
                .map(
                  (d) =>
                    `<option value="${d.id}" ${this.activeDisplayFilter === d.id ? "selected" : ""}>${d.name} (${d.equipment_ids?.length ?? 0} units)</option>`
                )
                .join("")}
            </select>
          </div>
          ${
            this.activeDisplayFilter !== "all" && activeDisp
              ? `<div class="text-[11px] text-sky-400 font-mono">
                  Active Display Scope: <strong>${activeDisp.name}</strong> • ${activeDisp.equipment_ids?.length ?? 0} mapped equipment
                </div>`
              : `<div class="text-[11px] text-dim">Showing complete physical topology graph</div>`
          }
        </div>

        ${
          draft
            ? `<div class="connection-draft-banner">
                <div class="banner-pulse"></div>
                <span>Wiring Stream from: <strong>${
                  draft.portName || draft.sourcePortId
                }</strong>. Click destination port or press ESC.</span>
                <button class="btn-cancel-draft">Cancel</button>
               </div>`
            : ""
        }

        <svg class="pid-svg" width="100%" height="100%" viewBox="0 0 1000 700">
          <defs>
            <pattern id="canvas-grid-dots" width="20" height="20" patternUnits="userSpaceOnUse">
              <circle cx="10" cy="10" r="1" fill="#1e293b" />
            </pattern>
            <pattern id="canvas-grid-major" width="100" height="100" patternUnits="userSpaceOnUse">
              <path d="M 100 0 L 0 0 0 100" fill="none" stroke="#172030" stroke-width="0.75" />
            </pattern>
            <marker id="pipe-arrow" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
              <path d="M 0 1.5 L 10 5 L 0 8.5 z" fill="#38bdf8" />
            </marker>
          </defs>

          <!-- Stationary Background -->
          <rect width="100%" height="100%" fill="url(#canvas-grid-dots)" class="canvas-bg" />
          <rect width="100%" height="100%" fill="url(#canvas-grid-major)" class="canvas-bg-major" pointer-events="none" />

          <!-- Dynamic Pan & Zoom Viewport Layer -->
          <g class="viewport-transform" transform="translate(${this.panX}, ${this.panY}) scale(${this.scale})">
            <!-- Process Streams (Edges) -->
            <g class="streams-layer">${streamsSvg}</g>

            <!-- Live Drafting Rubber-band Stream -->
            <g class="draft-layer">${draftPreviewSvg}</g>

            <!-- Field Instruments & Signal Lines -->
            <g class="instruments-layer">${instrumentsSvg}</g>

            <!-- Equipment Units (Nodes) -->
            <g class="equipment-layer">${equipmentSvg}</g>

            <!-- Live Collision & Magnetic Docking Preview Layer -->
            <g class="collision-preview-layer" pointer-events="none"></g>

            <!-- Selection Marquee Box -->
            <g class="marquee-layer">
              <rect class="selection-marquee" fill="rgba(56, 189, 248, 0.15)" stroke="#38bdf8" stroke-width="1.2" stroke-dasharray="4 2" display="none" pointer-events="none" />
            </g>
          </g>
        </svg>

        <!-- Floating Canvas Action Controls -->
        <div class="canvas-floating-controls">
          <button class="canvas-btn-floating btn-canvas-fullscreen" title="Toggle Fullscreen Editor Mode (F11)">
            ⛶ <span class="text-[10px]">Fullscreen</span>
          </button>
          <button class="canvas-btn-floating btn-canvas-fit" title="Fit Process to View (Reset Pan/Zoom)">
            ⤢ Fit
          </button>
          <button class="canvas-btn-floating btn-canvas-zoomin" title="Zoom In (+)">
            +
          </button>
          <button class="canvas-btn-floating btn-canvas-zoomout" title="Zoom Out (-)">
            −
          </button>
        </div>
      </div>
    `;

    this.bindCanvasEvents();
  }

  private bindCanvasEvents() {
    this.querySelector(".btn-canvas-fullscreen")?.addEventListener("click", () => {
      this.dispatchEvent(new CustomEvent("studio-toggle-fullscreen", { bubbles: true, composed: true }));
    });
    this.querySelector(".btn-canvas-fit")?.addEventListener("click", () => this.fitToView());
    this.querySelector(".btn-canvas-zoomin")?.addEventListener("click", () => this.zoomIn());
    this.querySelector(".btn-canvas-zoomout")?.addEventListener("click", () => this.zoomOut());

    const filterSel = this.querySelector(".select-display-filter") as HTMLSelectElement;
    filterSel?.addEventListener("change", () => {
      this.activeDisplayFilter = filterSel.value;
      this.render();
    });

    const svg = this.querySelector(".pid-svg") as SVGSVGElement;
    if (!svg) return;

    // Wheel Zoom & Pan (Standard mouse wheel scrolls/pans canvas, Ctrl/Cmd + wheel zooms)
    svg.addEventListener(
      "wheel",
      (e: WheelEvent) => {
        e.preventDefault();
        if (e.ctrlKey || e.metaKey) {
          // Ctrl/Cmd + Wheel = Zoom in/out anchored at mouse pointer
          const rect = svg.getBoundingClientRect();
          const pivotX = e.clientX - rect.left;
          const pivotY = e.clientY - rect.top;
          const factor = e.deltaY < 0 ? 1.1 : 0.9;
          this.setScale(this.scale * factor, pivotX, pivotY);
        } else {
          // Standard mouse wheel / trackpad = Smooth 2D scrolling / panning across flowsheet
          if (e.shiftKey) {
            // Shift + Wheel = Horizontal scroll
            this.panX -= (e.deltaX !== 0 ? e.deltaX : e.deltaY);
          } else {
            this.panX -= e.deltaX;
            this.panY -= e.deltaY;
          }
          this.updateTransform();
        }
      },
      { passive: false }
    );

    // Pan & Canvas Background Mouse Down (1 left-click pan, middle-click pan, or shift+click marquee)
    svg.addEventListener("mousedown", (e: MouseEvent) => {
      // Middle click or space+click initiates viewport panning
      if (e.button === 1 || this.isSpaceDown) {
        e.preventDefault();
        this.panState = {
          startX: e.clientX,
          startY: e.clientY,
          origPanX: this.panX,
          origPanY: this.panY,
        };
        this.style.cursor = "grabbing";
        return;
      }

      // Background click: 1 Left-click drag pans canvas, Shift+Left-click initiates marquee box selection
      const target = e.target as HTMLElement;
      if (
        target.classList.contains("canvas-bg") ||
        target.classList.contains("canvas-bg-major") ||
        target.tagName === "svg"
      ) {
        if (e.button === 0) {
          e.preventDefault();
          if (e.shiftKey) {
            // Shift + Left Click drag initiates Marquee box selection
            if (!e.ctrlKey && !e.metaKey) {
              studioStore.clearSelection();
            }
            studioStore.cancelConnection();
            this.cursorPreviewPos = null;
            this.activeSnapTarget = null;

            const worldPos = this.getSvgCoordinates(e);
            this.marqueeState = {
              startMouseWorld: worldPos,
              currentMouseWorld: worldPos,
            };
            return;
          }

          // 1 Left-click drag on canvas background initiates viewport panning
          if (!e.ctrlKey && !e.metaKey) {
            studioStore.clearSelection();
          }
          studioStore.cancelConnection();
          this.cursorPreviewPos = null;
          this.activeSnapTarget = null;

          this.panState = {
            startX: e.clientX,
            startY: e.clientY,
            origPanX: this.panX,
            origPanY: this.panY,
          };
          this.style.cursor = "grabbing";
          return;
        }
      }
    });

    // Cancel draft button
    const cancelBtn = this.querySelector(".btn-cancel-draft");
    cancelBtn?.addEventListener("click", () => {
      studioStore.cancelConnection();
      this.cursorPreviewPos = null;
      this.activeSnapTarget = null;
    });

    // HTML5 Drag-and-Drop support from palette onto canvas
    svg.addEventListener("dragover", (e: DragEvent) => {
      e.preventDefault();
      if (e.dataTransfer) {
        e.dataTransfer.dropEffect = "copy";
      }
    });

    svg.addEventListener("drop", (e: DragEvent) => {
      e.preventDefault();
      const raw = e.dataTransfer?.getData("application/json");
      if (!raw) return;

      try {
        const payload = JSON.parse(raw);
        if (payload.templateId) {
          const templates = studioStore.getTemplates();
          const tmpl = templates.find((t) => t.id === payload.templateId);
          if (tmpl) {
            const coords = this.getSvgCoordinates(e);
            const palette = document.querySelector("nebu-equipment-palette") as any;
            if (palette && typeof palette.addTemplateToCanvas === "function") {
              palette.addTemplateToCanvas(tmpl, coords);
            }
          }
        }
      } catch (err) {
        console.error("[nebudcs-canvas] Drag drop failed:", err);
      }
    });

    // Equipment selection & dragging
    this.querySelectorAll(".eq-node").forEach((node) => {
      const eqId = node.getAttribute("data-eq-id");
      if (!eqId) return;

      node.addEventListener("click", (e) => {
        const target = e.target as HTMLElement;
        if (target.classList.contains("port-pin") || target.classList.contains("resize-handle")) return;
        e.stopPropagation();
        const me = e as MouseEvent;
        const isToggle = me.shiftKey || me.ctrlKey || me.metaKey;
        studioStore.selectItem("equipment", eqId, false, isToggle);
      });

      node.addEventListener("mousedown", (e) => {
        const me = e as MouseEvent;
        if (me.button !== 0 || this.isSpaceDown) return;
        const target = me.target as HTMLElement;
        if (target.classList.contains("port-pin") || target.classList.contains("resize-handle")) return;

        e.stopPropagation();

        const isMulti = me.shiftKey || me.ctrlKey || me.metaKey;
        if (isMulti) {
          studioStore.selectItem("equipment", eqId, false, true);
        } else if (!studioStore.isSelected(eqId)) {
          studioStore.selectItem("equipment", eqId);
        }

        const proj = studioStore.getProject();
        if (!proj) return;

        const selectedIds = studioStore.getSelectedIds();
        const itemsToDrag: DragItemState[] = [];

        for (const id of selectedIds) {
          const eqItem = proj.equipment.find((i) => i.id === id);
          if (eqItem) {
            const coords = this.getEquipmentCoordinates(eqItem);
            itemsToDrag.push({
              id: eqItem.id,
              type: "equipment",
              origX: coords.x,
              origY: coords.y,
            });
          }
        }

        if (itemsToDrag.length === 0) {
          const eqItem = proj.equipment.find((i) => i.id === eqId);
          if (eqItem) {
            const coords = this.getEquipmentCoordinates(eqItem);
            itemsToDrag.push({
              id: eqItem.id,
              type: "equipment",
              origX: coords.x,
              origY: coords.y,
            });
          }
        }

        const worldPos = this.getSvgCoordinates(me);
        this.dragState = {
          startMouseWorld: worldPos,
          items: itemsToDrag,
          hasMoved: false,
        };

        // Raise dragged DOM nodes to top of equipment layer to prevent visual overlap
        const eqLayer = this.querySelector(".equipment-layer");
        if (eqLayer) {
          for (const item of itemsToDrag) {
            const domNode = this.querySelector(`[data-eq-id="${item.id}"]`);
            if (domNode) eqLayer.appendChild(domNode);
          }
        }
      });
    });

    // Resize Handles Mousedown
    this.querySelectorAll(".resize-handle").forEach((handleEl) => {
      handleEl.addEventListener("mousedown", (e) => {
        const me = e as MouseEvent;
        if (me.button !== 0) return;
        e.stopPropagation();

        const handle = handleEl.getAttribute("data-handle") as "nw" | "ne" | "sw" | "se";
        const eqId = handleEl.getAttribute("data-eq-id");
        if (!eqId) return;

        const proj = studioStore.getProject();
        const eq = proj?.equipment.find((i) => i.id === eqId);
        if (!eq) return;

        const coords = this.getEquipmentCoordinates(eq);
        const arch = this.getEquipmentArchetype(eq);
        const defaultDims = ARCHETYPE_DIMS[arch] || { w: 60, h: 80 };
        let origW = eq.display_width ?? defaultDims.w;
        let origH = eq.display_height ?? defaultDims.h;

        if (this.activeDisplayFilter !== "all") {
          const allDisplays = (proj?.displays && proj?.displays.length > 0 ? proj.displays : proj?.plant_structure?.displays) || [];
          const disp = allDisplays.find((d) => d.id === this.activeDisplayFilter);
          const pl = disp?.placements?.find((p: any) => p.equipment_id === eq.id);
          if (pl) {
            if (pl.width) origW = pl.width;
            if (pl.height) origH = pl.height;
          }
        }

        const worldPos = this.getSvgCoordinates(me);
        this.resizeState = {
          equipmentId: eqId,
          handle,
          startMouseWorld: worldPos,
          origWidth: origW,
          origHeight: origH,
          origX: coords.x,
          origY: coords.y,
        };
      });
    });

    // Instrument node drag
    this.querySelectorAll(".instrument-node").forEach((node) => {
      const instId = node.getAttribute("data-inst-id");
      if (!instId) return;

      node.addEventListener("click", (e) => {
        e.stopPropagation();
        studioStore.selectItem("instrument", instId);
      });

      node.addEventListener("mousedown", (e) => {
        const me = e as MouseEvent;
        if (me.button !== 0 || this.isSpaceDown) return;
        e.stopPropagation();

        studioStore.selectItem("instrument", instId);
        const proj = studioStore.getProject();
        const inst = proj?.instruments.find((i) => i.id === instId);
        if (inst && inst.position) {
          const worldPos = this.getSvgCoordinates(me);
          this.dragState = {
            startMouseWorld: worldPos,
            items: [{ id: instId, type: "instrument", origX: inst.position.x, origY: inst.position.y }],
            hasMoved: false,
          };
        }
      });
    });

    // Stream selection
    this.querySelectorAll("[data-stream-id]").forEach((el) => {
      el.addEventListener("click", (e) => {
        e.stopPropagation();
        const streamId = el.getAttribute("data-stream-id");
        if (streamId) {
          studioStore.selectItem("stream", streamId);
        }
      });
    });

    // Port wiring interaction
    this.querySelectorAll(".port-pin").forEach((pin) => {
      pin.addEventListener("click", (e) => {
        e.stopPropagation();
        const eqId = pin.getAttribute("data-eq-id");
        const portId = pin.getAttribute("data-port-id");
        const portName = pin.getAttribute("data-port-name") || portId || "port";
        if (!eqId || !portId) return;

        const draft = studioStore.getConnectionDraft();
        if (!draft) {
          studioStore.startConnection(eqId, portId, portName);
        } else {
          if (draft.sourceEquipmentId === eqId) {
            studioStore.cancelConnection();
            this.cursorPreviewPos = null;
            this.activeSnapTarget = null;
            return;
          }

          const p1 = this.getPortAbsoluteCoords(draft.sourceEquipmentId, draft.sourcePortId);
          const p2 = this.getPortAbsoluteCoords(eqId, portId);
          const route_points = computeOrthogonalWaypoints(p1, p2);

          const proj = studioStore.getProject();
          const count = (proj?.streams.length ?? 0) + 1;
          const streamName = `S-${count < 10 ? "0" + count : count}`;
          const newStreamId = `st-${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`;

          studioStore.addStream({
            id: newStreamId,
            name: streamName,
            stream_type: "Material",
            source_equipment_id: draft.sourceEquipmentId,
            source_port_id: draft.sourcePortId,
            sink_equipment_id: eqId,
            sink_port_id: portId,
            composition: {},
            route_points,
          });

          this.cursorPreviewPos = null;
          this.activeSnapTarget = null;
        }
      });
    });
  }

  // -------------------------------------------------------------------------
  // High-Performance Window Event Handlers (No InnerHTML Re-Rendering during Drag)
  // -------------------------------------------------------------------------

  private onWindowMouseMove(e: MouseEvent) {
    const cursor = this.getSvgCoordinates(e);

    // Dispatch cursor coordinate event for bottom status bar
    this.dispatchEvent(
      new CustomEvent("canvas-cursor-move", {
        bubbles: true,
        detail: { x: cursor.x, y: cursor.y },
      })
    );

    // 1. Viewport Panning
    if (this.panState) {
      const dx = e.clientX - this.panState.startX;
      const dy = e.clientY - this.panState.startY;
      this.panX = this.panState.origPanX + dx;
      this.panY = this.panState.origPanY + dy;
      this.updateTransform();
      return;
    }

    // 2. Multi-Entity Dragging
    if (this.dragState) {
      const dx = cursor.x - this.dragState.startMouseWorld.x;
      const dy = cursor.y - this.dragState.startMouseWorld.y;
      if (Math.abs(dx) > 1 || Math.abs(dy) > 1) {
        this.dragState.hasMoved = true;
      }

      const movedPositions = new Map<string, Position2dDto>();

      for (const item of this.dragState.items) {
        const rawX = item.origX + dx;
        const rawY = item.origY + dy;
        const finalX = this.snapGrid ? Math.round(rawX / this.gridSize) * this.gridSize : Math.round(rawX);
        const finalY = this.snapGrid ? Math.round(rawY / this.gridSize) * this.gridSize : Math.round(rawY);
        movedPositions.set(item.id, { x: finalX, y: finalY });

        if (item.type === "equipment") {
          const node = this.querySelector(`[data-eq-id="${item.id}"]`);
          if (node) {
            const rot = node.getAttribute("data-rotation") || "0";
            node.setAttribute("transform", `translate(${finalX}, ${finalY}) rotate(${rot})`);
          }
        } else if (item.type === "instrument") {
          const node = this.querySelector(`.instrument-node[data-inst-id="${item.id}"]`);
          if (node) {
            node.setAttribute("transform", `translate(${finalX}, ${finalY})`);
          }
        }
      }

      // Live update connected streams' path `d` attribute at 60fps
      const proj = studioStore.getProject();
      if (proj) {
        for (const st of proj.streams) {
          const srcMoved = movedPositions.has(st.source_equipment_id);
          const snkMoved = movedPositions.has(st.sink_equipment_id);
          if (srcMoved || snkMoved) {
            const srcPos = movedPositions.get(st.source_equipment_id) || this.getPortAbsoluteCoords(st.source_equipment_id, st.source_port_id);
            const snkPos = movedPositions.get(st.sink_equipment_id) || this.getPortAbsoluteCoords(st.sink_equipment_id, st.sink_port_id);
            const waypoints = computeOrthogonalWaypoints(srcPos, snkPos);
            const newD = waypoints.map((p, idx) => (idx === 0 ? `M ${p.x} ${p.y}` : `L ${p.x} ${p.y}`)).join(" ");
            const pathEl = this.querySelector(`.process-pipe[data-stream-id="${st.id}"], .stream-group[data-stream-id="${st.id}"] .process-pipe`);
            if (pathEl) pathEl.setAttribute("d", newD);
          }
        }

        // Live collision detection & magnetic port docking preview
        const displayEquipment = proj.equipment.map((e) => ({
          ...e,
          position: this.getEquipmentCoordinates(e),
        }));

        if (this.dragState.items.length === 1 && this.dragState.items[0].type === "equipment") {
          const item = this.dragState.items[0];
          const eq = proj.equipment.find((e) => e.id === item.id);
          const w = eq?.display_width || 100;
          const h = eq?.display_height || 70;
          const proposedPos = movedPositions.get(item.id)!;
          const rot = eq?.rotation ?? 0;
          const scale = eq?.scale ?? 1.0;

          const colTest = testEquipmentCollision(proposedPos, w, h, displayEquipment, item.id, 24, {
            rotation: rot,
            scale,
            definition_id: eq?.definition_id,
          });
          this.lastCollisionStatus = { isColliding: colTest.isColliding, collidingIds: colTest.collidingIds };

          let dockInfo: any = null;
          const tmpl = eq?.definition_id ? findTemplateById(eq.definition_id) : undefined;
          if (tmpl) {
            const others = displayEquipment.filter((e) => e.id !== item.id);
            dockInfo = calculateMagneticPortDock(tmpl, proposedPos, others, 25);
            if (dockInfo?.isDocked) {
              movedPositions.set(item.id, dockInfo.snappedEquipmentPos);
              const node = this.querySelector(`[data-eq-id="${item.id}"]`);
              if (node) {
                const rot = node.getAttribute("data-rotation") || "0";
                node.setAttribute("transform", `translate(${dockInfo.snappedEquipmentPos.x}, ${dockInfo.snappedEquipmentPos.y}) rotate(${rot})`);
              }
            }
          }
          this.renderCollisionPreview(item, proposedPos, w, h, colTest.isColliding, colTest.collidingIds, dockInfo);
        } else if (this.dragState.items.length > 1) {
          const groupItems = this.dragState.items
            .filter((it) => it.type === "equipment")
            .map((it) => {
              const eq = proj.equipment.find((e) => e.id === it.id);
              return {
                id: it.id,
                origPos: { x: it.origX, y: it.origY },
                width: eq?.display_width || 100,
                height: eq?.display_height || 70,
                rotation: eq?.rotation ?? 0,
                scale: eq?.scale ?? 1.0,
                definition_id: eq?.definition_id,
              };
            });
          if (groupItems.length > 0) {
            const grpTest = testGroupCollision(groupItems, { x: dx, y: dy }, displayEquipment, 24);
            this.lastCollisionStatus = { isColliding: grpTest.isColliding, collidingIds: grpTest.collidingIds };
            this.renderGroupCollisionPreview(groupItems, movedPositions, grpTest.isColliding, grpTest.collidingIds);
          } else {
            this.lastCollisionStatus = null;
          }
        } else {
          this.lastCollisionStatus = null;
        }
      }
      return;
    }

    // 3. Equipment Resizing
    if (this.resizeState) {
      const dx = cursor.x - this.resizeState.startMouseWorld.x;
      const dy = cursor.y - this.resizeState.startMouseWorld.y;
      let newW = this.resizeState.origWidth;
      let newH = this.resizeState.origHeight;

      switch (this.resizeState.handle) {
        case "se":
          newW = Math.max(40, this.resizeState.origWidth + dx * 2);
          newH = Math.max(40, this.resizeState.origHeight + dy * 2);
          break;
        case "sw":
          newW = Math.max(40, this.resizeState.origWidth - dx * 2);
          newH = Math.max(40, this.resizeState.origHeight + dy * 2);
          break;
        case "ne":
          newW = Math.max(40, this.resizeState.origWidth + dx * 2);
          newH = Math.max(40, this.resizeState.origHeight - dy * 2);
          break;
        case "nw":
          newW = Math.max(40, this.resizeState.origWidth - dx * 2);
          newH = Math.max(40, this.resizeState.origHeight - dy * 2);
          break;
      }

      if (this.snapGrid) {
        newW = Math.round(newW / this.gridSize) * this.gridSize;
        newH = Math.round(newH / this.gridSize) * this.gridSize;
      }

      const node = this.querySelector(`[data-eq-id="${this.resizeState.equipmentId}"]`);
      const halo = node?.querySelector(".eq-selection-halo");
      if (halo) {
        const hw = Math.round(newW / 2);
        const hh = Math.round(newH / 2);
        halo.setAttribute("x", String(-hw - 4));
        halo.setAttribute("y", String(-hh - 4));
        halo.setAttribute("width", String(newW + 8));
        halo.setAttribute("height", String(newH + 8));
      }
      return;
    }

    // 4. Marquee Selection Drag
    if (this.marqueeState) {
      this.marqueeState.currentMouseWorld = cursor;
      const minX = Math.min(this.marqueeState.startMouseWorld.x, cursor.x);
      const maxX = Math.max(this.marqueeState.startMouseWorld.x, cursor.x);
      const minY = Math.min(this.marqueeState.startMouseWorld.y, cursor.y);
      const maxY = Math.max(this.marqueeState.startMouseWorld.y, cursor.y);

      const marqueeEl = this.querySelector(".selection-marquee") as SVGRectElement;
      if (marqueeEl) {
        marqueeEl.setAttribute("x", String(minX));
        marqueeEl.setAttribute("y", String(minY));
        marqueeEl.setAttribute("width", String(maxX - minX));
        marqueeEl.setAttribute("height", String(maxY - minY));
        marqueeEl.setAttribute("display", "block");
      }
      return;
    }

    // 5. Interactive Connection Draft Preview
    const draft = studioStore.getConnectionDraft();
    if (draft) {
      this.cursorPreviewPos = cursor;

      let foundSnap: {
        eqId: string;
        portId: string;
        portName: string;
        absX: number;
        absY: number;
        isValid: boolean;
      } | null = null;

      const allPins = this.querySelectorAll(".port-pin");
      for (let i = 0; i < allPins.length; i++) {
        const pin = allPins[i] as SVGCircleElement;
        const eqId = pin.getAttribute("data-eq-id");
        const portId = pin.getAttribute("data-port-id");
        const portName = pin.getAttribute("data-port-name") || portId || "";
        const direction = pin.getAttribute("data-port-direction") || "inlet";

        if (!eqId || !portId || eqId === draft.sourceEquipmentId) continue;

        const absCoords = this.getPortAbsoluteCoords(eqId, portId);
        const dist = Math.hypot(absCoords.x - cursor.x, absCoords.y - cursor.y);

        if (dist < 22) {
          const isValid = direction === "inlet" || direction === "bidirectional";
          foundSnap = {
            eqId,
            portId,
            portName,
            absX: absCoords.x,
            absY: absCoords.y,
            isValid,
          };
          break;
        }
      }

      this.activeSnapTarget = foundSnap;
      this.renderDraftPreview(draft, cursor);
    }
  }

  private onWindowMouseUp(e: MouseEvent) {
    if (this.panState) {
      this.panState = null;
      this.style.cursor = this.isSpaceDown ? "grab" : "default";
    }

    if (this.dragState) {
      if (this.dragState.hasMoved) {
        if (this.lastCollisionStatus?.isColliding) {
          // Reject move atomically if any item collides (preserving relative offsets and valid flowsheet state)
          this.render();
        } else {
          const cursor = this.getSvgCoordinates(e);
          const dx = cursor.x - this.dragState.startMouseWorld.x;
          const dy = cursor.y - this.dragState.startMouseWorld.y;

          const moves: Array<{ id: string; type: any; pos: Position2dDto }> = [];
          for (const item of this.dragState.items) {
            const rawX = item.origX + dx;
            const rawY = item.origY + dy;
            const finalX = Math.max(30, this.snapGrid ? Math.round(rawX / this.gridSize) * this.gridSize : Math.round(rawX));
            const finalY = Math.max(30, this.snapGrid ? Math.round(rawY / this.gridSize) * this.gridSize : Math.round(rawY));
            moves.push({ id: item.id, type: item.type, pos: { x: finalX, y: finalY } });
          }
          studioStore.moveEntities(
            moves,
            this.activeDisplayFilter !== "all" ? this.activeDisplayFilter : undefined
          );
        }
      }
      this.clearCollisionPreview();
      this.lastCollisionStatus = null;
      this.dragState = null;
    }

    if (this.resizeState) {
      const cursor = this.getSvgCoordinates(e);
      const dx = cursor.x - this.resizeState.startMouseWorld.x;
      const dy = cursor.y - this.resizeState.startMouseWorld.y;
      let newW = this.resizeState.origWidth;
      let newH = this.resizeState.origHeight;

      switch (this.resizeState.handle) {
        case "se":
          newW = Math.max(40, this.resizeState.origWidth + dx * 2);
          newH = Math.max(40, this.resizeState.origHeight + dy * 2);
          break;
        case "sw":
          newW = Math.max(40, this.resizeState.origWidth - dx * 2);
          newH = Math.max(40, this.resizeState.origHeight + dy * 2);
          break;
        case "ne":
          newW = Math.max(40, this.resizeState.origWidth + dx * 2);
          newH = Math.max(40, this.resizeState.origHeight - dy * 2);
          break;
        case "nw":
          newW = Math.max(40, this.resizeState.origWidth - dx * 2);
          newH = Math.max(40, this.resizeState.origHeight - dy * 2);
          break;
      }

      if (this.snapGrid) {
        newW = Math.round(newW / this.gridSize) * this.gridSize;
        newH = Math.round(newH / this.gridSize) * this.gridSize;
      }
      studioStore.resizeEquipment(
        this.resizeState.equipmentId,
        newW,
        newH,
        this.activeDisplayFilter !== "all" ? this.activeDisplayFilter : undefined
      );
      this.resizeState = null;
    }

    if (this.marqueeState) {
      const minX = Math.min(this.marqueeState.startMouseWorld.x, this.marqueeState.currentMouseWorld.x);
      const maxX = Math.max(this.marqueeState.startMouseWorld.x, this.marqueeState.currentMouseWorld.x);
      const minY = Math.min(this.marqueeState.startMouseWorld.y, this.marqueeState.currentMouseWorld.y);
      const maxY = Math.max(this.marqueeState.startMouseWorld.y, this.marqueeState.currentMouseWorld.y);

      if (maxX - minX > 8 || maxY - minY > 8) {
        const proj = studioStore.getProject();
        if (proj) {
          const enclosed = proj.equipment.filter((eq) => {
            const coords = this.getEquipmentCoordinates(eq);
            return coords.x >= minX && coords.x <= maxX && coords.y >= minY && coords.y <= maxY;
          });
          if (enclosed.length > 0) {
            studioStore.selectMultiple(enclosed.map((e) => ({ type: "equipment", id: e.id })));
          }
        }
      }
      const marqueeEl = this.querySelector(".selection-marquee") as SVGRectElement;
      if (marqueeEl) marqueeEl.setAttribute("display", "none");
      this.marqueeState = null;
    }
  }

  private renderDraftPreview(
    draft: { sourceEquipmentId: string; sourcePortId: string },
    cursor: Position2dDto
  ) {
    const draftLayer = this.querySelector(".draft-layer");
    if (!draftLayer) return;

    const srcPos = this.getPortAbsoluteCoords(
      draft.sourceEquipmentId,
      draft.sourcePortId
    );
    const destPos = this.activeSnapTarget
      ? { x: this.activeSnapTarget.absX, y: this.activeSnapTarget.absY }
      : cursor;

    const previewWaypoints = computeOrthogonalWaypoints(srcPos, destPos);
    const draftPathD = previewWaypoints
      .map((p, idx) => (idx === 0 ? `M ${p.x} ${p.y}` : `L ${p.x} ${p.y}`))
      .join(" ");

    const validClass = this.activeSnapTarget?.isValid ? "valid-snap" : "";
    draftLayer.innerHTML = `
      <path d="${draftPathD}" class="pipe-draft-preview ${validClass}" />
    `;
  }

  private renderCollisionPreview(
    _item: DragItemState,
    pos: Position2dDto,
    w: number,
    h: number,
    isColliding: boolean,
    _collidingIds: string[],
    dockInfo: any
  ) {
    const layer = this.querySelector(".collision-preview-layer");
    if (!layer) return;

    let html = "";
    if (dockInfo?.isDocked) {
      const dX = pos.x + dockInfo.draggedPort.xOffset;
      const dY = pos.y + dockInfo.draggedPort.yOffset;
      const tX = dockInfo.targetEquipment.position.x + dockInfo.targetPort.xOffset;
      const tY = dockInfo.targetEquipment.position.y + dockInfo.targetPort.yOffset;

      html += `
        <line x1="${dX}" y1="${dY}" x2="${tX}" y2="${tY}" stroke="#38bdf8" stroke-width="2.5" stroke-dasharray="4 2"/>
        <circle cx="${dX}" cy="${dY}" r="5" fill="#38bdf8"/>
        <circle cx="${tX}" cy="${tY}" r="5" fill="#38bdf8"/>
        <g transform="translate(${pos.x - 85}, ${pos.y - h / 2 - 25})">
          <rect width="170" height="18" rx="3" fill="#0f172a" stroke="#38bdf8" stroke-width="1"/>
          <text x="85" y="12" text-anchor="middle" fill="#38bdf8" font-family="monospace" font-size="9" font-weight="bold">⚡ DOCK: ${dockInfo.draggedPort.id} ──► ${dockInfo.targetPort.id}</text>
        </g>
      `;
    } else if (isColliding) {
      const halfW = w / 2;
      const halfH = h / 2;
      const pad = 24;
      html += `
        <rect x="${pos.x - halfW - pad}" y="${pos.y - halfH - pad}" width="${w + pad * 2}" height="${h + pad * 2 + 20}"
          fill="rgba(239, 68, 68, 0.15)" stroke="#ef4444" stroke-width="2" stroke-dasharray="4 2" rx="4"/>
        <g transform="translate(${pos.x - 90}, ${pos.y - halfH - 24})">
          <rect width="180" height="18" rx="3" fill="#450a0a" stroke="#ef4444" stroke-width="1"/>
          <text x="90" y="12" text-anchor="middle" fill="#fca5a5" font-family="monospace" font-size="9" font-weight="bold">⚠️ CLEARANCE CONFLICT</text>
        </g>
      `;
    } else {
      const halfW = w / 2;
      const halfH = h / 2;
      html += `
        <rect x="${pos.x - halfW - 6}" y="${pos.y - halfH - 6}" width="${w + 12}" height="${h + 12}"
          fill="none" stroke="#22c55e" stroke-width="1.5" stroke-dasharray="2 2" rx="2" opacity="0.6"/>
      `;
    }

    layer.innerHTML = html;
  }

  private renderGroupCollisionPreview(
    groupItems: Array<{ id: string; width: number; height: number }>,
    movedPositions: Map<string, Position2dDto>,
    isColliding: boolean,
    collidingIds: string[]
  ) {
    const layer = this.querySelector(".collision-preview-layer");
    if (!layer) return;

    let html = "";
    for (const item of groupItems) {
      const pos = movedPositions.get(item.id);
      if (!pos) continue;
      const w = item.width || 100;
      const h = item.height || 70;
      const halfW = w / 2;
      const halfH = h / 2;
      const itemColliding = isColliding && (collidingIds.includes(item.id) || collidingIds.length > 0);

      if (itemColliding) {
        const pad = 24;
        html += `
          <rect x="${pos.x - halfW - pad}" y="${pos.y - halfH - pad}" width="${w + pad * 2}" height="${h + pad * 2 + 20}"
            fill="rgba(239, 68, 68, 0.15)" stroke="#ef4444" stroke-width="2" stroke-dasharray="4 2" rx="4"/>
          <g transform="translate(${pos.x - 90}, ${pos.y - halfH - 24})">
            <rect width="180" height="18" rx="3" fill="#450a0a" stroke="#ef4444" stroke-width="1"/>
            <text x="90" y="12" text-anchor="middle" fill="#fca5a5" font-family="monospace" font-size="9" font-weight="bold">⚠️ GROUP CLEARANCE CONFLICT</text>
          </g>
        `;
      } else {
        html += `
          <rect x="${pos.x - halfW - 6}" y="${pos.y - halfH - 6}" width="${w + 12}" height="${h + 12}"
            fill="none" stroke="#22c55e" stroke-width="1.5" stroke-dasharray="2 2" rx="2" opacity="0.6"/>
        `;
      }
    }
    layer.innerHTML = html;
  }

  private clearCollisionPreview() {
    const layer = this.querySelector(".collision-preview-layer");
    if (layer) layer.innerHTML = "";
  }
}

customElements.define("nebu-pid-canvas", NebuPidCanvas);
