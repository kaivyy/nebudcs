import { store, Store } from "../state/store";
import { WsClient } from "../websocket/client";
import { ApiClient } from "../api/client";
import { PidModeDto } from "../protocol/types";

export class NebuFaceplate extends HTMLElement {
  private unsubscribe?: () => void;
  public wsClient?: WsClient;
  public apiClient?: ApiClient;

  private loopTitle!: HTMLElement;
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
  private feedbackEl!: HTMLElement;

  private currentMode: PidModeDto = "AUTO";

  connectedCallback() {
    this.innerHTML = `
      <div class="faceplate-card">
        <div class="faceplate-header">
          <div class="loop-id-group">
            <span class="loop-name mono font-bold">LIC-101.OUT</span>
            <span class="loop-desc text-dim">LEVEL CONTROLLER</span>
          </div>
          <div class="mode-control">
            <select id="pidModeSelect">
              <option value="AUTO" selected>AUTO</option>
              <option value="MANUAL">MANUAL</option>
              <option value="CASCADE">CASCADE</option>
            </select>
          </div>
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
              <input type="number" id="spIn" min="0" max="100" step="0.5" value="50.0" />
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

          <div class="faceplate-feedback mono text-dim"></div>
        </div>
      </div>
    `;

    this.loopTitle = this.querySelector(".loop-name")!;
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
    this.feedbackEl = this.querySelector(".faceplate-feedback")!;

    // Mode change listener
    this.modeSelect.addEventListener("change", async () => {
      const mode = this.modeSelect.value as PidModeDto;
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

  private async setMode(mode: PidModeDto) {
    const loopName = store.selectedLoop;
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

  private async setSetpoint(sp: number) {
    const loopName = store.selectedLoop;
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

  private async setManualOutput(output: number) {
    const loopName = store.selectedLoop;
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

  private updateManualOutDisabled() {
    const isManual = this.currentMode === "MANUAL";
    this.outInput.disabled = !isManual;
    this.outApplyBtn.disabled = !isManual;
  }

  private showFeedback(msg: string, isError: boolean) {
    this.feedbackEl.textContent = msg;
    this.feedbackEl.style.color = isError ? "var(--status-alarm)" : "var(--status-good)";
    setTimeout(() => {
      if (this.feedbackEl.textContent === msg) {
        this.feedbackEl.textContent = "";
      }
    }, 3000);
  }

  private render(s: Store) {
    const loop = s.selectedLoop;
    this.loopTitle.textContent = loop;

    // Resolve PV, SP, OUT from tags
    let pv = s.getTagFloat("LIC-101.PV", s.getTagFloat("V-101.LEVEL", 50.0));
    let sp = s.getTagFloat("LIC-101.SP", 50.0);
    let out = s.getTagFloat("LIC-101.OUT", s.getTagFloat("FIC-101.OUT", 55.2));

    if (loop.startsWith("FIC")) {
      pv = s.getTagFloat("FIC-101.PV", 4500.0);
      sp = s.getTagFloat("FIC-101.SP", 4500.0);
      out = s.getTagFloat("FIC-101.OUT", 55.2);
    }

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
}

customElements.define("nebu-faceplate", NebuFaceplate);
