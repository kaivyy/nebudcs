import { store, Store } from "../state/store";
import { WsClient } from "../websocket/client";
import { ApiClient } from "../api/client";

export class NebuSimControls extends HTMLElement {
  private unsubscribe?: () => void;
  public wsClient?: WsClient;
  public apiClient?: ApiClient;

  private playBtn!: HTMLButtonElement;
  private pauseBtn!: HTMLButtonElement;
  private stepBtn!: HTMLButtonElement;
  private resetBtn!: HTMLButtonElement;
  private speedSelect!: HTMLSelectElement;
  private timeDisplay!: HTMLElement;
  private tickDisplay!: HTMLElement;
  private feedbackMsg!: HTMLElement;

  connectedCallback() {
    this.innerHTML = `
      <div class="sim-controls-panel">
        <div class="controls-group">
          <button class="btn-play" title="Resume continuous simulation">▶ Play</button>
          <button class="btn-pause" title="Pause simulation clock">⏸ Pause</button>
          <button class="btn-step" title="Advance by single discrete timestep">⏯ Step</button>
          <button class="btn-reset" title="Reset simulation baseline">⟲ Reset</button>
        </div>

        <div class="speed-group">
          <label for="speed-select" class="text-dim">Speed:</label>
          <select id="speed-select">
            <option value="0.5">0.5x</option>
            <option value="1.0" selected>1.0x</option>
            <option value="2.0">2.0x</option>
            <option value="5.0">5.0x</option>
            <option value="10.0">10.0x</option>
          </select>
        </div>

        <div class="telemetry-clock mono">
          <div class="time-readout">TIME: <span class="time-val">0.00</span> s</div>
          <div class="tick-readout text-dim">TICK: <span class="tick-val">0</span></div>
        </div>

        <div class="feedback-msg mono text-dim"></div>
      </div>
    `;

    this.playBtn = this.querySelector(".btn-play")!;
    this.pauseBtn = this.querySelector(".btn-pause")!;
    this.stepBtn = this.querySelector(".btn-step")!;
    this.resetBtn = this.querySelector(".btn-reset")!;
    this.speedSelect = this.querySelector("#speed-select")!;
    this.timeDisplay = this.querySelector(".time-val")!;
    this.tickDisplay = this.querySelector(".tick-val")!;
    this.feedbackMsg = this.querySelector(".feedback-msg")!;

    this.playBtn.addEventListener("click", () => this.sendSimControl({ action: "play" }));
    this.pauseBtn.addEventListener("click", () => this.sendSimControl({ action: "pause" }));
    this.stepBtn.addEventListener("click", () => this.sendSimControl({ action: "step" }));
    this.resetBtn.addEventListener("click", () => this.sendSimControl({ action: "reset" }));

    this.speedSelect.addEventListener("change", () => {
      const multiplier = parseFloat(this.speedSelect.value);
      this.sendSimControl({ action: "set_speed_multiplier", multiplier });
    });

    this.unsubscribe = store.subscribe((s) => this.render(s));
  }

  disconnectedCallback() {
    if (this.unsubscribe) {
      this.unsubscribe();
    }
  }

  private async sendSimControl(action: { action: string; multiplier?: number }) {
    this.showFeedback("Dispatching command...", false);
    try {
      if (this.wsClient && this.wsClient.getState() === "synchronized") {
        await this.wsClient.sendCommand({
          type: "sim_control",
          ...action,
        } as any);
      } else if (this.apiClient) {
        await this.apiClient.controlSim(action as any);
      }
      this.showFeedback("Command acknowledged", false);
    } catch (err: any) {
      this.showFeedback(`Command failed: ${err.message}`, true);
    }
  }

  private showFeedback(msg: string, isError: boolean) {
    this.feedbackMsg.textContent = msg;
    this.feedbackMsg.style.color = isError ? "var(--status-alarm)" : "var(--status-good)";
    setTimeout(() => {
      if (this.feedbackMsg.textContent === msg) {
        this.feedbackMsg.textContent = "";
      }
    }, 3000);
  }

  private render(s: Store) {
    this.timeDisplay.textContent = s.simTime.toFixed(2);
    this.tickDisplay.textContent = s.tickCount.toString();

    const isRunning = s.isSimRunning;
    this.playBtn.disabled = isRunning;
    this.pauseBtn.disabled = !isRunning;

    if (this.speedSelect.value !== s.speedMultiplier.toString()) {
      this.speedSelect.value = s.speedMultiplier.toString();
    }
  }
}

customElements.define("nebu-sim-controls", NebuSimControls);
