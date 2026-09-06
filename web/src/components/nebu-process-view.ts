import { store, Store } from "../state/store";

export class NebuProcessView extends HTMLElement {
  private unsubscribe?: () => void;

  // SVG Elements for high-speed dynamic attribute mutation
  private pumpImpeller!: SVGElement;
  private pumpStatusText!: SVGTextElement;
  private pipeDash1!: SVGPathElement;
  private pipeDash2!: SVGPathElement;
  private valveStem!: SVGLineElement;
  private valvePlug!: SVGRectElement;
  private valvePosText!: SVGTextElement;
  private vesselFluid!: SVGRectElement;
  private vesselLevelText!: SVGTextElement;
  private ti101Text!: SVGTextElement;
  private pi101Text!: SVGTextElement;
  private ft101Text!: SVGTextElement;

  connectedCallback() {
    this.innerHTML = `
      <div class="process-view-container">
        <div class="panel-header">
          <span class="panel-title">PROCESS FLOW & DYNAMIC TOPOLOGY</span>
          <span class="panel-subtitle text-dim mono">LOOP: K-101 → FV-101 → V-101</span>
        </div>

        <svg viewBox="0 0 800 380" class="process-svg" xmlns="http://www.w3.org/2000/svg">
          <defs>
            <linearGradient id="vesselGrad" x1="0%" y1="0%" x2="100%" y2="0%">
              <stop offset="0%" stop-color="#1e293b" />
              <stop offset="50%" stop-color="#334155" />
              <stop offset="100%" stop-color="#1e293b" />
            </linearGradient>
            <linearGradient id="fluidGrad" x1="0%" y1="0%" x2="100%" y2="0%">
              <stop offset="0%" stop-color="#0369a1" />
              <stop offset="50%" stop-color="#38bdf8" />
              <stop offset="100%" stop-color="#0284c7" />
            </linearGradient>
          </defs>

          <!-- Static Piping Network -->
          <!-- Inflow to Pump -->
          <path d="M 20 220 L 100 220" class="pipe-base" stroke="#475569" stroke-width="12" fill="none" />
          <!-- Pump discharge upward to valve line -->
          <path d="M 120 180 L 120 120 L 360 120" class="pipe-base" stroke="#475569" stroke-width="12" fill="none" />
          <!-- Valve discharge into Vessel top -->
          <path d="M 400 120 L 580 120 L 580 150" class="pipe-base" stroke="#475569" stroke-width="12" fill="none" />
          <!-- Vessel bottom drain -->
          <path d="M 580 310 L 580 360 L 760 360" class="pipe-base" stroke="#475569" stroke-width="12" fill="none" />

          <!-- Dynamic Flow Dashes (animated with flow rate) -->
          <path id="pipeDash1" d="M 120 180 L 120 120 L 360 120" class="pipe-flow" stroke="#38bdf8" stroke-width="4" stroke-dasharray="8 8" fill="none" />
          <path id="pipeDash2" d="M 400 120 L 580 120 L 580 150" class="pipe-flow" stroke="#38bdf8" stroke-width="4" stroke-dasharray="8 8" fill="none" />

          <!-- 1. Equipment: Centrifugal Pump K-101 -->
          <g class="equipment-node" transform="translate(100, 180)">
            <!-- Pump Casing -->
            <circle cx="20" cy="40" r="36" fill="#1e293b" stroke="#0ea5e9" stroke-width="3" />
            <!-- Tangential Discharge Nozzle -->
            <path d="M 20 4 L 20 0" stroke="#0ea5e9" stroke-width="8" />
            <!-- Impeller Group -->
            <g id="pumpImpeller" transform="translate(20, 40)">
              <line x1="-20" y1="0" x2="20" y2="0" stroke="#94a3b8" stroke-width="3" />
              <line x1="0" y1="-20" x2="0" y2="20" stroke="#94a3b8" stroke-width="3" />
              <circle cx="0" cy="0" r="8" fill="#38bdf8" />
            </g>
            <!-- Label -->
            <text x="20" y="95" text-anchor="middle" fill="#e2e8f0" font-weight="bold" font-size="12">PUMP K-101</text>
            <text id="pumpStatus" x="20" y="112" text-anchor="middle" fill="#10b981" font-size="11" font-family="monospace">RUNNING</text>
          </g>

          <!-- 2. Transmitters on Pump Discharge -->
          <!-- Temperature TI-101 -->
          <g transform="translate(190, 70)">
            <line x1="20" y1="50" x2="20" y2="28" stroke="#64748b" stroke-width="2" />
            <circle cx="20" cy="14" r="14" fill="#0f172a" stroke="#64748b" stroke-width="2" />
            <text x="20" y="18" text-anchor="middle" fill="#94a3b8" font-size="10" font-weight="bold">TI</text>
            <text id="ti101Val" x="20" y="-4" text-anchor="middle" fill="#38bdf8" font-family="monospace" font-size="11">185.4 °C</text>
          </g>

          <!-- Pressure PT-101 -->
          <g transform="translate(260, 70)">
            <line x1="20" y1="50" x2="20" y2="28" stroke="#64748b" stroke-width="2" />
            <circle cx="20" cy="14" r="14" fill="#0f172a" stroke="#64748b" stroke-width="2" />
            <text x="20" y="18" text-anchor="middle" fill="#94a3b8" font-size="10" font-weight="bold">PT</text>
            <text id="pi101Val" x="20" y="-4" text-anchor="middle" fill="#38bdf8" font-family="monospace" font-size="11">32.5 bar</text>
          </g>

          <!-- 3. Control Valve FV-101 -->
          <g class="equipment-node clickable" id="valveGroup" transform="translate(360, 95)" cursor="pointer">
            <!-- Actuator Diaphragm Bonnet -->
            <path d="M 10 5 C 10 -15, 30 -15, 30 5 Z" fill="#334155" stroke="#f59e0b" stroke-width="2" />
            <line x1="20" y1="5" x2="20" y2="25" id="valveStem" stroke="#cbd5e1" stroke-width="3" />
            <!-- Valve Plug Indicator -->
            <rect id="valvePlug" x="16" y="20" width="8" height="6" fill="#f59e0b" />
            <!-- Opposed Triangles (Valve Body) -->
            <polygon points="0,15 20,25 0,35" fill="#1e293b" stroke="#f59e0b" stroke-width="2" />
            <polygon points="40,15 20,25 40,35" fill="#1e293b" stroke="#f59e0b" stroke-width="2" />
            <!-- Label -->
            <text x="20" y="52" text-anchor="middle" fill="#e2e8f0" font-weight="bold" font-size="11">FV-101</text>
            <text id="valvePosVal" x="20" y="66" text-anchor="middle" fill="#f59e0b" font-family="monospace" font-size="11">55.2 %</text>
          </g>

          <!-- Flow Transmitter FT-101 -->
          <g transform="translate(450, 70)">
            <line x1="20" y1="50" x2="20" y2="28" stroke="#64748b" stroke-width="2" />
            <circle cx="20" cy="14" r="14" fill="#0f172a" stroke="#64748b" stroke-width="2" />
            <text x="20" y="18" text-anchor="middle" fill="#94a3b8" font-size="10" font-weight="bold">FT</text>
            <text id="ft101Val" x="20" y="-4" text-anchor="middle" fill="#38bdf8" font-family="monospace" font-size="11">4500 kg/h</text>
          </g>

          <!-- 4. Accumulator Vessel V-101 -->
          <g class="equipment-node" transform="translate(520, 140)">
            <!-- Outer Vessel Body -->
            <rect x="0" y="20" width="120" height="150" rx="20" fill="url(#vesselGrad)" stroke="#0ea5e9" stroke-width="3" />
            
            <!-- Liquid Level Clip Region -->
            <g clip-path="url(#vesselClip)">
              <!-- Background Grid / Level Ticks -->
              <line x1="10" y1="50" x2="110" y2="50" stroke="#334155" stroke-dasharray="2 4" />
              <line x1="10" y1="95" x2="110" y2="95" stroke="#334155" stroke-dasharray="2 4" />
              <line x1="10" y1="140" x2="110" y2="140" stroke="#334155" stroke-dasharray="2 4" />

              <!-- Liquid Level Fill (Dynamic Height) -->
              <rect id="vesselFluid" x="2" y="95" width="116" height="75" fill="url(#fluidGrad)" opacity="0.85" />
            </g>

            <clipPath id="vesselClip">
              <rect x="2" y="22" width="116" height="146" rx="18" />
            </clipPath>

            <!-- Digital Readout inside vessel -->
            <text x="60" y="15" text-anchor="middle" fill="#e2e8f0" font-weight="bold" font-size="13">VESSEL V-101</text>
            <text id="vesselLevelVal" x="60" y="90" text-anchor="middle" fill="#ffffff" font-weight="bold" font-family="monospace" font-size="18">50.0 %</text>
          </g>

          <!-- 5. Level Transmitter LT-101 & Controller LIC-101 -->
          <g class="equipment-node clickable" id="licGroup" transform="translate(680, 200)" cursor="pointer">
            <line x1="-40" y1="20" x2="0" y2="20" stroke="#94a3b8" stroke-dasharray="3 3" stroke-width="2" />
            <!-- LT Bubble -->
            <circle cx="20" cy="20" r="18" fill="#0f172a" stroke="#0ea5e9" stroke-width="2" />
            <text x="20" y="24" text-anchor="middle" fill="#38bdf8" font-size="10" font-weight="bold">LIC-101</text>
            <text x="20" y="52" text-anchor="middle" fill="#94a3b8" font-size="10">LEVEL CTRL</text>
          </g>
        </svg>
      </div>
    `;

    // Query SVG elements
    this.pumpImpeller = this.querySelector("#pumpImpeller")!;
    this.pumpStatusText = this.querySelector("#pumpStatus")!;
    this.pipeDash1 = this.querySelector("#pipeDash1")!;
    this.pipeDash2 = this.querySelector("#pipeDash2")!;
    this.valveStem = this.querySelector("#valveStem")!;
    this.valvePlug = this.querySelector("#valvePlug")!;
    this.valvePosText = this.querySelector("#valvePosVal")!;
    this.vesselFluid = this.querySelector("#vesselFluid")!;
    this.vesselLevelText = this.querySelector("#vesselLevelVal")!;
    this.ti101Text = this.querySelector("#ti101Val")!;
    this.pi101Text = this.querySelector("#pi101Val")!;
    this.ft101Text = this.querySelector("#ft101Val")!;

    // Clicking valve or LIC selects the controller faceplate
    this.querySelector("#valveGroup")?.addEventListener("click", () => {
      store.setSelectedLoop("LIC-101.OUT");
    });
    this.querySelector("#licGroup")?.addEventListener("click", () => {
      store.setSelectedLoop("LIC-101.OUT");
    });

    this.unsubscribe = store.subscribe((s) => this.render(s));
  }

  disconnectedCallback() {
    if (this.unsubscribe) {
      this.unsubscribe();
    }
  }

  private render(s: Store) {
    // 1. Process Values
    const ti101 = s.getTagFloat("TI-101", 185.4);
    const pi101 = s.getTagFloat("PI-101", 32.5);
    const flow = s.getTagFloat("FIC-101.PV", 4500.0);
    const valveOut = s.getTagFloat("FIC-101.OUT", s.getTagFloat("LIC-101.OUT", 55.2));
    
    // Level: either LIC-101.PV or V-101.LEVEL or scaled flow
    const level = s.getTagFloat("LIC-101.PV", s.getTagFloat("V-101.LEVEL", 50.0));

    // Update text readouts
    this.ti101Text.textContent = `${ti101.toFixed(1)} °C`;
    this.pi101Text.textContent = `${pi101.toFixed(1)} bar`;
    this.ft101Text.textContent = `${flow.toFixed(0)} kg/h`;
    this.valvePosText.textContent = `${valveOut.toFixed(1)} %`;
    this.vesselLevelText.textContent = `${level.toFixed(1)} %`;

    // 2. Dynamic Fluid Fill Height in Vessel
    // Vessel height is 146px inside clip. Top is y=22, bottom is y=168.
    const clampedLevel = Math.max(0, Math.min(100, level));
    const fluidHeight = (clampedLevel / 100) * 146;
    const fluidY = 168 - fluidHeight;
    this.vesselFluid.setAttribute("y", fluidY.toString());
    this.vesselFluid.setAttribute("height", fluidHeight.toString());

    // 3. Dynamic Valve Stem Position
    // Stem travel: y=5 (fully closed) to y=15 (fully open)
    const plugY = 20 - (valveOut / 100) * 8;
    this.valvePlug.setAttribute("y", plugY.toString());
    this.valveStem.setAttribute("y2", plugY.toString());

    // 4. Pump Rotation and Flow Animation
    if (s.isSimRunning) {
      this.pumpStatusText.textContent = "RUNNING";
      this.pumpStatusText.setAttribute("fill", "var(--status-good)");
      const angle = (s.simTime * 720) % 360;
      this.pumpImpeller.setAttribute("transform", `translate(20, 40) rotate(${angle})`);

      // Animate flow dashes
      const dashOffset = (-s.simTime * 80) % 32;
      this.pipeDash1.style.strokeDashoffset = `${dashOffset}px`;
      this.pipeDash2.style.strokeDashoffset = `${dashOffset}px`;
    } else {
      this.pumpStatusText.textContent = "PAUSED";
      this.pumpStatusText.setAttribute("fill", "var(--status-warn)");
    }
  }
}

customElements.define("nebu-process-view", NebuProcessView);
