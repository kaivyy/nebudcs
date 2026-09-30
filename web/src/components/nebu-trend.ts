/**
 * NebuDCS Generic Trend Component.
 *
 * Pen groups and tag selections are derived dynamically from store.activeProject.
 * No instance-specific tag names (LIC-101, FIC-101, etc.) are hard-coded here.
 * Engineering ranges come from instrument/control-loop metadata in the project model.
 *
 * Pen group options:
 *   - One option per control loop: PV + SP + OUT pens, ranges from output_min/max
 *   - One option per instrument: single pen, ranges from range_min/range_max
 * When no project is loaded, the trend shows an empty state prompt.
 */

import { store } from "../state/store";
import { ProjectDetailDto } from "../protocol/types";

interface PenConfig {
  tag: string;
  label: string;
  color: string;
  min: number;
  max: number;
  alarmHiHi?: number;
  alarmHi?: number;
  alarmLo?: number;
  alarmLoLo?: number;
}

interface PenGroupOption {
  value: string;
  label: string;
  pens: PenConfig[];
}

const PEN_COLORS = ["#38bdf8", "#facc15", "#f43f5e", "#a78bfa", "#f97316", "#22d3ee"];

export class NebuTrend extends HTMLElement {
  private canvas!: HTMLCanvasElement;
  private ctx!: CanvasRenderingContext2D;
  private legendEl!: HTMLElement;
  private tagSelect!: HTMLSelectElement;
  private windowSelect!: HTMLSelectElement;
  private animFrameId: number | null = null;

  // Crosshair hairline state
  private hoverX: number | null = null;
  private isHovering: boolean = false;

  // Current pens — populated from project model, never hardcoded
  private pens: PenConfig[] = [];

  // Available pen groups derived from project model
  private penGroups: PenGroupOption[] = [];

  // Track last project to detect changes
  private lastProject: ProjectDetailDto | null = null;

  // Unsubscribe handle
  private storeUnsub?: () => void;

  connectedCallback() {
    this.innerHTML = `
      <div class="trend-card">
        <div class="panel-header">
          <div class="title-group">
            <span class="panel-title">REALTIME PROCESS TREND &amp; HISTORIAN</span>
            <span class="panel-subtitle text-dim mono" id="trendWindowSub">60s ROLLING WINDOW</span>
          </div>
          <div class="trend-controls">
            <select id="trendWindowSelect" title="Select Trend Time Window">
              <option value="30">30s Window</option>
              <option value="60" selected>60s Window</option>
              <option value="120">2m Window</option>
              <option value="300">5m Window</option>
              <option value="900">15m Window</option>
              <option value="3600">1h Window</option>
              <option value="28800">8h Window</option>
              <option value="86400">24h Window</option>
            </select>
            <select id="trendTagSelect" title="Select Process Pen Group">
              <option value="">-- Load a project --</option>
            </select>
          </div>
        </div>

        <div class="hairline-inspector mono text-dim" id="hairlineInspector">
          <span>CURSOR: <strong id="hairlineTime">--</strong></span>
          <span id="hairlineValues">Hover over trend to inspect values</span>
        </div>

        <div class="canvas-wrapper" style="position: relative;">
          <canvas id="trendCanvas" width="700" height="200"></canvas>
        </div>

        <div class="trend-legend mono"></div>
      </div>
    `;

    this.canvas = this.querySelector("#trendCanvas")!;
    this.ctx = this.canvas.getContext("2d") as CanvasRenderingContext2D;
    this.legendEl = this.querySelector(".trend-legend")!;
    this.tagSelect = this.querySelector("#trendTagSelect")!;
    this.windowSelect = this.querySelector("#trendWindowSelect")!;

    this.tagSelect.addEventListener("change", () => {
      const sel = this.tagSelect.value;
      const group = this.penGroups.find((g) => g.value === sel);
      if (group) {
        this.pens = group.pens;
      }
    });

    this.windowSelect.addEventListener("change", () => {
      const sec = parseInt(this.windowSelect.value, 10) || 60;
      store.setTrendTimeWindow(sec);
      const sub = this.querySelector("#trendWindowSub");
      if (sub) {
        sub.textContent = `${sec}s ROLLING WINDOW`;
      }
    });

    // Crosshair mouse tracking
    this.canvas.addEventListener("mousemove", (e) => {
      const rect = this.canvas.getBoundingClientRect();
      const scaleX = this.canvas.width / rect.width;
      this.hoverX = (e.clientX - rect.left) * scaleX;
      this.isHovering = true;
    });

    this.canvas.addEventListener("mouseleave", () => {
      this.hoverX = null;
      this.isHovering = false;
      const timeSpan = this.querySelector("#hairlineTime");
      const valSpan = this.querySelector("#hairlineValues");
      if (timeSpan && valSpan) {
        timeSpan.textContent = "--";
        valSpan.textContent = "Hover over trend to inspect values";
      }
    });

    // Subscribe to store — rebuild pen options when project changes and sync active target
    let lastTargetKey = "";
    this.storeUnsub = store.subscribe((s) => {
      if (s.activeProject !== this.lastProject) {
        this.lastProject = s.activeProject;
        this.rebuildPenOptions(s.activeProject);
      }
      const currentTarget = s.selectedFaceplate;
      const targetKey = currentTarget ? `${currentTarget.id}:${currentTarget.pvTag || ""}` : (s.selectedEquipmentId || "");
      if (targetKey && targetKey !== lastTargetKey) {
        lastTargetKey = targetKey;
        this.syncPenGroupWithTarget(currentTarget?.id || s.selectedEquipmentId || "", currentTarget?.pvTag);
      }
      if (this.ctx) {
        this.draw();
      }
    });

    if (this.ctx) {
      this.startRenderLoop();
    }
  }

  private syncPenGroupWithTarget(targetId: string, pvTag?: string) {
    if (!this.penGroups || this.penGroups.length === 0) return;
    const match = this.penGroups.find(
      (g) =>
        g.value.toLowerCase() === targetId.toLowerCase() ||
        (pvTag && g.pens.some((p) => p.tag.toLowerCase() === pvTag.toLowerCase())) ||
        g.pens.some((p) => p.tag.toLowerCase().includes(targetId.toLowerCase()))
    );
    if (match && this.tagSelect && this.tagSelect.value !== match.value) {
      this.tagSelect.value = match.value;
      this.pens = match.pens;
    }
  }

  disconnectedCallback() {
    if (this.animFrameId !== null) {
      cancelAnimationFrame(this.animFrameId);
      this.animFrameId = null;
    }
    if (this.storeUnsub) {
      this.storeUnsub();
    }
  }

  // ---------------------------------------------------------------------------
  // Build pen group options from active project model (no hardcoded IDs)
  // ---------------------------------------------------------------------------
  private rebuildPenOptions(proj: ProjectDetailDto | null) {
    this.penGroups = [];

    if (!proj) {
      this.tagSelect.innerHTML = '<option value="">-- Load a project --</option>';
      this.pens = [];
      return;
    }

    // One group per control loop: PV + SP + OUT
    for (const cl of proj.control_loops) {
      const rangeMin = cl.output_min ?? 0;
      const rangeMax = cl.output_max ?? 100;

      const hihiAlm = proj.alarms.find(
        (a) => a.pv_tag === cl.pv_tag && (a.alarm_type?.toUpperCase().includes("HIHI") || a.name?.toUpperCase().includes("HIHI"))
      );
      const hiAlm = proj.alarms.find(
        (a) => a.pv_tag === cl.pv_tag && ((a.alarm_type?.toUpperCase().includes("HI") && !a.alarm_type?.toUpperCase().includes("HIHI")) || (a.name?.toUpperCase().includes("HIGH") && !a.name?.toUpperCase().includes("HIHI")))
      );
      const loAlm = proj.alarms.find(
        (a) => a.pv_tag === cl.pv_tag && ((a.alarm_type?.toUpperCase().includes("LO") && !a.alarm_type?.toUpperCase().includes("LOLO")) || (a.name?.toUpperCase().includes("LOW") && !a.name?.toUpperCase().includes("LOLO")))
      );
      const loloAlm = proj.alarms.find(
        (a) => a.pv_tag === cl.pv_tag && (a.alarm_type?.toUpperCase().includes("LOLO") || a.name?.toUpperCase().includes("LOLO"))
      );

      const span = rangeMax - rangeMin || 100;
      const alarmHiHi = hihiAlm?.trip_limit ?? (rangeMin + 0.9 * span);
      const alarmHi = hiAlm?.trip_limit ?? (rangeMin + 0.8 * span);
      const alarmLo = loAlm?.trip_limit ?? (rangeMin + 0.2 * span);
      const alarmLoLo = loloAlm?.trip_limit ?? (rangeMin + 0.1 * span);

      this.penGroups.push({
        value: cl.name,
        label: `${cl.name} (PV / SP / OUT)`,
        pens: [
          {
            tag: cl.pv_tag,
            label: "PV",
            color: PEN_COLORS[0],
            min: rangeMin,
            max: rangeMax,
            alarmHiHi,
            alarmHi,
            alarmLo,
            alarmLoLo,
          },
          { tag: `${cl.name}.SP`, label: "SP", color: PEN_COLORS[1], min: rangeMin, max: rangeMax },
          { tag: cl.out_tag, label: "OUT", color: PEN_COLORS[2], min: 0, max: 100 },
        ],
      });
    }

    // One group per standalone instrument
    for (const inst of proj.instruments) {
      const rangeMin = inst.range_min;
      const rangeMax = inst.range_max;
      const span = rangeMax - rangeMin || 100;

      const hihiAlm = proj.alarms.find(
        (a) => a.pv_tag === inst.tag && (a.alarm_type?.toUpperCase().includes("HIHI") || a.name?.toUpperCase().includes("HIHI"))
      );
      const hiAlm = proj.alarms.find(
        (a) => a.pv_tag === inst.tag && ((a.alarm_type?.toUpperCase().includes("HI") && !a.alarm_type?.toUpperCase().includes("HIHI")) || (a.name?.toUpperCase().includes("HIGH") && !a.name?.toUpperCase().includes("HIHI")))
      );
      const loAlm = proj.alarms.find(
        (a) => a.pv_tag === inst.tag && ((a.alarm_type?.toUpperCase().includes("LO") && !a.alarm_type?.toUpperCase().includes("LOLO")) || (a.name?.toUpperCase().includes("LOW") && !a.name?.toUpperCase().includes("LOLO")))
      );
      const loloAlm = proj.alarms.find(
        (a) => a.pv_tag === inst.tag && (a.alarm_type?.toUpperCase().includes("LOLO") || a.name?.toUpperCase().includes("LOLO"))
      );

      const alarmHiHi = hihiAlm?.trip_limit ?? (rangeMin + 0.9 * span);
      const alarmHi = hiAlm?.trip_limit ?? (rangeMin + 0.8 * span);
      const alarmLo = loAlm?.trip_limit ?? (rangeMin + 0.2 * span);
      const alarmLoLo = loloAlm?.trip_limit ?? (rangeMin + 0.1 * span);

      this.penGroups.push({
        value: inst.tag,
        label: `${inst.tag} (${inst.measurement_kind})`,
        pens: [
          {
            tag: inst.tag,
            label: `${inst.tag}`,
            color: PEN_COLORS[3],
            min: inst.range_min,
            max: inst.range_max,
            alarmHiHi,
            alarmHi,
            alarmLo,
            alarmLoLo,
          },
        ],
      });
    }

    // Rebuild <select> options
    if (this.penGroups.length === 0) {
      this.tagSelect.innerHTML = '<option value="">-- No loops or instruments defined --</option>';
      this.pens = [];
    } else {
      this.tagSelect.innerHTML = this.penGroups
        .map((g) => `<option value="${g.value}">${g.label}</option>`)
        .join("");
      // Default to first group
      this.pens = this.penGroups[0].pens;
      this.tagSelect.value = this.penGroups[0].value;
    }
  }

  private startRenderLoop() {
    const render = () => {
      this.draw();
      this.animFrameId = requestAnimationFrame(render);
    };
    this.animFrameId = requestAnimationFrame(render);
  }

  // ---------------------------------------------------------------------------
  // Draw — operates on this.pens[], which are dynamically configured
  // ---------------------------------------------------------------------------
  private draw() {
    if (!this.ctx) return;
    const w = this.canvas.width;
    const h = this.canvas.height;
    const ctx = this.ctx;

    // Clear background
    ctx.fillStyle = "#0b0f14";
    ctx.fillRect(0, 0, w, h);

    if (this.pens.length === 0) {
      // No project / no pens — show empty state
      ctx.fillStyle = "#334155";
      ctx.font = "13px monospace";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText("Load a project to display trend data", w / 2, h / 2);
      return;
    }

    // Padding for axes
    const padLeft = 50;
    const padRight = 75;
    const padTop = 15;
    const padBottom = 25;
    const plotW = w - padLeft - padRight;
    const plotH = h - padTop - padBottom;

    // Grid
    ctx.strokeStyle = "#1e293b";
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let i = 0; i <= 4; i++) {
      const y = padTop + (plotH / 4) * i;
      ctx.moveTo(padLeft, y);
      ctx.lineTo(padLeft + plotW, y);
    }
    for (let i = 0; i <= 5; i++) {
      const x = padLeft + (plotW / 5) * i;
      ctx.moveTo(x, padTop);
      ctx.lineTo(x, padTop + plotH);
    }
    ctx.stroke();

    // Time window
    const currentTime = store.simTime;
    const timeWindow = store.trendTimeWindowSec || 60.0;
    const startTime = Math.max(0, currentTime - timeWindow);

    // Axis labels
    ctx.fillStyle = "#64748b";
    ctx.font = "10px monospace";
    ctx.textAlign = "right";
    ctx.textBaseline = "middle";

    const primaryPen = this.pens[0];
    const minVal = primaryPen ? primaryPen.min : 0;
    const maxVal = primaryPen ? primaryPen.max : 100;

    ctx.fillText(`${maxVal.toFixed(0)}`, padLeft - 6, padTop);
    ctx.fillText(`${((minVal + maxVal) / 2).toFixed(0)}`, padLeft - 6, padTop + plotH / 2);
    ctx.fillText(`${minVal.toFixed(0)}`, padLeft - 6, padTop + plotH);

    ctx.textAlign = "center";
    ctx.textBaseline = "top";
    ctx.fillText(`-${timeWindow}s`, padLeft, padTop + plotH + 6);
    ctx.fillText(`Now (${currentTime.toFixed(1)}s)`, padLeft + plotW, padTop + plotH + 6);

    // Draw Alarm Limit Lines (HIHI, HI, LO, LOLO) with Right-Side Labels
    if (primaryPen) {
      const limits: Array<{ val?: number; label: string; color: string }> = [
        { val: primaryPen.alarmHiHi, label: "HIHI", color: "#ef4444" },
        { val: primaryPen.alarmHi, label: "HI", color: "#f59e0b" },
        { val: primaryPen.alarmLo, label: "LO", color: "#f59e0b" },
        { val: primaryPen.alarmLoLo, label: "LOLO", color: "#ef4444" },
      ];

      for (const lim of limits) {
        if (lim.val !== undefined) {
          const frac = (lim.val - minVal) / (maxVal - minVal || 1);
          if (frac >= 0 && frac <= 1) {
            const limY = padTop + (1 - frac) * plotH;
            ctx.strokeStyle = lim.color;
            ctx.lineWidth = 1.2;
            ctx.setLineDash([4, 4]);
            ctx.beginPath();
            ctx.moveTo(padLeft, limY);
            ctx.lineTo(padLeft + plotW, limY);
            ctx.stroke();
            ctx.setLineDash([]);

            // Right-side engineering label
            ctx.fillStyle = lim.color;
            ctx.font = "9px monospace";
            ctx.textAlign = "left";
            ctx.textBaseline = "middle";
            ctx.fillText(`${lim.label}: ${lim.val.toFixed(1)}`, padLeft + plotW + 4, limY);
          }
        }
      }
    }

    // Historical Event / Disturbance / Trip Markers
    const chain = store.firstOutEvent?.disturbance_chain || [];
    for (const ev of chain) {
      if (ev.sim_time_sec >= startTime && ev.sim_time_sec <= currentTime) {
        const xFrac = (ev.sim_time_sec - startTime) / timeWindow;
        const ex = padLeft + xFrac * plotW;
        const isTrip = ev.event_type.includes("trip") || ev.event_type.includes("sif");
        ctx.strokeStyle = isTrip ? "rgba(239, 68, 68, 0.7)" : "rgba(245, 158, 11, 0.7)";
        ctx.lineWidth = 1.5;
        ctx.setLineDash([2, 3]);
        ctx.beginPath();
        ctx.moveTo(ex, padTop);
        ctx.lineTo(ex, padTop + plotH);
        ctx.stroke();
        ctx.setLineDash([]);
        ctx.fillStyle = isTrip ? "#ef4444" : "#f59e0b";
        ctx.font = "8px monospace";
        ctx.textAlign = "center";
        ctx.fillText(`▲ ${ev.source_id}`, ex, padTop + 8);
      }
    }

    // Hairline
    let hairlineTargetTime: number | null = null;
    if (this.isHovering && this.hoverX !== null && this.hoverX >= padLeft && this.hoverX <= padLeft + plotW) {
      const frac = (this.hoverX - padLeft) / plotW;
      hairlineTargetTime = startTime + frac * timeWindow;
    }

    let hairlineInspectorValues: string[] = [];

    // Pens
    let legendHtml = "";
    for (const pen of this.pens) {
      const history = store.getTrendData(pen.tag);
      const currentVal = store.getTagFloat(pen.tag, 0);

      legendHtml += `
        <div class="legend-item" style="color: ${pen.color};">
          <span class="legend-dot" style="background: ${pen.color};"></span>
          <span>${pen.label}:</span>
          <strong>${currentVal.toFixed(1)}</strong>
        </div>
      `;

      if (hairlineTargetTime !== null && history.length > 0) {
        let nearestVal = currentVal;
        let minDiff = Infinity;
        for (const pt of history) {
          const diff = Math.abs(pt.time - hairlineTargetTime);
          if (diff < minDiff) {
            minDiff = diff;
            nearestVal = pt.value;
          }
        }
        hairlineInspectorValues.push(
          `<span style="color: ${pen.color}">${pen.label}: ${nearestVal.toFixed(1)}</span>`
        );
      }

      if (history.length < 2) continue;

      ctx.strokeStyle = pen.color;
      ctx.lineWidth = 2;
      ctx.beginPath();

      let isFirst = true;
      for (const pt of history) {
        if (pt.time < startTime) continue;
        const xFrac = (pt.time - startTime) / timeWindow;
        const px = padLeft + Math.max(0, Math.min(1, xFrac)) * plotW;
        const yFrac = (pt.value - pen.min) / (pen.max - pen.min || 1);
        const py = padTop + (1 - Math.max(0, Math.min(1, yFrac))) * plotH;

        if (isFirst) {
          ctx.moveTo(px, py);
          isFirst = false;
        } else {
          ctx.lineTo(px, py);
        }
      }
      ctx.stroke();
    }

    this.legendEl.innerHTML = legendHtml;

    // Hairline cursor
    if (this.isHovering && this.hoverX !== null && this.hoverX >= padLeft && this.hoverX <= padLeft + plotW) {
      ctx.strokeStyle = "#00e5ff";
      ctx.lineWidth = 1;
      ctx.setLineDash([3, 3]);
      ctx.beginPath();
      ctx.moveTo(this.hoverX, padTop);
      ctx.lineTo(this.hoverX, padTop + plotH);
      ctx.stroke();
      ctx.setLineDash([]);

      const timeSpan = this.querySelector("#hairlineTime");
      const valSpan = this.querySelector("#hairlineValues");
      if (timeSpan && valSpan && hairlineTargetTime !== null) {
        timeSpan.textContent = `T = ${hairlineTargetTime.toFixed(1)}s`;
        valSpan.innerHTML = hairlineInspectorValues.join("  |  ");
      }
    }
  }
}

customElements.define("nebu-trend", NebuTrend);
