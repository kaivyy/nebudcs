/**
 * Equipment Palette Web Component.
 *
 * Provides an authoritative, searchable, categorized catalog of universal
 * process equipment, flow control, piping, instrumentation, electrical, safety,
 * refining, petrochemical, cement, power, chemical, LNG, and custom templates.
 *
 * Implements deterministic collision-free placement via findCollisionFreePosition.
 */

import { TemplateSummaryDto } from "../protocol/types";
import { studioStore } from "../state/studio-store";
import { findCollisionFreePosition } from "../domain/geometry-collision";
import { findTemplateById, registerCustomTemplate, STANDARD_REFS } from "../domain/equipment-template-catalog";

export class NebuEquipmentPalette extends HTMLElement {
  private searchQuery = "";
  private selectedCategory = "all";
  private unsubscribe: (() => void) | null = null;
  private isCreatingCustom = false;

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

  private filterTemplates(templates: TemplateSummaryDto[]): TemplateSummaryDto[] {
    return templates.filter((t) => {
      const q = this.searchQuery.toLowerCase();
      const matchesSearch =
        !this.searchQuery ||
        t.name.toLowerCase().includes(q) ||
        t.id.toLowerCase().includes(q) ||
        t.description.toLowerCase().includes(q) ||
        t.category.toLowerCase().includes(q);

      if (!matchesSearch) return false;
      if (this.selectedCategory === "all") return true;

      const catLower = t.category.toLowerCase();
      const selLower = this.selectedCategory.toLowerCase();

      if (selLower === "process" && (catLower.includes("process") || catLower.includes("rotating") || catLower.includes("heat") || catLower.includes("column") || catLower.includes("reactor"))) return true;
      if (selLower === "valves" && (catLower.includes("valve") || catLower.includes("flow"))) return true;
      if (selLower === "vessels" && (catLower.includes("vessel") || catLower.includes("tank") || catLower.includes("pipe") || catLower.includes("separator") || catLower.includes("filter") || catLower.includes("mixer"))) return true;
      if (selLower === "sensors" && (catLower.includes("instrument") || catLower.includes("sensor") || catLower.includes("electrical") || catLower.includes("safety") || catLower.includes("sis") || catLower.includes("control"))) return true;
      if (selLower === "industry" && (catLower.includes("refin") || catLower.includes("petro") || catLower.includes("cement") || catLower.includes("power") || catLower.includes("chemic") || catLower.includes("lng") || catLower.includes("gas") || catLower.includes("custom") || catLower.includes("multi") || (t as any).isCustom)) return true;

      return catLower.includes(selLower);
    });
  }

  public addTemplateToCanvas(template: TemplateSummaryDto, atPos?: { x: number; y: number }) {
    const proj = studioStore.getProject();
    if (!proj) return;

    const fullTmpl = findTemplateById(template.id);
    const w = fullTmpl?.defaultWidth || 110;
    const h = fullTmpl?.defaultHeight || 70;

    // Requested starting position
    const requestedPos = atPos || {
      x: 300 + (proj.equipment.length % 5) * 160,
      y: 200 + Math.floor(proj.equipment.length / 5) * 140,
    };

    // DETERMINISTIC COLLISION-FREE PLACEMENT
    const freePos = findCollisionFreePosition(requestedPos, w, h, proj.equipment, {
      gridSize: 20,
      clearance: 24,
      canvasWidth: 1920,
      canvasHeight: 1080,
      definition_id: template.id,
      streams: proj.streams || [],
    });

    if (template.id.startsWith("standard.instrument.") || template.category.toLowerCase().includes("instrument")) {
      const tagPrefix = this.getPrefixForTemplate(template);
      const existingCount = proj.instruments.filter((i) => i.tag.startsWith(tagPrefix)).length + 1;
      const tag = `${tagPrefix}-${100 + existingCount}`;
      const newId = `inst-${crypto.randomUUID ? crypto.randomUUID().slice(0, 8) : Date.now()}`;
      studioStore.addInstrument({
        id: newId,
        tag,
        measurement_kind: fullTmpl?.subtype || "Generic",
        position: freePos,
        range_min: 0,
        range_max: 100,
        unit: "%",
      });
      return;
    }

    const prefix = this.getPrefixForTemplate(template);
    const existingCount = proj.equipment.filter((e) => e.name.startsWith(prefix)).length + 1;
    const name = `${prefix}-${100 + existingCount}`;
    const newId = `eq-${crypto.randomUUID ? crypto.randomUUID().slice(0, 8) : Date.now()}`;

    studioStore.addEquipment({
      id: newId,
      definition_id: template.id,
      definition_version: template.version,
      name,
      parameters: {},
      position: freePos,
      display_width: w,
      display_height: h,
    });
  }

  private getPrefixForTemplate(tmpl: TemplateSummaryDto): string {
    const id = tmpl.id.toLowerCase();
    const cat = tmpl.category.toLowerCase();

    if (id.includes("pump")) return "P";
    if (id.includes("valve") && (id.includes("control") || id.includes("globe"))) return "FV";
    if (id.includes("valve") && id.includes("psv")) return "PSV";
    if (id.includes("valve") && id.includes("esd")) return "XV";
    if (id.includes("valve") && id.includes("on_off")) return "XV";
    if (id.includes("valve") && id.includes("check")) return "NRV";
    if (id.includes("valve") && id.includes("regulator")) return "PCV";
    if (id.includes("compressor")) return "C";
    if (id.includes("turbine")) return "ST";
    if (id.includes("fan") || id.includes("blower")) return "FN";
    if (id.includes("tank")) return "TK";
    if (id.includes("column") || id.includes("fractionator")) return "T";
    if (id.includes("furnace") || id.includes("heater") || id.includes("boiler")) return "H";
    if (id.includes("exchanger") || id.includes("cooler")) return "E";
    if (id.includes("separator") || id.includes("vessel") || id.includes("drum") || id.includes("crystallizer")) return "V";
    if (id.includes("filter") || id.includes("strainer")) return "FL";
    if (id.includes("mixer")) return "MIX";
    if (id.includes("reactor")) return "R";
    if (id.includes("kiln")) return "KILN";
    if (id.includes("pt")) return "PT";
    if (id.includes("pi")) return "PI";
    if (id.includes("tt")) return "TT";
    if (id.includes("ti")) return "TI";
    if (id.includes("ft")) return "FT";
    if (id.includes("fi")) return "FI";
    if (id.includes("lt")) return "LT";
    if (id.includes("li")) return "LI";
    if (id.includes("pdt")) return "PDT";
    if (id.includes("pdi")) return "PDI";
    if (id.includes("dp_transmitter")) return "PDT";
    if (id.includes("switch_pressure_low")) return "PSL";
    if (id.includes("switch_pressure")) return "PSH";
    if (id.includes("switch_temperature_low")) return "TSL";
    if (id.includes("switch_temperature")) return "TSH";
    if (id.includes("switch_flow")) return "FS";
    if (id.includes("switch_level_high")) return "LSH";
    if (id.includes("switch_level_low")) return "LSL";
    if (id.includes("analyzer_ph")) return "AT";
    if (id.includes("analyzer_gc")) return "AT";
    if (id.includes("analyzer")) return "AT";
    if (id.includes("control.fic")) return "FIC";
    if (id.includes("control.pic")) return "PIC";
    if (id.includes("control.tic")) return "TIC";
    if (id.includes("control.lic")) return "LIC";
    if (id.includes("control.ratio")) return "FFC";
    if (id.includes("control.selector")) return "FY";
    if (id.includes("interlock")) return "SIS";
    if (id.includes("orifice")) return "RO";
    if (id.includes("rupture_disk")) return "PSE";
    if (cat.includes("instrument")) return "IT";
    if (cat.includes("electrical")) return "M";
    if (cat.includes("safety")) return "SIS";
    return "EQ";
  }

  private getIconForTemplate(tmpl: TemplateSummaryDto): string {
    const id = tmpl.id.toLowerCase();
    if (id.includes("pump")) {
      return `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><circle cx="11" cy="13" r="7"/><path d="M 11 6 L 19 6 L 19 13"/><path d="M 11 13 L 16 9.5 L 11 6 Z" fill="currentColor" opacity="0.4"/></svg>`;
    } else if (id.includes("valve")) {
      return `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><polygon points="4,7 12,12 4,17"/><polygon points="20,7 12,12 20,17"/><line x1="12" y1="12" x2="12" y2="4"/><ellipse cx="12" cy="3.5" rx="4" ry="1.5" fill="currentColor"/></svg>`;
    } else if (id.includes("compressor") || id.includes("turbine")) {
      return `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><polygon points="5,8 19,4 19,20 5,16"/></svg>`;
    } else if (id.includes("column") || id.includes("fractionator")) {
      return `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="7" y="3" width="10" height="18" rx="2"/><line x1="7" y1="7" x2="17" y2="7"/><line x1="7" y1="11" x2="17" y2="11"/><line x1="7" y1="15" x2="17" y2="15"/></svg>`;
    } else if (id.includes("furnace") || id.includes("heater") || id.includes("boiler")) {
      return `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="5" y="9" width="14" height="11" rx="1"/><polygon points="9,9 11,4 13,4 15,9"/><line x1="12" y1="4" x2="12" y2="2"/><path d="M 8 18 Q 10 14 12 18 Q 14 14 16 18" fill="none" stroke="#f59e0b" stroke-width="1.5"/><line x1="4" y1="21" x2="20" y2="21" stroke="#eab308" stroke-width="1.5"/></svg>`;
    } else if (id.includes("heat") || id.includes("exchanger") || id.includes("cooler")) {
      return `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><circle cx="12" cy="12" r="8"/><path d="M 7 12 Q 9.5 7 12 12 T 17 12"/></svg>`;
    } else if (id.includes("tank")) {
      return `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="5" y="7" width="14" height="12" rx="1.5"/><path d="M 5 7 Q 12 4 19 7"/></svg>`;
    } else if (id.includes("reactor")) {
      return `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="6" y="5" width="12" height="14" rx="4"/><line x1="12" y1="2" x2="12" y2="15"/><ellipse cx="12" cy="15" rx="3.5" ry="1.5" fill="currentColor"/></svg>`;
    } else if (id.includes("instrument") || id.includes("sensor") || id.includes("pt") || id.includes("tt") || id.includes("ft") || id.includes("lt")) {
      return `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><circle cx="12" cy="12" r="7"/><line x1="5" y1="12" x2="19" y2="12"/></svg>`;
    }
    return `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="6" y="4" width="12" height="16" rx="3"/></svg>`;
  }

  render() {
    const templates = studioStore.getTemplates();
    const filtered = this.filterTemplates(templates);

    // Multi-industry hierarchical categories (6 primary functional groups)
    const categories = [
      { id: "all", label: "All Items" },
      { id: "process", label: "Process & Rotating" },
      { id: "valves", label: "Valves & Flow" },
      { id: "vessels", label: "Vessels & Piping" },
      { id: "sensors", label: "Sensors & Safety" },
      { id: "industry", label: "Multi-Industry & Custom" },
    ];

    this.innerHTML = `
      <div class="palette-container">
        <!-- Header & Search -->
        <div class="palette-header">
          <div class="palette-title-row">
            <span class="palette-caption">UNIVERSAL CATALOG</span>
            <span class="badge badge-dim mono">${filtered.length} Items</span>
            <button class="btn-create-custom" title="Create Custom Equipment Template">+ Custom</button>
          </div>
          <div class="palette-search-wrapper">
            <input
              type="text"
              class="palette-search"
              placeholder="Search equipment, standards, ISA tags..."
              value="${this.searchQuery}"
            />
            ${this.searchQuery ? `<button class="btn-clear-search" title="Clear">✕</button>` : ""}
          </div>
          <div class="palette-categories">
            ${categories
              .map(
                (c) => `
              <button
                class="cat-chip ${this.selectedCategory === c.id ? "active" : ""}"
                data-category="${c.id}"
              >${c.label}</button>
            `
              )
              .join("")}
          </div>
        </div>

        <!-- Custom Equipment Creation Modal Form (if active) -->
        ${
          this.isCreatingCustom
            ? `
          <div class="custom-template-modal">
            <div class="modal-title-row">
              <strong>Define Custom Equipment</strong>
              <button class="btn-close-modal">✕</button>
            </div>
            <div class="modal-field">
              <label>Template Name:</label>
              <input type="text" id="custName" value="Custom Agitated Reactor" />
            </div>
            <div class="modal-field">
              <label>Category:</label>
              <select id="custCat">
                <option value="Chemical Process">Chemical Process</option>
                <option value="Process Equipment">Process Equipment</option>
                <option value="Petrochemical">Petrochemical</option>
                <option value="Refining">Refining</option>
                <option value="Custom">Custom</option>
              </select>
            </div>
            <div class="modal-row">
              <div class="modal-field">
                <label>Width (px):</label>
                <input type="number" id="custW" value="120" />
              </div>
              <div class="modal-field">
                <label>Height (px):</label>
                <input type="number" id="custH" value="90" />
              </div>
            </div>
            <div class="modal-actions">
              <button class="btn-cancel-custom">Cancel</button>
              <button class="btn-save-custom">Create & Register</button>
            </div>
          </div>
        `
            : ""
        }

        <!-- Catalog Items List -->
        <div class="palette-list">
          <div class="palette-section">
            <div class="section-header">
              <span class="section-title">${this.selectedCategory.toUpperCase()} TEMPLATES</span>
              <span class="section-count">${filtered.length}</span>
            </div>
            <div class="section-items">
              ${
                filtered.length === 0
                  ? `<div class="palette-empty">No matching equipment templates found</div>`
                  : filtered
                      .map((t) => {
                        const fullTmpl = findTemplateById(t.id);
                        const stdRef = fullTmpl?.standardReferences?.[0]?.code || "ISO 10628";
                        const isDemo = fullTmpl?.specifications && Object.values(fullTmpl.specifications).some((s) => s.isDemoDefault);
                        return `
                    <div
                      class="template-card"
                      draggable="true"
                      data-id="${t.id}"
                      data-name="${t.name}"
                      title="${t.description} — Click or drag for collision-free placement"
                    >
                      <div class="template-glyph">
                        ${this.getIconForTemplate(t)}
                      </div>
                      <div class="template-info">
                        <div class="template-card-header">
                          <span class="template-title">${t.name}</span>
                          <span class="template-badge">${t.category}</span>
                        </div>
                        <div class="template-card-meta">
                          <span>${t.port_count} Ports</span>
                          <span class="std-badge">[${stdRef}]</span>
                          ${isDemo ? `<span class="demo-badge">[DEMO DEFAULT]</span>` : ""}
                        </div>
                      </div>
                      <button class="add-to-canvas-btn" data-id="${t.id}" title="Place collision-free on canvas">+</button>
                    </div>
                  `;
                      })
                      .join("")
              }
            </div>
          </div>
        </div>
      </div>
    `;

    this.bindEvents();
  }

  private bindEvents() {
    // Search input
    const searchInput = this.querySelector(".palette-search") as HTMLInputElement;
    if (searchInput) {
      searchInput.addEventListener("input", (e) => {
        this.searchQuery = (e.target as HTMLInputElement).value;
        this.render();
      });
    }

    // Clear search
    this.querySelector(".btn-clear-search")?.addEventListener("click", () => {
      this.searchQuery = "";
      this.render();
    });

    // Category chips
    this.querySelectorAll(".cat-chip").forEach((chip) => {
      chip.addEventListener("click", () => {
        const cat = chip.getAttribute("data-category");
        if (cat) {
          this.selectedCategory = cat;
          this.render();
        }
      });
    });

    // Custom Equipment button
    this.querySelector(".btn-create-custom")?.addEventListener("click", () => {
      this.isCreatingCustom = true;
      this.render();
    });

    this.querySelector(".btn-close-modal")?.addEventListener("click", () => {
      this.isCreatingCustom = false;
      this.render();
    });

    this.querySelector(".btn-cancel-custom")?.addEventListener("click", () => {
      this.isCreatingCustom = false;
      this.render();
    });

    this.querySelector(".btn-save-custom")?.addEventListener("click", () => {
      const name = (this.querySelector("#custName") as HTMLInputElement)?.value || "Custom Equipment";
      const cat = (this.querySelector("#custCat") as HTMLSelectElement)?.value || "Custom";
      const w = parseInt((this.querySelector("#custW") as HTMLInputElement)?.value || "100", 10);
      const h = parseInt((this.querySelector("#custH") as HTMLInputElement)?.value || "70", 10);

      const customId = `custom.equipment.${Date.now()}`;
      registerCustomTemplate({
        id: customId,
        category: cat,
        subtype: "custom",
        displayName: name,
        description: `Custom engineered ${name} definition.`,
        author: "Plant Engineer",
        defaultWidth: w,
        defaultHeight: h,
        minimumWidth: 60,
        minimumHeight: 40,
        ports: [
          { id: "INLET", name: "Inlet Nozzle", domain: "Material", direction: "INLET", xOffset: -Math.round(w / 2), yOffset: 0 },
          { id: "OUTLET", name: "Outlet Nozzle", domain: "Material", direction: "OUTLET", xOffset: Math.round(w / 2), yOffset: 0 },
        ],
        capabilities: ["MaterialTransfer"],
        specifications: {},
        standardReferences: [STANDARD_REFS.ISO_10628 || { code: "ISO 10628", title: "Graphical Symbols" }],
        collisionBounds: {
          width: w,
          height: h,
          labelHeight: 18,
          interactionPadding: 12,
          engineeringClearance: 24,
        },
        symbolDefinition: "custom",
        telemetryDefinitions: ["STATUS"],
        faceplateKind: "generic",
        isCustom: true,
      });

      const newTmpl: TemplateSummaryDto = {
        id: customId,
        name,
        version: "1.0.0",
        category: cat,
        source: "user_defined",
        description: `Custom engineered ${name} definition.`,
        parameter_count: 3,
        port_count: 2,
        curve_count: 1,
        trip_count: 1,
        author: "Plant Engineer",
        ports: [
          { id: "INLET", name: "Inlet Nozzle", domain: "Material", direction: "INLET" },
          { id: "OUTLET", name: "Outlet Nozzle", domain: "Material", direction: "OUTLET" },
        ],
      };

      studioStore.addCustomTemplate(newTmpl);
      this.isCreatingCustom = false;
      this.addTemplateToCanvas(newTmpl);
    });

    // Drag-and-Drop start
    this.querySelectorAll(".template-card").forEach((card) => {
      card.addEventListener("dragstart", (e) => {
        const id = card.getAttribute("data-id");
        if (!id) return;
        const de = e as DragEvent;
        if (de.dataTransfer) {
          de.dataTransfer.setData("application/json", JSON.stringify({ templateId: id }));
          de.dataTransfer.effectAllowed = "copy";
        }
      });

      // Quick-add button
      card.querySelector(".add-to-canvas-btn")?.addEventListener("click", (e) => {
        e.stopPropagation();
        const id = card.getAttribute("data-id");
        const tmpl = studioStore.getTemplates().find((t) => t.id === id);
        if (tmpl) {
          this.addTemplateToCanvas(tmpl);
        }
      });

      // Click card to place
      card.addEventListener("click", () => {
        const id = card.getAttribute("data-id");
        const tmpl = studioStore.getTemplates().find((t) => t.id === id);
        if (tmpl) {
          this.addTemplateToCanvas(tmpl);
        }
      });
    });
  }
}

if (!customElements.get("nebu-equipment-palette")) {
  customElements.define("nebu-equipment-palette", NebuEquipmentPalette);
}
