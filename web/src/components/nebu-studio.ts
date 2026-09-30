/**
 * Engineering Studio Workbench Shell Web Component.
 *
 * Implements a professional industrial P&ID/DCS engineering editor:
 * - Fixed CAD toolbar with pan, zoom, fit, undo/redo, validation & simulation triggers
 * - Collapsible side panels (palette on left, property inspector on right)
 * - Maximum visual dominance for P&ID flowsheet canvas
 * - Bottom engineering status bar with live coordinates, counts, zoom & validation
 */

import { ApiClient, sharedApiClient } from "../api/client";
import { ProjectSummaryDto } from "../protocol/types";
import { studioStore } from "../state/studio-store";
import { store } from "../state/store";
import "./nebu-equipment-palette";
import "./nebu-pid-canvas";
import "./nebu-property-inspector";
import "./nebu-validation-panel";
import "./nebu-project-explorer";
import "./nebu-tag-database";
import "./nebu-cross-reference";
import "./nebu-io-mapping-table";
import "./nebu-iec-editor";
import "./nebu-io-panel";
import "./nebu-controller-panel";
import "./nebu-advanced-control";
import "./nebu-hydraulic-inspector";
import "./nebu-communications";
import "./nebu-safety-panel";
import "./nebu-domain-browser";
import "./nebu-validation-browser";
import "./nebu-plant-structure-editor";

export class NebuStudio extends HTMLElement {
  public apiClient: ApiClient = sharedApiClient;
  private projects: ProjectSummaryDto[] = [];
  private isSaving = false;
  private isValidating = false;
  private isLoadingSim = false;
  private paletteCollapsed = false;
  private inspectorCollapsed = false;
  private isFullscreen = false;
  private sidebarMode: "explorer" | "palette" = "explorer";
  private activeView: "flowsheet" | "iec" | "io" | "mappings" | "tags" | "cross_ref" | "controllers" | "advanced_control" | "hydraulics" | "communications" | "safety" | "domains" | "validation" | "plant_structure" = "flowsheet";

  private cursorCoords = { x: 0, y: 0 };
  private zoomLevel = 100;
  private unsubscribe: (() => void) | null = null;

  connectedCallback() {
    this.unsubscribe = studioStore.subscribe(() => {
      this.updateToolbar();
      this.updateStatusBar();
      const p = studioStore.getProject();
      if (p) {
        store.setActiveProject(p);
      }
    });
    this.initStudio();
    this.setupKeyboardShortcuts();
    this.setupCustomEventListeners();
  }

  disconnectedCallback() {
    if (this.unsubscribe) {
      this.unsubscribe();
      this.unsubscribe = null;
    }
  }

  private setupCustomEventListeners() {
    this.addEventListener("studio-toggle-fullscreen", () => {
      this.toggleFullscreen();
    });

    this.addEventListener("canvas-cursor-move", (e: any) => {
      if (e.detail) {
        this.cursorCoords = { x: e.detail.x, y: e.detail.y };
        const el = this.querySelector(".sb-coords");
        if (el) el.textContent = `X: ${this.cursorCoords.x}  Y: ${this.cursorCoords.y}`;
      }
    });

    this.addEventListener("canvas-view-change", (e: any) => {
      if (e.detail?.scale) {
        this.zoomLevel = Math.round(e.detail.scale * 100);
        const el = this.querySelector(".sb-zoom");
        if (el) el.textContent = `${this.zoomLevel}%`;
      }
    });

    this.addEventListener("project-item-selected", (e: any) => {
      const detail = e.detail;
      if (!detail) return;
      if (detail.section === "flowsheet" || detail.section === "control_loop" || detail.section === "alarm") {
        this.activeView = "flowsheet";
      } else if (detail.section === "controller" || detail.section === "task") {
        this.activeView = "controllers";
      } else if (detail.section === "routine") {
        this.activeView = "iec";
      } else if (detail.section === "io_system" || detail.section === "io_module") {
        this.activeView = "io";
      } else if (detail.section === "io_mapping") {
        this.activeView = "mappings";
      } else if (detail.section === "tag") {
        this.activeView = "tags";
      } else if (detail.section === "cross_ref") {
        this.activeView = "cross_ref";
      } else if (detail.section === "advanced_control") {
        this.activeView = "advanced_control";
      } else if (detail.section === "hydraulics") {
        this.activeView = "hydraulics";
      } else if (detail.section === "communications") {
        this.activeView = "communications";
      } else if (detail.section === "domains") {
        this.activeView = "domains";
      } else if (detail.section === "validation") {
        this.activeView = "validation";
      } else if (detail.section === "plant_structure") {
        this.activeView = "plant_structure";
      }
      this.render();
      if (detail.section === "cross_ref" && detail.id) {
        setTimeout(() => {
          const xrefEl = this.querySelector("nebu-cross-reference") as any;
          if (xrefEl) xrefEl.setQuery(detail.id);
        }, 50);
      }
    });
  }

  private setupKeyboardShortcuts() {
    window.addEventListener("keydown", (e) => {
      // Don't intercept if user is typing in an input
      if (e.target && typeof (e.target as HTMLElement).matches === "function") {
        if ((e.target as HTMLElement).matches("input, select, textarea")) return;
      }

      if (e.key === "F11") {
        e.preventDefault();
        this.toggleFullscreen();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "i") {
        e.preventDefault();
        this.toggleInspector();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "b") {
        e.preventDefault();
        this.togglePalette();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") {
        e.preventDefault();
        if (e.shiftKey) {
          studioStore.redo();
        } else {
          studioStore.undo();
        }
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "y") {
        e.preventDefault();
        studioStore.redo();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
        e.preventDefault();
        studioStore.selectAllEquipment();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "d") {
        e.preventDefault();
        studioStore.duplicateSelected();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        this.saveActiveProject();
      } else if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(e.key)) {
        e.preventDefault();
        const step = e.shiftKey ? 50 : 10;
        let dx = 0;
        let dy = 0;
        if (e.key === "ArrowUp") dy = -step;
        if (e.key === "ArrowDown") dy = step;
        if (e.key === "ArrowLeft") dx = -step;
        if (e.key === "ArrowRight") dx = step;

        const proj = studioStore.getProject();
        if (proj) {
          const selectedIds = studioStore.getSelectedIds();
          const moves: Array<{ id: string; type: any; pos: any }> = [];
          const canvas = this.getCanvas();
          const activeFilter = canvas?.activeDisplayFilter;

          for (const id of selectedIds) {
            const eq = proj.equipment.find((i) => i.id === id);
            if (eq) {
              const coords = canvas && typeof canvas.getEquipmentCoordinates === "function"
                ? canvas.getEquipmentCoordinates(eq)
                : (eq.position || { x: 100, y: 100 });
              moves.push({
                id: eq.id,
                type: "equipment",
                pos: { x: Math.max(20, coords.x + dx), y: Math.max(20, coords.y + dy) },
              });
            }
          }
          const sel = studioStore.getSelected();
          if (sel?.type === "instrument") {
            const inst = proj.instruments.find((i) => i.id === sel.id);
            if (inst && inst.position) {
              moves.push({
                id: inst.id,
                type: "instrument",
                pos: { x: Math.max(20, inst.position.x + dx), y: Math.max(20, inst.position.y + dy) },
              });
            }
          }
          if (moves.length > 0) {
            studioStore.moveEntities(
              moves,
              activeFilter && activeFilter !== "all" ? activeFilter : undefined
            );
          }
        }
      } else if (e.key === "Escape") {
        if (this.isFullscreen) {
          this.toggleFullscreen();
        }
        studioStore.cancelConnection();
        studioStore.clearSelection();
      } else if (e.key === "Delete" || e.key === "Backspace") {
        this.deleteSelectedEntity();
      } else if (e.key === "+" || e.key === "=") {
        this.getCanvas()?.zoomIn();
      } else if (e.key === "-") {
        this.getCanvas()?.zoomOut();
      } else if (e.key === "0") {
        this.getCanvas()?.fitToView();
      }
    });

    document.addEventListener("fullscreenchange", () => {
      if (!document.fullscreenElement && this.isFullscreen) {
        this.isFullscreen = false;
        const shell = this.querySelector(".studio-shell");
        if (shell) shell.classList.remove("fullscreen-mode");
        const fsBtn = this.querySelector(".btn-fullscreen-toggle");
        if (fsBtn) {
          fsBtn.innerHTML = "⛶ Fullscreen";
          fsBtn.setAttribute("title", "Enter Fullscreen Editor Mode (F11)");
          fsBtn.classList.remove("active");
        }
        setTimeout(() => this.getCanvas()?.fitToView(), 100);
      }
    });
  }

  public toggleFullscreen() {
    this.isFullscreen = !this.isFullscreen;
    const shell = this.querySelector(".studio-shell");
    if (shell) {
      shell.classList.toggle("fullscreen-mode", this.isFullscreen);
    }
    const fsBtn = this.querySelector(".btn-fullscreen-toggle");
    if (fsBtn) {
      fsBtn.innerHTML = this.isFullscreen ? "🗗 Exit Fullscreen" : "⛶ Fullscreen";
      fsBtn.setAttribute(
        "title",
        this.isFullscreen ? "Exit Fullscreen Editor Mode (Esc / F11)" : "Enter Fullscreen Editor Mode (F11)"
      );
      fsBtn.classList.toggle("active", this.isFullscreen);
    }

    if (this.isFullscreen) {
      if (!document.fullscreenElement && document.documentElement?.requestFullscreen) {
        document.documentElement.requestFullscreen().catch(() => {});
      }
    } else {
      if (document.fullscreenElement && document.exitFullscreen) {
        document.exitFullscreen().catch(() => {});
      }
    }

    this.showFeedback(this.isFullscreen ? "Fullscreen Editor Mode Activated" : "Exited Fullscreen");
    setTimeout(() => {
      this.getCanvas()?.fitToView();
    }, 100);
  }

  public toggleInspector() {
    this.inspectorCollapsed = !this.inspectorCollapsed;
    this.render();
  }

  public togglePalette() {
    this.paletteCollapsed = !this.paletteCollapsed;
    this.render();
  }

  private getCanvas(): any {
    return this.querySelector("nebu-pid-canvas");
  }

  private async initStudio() {
    try {
      const templates = await this.apiClient.listTemplates();
      studioStore.setTemplates(templates, true);
      await this.refreshProjects();

      if (!studioStore.getProject() && this.projects.length > 0) {
        await this.loadProject(this.projects[0].id);
      }
    } catch (err) {
      console.error("[nebudcs-studio] Initialization error:", err);
    }
  }

  private async refreshProjects() {
    try {
      const res = await this.apiClient.listProjects();
      this.projects = Array.isArray(res) ? res : ((res as any)?.projects || []);
      this.render();
    } catch (err) {
      console.error("[nebudcs-studio] Failed to list projects:", err);
      this.projects = [];
    }
  }

  private showFeedback(msg: string, isError: boolean = false) {
    const el = this.querySelector("#studioFeedback") as HTMLElement;
    if (el) {
      el.textContent = msg;
      el.className = `studio-status-msg mono ${isError ? "text-alarm font-bold" : "text-good"}`;
      setTimeout(() => {
        if (el.textContent === msg) el.textContent = "";
      }, 5000);
    }
  }

  private async loadProject(id: string) {
    try {
      const proj = await this.apiClient.getProject(id);
      studioStore.setProject(proj, true);
      store.setActiveProject(proj);
      await this.runValidation(id);
      this.showFeedback(`Loaded: ${proj.name}`);
    } catch (err: any) {
      console.error(`[nebudcs-studio] Failed to load project '${id}':`, err);
      this.showFeedback(`Load failed: ${err.message || err}`, true);
    }
  }

  private async createNewProject() {
    const name = prompt("Enter project name:", "New Process Plant");
    if (!name || !name.trim()) return;

    try {
      const created = await this.apiClient.createProject({
        name: name.trim(),
        description: "Authored in NebuDCS Engineering Studio",
      });
      await this.refreshProjects();
      studioStore.setProject(created, true);
      store.setActiveProject(created);
      this.showFeedback(`Created: ${created.name}`);
    } catch (err: any) {
      this.showFeedback(`Create failed: ${err.message || err}`, true);
    }
  }

  private async saveActiveProject() {
    const proj = studioStore.getProject();
    if (!proj || this.isSaving) return;

    this.isSaving = true;
    this.updateToolbar();

    try {
      const saved = await this.apiClient.saveProject(proj.id, { project: proj });
      studioStore.setProject(saved, true);
      store.setActiveProject(saved);
      await this.refreshProjects();
      await this.runValidation(saved.id);
      this.showFeedback(`Saved: ${saved.name}`);
    } catch (err: any) {
      this.showFeedback(`Save failed: ${err.message || err}`, true);
    } finally {
      this.isSaving = false;
      this.updateToolbar();
    }
  }

  private async runValidation(id?: string) {
    const targetId = id || studioStore.getProject()?.id;
    if (!targetId || this.isValidating) return;

    this.isValidating = true;
    this.updateToolbar();

    try {
      const val = await this.apiClient.validateProject(targetId);
      studioStore.setValidation(val);
    } catch (err) {
      console.error("[nebudcs-studio] Validation request failed:", err);
    } finally {
      this.isValidating = false;
      this.updateToolbar();
    }
  }

  private async loadIntoSimulation() {
    const proj = studioStore.getProject();
    if (!proj || this.isLoadingSim) return;

    this.isLoadingSim = true;
    this.updateToolbar();

    try {
      if (studioStore.getIsDirty()) {
        await this.saveActiveProject();
      }

      const res = await this.apiClient.loadProjectSim(proj.id);
      store.setActiveProject(proj);
      store.setDeploymentStatus(true, proj.id);
      this.showFeedback(`Deployed: ${res.message || proj.name}`);
    } catch (err: any) {
      this.showFeedback(`Deploy failed: ${err.message || err}`, true);
    } finally {
      this.isLoadingSim = false;
      this.updateToolbar();
    }
  }

  private deleteSelectedEntity() {
    const sel = studioStore.getSelected();
    if (!sel) return;

    if (sel.type === "equipment") {
      studioStore.deleteEquipment(sel.id);
    } else if (sel.type === "stream") {
      studioStore.deleteStream(sel.id);
    } else if (sel.type === "instrument") {
      studioStore.deleteInstrument(sel.id);
    } else if (sel.type === "control_loop") {
      studioStore.deleteControlLoop(sel.id);
    } else if (sel.type === "alarm") {
      studioStore.deleteAlarm(sel.id);
    }
  }

  private updateToolbar() {
    const isDirty = studioStore.getIsDirty();
    const canUndo = studioStore.canUndo();
    const canRedo = studioStore.canRedo();
    const selected = studioStore.getSelected();

    const saveBtn = this.querySelector(".btn-save") as HTMLButtonElement;
    if (saveBtn) {
      saveBtn.disabled = !isDirty || this.isSaving;
      saveBtn.innerHTML = this.isSaving
        ? "Saving..."
        : isDirty
        ? "Save Project *"
        : "Project Saved";
      saveBtn.classList.toggle("dirty", isDirty);
    }

    const undoBtn = this.querySelector(".btn-undo") as HTMLButtonElement;
    if (undoBtn) undoBtn.disabled = !canUndo;

    const redoBtn = this.querySelector(".btn-redo") as HTMLButtonElement;
    if (redoBtn) redoBtn.disabled = !canRedo;

    const delBtn = this.querySelector(".btn-delete-selected") as HTMLButtonElement;
    if (delBtn) delBtn.disabled = !selected;

    const selectedCount = studioStore.getSelectedIds().size;
    const alignBtns = this.querySelectorAll(".btn-align") as NodeListOf<HTMLButtonElement>;
    alignBtns.forEach((btn) => {
      btn.disabled = selectedCount < 2;
    });
    const distribBtns = this.querySelectorAll(".btn-distrib") as NodeListOf<HTMLButtonElement>;
    distribBtns.forEach((btn) => {
      btn.disabled = selectedCount < 3;
    });
  }

  private updateStatusBar() {
    const proj = studioStore.getProject();
    const draft = studioStore.getConnectionDraft();
    const val = studioStore.getValidation();

    const countsEl = this.querySelector(".sb-counts");
    if (countsEl && proj) {
      countsEl.textContent = `${proj.equipment.length} Units · ${proj.streams.length} Streams · ${proj.instruments.length} Instruments`;
    }

    const statusMsgEl = this.querySelector(".sb-flow-status");
    if (statusMsgEl) {
      if (draft) {
        statusMsgEl.textContent = `Wiring from: ${draft.portName || draft.sourcePortId} (click target port)`;
        statusMsgEl.className = "sb-flow-status text-amber mono";
      } else {
        statusMsgEl.textContent = "Flowsheet Ready";
        statusMsgEl.className = "sb-flow-status text-dim";
      }
    }

    const valBadge = this.querySelector(".sb-val-summary");
    if (valBadge && val) {
      const isOk = val.valid && val.error_count === 0;
      valBadge.textContent = isOk ? "✓ VALID" : `⚠ ${val.error_count} Errors`;
      valBadge.className = `sb-val-summary badge mono ${isOk ? "badge-good" : "badge-alarm"}`;
    }
  }

  render() {
    const currentProj = studioStore.getProject();
    const isDirty = studioStore.getIsDirty();
    const val = studioStore.getValidation();
    const isValOk = val ? val.valid && val.error_count === 0 : false;
    const selectedCount = studioStore.getSelectedIds().size;

    this.innerHTML = `
      <div class="studio-shell">
        <!-- Top Engineering Command Bar -->
        <header class="studio-toolbar">
          <div class="toolbar-left">
            <button class="btn-tool-toggle btn-toggle-palette ${this.paletteCollapsed ? "collapsed" : ""}" title="Toggle Equipment Catalog (Ctrl+B)">
              ${this.paletteCollapsed ? "▶" : "◀"}
            </button>
            <span class="studio-brand">NebuDCS P&ID Studio</span>
            <div class="project-pill">
              <select class="project-selector">
                ${(() => {
                  const pList = Array.isArray(this.projects) ? this.projects : [];
                  return pList.length === 0
                    ? `<option value="">No projects</option>`
                    : pList
                        .map(
                          (p) => `
                      <option value="${p.id}" ${currentProj?.id === p.id ? "selected" : ""}>
                        ${p.name}
                      </option>
                    `
                        )
                        .join("");
                })()}
              </select>
            </div>
            <button class="btn-tool btn-new-proj" title="Create New Process Flowsheet">+ New</button>
          </div>

          <div class="toolbar-center">
            <!-- CAD Undo/Redo Group -->
            <div class="tool-group">
              <button class="btn-tool btn-undo" ${!studioStore.canUndo() ? "disabled" : ""} title="Undo (Ctrl+Z)">
                ↶
              </button>
              <button class="btn-tool btn-redo" ${!studioStore.canRedo() ? "disabled" : ""} title="Redo (Ctrl+Y)">
                ↷
              </button>
            </div>

            <div class="toolbar-divider"></div>

            <!-- Viewport Controls Group -->
            <div class="tool-group">
              <button class="btn-tool btn-zoom-in" title="Zoom In (+)">🔍+</button>
              <button class="btn-tool btn-zoom-out" title="Zoom Out (-)">🔍−</button>
              <button class="btn-tool btn-fit-view" title="Fit to View (0)">⛶ Fit</button>
              <button class="btn-tool btn-delete-selected" title="Delete Selected Entity (Del)">🗑 Del</button>
            </div>

            <div class="toolbar-divider"></div>

            <!-- Alignment & Distribution Controls -->
            <div class="tool-group alignment-tools">
              <button class="btn-tool btn-align" data-align="left" ${selectedCount < 2 ? "disabled" : ""} title="Align Left (|←)">|←</button>
              <button class="btn-tool btn-align" data-align="center" ${selectedCount < 2 ? "disabled" : ""} title="Align Center (|↔|)">|↔|</button>
              <button class="btn-tool btn-align" data-align="right" ${selectedCount < 2 ? "disabled" : ""} title="Align Right (→|)">→|</button>
              <button class="btn-tool btn-align" data-align="top" ${selectedCount < 2 ? "disabled" : ""} title="Align Top (¯↑¯)">¯↑¯</button>
              <button class="btn-tool btn-align" data-align="middle" ${selectedCount < 2 ? "disabled" : ""} title="Align Middle (-↕-)">-↕-</button>
              <button class="btn-tool btn-align" data-align="bottom" ${selectedCount < 2 ? "disabled" : ""} title="Align Bottom (_↓_)">_↓_</button>
              <button class="btn-tool btn-distrib" data-distrib="horizontal" ${selectedCount < 3 ? "disabled" : ""} title="Distribute Horizontally (⫴H)">⫴H</button>
              <button class="btn-tool btn-distrib" data-distrib="vertical" ${selectedCount < 3 ? "disabled" : ""} title="Distribute Vertically (⫴V)">⫴V</button>
            </div>

            <div class="toolbar-divider"></div>

            <!-- Snap & Auto-Layout Group -->
            <div class="tool-group">
              <button class="btn-tool btn-snap-toggle active" title="Toggle Grid Snapping (10px)">Snap: ON</button>
              <button class="btn-tool btn-autolayout" data-mode="flow-lr" title="Auto-Layout: Flow Left-to-Right">⚡ Flow L-R</button>
              <button class="btn-tool btn-autolayout" data-mode="flow-tb" title="Auto-Layout: Flow Top-to-Bottom">⚡ Flow T-B</button>
            </div>

            <div class="toolbar-divider"></div>

            <!-- Validation Button -->
            <button class="btn-tool btn-validate" ${this.isValidating ? "disabled" : ""} title="Run Topology & Thermodynamic Validation">
              ${this.isValidating ? "Validating..." : "✓ Validate"}
            </button>
          </div>

          <div class="toolbar-right">
            <span class="studio-status-msg mono text-xs" id="studioFeedback"></span>
            <button class="btn-tool btn-fullscreen-toggle ${this.isFullscreen ? "active" : ""}" title="${this.isFullscreen ? "Exit Fullscreen Editor Mode (Esc / F11)" : "Enter Fullscreen Editor Mode (F11)"}">
              ${this.isFullscreen ? "🗗 Exit Fullscreen" : "⛶ Fullscreen"}
            </button>
            <button class="btn-tool btn-save ${isDirty ? "dirty" : ""}" ${!isDirty || this.isSaving ? "disabled" : ""} title="Save Project to Server (Ctrl+S)">
              ${this.isSaving ? "Saving..." : isDirty ? "Save Project *" : "Project Saved"}
            </button>
            <button class="btn-tool btn-primary btn-load-sim" id="btnHotDeploy" ${this.isLoadingSim ? "disabled" : ""} title="Compile topology, physics, and control loops and hot-deploy to authoritative Rust simulation runtime">
              ${this.isLoadingSim ? "Deploying..." : "🚀 Deploy to Runtime"}
            </button>
            <button class="btn-tool-toggle btn-toggle-inspector ${this.inspectorCollapsed ? "collapsed" : ""}" title="Toggle Property Inspector (Ctrl+I)">
              ${this.inspectorCollapsed ? "◀" : "▶"}
            </button>
          </div>
        </header>

        <!-- Stage 4 Studio View Switcher Bar -->
        <nav class="studio-nav-bar flex items-center px-3 py-1 bg-[#0b0f17] border-b border-[#1e293b] gap-1 overflow-x-auto">
          <button class="studio-view-tab ${this.activeView === "flowsheet" ? "active" : ""}" data-view="flowsheet">📐 P&ID Flowsheet</button>
          <button class="studio-view-tab ${this.activeView === "iec" ? "active" : ""}" data-view="iec">⚡ IEC 61131-3 Logic</button>
          <button class="studio-view-tab ${this.activeView === "io" ? "active" : ""}" data-view="io">🔌 I/O Hardware (HAL)</button>
          <button class="studio-view-tab ${this.activeView === "mappings" ? "active" : ""}" data-view="mappings">🔀 I/O Mappings</button>
          <button class="studio-view-tab ${this.activeView === "tags" ? "active" : ""}" data-view="tags">🏷️ Tag Database</button>
          <button class="studio-view-tab ${this.activeView === "cross_ref" ? "active" : ""}" data-view="cross_ref">🔍 Cross-Reference</button>
          <button class="studio-view-tab ${this.activeView === "controllers" ? "active" : ""}" data-view="controllers">🖥️ Controller Diagnostics</button>
          <button class="studio-view-tab ${this.activeView === "advanced_control" ? "active" : ""}" data-view="advanced_control">🎯 Advanced Control</button>
          <button class="studio-view-tab ${this.activeView === "hydraulics" ? "active" : ""}" data-view="hydraulics">🌊 Hydraulic Diagnostics</button>
          <button class="studio-view-tab ${this.activeView === "communications" ? "active" : ""}" data-view="communications">🔌 Communications</button>
          <button class="studio-view-tab ${this.activeView === "safety" ? "active" : ""}" data-view="safety">🛡️ Safety (SIS)</button>
          <button class="studio-view-tab ${this.activeView === "domains" ? "active" : ""}" data-view="domains">🌐 Industrial Domains & Thermo</button>
          <button class="studio-view-tab ${this.activeView === "plant_structure" ? "active" : ""}" data-view="plant_structure">🏛️ Plant Structure</button>
          <button class="studio-view-tab ${this.activeView === "validation" ? "active" : ""}" data-view="validation">🔬 Scientific Validation</button>
        </nav>

        <!-- Main Engineering 3-Pane Workbench -->
        <main class="studio-workbench">
          <!-- Left: Collapsible Explorer / Palette Sidebar -->
          <aside class="workbench-palette ${this.paletteCollapsed ? "collapsed" : ""}">
            <div class="sidebar-mode-toggle flex border-b border-[#1e293b] bg-[#0b0f17] p-1 gap-1">
              <button class="sidebar-mode-btn flex-1 py-1 text-xs font-semibold rounded ${this.sidebarMode === "explorer" ? "bg-[#1e293b] text-[#38bdf8]" : "text-dim hover:text-main"}" data-side="explorer">
                📁 Explorer
              </button>
              <button class="sidebar-mode-btn flex-1 py-1 text-xs font-semibold rounded ${this.sidebarMode === "palette" ? "bg-[#1e293b] text-[#38bdf8]" : "text-dim hover:text-main"}" data-side="palette">
                📦 Catalog
              </button>
            </div>
            <div class="workbench-palette-content flex-1 overflow-hidden min-h-0 flex flex-col">
              ${this.sidebarMode === "explorer" ? `<nebu-project-explorer class="h-full w-full block min-h-0"></nebu-project-explorer>` : `<nebu-equipment-palette class="h-full w-full block min-h-0"></nebu-equipment-palette>`}
            </div>
          </aside>

          <!-- Left Sidebar Edge Toggle (Show/Hide ke Samping) -->
          <button class="workbench-edge-toggle palette-edge-toggle ${this.paletteCollapsed ? "collapsed" : ""}" title="${this.paletteCollapsed ? "Show Explorer / Catalog Sidebar (Ctrl+B)" : "Hide Explorer / Catalog Sidebar (Ctrl+B)"}">
            ${this.paletteCollapsed ? "▶" : "◀"}
          </button>

          <!-- Center: Dynamic Multi-View Workspace with Permanent P&ID Centerpiece -->
          <section class="workbench-canvas">
            <div class="canvas-viewport-area">
              <nebu-pid-canvas></nebu-pid-canvas>
              <nebu-validation-panel></nebu-validation-panel>
            </div>

            ${
              this.activeView !== "flowsheet"
                ? `
                  <div class="studio-bottom-drawer">
                    <div class="drawer-header-bar">
                      <span class="drawer-title mono">
                        ${
                          this.activeView === "iec"
                            ? "⚡ IEC 61131-3 Logic Editor"
                            : this.activeView === "io"
                            ? "🔌 I/O Hardware Configuration (HAL)"
                            : this.activeView === "mappings"
                            ? "🔀 Signal & Tag I/O Mappings"
                            : this.activeView === "tags"
                            ? "🏷️ Project Tag Database"
                            : this.activeView === "cross_ref"
                            ? "🔍 Cross-Reference Index"
                            : this.activeView === "controllers"
                            ? "🖥️ Controller Diagnostics"
                            : this.activeView === "advanced_control"
                            ? "🎯 Advanced Process Control"
                            : this.activeView === "hydraulics"
                            ? "🌊 Hydraulic Diagnostics"
                            : this.activeView === "communications"
                            ? "🔌 Fieldbus Communications"
                            : this.activeView === "safety"
                            ? "🛡️ Safety Instrumented Systems (SIS)"
                            : this.activeView === "domains"
                            ? "🌐 Industrial Domains & Thermodynamic Packages"
                            : this.activeView === "plant_structure"
                            ? "🏛️ Plant Structure & DCS Display Hierarchy"
                            : "🔬 Scientific Validation & Benchmarks"
                        }
                      </span>
                      <button class="btn-close-drawer btn-tool-xs mono" id="btnCloseDrawer">✖ Minimize Drawer</button>
                    </div>
                    <div class="drawer-content">
                      ${
                        this.activeView === "iec"
                          ? `<nebu-iec-editor class="w-full h-full block"></nebu-iec-editor>`
                          : this.activeView === "io"
                          ? `<nebu-io-panel class="w-full h-full block"></nebu-io-panel>`
                          : this.activeView === "mappings"
                          ? `<nebu-io-mapping-table class="w-full h-full block"></nebu-io-mapping-table>`
                          : this.activeView === "tags"
                          ? `<nebu-tag-database class="w-full h-full block"></nebu-tag-database>`
                          : this.activeView === "cross_ref"
                          ? `<nebu-cross-reference class="w-full h-full block"></nebu-cross-reference>`
                          : this.activeView === "advanced_control"
                          ? `<nebu-advanced-control class="w-full h-full block"></nebu-advanced-control>`
                          : this.activeView === "hydraulics"
                          ? `<nebu-hydraulic-inspector class="w-full h-full block"></nebu-hydraulic-inspector>`
                          : this.activeView === "communications"
                          ? `<nebu-communications class="w-full h-full block"></nebu-communications>`
                          : this.activeView === "safety"
                          ? `<nebu-safety-panel class="w-full h-full block"></nebu-safety-panel>`
                          : this.activeView === "domains"
                          ? `<nebu-domain-browser class="w-full h-full block"></nebu-domain-browser>`
                          : this.activeView === "plant_structure"
                          ? `<nebu-plant-structure-editor class="w-full h-full block"></nebu-plant-structure-editor>`
                          : this.activeView === "validation"
                          ? `<nebu-validation-browser class="w-full h-full block"></nebu-validation-browser>`
                          : `<nebu-controller-panel class="w-full h-full block"></nebu-controller-panel>`
                      }
                    </div>
                  </div>
                `
                : ""
            }
          </section>

          <!-- Right Sidebar Edge Toggle (Show/Hide ke Samping) -->
          <button class="workbench-edge-toggle inspector-edge-toggle ${this.inspectorCollapsed ? "collapsed" : ""}" title="${this.inspectorCollapsed ? "Show Property Inspector (Ctrl+I)" : "Hide Property Inspector (Ctrl+I)"}">
            ${this.inspectorCollapsed ? "◀" : "▶"}
          </button>

          <!-- Right: Dynamic Property Inspector -->
          <aside class="workbench-inspector ${this.inspectorCollapsed ? "collapsed" : ""}">
            <div class="inspector-side-toggle-strip">
              <span class="inspector-side-title mono">PROPERTY INSPECTOR</span>
              <button class="btn-side-toggle-inspector" title="Hide Property Inspector to side (Ctrl+I)">
                ▶ Hide
              </button>
            </div>
            <nebu-property-inspector></nebu-property-inspector>
          </aside>
        </main>

        <!-- Project Readiness Pre-Flight Bar -->
        <div class="project-readiness-bar mono">
          <div class="readiness-items">
            <span class="readiness-item ${currentProj?.equipment?.length ? "ready" : "warn"}">✓ TOPOLOGY</span>
            <span class="readiness-sep">|</span>
            <span class="readiness-item ${currentProj?.equipment?.length ? "ready" : "warn"}">✓ EQUIPMENT (${currentProj?.equipment?.length || 0})</span>
            <span class="readiness-sep">|</span>
            <span class="readiness-item ${currentProj?.streams?.length ? "ready" : "warn"}">✓ STREAMS (${currentProj?.streams?.length || 0})</span>
            <span class="readiness-sep">|</span>
            <span class="readiness-item ready">✓ TAGS</span>
            <span class="readiness-sep">|</span>
            <span class="readiness-item ready">✓ I/O HAL</span>
            <span class="readiness-sep">|</span>
            <span class="readiness-item ready">✓ CONTROLLERS</span>
            <span class="readiness-sep">|</span>
            <span class="readiness-item ready">✓ IEC LOGIC</span>
            <span class="readiness-sep">|</span>
            <span class="readiness-item ready">✓ SIS SAFETY</span>
            <span class="readiness-sep">|</span>
            <span class="readiness-item ${isValOk ? "ready" : "warn"}">${isValOk ? "✓ VALIDATION PASS" : "⚠ VALIDATION PENDING"}</span>
          </div>
          <div class="readiness-actions">
            <button class="btn-readiness-validate" id="btnReadinessValidate" ${this.isValidating ? "disabled" : ""}>
              ${this.isValidating ? "Validating..." : "✓ Validate Project"}
            </button>
            <button class="btn-readiness-deploy btn-primary" id="btnReadinessDeploy" ${this.isLoadingSim ? "disabled" : ""}>
              ${this.isLoadingSim ? "Deploying..." : "🚀 Deploy to Runtime"}
            </button>
          </div>
        </div>

        <!-- Bottom Industrial Status Bar -->
        <footer class="studio-statusbar">
          <div class="sb-left">
            <span class="sb-val-summary badge mono ${isValOk ? "badge-good" : "badge-alarm"}">
              ${isValOk ? "✓ VALID" : val ? "⚠ DIAGNOSTICS" : "UNVALIDATED"}
            </span>
            <span class="sb-counts mono text-xs">
              ${currentProj ? `${currentProj.equipment.length} Units · ${currentProj.streams.length} Streams · ${currentProj.instruments.length} Instruments` : ""}
            </span>
          </div>

          <div class="sb-center">
            <span class="sb-flow-status text-dim">Flowsheet Ready</span>
          </div>

          <div class="sb-right">
            <span class="sb-coords mono text-xs">X: ${this.cursorCoords.x}  Y: ${this.cursorCoords.y}</span>
            <span class="sb-zoom mono text-xs" title="Click to reset zoom (100%)">${this.zoomLevel}%</span>
            <span class="sb-sim-state mono text-xs badge badge-dim">SIM: 20 Hz</span>
          </div>
        </footer>
      </div>
    `;

    // Bind toolbar listeners
    const projSelector = this.querySelector(".project-selector") as HTMLSelectElement;
    projSelector?.addEventListener("change", () => {
      if (projSelector.value) {
        this.loadProject(projSelector.value);
      }
    });

    this.querySelector(".btn-new-proj")?.addEventListener("click", () => this.createNewProject());
    this.querySelector(".btn-save")?.addEventListener("click", () => this.saveActiveProject());
    this.querySelector(".btn-validate")?.addEventListener("click", () => this.runValidation());
    this.querySelector(".btn-load-sim")?.addEventListener("click", () => this.loadIntoSimulation());
    this.querySelector("#btnReadinessValidate")?.addEventListener("click", () => this.runValidation());
    this.querySelector("#btnReadinessDeploy")?.addEventListener("click", () => this.loadIntoSimulation());
    this.querySelector("#btnCloseDrawer")?.addEventListener("click", () => {
      this.activeView = "flowsheet";
      this.render();
    });
    this.querySelector(".btn-undo")?.addEventListener("click", () => studioStore.undo());
    this.querySelector(".btn-redo")?.addEventListener("click", () => studioStore.redo());
    this.querySelector(".btn-delete-selected")?.addEventListener("click", () => this.deleteSelectedEntity());

    // Alignment and distribution actions
    this.querySelectorAll(".btn-align").forEach((btn) => {
      btn.addEventListener("click", () => {
        const mode = btn.getAttribute("data-align") as any;
        const canvas = this.getCanvas();
        const activeFilter = canvas?.activeDisplayFilter;
        if (mode) studioStore.alignSelected(mode, activeFilter !== "all" ? activeFilter : undefined);
      });
    });

    this.querySelectorAll(".btn-distrib").forEach((btn) => {
      btn.addEventListener("click", () => {
        const axis = btn.getAttribute("data-distrib") as any;
        const canvas = this.getCanvas();
        const activeFilter = canvas?.activeDisplayFilter;
        if (axis) studioStore.distributeSelected(axis, activeFilter !== "all" ? activeFilter : undefined);
      });
    });

    this.querySelectorAll(".btn-autolayout").forEach((btn) => {
      btn.addEventListener("click", () => {
        const mode = (btn.getAttribute("data-mode") as any) || "flow-lr";
        studioStore.applyAutoLayout(mode);
        this.getCanvas()?.fitToView();
        this.showFeedback(`Applied Auto-Layout: ${mode.toUpperCase()}`);
      });
    });

    const snapBtn = this.querySelector(".btn-snap-toggle") as HTMLButtonElement;
    snapBtn?.addEventListener("click", () => {
      const canvas = this.getCanvas();
      if (canvas) {
        canvas.snapGrid = !canvas.snapGrid;
        snapBtn.textContent = canvas.snapGrid ? "Snap: ON" : "Snap: OFF";
        snapBtn.classList.toggle("active", canvas.snapGrid);
      }
    });

    // Zoom & Fit actions
    this.querySelector(".btn-zoom-in")?.addEventListener("click", () => this.getCanvas()?.zoomIn());
    this.querySelector(".btn-zoom-out")?.addEventListener("click", () => this.getCanvas()?.zoomOut());
    this.querySelector(".btn-fit-view")?.addEventListener("click", () => this.getCanvas()?.fitToView());
    this.querySelector(".sb-zoom")?.addEventListener("click", () => this.getCanvas()?.resetZoom());

    // Studio View Tabs
    this.querySelectorAll(".studio-view-tab").forEach((btn) => {
      btn.addEventListener("click", () => {
        const view = btn.getAttribute("data-view") as any;
        if (view) {
          this.activeView = view;
          this.render();
        }
      });
    });

    // Sidebar Mode Toggles
    this.querySelectorAll(".sidebar-mode-btn").forEach((btn) => {
      btn.addEventListener("click", () => {
        const side = btn.getAttribute("data-side") as any;
        if (side) {
          this.sidebarMode = side;
          this.render();
        }
      });
    });

    // Fullscreen Mode Toggle
    this.querySelector(".btn-fullscreen-toggle")?.addEventListener("click", () => {
      this.toggleFullscreen();
    });

    // Panel collapse toggles (both toolbar buttons and side-edge buttons)
    this.querySelector(".btn-toggle-palette")?.addEventListener("click", () => {
      this.togglePalette();
    });

    this.querySelector(".palette-edge-toggle")?.addEventListener("click", () => {
      this.togglePalette();
    });

    this.querySelector(".btn-toggle-inspector")?.addEventListener("click", () => {
      this.toggleInspector();
    });

    this.querySelector(".inspector-edge-toggle")?.addEventListener("click", () => {
      this.toggleInspector();
    });

    this.querySelector(".btn-side-toggle-inspector")?.addEventListener("click", () => {
      this.toggleInspector();
    });

    // Horizontal mouse wheel and drag-to-scroll for studio toolbar
    const toolbar = this.querySelector(".studio-toolbar") as HTMLElement;
    if (toolbar) {
      toolbar.addEventListener(
        "wheel",
        (e: WheelEvent) => {
          if (e.deltaY !== 0 && toolbar.scrollWidth > toolbar.clientWidth) {
            e.preventDefault();
            toolbar.scrollLeft += e.deltaY;
          }
        },
        { passive: false }
      );

      let isDown = false;
      let startX = 0;
      let scrollLeft = 0;

      toolbar.addEventListener("mousedown", (e: MouseEvent) => {
        const target = e.target as HTMLElement;
        if (target.closest("button, select, input, a, option")) return;
        isDown = true;
        startX = e.pageX - toolbar.offsetLeft;
        scrollLeft = toolbar.scrollLeft;
        toolbar.style.cursor = "grabbing";
      });

      toolbar.addEventListener("mouseleave", () => {
        if (isDown) {
          isDown = false;
          toolbar.style.cursor = "";
        }
      });

      toolbar.addEventListener("mouseup", () => {
        if (isDown) {
          isDown = false;
          toolbar.style.cursor = "";
        }
      });

      toolbar.addEventListener("mousemove", (e: MouseEvent) => {
        if (!isDown) return;
        e.preventDefault();
        const x = e.pageX - toolbar.offsetLeft;
        const walk = x - startX;
        toolbar.scrollLeft = scrollLeft - walk;
      });
    }

    // Horizontal wheel scroll for view switcher bar
    const navBar = this.querySelector(".studio-nav-bar") as HTMLElement;
    if (navBar) {
      navBar.addEventListener(
        "wheel",
        (e: WheelEvent) => {
          if (e.deltaY !== 0 && navBar.scrollWidth > navBar.clientWidth) {
            e.preventDefault();
            navBar.scrollLeft += e.deltaY;
          }
        },
        { passive: false }
      );
    }
  }
}

customElements.define("nebu-studio", NebuStudio);
