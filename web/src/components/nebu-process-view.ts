/**
 * NebuDCS High-Performance Process Graphic Workstation (ISA-101 / ISA-5.1 / IEC 63303).
 *
 * Professional DCS/SCADA Process Graphic Engine:
 * 1. Process Graphic First:
 *    - Dominates operator station with high-density, structured industrial flowsheet
 *    - Recognizable solid equipment bodies (columns with trays/sump, furnaces with active flames,
 *      pumps with volute casings and rotating motors, compressors with anti-surge, heat exchangers
 *      with tube sheets, 3-phase separators with weirs/boots, atmospheric tanks, control valves)
 *
 * 2. Industrial Piping & Flow Grammar:
 *    - Heavy primary process lines (4.5px solid) with distinct boundaries
 *    - Secondary process lines (3.0px solid)
 *    - Color-coded utility lines (HP/MP/LP Steam, Cooling Water, Fuel Gas, Instrument Air)
 *    - Unambiguous crossing grammar: semicircular bridge loops or gaps on crossing lines
 *    - Weld tees: prominent 4.5px junction dots at physical connections
 *    - Dynamic embedded flow chevrons showing real-time direction and velocity
 *    - Off-page TO/FROM connector arrow badges linking to adjacent process units
 *
 * 3. Industrial Instrumentation (ISA-5.1):
 *    - Standard 28px diameter bubbles with horizontal DCS dividing line
 *    - Attached high-contrast PV callout pills with engineering units
 *    - ISA-101 quality flags ([V] Good, [?] Uncertain, [!] Bad, [X] Comm Fail)
 *    - ISA-18.2 alarm severity halos and blinking alert tags (HIHI, HI, LO, LOLO)
 *
 * 4. Dedicated Multi-Display Hierarchy:
 *    - CDU-100 Overview (disp-unit-01): Complete balanced unit process flowsheet
 *    - Feed & Pre-Heat (disp-feed-preheat): Storage tanks, charge pumps A/B, exchanger battery
 *    - Furnace H-101 (disp-furnace): Refractory heater, 4 burners with flames, skin TCs, draft
 *    - Column T-101 (disp-column): 30 fractionating trays, condenser, reflux drum, sidestreams
 *    - Compression C-301 (disp-compression): Multi-stage compressor, scrubber, anti-surge loop
 *    - Separation V-103 (disp-separation): 3-phase separator with weir, boot level, gas backpressure
 *    - Utilities Network (disp-utilities): HP/MP/LP steam headers, cooling water, fuel gas
 */

import { store, Store, DisplayViewModel } from "../state/store";
import {
  BoundaryConditionDto,
  FaceplateKind,
  ProjectDetailDto,
  StudioControlLoopDto,
  StudioEquipmentDto,
  StudioInstrumentDto,
  StudioStreamDto,
} from "../protocol/types";
import {
  calculateEquipmentBounds,
  determineOptimalTelemetryZone,
} from "../domain/geometry-collision";

// ---------------------------------------------------------------------------
// Utility: derive equipment kind from name, parameters, and definition ID
// ---------------------------------------------------------------------------
export function equipmentKind(
  eq?: { name?: string; id?: string; definition_id?: string; parameters?: Record<string, number> } | null,
  defId?: string
): FaceplateKind {
  const d = (defId || eq?.definition_id || "").toLowerCase();
  const name = (eq?.name || "").toLowerCase();
  const id = (eq?.id || "").toLowerCase();
  const params = eq?.parameters || {};

  // 1. Authoritative Definition ID match
  if (d.includes("pump")) return "pump";
  if (d.includes("compressor")) return "compressor";
  if (d.includes("valve")) return "valve";
  if (d.includes("furnace") || d.includes("heater") || d.includes("fired")) return "furnace" as any;
  if (d.includes("exchanger") || d.includes("cooler") || d.includes("reboiler") || d.includes("condenser")) return "heat_exchanger";
  if (d.includes("column") || d.includes("fractionat")) return "column";
  if (d.includes("reactor")) return "reactor";
  if (d.includes("vessel") || d.includes("drum")) return "vessel";
  if (d.includes("separator")) return "separator";
  if (d.includes("tank")) return "tank";
  if (d.includes("turbine")) return "turbine";

  // 2. Explicit name / tag prefix patterns (ISA-5.1)
  if (name.includes("pump") || name.startsWith("p-") || name.startsWith("p_") || id.startsWith("p-") || id.startsWith("pump")) return "pump";
  if (name.includes("compressor") || name.startsWith("c-3") || name.startsWith("k-3") || id.startsWith("c-") || id.startsWith("k-3")) return "compressor";
  if (name.includes("valve") || name.startsWith("cv-") || name.startsWith("xv-") || name.startsWith("pv-") || name.startsWith("lv-") || name.startsWith("fv-") || name.startsWith("tv-") || id.startsWith("fv-") || id.startsWith("valve")) return "valve";
  if (name.includes("furnace") || name.includes("heater") || name.startsWith("h-") || id.startsWith("h-") || name.startsWith("f-") || id.startsWith("f-")) return "furnace";
  if (name.includes("exchanger") || name.includes("cooler") || name.startsWith("e-") || name.startsWith("hx") || id.startsWith("e-")) return "heat_exchanger";
  if (name.includes("column") || name.startsWith("t-") || name.startsWith("col") || id.startsWith("t-")) return "column";
  if (name.includes("reactor") || name.startsWith("r-") || id.startsWith("r-")) return "reactor";
  if (name.includes("separator") || name.startsWith("s-") || id.startsWith("s-")) return "separator";
  if (name.includes("tank") || name.startsWith("tk-") || id.startsWith("tk-")) return "tank";
  if (name.includes("vessel") || name.includes("drum") || name.startsWith("v-") || id.startsWith("v-")) return "vessel";
  if (name.includes("turbine") || name.startsWith("st-") || name.startsWith("gt-")) return "turbine";
  if (id.startsWith("k-") || name.startsWith("k-")) return "pump";

  // 3. Parameter inference
  if (params.speed_rpm !== undefined || params.head_m !== undefined || params.flow_rated !== undefined) return "pump";
  if (params.stroke_pct !== undefined || params.cv !== undefined || params.valve_cv !== undefined) return "valve";
  if (params.surge_margin !== undefined || params.pressure_ratio !== undefined) return "compressor";
  if (params.ua_w_k !== undefined || params.area_m2 !== undefined) return "heat_exchanger";

  return "vessel";
}

// ---------------------------------------------------------------------------
// Utility: abbreviate measurement kind for instrument bubble label (ISA-5.1)
// ---------------------------------------------------------------------------
function instrumentAbbrev(tag: string, measurementKind?: string): { letters: string; loopNum: string } {
  const parts = tag.split(/[-_.]/);
  if (parts.length >= 2) {
    const letters = parts[0].toUpperCase();
    const loopNum = parts[1].replace(/[^0-9]/g, "") || "101";
    return { letters, loopNum };
  }

  const k = (measurementKind || "").toLowerCase();
  let letters = "PI";
  if (k.includes("temp")) letters = "TI";
  else if (k.includes("pres")) letters = "PI";
  else if (k.includes("flow")) letters = "FI";
  else if (k.includes("level")) letters = "LI";
  else if (k.includes("anal") || k.includes("quality")) letters = "AI";
  else if (k.includes("speed") || k.includes("rpm")) letters = "SI";
  else if (k.includes("vibr")) letters = "VI";

  const num = tag.replace(/[^0-9]/g, "") || "101";
  return { letters, loopNum: num };
}

// ---------------------------------------------------------------------------
// SVG Symbol Renderers — Authentic DCS Process Symbols
// ---------------------------------------------------------------------------

/**
 * Centrifugal Pump Symbol
 */
function renderPumpSymbolHtml(
  cx: number,
  cy: number,
  isRunning: boolean,
  isTripped: boolean,
  _diag?: any
): string {
  const stroke = isTripped ? "#ef4444" : "#475569";
  const bodyFill = "#172033";
  const runColor = isTripped ? "#ef4444" : isRunning ? "#10b981" : "#64748b";
  const r = 28;

  return `
    <!-- Pump Baseplate & Foundation -->
    <rect x="${cx - 42}" y="${cy + 26}" width="84" height="8" rx="2" fill="#0f172a" stroke="#334155" stroke-width="1.5"/>
    <line x1="${cx - 36}" y1="${cy + 34}" x2="${cx - 36}" y2="${cy + 40}" stroke="#475569" stroke-width="2"/>
    <line x1="${cx + 36}" y1="${cy + 34}" x2="${cx + 36}" y2="${cy + 40}" stroke="#475569" stroke-width="2"/>

    <!-- TEFC Electric Motor Housing (Left) -->
    <g class="pump-motor">
      <rect x="${cx - 52}" y="${cy - 16}" width="24" height="32" rx="3" fill="#1e293b" stroke="${stroke}" stroke-width="1.5"/>
      <line x1="${cx - 48}" y1="${cy - 12}" x2="${cx - 48}" y2="${cy + 12}" stroke="#334155" stroke-width="1"/>
      <line x1="${cx - 44}" y1="${cy - 12}" x2="${cx - 44}" y2="${cy + 12}" stroke="#334155" stroke-width="1"/>
      <line x1="${cx - 40}" y1="${cy - 12}" x2="${cx - 40}" y2="${cy + 12}" stroke="#334155" stroke-width="1"/>
      <rect x="${cx - 46}" y="${cy - 22}" width="12" height="6" rx="1" fill="#334155" stroke="${stroke}" stroke-width="1"/>
      <rect x="${cx - 28}" y="${cy - 7}" width="8" height="14" fill="#0f172a" stroke="#475569" stroke-width="1"/>
    </g>

    <!-- Spiral Volute Casing (Main Body) -->
    <circle cx="${cx + 8}" cy="${cy}" r="${r}" fill="${bodyFill}" stroke="${stroke}" stroke-width="2.5" class="eq-casing"/>

    <!-- Tangential Discharge Nozzle with Flange (Top) -->
    <path d="M${cx + 26},${cy - 12} L${cx + 26},${cy - 36} L${cx + 34},${cy - 36} L${cx + 34},${cy - 8}" fill="${bodyFill}" stroke="${stroke}" stroke-width="2"/>
    <line x1="${cx + 23}" y1="${cy - 36}" x2="${cx + 37}" y2="${cy - 36}" stroke="${stroke}" stroke-width="3"/>

    <!-- Axial Suction Nozzle with Flange (Right) -->
    <line x1="${cx + 36}" y1="${cy}" x2="${cx + 46}" y2="${cy}" stroke="${stroke}" stroke-width="3"/>
    <line x1="${cx + 46}" y1="${cy - 8}" x2="${cx + 46}" y2="${cy + 8}" stroke="${stroke}" stroke-width="3"/>

    <!-- Rotating Impeller / Running Arc Indicator -->
    <g class="impeller-group ${isRunning && !isTripped ? "impeller-running" : ""}">
      <circle cx="${cx + 8}" cy="${cy}" r="18" fill="none" stroke="${runColor}" stroke-width="2" stroke-dasharray="16 8" opacity="0.8"/>
      <circle cx="${cx + 8}" cy="${cy}" r="5" fill="${runColor}"/>
      <path d="M${cx + 8},${cy} Q${cx + 18},${cy - 10} ${cx + 22},${cy - 4}" fill="none" stroke="${runColor}" stroke-width="2.5"/>
      <path d="M${cx + 8},${cy} Q${cx + 2},${cy + 18} ${cx - 4},${cy + 14}" fill="none" stroke="${runColor}" stroke-width="2.5"/>
      <path d="M${cx + 8},${cy} Q${cx - 6},${cy - 14} ${cx - 2},${cy - 18}" fill="none" stroke="${runColor}" stroke-width="2.5"/>
    </g>
  `;
}

/**
 * Centrifugal Compressor Symbol
 */
function renderCompressorSymbolHtml(
  cx: number,
  cy: number,
  isRunning: boolean,
  isTripped: boolean,
  _diag?: any
): string {
  const stroke = isTripped ? "#ef4444" : "#475569";
  const bodyFill = "#172033";
  const runColor = isTripped ? "#ef4444" : isRunning ? "#10b981" : "#64748b";

  return `
    <rect x="${cx - 48}" y="${cy + 28}" width="96" height="8" rx="2" fill="#0f172a" stroke="#334155" stroke-width="1.5"/>
    <rect x="${cx - 56}" y="${cy - 18}" width="22" height="36" rx="2" fill="#1e293b" stroke="${stroke}" stroke-width="1.5"/>
    <line x1="${cx - 34}" y1="${cy}" x2="${cx - 28}" y2="${cy}" stroke="#94a3b8" stroke-width="3"/>
    <polygon points="${cx - 26},${cy - 24} ${cx + 26},${cy - 14} ${cx + 26},${cy + 14} ${cx - 26},${cy + 24}" fill="${bodyFill}" stroke="${stroke}" stroke-width="2.5" class="eq-casing"/>
    <line x1="${cx - 14}" y1="${cy - 20}" x2="${cx - 14}" y2="${cy + 20}" stroke="${runColor}" stroke-width="2" stroke-dasharray="3 3"/>
    <line x1="${cx}" y1="${cy - 17}" x2="${cx}" y2="${cy + 17}" stroke="${runColor}" stroke-width="2" stroke-dasharray="3 3"/>
    <line x1="${cx + 14}" y1="${cy - 15}" x2="${cx + 14}" y2="${cy + 15}" stroke="${runColor}" stroke-width="2" stroke-dasharray="3 3"/>
    <line x1="${cx - 16}" y1="${cy - 22}" x2="${cx - 16}" y2="${cy - 36}" stroke="${stroke}" stroke-width="2.5"/>
    <line x1="${cx - 22}" y1="${cy - 36}" x2="${cx - 10}" y2="${cy - 36}" stroke="${stroke}" stroke-width="3"/>
    <line x1="${cx + 18}" y1="${cy - 15}" x2="${cx + 18}" y2="${cy - 36}" stroke="${stroke}" stroke-width="2.5"/>
    <line x1="${cx + 12}" y1="${cy - 36}" x2="${cx + 24}" y2="${cy - 36}" stroke="${stroke}" stroke-width="3"/>
    <line x1="${cx - 30}" y1="${cy}" x2="${cx + 32}" y2="${cy}" stroke="#64748b" stroke-width="1.5" stroke-dasharray="6 3"/>
  `;
}

/**
 * Distillation Column Symbol
 */
function renderColumnSymbolHtml(
  cx: number,
  cy: number,
  levelVal: number,
  isTripped: boolean,
  _diag?: any
): string {
  const stroke = isTripped ? "#ef4444" : "#475569";
  const bodyFill = "#141c2c";
  const w = 48;
  const h = 170;
  const left = cx - w / 2;
  const top = cy - h / 2;

  const sumpMaxH = 34;
  const sumpFillH = Math.max(4, Math.min(sumpMaxH, (levelVal / 100) * sumpMaxH));

  return `
    <path d="M${left - 4},${top + h + 22} L${left + 3},${top + h} L${left + w - 3},${top + h} L${left + w + 4},${top + h + 22} Z" fill="#0f172a" stroke="${stroke}" stroke-width="2"/>
    <rect x="${left - 8}" y="${top + h + 22}" width="${w + 16}" height="6" rx="1" fill="#1e293b" stroke="#334155" stroke-width="1"/>
    <circle cx="${left - 3}" cy="${top + h + 25}" r="2" fill="#94a3b8"/>
    <circle cx="${left + w + 3}" cy="${top + h + 25}" r="2" fill="#94a3b8"/>
    <ellipse cx="${cx}" cy="${top + h + 11}" rx="7" ry="5" fill="#090d14" stroke="#334155" stroke-width="1"/>

    <rect x="${left}" y="${top}" width="${w}" height="${h}" rx="14" fill="${bodyFill}" stroke="${stroke}" stroke-width="2.5" class="eq-casing"/>

    <g class="column-trays" stroke="#334155" stroke-width="1.5">
      <line x1="${left + 6}" y1="${top + 25}" x2="${left + w - 8}" y2="${top + 25}"/>
      <line x1="${left + 8}" y1="${top + 38}" x2="${left + w - 6}" y2="${top + 38}"/>
      <line x1="${left + 6}" y1="${top + 51}" x2="${left + w - 8}" y2="${top + 51}"/>
      <line x1="${left + 8}" y1="${top + 64}" x2="${left + w - 6}" y2="${top + 64}"/>
      <line x1="${left + 6}" y1="${top + 78}" x2="${left + w - 8}" y2="${top + 78}" stroke="#38bdf8" stroke-width="2"/>
      <line x1="${left + 8}" y1="${top + 92}" x2="${left + w - 6}" y2="${top + 92}"/>
      <line x1="${left + 6}" y1="${top + 106}" x2="${left + w - 8}" y2="${top + 106}"/>
      <line x1="${left + 8}" y1="${top + 120}" x2="${left + w - 6}" y2="${top + 120}"/>
      <line x1="${left + 6}" y1="${top + 134}" x2="${left + w - 8}" y2="${top + 134}"/>
    </g>

    <rect id="vesselFluid" x="${left + 2}" y="${top + h - sumpFillH}" width="${w - 4}" height="${sumpFillH}" rx="4" fill="#0284c7" opacity="0.5"/>
    <line x1="${left + 2}" y1="${top + h - sumpFillH}" x2="${left + w - 2}" y2="${top + h - sumpFillH}" stroke="#38bdf8" stroke-width="1.5"/>
    <text id="vesselLevelVal" x="${cx}" y="${top + h - 8}" text-anchor="middle" fill="#e0f2fe" font-size="9" font-weight="bold" font-family="monospace">${levelVal.toFixed(1)} %</text>

    <line x1="${cx}" y1="${top}" x2="${cx}" y2="${top - 16}" stroke="${stroke}" stroke-width="3"/>
    <line x1="${cx - 8}" y1="${top - 16}" x2="${cx + 8}" y2="${top - 16}" stroke="${stroke}" stroke-width="3"/>
    <line x1="${left}" y1="${top + 20}" x2="${left - 12}" y2="${top + 20}" stroke="${stroke}" stroke-width="2.5"/>
    <line x1="${left - 12}" y1="${top + 14}" x2="${left - 12}" y2="${top + 26}" stroke="${stroke}" stroke-width="2.5"/>
    <line x1="${left}" y1="${top + 78}" x2="${left - 14}" y2="${top + 78}" stroke="${stroke}" stroke-width="3"/>
    <line x1="${left - 14}" y1="${top + 72}" x2="${left - 14}" y2="${top + 84}" stroke="${stroke}" stroke-width="3"/>
    <line x1="${left + w}" y1="${top + 45}" x2="${left + w + 12}" y2="${top + 45}" stroke="${stroke}" stroke-width="2"/>
    <line x1="${left + w}" y1="${top + 112}" x2="${left + w + 12}" y2="${top + 112}" stroke="${stroke}" stroke-width="2"/>
    <line x1="${left}" y1="${top + 144}" x2="${left - 12}" y2="${top + 144}" stroke="${stroke}" stroke-width="2.5"/>
    <line x1="${cx}" y1="${top + h}" x2="${cx}" y2="${top + h + 14}" stroke="${stroke}" stroke-width="3"/>
    <line x1="${cx - 8}" y1="${top + h + 14}" x2="${cx + 8}" y2="${top + h + 14}" stroke="${stroke}" stroke-width="3"/>
  `;
}

/**
 * Fired Process Heater / Furnace / Heating Boiler Symbol
 * High-Fidelity Industrial SCADA Air-Gas Circuit (Siemens PCS7 / SACI Architecture)
 */
function renderFurnaceSymbolHtml(
  cx: number,
  cy: number,
  isRunning: boolean,
  isTripped: boolean,
  _diag?: any,
  s?: Store
): string {
  const stroke = isTripped ? "#ef4444" : "#475569";
  const bodyFill = "#111827";
  const w = 150;
  const h = 110;
  const left = cx - w / 2;
  const top = cy - h / 2 + 10;

  // Live telemetry readings
  const cit = s ? s.getTagFloat("TI-101-CIT", 287.0) : 287.0;
  const cot = s ? s.getTagFloat("TT-101", 332.0) : 332.0;
  const rit = s ? s.getTagFloat("TI-101-RIT", 308.5) : 308.5;
  const rot = s ? s.getTagFloat("TI-101-ROT", 332.0) : 332.0;
  const skinA = s ? s.getTagFloat("TI-101-SK1", 395.0) : 395.0;
  const skinB = s ? s.getTagFloat("TI-101-SK2", 398.0) : 398.0;
  const fgPress = s ? s.getTagFloat("PT-104", 4.2) : 4.2;
  const fgFlow = isTripped ? 0.0 : (s ? s.getTagFloat("FT-105", 1250.0) : 1250.0);
  const airFlow = isTripped ? 0.0 : (s ? s.getTagFloat("FT-Air", 14250.0) : 14250.0);
  const draft = s ? s.getTagFloat("PDT-101", -4.8) : -4.8;
  const stackTemp = s ? s.getTagFloat("TI-104", 180.0) : 180.0;
  const flueO2 = s ? s.getTagFloat("AI-101", 3.2) : 3.2;

  // DB&B valve states
  const v1Color = isTripped ? "#ef4444" : "#10b981"; // XV-101A
  const v2Color = isTripped ? "#ef4444" : "#10b981"; // XV-101B
  const ventColor = isTripped ? "#f59e0b" : "#64748b"; // XV-101V Bleed

  // Flame color & visibility
  const flameVisible = isRunning && !isTripped;
  const flameScannerColor = flameVisible ? "#10b981" : "#ef4444";

  // Alarm severity stroke lookup
  const getAlarmStroke = (tag: string, defaultStroke: string) => {
    if (!s) return defaultStroke;
    const a = Array.from(s.alarms.values()).find(
      (al) => al.alarm_name.includes(tag) && al.state !== "NORMAL" && al.state !== "CLEARED"
    );
    if (!a) return defaultStroke;
    return a.severity === "CRITICAL" ? "#ef4444" : "#f59e0b";
  };

  return `
    <!-- Foundation Piers -->
    <rect x="${left + 14}" y="${top + h}" width="20" height="14" fill="#0f172a" stroke="#334155" stroke-width="1.5"/>
    <rect x="${left + w - 34}" y="${top + h}" width="20" height="14" fill="#0f172a" stroke="#334155" stroke-width="1.5"/>

    <!-- Main Firebox & Convection Casing -->
    <rect x="${left}" y="${top}" width="${w}" height="${h}" rx="4" fill="${bodyFill}" stroke="${stroke}" stroke-width="2.5" class="eq-casing"/>

    <!-- Refractory Lining Detail (Inner Dashed Boundary) -->
    <rect x="${left + 5}" y="${top + 5}" width="${w - 10}" height="${h - 10}" rx="2" fill="none" stroke="#334155" stroke-dasharray="4 2" stroke-width="1"/>

    <!-- Convection Section / Radiant Section Boundary Divider -->
    <line x1="${left + 4}" y1="${top + 38}" x2="${left + w - 4}" y2="${top + 38}" stroke="#475569" stroke-dasharray="4 2" stroke-width="1.4"/>
    <text x="${left + 8}" y="${top + 14}" fill="#94a3b8" font-family="monospace" font-size="7.5" font-weight="bold">CONVECTION</text>
    <text x="${left + 8}" y="${top + 49}" fill="#94a3b8" font-family="monospace" font-size="7.5" font-weight="bold">RADIANT FIREBOX</text>

    <!-- Stack Hood, Breeching & Flue Stack -->
    <polygon points="${left + 36},${top} ${left + 54},${top - 26} ${left + w - 54},${top - 26} ${left + w - 36},${top}" fill="#1e293b" stroke="${stroke}" stroke-width="2"/>
    <rect x="${cx - 16}" y="${top - 62}" width="32" height="36" fill="#1e293b" stroke="${stroke}" stroke-width="2"/>

    <!-- Stack Damper (ZI-101) with Modulating Blade -->
    <line x1="${cx - 13}" y1="${top - 44}" x2="${cx + 13}" y2="${top - 44}" stroke="#f59e0b" stroke-width="2.5"/>
    <text x="${cx + 20}" y="${top - 46}" fill="#f59e0b" font-family="monospace" font-size="7" font-weight="bold">DMPR 45%</text>

    <!-- Flue Gas Stack Telemetry Callout: TI-104 & AI-101 O2 -->
    <g class="scada-callout-stack">
      <g class="equipment-node clickable" data-inst-tag="TI-104" cursor="pointer">
        <rect x="${cx - 72}" y="${top - 62}" width="52" height="22" rx="2" fill="#0b121e" stroke="${getAlarmStroke("TI-104", "#38bdf8")}" stroke-width="1"/>
        <text x="${cx - 46}" y="${top - 51}" text-anchor="middle" fill="#94a3b8" font-family="monospace" font-size="7">TI-104</text>
        <text x="${cx - 46}" y="${top - 41}" text-anchor="middle" fill="#38bdf8" font-family="monospace" font-size="8.5" font-weight="bold">${stackTemp.toFixed(0)}°C</text>
      </g>

      <g class="equipment-node clickable" data-inst-tag="AI-101" cursor="pointer">
        <rect x="${cx + 20}" y="${top - 38}" width="56" height="22" rx="2" fill="#0b121e" stroke="${getAlarmStroke("AI-101", "#10b981")}" stroke-width="1"/>
        <text x="${cx + 48}" y="${top - 27}" text-anchor="middle" fill="#94a3b8" font-family="monospace" font-size="7">AI-101 (O₂)</text>
        <text x="${cx + 48}" y="${top - 17}" text-anchor="middle" fill="#10b981" font-family="monospace" font-size="8.5" font-weight="bold">${flueO2.toFixed(1)}%</text>
      </g>
    </g>

    <!-- Firebox Negative Draft Manometer Tap (PDT-101) -->
    <g class="equipment-node clickable" data-inst-tag="PDT-101" cursor="pointer">
      <circle cx="${left + 18}" cy="${top + 46}" r="3" fill="#0284c7" stroke="#38bdf8" stroke-width="1"/>
      <rect x="${left - 48}" y="${top + 38}" width="62" height="16" rx="2" fill="#0b121e" stroke="${getAlarmStroke("PDT-101", "#0284c7")}" stroke-width="1"/>
      <text x="${left - 17}" y="${top + 50}" text-anchor="middle" fill="#38bdf8" font-family="monospace" font-size="7.5" font-weight="bold">${draft.toFixed(1)} mmH₂O</text>
    </g>

    <!-- Convection Preheat Tubes (Pass A & B Serpentine) -->
    <g class="furnace-tubes-conv" stroke="#38bdf8" stroke-width="2" fill="none" opacity="0.9">
      <path d="M${left + 16},${top + 22} L${left + w - 20},${top + 22} C${left + w - 12},${top + 22} ${left + w - 12},${top + 31} ${left + w - 20},${top + 31} L${left + 24},${top + 31}"/>
    </g>

    <!-- Radiant Tubes with Pass A & B Vertical Serpentine -->
    <g class="furnace-tubes-rad" stroke="#f97316" stroke-width="2.2" fill="none">
      <path d="M${left + 24},${top + 56} L${left + w - 24},${top + 56} C${left + w - 14},${top + 56} ${left + w - 14},${top + 70} ${left + w - 24},${top + 70} L${left + 24},${top + 70} C${left + 14},${top + 70} ${left + 14},${top + 84} ${left + 24},${top + 84} L${left + w - 20},${top + 84}"/>
    </g>

    <!-- Pass A & Pass B Skin Thermocouple Badges (TMT / Tube Metal Temperature) -->
    <g class="equipment-node clickable" data-inst-tag="TI-101-SK1" cursor="pointer">
      <circle cx="${left + 36}" cy="${top + 70}" r="3" fill="#ef4444" stroke="#ffffff" stroke-width="0.8"/>
      <rect x="${left + 42}" y="${top + 64}" width="44" height="13" rx="1.5" fill="#1e1010" stroke="${getAlarmStroke("TI-101-SK1", "#ef4444")}" stroke-width="0.8"/>
      <text x="${left + 64}" y="${top + 74}" text-anchor="middle" fill="#ef4444" font-family="monospace" font-size="7" font-weight="bold">SK1 ${skinA.toFixed(0)}°C</text>
    </g>

    <g class="equipment-node clickable" data-inst-tag="TI-101-SK2" cursor="pointer">
      <circle cx="${left + w - 36}" cy="${top + 70}" r="3" fill="#ef4444" stroke="#ffffff" stroke-width="0.8"/>
      <rect x="${left + w - 86}" y="${top + 64}" width="44" height="13" rx="1.5" fill="#1e1010" stroke="${getAlarmStroke("TI-101-SK2", "#ef4444")}" stroke-width="0.8"/>
      <text x="${left + w - 64}" y="${top + 74}" text-anchor="middle" fill="#ef4444" font-family="monospace" font-size="7" font-weight="bold">SK2 ${skinB.toFixed(0)}°C</text>
    </g>

    <!-- Process Temperatures: CIT, RIT, ROT, COT Callouts -->
    <g class="furnace-process-temps">
      <!-- CIT (Coil Inlet Temp) at left entrance -->
      <g class="equipment-node clickable" data-inst-tag="TI-101-CIT" cursor="pointer">
        <rect x="${left - 52}" y="${top + 16}" width="50" height="15" rx="2" fill="#0b121e" stroke="${getAlarmStroke("TI-101-CIT", "#38bdf8")}" stroke-width="1"/>
        <text x="${left - 27}" y="${top + 27}" text-anchor="middle" fill="#38bdf8" font-family="monospace" font-size="7.5" font-weight="bold">CIT ${cit.toFixed(0)}°C</text>
      </g>

      <!-- RIT (Radiant Inlet Temp) at crossover -->
      <g class="equipment-node clickable" data-inst-tag="TI-101-RIT" cursor="pointer">
        <rect x="${left + 12}" y="${top + 38}" width="54" height="14" rx="2" fill="#1e1b12" stroke="${getAlarmStroke("TI-101-RIT", "#fbbf24")}" stroke-width="0.8"/>
        <text x="${left + 39}" y="${top + 48}" text-anchor="middle" fill="#fbbf24" font-family="monospace" font-size="7" font-weight="bold">RIT ${rit.toFixed(0)}°C</text>
      </g>

      <!-- ROT & COT (Radiant & Coil Outlet Temp) at right exit -->
      <g class="equipment-node clickable" data-inst-tag="TI-101-ROT" cursor="pointer">
        <rect x="${left + w + 4}" y="${top + 58}" width="56" height="14" rx="2" fill="#1e1b12" stroke="${getAlarmStroke("TI-101-ROT", "#fbbf24")}" stroke-width="0.8"/>
        <text x="${left + w + 32}" y="${top + 68}" text-anchor="middle" fill="#fbbf24" font-family="monospace" font-size="7" font-weight="bold">ROT ${rot.toFixed(0)}°C</text>
      </g>
      <g class="equipment-node clickable" data-inst-tag="TT-101" cursor="pointer">
        <rect x="${left + w + 4}" y="${top + 76}" width="56" height="16" rx="2" fill="#1f1307" stroke="${getAlarmStroke("TT-101", "#f97316")}" stroke-width="1"/>
        <text x="${left + w + 32}" y="${top + 88}" text-anchor="middle" fill="#f97316" font-family="monospace" font-size="8" font-weight="bold">COT ${cot.toFixed(0)}°C</text>
      </g>
    </g>

    <!-- Burner Assemblies with Flame Scanners & Roaring Flames -->
    <g class="furnace-burners">
      <!-- Burner 1 (Left) -->
      <rect x="${left + 22}" y="${top + h - 12}" width="20" height="12" rx="2" fill="#334155" stroke="#475569" stroke-width="1.2"/>
      <circle cx="${left + 32}" cy="${top + h - 3}" r="2" fill="${flameScannerColor}" title="FS-1 Flame Scanner"/>
      ${flameVisible ? `
        <polygon points="${left + 25},${top + h - 12} ${left + 32},${top + h - 36} ${left + 39},${top + h - 12}" fill="#f59e0b" class="flame-active"/>
        <polygon points="${left + 28},${top + h - 12} ${left + 32},${top + h - 24} ${left + 36},${top + h - 12}" fill="#ef4444"/>
      ` : `<line x1="${left + 32}" y1="${top + h - 12}" x2="${left + 32}" y2="${top + h - 18}" stroke="#64748b" stroke-width="1.5"/>`}

      <!-- Burner 2 (Center) -->
      <rect x="${cx - 10}" y="${top + h - 12}" width="20" height="12" rx="2" fill="#334155" stroke="#475569" stroke-width="1.2"/>
      <circle cx="${cx}" cy="${top + h - 3}" r="2" fill="${flameScannerColor}" title="FS-2 Flame Scanner"/>
      ${flameVisible ? `
        <polygon points="${cx - 7},${top + h - 12} ${cx},${top + h - 40} ${cx + 7},${top + h - 12}" fill="#f59e0b" class="flame-active"/>
        <polygon points="${cx - 4},${top + h - 12} ${cx},${top + h - 26} ${cx + 4},${top + h - 12}" fill="#ef4444"/>
      ` : `<line x1="${cx}" y1="${top + h - 12}" x2="${cx}" y2="${top + h - 18}" stroke="#64748b" stroke-width="1.5"/>`}

      <!-- Burner 3 (Right) -->
      <rect x="${left + w - 42}" y="${top + h - 12}" width="20" height="12" rx="2" fill="#334155" stroke="#475569" stroke-width="1.2"/>
      <circle cx="${left + w - 32}" cy="${top + h - 3}" r="2" fill="${flameScannerColor}" title="FS-3 Flame Scanner"/>
      ${flameVisible ? `
        <polygon points="${left + w - 39},${top + h - 12} ${left + w - 32},${top + h - 36} ${left + w - 25},${top + h - 12}" fill="#f59e0b" class="flame-active"/>
        <polygon points="${left + w - 36},${top + h - 12} ${left + w - 32},${top + h - 24} ${left + w - 28},${top + h - 12}" fill="#ef4444"/>
      ` : `<line x1="${left + w - 32}" y1="${top + h - 12}" x2="${left + w - 32}" y2="${top + h - 18}" stroke="#64748b" stroke-width="1.5"/>`}
    </g>

    <!-- ========================================================================= -->
    <!-- AIR CIRCUIT (FORCED DRAFT FAN, AIR DAMPER, FT-AIR, PT-AIR)               -->
    <!-- Based on ResearchGate SACI 2009 Air-Gas Circuit SCADA Architecture       -->
    <!-- ========================================================================= -->
    <g class="furnace-air-circuit">
      <!-- Centrifugal FD Fan (FA-101) -->
      <circle cx="${left - 30}" cy="${top + h - 6}" r="11" fill="#0f172a" stroke="#0284c7" stroke-width="1.8"/>
      <!-- Motor Box -->
      <rect x="${left - 48}" y="${top + h - 12}" width="12" height="12" rx="1.5" fill="#1e293b" stroke="#0284c7" stroke-width="1"/>
      <text x="${left - 42}" y="${top + h - 3}" text-anchor="middle" fill="#38bdf8" font-family="monospace" font-size="7.5" font-weight="bold">M</text>
      <!-- Impeller Blades -->
      <line x1="${left - 30}" y1="${top + h - 14}" x2="${left - 30}" y2="${top + h + 2}" stroke="#38bdf8" stroke-width="1"/>
      <line x1="${left - 38}" y1="${top + h - 6}" x2="${left - 22}" y2="${top + h - 6}" stroke="#38bdf8" stroke-width="1"/>

      <!-- Air Suction Intake Arrow -->
      <path d="M${left - 48},${top + h + 6} L${left - 36},${top + h + 6}" stroke="#38bdf8" stroke-width="1.5" marker-end="url(#arrow-marker)"/>
      <text x="${left - 54}" y="${top + h + 15}" fill="#38bdf8" font-family="monospace" font-size="6.5">AIR IN</text>

      <!-- Air Duct from Fan to Windbox Plenum -->
      <path d="M${left - 20},${top + h - 6} L${left + 14},${top + h - 6}" stroke="#0284c7" stroke-width="3" fill="none"/>

      <!-- Air Damper (FZ-101) -->
      <line x1="${left - 10}" y1="${top + h - 11}" x2="${left - 2}" y2="${top + h - 1}" stroke="#f59e0b" stroke-width="2"/>
      <rect x="${left - 8}" y="${top + h - 18}" width="18" height="9" rx="1" fill="#0b121e" stroke="#f59e0b" stroke-width="0.6"/>
      <text x="${left + 1}" y="${top + h - 11}" text-anchor="middle" fill="#f59e0b" font-family="monospace" font-size="6.5" font-weight="bold">45%</text>

      <!-- Air Flow & Pressure Callout -->
      <g class="equipment-node clickable" data-inst-tag="FT-Air" cursor="pointer">
        <rect x="${left - 54}" y="${top + h - 36}" width="56" height="22" rx="2" fill="#0b121e" stroke="${getAlarmStroke("FT-Air", "#0284c7")}" stroke-width="1"/>
        <text x="${left - 26}" y="${top + h - 25}" text-anchor="middle" fill="#94a3b8" font-family="monospace" font-size="6.5">FT-AIR (m³/h)</text>
        <text x="${left - 26}" y="${top + h - 15}" text-anchor="middle" fill="#38bdf8" font-family="monospace" font-size="8.5" font-weight="bold">${airFlow.toFixed(0)}</text>
      </g>
    </g>

    <!-- ========================================================================= -->
    <!-- FUEL GAS CIRCUIT (DOUBLE BLOCK & BLEED, FT-105, PT-104, FCV-101)          -->
    <!-- ========================================================================= -->
    <g class="furnace-fuel-circuit">
      <!-- Main FG Supply Line -->
      <line x1="${left - 52}" y1="${top + h + 18}" x2="${left + w + 16}" y2="${top + h + 18}" stroke="#eab308" stroke-width="2.5"/>

      <!-- Double Block & Bleed Safety Shutoff System -->
      <!-- SSV 1: XV-101A -->
      <polygon points="${left - 16},${top + h + 14} ${left - 8},${top + h + 18} ${left - 16},${top + h + 22}" fill="${v1Color}"/>
      <polygon points="${left},${top + h + 14} ${left - 8},${top + h + 18} ${left},${top + h + 22}" fill="${v1Color}"/>
      <text x="${left - 8}" y="${top + h + 28}" text-anchor="middle" fill="#eab308" font-family="monospace" font-size="6.5">XV-1A</text>

      <!-- Bleed / Vent Valve: XV-101V -->
      <line x1="${left + 12}" y1="${top + h + 18}" x2="${left + 12}" y2="${top + h + 7}" stroke="#eab308" stroke-width="1.5"/>
      <polygon points="${left + 9},${top + h + 11} ${left + 12},${top + h + 7} ${left + 15},${top + h + 11}" fill="${ventColor}"/>
      <text x="${left + 12}" y="${top + h + 4}" text-anchor="middle" fill="${ventColor}" font-family="monospace" font-size="6">VENT</text>

      <!-- SSV 2: XV-101B -->
      <polygon points="${left + 24},${top + h + 14} ${left + 32},${top + h + 18} ${left + 24},${top + h + 22}" fill="${v2Color}"/>
      <polygon points="${left + 40},${top + h + 14} ${left + 32},${top + h + 18} ${left + 40},${top + h + 22}" fill="${v2Color}"/>
      <text x="${left + 32}" y="${top + h + 28}" text-anchor="middle" fill="#eab308" font-family="monospace" font-size="6.5">XV-1B</text>

      <!-- Fuel Gas Flow Transmitter Orifice (FT-105) -->
      <circle cx="${left + 58}" cy="${top + h + 18}" r="5" fill="#1e293b" stroke="#eab308" stroke-width="1.2"/>
      <text x="${left + 58}" y="${top + h + 21}" text-anchor="middle" fill="#eab308" font-family="monospace" font-size="6" font-weight="bold">FT</text>

      <!-- Modulating Fuel Control Valve (FCV-101) -->
      <polygon points="${left + 80},${top + h + 14} ${left + 88},${top + h + 18} ${left + 80},${top + h + 22}" fill="#38bdf8"/>
      <polygon points="${left + 96},${top + h + 14} ${left + 88},${top + h + 18} ${left + 96},${top + h + 22}" fill="#38bdf8"/>
      <line x1="${left + 88}" y1="${top + h + 18}" x2="${left + 88}" y2="${top + h + 10}" stroke="#94a3b8" stroke-width="1.2"/>
      <ellipse cx="${left + 88}" cy="${top + h + 9}" rx="4" ry="2" fill="#1e293b" stroke="#94a3b8"/>

      <!-- FG Telemetry Badge (PT-104 & FT-105) -->
      <g class="equipment-node clickable" data-inst-tag="PT-104" cursor="pointer">
        <rect x="${left - 54}" y="${top + h + 3}" width="46" height="14" rx="1.5" fill="#0b121e" stroke="${getAlarmStroke("PT-104", "#eab308")}" stroke-width="1"/>
        <text x="${left - 31}" y="${top + h + 13}" text-anchor="middle" fill="#eab308" font-family="monospace" font-size="7" font-weight="bold">${fgPress.toFixed(1)}b | PT104</text>
      </g>

      <g class="equipment-node clickable" data-inst-tag="FT-105" cursor="pointer">
        <rect x="${left + 104}" y="${top + h + 10}" width="52" height="15" rx="1.5" fill="#0b121e" stroke="${getAlarmStroke("FT-105", "#eab308")}" stroke-width="1"/>
        <text x="${left + 130}" y="${top + h + 21}" text-anchor="middle" fill="#eab308" font-family="monospace" font-size="7.5" font-weight="bold">${fgFlow.toFixed(0)} Nm³/h</text>
      </g>
    </g>

    <!-- ========================================================================= -->
    <!-- AUXILIARY HEADERS: FUEL OIL (FO) & ATOMIZING STEAM (STM)                  -->
    <!-- ========================================================================= -->
    <g class="furnace-aux-headers">
      <!-- Fuel Oil Line (Amber #d97706) -->
      <line x1="${left - 52}" y1="${top + h + 32}" x2="${left + w + 16}" y2="${top + h + 32}" stroke="#d97706" stroke-width="1.8"/>
      <g class="equipment-node clickable" data-inst-tag="FT-107" cursor="pointer">
        <rect x="${left - 54}" y="${top + h + 28}" width="104" height="12" rx="1.5" fill="#0b121e" stroke="${getAlarmStroke("FT-107", "#d97706")}" stroke-width="0.8"/>
        <text x="${left - 2}" y="${top + h + 37}" text-anchor="middle" fill="#d97706" font-family="monospace" font-size="6.5" font-weight="bold">FO 6.0b (FT-107: 120 kg/h)</text>
      </g>

      <!-- Atomizing Steam Line (Dashed Cyan/Orange #38bdf8) -->
      <line x1="${left - 52}" y1="${top + h + 44}" x2="${left + w + 16}" y2="${top + h + 44}" stroke="#38bdf8" stroke-width="1.6" stroke-dasharray="4 2"/>
      <g class="equipment-node clickable" data-inst-tag="PT-105" cursor="pointer">
        <rect x="${left + w - 44}" y="${top + h + 42}" width="60" height="12" rx="1.5" fill="#0b121e" stroke="${getAlarmStroke("PT-105", "#38bdf8")}" stroke-width="0.8"/>
        <text x="${left + w - 14}" y="${top + h + 51}" text-anchor="middle" fill="#38bdf8" font-family="monospace" font-size="6.5" font-weight="bold">STM 7.5b (PT-105)</text>
      </g>
    </g>
  `;
}

/**
 * Shell & Tube Heat Exchanger Symbol
 */
function renderExchangerSymbolHtml(
  cx: number,
  cy: number,
  isTripped: boolean,
  _diag?: any
): string {
  const stroke = isTripped ? "#ef4444" : "#475569";
  const bodyFill = "#141c2c";
  const w = 76;
  const h = 42;
  const left = cx - w / 2;
  const top = cy - h / 2;

  return `
    <polygon points="${left + 12},${top + h} ${left + 8},${top + h + 10} ${left + 20},${top + h + 10} ${left + 16},${top + h}" fill="#0f172a" stroke="#334155" stroke-width="1"/>
    <polygon points="${left + w - 16},${top + h} ${left + w - 20},${top + h + 10} ${left + w - 8},${top + h + 10} ${left + w - 12},${top + h}" fill="#0f172a" stroke="#334155" stroke-width="1"/>
    <rect x="${left}" y="${top}" width="${w}" height="${h}" rx="6" fill="${bodyFill}" stroke="${stroke}" stroke-width="2.5" class="eq-casing"/>
    <path d="M${left},${top} C${left - 10},${top + 6} ${left - 10},${top + h - 6} ${left},${top + h} Z" fill="#1e293b" stroke="${stroke}" stroke-width="2"/>
    <line x1="${left}" y1="${top - 2}" x2="${left}" y2="${top + h + 2}" stroke="${stroke}" stroke-width="2.5"/>
    <path d="M${left + w},${top} C${left + w + 10},${top + 6} ${left + w + 10},${top + h - 6} ${left + w},${top + h} Z" fill="#1e293b" stroke="${stroke}" stroke-width="2"/>
    <line x1="${left + w}" y1="${top - 2}" x2="${left + w}" y2="${top + h + 2}" stroke="${stroke}" stroke-width="2.5"/>
    <line x1="${left + 18}" y1="${top}" x2="${left + 18}" y2="${top + h - 10}" stroke="#475569" stroke-width="1.5"/>
    <line x1="${left + 38}" y1="${top + 10}" x2="${left + 38}" y2="${top + h}" stroke="#475569" stroke-width="1.5"/>
    <line x1="${left + 58}" y1="${top}" x2="${left + 58}" y2="${top + h - 10}" stroke="#475569" stroke-width="1.5"/>
    <path d="M${left + 8},${top + h / 2} C${left + 30},${top + 10} ${left + 50},${top + h - 10} ${left + w - 8},${top + h / 2}" fill="none" stroke="#38bdf8" stroke-width="1.5" stroke-dasharray="3 3"/>
    <line x1="${left + 20}" y1="${top}" x2="${left + 20}" y2="${top - 10}" stroke="${stroke}" stroke-width="2.5"/>
    <line x1="${left + 15}" y1="${top - 10}" x2="${left + 25}" y2="${top - 10}" stroke="${stroke}" stroke-width="2.5"/>
    <line x1="${left + w - 20}" y1="${top + h}" x2="${left + w - 20}" y2="${top + h + 10}" stroke="${stroke}" stroke-width="2.5"/>
    <line x1="${left + w - 25}" y1="${top + h + 10}" x2="${left + w - 15}" y2="${top + h + 10}" stroke="${stroke}" stroke-width="2.5"/>
  `;
}

/**
 * Robust port-aware orthogonal routing between source and destination nozzles.
 */
export function computePortAwareWaypoints(
  p1: { x: number; y: number; dir?: "left" | "right" | "top" | "bottom" },
  p2: { x: number; y: number; dir?: "left" | "right" | "top" | "bottom" }
): { x: number; y: number }[] {
  const stubLen = 16;
  const s1 = { x: p1.x, y: p1.y };
  if (p1.dir === "top") s1.y -= stubLen;
  else if (p1.dir === "bottom") s1.y += stubLen;
  else if (p1.dir === "left") s1.x -= stubLen;
  else s1.x += stubLen;

  const s2 = { x: p2.x, y: p2.y };
  if (p2.dir === "top") s2.y -= stubLen;
  else if (p2.dir === "bottom") s2.y += stubLen;
  else if (p2.dir === "right") s2.x += stubLen;
  else s2.x -= stubLen;

  const pts: { x: number; y: number }[] = [{ x: p1.x, y: p1.y }, s1];

  if ((p1.dir === "right" || !p1.dir) && (p2.dir === "left" || !p2.dir)) {
    if (s2.x > s1.x + 10) {
      const midX = Math.round((s1.x + s2.x) / 2);
      pts.push({ x: midX, y: s1.y });
      pts.push({ x: midX, y: s2.y });
    } else {
      const detourY = Math.max(s1.y, s2.y) + 35;
      pts.push({ x: s1.x, y: detourY });
      pts.push({ x: s2.x, y: detourY });
    }
  } else if (p1.dir === "bottom" && p2.dir === "left") {
    pts.push({ x: s1.x, y: s2.y });
  } else if (p1.dir === "top" && p2.dir === "left") {
    pts.push({ x: s1.x, y: s2.y });
  } else if (p1.dir === "right" && p2.dir === "top") {
    pts.push({ x: s2.x, y: s1.y });
  } else if (p1.dir === "bottom" && p2.dir === "top") {
    const midY = Math.round((s1.y + s2.y) / 2);
    pts.push({ x: s1.x, y: midY });
    pts.push({ x: s2.x, y: midY });
  } else if (p1.dir === "top" && p2.dir === "top") {
    const topY = Math.min(s1.y, s2.y) - 20;
    pts.push({ x: s1.x, y: topY });
    pts.push({ x: s2.x, y: topY });
  } else if (p1.dir === "bottom" && p2.dir === "bottom") {
    const botY = Math.max(s1.y, s2.y) + 20;
    pts.push({ x: s1.x, y: botY });
    pts.push({ x: s2.x, y: botY });
  } else {
    const midX = Math.round((s1.x + s2.x) / 2);
    pts.push({ x: midX, y: s1.y });
    pts.push({ x: midX, y: s2.y });
  }

  pts.push(s2);
  pts.push({ x: p2.x, y: p2.y });

  const cleanPts: { x: number; y: number }[] = [pts[0]];
  for (let i = 1; i < pts.length - 1; i++) {
    const prev = cleanPts[cleanPts.length - 1];
    const curr = pts[i];
    const next = pts[i + 1];
    if ((prev.x === curr.x && curr.x === next.x) || (prev.y === curr.y && curr.y === next.y)) {
      continue;
    }
    cleanPts.push(curr);
  }
  cleanPts.push(pts[pts.length - 1]);
  return cleanPts;
}

/**
 * Fixed-Bed Catalytic Hydrotreater Reactor Symbol (R-101)
 */
function renderReactorSymbolHtml(
  cx: number,
  cy: number,
  tempVal: number,
  _pressVal: number,
  isTripped: boolean,
  _diag?: any
): string {
  const stroke = isTripped ? "#ef4444" : "#475569";
  const bodyFill = "#0d1522";
  const w = 68;
  const h = 116;
  const left = cx - w / 2;
  const top = cy - h / 2;
  const bedW = w - 12;
  const bedH = 26;
  const bed1Top = top + 26;
  const bed2Top = top + 64;

  return `
    <!-- Reactor Skirt & Support -->
    <rect x="${left + 12}" y="${top + h}" width="10" height="14" fill="#0f172a" stroke="#334155" stroke-width="1.5"/>
    <rect x="${left + w - 22}" y="${top + h}" width="10" height="14" fill="#0f172a" stroke="#334155" stroke-width="1.5"/>
    <line x1="${left + 8}" y1="${top + h + 12}" x2="${left + w - 8}" y2="${top + h + 12}" stroke="#334155" stroke-width="2"/>

    <!-- Reactor Heavy-Wall Pressure Shell (Dished Heads) -->
    <rect x="${left}" y="${top}" width="${w}" height="${h}" rx="18" fill="${bodyFill}" stroke="${stroke}" stroke-width="3" class="eq-casing"/>
    <rect x="${left + 3}" y="${top + 3}" width="${w - 6}" height="${h - 6}" rx="15" fill="none" stroke="#1e293b" stroke-width="1.5"/>

    <!-- Top Inlet Distributor Tray -->
    <rect x="${left + 8}" y="${top + 16}" width="${w - 16}" height="4" fill="#334155" stroke="#475569" stroke-width="0.8"/>
    <line x1="${left + 12}" y1="${top + 20}" x2="${left + w - 12}" y2="${top + 20}" stroke="#38bdf8" stroke-width="1" stroke-dasharray="2 3"/>

    <!-- Catalyst Bed 1 (Hydrodesulfurization CoMo/NiMo) -->
    <g class="catalyst-bed-1">
      <rect x="${left + 6}" y="${bed1Top}" width="${bedW}" height="${bedH}" rx="2" fill="#1e293b" stroke="#0ea5e9" stroke-width="1" stroke-opacity="0.6"/>
      <line x1="${left + 10}" y1="${bed1Top + 6}" x2="${left + bedW + 2}" y2="${bed1Top + 6}" stroke="#0284c7" stroke-width="1" stroke-dasharray="3 3"/>
      <line x1="${left + 8}" y1="${bed1Top + 13}" x2="${left + bedW + 4}" y2="${bed1Top + 13}" stroke="#0284c7" stroke-width="1" stroke-dasharray="3 3"/>
      <line x1="${left + 10}" y1="${bed1Top + 20}" x2="${left + bedW + 2}" y2="${bed1Top + 20}" stroke="#0284c7" stroke-width="1" stroke-dasharray="3 3"/>
      <text x="${cx}" y="${bed1Top + 16}" text-anchor="middle" fill="#7dd3fc" font-size="8" font-family="monospace" font-weight="bold" opacity="0.85">BED 1 (NiMo)</text>
    </g>

    <!-- Inter-Bed Quench & Redistribution Zone -->
    <rect x="${left + 10}" y="${top + 55}" width="${w - 20}" height="4" fill="#334155" stroke="#475569" stroke-width="0.8"/>
    <line x1="${left + 14}" y1="${top + 59}" x2="${left + w - 14}" y2="${top + 59}" stroke="#38bdf8" stroke-width="1" stroke-dasharray="2 2"/>

    <!-- Catalyst Bed 2 (Finishing / HDN Bed) -->
    <g class="catalyst-bed-2">
      <rect x="${left + 6}" y="${bed2Top}" width="${bedW}" height="${bedH}" rx="2" fill="#1e293b" stroke="#0ea5e9" stroke-width="1" stroke-opacity="0.6"/>
      <line x1="${left + 10}" y1="${bed2Top + 6}" x2="${left + bedW + 2}" y2="${bed2Top + 6}" stroke="#0284c7" stroke-width="1" stroke-dasharray="3 3"/>
      <line x1="${left + 8}" y1="${bed2Top + 13}" x2="${left + bedW + 4}" y2="${bed2Top + 13}" stroke="#0284c7" stroke-width="1" stroke-dasharray="3 3"/>
      <line x1="${left + 10}" y1="${bed2Top + 20}" x2="${left + bedW + 2}" y2="${bed2Top + 20}" stroke="#0284c7" stroke-width="1" stroke-dasharray="3 3"/>
      <text x="${cx}" y="${bed2Top + 16}" text-anchor="middle" fill="#7dd3fc" font-size="8" font-family="monospace" font-weight="bold" opacity="0.85">BED 2 (CoMo)</text>
    </g>

    <!-- Bottom Bed Support Grid & Collector Screen -->
    <rect x="${left + 8}" y="${top + 93}" width="${w - 16}" height="4" fill="#334155" stroke="#475569" stroke-width="0.8"/>

    <!-- Multi-Point Bed Thermocouple Sensors Indicator (TT-101 / TT-102) -->
    <g class="reactor-sensors">
      <rect x="${cx - 28}" y="${top + h - 14}" width="56" height="12" rx="2" fill="#0b0f17" stroke="#334155" stroke-width="0.75"/>
      <text x="${cx}" y="${top + h - 5}" text-anchor="middle" fill="#38bdf8" font-family="monospace" font-size="8" font-weight="bold">${tempVal.toFixed(1)}°C</text>
    </g>

    <!-- Top Inlet Nozzle -->
    <line x1="${cx}" y1="${top}" x2="${cx}" y2="${top - 12}" stroke="${stroke}" stroke-width="3"/>
    <line x1="${cx - 6}" y1="${top - 12}" x2="${cx + 6}" y2="${top - 12}" stroke="${stroke}" stroke-width="3"/>

    <!-- Bottom Effluent Nozzle -->
    <line x1="${cx}" y1="${top + h}" x2="${cx}" y2="${top + h + 14}" stroke="${stroke}" stroke-width="3"/>
    <line x1="${cx - 6}" y1="${top + h + 14}" x2="${cx + 6}" y2="${top + h + 14}" stroke="${stroke}" stroke-width="3"/>
  `;
}

/**
 * Vertical / Horizontal Separator & Vessel Symbol
 */
function renderVesselSymbolHtml(
  cx: number,
  cy: number,
  levelVal: number,
  isTripped: boolean,
  _diag?: any
): string {
  const stroke = isTripped ? "#ef4444" : "#475569";
  const bodyFill = "#141c2c";
  const w = 58;
  const h = 96;
  const left = cx - w / 2;
  const top = cy - h / 2;

  const maxFluidH = h - 22;
  const fluidH = Math.max(4, Math.min(maxFluidH, (levelVal / 100) * maxFluidH));

  return `
    <rect x="${left + 10}" y="${top + h}" width="10" height="12" fill="#0f172a" stroke="#334155" stroke-width="1.5"/>
    <rect x="${left + w - 20}" y="${top + h}" width="10" height="12" fill="#0f172a" stroke="#334155" stroke-width="1.5"/>
    <rect x="${left + 6}" y="${top + h + 10}" width="${w - 12}" height="3" fill="#334155"/>
    <rect x="${left}" y="${top}" width="${w}" height="${h}" rx="14" fill="${bodyFill}" stroke="${stroke}" stroke-width="2.5" class="eq-casing"/>
    <rect x="${left + 6}" y="${top + 14}" width="${w - 12}" height="8" rx="1" fill="#1e293b" stroke="#475569" stroke-width="1" stroke-dasharray="2 2"/>
    <rect id="vesselFluid" x="${left + 3}" y="${top + h - fluidH - 4}" width="${w - 6}" height="${fluidH}" rx="6" fill="#0284c7" opacity="0.45"/>
    <line x1="${left + 3}" y1="${top + h - fluidH - 4}" x2="${left + w - 3}" y2="${top + h - fluidH - 4}" stroke="#38bdf8" stroke-width="1.5"/>

    <g class="vessel-grad-scale" stroke="#475569" stroke-width="1">
      <line x1="${left + 4}" y1="${top + h - 10}" x2="${left + 8}" y2="${top + h - 10}"/>
      <line x1="${left + 4}" y1="${top + h / 2}" x2="${left + 10}" y2="${top + h / 2}"/>
      <line x1="${left + 4}" y1="${top + 28}" x2="${left + 8}" y2="${top + 28}"/>
    </g>

    <text id="vesselLevelVal" x="${cx}" y="${top + h - 12}" text-anchor="middle" fill="#e0f2fe" font-size="9.5" font-weight="bold" font-family="monospace">${levelVal.toFixed(1)} %</text>
    <line x1="${cx}" y1="${top}" x2="${cx}" y2="${top - 14}" stroke="${stroke}" stroke-width="2.5"/>
    <polygon points="${cx - 5},${top - 18} ${cx + 5},${top - 18} ${cx},${top - 14}" fill="#f59e0b" stroke="#ffffff" stroke-width="0.5"/>
    <text x="${cx}" y="${top - 20}" text-anchor="middle" fill="#94a3b8" font-size="7" font-family="monospace">PSV</text>
    <line x1="${left}" y1="${top + 34}" x2="${left - 12}" y2="${top + 34}" stroke="${stroke}" stroke-width="2.5"/>
    <line x1="${left - 12}" y1="${top + 28}" x2="${left - 12}" y2="${top + 40}" stroke="${stroke}" stroke-width="2.5"/>
    <line x1="${cx}" y1="${top + h}" x2="${cx}" y2="${top + h + 14}" stroke="${stroke}" stroke-width="3"/>
    <line x1="${cx - 8}" y1="${top + h + 14}" x2="${cx + 8}" y2="${top + h + 14}" stroke="${stroke}" stroke-width="3"/>
  `;
}

/**
 * Control Valve Symbol
 */
function renderValveSymbolHtml(
  cx: number,
  cy: number,
  actPct: number,
  _cmdPct: number,
  isTripped: boolean,
  _diag?: any
): string {
  const stroke = isTripped ? "#ef4444" : "#475569";
  const bodyFill = "#141c2c";
  const isOpen = actPct > 5.0;
  const flowFill = isOpen ? "#38bdf8" : "#475569";

  return `
    <polygon id="valvePlug" points="${cx - 18},${cy - 12} ${cx},${cy} ${cx - 18},${cy + 12}" fill="${bodyFill}" stroke="${stroke}" stroke-width="2" class="eq-casing"/>
    <polygon points="${cx + 18},${cy - 12} ${cx},${cy} ${cx + 18},${cy + 12}" fill="${bodyFill}" stroke="${stroke}" stroke-width="2"/>
    <line x1="${cx - 20}" y1="${cy - 12}" x2="${cx - 20}" y2="${cy + 12}" stroke="${stroke}" stroke-width="2.5"/>
    <line x1="${cx + 20}" y1="${cy - 12}" x2="${cx + 20}" y2="${cy + 12}" stroke="${stroke}" stroke-width="2.5"/>
    <circle cx="${cx}" cy="${cy}" r="3.5" fill="${flowFill}"/>
    <line x1="${cx}" y1="${cy}" x2="${cx}" y2="${cy - 18}" stroke="${stroke}" stroke-width="2.5"/>
    <line x1="${cx - 6}" y1="${cy - 14}" x2="${cx + 6}" y2="${cy - 14}" stroke="#64748b" stroke-width="1.5"/>
    <path d="M${cx - 14},${cy - 22} C${cx - 14},${cy - 34} ${cx + 14},${cy - 34} ${cx + 14},${cy - 22} Z" fill="#1e293b" stroke="${stroke}" stroke-width="2"/>
    <line x1="${cx - 14}" y1="${cy - 22}" x2="${cx + 14}" y2="${cy - 22}" stroke="${stroke}" stroke-width="2"/>
    <rect x="${cx + 6}" y="${cy - 16}" width="10" height="9" rx="1" fill="#0f172a" stroke="#0ea5e9" stroke-width="1"/>
    <line x1="${cx + 16}" y1="${cy - 12}" x2="${cx + 24}" y2="${cy - 12}" stroke="#a855f7" stroke-width="1.5" stroke-dasharray="2 2"/>
    <text x="${cx - 10}" y="${cy - 12}" text-anchor="middle" fill="#94a3b8" font-size="7" font-weight="bold" font-family="monospace">FC</text>
  `;
}

/**
 * Atmospheric Storage Tank Symbol
 */
function renderTankSymbolHtml(
  cx: number,
  cy: number,
  levelVal: number,
  isTripped: boolean,
  _diag?: any
): string {
  const stroke = isTripped ? "#ef4444" : "#475569";
  const bodyFill = "#141c2c";
  const w = 84;
  const h = 72;
  const left = cx - w / 2;
  const top = cy - h / 2 + 8;
  const fluidH = Math.max(4, Math.min(h - 10, (levelVal / 100) * (h - 10)));

  return `
    <rect x="${left - 4}" y="${top + h}" width="${w + 8}" height="6" rx="1" fill="#0f172a" stroke="#334155" stroke-width="1"/>
    <path d="M${left},${top} L${cx},${top - 16} L${left + w},${top} L${left + w},${top + h} L${left},${top + h} Z" fill="${bodyFill}" stroke="${stroke}" stroke-width="2.5" class="eq-casing"/>
    <line x1="${left}" y1="${top + 24}" x2="${left + w}" y2="${top + 24}" stroke="#334155" stroke-width="1.5"/>
    <rect x="${left + 3}" y="${top + h - fluidH}" width="${w - 6}" height="${fluidH}" rx="2" fill="#0284c7" opacity="0.45"/>
    <line x1="${left + 3}" y1="${top + h - fluidH}" x2="${left + w - 3}" y2="${top + h - fluidH}" stroke="#38bdf8" stroke-width="1.5"/>
    <line x1="${cx}" y1="${top - 16}" x2="${cx}" y2="${top - 24}" stroke="${stroke}" stroke-width="2"/>
    <line x1="${cx - 4}" y1="${top - 24}" x2="${cx + 4}" y2="${top - 24}" stroke="${stroke}" stroke-width="2"/>
    <text x="${cx}" y="${top + h - 8}" text-anchor="middle" fill="#e0f2fe" font-size="9" font-weight="bold" font-family="monospace">${levelVal.toFixed(1)} %</text>
  `;
}

// ---------------------------------------------------------------------------
// Main NebuProcessView Component
// ---------------------------------------------------------------------------
export class NebuProcessView extends HTMLElement {
  private unsubscribe?: () => void;
  public svgEl!: SVGSVGElement;
  private unitTitleEl!: HTMLElement;
  private unitSubEl!: HTMLElement;

  private zoom: number = 1.0;
  private viewX: number = 0;
  private viewY: number = 0;
  public baseW: number = 1920;
  public baseH: number = 1080;
  private lastRenderedDisplayId: string | null = null;
  private isDragging: boolean = false;
  private dragStartX: number = 0;
  private dragStartY: number = 0;

  connectedCallback() {
    this.innerHTML = `
      <div class="process-view-card" style="display:flex;flex-direction:column;width:100%;height:100%;overflow:hidden;">
        <div class="panel-header">
          <div class="title-group">
            <span class="panel-title" id="processViewTitle">PROCESS GRAPHIC WORKSTATION</span>
            <span class="panel-subtitle text-dim mono" id="processViewSub">ISA-101 HIGH PERFORMANCE OPERATOR STATION</span>
          </div>
          <div class="header-tools">
            <span class="badge badge-info mono" id="pvQualityBadge">ISA-101 HIGH-PERFORMANCE</span>
            <button class="btn-tool-xs" id="btnFitView" title="Reset Viewport Scale">FIT</button>
          </div>
        </div>

        <div class="svg-viewport-wrapper" style="position:relative;flex:1;width:100%;height:100%;overflow:hidden;min-height:0;">
          <svg
            id="processSvg"
            class="process-svg"
            viewBox="0 0 1920 1080"
            preserveAspectRatio="xMidYMid meet"
            xmlns="http://www.w3.org/2000/svg"
          >
            <defs>
              <pattern id="gridPattern" width="24" height="24" patternUnits="userSpaceOnUse">
                <path d="M 24 0 L 0 0 0 24" fill="none" stroke="#141c28" stroke-width="0.8" />
              </pattern>
              <filter id="glow-alarm" x="-30%" y="-30%" width="160%" height="160%">
                <feGaussianBlur stdDeviation="5" result="blur" />
                <feMerge>
                  <feMergeNode in="blur" />
                  <feMergeNode in="SourceGraphic" />
                </feMerge>
              </filter>
              <filter id="glow-cyan" x="-30%" y="-30%" width="160%" height="160%">
                <feGaussianBlur stdDeviation="4" result="blur" />
                <feMerge>
                  <feMergeNode in="blur" />
                  <feMergeNode in="SourceGraphic" />
                </feMerge>
              </filter>
              <marker id="arrow-cyan" viewBox="0 -5 10 10" refX="5" refY="0" markerWidth="6" markerHeight="6" orient="auto">
                <path d="M0,-4 L8,0 L0,4 Z" fill="#38bdf8" />
              </marker>
              <marker id="arrow-steam" viewBox="0 -5 10 10" refX="5" refY="0" markerWidth="6" markerHeight="6" orient="auto">
                <path d="M0,-4 L8,0 L0,4 Z" fill="#f97316" />
              </marker>
            </defs>

            <rect width="100%" height="100%" fill="#0b0f17" />
            <rect width="100%" height="100%" fill="url(#gridPattern)" opacity="0.65" />

            <g id="utilityHeadersLayer"></g>
            <g id="streamsLayer"></g>
            <g id="equipmentLayer"></g>
            <g id="instrumentsLayer"></g>
            <g id="controlLoopsLayer"></g>
          </svg>

          <div id="validationBannerContainer"></div>
          <div id="streamDetailCardContainer"></div>

          <div class="canvas-minimap-container" id="minimapContainer" style="position:absolute;bottom:12px;right:14px;background:#0d131e;border:1px solid #334155;border-radius:4px;padding:3px;box-shadow:0 4px 12px rgba(0,0,0,0.6);z-index:40;cursor:pointer;" title="Process Overview Minimap (Click to Pan)">
            <svg id="minimapSvg" width="140" height="78" viewBox="0 0 1920 1080">
              <rect width="100%" height="100%" fill="#080c14" />
              <g id="minimapEqLayer"></g>
              <rect id="minimapViewportRect" x="0" y="0" width="1920" height="1080" fill="rgba(56, 189, 248, 0.15)" stroke="#38bdf8" stroke-width="12" rx="6"/>
            </svg>
          </div>
        </div>
      </div>
    `;

    this.svgEl = this.querySelector("#processSvg")!;
    this.unitTitleEl = this.querySelector("#processViewTitle")!;
    this.unitSubEl = this.querySelector("#processViewSub")!;

    this.querySelector("#btnFitView")?.addEventListener("click", () => this.fitView());

    this.svgEl.addEventListener("mousedown", (e) => {
      const tag = (e.target as HTMLElement).tagName;
      if (tag === "svg" || tag === "rect") {
        this.isDragging = true;
        this.dragStartX = e.clientX;
        this.dragStartY = e.clientY;
      }
    });

    window.addEventListener("mousemove", (e) => {
      if (!this.isDragging) return;
      const dx = (e.clientX - this.dragStartX) / this.zoom;
      const dy = (e.clientY - this.dragStartY) / this.zoom;
      this.dragStartX = e.clientX;
      this.dragStartY = e.clientY;
      this.viewX -= dx;
      this.viewY -= dy;
      this.updateViewBox();
    });

    window.addEventListener("mouseup", () => {
      this.isDragging = false;
    });

    this.svgEl.addEventListener("wheel", (e) => {
      e.preventDefault();
      const delta = e.deltaY < 0 ? 0.15 : -0.15;
      this.zoomStep(delta);
    });

    const miniSvg = this.querySelector("#minimapSvg");
    miniSvg?.addEventListener("click", (e: any) => {
      const rect = miniSvg.getBoundingClientRect();
      const clickX = ((e.clientX - rect.left) / rect.width) * this.baseW;
      const clickY = ((e.clientY - rect.top) / rect.height) * this.baseH;
      const curW = this.baseW / this.zoom;
      const curH = this.baseH / this.zoom;
      this.viewX = Math.max(-200, Math.min(this.baseW - curW + 200, clickX - curW / 2));
      this.viewY = Math.max(-100, Math.min(this.baseH - curH + 100, clickY - curH / 2));
      this.updateViewBox();
    });

    window.addEventListener("dcs-canvas-fit", () => this.fitView());
    window.addEventListener("dcs-canvas-zoom-in", () => this.zoomStep(0.2));
    window.addEventListener("dcs-canvas-zoom-out", () => this.zoomStep(-0.2));
    window.addEventListener("dcs-canvas-zoom-reset", () => this.resetZoom());

    this.unsubscribe = store.subscribe((s) => this.update(s));
  }

  public updateViewBox() {
    const w = this.baseW / this.zoom;
    const h = this.baseH / this.zoom;
    this.svgEl.setAttribute("viewBox", `${this.viewX} ${this.viewY} ${w} ${h}`);
    this.updateMinimap();
  }

  public computeDisplayContentBounds(vm: DisplayViewModel): {
    minX: number;
    minY: number;
    maxX: number;
    maxY: number;
    hasElements: boolean;
  } {
    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;
    let hasElements = false;

    for (const eq of vm.equipment) {
      hasElements = true;
      const coords = this.getEquipmentCoordinates(eq);
      const w = eq.display_width || 120;
      const h = eq.display_height || 90;
      minX = Math.min(minX, coords.x - w / 2 - 40);
      maxX = Math.max(maxX, coords.x + w / 2 + 40);
      minY = Math.min(minY, coords.y - h / 2 - 30);
      maxY = Math.max(maxY, coords.y + h / 2 + 75);
    }

    for (const st of vm.streams) {
      if (st.route_points && st.route_points.length > 0) {
        hasElements = true;
        for (const pt of st.route_points) {
          minX = Math.min(minX, pt.x - 20);
          maxX = Math.max(maxX, pt.x + 20);
          minY = Math.min(minY, pt.y - 20);
          maxY = Math.max(maxY, pt.y + 20);
        }
      }
    }

    for (const inst of vm.instruments) {
      const pos = inst.position || (inst as any);
      if (pos && pos.x !== undefined && pos.y !== undefined) {
        hasElements = true;
        minX = Math.min(minX, pos.x - 30);
        maxX = Math.max(maxX, pos.x + 30);
        minY = Math.min(minY, pos.y - 30);
        maxY = Math.max(maxY, pos.y + 30);
      }
    }

    if (!hasElements || !isFinite(minX) || !isFinite(minY)) {
      return { minX: 0, minY: 0, maxX: 1920, maxY: 1080, hasElements: false };
    }

    return { minX, minY, maxX, maxY, hasElements: true };
  }

  public fitView(vm?: DisplayViewModel) {
    const targetVm = vm || store.getDisplayViewModel();
    const bounds = this.computeDisplayContentBounds(targetVm);

    if (bounds.hasElements) {
      const padX = 60;
      const padY = 50;
      const rawW = bounds.maxX - bounds.minX + padX * 2;
      const rawH = bounds.maxY - bounds.minY + padY * 2;

      this.baseW = Math.max(960, Math.round(rawW));
      this.baseH = Math.max(540, Math.round(rawH));
      this.viewX = Math.round(bounds.minX - padX);
      this.viewY = Math.round(bounds.minY - padY);
      this.zoom = 1.0;
    } else if (targetVm.viewport && targetVm.viewport.width > 0) {
      this.viewX = targetVm.viewport.x;
      this.viewY = targetVm.viewport.y;
      this.baseW = targetVm.viewport.width;
      this.baseH = targetVm.viewport.height;
      this.zoom = targetVm.viewport.zoom || 1.0;
    } else {
      this.viewX = 0;
      this.viewY = 0;
      this.baseW = 1920;
      this.baseH = 1080;
      this.zoom = 1.0;
    }

    this.updateViewBox();
  }

  private updateMinimap() {
    const miniSvg = this.querySelector("#minimapSvg");
    if (miniSvg) {
      miniSvg.setAttribute("viewBox", `${this.viewX} ${this.viewY} ${this.baseW} ${this.baseH}`);
    }
    const miniRect = this.querySelector("#minimapViewportRect");
    if (miniRect) {
      miniRect.setAttribute("x", `${this.viewX}`);
      miniRect.setAttribute("y", `${this.viewY}`);
      miniRect.setAttribute("width", `${this.baseW / this.zoom}`);
      miniRect.setAttribute("height", `${this.baseH / this.zoom}`);
    }
  }

  public zoomStep(delta: number) {
    const oldZoom = this.zoom;
    this.zoom = Math.max(0.4, Math.min(3.0, this.zoom + delta));
    const curW = this.baseW / this.zoom;
    const curH = this.baseH / this.zoom;
    const oldW = this.baseW / oldZoom;
    const oldH = this.baseH / oldZoom;
    this.viewX += (oldW - curW) / 2;
    this.viewY += (oldH - curH) / 2;
    this.updateViewBox();
  }

  public resetZoom() {
    this.zoom = 1.0;
    this.updateViewBox();
  }

  disconnectedCallback() {
    if (this.unsubscribe) {
      this.unsubscribe();
    }
  }

  private lastFocusedEntityId: string | null = null;

  public render() {
    this.update(store);
  }

  private update(s: Store) {
    const proj = s.activeProject;
    if (!proj) return;

    const activeDisp = s.getActiveDisplay();
    const dispName = activeDisp?.name || "PROCESS FLOWSHEET";
    this.unitTitleEl.textContent = `${dispName.toUpperCase()} (ISA-101)`;
    this.unitSubEl.textContent = `PROJECT: ${proj.name.toUpperCase()} • ${proj.equipment.length} EQUIPMENT • ${proj.instruments.length} INSTRUMENTS`;

    if (s.focusedEntityId && s.focusedEntityId !== this.lastFocusedEntityId) {
      this.lastFocusedEntityId = s.focusedEntityId;
      this.centerOnEntity(s.focusedEntityId, proj);
    }

    const utilityLayer = this.svgEl.querySelector("#utilityHeadersLayer")!;
    const streamsLayer = this.svgEl.querySelector("#streamsLayer")!;
    const equipmentLayer = this.svgEl.querySelector("#equipmentLayer")!;
    const instrumentsLayer = this.svgEl.querySelector("#instrumentsLayer")!;
    const controlLoopsLayer = this.svgEl.querySelector("#controlLoopsLayer")!;

    utilityLayer.innerHTML = "";
    streamsLayer.innerHTML = "";
    equipmentLayer.innerHTML = "";
    instrumentsLayer.innerHTML = "";
    controlLoopsLayer.innerHTML = "";

    // 1. Utility Headers across top
    this.renderUtilityHeaders(utilityLayer, s);

    const dispId = s.activeDisplayId || "disp-unit-01";
    const vm = s.getDisplayViewModel();

    // Auto-fit to active display content bounds on display switch
    if (this.lastRenderedDisplayId !== dispId) {
      this.lastRenderedDisplayId = dispId;
      this.fitView(vm);
    }

    const bannerEl = this.querySelector("#validationBannerContainer");
    if (bannerEl) {
      if (vm.validationErrors && vm.validationErrors.length > 0) {
        bannerEl.innerHTML = `
          <div style="position:absolute;top:12px;left:50%;transform:translateX(-50%);background:#7f1d1d;color:#fecaca;border:1px solid #ef4444;border-radius:4px;padding:6px 14px;font-size:12px;font-family:monospace;box-shadow:0 4px 12px rgba(0,0,0,0.7);z-index:50;display:flex;align-items:center;gap:8px;">
            <span style="font-weight:bold;">⚠ DISPLAY GEOMETRY VALIDATION:</span>
            <span>${vm.validationErrors.join("; ")}</span>
          </div>
        `;
      } else {
        bannerEl.innerHTML = "";
      }
    }

    const renderedEqIds = new Set(vm.equipment.map((e) => e.id));

    this.renderStreams(streamsLayer, proj, s, vm.streams, renderedEqIds);
    if (vm.boundaryConditions && vm.boundaryConditions.length > 0) {
      this.renderBoundaryConditions(streamsLayer, vm.boundaryConditions, vm.equipment, s);
    }
    for (const eq of vm.equipment) {
      this.renderEquipment(equipmentLayer, eq, proj, s);
    }
    for (const inst of vm.instruments) {
      this.renderInstrument(instrumentsLayer, inst, s, proj);
    }
    for (const cl of vm.controlLoops) {
      this.renderControlLoop(controlLoopsLayer, cl, s, proj);
    }

    // 6. Docked Stream Detail Card
    this.renderStreamDetailCard(s, proj);

    // 7. Minimap Equipment (Strictly scoped to active DisplayViewModel)
    const miniEqLayer = this.querySelector("#minimapEqLayer");
    if (miniEqLayer) {
      const vm = s.getDisplayViewModel();
      miniEqLayer.innerHTML = vm.equipment
        .map((eq) => {
          const coords = this.getEquipmentCoordinates(eq);
          const isSel = s.selectedEquipmentId === eq.id;
          return `<rect x="${coords.x - 20}" y="${coords.y - 15}" width="40" height="30" fill="${isSel ? "#38bdf8" : "#475569"}" rx="3" opacity="0.8"/>`;
        })
        .join("");
    }
  }

  public getEquipmentCoordinates(eq: StudioEquipmentDto): { x: number; y: number } {
    const vm = store.getDisplayViewModel();
    const pl = vm.placements.get(eq.id);
    if (pl) {
      return { x: pl.x, y: pl.y };
    }
    if (eq.position) {
      return { x: eq.position.x, y: eq.position.y };
    }
    let fallbackIdx = vm.equipment ? vm.equipment.findIndex((e) => e.id === eq.id) : -1;
    if (fallbackIdx < 0) {
      let hash = 0;
      for (let i = 0; i < (eq.id || "").length; i++) {
        hash = (hash << 5) - hash + eq.id.charCodeAt(i);
        hash |= 0;
      }
      fallbackIdx = Math.abs(hash) % 25;
    }
    return { x: 140 + (fallbackIdx % 5) * 160, y: 160 + Math.floor(fallbackIdx / 5) * 160 };
  }

  public getPortCoordinates(
    eq: StudioEquipmentDto,
    portNameOrId?: string
  ): { x: number; y: number; dir: "left" | "right" | "top" | "bottom" } {
    const coords = this.getEquipmentCoordinates(eq);
    const eqX = coords.x;
    const eqY = coords.y;
    const vm = store.getDisplayViewModel();
    const pl = vm.placements.get(eq.id);
    const rot = pl?.rotation ?? eq.rotation ?? 0;
    const arch = equipmentKind(eq, eq.definition_id);
    const pName = (portNameOrId || "").toLowerCase();

    let relX = 30;
    let relY = 0;
    let normalDir: "left" | "right" | "top" | "bottom" = "right";

    let hw = 30;
    let hh = 40;
    if (arch === "pump") {
      hw = 24;
      hh = 24;
      if (pName.includes("suction") || pName.includes("inlet")) {
        relX = -hw - 4;
        relY = 0;
        normalDir = "left";
      } else if (pName.includes("discharge") || pName.includes("outlet")) {
        relX = 0;
        relY = -hh - 4;
        normalDir = "top";
      } else {
        relX = hw + 4;
        relY = 0;
        normalDir = "right";
      }
    } else if (arch === "compressor") {
      hw = 36;
      hh = 28;
      if (pName.includes("suction") || pName.includes("inlet")) {
        relX = -hw - 4;
        relY = 0;
        normalDir = "left";
      } else if (pName.includes("discharge") || pName.includes("outlet")) {
        relX = 0;
        relY = -hh - 4;
        normalDir = "top";
      } else {
        relX = hw + 4;
        relY = 0;
        normalDir = "right";
      }
    } else if (arch === "valve") {
      hw = 18;
      hh = 20;
      if (pName.includes("inlet") || pName.includes("feed")) {
        relX = -hw - 4;
        relY = 0;
        normalDir = "left";
      } else {
        relX = hw + 4;
        relY = 0;
        normalDir = "right";
      }
    } else if (arch === "column") {
      hw = 24;
      hh = 120;
      if (pName.includes("gas") || pName.includes("overhead") || pName.includes("vapor")) {
        relX = 0;
        relY = -hh - 6;
        normalDir = "top";
      } else if (pName.includes("liquid") || pName.includes("bottom")) {
        relX = 0;
        relY = hh + 6;
        normalDir = "bottom";
      } else if (pName.includes("reflux")) {
        relX = hw + 6;
        relY = -Math.round(hh * 0.6);
        normalDir = "right";
      } else {
        relX = -hw - 6;
        relY = 0;
        normalDir = "left";
      }
    } else if (arch === "reactor") {
      hw = 34;
      hh = 56;
      if (pName.includes("feed") || pName.includes("inlet") || pName.includes("process_in")) {
        relX = 0;
        relY = -hh - 6;
        normalDir = "top";
      } else if (pName.includes("effluent") || pName.includes("outlet") || pName.includes("process_out")) {
        relX = 0;
        relY = hh + 6;
        normalDir = "bottom";
      } else {
        relX = -hw - 6;
        relY = 0;
        normalDir = "left";
      }
    } else if (arch === "heat_exchanger") {
      hw = 35;
      hh = 25;
      if (pName.includes("shell_in")) {
        relX = -hw - 4;
        relY = -10;
        normalDir = "left";
      } else if (pName.includes("shell_out")) {
        relX = hw + 4;
        relY = -10;
        normalDir = "right";
      } else if (pName.includes("tube_in")) {
        relX = -hw - 4;
        relY = 10;
        normalDir = "left";
      } else if (pName.includes("tube_out")) {
        relX = hw + 4;
        relY = 10;
        normalDir = "right";
      } else {
        relX = -hw - 4;
        relY = 0;
        normalDir = "left";
      }
    } else if ((arch as string) === "furnace" || (arch as any) === "heater") {
      hw = 75;
      hh = 55;
      if (pName.includes("process_in") || pName.includes("inlet")) {
        relX = -hw - 4;
        relY = -23;
        normalDir = "left";
      } else if (pName.includes("process_out") || pName.includes("outlet")) {
        relX = hw + 4;
        relY = 39;
        normalDir = "right";
      } else {
        relX = 0;
        relY = hh + 6;
        normalDir = "bottom";
      }
    } else if (arch === "tank") {
      hw = 44;
      hh = 38;
      if (pName.includes("inlet") || pName.includes("feed")) {
        relX = -hw - 4;
        relY = -12;
        normalDir = "left";
      } else {
        relX = hw + 4;
        relY = 12;
        normalDir = "right";
      }
    } else if ((arch as string) === "mixer") {
      hw = 18;
      hh = 18;
      if (pName.includes("primary")) {
        relX = -hw - 4;
        relY = -6;
        normalDir = "left";
      } else if (pName.includes("secondary")) {
        relX = -hw - 4;
        relY = 6;
        normalDir = "left";
      } else {
        relX = hw + 4;
        relY = 0;
        normalDir = "right";
      }
    } else if ((arch as string) === "splitter") {
      hw = 18;
      hh = 18;
      if (pName.includes("feed") || pName.includes("inlet")) {
        relX = -hw - 4;
        relY = 0;
        normalDir = "left";
      } else if (pName.includes("branch_a")) {
        relX = hw + 4;
        relY = -6;
        normalDir = "right";
      } else {
        relX = hw + 4;
        relY = 6;
        normalDir = "right";
      }
    } else {
      // Vessel / Separator / Drum
      hw = 30;
      hh = 50;
      if (pName.includes("gas") || pName.includes("vapor")) {
        relX = 0;
        relY = -hh - 6;
        normalDir = "top";
      } else if (pName.includes("liquid") || pName.includes("bottom") || pName.includes("effluent")) {
        relX = 0;
        relY = hh + 6;
        normalDir = "bottom";
      } else if (pName.includes("psv") || pName.includes("relief")) {
        relX = hw + 4;
        relY = -hh + 10;
        normalDir = "top";
      } else {
        relX = -hw - 6;
        relY = -10;
        normalDir = "left";
      }
    }

    if (rot !== 0) {
      const rad = (rot * Math.PI) / 180;
      const cos = Math.cos(rad);
      const sin = Math.sin(rad);
      const rx = relX * cos - relY * sin;
      const ry = relX * sin + relY * cos;
      return { x: Math.round(eqX + rx), y: Math.round(eqY + ry), dir: normalDir };
    }

    return { x: Math.round(eqX + relX), y: Math.round(eqY + relY), dir: normalDir };
  }

  public centerOnEntity(entityId: string, proj?: ProjectDetailDto) {
    const p: ProjectDetailDto | null = proj || store.activeProject;
    if (!p) return;
    const vm = store.getDisplayViewModel();
    const eq = vm.equipment.find((e: StudioEquipmentDto) => e.id === entityId || e.name === entityId);
    if (eq) {
      const coords = this.getEquipmentCoordinates(eq);
      this.viewX = Math.max(0, coords.x - 450);
      this.viewY = Math.max(0, coords.y - 280);
      this.updateViewBox();
      return;
    }
    const inst = vm.instruments.find((i: StudioInstrumentDto) => i.tag === entityId || i.id === entityId);
    if (inst && inst.position) {
      const cx = inst.position.x;
      const cy = inst.position.y;
      this.viewX = Math.max(0, cx - 450);
      this.viewY = Math.max(0, cy - 280);
      this.updateViewBox();
    }
  }

  private renderUtilityHeaders(container: Element, _s: Store) {
    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");
    g.setAttribute("class", "utility-headers-group");

    g.innerHTML = `
      <!-- HP Steam Header (40 bar) -->
      <line x1="40" y1="36" x2="1240" y2="36" stroke="#f97316" stroke-width="2.5" stroke-dasharray="6 3"/>
      <rect x="42" y="26" width="136" height="18" rx="2" fill="#18130e" stroke="#f97316" stroke-width="0.8"/>
      <text x="110" y="38" text-anchor="middle" fill="#f97316" font-family="monospace" font-size="8.5" font-weight="bold">HP STEAM (40 bar, 380°C)</text>

      <!-- Cooling Water Supply Header -->
      <line x1="40" y1="58" x2="1240" y2="58" stroke="#06b6d4" stroke-width="2"/>
      <rect x="42" y="48" width="130" height="18" rx="2" fill="#0b171c" stroke="#06b6d4" stroke-width="0.8"/>
      <text x="107" y="60" text-anchor="middle" fill="#06b6d4" font-family="monospace" font-size="8.5" font-weight="bold">CW SUPPLY (30°C, 4.5 bar)</text>

      <!-- Fuel Gas Header -->
      <line x1="40" y1="80" x2="1240" y2="80" stroke="#eab308" stroke-width="2"/>
      <rect x="42" y="70" width="130" height="18" rx="2" fill="#18170c" stroke="#eab308" stroke-width="0.8"/>
      <text x="107" y="82" text-anchor="middle" fill="#eab308" font-family="monospace" font-size="8.5" font-weight="bold">FUEL GAS (4.2 bar)</text>
    `;

    container.appendChild(g);
  }

  private renderStreamDetailCard(s: Store, proj: ProjectDetailDto) {
    const container = this.querySelector("#streamDetailCardContainer");
    if (!container) return;

    if (!s.selectedStreamId) {
      container.innerHTML = "";
      return;
    }

    const stream = proj.streams.find((st) => st.name === s.selectedStreamId);
    if (!stream) {
      container.innerHTML = "";
      return;
    }

    let flowRate = s.isSimRunning ? 45.2 : 0.0;
    if ((stream as any).flow_m3_h !== undefined) {
      flowRate = (stream as any).flow_m3_h;
    } else {
      for (const [tag, sample] of s.tags.entries()) {
        if (typeof sample.value === "number" && (tag.toLowerCase().includes("fic") || tag.toLowerCase().includes("flow"))) {
          flowRate = sample.value;
          break;
        }
      }
    }

    const isReverse = flowRate < -0.1;
    const isZero = Math.abs(flowRate) <= 0.1;

    const sourceEq = proj.equipment.find(
      (e) => e.id === (stream as any).source_equipment_id || (stream as any).from_equipment?.includes(e.name) || stream.name.includes(e.name)
    ) || proj.equipment[0];
    const destEq = proj.equipment.find(
      (e) => e.id === (stream as any).destination_equipment_id || (stream as any).to_equipment?.includes(e.name) || (e !== sourceEq && proj.equipment.length > 1)
    ) || (proj.equipment.find((e) => e !== sourceEq) || proj.equipment[1] || proj.equipment[0]);

    container.innerHTML = `
      <div class="stream-detail-card mono font-small" style="position:absolute;bottom:14px;left:50%;transform:translateX(-50%);background:rgba(14,20,31,0.95);border:1px solid #38bdf8;border-radius:4px;padding:3px 10px;box-shadow:0 4px 16px rgba(0,0,0,0.6);z-index:50;display:flex;align-items:center;gap:8px;backdrop-filter:blur(4px);height:28px;">
        <div style="display:flex;align-items:center;gap:6px;">
          <strong class="text-accent" style="font-size:10.5px;color:#38bdf8;">${stream.name}</strong>
          <span class="text-dim" style="font-size:9.5px;">|</span>
          <strong class="text-info" style="font-size:10.5px;color:#f8fafc;">${flowRate.toFixed(1)} m³/h</strong>
          <span class="badge ${isReverse ? "badge-alarm" : isZero ? "badge-secondary" : "badge-good"}" style="font-size:8.5px;padding:1px 5px;line-height:1.1;">
            ${isReverse ? "REVERSE" : isZero ? "NO FLOW" : "NORMAL"}
          </span>
        </div>
        <div style="display:flex;align-items:center;gap:4px;">
          <button class="btn btn-xs btn-outline-primary btn-jump-from" data-eq-id="${sourceEq?.id}" style="font-size:9px;padding:1px 6px;height:20px;display:flex;align-items:center;gap:3px;" title="Jump to Source Equipment">
            <span>⬉</span> <span>${sourceEq?.name || "SRC"}</span>
          </button>
          <button class="btn btn-xs btn-outline-primary btn-jump-to" data-eq-id="${destEq?.id}" style="font-size:9px;padding:1px 6px;height:20px;display:flex;align-items:center;gap:3px;" title="Jump to Destination Equipment">
            <span>${destEq?.name || "DST"}</span> <span>⬈</span>
          </button>
          <button class="btn btn-xs btn-secondary btn-close-stream-card" style="font-size:9px;padding:1px 5px;height:20px;margin-left:2px;" title="Close Stream Info">✕</button>
        </div>
      </div>
    `;

    container.querySelector(".btn-jump-from")?.addEventListener("click", () => {
      if (sourceEq) {
        store.openFaceplate({
          kind: equipmentKind(sourceEq),
          id: sourceEq.id,
          name: sourceEq.name,
          description: sourceEq.name,
        });
        store.setFocusedEntity(sourceEq.id);
        store.selectEquipment(sourceEq.id);
        this.centerOnEntity(sourceEq.id, proj);
      }
    });

    container.querySelector(".btn-jump-to")?.addEventListener("click", () => {
      if (destEq) {
        store.openFaceplate({
          kind: equipmentKind(destEq),
          id: destEq.id,
          name: destEq.name,
          description: destEq.name,
        });
        store.setFocusedEntity(destEq.id);
        store.selectEquipment(destEq.id);
        this.centerOnEntity(destEq.id, proj);
      }
    });

    container.querySelector(".btn-close-stream-card")?.addEventListener("click", () => {
      store.selectStream(null);
    });
  }

  private buildPathWithBridges(
    points: { x: number; y: number }[],
    currentStreamName: string,
    allStreams: { name: string; route_points: { x: number; y: number }[] }[]
  ): string {
    if (points.length < 2) return "";
    let pathStr = `M ${points[0].x},${points[0].y}`;

    for (let i = 0; i < points.length - 1; i++) {
      const p1 = points[i];
      const p2 = points[i + 1];
      const dx = p2.x - p1.x;
      const dy = p2.y - p1.y;
      const len = Math.hypot(dx, dy);

      if (len < 1e-4) continue;

      const ux = dx / len;
      const uy = dy / len;
      const isCurVert = Math.abs(dy) > Math.abs(dx);

      const crossings: { dist: number; ix: number; iy: number }[] = [];

      for (const other of allStreams) {
        if (other.name === currentStreamName) continue;
        for (let j = 0; j < other.route_points.length - 1; j++) {
          const q1 = other.route_points[j];
          const q2 = other.route_points[j + 1];
          const qdx = q2.x - q1.x;
          const qdy = q2.y - q1.y;
          const qlen = Math.hypot(qdx, qdy);
          if (qlen < 1e-4) continue;

          const denom = dx * qdy - dy * qdx;
          if (Math.abs(denom) < 1e-5) continue; // parallel or collinear

          const t = ((q1.x - p1.x) * qdy - (q1.y - p1.y) * qdx) / denom;
          const u = ((q1.x - p1.x) * dy - (q1.y - p1.y) * dx) / denom;

          const marginT = 14 / len;
          const marginU = 14 / qlen;

          if (t > marginT && t < 1 - marginT && u > marginU && u < 1 - marginU) {
            const isOtherVert = Math.abs(qdy) > Math.abs(qdx);
            let shouldBridge = false;
            if (isCurVert && !isOtherVert) {
              shouldBridge = true;
            } else if (!isCurVert && isOtherVert) {
              shouldBridge = false;
            } else {
              shouldBridge = currentStreamName > other.name;
            }

            if (shouldBridge) {
              crossings.push({
                dist: t * len,
                ix: p1.x + t * dx,
                iy: p1.y + t * dy,
              });
            }
          }
        }
      }

      if (crossings.length === 0) {
        pathStr += ` L ${p2.x},${p2.y}`;
      } else {
        crossings.sort((a, b) => a.dist - b.dist);
        const R = 7;
        for (const cr of crossings) {
          const preX = (cr.ix - ux * R).toFixed(1);
          const preY = (cr.iy - uy * R).toFixed(1);
          const postX = (cr.ix + ux * R).toFixed(1);
          const postY = (cr.iy + uy * R).toFixed(1);
          pathStr += ` L ${preX},${preY} A ${R} ${R} 0 0 1 ${postX},${postY}`;
        }
        pathStr += ` L ${p2.x},${p2.y}`;
      }
    }

    return pathStr;
  }

  private renderStreams(
    container: Element,
    proj: ProjectDetailDto,
    s: Store,
    streamsToRender?: StudioStreamDto[],
    renderedEqIds?: Set<string>
  ) {
    const streamsList = streamsToRender || proj.streams;
    const activeEqIds = renderedEqIds || new Set(proj.equipment.map((e) => e.id));
    const activeDisp = store.getActiveDisplay();

    // 1. Pre-categorize boundary streams per equipment to detect multiple connections & opposing stubs
    const outgoingBoundaryStreams = new Map<string, StudioStreamDto[]>();
    const incomingBoundaryStreams = new Map<string, StudioStreamDto[]>();

    for (const stream of streamsList) {
      const sourceEq = proj.equipment.find(
        (e) =>
          e.id === (stream as any).source_equipment_id ||
          e.id === (stream as any).source?.equipment_id ||
          (stream as any).from_equipment?.includes(e.name) ||
          stream.name.includes(e.name)
      );
      const destEq = proj.equipment.find(
        (e) =>
          e.id === (stream as any).sink_equipment_id ||
          e.id === (stream as any).sink?.equipment_id ||
          e.id === (stream as any).destination_equipment_id ||
          (stream as any).to_equipment?.includes(e.name)
      );
      const hasSource = Boolean(sourceEq && activeEqIds.has(sourceEq.id));
      const hasDest = Boolean(destEq && activeEqIds.has(destEq.id));

      if (hasSource && !hasDest && sourceEq) {
        const list = outgoingBoundaryStreams.get(sourceEq.id) || [];
        list.push(stream);
        outgoingBoundaryStreams.set(sourceEq.id, list);
      } else if (hasDest && !hasSource && destEq) {
        const list = incomingBoundaryStreams.get(destEq.id) || [];
        list.push(stream);
        incomingBoundaryStreams.set(destEq.id, list);
      }
    }

    // 2. Resolve and compute route points for all streams to be rendered
    const streamEntries: {
      stream: StudioStreamDto;
      routePoints: { x: number; y: number }[];
      sourceEq?: StudioEquipmentDto;
      destEq?: StudioEquipmentDto;
      hasSource: boolean;
      hasDest: boolean;
      flowRate: number;
    }[] = [];

    for (const stream of streamsList) {
      const sourceEq = proj.equipment.find(
        (e) =>
          e.id === (stream as any).source_equipment_id ||
          e.id === (stream as any).source?.equipment_id ||
          (stream as any).from_equipment?.includes(e.name) ||
          stream.name.includes(e.name)
      );
      const destEq = proj.equipment.find(
        (e) =>
          e.id === (stream as any).sink_equipment_id ||
          e.id === (stream as any).sink?.equipment_id ||
          e.id === (stream as any).destination_equipment_id ||
          (stream as any).to_equipment?.includes(e.name)
      );

      const hasSource = Boolean(sourceEq && activeEqIds.has(sourceEq.id));
      const hasDest = Boolean(destEq && activeEqIds.has(destEq.id));

      if (activeDisp && (activeDisp.equipment_ids?.length || activeDisp.placements?.length)) {
        if (!hasSource && !hasDest) {
          continue;
        }
      }

      let routePoints: { x: number; y: number }[] = [];

      if (hasSource && hasDest && sourceEq && destEq) {
        const srcPortId = stream.source_port_id || (stream as any).source?.port_id;
        const snkPortId = stream.sink_port_id || (stream as any).sink?.port_id;
        const srcPort = this.getPortCoordinates(sourceEq, srcPortId);
        const snkPort = this.getPortCoordinates(destEq, snkPortId);
        if (stream.route_points && stream.route_points.length >= 2) {
          routePoints = stream.route_points.map((p) => ({ ...p }));
          routePoints[0] = { x: srcPort.x, y: srcPort.y };
          routePoints[routePoints.length - 1] = { x: snkPort.x, y: snkPort.y };
        } else {
          routePoints = computePortAwareWaypoints(srcPort, snkPort);
        }
      } else if (hasSource && sourceEq) {
        const srcPortId = stream.source_port_id || (stream as any).source?.port_id;
        const srcPort = this.getPortCoordinates(sourceEq, srcPortId);
        const outList = outgoingBoundaryStreams.get(sourceEq.id) || [];
        const outIdx = Math.max(0, outList.findIndex((st) => st.id === stream.id || st.name === stream.name));
        const outCount = outList.length;

        // Check for neighbor equipment in forward path that might have opposing incoming badge
        const forwardNeighbors = Array.from(activeEqIds)
          .filter((id) => id !== sourceEq.id)
          .map((id) => proj.equipment.find((e) => e.id === id))
          .filter((e): e is StudioEquipmentDto => {
            if (!e) return false;
            const c = this.getEquipmentCoordinates(e);
            return c.x > srcPort.x && Math.abs(c.y - srcPort.y) < 70;
          })
          .sort((a, b) => this.getEquipmentCoordinates(a).x - this.getEquipmentCoordinates(b).x);

        const nearest = forwardNeighbors[0];
        const neighborCoords = nearest ? this.getEquipmentCoordinates(nearest) : null;
        const dist = neighborCoords ? neighborCoords.x - srcPort.x : 999;
        const hasOpposing = nearest ? incomingBoundaryStreams.has(nearest.id) : false;
        const isConstrained = dist < 320 || hasOpposing;

        if (isConstrained) {
          // Track Splitting: Route outgoing TO badge to UPPER track
          const trackY = srcPort.y - 28 - (outIdx * 24);
          const stubLen = Math.min(50, Math.max(20, (dist - 160) * 0.45));
          const endX = srcPort.x + (stubLen || 35);
          routePoints = [
            { x: srcPort.x, y: srcPort.y },
            { x: srcPort.x + 16, y: srcPort.y },
            { x: srcPort.x + 16, y: trackY },
            { x: endX, y: trackY },
          ];
        } else {
          // Staggered horizontal exit with clean 24px vertical separation for multiple badges
          const yShift = (outIdx - (outCount - 1) / 2) * 24;
          const stub = 70;
          const dx = srcPort.dir === "left" ? -stub : srcPort.dir === "right" ? stub : 0;
          const dy = srcPort.dir === "top" ? -stub : srcPort.dir === "bottom" ? stub : 0;
          routePoints = [
            { x: srcPort.x, y: srcPort.y },
            { x: srcPort.x + (dx ? Math.sign(dx) * 16 : 16), y: srcPort.y },
            { x: srcPort.x + (dx ? Math.sign(dx) * 16 : 16), y: srcPort.y + dy + yShift },
            { x: srcPort.x + (dx || 70), y: srcPort.y + dy + yShift },
          ];
        }
      } else if (hasDest && destEq) {
        const snkPortId = stream.sink_port_id || (stream as any).sink?.port_id;
        const snkPort = this.getPortCoordinates(destEq, snkPortId);
        const inList = incomingBoundaryStreams.get(destEq.id) || [];
        const inIdx = Math.max(0, inList.findIndex((st) => st.id === stream.id || st.name === stream.name));
        const inCount = inList.length;

        // Check for upstream neighbor equipment to the left
        const backwardNeighbors = Array.from(activeEqIds)
          .filter((id) => id !== destEq.id)
          .map((id) => proj.equipment.find((e) => e.id === id))
          .filter((e): e is StudioEquipmentDto => {
            if (!e) return false;
            const c = this.getEquipmentCoordinates(e);
            return c.x < snkPort.x && Math.abs(c.y - snkPort.y) < 70;
          })
          .sort((a, b) => this.getEquipmentCoordinates(b).x - this.getEquipmentCoordinates(a).x);

        const nearestUpstream = backwardNeighbors[0];
        const upstreamCoords = nearestUpstream ? this.getEquipmentCoordinates(nearestUpstream) : null;
        const dist = upstreamCoords ? snkPort.x - upstreamCoords.x : 999;
        const hasOpposing = nearestUpstream ? outgoingBoundaryStreams.has(nearestUpstream.id) : false;
        const isConstrained = dist < 320 || hasOpposing;

        if (isConstrained) {
          // Track Splitting: Route incoming FROM badge to LOWER track
          const trackY = snkPort.y + 28 + (inIdx * 24);
          const stubLen = Math.min(50, Math.max(20, (dist - 160) * 0.45));
          const startX = snkPort.x - (stubLen || 35);
          routePoints = [
            { x: startX, y: trackY },
            { x: snkPort.x - 16, y: trackY },
            { x: snkPort.x - 16, y: snkPort.y },
            { x: snkPort.x, y: snkPort.y },
          ];
        } else {
          // Staggered horizontal entry with clean 24px vertical separation for multiple badges
          const yShift = (inIdx - (inCount - 1) / 2) * 24;
          const stub = 70;
          const dx = snkPort.dir === "left" ? -stub : snkPort.dir === "right" ? stub : 0;
          const dy = snkPort.dir === "top" ? -stub : snkPort.dir === "bottom" ? stub : 0;
          routePoints = [
            { x: snkPort.x + (dx || -70), y: snkPort.y + dy + yShift },
            { x: snkPort.x + (dx ? Math.sign(dx) * 16 : -16), y: snkPort.y + dy + yShift },
            { x: snkPort.x + (dx ? Math.sign(dx) * 16 : -16), y: snkPort.y },
            { x: snkPort.x, y: snkPort.y },
          ];
        }
      } else if (stream.route_points && stream.route_points.length >= 2) {
        routePoints = stream.route_points.map((p) => ({ ...p }));
      }

      if (routePoints.length < 2) continue;

      let flowRate = 0.0;
      // 1. Authoritative Stream Flow Tag from Rust Physics
      const directTag =
        s.getTag(`${stream.name}.FLOW`) ||
        s.getTag(`${stream.name.toUpperCase()}.FLOW`) ||
        s.getTag(`${stream.id}.FLOW`);
      if (directTag) {
        flowRate = s.getTagFloat(directTag.tag);
      } else if ((stream as any).flow_m3_h !== undefined) {
        flowRate = (stream as any).flow_m3_h;
      } else if (sourceEq) {
        // 2. Source equipment flow or diagnostics
        const eqFlowTag = s.getTag(`${sourceEq.name}.FLOW`) || s.getTag(`${sourceEq.name.toUpperCase()}.FLOW`);
        if (eqFlowTag) {
          flowRate = s.getTagFloat(eqFlowTag.tag);
        }
      }

      // Check pump/compressor trip or speed or valve closure
      if (sourceEq) {
        const srcDiag = s.diagnosticsMap?.get(sourceEq.id) || s.diagnosticsMap?.get(sourceEq.name);
        const isTripped = Boolean(
          srcDiag?.is_tripped ||
            srcDiag?.sis_override_active ||
            (s.getTag(`${sourceEq.name}.TRIP`) && s.getTagBool(`${sourceEq.name}.TRIP`)) ||
            (s.getTag(`${sourceEq.name}.IS_TRIPPED`) && s.getTagBool(`${sourceEq.name}.IS_TRIPPED`))
        );
        const speedTag = s.getTag(`${sourceEq.name}.SPEED`) || s.getTag(`${sourceEq.name}.SPEED_RPM`);
        const speed = speedTag ? s.getTagFloat(speedTag.tag) : undefined;

        const strokeTag = s.getTag(`${sourceEq.name}.POS`) || s.getTag(`${sourceEq.name}.VALVE_POS`);
        const stroke = strokeTag ? s.getTagFloat(strokeTag.tag) : undefined;

        if (isTripped || (speed !== undefined && speed < 10.0) || (stroke !== undefined && stroke < 0.5)) {
          flowRate = 0.0;
        }
      }

      streamEntries.push({
        stream,
        routePoints,
        sourceEq,
        destEq,
        hasSource,
        hasDest,
        flowRate,
      });
    }

    // Geometries list for crossing bridges calculation
    const streamGeometries = streamEntries.map((e) => ({
      name: e.stream.name,
      route_points: e.routePoints,
    }));

    for (const entry of streamEntries) {
      const { stream, routePoints, sourceEq, destEq, hasSource, hasDest, flowRate } = entry;
      const isSelected = s.selectedStreamId === stream.name;
      const pathD = this.buildPathWithBridges(routePoints, stream.name, streamGeometries);

      const isReverse = flowRate < -0.1;
      const isZero = Math.abs(flowRate) <= 0.1;

      // 1. Heavy Casing / Outer Border (4.5px solid primary process header)
      const casing = document.createElementNS("http://www.w3.org/2000/svg", "path");
      casing.setAttribute("d", pathD);
      casing.setAttribute("stroke", isSelected ? "#00e5ff" : "#0f172a");
      casing.setAttribute("stroke-width", isSelected ? "9" : "7");
      casing.setAttribute("fill", "none");
      casing.setAttribute("stroke-linecap", "round");
      casing.setAttribute("stroke-linejoin", "round");
      casing.setAttribute("cursor", "pointer");
      if (isSelected) {
        casing.setAttribute("filter", "url(#glow-cyan)");
      }
      casing.addEventListener("click", (e) => {
        e.stopPropagation();
        store.selectStream(stream.name);
      });
      container.appendChild(casing);

      // 2. Primary Process Line Body (4.5px)
      const coreColor = isSelected ? "#38bdf8" : isReverse ? "#ef4444" : isZero ? "#334155" : "#0284c7";
      const core = document.createElementNS("http://www.w3.org/2000/svg", "path");
      core.setAttribute("d", pathD);
      core.setAttribute("stroke", coreColor);
      core.setAttribute("stroke-width", "4.5");
      core.setAttribute("fill", "none");
      core.setAttribute("stroke-linecap", "round");
      core.setAttribute("stroke-linejoin", "round");
      core.setAttribute("cursor", "pointer");
      core.addEventListener("click", (e) => {
        e.stopPropagation();
        store.selectStream(stream.name);
      });
      container.appendChild(core);

      // 3. Dynamic Animated Flow Chevrons & Core Dash
      if (!isZero) {
        const flow = document.createElementNS("http://www.w3.org/2000/svg", "path");
        flow.setAttribute("d", pathD);
        flow.setAttribute("stroke", isReverse ? "#fca5a5" : "#7dd3fc");
        flow.setAttribute("stroke-width", "2");
        flow.setAttribute("stroke-dasharray", "6 14");
        flow.setAttribute("fill", "none");
        if (s.isSimRunning) {
          const speed = Math.max(15, Math.min(100, Math.abs(flowRate) * 0.5));
          const mult = isReverse ? speed : -speed;
          const dashOffset = (mult * s.simTime) % 40;
          flow.style.strokeDashoffset = `${dashOffset}`;
        }
        container.appendChild(flow);
      }

      // 4. Prominent Weld Tees at stream endpoints & junctions
      const pStart = routePoints[0];
      const pEnd = routePoints[routePoints.length - 1];

      const startDot = document.createElementNS("http://www.w3.org/2000/svg", "circle");
      startDot.setAttribute("cx", `${pStart.x}`);
      startDot.setAttribute("cy", `${pStart.y}`);
      startDot.setAttribute("r", "4.5");
      startDot.setAttribute("fill", "#94a3b8");
      startDot.setAttribute("stroke", "#0f172a");
      startDot.setAttribute("stroke-width", "1.5");
      container.appendChild(startDot);

      const endDot = document.createElementNS("http://www.w3.org/2000/svg", "circle");
      endDot.setAttribute("cx", `${pEnd.x}`);
      endDot.setAttribute("cy", `${pEnd.y}`);
      endDot.setAttribute("r", "4.5");
      endDot.setAttribute("fill", "#94a3b8");
      endDot.setAttribute("stroke", "#0f172a");
      endDot.setAttribute("stroke-width", "1.5");
      container.appendChild(endDot);

      // 5. Flow Direction Arrow Chevrons along each segment
      for (let i = 0; i < routePoints.length - 1; i++) {
        const p1 = routePoints[i];
        const p2 = routePoints[i + 1];
        const dx = p2.x - p1.x;
        const dy = p2.y - p1.y;
        const len = Math.hypot(dx, dy);
        if (len > 35) {
          const mx = p1.x + dx * 0.5;
          const my = p1.y + dy * 0.5;
          let angleDeg = (Math.atan2(dy, dx) * 180) / Math.PI;
          if (isReverse) angleDeg += 180;

          const chevron = document.createElementNS("http://www.w3.org/2000/svg", "polygon");
          chevron.setAttribute("points", "-6,-4 4,0 -6,4");
          chevron.setAttribute("fill", isReverse ? "#ef4444" : isZero ? "#64748b" : "#38bdf8");
          chevron.setAttribute("transform", `translate(${mx},${my}) rotate(${angleDeg})`);
          container.appendChild(chevron);
        }
      }

      // 6. Clickable Off-Page TO / FROM Continuation Arrow Badges
      // ONLY render off-page continuation badge when destination is on another display
      if (destEq && !hasDest) {
        const toBadge = document.createElementNS("http://www.w3.org/2000/svg", "g");
        toBadge.setAttribute("class", "stream-connector-badge off-page-badge");
        toBadge.setAttribute("cursor", "pointer");
        toBadge.innerHTML = `
          <polygon points="${pEnd.x + 6},${pEnd.y - 9} ${pEnd.x + 94},${pEnd.y - 9} ${pEnd.x + 104},${pEnd.y} ${pEnd.x + 94},${pEnd.y + 9} ${pEnd.x + 6},${pEnd.y + 9}" fill="#0f172a" stroke="#38bdf8" stroke-width="1"/>
          <text x="${pEnd.x + 50}" y="${pEnd.y + 3}" text-anchor="middle" font-family="monospace" font-size="8" font-weight="bold" fill="#38bdf8">──► TO ${destEq.name}</text>
        `;
        toBadge.addEventListener("click", (e) => {
          e.stopPropagation();

          const explicitLink = (proj.navigation_links || []).find(
            (l) => l.target_element_id === destEq.id || (l.source_element_id === destEq.id && l.source_display_id === activeDisp?.id)
          );

          if (explicitLink) {
            store.executeNavigationLink(explicitLink);
          } else {
            const targetDisp = store.displays.find((d) => d.equipment_ids?.includes(destEq.id));
            if (targetDisp) {
              store.navigateDisplay(targetDisp.id);
            }
            store.setFocusedEntity(destEq.id);
            store.selectEquipment(destEq.id);
            store.openFaceplate({
              kind: equipmentKind(destEq),
              id: destEq.id,
              name: destEq.name,
              description: destEq.name,
            });
          }
        });
        container.appendChild(toBadge);
      }

      // ONLY render off-page continuation badge when source is on another display
      if (sourceEq && !hasSource) {
        const fromBadge = document.createElementNS("http://www.w3.org/2000/svg", "g");
        fromBadge.setAttribute("class", "stream-connector-badge off-page-badge");
        fromBadge.setAttribute("cursor", "pointer");
        fromBadge.innerHTML = `
          <polygon points="${pStart.x - 6},${pStart.y - 9} ${pStart.x - 94},${pStart.y - 9} ${pStart.x - 104},${pStart.y} ${pStart.x - 94},${pStart.y + 9} ${pStart.x - 6},${pStart.y + 9}" fill="#0f172a" stroke="#38bdf8" stroke-width="1"/>
          <text x="${pStart.x - 52}" y="${pStart.y + 3}" text-anchor="middle" font-family="monospace" font-size="8" font-weight="bold" fill="#38bdf8">FROM ${sourceEq.name} ──►</text>
        `;
        fromBadge.addEventListener("click", (e) => {
          e.stopPropagation();

          const explicitLink = (proj.navigation_links || []).find(
            (l) => l.target_element_id === sourceEq.id
          );

          if (explicitLink) {
            store.executeNavigationLink(explicitLink);
          } else {
            const targetDisp = store.displays.find((d) => d.equipment_ids?.includes(sourceEq.id));
            if (targetDisp) {
              store.navigateDisplay(targetDisp.id);
            }
            store.setFocusedEntity(sourceEq.id);
            store.selectEquipment(sourceEq.id);
            store.openFaceplate({
              kind: equipmentKind(sourceEq),
              id: sourceEq.id,
              name: sourceEq.name,
              description: sourceEq.name,
            });
          }
        });
        container.appendChild(fromBadge);
      }

      // 7. Stream Label with Flowrate Readout
      const mid = routePoints[Math.floor(routePoints.length / 2)];
      const label = document.createElementNS("http://www.w3.org/2000/svg", "text");
      label.setAttribute("x", `${mid.x}`);
      label.setAttribute("y", `${mid.y - 12}`);
      label.setAttribute("text-anchor", "middle");
      label.setAttribute("fill", isSelected ? "#00e5ff" : isReverse ? "#ef4444" : isZero ? "#64748b" : "#7dd3fc");
      label.setAttribute("font-size", "9.5");
      label.setAttribute("font-family", "monospace");
      label.setAttribute("font-weight", isSelected || isReverse ? "bold" : "normal");
      label.setAttribute("cursor", "pointer");
      label.textContent = isReverse
        ? `${stream.name} [${flowRate.toFixed(1)} m³/h REVERSE]`
        : isZero
        ? `${stream.name} [0.0 m³/h]`
        : `${stream.name} [${flowRate.toFixed(1)} m³/h]`;
      label.addEventListener("click", (e) => {
        e.stopPropagation();
        store.selectStream(stream.name);
      });
      container.appendChild(label);
    }
  }

  private renderEquipment(
    container: Element,
    eq: StudioEquipmentDto,
    proj: ProjectDetailDto,
    s: Store
  ) {
    const coords = this.getEquipmentCoordinates(eq);
    const cx = coords.x;
    const cy = coords.y;
    const kind = equipmentKind(eq, eq.definition_id);

    // Active alarms on this equipment
    const eqInstruments = proj.instruments.filter((i) => i.equipment_id === eq.id);
    const activeEqAlarms = Array.from(s.alarms.values()).filter((alm) => {
      if (alm.state !== "ACTIVE_UNACK" && alm.state !== "ACTIVE_ACK") return false;
      return (
        alm.alarm_name.includes(eq.name) ||
        alm.alarm_name.includes(eq.id) ||
        eqInstruments.some((i) => alm.alarm_name.includes(i.tag))
      );
    });
    const hasActiveAlarm = activeEqAlarms.length > 0;
    const hasUnackAlarm = activeEqAlarms.some((a) => a.state === "ACTIVE_UNACK" || !(a as any).acknowledged);

    // Live diagnostics from simulation core
    const diag = s.diagnosticsMap.get(eq.id) || s.diagnosticsMap.get(eq.name);
    const isTripped = diag?.is_tripped || diag?.sis_override_active || false;
    const isFocused = s.focusedEntityId === eq.id || s.selectedEquipmentId === eq.id;
    const isCritical = activeEqAlarms.some((alm) => alm.severity === "CRITICAL");

    const haloStroke = isTripped || isCritical
      ? "#ef4444"
      : hasActiveAlarm
      ? "#f59e0b"
      : isFocused
      ? "#00e5ff"
      : "transparent";

    const haloFilter = isTripped || hasActiveAlarm
      ? `filter="url(#glow-alarm)"`
      : isFocused
      ? `filter="url(#glow-cyan)"`
      : "";

    let levelVal = 50.0;
    const levelInst = eqInstruments.find((i) => i.tag.includes("LT") || i.tag.includes("LIC") || i.tag.includes("LEVEL"));
    if (levelInst && s.getTag(levelInst.tag)) {
      levelVal = s.getTagFloat(levelInst.tag);
    } else if (s.getTag(`${eq.name}.LEVEL`)) {
      levelVal = s.getTagFloat(`${eq.name}.LEVEL`);
    } else if (s.getTag(`${eq.id}.LEVEL`)) {
      levelVal = s.getTagFloat(`${eq.id}.LEVEL`);
    } else if (eq.parameters?.level !== undefined) {
      levelVal = eq.parameters.level;
    } else {
      const init = proj.initial_conditions?.find((ic) => ic.entity_id === eq.name && ic.variable === "level");
      if (init) {
        levelVal = init.value <= 1.0 ? init.value * 100.0 : init.value;
      } else {
        for (const [tag, sample] of s.tags.entries()) {
          if (
            typeof sample.value === "number" &&
            (tag.toLowerCase().includes("lic") || tag.toLowerCase().includes("level"))
          ) {
            levelVal = sample.value;
            break;
          }
        }
      }
    }

    const tempInst = eqInstruments.find((i) => i.tag.includes("TT") || i.tag.includes("TIC") || i.tag.includes("TEMP"));
    const tempVal = tempInst ? s.getTagFloat(tempInst.tag, 332.0) : s.getTagFloat(`${eq.name}.TEMP`, eq.parameters?.operating_temp ?? 332.0);
    const pressInst = eqInstruments.find((i) => i.tag.includes("PT") || i.tag.includes("PIC") || i.tag.includes("PRESS"));
    const pressVal = pressInst ? s.getTagFloat(pressInst.tag, 32.0) : s.getTagFloat(`${eq.name}.PRESS`, eq.parameters?.operating_pressure ? eq.parameters.operating_pressure / 100000.0 : 32.0);

    const targetLoop = proj.control_loops.find((cl) =>
      eqInstruments.some((i) => i.tag === cl.pv_tag)
    );
    const eqSpeed = s.getTagFloat(`${eq.name}.SPEED`) ?? s.getTagFloat(`${eq.name}.SPEED_RPM`) ?? diag?.speed_rpm;
    const eqFlow = s.getTagFloat(`${eq.name}.FLOW`);
    const isRunning = !isTripped && (eqSpeed !== undefined ? eqSpeed > 50 : eqFlow !== undefined ? eqFlow > 0.2 : diag ? (diag.speed_rpm !== undefined ? diag.speed_rpm > 50 : s.isSimRunning) : s.isSimRunning);
    const tagPos = s.getTagFloat(`${eq.name}.POS`) ?? s.getTagFloat(`${eq.name}.VALVE_POS`);
    const actStroke = tagPos ?? diag?.valve_actual_pct ?? (targetLoop?.out_tag ? s.getTagFloat(targetLoop.out_tag, 55.2) : 55.2);
    const cmdStroke = diag?.valve_command_pct ?? actStroke;

    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");

    // Exact unique IDs for Test Suite backward compatibility (assign to first instance of each kind)
    if (kind === "pump" && !container.querySelector("#pumpGroup")) {
      g.id = "pumpGroup";
    } else if ((kind === "vessel" || kind === "separator") && !container.querySelector("#vesselGroup")) {
      g.id = "vesselGroup";
    } else if (kind === "valve" && !container.querySelector("#valveGroup")) {
      g.id = "valveGroup";
    } else if (kind === "compressor" && !container.querySelector("#compressorGroup")) {
      g.id = "compressorGroup";
    } else if (kind === "reactor" && !container.querySelector("#reactorGroup")) {
      g.id = "reactorGroup";
    } else if (kind === "column" && !container.querySelector("#columnGroup")) {
      g.id = "columnGroup";
    } else if (kind === "heat_exchanger" && !container.querySelector("#heatExchangerGroup")) {
      g.id = "heatExchangerGroup";
    } else if (kind === "tank" && !container.querySelector("#tankGroup")) {
      g.id = "tankGroup";
    } else if ((kind as any) === "furnace" && !container.querySelector("#heaterGroup")) {
      g.id = "heaterGroup";
    }

    g.setAttribute(
      "class",
      `equipment-node clickable ${isTripped ? "trip-active" : ""} ${
        hasActiveAlarm || isTripped ? (hasUnackAlarm ? "alarm-pulsing" : "alarm-active") : ""
      }`
    );
    g.setAttribute("cursor", "pointer");
    g.setAttribute("role", "button");
    g.setAttribute("data-eq-id", eq.id);
    g.setAttribute("data-cx", String(cx));
    g.setAttribute("data-cy", String(cy));
    const activeDisp = store.getActiveDisplay();
    const pl = activeDisp?.placements?.find((p) => p.equipment_id === eq.id);
    const rotation = pl?.rotation ?? eq.rotation ?? 0;
    g.setAttribute("data-rotation", String(rotation));
    if (rotation) {
      g.setAttribute("transform", `rotate(${rotation} ${cx} ${cy})`);
    }

    const vm = store.getDisplayViewModel();
    const optimalZone = determineOptimalTelemetryZone(
      {
        id: eq.id,
        name: eq.name,
        position: { x: cx, y: cy },
        definition_id: eq.definition_id,
        display_width: eq.display_width,
        display_height: eq.display_height,
        rotation,
        scale: 1.0,
      },
      vm.equipment,
      vm.streams
    );

    const eqBounds = calculateEquipmentBounds(
      {
        name: eq.name,
        position: { x: cx, y: cy },
        definition_id: eq.definition_id,
        display_width: eq.display_width,
        display_height: eq.display_height,
        rotation,
        scale: 1.0,
      },
      0,
      { telemetryZone: optimalZone }
    );

    const haloR = kind === "column" ? 72 : kind === "reactor" ? 64 : kind === "vessel" || kind === "separator" ? 54 : (kind as any) === "furnace" ? 64 : 44;

    // Render detailed symbol body
    let symbolHtml = "";
    switch (kind) {
      case "pump":
        symbolHtml = renderPumpSymbolHtml(cx, cy, isRunning, isTripped, diag);
        break;
      case "compressor":
        symbolHtml = renderCompressorSymbolHtml(cx, cy, isRunning, isTripped, diag);
        break;
      case "reactor":
        symbolHtml = renderReactorSymbolHtml(cx, cy, tempVal, pressVal, isTripped, diag);
        break;
      case "column":
        symbolHtml = renderColumnSymbolHtml(cx, cy, levelVal, isTripped, diag);
        break;
      case "furnace" as any:
        symbolHtml = renderFurnaceSymbolHtml(cx, cy, isRunning, isTripped, diag, store);
        break;
      case "heat_exchanger":
        symbolHtml = renderExchangerSymbolHtml(cx, cy, isTripped, diag);
        break;
      case "valve":
        symbolHtml = renderValveSymbolHtml(cx, cy, actStroke, cmdStroke, isTripped, diag);
        break;
      case "tank":
        symbolHtml = renderTankSymbolHtml(cx, cy, levelVal, isTripped, diag);
        break;
      case "separator":
      case "vessel":
      default:
        symbolHtml = renderVesselSymbolHtml(cx, cy, levelVal, isTripped, diag);
        break;
    }

    // Contextual ISA-18.2 In-Situ Equipment Alarm Badge
    let eqAlarmBadge = "";
    if (hasActiveAlarm) {
      const badgeFill = isCritical ? "#ef4444" : "#f59e0b";
      const badgeText = isCritical ? "▲▲ TRIP / CRIT" : "▲ ALARM";
      eqAlarmBadge = `
        <g class="${hasUnackAlarm ? "alarm-pulsing" : ""}">
          <rect x="${cx - 38}" y="${cy - haloR - 16}" width="76" height="15" rx="2" fill="${badgeFill}" stroke="#ffffff" stroke-width="0.75"/>
          <text x="${cx}" y="${cy - haloR - 5}" text-anchor="middle" fill="#000000" font-size="8.5" font-weight="bold" font-family="monospace">${badgeText}</text>
        </g>
      `;
    }

    // Prominent SIS ESD Trip overlay banner
    const tripOverlay = isTripped
      ? `
        <g class="sis-trip-banner">
          <rect x="${cx - 38}" y="${cy - 12}" width="76" height="24" rx="3" fill="#ef4444" stroke="#ffffff" stroke-width="1.5"/>
          <text x="${cx}" y="${cy + 4}" text-anchor="middle" fill="#ffffff" font-size="9.5" font-weight="bold" font-family="monospace">ESD TRIP</text>
        </g>
      `
      : "";

    // Industrial DCS On-Graphic Live Equipment Telemetry Plaque
    const tb = eqBounds.telemetryBox;
    const tbCenterX = tb.x + tb.width / 2;
    let eqTelemetryPlaque = "";
    if (kind === "pump") {
      const speed = diag?.speed_rpm ?? (isRunning ? 2950 : 0);
      const flow = diag?.flow_m3_h ?? (isRunning ? 45.2 : 0.0);
      const power = diag?.power_kw ?? (isRunning ? 75.4 : 0.0);
      const stateDot = isTripped
        ? `<tspan fill="#ef4444">▲▲ TRIP</tspan>`
        : isRunning
        ? `<tspan fill="#10b981">● RUN</tspan>`
        : `<tspan fill="#64748b">■ STOP</tspan>`;

      eqTelemetryPlaque = `
        <g class="eq-telemetry-badge" pointer-events="none">
          <rect x="${tb.x}" y="${tb.y}" width="${tb.width}" height="${tb.height}" rx="2" fill="#090e17" fill-opacity="0.94" stroke="#334155" stroke-width="0.8"/>
          <text x="${tbCenterX}" y="${tb.y + 11}" text-anchor="middle" font-family="monospace" font-size="8" font-weight="bold">${stateDot} <tspan fill="#38bdf8">${speed.toFixed(0)} RPM</tspan></text>
          <text x="${tbCenterX}" y="${tb.y + 21}" text-anchor="middle" font-family="monospace" font-size="7.5" fill="#94a3b8"><tspan fill="#e2e8f0">${flow.toFixed(0)}m³/h</tspan> | ${power.toFixed(0)}kW</text>
        </g>
      `;
    } else if (kind === "valve") {
      const mismatch = Math.abs(cmdStroke - actStroke);
      const isMoving = mismatch > 2.0 && s.isSimRunning;
      const isFault = mismatch > 5.0 && !isMoving;
      const valveStatus = isFault
        ? `<tspan fill="#ef4444" font-weight="bold">⚠ FLT</tspan>`
        : isMoving
        ? `<tspan fill="#38bdf8">⟳ MOV</tspan>`
        : actStroke > 5
        ? `<tspan fill="#10b981">● OPN</tspan>`
        : `<tspan fill="#64748b">■ CLS</tspan>`;

      eqTelemetryPlaque = `
        <g class="eq-telemetry-badge" pointer-events="none">
          <rect x="${tb.x}" y="${tb.y}" width="${tb.width}" height="${tb.height}" rx="2" fill="#090e17" fill-opacity="0.94" stroke="${isFault ? "#ef4444" : "#334155"}" stroke-width="0.8"/>
          <text x="${tbCenterX}" y="${tb.y + 11}" text-anchor="middle" font-family="monospace" font-size="8" font-weight="bold">${valveStatus} <tspan fill="#38bdf8">ACT: ${actStroke.toFixed(0)}%</tspan></text>
          <text x="${tbCenterX}" y="${tb.y + 21}" text-anchor="middle" font-family="monospace" font-size="7.5" fill="#94a3b8">CMD: ${cmdStroke.toFixed(0)}%</text>
        </g>
      `;
    } else if (kind === "compressor") {
      const speed = diag?.speed_rpm ?? (isRunning ? 9800 : 0);
      const ratio = diag?.pressure_ratio !== undefined ? diag.pressure_ratio.toFixed(1) : "3.5";
      const stateDot = isTripped
        ? `<tspan fill="#ef4444">▲▲ TRIP</tspan>`
        : isRunning
        ? `<tspan fill="#10b981">● RUN</tspan>`
        : `<tspan fill="#64748b">■ IDLE</tspan>`;

      eqTelemetryPlaque = `
        <g class="eq-telemetry-badge" pointer-events="none">
          <rect x="${tb.x}" y="${tb.y}" width="${tb.width}" height="${tb.height}" rx="2" fill="#090e17" fill-opacity="0.94" stroke="#334155" stroke-width="0.8"/>
          <text x="${tbCenterX}" y="${tb.y + 11}" text-anchor="middle" font-family="monospace" font-size="8" font-weight="bold">${stateDot} <tspan fill="#38bdf8">${speed.toFixed(0)} RPM</tspan></text>
          <text x="${tbCenterX}" y="${tb.y + 21}" text-anchor="middle" font-family="monospace" font-size="7.5" fill="#94a3b8">PR: ${ratio}</text>
        </g>
      `;
    } else if (kind === "column") {
      eqTelemetryPlaque = `
        <g class="eq-telemetry-badge" pointer-events="none">
          <rect x="${tb.x}" y="${tb.y}" width="${tb.width}" height="${tb.height}" rx="2" fill="#090e17" fill-opacity="0.94" stroke="#334155" stroke-width="0.8"/>
          <text x="${tbCenterX}" y="${tb.y + 15}" text-anchor="middle" font-family="monospace" font-size="8" fill="#94a3b8"><tspan fill="#38bdf8">LVL: ${levelVal.toFixed(1)}%</tspan> | 1.8b</text>
        </g>
      `;
    } else if (kind === "vessel" || kind === "separator") {
      const p = diag?.discharge_pressure_kpa ? (diag.discharge_pressure_kpa / 100).toFixed(1) : "34.2";
      eqTelemetryPlaque = `
        <g class="eq-telemetry-badge" pointer-events="none">
          <rect x="${tb.x}" y="${tb.y}" width="${tb.width}" height="${tb.height}" rx="2" fill="#090e17" fill-opacity="0.94" stroke="#334155" stroke-width="0.8"/>
          <text x="${tbCenterX}" y="${tb.y + 15}" text-anchor="middle" font-family="monospace" font-size="8" fill="#94a3b8"><tspan fill="#38bdf8">LVL: ${levelVal.toFixed(1)}%</tspan> | ${p}b</text>
        </g>
      `;
    } else if (kind === "heat_exchanger") {
      const duty = diag?.thermal_duty_kw ? diag.thermal_duty_kw.toFixed(0) : "2450";
      const dt = diag?.delta_t_c ? diag.delta_t_c.toFixed(0) : "48";
      eqTelemetryPlaque = `
        <g class="eq-telemetry-badge" pointer-events="none">
          <rect x="${tb.x}" y="${tb.y}" width="${tb.width}" height="${tb.height}" rx="2" fill="#090e17" fill-opacity="0.94" stroke="#334155" stroke-width="0.8"/>
          <text x="${tbCenterX}" y="${tb.y + 15}" text-anchor="middle" font-family="monospace" font-size="8" fill="#94a3b8"><tspan fill="#38bdf8">${duty}kW</tspan> | ΔT:${dt}°C</text>
        </g>
      `;
    } else if ((kind as any) === "furnace") {
      const duty = store.getTagFloat("F-101.DUTY", 4.8);
      const cot = store.getTagFloat("TT-101", 332.0);
      eqTelemetryPlaque = `
        <g class="eq-telemetry-badge" pointer-events="none">
          <rect x="${tb.x}" y="${tb.y}" width="${tb.width}" height="${tb.height}" rx="2" fill="#090e17" fill-opacity="0.94" stroke="#334155" stroke-width="0.8"/>
          <text x="${tbCenterX}" y="${tb.y + 15}" text-anchor="middle" font-family="monospace" font-size="8" fill="#94a3b8"><tspan fill="#f59e0b">${duty.toFixed(1)}MW</tspan> | COT:${cot.toFixed(0)}°C</text>
        </g>
      `;
    } else if (kind === "reactor") {
      eqTelemetryPlaque = `
        <g class="eq-telemetry-badge" pointer-events="none">
          <rect x="${tb.x}" y="${tb.y}" width="${tb.width}" height="${tb.height}" rx="2" fill="#090e17" fill-opacity="0.94" stroke="#334155" stroke-width="0.8"/>
          <text x="${tbCenterX}" y="${tb.y + 15}" text-anchor="middle" font-family="monospace" font-size="8" fill="#94a3b8"><tspan fill="#38bdf8">${tempVal.toFixed(1)}°C</tspan> | ${pressVal.toFixed(1)}b</text>
        </g>
      `;
    }

    g.innerHTML = `
      <circle class="focus-ring" cx="${cx}" cy="${cy}" r="${haloR}"
        fill="none" stroke="${haloStroke}" stroke-width="2.5" ${haloFilter}/>

      ${symbolHtml}

      ${eqAlarmBadge}
      ${tripOverlay}

      <g class="eq-name-banner">
        <rect x="${eqBounds.labelBox.x}" y="${eqBounds.labelBox.y}" width="${eqBounds.labelBox.width}" height="${eqBounds.labelBox.height}" rx="2" fill="#090e17" fill-opacity="0.94" stroke="#38bdf8" stroke-width="0.8"/>
        <text x="${cx}" y="${eqBounds.labelBox.y + Math.round(eqBounds.labelBox.height * 0.75)}" text-anchor="middle"
          fill="#f8fafc" font-weight="bold" font-size="9" font-family="monospace">
          ${eq.name}
        </text>
      </g>

      ${eqTelemetryPlaque}
    `;

    // Bind click events on any in-situ instrument nodes inside the equipment graphic
    const inSituNodes = g.querySelectorAll("[data-inst-tag]");
    inSituNodes.forEach((node) => {
      node.addEventListener("click", (e) => {
        e.stopPropagation();
        const tag = node.getAttribute("data-inst-tag");
        if (!tag) return;
        const inst = proj?.instruments.find((i) => i.tag === tag);
        store.openFaceplate({
          kind: "instrument",
          id: tag,
          name: tag,
          description: inst ? `${inst.measurement_kind} — ${tag}` : `HEATER TELEMETRY — ${tag}`,
          pvTag: tag,
          unit: inst?.unit || "",
          rangeMin: inst?.range_min,
          rangeMax: inst?.range_max,
        });
        store.setFocusedEntity(tag);
      });
    });

    g.addEventListener("click", () => {
      store.openFaceplate({
        kind,
        id: eq.id,
        name: eq.name,
        description: `${kind.toUpperCase()} — ${eq.name}`,
        pvTag: targetLoop?.pv_tag ?? eqInstruments[0]?.tag,
        outTag: targetLoop?.out_tag,
        parameters: eq.parameters,
      });
      store.setFocusedEntity(eq.id);
      store.selectEquipment(eq.id);
    });

    container.appendChild(g);
  }

  private renderInstrument(
    container: Element,
    inst: StudioInstrumentDto,
    s: Store,
    proj?: ProjectDetailDto
  ) {
    const inSituFurnaceTags = new Set([
      "TI-104", "AI-101", "PDT-101", "TI-101-CIT", "TI-101-RIT", "TI-101-ROT",
      "TT-101", "TI-101-SK1", "TI-101-SK2", "FT-Air", "PT-Air", "FZ-101",
      "PT-104", "FT-105", "PT-105", "PT-107", "FT-107", "FFC-101"
    ]);
    if (inst.equipment_id && inSituFurnaceTags.has(inst.tag)) {
      const parentEq = proj?.equipment.find((e) => e.id === inst.equipment_id);
      if (parentEq && equipmentKind(parentEq, parentEq.definition_id) === "furnace") {
        return;
      }
    }

    let ix = inst.position?.x ?? 200;
    let iy = inst.position?.y ?? 100;

    if (inst.equipment_id && proj) {
      const parentEq = proj.equipment.find((e) => e.id === inst.equipment_id);
      if (parentEq) {
        const coords = this.getEquipmentCoordinates(parentEq);
        if (inst.position && parentEq.position) {
          const dx = inst.position.x - parentEq.position.x;
          const dy = inst.position.y - parentEq.position.y;
          ix = coords.x + dx;
          iy = coords.y + dy;
        } else {
          const eqInstruments = (proj.instruments || []).filter((i) => i.equipment_id === parentEq.id);
          const instIdx = Math.max(0, eqInstruments.findIndex((i) => i.id === inst.id));
          const kind = equipmentKind(parentEq, parentEq.definition_id);
          const haloR = kind === "column" ? 72 : kind === "reactor" ? 64 : kind === "vessel" || kind === "separator" ? 54 : 44;
          ix = coords.x - 52 - (instIdx * 48);
          iy = coords.y - haloR - 22;
        }
      }
    }
    const { letters, loopNum } = instrumentAbbrev(inst.tag, inst.measurement_kind);
    const tagSample = s.tags.get(inst.tag);
    const quality: string = tagSample?.quality ?? (s.tags.has(inst.tag) ? "GOOD" : "NOT_CONNECTED");
    const tagVal = s.getTagFloat(inst.tag, Number.NaN);

    const isBad = quality === "BAD" || quality === "NOT_CONNECTED" || Number.isNaN(tagVal);
    const valStr = isBad ? "----" : tagVal.toFixed(1);
    const valColor = isBad ? "#ef4444" : "#38bdf8";

    const instAlarm = Array.from(s.alarms.values()).find(
      (a) =>
        a.alarm_name.includes(inst.tag) &&
        a.state !== "NORMAL" &&
        a.state !== "CLEARED"
    );

    let alarmBadgeSvg = "";
    if (instAlarm) {
      const isUnack = instAlarm.state === "ACTIVE_UNACK" || !(instAlarm as any).acknowledged;
      const isCrit = instAlarm.severity === "CRITICAL";
      const badgeFill = isCrit ? "#ef4444" : "#f59e0b";
      const textLabel = isCrit ? "▲▲ HIHI" : "▲ HIGH";
      alarmBadgeSvg = `
        <g class="${isUnack ? "alarm-pulsing" : ""}">
          <rect x="${ix - 24}" y="${iy - 44}" width="48" height="13" rx="2" fill="${badgeFill}" stroke="#ffffff" stroke-width="0.75"/>
          <text x="${ix}" y="${iy - 35}" text-anchor="middle" fill="#000000" font-family="monospace" font-size="8" font-weight="bold">${textLabel}</text>
        </g>
      `;
    }

    let validityFlag = `<tspan fill="#10b981">[V]</tspan>`;
    if (quality === "BAD") validityFlag = `<tspan fill="#ef4444" font-weight="bold">[!]</tspan>`;
    else if (quality === "UNCERTAIN") validityFlag = `<tspan fill="#f59e0b">[?]</tspan>`;
    else if (quality === "NOT_CONNECTED") validityFlag = `<tspan fill="#a855f7">[X]</tspan>`;

    const isFocused = s.focusedEntityId === inst.tag;
    const safeInstId = `${inst.tag.toLowerCase().replace(/[^a-z0-9]/g, "")}Val`;

    const circleStroke = isFocused
      ? "#00e5ff"
      : instAlarm
      ? (instAlarm.severity === "CRITICAL" ? "#ef4444" : "#f59e0b")
      : isBad
      ? "#ef4444"
      : "#475569";

    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");
    g.setAttribute("class", "equipment-node clickable");
    g.setAttribute("cursor", "pointer");
    g.setAttribute("data-inst-tag", inst.tag);

    g.innerHTML = `
      <line x1="${ix}" y1="${iy + 15}" x2="${ix}" y2="${iy + 30}" stroke="#64748b" stroke-width="1.5" stroke-dasharray="3 3"/>

      <circle cx="${ix}" cy="${iy}" r="15" fill="#0f172a" stroke="${circleStroke}" stroke-width="${isFocused || instAlarm ? "2.5" : "1.8"}" class="inst-circle"/>
      <line x1="${ix - 15}" y1="${iy}" x2="${ix + 15}" y2="${iy}" stroke="${circleStroke}" stroke-width="1.2"/>

      <text x="${ix}" y="${iy - 3}" text-anchor="middle" fill="#38bdf8" font-size="8.5" font-weight="bold" font-family="monospace">${letters}</text>
      <text x="${ix}" y="${iy + 10}" text-anchor="middle" fill="#94a3b8" font-size="8" font-weight="bold" font-family="monospace">${loopNum}</text>

      <rect x="${ix - 36}" y="${iy - 34}" width="72" height="16" rx="3" fill="#0b0f17" stroke="#334155" stroke-width="0.8"/>
      <text id="${safeInstId}" x="${ix}" y="${iy - 23}" text-anchor="middle" fill="${valColor}" font-family="monospace" font-size="9" font-weight="bold">${valStr} ${inst.unit}</text>

      <text x="${ix}" y="${iy + 40}" text-anchor="middle" font-family="monospace" font-size="8" fill="#94a3b8">${validityFlag} ${inst.tag}</text>
      ${alarmBadgeSvg}
    `;

    g.addEventListener("click", () => {
      store.openFaceplate({
        kind: "instrument",
        id: inst.tag,
        name: inst.tag,
        description: `${inst.measurement_kind} — ${inst.tag}`,
        pvTag: inst.tag,
        unit: inst.unit,
        rangeMin: inst.range_min,
        rangeMax: inst.range_max,
      });
      store.setFocusedEntity(inst.tag);
    });

    container.appendChild(g);
  }

  private renderControlLoop(
    container: Element,
    cl: StudioControlLoopDto,
    s: Store,
    proj?: ProjectDetailDto
  ) {
    let cx = cl.position?.x ?? 680;
    let cy = cl.position?.y ?? 200;

    const parentEqId = (cl as any).equipment_id || (proj ? proj.instruments.find((i) => i.tag === cl.pv_tag)?.equipment_id : undefined);
    if (parentEqId && proj) {
      const parentEq = proj.equipment.find((e) => e.id === parentEqId);
      if (parentEq) {
        const coords = this.getEquipmentCoordinates(parentEq);
        const kind = equipmentKind(parentEq, parentEq.definition_id);
        if (kind === "furnace" && cl.name === "TIC-101") {
          cx = coords.x + 120;
          cy = coords.y - 70;
        } else if (cl.position && parentEq.position) {
          const dx = cl.position.x - parentEq.position.x;
          const dy = cl.position.y - parentEq.position.y;
          cx = coords.x + dx;
          cy = coords.y + dy;
        } else {
          const eqLoops = (proj.control_loops || []).filter((l) => {
            const lEqId = (l as any).equipment_id || proj.instruments.find((i) => i.tag === l.pv_tag)?.equipment_id;
            return lEqId === parentEq.id;
          });
          const loopIdx = Math.max(0, eqLoops.findIndex((l) => l.name === cl.name));
          const haloR = kind === "column" ? 72 : kind === "reactor" ? 64 : kind === "vessel" || kind === "separator" ? 54 : 44;
          cx = coords.x + 52 + (loopIdx * 52);
          cy = coords.y - haloR - 22;
        }
      }
    }
    const pvVal = s.getTagFloat(cl.pv_tag, cl.sp);
    const isFocused = s.focusedEntityId === cl.name;

    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");
    g.setAttribute("class", "equipment-node clickable");
    g.setAttribute("cursor", "pointer");
    g.setAttribute("data-loop-name", cl.name);

    g.innerHTML = `
      <circle class="focus-ring" cx="${cx}" cy="${cy}" r="26"
        fill="none"
        stroke="${isFocused ? "#00e5ff" : "transparent"}"
        stroke-width="2"
        ${isFocused ? 'filter="url(#glow-cyan)"' : ""}/>
      <circle cx="${cx}" cy="${cy}" r="18" fill="#0f172a" stroke="#0ea5e9" stroke-width="2" class="inst-circle"/>
      <line x1="${cx - 18}" y1="${cy}" x2="${cx + 18}" y2="${cy}" stroke="#0ea5e9" stroke-width="1.2"/>
      <text x="${cx}" y="${cy - 4}" text-anchor="middle" fill="#38bdf8" font-size="8" font-weight="bold">${cl.name}</text>
      <text x="${cx}" y="${cy + 11}" text-anchor="middle" fill="#64748b" font-size="7.5" font-weight="bold">CTRL</text>

      <rect x="${cx - 28}" y="${cy + 22}" width="56" height="15" rx="2" fill="#0b0f17" stroke="#334155" stroke-width="0.8"/>
      <text x="${cx}" y="${cy + 33}" text-anchor="middle" fill="#38bdf8" font-family="monospace" font-size="9" font-weight="bold">${pvVal.toFixed(1)}</text>
    `;

    g.addEventListener("click", () => {
      store.setSelectedLoop(cl.out_tag);
      store.openFaceplate({
        kind: "control_loop",
        id: cl.name,
        name: cl.name,
        description: "PROCESS CONTROLLER",
        pvTag: cl.pv_tag,
        spTag: `${cl.name}.SP`,
        outTag: cl.out_tag,
        mode: (cl.mode as any) || "AUTO",
        sp: cl.sp,
        unit: "%",
      });
      store.setFocusedEntity(cl.name);
    });

    container.appendChild(g);
  }

  private renderBoundaryConditions(
    container: Element,
    bcs: BoundaryConditionDto[],
    equipment: StudioEquipmentDto[],
    _s: Store
  ) {
    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");
    g.setAttribute("class", "boundary-conditions-layer");

    for (const bc of bcs) {
      if (!bc.enabled) continue;
      const targetEq = equipment.find(
        (e) => e.id === bc.target_equipment_id || e.name === bc.target_equipment_id
      );
      if (!targetEq) continue;

      const pIn = this.getPortCoordinates(targetEq, bc.target_port || "inlet");
      const boxW = 150;
      const boxH = 44;
      const boxX = Math.max(15, pIn.x - boxW - 35);
      const boxY = pIn.y - Math.round(boxH / 2);

      const isFeed = bc.condition_type === "feed";
      const borderColor = isFeed ? "#10b981" : "#f59e0b";
      const titleColor = isFeed ? "#34d399" : "#fbbf24";
      const typeLabel = isFeed ? "PLANT FEED" : "UTILITY SUPPLY";

      const bcGroup = document.createElementNS("http://www.w3.org/2000/svg", "g");
      bcGroup.setAttribute("class", "boundary-node clickable");
      bcGroup.setAttribute("cursor", "pointer");
      bcGroup.setAttribute("data-bc-id", bc.id);

      // Connecting pipe from boundary box to equipment port
      const pipe = document.createElementNS("http://www.w3.org/2000/svg", "line");
      pipe.setAttribute("x1", `${boxX + boxW}`);
      pipe.setAttribute("y1", `${pIn.y}`);
      pipe.setAttribute("x2", `${pIn.x}`);
      pipe.setAttribute("y2", `${pIn.y}`);
      pipe.setAttribute("stroke", borderColor);
      pipe.setAttribute("stroke-width", "3");
      pipe.setAttribute("stroke-dasharray", "4 3");
      bcGroup.appendChild(pipe);

      // Arrowhead entering port
      const arrow = document.createElementNS("http://www.w3.org/2000/svg", "polygon");
      arrow.setAttribute(
        "points",
        `${pIn.x - 8},${pIn.y - 4} ${pIn.x},${pIn.y} ${pIn.x - 8},${pIn.y + 4}`
      );
      arrow.setAttribute("fill", borderColor);
      bcGroup.appendChild(arrow);

      // Boundary box
      const rect = document.createElementNS("http://www.w3.org/2000/svg", "rect");
      rect.setAttribute("x", `${boxX}`);
      rect.setAttribute("y", `${boxY}`);
      rect.setAttribute("width", `${boxW}`);
      rect.setAttribute("height", `${boxH}`);
      rect.setAttribute("rx", "5");
      rect.setAttribute("fill", "#0b121e");
      rect.setAttribute("stroke", borderColor);
      rect.setAttribute("stroke-width", "1.5");
      bcGroup.appendChild(rect);

      // Header label
      const header = document.createElementNS("http://www.w3.org/2000/svg", "text");
      header.setAttribute("x", `${boxX + 8}`);
      header.setAttribute("y", `${boxY + 13}`);
      header.setAttribute("fill", titleColor);
      header.setAttribute("font-size", "8.5");
      header.setAttribute("font-weight", "bold");
      header.setAttribute("font-family", "monospace");
      header.textContent = `▶ ${typeLabel}`;
      bcGroup.appendChild(header);

      // Name label
      const nameTxt = document.createElementNS("http://www.w3.org/2000/svg", "text");
      nameTxt.setAttribute("x", `${boxX + 8}`);
      nameTxt.setAttribute("y", `${boxY + 25}`);
      nameTxt.setAttribute("fill", "#e2e8f0");
      nameTxt.setAttribute("font-size", "8");
      nameTxt.setAttribute("font-family", "monospace");
      nameTxt.setAttribute("font-weight", "600");
      nameTxt.textContent = bc.name.length > 22 ? bc.name.slice(0, 21) + "…" : bc.name;
      bcGroup.appendChild(nameTxt);

      // Flow rate & temperature readout
      const flowTxt = document.createElementNS("http://www.w3.org/2000/svg", "text");
      flowTxt.setAttribute("x", `${boxX + 8}`);
      flowTxt.setAttribute("y", `${boxY + 37}`);
      flowTxt.setAttribute("fill", "#94a3b8");
      flowTxt.setAttribute("font-size", "7.5");
      flowTxt.setAttribute("font-family", "monospace");
      flowTxt.textContent = `${bc.flow_value} ${bc.flow_unit} | ${bc.temperature_value}°C`;
      bcGroup.appendChild(flowTxt);

      bcGroup.addEventListener("click", (e) => {
        e.stopPropagation();
        store.setFocusedEntity(bc.id);
        store.openFaceplate({
          kind: "instrument",
          id: bc.id,
          name: bc.name,
          description: `BOUNDARY: ${bc.name} (${bc.condition_type})`,
          pvTag: `${bc.name}.FLOW`,
          unit: bc.flow_unit,
        });
      });

      g.appendChild(bcGroup);
    }

    container.appendChild(g);
  }
}

customElements.define("nebu-process-view", NebuProcessView);
