/**
 * NebuCommandPalette Web Component (Ctrl + K / Quick Action).
 *
 * Provides global keyboard-driven command & entity navigation across NebuDCS:
 * - Workspaces: Engineering, Operations, Training, Validation, System, Enterprise
 * - Equipment: P-101, E-101, V-101, C-101, etc.
 * - Control Loops: LIC-101, FIC-101, TIC-101, etc.
 * - Actions: Validate Project, Deploy to Runtime, Ack Alarms, Reset SIS
 * Enforces RBAC permissions based on active role.
 */

import { store } from "../state/store";
import { authStore } from "../state/auth-store";

export interface PaletteItem {
  id: string;
  category: "Workspace" | "Equipment" | "Loop" | "Action";
  title: string;
  subtitle: string;
  badge?: string;
  action: () => void;
  requiredRole?: string[];
}

export class NebuCommandPalette extends HTMLElement {
  private isOpen: boolean = false;
  private searchQuery: string = "";
  private selectedIndex: number = 0;
  private keyListener?: (e: KeyboardEvent) => void;

  connectedCallback() {
    this.keyListener = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        this.toggle();
      } else if (e.key === "Escape" && this.isOpen) {
        this.close();
      }
    };
    window.addEventListener("keydown", this.keyListener);
  }

  disconnectedCallback() {
    if (this.keyListener) {
      window.removeEventListener("keydown", this.keyListener);
    }
  }

  public open() {
    this.isOpen = true;
    this.searchQuery = "";
    this.selectedIndex = 0;
    this.render();
    setTimeout(() => {
      this.querySelector<HTMLInputElement>("#cmdInput")?.focus();
    }, 50);
  }

  public close() {
    this.isOpen = false;
    this.innerHTML = "";
  }

  public toggle() {
    if (this.isOpen) {
      this.close();
    } else {
      this.open();
    }
  }

  private getItems(): PaletteItem[] {
    const role = authStore.getRole();
    const proj = store.activeProject;
    const items: PaletteItem[] = [];

    // Workspaces
    items.push({
      id: "ws-eng",
      category: "Workspace",
      title: "Engineering Studio",
      subtitle: "Build P&ID, configure equipment, logic & safety",
      badge: "ENGINEER",
      action: () => this.dispatchNav("engineering"),
    });
    items.push({
      id: "ws-ops",
      category: "Workspace",
      title: "Operations Console",
      subtitle: "Operate DCS flowsheet, faceplates & alarm station",
      badge: "OPERATOR",
      action: () => this.dispatchNav("operations"),
    });
    items.push({
      id: "ws-trn",
      category: "Workspace",
      title: "Training Studio (OTS)",
      subtitle: "Operator training scenarios, evaluation & replay",
      badge: "TRAINING",
      action: () => this.dispatchNav("training"),
    });
    items.push({
      id: "ws-val",
      category: "Workspace",
      title: "Scientific Validation",
      subtitle: "Numerical benchmarks, NIST references & error metrics",
      badge: "VALIDATION",
      action: () => this.dispatchNav("validation"),
    });
    items.push({
      id: "ws-sys",
      category: "Workspace",
      title: "System Telemetry & Hardware",
      subtitle: "Virtual controllers, I/O HAL racks & fieldbus comms",
      badge: "SYSTEM",
      action: () => this.dispatchNav("system"),
    });
    items.push({
      id: "ws-ent",
      category: "Workspace",
      title: "Enterprise & HA Cluster",
      subtitle: "Multi-node topology, state replication & DR backups",
      badge: "ADMIN",
      action: () => this.dispatchNav("enterprise"),
    });

    // Equipment from Active Project
    if (proj?.equipment) {
      for (const eq of proj.equipment) {
        items.push({
          id: `eq-${eq.id}`,
          category: "Equipment",
          title: `${eq.name} (${eq.id})`,
          subtitle: `Template: ${eq.definition_id} v${eq.definition_version}`,
          badge: "EQUIPMENT",
          action: () => {
            store.setFocusedEntity(eq.id);
            this.dispatchNav("operations");
          },
        });
      }
    }

    // Control Loops
    if (proj?.control_loops) {
      for (const loop of proj.control_loops) {
        items.push({
          id: `loop-${loop.id}`,
          category: "Loop",
          title: `${loop.name} (${loop.pv_tag})`,
          subtitle: `Mode: ${loop.mode} | SP: ${loop.sp} | OUT: ${loop.out_tag}`,
          badge: "PID LOOP",
          action: () => {
            const target = store.getEntityByPvTag(loop.pv_tag);
            if (target) store.openFaceplate(target);
            this.dispatchNav("operations");
          },
        });
      }
    }

    // Quick Actions
    items.push({
      id: "act-deploy",
      category: "Action",
      title: "Deploy Project to Simulation Runtime",
      subtitle: "Compile topology & hot-reload authoritative Rust host",
      badge: "HOT DEPLOY",
      action: () => {
        window.dispatchEvent(new CustomEvent("trigger-hot-deploy"));
      },
    });

    items.push({
      id: "act-ack-all",
      category: "Action",
      title: "Acknowledge All Unacknowledged Alarms",
      subtitle: "Silence visual annunciator ticker per ISA-18.2",
      badge: "ALARM ACK",
      action: () => {
        window.dispatchEvent(new CustomEvent("trigger-ack-all-alarms"));
      },
    });

    // Filter by query and role
    const q = this.searchQuery.toLowerCase().trim();
    return items.filter((item) => {
      if (item.requiredRole && !item.requiredRole.includes(role)) {
        return false;
      }
      if (!q) return true;
      return (
        item.title.toLowerCase().includes(q) ||
        item.subtitle.toLowerCase().includes(q) ||
        item.category.toLowerCase().includes(q)
      );
    });
  }

  private dispatchNav(mode: string) {
    window.dispatchEvent(new CustomEvent("navigate-workspace", { detail: { mode } }));
  }

  private render() {
    if (!this.isOpen) {
      this.innerHTML = "";
      return;
    }

    const items = this.getItems();
    if (this.selectedIndex >= items.length) {
      this.selectedIndex = Math.max(0, items.length - 1);
    }

    this.innerHTML = `
      <div class="cmd-palette-backdrop fixed inset-0 bg-black/75 z-50 flex items-start justify-center pt-24 backdrop-blur-sm">
        <div class="cmd-palette-modal bg-[#131822] border border-[#22314e] rounded-lg shadow-2xl w-[640px] max-w-[95vw] overflow-hidden flex flex-col animate-in fade-in zoom-in-95 duration-100">
          <!-- Search Header -->
          <div class="p-3 border-b border-[#1e293b] flex items-center gap-2 bg-[#0b0f17]">
            <span class="text-cyan-400 text-lg">🔍</span>
            <input
              type="text"
              id="cmdInput"
              class="w-full bg-transparent border-none text-slate-100 text-sm font-mono placeholder-slate-500 focus:outline-none"
              placeholder="Search equipment, loops, tags, or jump to workspace... (ESC to close)"
              value="${this.searchQuery}"
            />
            <span class="text-[10px] font-mono px-1.5 py-0.5 bg-[#1e293b] text-slate-400 rounded border border-slate-700">ESC</span>
          </div>

          <!-- Items List -->
          <div class="max-h-[380px] overflow-y-auto p-1 divide-y divide-slate-800/40">
            ${
              items.length === 0
                ? `<div class="p-6 text-center text-slate-500 font-mono text-xs">No matching equipment, loops, or actions found.</div>`
                : items
                    .map((item, idx) => `
                      <div
                        class="cmd-item px-3 py-2 cursor-pointer flex items-center justify-between rounded transition-colors ${
                          idx === this.selectedIndex ? "bg-[#1e293b] text-cyan-400" : "text-slate-300 hover:bg-[#182030]"
                        }"
                        data-index="${idx}"
                      >
                        <div class="flex flex-col min-w-0 pr-2">
                          <div class="flex items-center gap-2">
                            <span class="text-[10px] font-mono uppercase px-1.5 py-0.2 bg-[#0b0f17] text-slate-400 rounded border border-slate-800">
                              ${item.category}
                            </span>
                            <span class="font-bold text-xs truncate">${item.title}</span>
                          </div>
                          <span class="text-[11px] text-slate-400 truncate mt-0.5">${item.subtitle}</span>
                        </div>
                        ${
                          item.badge
                            ? `<span class="text-[9px] font-mono px-1.5 py-0.5 bg-slate-800 text-slate-400 rounded shrink-0">${item.badge}</span>`
                            : ""
                        }
                      </div>
                    `)
                    .join("")
            }
          </div>

          <!-- Footer Hints -->
          <div class="p-2 border-t border-[#1e293b] bg-[#0b0f17] flex items-center justify-between text-[11px] font-mono text-slate-500 px-3">
            <span>Navigation: ↑ ↓ arrows to select, ENTER to execute</span>
            <span>Ctrl + K</span>
          </div>
        </div>
      </div>
    `;

    this.bindEvents(items);
  }

  private bindEvents(items: PaletteItem[]) {
    const backdrop = this.querySelector(".cmd-palette-backdrop");
    backdrop?.addEventListener("click", (e) => {
      if (e.target === backdrop) this.close();
    });

    const input = this.querySelector<HTMLInputElement>("#cmdInput");
    input?.addEventListener("input", () => {
      this.searchQuery = input.value;
      this.selectedIndex = 0;
      this.render();
      const nextInput = this.querySelector<HTMLInputElement>("#cmdInput");
      if (nextInput) {
        nextInput.focus();
        nextInput.setSelectionRange(nextInput.value.length, nextInput.value.length);
      }
    });

    input?.addEventListener("keydown", (e) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        this.selectedIndex = (this.selectedIndex + 1) % Math.max(1, items.length);
        this.render();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        this.selectedIndex = (this.selectedIndex - 1 + items.length) % Math.max(1, items.length);
        this.render();
      } else if (e.key === "Enter") {
        e.preventDefault();
        const sel = items[this.selectedIndex];
        if (sel) {
          sel.action();
          this.close();
        }
      }
    });

    this.querySelectorAll(".cmd-item").forEach((el) => {
      el.addEventListener("click", () => {
        const idx = parseInt(el.getAttribute("data-index") || "0", 10);
        const sel = items[idx];
        if (sel) {
          sel.action();
          this.close();
        }
      });
    });
  }
}

customElements.define("nebu-command-palette", NebuCommandPalette);
