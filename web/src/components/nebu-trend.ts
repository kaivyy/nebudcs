import { store } from "../state/store";

interface PenConfig {
  tag: string;
  label: string;
  color: string;
  min: number;
  max: number;
}

export class NebuTrend extends HTMLElement {
  private canvas!: HTMLCanvasElement;
  private ctx!: CanvasRenderingContext2D;
  private legendEl!: HTMLElement;
  private tagSelect!: HTMLSelectElement;
  private animFrameId: number | null = null;

  private pens: PenConfig[] = [
    { tag: "LIC-101.PV", label: "PV (Level)", color: "#38bdf8", min: 0, max: 100 },
    { tag: "LIC-101.SP", label: "SP (Target)", color: "#facc15", min: 0, max: 100 },
    { tag: "LIC-101.OUT", label: "OUT (Valve)", color: "#f43f5e", min: 0, max: 100 },
  ];

  connectedCallback() {
    this.innerHTML = `
      <div class="trend-card">
        <div class="panel-header">
          <div class="title-group">
            <span class="panel-title">REALTIME PROCESS TREND</span>
            <span class="panel-subtitle text-dim mono">60s ROLLING WINDOW</span>
          </div>
          <div class="trend-controls">
            <select id="trendTagSelect">
              <option value="LIC-101">LIC-101 Group (PV, SP, OUT)</option>
              <option value="FIC-101">FIC-101 Group (PV, SP, OUT)</option>
              <option value="PI-101">PI-101 (Discharge Pressure)</option>
              <option value="TI-101">TI-101 (Temperature)</option>
            </select>
          </div>
        </div>

        <div class="canvas-wrapper">
          <canvas id="trendCanvas" width="700" height="200"></canvas>
        </div>

        <div class="trend-legend mono"></div>
      </div>
    `;

    this.canvas = this.querySelector("#trendCanvas")!;
    this.ctx = this.canvas.getContext("2d") as CanvasRenderingContext2D;
    this.legendEl = this.querySelector(".trend-legend")!;
    this.tagSelect = this.querySelector("#trendTagSelect")!;

    this.tagSelect.addEventListener("change", () => {
      this.updatePensFromSelection(this.tagSelect.value);
    });

    // Start render loop if canvas 2D context is available
    if (this.ctx) {
      this.startRenderLoop();
    }
  }

  disconnectedCallback() {
    if (this.animFrameId !== null) {
      cancelAnimationFrame(this.animFrameId);
      this.animFrameId = null;
    }
  }

  private updatePensFromSelection(sel: string) {
    if (sel === "LIC-101") {
      this.pens = [
        { tag: "LIC-101.PV", label: "PV (Level)", color: "#38bdf8", min: 0, max: 100 },
        { tag: "LIC-101.SP", label: "SP (Target)", color: "#facc15", min: 0, max: 100 },
        { tag: "LIC-101.OUT", label: "OUT (Valve)", color: "#f43f5e", min: 0, max: 100 },
      ];
    } else if (sel === "FIC-101") {
      this.pens = [
        { tag: "FIC-101.PV", label: "PV (Flow)", color: "#38bdf8", min: 0, max: 6000 },
        { tag: "FIC-101.SP", label: "SP (Target)", color: "#facc15", min: 0, max: 6000 },
        { tag: "FIC-101.OUT", label: "OUT (Demand)", color: "#f43f5e", min: 0, max: 100 },
      ];
    } else if (sel === "PI-101") {
      this.pens = [
        { tag: "PI-101", label: "PI-101 (Discharge Pressure)", color: "#38bdf8", min: 0, max: 50 },
      ];
    } else if (sel === "TI-101") {
      this.pens = [
        { tag: "TI-101", label: "TI-101 (Temperature)", color: "#f97316", min: 100, max: 250 },
      ];
    }
  }

  private startRenderLoop() {
    const render = () => {
      this.draw();
      this.animFrameId = requestAnimationFrame(render);
    };
    this.animFrameId = requestAnimationFrame(render);
  }

  private draw() {
    if (!this.ctx) return;
    const w = this.canvas.width;
    const h = this.canvas.height;
    const ctx = this.ctx;

    // Clear background
    ctx.fillStyle = "#0b0f14";
    ctx.fillRect(0, 0, w, h);

    // Padding for axes
    const padLeft = 45;
    const padRight = 15;
    const padTop = 15;
    const padBottom = 25;
    const plotW = w - padLeft - padRight;
    const plotH = h - padTop - padBottom;

    // Draw Grid Lines
    ctx.strokeStyle = "#1e293b";
    ctx.lineWidth = 1;
    ctx.beginPath();
    // 4 horizontal grid lines
    for (let i = 0; i <= 4; i++) {
      const y = padTop + (plotH / 4) * i;
      ctx.moveTo(padLeft, y);
      ctx.lineTo(padLeft + plotW, y);
    }
    // 5 vertical grid lines
    for (let i = 0; i <= 5; i++) {
      const x = padLeft + (plotW / 5) * i;
      ctx.moveTo(x, padTop);
      ctx.lineTo(x, padTop + plotH);
    }
    ctx.stroke();

    // Current sim time is the right edge
    const currentTime = store.simTime;
    const timeWindow = 60.0; // 60 seconds
    const startTime = Math.max(0, currentTime - timeWindow);

    // Draw Axis Labels
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

    // Draw Pens
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

      if (history.length < 2) continue;

      ctx.strokeStyle = pen.color;
      ctx.lineWidth = 2;
      ctx.beginPath();

      let isFirst = true;
      for (const pt of history) {
        if (pt.time < startTime) continue;

        const xFrac = (pt.time - startTime) / timeWindow;
        const clampedX = Math.max(0, Math.min(1, xFrac));
        const px = padLeft + clampedX * plotW;

        const yFrac = (pt.value - pen.min) / (pen.max - pen.min || 1);
        const clampedY = Math.max(0, Math.min(1, yFrac));
        const py = padTop + (1 - clampedY) * plotH;

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
  }
}

customElements.define("nebu-trend", NebuTrend);
