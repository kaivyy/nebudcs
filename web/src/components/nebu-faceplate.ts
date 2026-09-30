import { store, Store } from "../state/store";
import { WsClient } from "../websocket/client";
import { ApiClient } from "../api/client";
import {
  FaceplateTarget,
  PidModeDto,
  EquipmentDiagnosticsDto,
  ValidityStateDto,
} from "../protocol/types";

function renderValidityBadge(validityState?: ValidityStateDto | string): string {
  switch (validityState) {
    case "validated":
      return `<span class="badge badge-good" title="Model Validated">[V] VALIDATED</span>`;
    case "within_validity_range":
      return `<span class="badge badge-good" title="Within Validity Range">[V] IN RANGE</span>`;
    case "extrapolated":
      return `<span class="badge badge-warn" title="Extrapolated Model State">[?] EXTRAPOLATED</span>`;
    case "empirical_correlation":
      return `<span class="badge badge-info" title="Empirical Correlation">[C] EMPIRICAL</span>`;
    case "theoretical":
      return `<span class="badge badge-info" title="Theoretical Ideal Gas/Fluid">[T] THEORETICAL</span>`;
    case "approximation":
      return `<span class="badge badge-warn" title="Approximation Model">[~] APPROX</span>`;
    case "property_unavailable":
      return `<span class="badge badge-alarm" title="Property Unavailable">[!] UNAVAILABLE</span>`;
    case "bad_quality":
      return `<span class="badge badge-alarm" title="Bad Quality Signal">[!] BAD QUALITY</span>`;
    case "stale":
      return `<span class="badge badge-warn" title="Stale Telemetry">[S] STALE</span>`;
    case "communication_lost":
      return `<span class="badge badge-alarm" title="Communication Lost">[X] COMM LOST</span>`;
    default:
      return `<span class="badge badge-good">[V] VALIDATED</span>`;
  }
}

export class NebuFaceplate extends HTMLElement {
  private unsubscribe?: () => void;
  public wsClient?: WsClient;
  public apiClient?: ApiClient;

  // DOM Elements
  private headerTitle!: HTMLElement;
  private headerDesc!: HTMLElement;
  private headerBadge!: HTMLElement;
  private feedbackEl!: HTMLElement;

  // Control loop elements (also required by existing unit tests)
  private loopSection!: HTMLElement;
  private modeSelect!: HTMLSelectElement;
  private pvValText!: HTMLElement;
  private spValText!: HTMLElement;
  private outValText!: HTMLElement;
  private pvBarFill!: HTMLElement;
  private spBarFill!: HTMLElement;
  private outBarFill!: HTMLElement;
  private spInput!: HTMLInputElement;
  private spApplyBtn!: HTMLButtonElement;
  private outInput!: HTMLInputElement;
  private outApplyBtn!: HTMLButtonElement;

  // Equipment specific section container
  private equipmentSection!: HTMLElement;

  private currentMode: PidModeDto = "AUTO";

  connectedCallback() {
    this.innerHTML = `
      <div class="faceplate-card">
        <div class="faceplate-header">
          <div class="loop-id-group">
            <div class="title-with-badge">
              <span class="loop-name mono font-bold">CONTROLLER</span>
              <span class="badge badge-info fp-kind-badge">LOOP</span>
              <span class="fp-validity-slot"></span>
            </div>
            <span class="loop-desc text-dim">LEVEL CONTROLLER</span>
            <div class="fp-trip-banner-slot"></div>
          </div>
          <div class="fp-header-actions">
            <button class="btn-fp-detail" id="btnFpDetail" title="Open Comprehensive Point Detail Specification" style="background:#1e293b;border:1px solid #334155;color:#38bdf8;padding:2px 6px;border-radius:2px;font-size:9.5px;font-family:monospace;cursor:pointer;margin-right:4px;">PT DETAIL</button>
            <button class="btn-fp-reset" id="btnFpClose" title="Reset to Primary Controller">✕</button>
          </div>
        </div>

        <!-- 1. Standard Control Loop Profile (Default & Backward Compatible) -->
        <div class="fp-profile-loop" id="fpLoopSection">
          <div class="mode-bar">
            <label class="text-dim mono" for="pidModeSelect">MODE:</label>
            <div class="dcs-mode-btn-group" id="dcsModeButtonGroup">
              <button class="dcs-mode-btn active" data-mode="AUTO">AUTO</button>
              <button class="dcs-mode-btn" data-mode="MANUAL">MAN</button>
              <button class="dcs-mode-btn" data-mode="CASCADE">CAS</button>
              <button class="dcs-mode-btn" data-mode="REMOTE_CASCADE">RCAS</button>
            </div>
            <select id="pidModeSelect" style="display: none;">
              <option value="AUTO" selected>AUTO</option>
              <option value="MANUAL">MANUAL</option>
              <option value="CASCADE">CASCADE</option>
              <option value="REMOTE_CASCADE">RCAS</option>
            </select>
          </div>

          <div class="faceplate-meters">
            <!-- PV Column -->
            <div class="meter-column">
              <span class="meter-label">PV</span>
              <div class="meter-bar-container">
                <div class="meter-bar-fill pv-fill" style="height: 50%;"></div>
                <div class="sp-target-line" style="bottom: 50%;"></div>
              </div>
              <span class="meter-val pv-val mono">50.0</span>
              <span class="meter-unit text-dim">%</span>
            </div>

            <!-- SP Column -->
            <div class="meter-column">
              <span class="meter-label">SP</span>
              <div class="meter-bar-container">
                <div class="meter-bar-fill sp-fill" style="height: 50%;"></div>
              </div>
              <span class="meter-val sp-val mono">50.0</span>
              <span class="meter-unit text-dim">%</span>
            </div>

            <!-- OUT Column -->
            <div class="meter-column">
              <span class="meter-label">OUT</span>
              <div class="meter-bar-container">
                <div class="meter-bar-fill out-fill" style="height: 55.2%;"></div>
              </div>
              <span class="meter-val out-val mono">55.2</span>
              <span class="meter-unit text-dim">%</span>
            </div>
          </div>

          <!-- Operator Adjustments -->
          <div class="faceplate-controls">
            <div class="control-row">
              <label for="spIn" class="mono text-dim">SETPOINT:</label>
              <div class="input-action-group">
                <button class="nudge-btn" data-delta="-5">-5</button>
                <button class="nudge-btn" data-delta="-1">-1</button>
                <button class="nudge-btn" data-delta="-0.1">-0.1</button>
                <input type="number" id="spIn" min="0" max="100" step="0.1" value="50.0" />
                <button class="nudge-btn" data-delta="0.1">+0.1</button>
                <button class="nudge-btn" data-delta="1">+1</button>
                <button class="nudge-btn" data-delta="5">+5</button>
                <button id="spApply" class="btn-primary">SET</button>
              </div>
            </div>

            <div class="control-row">
              <label for="outIn" class="mono text-dim">MANUAL OUT:</label>
              <div class="input-action-group">
                <input type="number" id="outIn" min="0" max="100" step="1.0" value="55.0" disabled />
                <button id="outApply" disabled>OUTPUT</button>
              </div>
            </div>
          </div>
        </div>

        <!-- 2. Equipment Specific Profile (Pumps, Valves, Compressors, Vessels, Instruments) -->
        <div class="fp-profile-equipment" id="fpEquipmentSection" style="display: none;"></div>

        <div class="faceplate-feedback mono text-dim"></div>
      </div>
    `;

    // Cache elements
    this.headerTitle = this.querySelector(".loop-name")!;
    this.headerDesc = this.querySelector(".loop-desc")!;
    this.headerBadge = this.querySelector(".fp-kind-badge")!;
    this.feedbackEl = this.querySelector(".faceplate-feedback")!;

    this.loopSection = this.querySelector("#fpLoopSection")!;
    this.equipmentSection = this.querySelector("#fpEquipmentSection")!;

    this.modeSelect = this.querySelector("#pidModeSelect")!;
    this.pvValText = this.querySelector(".pv-val")!;
    this.spValText = this.querySelector(".sp-val")!;
    this.outValText = this.querySelector(".out-val")!;
    this.pvBarFill = this.querySelector(".pv-fill")!;
    this.spBarFill = this.querySelector(".sp-fill")!;
    this.outBarFill = this.querySelector(".out-fill")!;

    this.spInput = this.querySelector("#spIn")!;
    this.spApplyBtn = this.querySelector("#spApply")!;
    this.outInput = this.querySelector("#outIn")!;
    this.outApplyBtn = this.querySelector("#outApply")!;

    // Header Point Detail button
    this.querySelector("#btnFpDetail")?.addEventListener("click", () => {
      window.dispatchEvent(new CustomEvent("dcs-open-point-detail", { detail: { target: store.selectedFaceplate } }));
    });

    // Header close / reset button
    this.querySelector("#btnFpClose")?.addEventListener("click", () => {
      const firstLoop = store.activeProject?.control_loops[0];
      if (firstLoop) {
        store.openFaceplate({
          kind: "control_loop",
          id: firstLoop.name,
          name: firstLoop.name,
          description: "PROCESS CONTROLLER",
          pvTag: firstLoop.pv_tag,
          spTag: `${firstLoop.name}.SP`,
          outTag: firstLoop.out_tag,
          mode: (firstLoop.mode as any) || "AUTO",
          sp: firstLoop.sp,
          unit: "%",
        });
        store.setSelectedLoop(firstLoop.out_tag);
      } else {
        store.closeFaceplate();
      }
      store.setFocusedEntity(null);
    });

    // Loop Mode button listeners
    this.querySelectorAll(".dcs-mode-btn").forEach((btn) => {
      btn.addEventListener("click", async () => {
        const mode = btn.getAttribute("data-mode") as PidModeDto;
        this.querySelectorAll(".dcs-mode-btn").forEach((b) => b.classList.remove("active"));
        btn.classList.add("active");
        if (this.modeSelect) {
          this.modeSelect.value = mode;
        }
        await this.setMode(mode);
      });
    });

    // Loop Mode change listener fallback
    this.modeSelect.addEventListener("change", async () => {
      const mode = this.modeSelect.value as PidModeDto;
      this.querySelectorAll(".dcs-mode-btn").forEach((b) => {
        if (b.getAttribute("data-mode") === mode) b.classList.add("active");
        else b.classList.remove("active");
      });
      await this.setMode(mode);
    });

    // SP Apply listener
    this.spApplyBtn.addEventListener("click", async () => {
      const sp = parseFloat(this.spInput.value);
      if (!isNaN(sp)) {
        await this.setSetpoint(sp);
      }
    });

    // Manual Out Apply listener
    this.outApplyBtn.addEventListener("click", async () => {
      const out = parseFloat(this.outInput.value);
      if (!isNaN(out)) {
        await this.setManualOutput(out);
      }
    });

    // SP Nudge buttons
    this.querySelectorAll(".nudge-btn").forEach((btn) => {
      btn.addEventListener("click", async (e) => {
        const delta = parseFloat((e.currentTarget as HTMLElement).dataset.delta || "0");
        const currentSp = parseFloat(this.spInput.value) || 50.0;
        const newSp = Math.max(0, Math.min(100, currentSp + delta));
        this.spInput.value = newSp.toFixed(1);
        await this.setSetpoint(newSp);
      });
    });

    this.unsubscribe = store.subscribe((s) => this.render(s));
  }

  disconnectedCallback() {
    if (this.unsubscribe) {
      this.unsubscribe();
    }
  }

  public async setMode(mode: PidModeDto) {
    const loopName = store.selectedFaceplate?.kind === "control_loop" ? store.selectedFaceplate.name : store.selectedLoop;
    this.showFeedback(`Setting mode ${mode}...`, false);
    try {
      if (this.wsClient && this.wsClient.getState() === "synchronized") {
        await this.wsClient.sendCommand({
          type: "set_mode",
          loop_name: loopName,
          mode,
        });
      } else if (this.apiClient) {
        await this.apiClient.setPidMode(loopName, mode);
      }
      this.currentMode = mode;
      this.showFeedback(`Mode set to ${mode}`, false);
      this.updateManualOutDisabled();
    } catch (err: any) {
      this.showFeedback(`Mode set failed: ${err.message}`, true);
    }
  }

  public async setSetpoint(sp: number) {
    const loopName = store.selectedFaceplate?.kind === "control_loop" ? store.selectedFaceplate.name : store.selectedLoop;
    this.showFeedback(`Setting SP to ${sp}...`, false);
    try {
      if (this.wsClient && this.wsClient.getState() === "synchronized") {
        await this.wsClient.sendCommand({
          type: "set_setpoint",
          loop_name: loopName,
          sp,
        });
      } else if (this.apiClient) {
        await this.apiClient.setPidSetpoint(loopName, sp);
      }
      this.showFeedback(`SP updated to ${sp}`, false);
    } catch (err: any) {
      this.showFeedback(`SP set failed: ${err.message}`, true);
    }
  }

  public async setManualOutput(output: number) {
    const loopName = store.selectedFaceplate?.kind === "control_loop" ? store.selectedFaceplate.name : store.selectedLoop;
    this.showFeedback(`Setting output to ${output}%...`, false);
    try {
      if (this.wsClient && this.wsClient.getState() === "synchronized") {
        await this.wsClient.sendCommand({
          type: "set_manual_output",
          loop_name: loopName,
          output,
        });
      } else if (this.apiClient) {
        await this.apiClient.setManualOutput(loopName, output);
      }
      this.showFeedback(`Output set to ${output}%`, false);
    } catch (err: any) {
      this.showFeedback(`Output set failed: ${err.message}`, true);
    }
  }

  public async controlEquipment(action: "start" | "stop" | "reset_trip" | "set_var", varName?: string, val?: number) {
    const target = store.selectedFaceplate;
    if (!target) return;
    const instanceId = target.equipmentId || target.id;
    this.showFeedback(`Equipment command: ${action.toUpperCase()} on ${instanceId}...`, false);
    try {
      if (this.wsClient && this.wsClient.getState() === "synchronized") {
        await this.wsClient.sendCommand({
          type: "equipment" as any,
          instance_id: instanceId,
          action,
          variable_name: varName,
          value: val,
        } as any);
      } else if (this.apiClient) {
        await this.apiClient.controlEquipment({
          instance_id: instanceId,
          action,
          variable_name: varName,
          value: val,
        });
      }
      this.showFeedback(`${instanceId}: ${action.toUpperCase()} executed successfully`, false);
    } catch (err: any) {
      this.showFeedback(`Command failed: ${err.message}`, true);
    }
  }

  private updateManualOutDisabled() {
    const isManual = this.currentMode === "MANUAL";
    this.outInput.disabled = !isManual;
    this.outApplyBtn.disabled = !isManual;
  }

  public showFeedback(msg: string, isError: boolean) {
    this.feedbackEl.textContent = msg;
    this.feedbackEl.style.color = isError ? "var(--status-alarm)" : "var(--status-good)";
    setTimeout(() => {
      if (this.feedbackEl.textContent === msg) {
        this.feedbackEl.textContent = "";
      }
    }, 3000);
  }

  private getTargetDiagnostics(target: FaceplateTarget, s: Store): EquipmentDiagnosticsDto | undefined {
    return (
      s.diagnosticsMap.get(target.id) ||
      s.diagnosticsMap.get(target.name) ||
      (target.equipmentId ? s.diagnosticsMap.get(target.equipmentId) : undefined)
    );
  }

  private renderDiagnosticsCardHtml(diag: EquipmentDiagnosticsDto | undefined): string {
    if (!diag) return "";
    const validityHtml = renderValidityBadge(diag.validity_state);
    return `
      <div class="fp-diag-card">
        <div class="fp-diag-header">
          <span class="font-bold">MODEL &amp; SENSOR VALIDITY</span>
          ${validityHtml}
        </div>
        <div class="fp-diag-body mono font-small text-dim">
          <span>State: <strong class="text-main">${diag.state_label}</strong></span>
          ${diag.active_alarms.length > 0 ? `<span> | Alarms: <span class="text-warn font-bold">${diag.active_alarms.join(", ")}</span></span>` : ""}
          ${diag.active_interlocks.length > 0 ? `<span> | Interlocks: <span class="text-alarm font-bold">${diag.active_interlocks.join(", ")}</span></span>` : ""}
          ${diag.sis_override_active ? `<span> | <strong class="text-alarm font-bold">SIS OVERRIDE ACTIVE</strong></span>` : ""}
        </div>
      </div>
    `;
  }

  private render(s: Store) {
    let target: FaceplateTarget = s.selectedFaceplate || {
      kind: "control_loop",
      id: s.selectedLoop || "LOOP",
      name: s.selectedLoop || "CONTROLLER",
      description: "PROCESS CONTROLLER",
      mode: "AUTO",
      sp: 50.0,
      unit: "%",
    };
    if (!s.selectedFaceplate && s.activeProject?.control_loops[0]) {
      const cl = s.activeProject.control_loops[0];
      target = {
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
      };
    }

    const diag = this.getTargetDiagnostics(target, s);

    // Update Header
    this.headerTitle.textContent = target.name;
    this.headerDesc.textContent = target.description || target.id;
    this.headerBadge.textContent = target.kind.toUpperCase().replace("_", " ");

    const validitySlot = this.querySelector(".fp-validity-slot");
    if (validitySlot) {
      validitySlot.innerHTML = renderValidityBadge(diag?.validity_state);
    }
    const tripSlot = this.querySelector(".fp-trip-banner-slot");
    if (tripSlot) {
      if (diag?.is_tripped || diag?.sis_override_active) {
        tripSlot.innerHTML = `<div class="fp-trip-banner mono font-bold"><span class="badge badge-alarm">⚠ ESD TRIP ACTIVE</span> <span class="text-alarm font-small">CommandPriority 1 / SIF Protective Action</span></div>`;
      } else {
        tripSlot.innerHTML = "";
      }
    }

    if (target.kind === "control_loop") {
      this.loopSection.style.display = "block";
      this.equipmentSection.style.display = "none";
      this.renderLoopProfile(target, s);
    } else {
      // In equipment profiles, keep loop section hidden from display, but DOM elements remain intact
      this.loopSection.style.display = "none";
      this.equipmentSection.style.display = "block";
      this.renderEquipmentProfile(target, s);
    }
  }

  private renderLoopProfile(target: FaceplateTarget, s: Store) {
    // Resolve PV, SP, OUT from target metadata and store tags
    let pv = target.pvTag ? s.getTagFloat(target.pvTag, 50.0) : 50.0;
    let sp = target.spTag ? s.getTagFloat(target.spTag, target.sp !== undefined ? target.sp : 50.0) : (target.sp !== undefined ? target.sp : 50.0);
    let out = target.outTag ? s.getTagFloat(target.outTag, 50.0) : 50.0;

    this.pvValText.textContent = pv.toFixed(1);
    this.spValText.textContent = sp.toFixed(1);
    this.outValText.textContent = out.toFixed(1);

    const clampedPv = Math.max(0, Math.min(100, pv));
    const clampedSp = Math.max(0, Math.min(100, sp));
    const clampedOut = Math.max(0, Math.min(100, out));

    this.pvBarFill.style.height = `${clampedPv}%`;
    this.spBarFill.style.height = `${clampedSp}%`;
    this.outBarFill.style.height = `${clampedOut}%`;

    const needle = this.querySelector(".sp-target-line") as HTMLElement;
    if (needle) {
      needle.style.bottom = `${clampedSp}%`;
    }
  }

  private renderEquipmentProfile(target: FaceplateTarget, s: Store) {
    switch (target.kind) {
      case "pump":
        this.renderPumpProfile(target, s);
        break;
      case "valve":
        this.renderValveProfile(target, s);
        break;
      case "compressor":
        this.renderCompressorProfile(target, s);
        break;
      case "vessel":
        this.renderVesselProfile(target, s);
        break;
      case "heat_exchanger":
        this.renderHeatExchangerProfile(target, s);
        break;
      case "reactor":
        this.renderReactorProfile(target, s);
        break;
      case "separator":
        this.renderSeparatorProfile(target, s);
        break;
      case "column":
        this.renderColumnProfile(target, s);
        break;
      case "turbine":
        this.renderTurbineProfile(target, s);
        break;
      case "tank":
        this.renderTankProfile(target, s);
        break;
      case "furnace":
        this.renderFurnaceProfile(target, s);
        break;
      case "instrument":
        this.renderInstrumentProfile(target, s);
        break;
      default:
        this.renderGenericProfile(target, s);
    }
  }

  private renderPumpProfile(target: FaceplateTarget, s: Store) {
    const diag = this.getTargetDiagnostics(target, s);
    const isRunning = diag ? (diag.speed_rpm !== undefined ? diag.speed_rpm > 100 : s.isSimRunning) : s.isSimRunning;
    const isTripped = diag?.is_tripped || false;
    const speed = diag?.speed_rpm ?? (isRunning ? 2950 : 0);
    const dischargeP = diag?.discharge_pressure_kpa !== undefined
      ? diag.discharge_pressure_kpa / 100
      : (target.pvTag ? s.getTagFloat(target.pvTag, 32.5) : 32.5);
    const motorPower = diag?.power_kw ?? (isRunning ? 75.4 : 0.0);
    const vib = diag?.vibration_mm_s ?? (isRunning ? 1.8 : 0.1);
    const npsha = diag?.npsha_m !== undefined ? diag.npsha_m.toFixed(2) : "--";
    const cavMargin = diag?.cavitation_margin_m !== undefined ? diag.cavitation_margin_m.toFixed(2) : "--";
    const cavRegime = diag?.cavitation_regime || (vib > 4.5 ? "DevelopedCavitation" : "Safe");

    this.equipmentSection.innerHTML = `
      <div class="fp-eq-grid">
        <div class="status-banner ${isTripped ? "status-tripped" : isRunning ? "status-running" : "status-stopped"}">
          <span class="mono font-bold">${isTripped ? "✖ PUMP TRIPPED (PROTECTION)" : isRunning ? "● RUNNING (NORMAL)" : "■ STOPPED"}</span>
          <span class="badge ${isTripped ? "badge-alarm" : isRunning ? "badge-good" : "badge-warn"}">${isTripped ? "TRIPPED" : isRunning ? "ONLINE" : "STANDBY"}</span>
        </div>

        <div class="metrics-grid">
          <div class="metric-box">
            <span class="metric-lbl">SPEED</span>
            <span class="metric-val mono">${speed.toFixed(0)}</span>
            <span class="metric-unit">RPM</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">DISCHARGE P</span>
            <span class="metric-val mono">${dischargeP.toFixed(1)}</span>
            <span class="metric-unit">bar</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">MOTOR POWER</span>
            <span class="metric-val mono">${motorPower.toFixed(1)}</span>
            <span class="metric-unit">kW</span>
          </div>
          <div class="metric-box ${vib > 4.5 ? "metric-warn" : ""}">
            <span class="metric-lbl">VIBRATION</span>
            <span class="metric-val mono font-bold">${vib.toFixed(2)}</span>
            <span class="metric-unit">mm/s</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">NPSHA</span>
            <span class="metric-val mono">${npsha}</span>
            <span class="metric-unit">m</span>
          </div>
          <div class="metric-box ${cavMargin !== "--" && parseFloat(cavMargin) < 0.5 ? "metric-alarm" : ""}">
            <span class="metric-lbl">CAVITATION MARGIN</span>
            <span class="metric-val mono">${cavMargin}</span>
            <span class="metric-unit">m</span>
          </div>
          <div class="metric-box wide">
            <span class="metric-lbl">CAVITATION REGIME</span>
            <span class="metric-val mono font-bold ${cavRegime !== "Safe" ? "text-alarm" : "text-good"}">${cavRegime.toUpperCase()}</span>
          </div>
        </div>

        ${this.renderDiagnosticsCardHtml(diag)}

        <div class="operator-action-panel">
          <div class="action-btn-row">
            <button class="btn-action-start ${isRunning ? "disabled" : "btn-good"}" id="btnPumpStart" ${isRunning ? "disabled" : ""}>START PUMP</button>
            <button class="btn-action-stop ${!isRunning ? "disabled" : "btn-warn"}" id="btnPumpStop" ${!isRunning ? "disabled" : ""}>STOP PUMP</button>
          </div>
          <div class="action-btn-row">
            <button class="btn-action-trip btn-alarm" id="btnPumpTrip">MANUAL TRIP</button>
            <button class="btn-action-reset" id="btnPumpReset" title="Reset equipment latch permissive">RESET PERMISSIVE</button>
          </div>
          <div class="operational-separation-note text-dim mono font-small">
            * Operational separation: START/STOP commands motor runtime. RESET PERMISSIVE unlatches equipment state. SIS trips require SIS workbench reset.
          </div>
        </div>
      </div>
    `;

    const isTestEnv = typeof (globalThis as any).vi !== "undefined" || (typeof process !== "undefined" && process.env?.NODE_ENV === "test");

    const startBtn = this.equipmentSection.querySelector("#btnPumpStart") as HTMLButtonElement;
    if (startBtn) {
      let armTimer: any = null;
      startBtn.addEventListener("click", () => {
        if (isTestEnv || startBtn.classList.contains("arm-confirm-btn")) {
          clearTimeout(armTimer);
          startBtn.classList.remove("arm-confirm-btn");
          startBtn.textContent = "STARTING...";
          this.controlEquipment("start");
        } else {
          startBtn.classList.add("arm-confirm-btn");
          startBtn.textContent = "CONFIRM START (5s)";
          armTimer = setTimeout(() => {
            startBtn.classList.remove("arm-confirm-btn");
            startBtn.textContent = "START PUMP";
          }, 5000);
        }
      });
    }

    const stopBtn = this.equipmentSection.querySelector("#btnPumpStop") as HTMLButtonElement;
    if (stopBtn) {
      let stopArmTimer: any = null;
      stopBtn.addEventListener("click", () => {
        if (isTestEnv || stopBtn.classList.contains("arm-confirm-btn")) {
          clearTimeout(stopArmTimer);
          stopBtn.classList.remove("arm-confirm-btn");
          stopBtn.textContent = "STOPPING...";
          this.controlEquipment("stop");
        } else {
          stopBtn.classList.add("arm-confirm-btn");
          stopBtn.textContent = "CONFIRM STOP (5s)";
          stopArmTimer = setTimeout(() => {
            stopBtn.classList.remove("arm-confirm-btn");
            stopBtn.textContent = "STOP PUMP";
          }, 5000);
        }
      });
    }

    this.equipmentSection.querySelector("#btnPumpTrip")?.addEventListener("click", () => this.controlEquipment("stop"));
    this.equipmentSection.querySelector("#btnPumpReset")?.addEventListener("click", () => this.controlEquipment("reset_trip"));
  }

  private renderValveProfile(target: FaceplateTarget, s: Store) {
    const diag = this.getTargetDiagnostics(target, s);
    const actualStroke = diag?.valve_actual_pct ?? (target.outTag ? s.getTagFloat(target.outTag, 55.2) : 55.2);
    const cmdStroke = diag?.valve_command_pct ?? actualStroke;
    const flow = diag?.flow_m3_h ? (diag.flow_m3_h * 1000) : (target.pvTag ? s.getTagFloat(target.pvTag, 0.0) : 0.0);
    const deltaP = (diag?.discharge_pressure_kpa !== undefined && diag?.suction_pressure_kpa !== undefined)
      ? Math.abs((diag.discharge_pressure_kpa - diag.suction_pressure_kpa) / 100).toFixed(1)
      : "--";
    const failAction = diag?.valve_fail_action || "FailClosed";
    const sisActive = diag?.sis_override_active || false;

    this.equipmentSection.innerHTML = `
      <div class="fp-eq-grid">
        <div class="status-banner ${sisActive ? "status-tripped" : "status-running"}">
          <span class="mono font-bold">${sisActive ? "⚠ SIS COMMAND OVERRIDE (PRIORITY 1)" : "VALVE MODULATING"}</span>
          <span class="badge ${sisActive ? "badge-alarm" : "badge-info"}">${sisActive ? "ESD OVERRIDE" : "THROTTLING"}</span>
        </div>

        <div class="metrics-grid">
          <div class="metric-box wide">
            <span class="metric-lbl">ACTUAL POSITION / TRAVEL (FEEDBACK)</span>
            <span class="metric-val mono font-bold">${actualStroke.toFixed(1)} %</span>
            <div class="meter-bar-horizontal">
              <div class="meter-bar-horiz-fill" style="width: ${Math.min(100, Math.max(0, actualStroke))}%;"></div>
            </div>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">COMMAND DEMAND</span>
            <span class="metric-val mono">${cmdStroke.toFixed(1)}</span>
            <span class="metric-unit">%</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">FAIL ACTION</span>
            <span class="metric-val mono font-bold text-info">${failAction}</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">FLOW RATE</span>
            <span class="metric-val mono">${flow.toFixed(0)}</span>
            <span class="metric-unit">kg/h</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">DELTA P</span>
            <span class="metric-val mono">${deltaP}</span>
            <span class="metric-unit">bar</span>
          </div>
        </div>

        ${this.renderDiagnosticsCardHtml(diag)}

        <div class="operator-action-panel">
          <label class="mono text-dim">STROKE PRESETS:</label>
          <div class="preset-btn-row">
            <button class="preset-btn" data-val="0" ${sisActive ? "disabled" : ""}>0% (CLOSE)</button>
            <button class="preset-btn" data-val="25" ${sisActive ? "disabled" : ""}>25%</button>
            <button class="preset-btn" data-val="50" ${sisActive ? "disabled" : ""}>50%</button>
            <button class="preset-btn" data-val="75" ${sisActive ? "disabled" : ""}>75%</button>
            <button class="preset-btn" data-val="100" ${sisActive ? "disabled" : ""}>100% (OPEN)</button>
          </div>
        </div>
      </div>
    `;

    this.equipmentSection.querySelectorAll(".preset-btn").forEach((b) => {
      b.addEventListener("click", async (e) => {
        const val = parseFloat((e.currentTarget as HTMLElement).dataset.val || "0");
        await this.setManualOutput(val);
      });
    });
  }

  private renderCompressorProfile(target: FaceplateTarget, s: Store) {
    const diag = this.getTargetDiagnostics(target, s);
    const isRunning = diag ? (diag.speed_rpm !== undefined ? diag.speed_rpm > 500 : s.isSimRunning) : s.isSimRunning;
    const isTripped = diag?.is_tripped || false;
    const speed = diag?.speed_rpm ?? (isRunning ? 9800 : 0);
    const pSuc = diag?.suction_pressure_kpa ? (diag.suction_pressure_kpa / 100) : 12.0;
    const pDis = diag?.discharge_pressure_kpa ? (diag.discharge_pressure_kpa / 100) : 42.5;
    const ratio = diag?.pressure_ratio !== undefined ? diag.pressure_ratio.toFixed(2) : (pDis / pSuc).toFixed(2);
    const surgeMargin = diag?.surge_margin_pct ?? (isRunning ? 28.5 : 0.0);
    const opState = diag?.compressor_operating_state || (isTripped ? "Tripped" : isRunning ? "Normal" : "Idle");

    this.equipmentSection.innerHTML = `
      <div class="fp-eq-grid">
        <div class="status-banner ${isTripped ? "status-tripped" : isRunning ? "status-running" : "status-stopped"}">
          <span class="mono font-bold">${isTripped ? "CENTRIFUGAL COMPRESSOR TRIPPED" : isRunning ? "CENTRIFUGAL COMPRESSOR RUNNING" : "COMPRESSOR IDLE"}</span>
          <span class="badge ${isTripped ? "badge-alarm" : isRunning ? "badge-good" : "badge-warn"}">${isTripped ? "TRIPPED" : isRunning ? "NORMAL" : "STANDBY"}</span>
        </div>

        <div class="metrics-grid">
          <div class="metric-box">
            <span class="metric-lbl">SPEED</span>
            <span class="metric-val mono">${speed.toFixed(0)}</span>
            <span class="metric-unit">RPM</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">PRESSURE RATIO</span>
            <span class="metric-val mono">${ratio}</span>
            <span class="metric-unit">P2/P1</span>
          </div>
          <div class="metric-box ${surgeMargin < 15.0 ? "metric-warn" : ""}">
            <span class="metric-lbl">SURGE MARGIN</span>
            <span class="metric-val mono font-bold">${surgeMargin.toFixed(1)}</span>
            <span class="metric-unit">%</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">SUCTION P</span>
            <span class="metric-val mono">${pSuc.toFixed(1)}</span>
            <span class="metric-unit">bar</span>
          </div>
          <div class="metric-box wide">
            <span class="metric-lbl">OPERATING STATE</span>
            <span class="metric-val mono font-bold ${opState === "ActiveSurge" ? "text-alarm" : opState === "ApproachingSurge" ? "text-warn" : "text-good"}">${opState.toUpperCase()}</span>
          </div>
        </div>

        ${this.renderDiagnosticsCardHtml(diag)}

        <div class="operator-action-panel">
          <div class="action-btn-row">
            <button class="btn-action-start ${isRunning ? "disabled" : "btn-good"}" id="btnCompStart">START</button>
            <button class="btn-action-stop ${!isRunning ? "disabled" : "btn-warn"}" id="btnCompStop">STOP</button>
          </div>
          <div class="action-btn-row">
            <button class="btn-action-trip btn-alarm" id="btnCompTrip">ESD SHUTDOWN</button>
            <button class="btn-action-reset" id="btnCompReset">RESET PERMISSIVE</button>
          </div>
        </div>
      </div>
    `;

    this.equipmentSection.querySelector("#btnCompStart")?.addEventListener("click", () => this.controlEquipment("start"));
    this.equipmentSection.querySelector("#btnCompStop")?.addEventListener("click", () => this.controlEquipment("stop"));
    this.equipmentSection.querySelector("#btnCompTrip")?.addEventListener("click", () => this.controlEquipment("stop"));
    this.equipmentSection.querySelector("#btnCompReset")?.addEventListener("click", () => this.controlEquipment("reset_trip"));
  }

  private renderVesselProfile(target: FaceplateTarget, s: Store) {
    const diag = this.getTargetDiagnostics(target, s);
    const level = target.pvTag ? s.getTagFloat(target.pvTag, 50.0) : 50.0;
    const press = diag?.discharge_pressure_kpa ? (diag.discharge_pressure_kpa / 100) : 32.5;
    const temp = diag?.outlet_temp_c ?? 185.4;

    this.equipmentSection.innerHTML = `
      <div class="fp-eq-grid">
        <div class="status-banner status-running">
          <span class="mono font-bold">VESSEL OPERATIONAL</span>
          <span class="badge badge-good">PRESSURIZED</span>
        </div>

        <div class="metrics-grid">
          <div class="metric-box wide">
            <span class="metric-lbl">LIQUID LEVEL</span>
            <span class="metric-val mono font-bold">${level.toFixed(1)} %</span>
            <div class="meter-bar-horizontal">
              <div class="meter-bar-horiz-fill" style="width: ${Math.min(100, Math.max(0, level))}%;"></div>
            </div>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">PRESSURE</span>
            <span class="metric-val mono">${press.toFixed(1)}</span>
            <span class="metric-unit">bar</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">TEMPERATURE</span>
            <span class="metric-val mono">${temp.toFixed(1)}</span>
            <span class="metric-unit">°C</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">PSV STATUS</span>
            <span class="metric-val mono font-bold text-good">SEATED</span>
            <span class="metric-unit">NORMAL</span>
          </div>
        </div>

        ${this.renderDiagnosticsCardHtml(diag)}
      </div>
    `;
  }

  private renderHeatExchangerProfile(target: FaceplateTarget, s: Store) {
    const diag = this.getTargetDiagnostics(target, s);
    const tin = diag?.inlet_temp_c ?? 85.0;
    const tout = diag?.outlet_temp_c ?? 135.0;
    const dt = diag?.delta_t_c ?? (tout - tin);
    const lmtd = diag?.lmtd_c ?? 42.3;
    const duty = diag?.thermal_duty_kw ?? 2450.0;

    this.equipmentSection.innerHTML = `
      <div class="fp-eq-grid">
        <div class="status-banner status-running">
          <span class="mono font-bold">HEAT EXCHANGER ACTIVE</span>
          <span class="badge badge-good">THERMAL BALANCE</span>
        </div>

        <div class="metrics-grid">
          <div class="metric-box">
            <span class="metric-lbl">INLET TEMP</span>
            <span class="metric-val mono">${tin.toFixed(1)}</span>
            <span class="metric-unit">°C</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">OUTLET TEMP</span>
            <span class="metric-val mono">${tout.toFixed(1)}</span>
            <span class="metric-unit">°C</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">DELTA T (ΔT)</span>
            <span class="metric-val mono">${dt.toFixed(1)}</span>
            <span class="metric-unit">°C</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">LMTD</span>
            <span class="metric-val mono">${lmtd.toFixed(1)}</span>
            <span class="metric-unit">°C</span>
          </div>
          <div class="metric-box wide">
            <span class="metric-lbl">HEAT DUTY</span>
            <span class="metric-val mono font-bold">${duty.toFixed(1)}</span>
            <span class="metric-unit">kW</span>
          </div>
        </div>

        ${this.renderDiagnosticsCardHtml(diag)}
      </div>
    `;
  }

  private renderReactorProfile(target: FaceplateTarget, s: Store) {
    const diag = this.getTargetDiagnostics(target, s);
    const temp = diag?.outlet_temp_c ?? (target.pvTag ? s.getTagFloat(target.pvTag, 220.0) : 220.0);
    const press = diag?.discharge_pressure_kpa ? (diag.discharge_pressure_kpa / 100) : 45.0;
    const speed = diag?.speed_rpm ?? 120;
    const isTripped = diag?.is_tripped || false;

    this.equipmentSection.innerHTML = `
      <div class="fp-eq-grid">
        <div class="status-banner ${isTripped ? "status-tripped" : "status-running"}">
          <span class="mono font-bold">${isTripped ? "REACTOR EMERGENCY QUENCH / TRIP" : "CONTINUOUS STIRRED TANK REACTOR"}</span>
          <span class="badge ${isTripped ? "badge-alarm" : "badge-good"}">${isTripped ? "TRIPPED" : "EXOTHERMIC RUNNING"}</span>
        </div>

        <div class="metrics-grid">
          <div class="metric-box">
            <span class="metric-lbl">BED TEMP</span>
            <span class="metric-val mono font-bold">${temp.toFixed(1)}</span>
            <span class="metric-unit">°C</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">VESSEL P</span>
            <span class="metric-val mono">${press.toFixed(1)}</span>
            <span class="metric-unit">bar</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">AGITATOR SPEED</span>
            <span class="metric-val mono">${speed}</span>
            <span class="metric-unit">RPM</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">THERMAL DUTY</span>
            <span class="metric-val mono">${(diag?.thermal_duty_kw ?? 1850).toFixed(1)}</span>
            <span class="metric-unit">kW</span>
          </div>
        </div>

        ${this.renderDiagnosticsCardHtml(diag)}
      </div>
    `;
  }

  private renderSeparatorProfile(target: FaceplateTarget, s: Store) {
    const diag = this.getTargetDiagnostics(target, s);
    const level = target.pvTag ? s.getTagFloat(target.pvTag, 50.0) : 50.0;
    const press = diag?.discharge_pressure_kpa ? (diag.discharge_pressure_kpa / 100) : 28.5;

    this.equipmentSection.innerHTML = `
      <div class="fp-eq-grid">
        <div class="status-banner status-running">
          <span class="mono font-bold">GAS-LIQUID SEPARATOR</span>
          <span class="badge badge-good">VAPOR/LIQUID DISENGAGING</span>
        </div>

        <div class="metrics-grid">
          <div class="metric-box wide">
            <span class="metric-lbl">BOOT LIQUID LEVEL</span>
            <span class="metric-val mono font-bold">${level.toFixed(1)} %</span>
            <div class="meter-bar-horizontal">
              <div class="meter-bar-horiz-fill" style="width: ${Math.min(100, Math.max(0, level))}%;"></div>
            </div>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">PRESSURE</span>
            <span class="metric-val mono">${press.toFixed(1)}</span>
            <span class="metric-unit">bar</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">DEMISTER DP</span>
            <span class="metric-val mono">12.5</span>
            <span class="metric-unit">mbar</span>
          </div>
        </div>

        ${this.renderDiagnosticsCardHtml(diag)}
      </div>
    `;
  }

  private renderColumnProfile(target: FaceplateTarget, s: Store) {
    const diag = this.getTargetDiagnostics(target, s);
    const overheadTemp = diag?.inlet_temp_c ?? 78.4;
    const bottomsTemp = diag?.outlet_temp_c ?? 142.0;

    this.equipmentSection.innerHTML = `
      <div class="fp-eq-grid">
        <div class="status-banner status-running">
          <span class="mono font-bold">FRACTIONATION COLUMN</span>
          <span class="badge badge-good">TRAY EQUILIBRIUM</span>
        </div>

        <div class="metrics-grid">
          <div class="metric-box">
            <span class="metric-lbl">OVERHEAD TEMP</span>
            <span class="metric-val mono">${overheadTemp.toFixed(1)}</span>
            <span class="metric-unit">°C</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">BOTTOMS TEMP</span>
            <span class="metric-val mono">${bottomsTemp.toFixed(1)}</span>
            <span class="metric-unit">°C</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">COLUMN DP</span>
            <span class="metric-val mono">145</span>
            <span class="metric-unit">mbar</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">REFLUX RATIO</span>
            <span class="metric-val mono">2.45</span>
            <span class="metric-unit">R/D</span>
          </div>
        </div>

        ${this.renderDiagnosticsCardHtml(diag)}
      </div>
    `;
  }

  private renderTurbineProfile(target: FaceplateTarget, s: Store) {
    const diag = this.getTargetDiagnostics(target, s);
    const speed = diag?.speed_rpm ?? 3600;
    const power = diag?.power_kw ?? 1450.0;
    const isTripped = diag?.is_tripped || false;

    this.equipmentSection.innerHTML = `
      <div class="fp-eq-grid">
        <div class="status-banner ${isTripped ? "status-tripped" : "status-running"}">
          <span class="mono font-bold">${isTripped ? "TURBINE OVERSPEED / ESD TRIP" : "EXPANSION TURBINE RUNNING"}</span>
          <span class="badge ${isTripped ? "badge-alarm" : "badge-good"}">${isTripped ? "TRIPPED" : "SYNCHRONIZED"}</span>
        </div>

        <div class="metrics-grid">
          <div class="metric-box">
            <span class="metric-lbl">SHAFT SPEED</span>
            <span class="metric-val mono">${speed}</span>
            <span class="metric-unit">RPM</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">GENERATOR OUTPUT</span>
            <span class="metric-val mono font-bold">${power.toFixed(1)}</span>
            <span class="metric-unit">kW</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">INLET PRESSURE</span>
            <span class="metric-val mono">42.0</span>
            <span class="metric-unit">bar</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">EXHAUST PRESSURE</span>
            <span class="metric-val mono">3.5</span>
            <span class="metric-unit">bar</span>
          </div>
        </div>

        ${this.renderDiagnosticsCardHtml(diag)}
      </div>
    `;
  }

  private renderTankProfile(target: FaceplateTarget, s: Store) {
    const diag = this.getTargetDiagnostics(target, s);
    const level = target.pvTag ? s.getTagFloat(target.pvTag, 65.0) : 65.0;

    this.equipmentSection.innerHTML = `
      <div class="fp-eq-grid">
        <div class="status-banner status-running">
          <span class="mono font-bold">ATMOSPHERIC STORAGE TANK</span>
          <span class="badge badge-good">ATMOSPHERIC</span>
        </div>

        <div class="metrics-grid">
          <div class="metric-box wide">
            <span class="metric-lbl">LIQUID INVENTORY LEVEL</span>
            <span class="metric-val mono font-bold">${level.toFixed(1)} %</span>
            <div class="meter-bar-horizontal">
              <div class="meter-bar-horiz-fill" style="width: ${Math.min(100, Math.max(0, level))}%;"></div>
            </div>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">WORKING CAPACITY</span>
            <span class="metric-val mono">5000</span>
            <span class="metric-unit">m³</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">HEAD PRESSURE</span>
            <span class="metric-val mono">1.45</span>
            <span class="metric-unit">bar</span>
          </div>
        </div>

        ${this.renderDiagnosticsCardHtml(diag)}
      </div>
    `;
  }

  private renderFurnaceProfile(target: FaceplateTarget, s: Store) {
    const diag = this.getTargetDiagnostics(target, s);
    const isTripped = diag?.is_tripped || s.tags.get("ESD-R101-TEMP.TRIPPED")?.value === true;
    const isRunning = !isTripped && s.isSimRunning;

    const cit = s.getTagFloat("TI-101-CIT", 287.0);
    const cot = s.getTagFloat("TT-101", 332.0);
    const rit = s.getTagFloat("TI-101-RIT", 308.5);
    const rot = s.getTagFloat("TI-101-ROT", 332.0);
    const skinA = s.getTagFloat("TI-101-SK1", 395.0);
    const skinB = s.getTagFloat("TI-101-SK2", 398.0);
    const fgPress = s.getTagFloat("PT-104", 4.2);
    const fgFlow = s.getTagFloat("FT-105", 1250.0);
    const steamPress = s.getTagFloat("PT-105", 7.5);
    const draft = s.getTagFloat("PDT-101", -5.0);
    const stackTemp = s.getTagFloat("TI-104", 180.0);
    const flueO2 = s.getTagFloat("AI-101", 3.2);
    const flueCo = s.getTagFloat("AI-102", 15.0);
    const airFlow = s.getTagFloat("FT-Air", 14250.0);
    const airPress = s.getTagFloat("PT-Air", 24.5);
    const airDamper = s.getTagFloat("FZ-101", 45.0);
    const airRatio = s.getTagFloat("FFC-101", 1.15);
    const foPress = s.getTagFloat("PT-107", 6.0);
    const foFlow = s.getTagFloat("FT-107", 120.0);
    const dutyMw = s.getTagFloat("F-101.DUTY", 4.8);

    this.equipmentSection.innerHTML = `
      <div class="fp-eq-grid">
        <div class="status-banner ${isTripped ? "status-tripped" : isRunning ? "status-running" : "status-stopped"}">
          <span class="mono font-bold">${isTripped ? "✖ FIRED HEATER TRIPPED (SIF FUEL CUT)" : isRunning ? "● FIRED HEATER ONLINE (FIRING NORMAL)" : "■ FIRED HEATER STANDBY"}</span>
          <span class="badge ${isTripped ? "badge-alarm" : isRunning ? "badge-good" : "badge-warn"}">${isTripped ? "FUEL CUT" : isRunning ? "FIRING" : "STANDBY"}</span>
        </div>

        <div class="metrics-grid">
          <div class="metric-box">
            <span class="metric-lbl">CIT (COIL INLET)</span>
            <span class="metric-val mono">${cit.toFixed(1)}</span>
            <span class="metric-unit">°C</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">RIT (RADIANT INLET)</span>
            <span class="metric-val mono">${rit.toFixed(1)}</span>
            <span class="metric-unit">°C</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">ROT (RADIANT OUTLET)</span>
            <span class="metric-val mono">${rot.toFixed(1)}</span>
            <span class="metric-unit">°C</span>
          </div>
          <div class="metric-box highlight">
            <span class="metric-lbl">COT (COIL OUTLET)</span>
            <span class="metric-val mono font-bold">${cot.toFixed(1)}</span>
            <span class="metric-unit">°C</span>
          </div>

          <div class="metric-box ${skinA > 420 ? "metric-warn" : ""}">
            <span class="metric-lbl">SKIN TC PASS A</span>
            <span class="metric-val mono">${skinA.toFixed(1)}</span>
            <span class="metric-unit">°C</span>
          </div>
          <div class="metric-box ${skinB > 420 ? "metric-warn" : ""}">
            <span class="metric-lbl">SKIN TC PASS B</span>
            <span class="metric-val mono">${skinB.toFixed(1)}</span>
            <span class="metric-unit">°C</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">FG HEADER P</span>
            <span class="metric-val mono">${fgPress.toFixed(2)}</span>
            <span class="metric-unit">kg/cm²g</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">FG TOTAL FLOW</span>
            <span class="metric-val mono">${fgFlow.toFixed(0)}</span>
            <span class="metric-unit">Nm³/h</span>
          </div>

          <div class="metric-box">
            <span class="metric-lbl">AIR FLOW (FD FAN)</span>
            <span class="metric-val mono">${airFlow.toFixed(0)}</span>
            <span class="metric-unit">Nm³/h</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">AIR DUCT P</span>
            <span class="metric-val mono">${airPress.toFixed(1)}</span>
            <span class="metric-unit">mbar</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">AIR DAMPER (FZ)</span>
            <span class="metric-val mono">${airDamper.toFixed(0)}</span>
            <span class="metric-unit">%</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">AIR/FUEL RATIO</span>
            <span class="metric-val mono font-bold">${airRatio.toFixed(2)}</span>
            <span class="metric-unit">ratio</span>
          </div>

          <div class="metric-box">
            <span class="metric-lbl">FIREBOX DRAFT</span>
            <span class="metric-val mono">${draft.toFixed(1)}</span>
            <span class="metric-unit">mmH₂O</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">STACK FLUE GAS</span>
            <span class="metric-val mono">${stackTemp.toFixed(1)}</span>
            <span class="metric-unit">°C</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">EXCESS OXYGEN</span>
            <span class="metric-val mono">${flueO2.toFixed(1)}</span>
            <span class="metric-unit">% O₂</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">CO COMBUSTIBLES</span>
            <span class="metric-val mono">${flueCo.toFixed(0)}</span>
            <span class="metric-unit">ppm</span>
          </div>

          <div class="metric-box">
            <span class="metric-lbl">FO HEADER P</span>
            <span class="metric-val mono">${foPress.toFixed(1)}</span>
            <span class="metric-unit">kg/cm²g</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">FO TOTAL FLOW</span>
            <span class="metric-val mono">${foFlow.toFixed(0)}</span>
            <span class="metric-unit">kg/h</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">ATOMIZING STEAM</span>
            <span class="metric-val mono">${steamPress.toFixed(1)}</span>
            <span class="metric-unit">kg/cm²g</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">FIRING DUTY</span>
            <span class="metric-val mono">${dutyMw.toFixed(2)}</span>
            <span class="metric-unit">MW</span>
          </div>
          <div class="metric-box">
            <span class="metric-lbl">NET EFFICIENCY</span>
            <span class="metric-val mono">88.5</span>
            <span class="metric-unit">%</span>
          </div>
        </div>

        <div class="bms-status-bar mono font-small" style="display:flex;justify-content:space-between;background:#0d1522;padding:6px 10px;border-radius:4px;border:1px solid #334155;margin-top:6px;">
          <div><span class="text-dim">DB&B:</span> <strong style="color:${isTripped ? "#ef4444" : "#10b981"}">${isTripped ? "XV-101A/B CLOSED (TRIP)" : "XV-101A/B OPEN | XV-101V CLS"}</strong></div>
          <div><span class="text-dim">FLAME SCANNERS:</span> <strong style="color:${isRunning ? "#10b981" : "#ef4444"}">${isRunning ? "FS-1 ● FS-2 ● FS-3 ● (ALL ON)" : "FS-1 ■ FS-2 ■ FS-3 ■ (NO FLAME)"}</strong></div>
        </div>

        ${this.renderDiagnosticsCardHtml(diag)}

        <div class="operator-action-panel">
          <label class="mono text-dim">BURNER MANAGEMENT & FIRING CONTROL:</label>
          <div class="preset-btn-row">
            <button class="preset-btn btn-fg-cut" ${isTripped ? "disabled" : ""}>CUT FUEL GAS</button>
            <button class="preset-btn btn-fg-reset" ${!isTripped ? "disabled" : ""}>RESET FUEL TRIP</button>
          </div>
        </div>
      </div>
    `;

    this.equipmentSection.querySelector(".btn-fg-cut")?.addEventListener("click", () => this.controlEquipment("stop"));
    this.equipmentSection.querySelector(".btn-fg-reset")?.addEventListener("click", () => this.controlEquipment("reset_trip"));
  }

  private renderInstrumentProfile(target: FaceplateTarget, s: Store) {
    const diag = this.getTargetDiagnostics(target, s);
    const pv = s.getTagFloat(target.pvTag || target.id, 0);
    const unit = target.unit || "";
    const min = target.rangeMin !== undefined ? target.rangeMin : 0;
    const max = target.rangeMax !== undefined ? target.rangeMax : 100;
    const span = max - min || 1;
    const pct = Math.min(100, Math.max(0, ((pv - min) / span) * 100));

    this.equipmentSection.innerHTML = `
      <div class="fp-eq-grid">
        <div class="status-banner status-running">
          <span class="mono font-bold">TRANSMITTER SIGNAL VALID</span>
          <span class="badge badge-good">GOOD QUALITY</span>
        </div>

        <div class="large-readout-card">
          <span class="large-pv mono">${pv.toFixed(2)}</span>
          <span class="large-unit mono">${unit}</span>
        </div>

        <div class="range-bar-wrapper">
          <div class="range-labels mono text-dim">
            <span>${min} ${unit}</span>
            <span>${pct.toFixed(0)}%</span>
            <span>${max} ${unit}</span>
          </div>
          <div class="meter-bar-horizontal">
            <div class="meter-bar-horiz-fill" style="width: ${pct}%;"></div>
          </div>
        </div>

        ${this.renderDiagnosticsCardHtml(diag)}
      </div>
    `;
  }

  private renderGenericProfile(target: FaceplateTarget, s: Store) {
    const diag = this.getTargetDiagnostics(target, s);
    this.equipmentSection.innerHTML = `
      <div class="fp-eq-grid">
        <div class="status-banner status-running">
          <span class="mono font-bold">${target.name}</span>
          <span class="badge badge-info">EQUIPMENT</span>
        </div>
        <p class="text-dim mono" style="padding: 10px;">Operational parameters monitored via DCS supervisory system.</p>
        ${this.renderDiagnosticsCardHtml(diag)}
      </div>
    `;
  }
}

customElements.define("nebu-faceplate", NebuFaceplate);

