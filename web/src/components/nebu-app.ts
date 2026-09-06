import { store } from "../state/store";
import { WsClient } from "../websocket/client";
import { ApiClient } from "../api/client";
import "./nebu-connection-status";
import "./nebu-sim-controls";
import "./nebu-process-view";
import "./nebu-faceplate";
import "./nebu-alarm-panel";
import "./nebu-trend";
import "./nebu-tag-table";
import "./nebu-scenario-panel";

export class NebuApp extends HTMLElement {
  public wsClient: WsClient;
  public apiClient: ApiClient;

  constructor() {
    super();
    this.apiClient = new ApiClient();
    this.wsClient = new WsClient();
  }

  connectedCallback() {
    this.innerHTML = `
      <header class="app-header">
        <div class="header-left">
          <div class="brand">
            <span class="brand-logo">⚙</span>
            <span class="brand-title">NebuDCS</span>
            <span class="brand-phase badge badge-info">PHASE 9 SCENARIO & FAULT</span>
          </div>
          <nebu-connection-status></nebu-connection-status>
        </div>

        <div class="header-right">
          <nebu-sim-controls id="appSimControls"></nebu-sim-controls>
        </div>
      </header>

      <main class="app-main">
        <!-- Left Column: Process Graphics, Control Faceplate & Realtime Trend -->
        <section class="viewport-column main-viewport">
          <nebu-process-view></nebu-process-view>
          
          <div class="hmi-subgrid">
            <nebu-faceplate id="appFaceplate"></nebu-faceplate>
            <nebu-trend id="appTrend"></nebu-trend>
          </div>
        </section>

        <!-- Right Column: Scenario & Faults, Alarms & Tag Inspector -->
        <aside class="sidebar-column side-viewport">
          <nebu-scenario-panel id="appScenarioPanel"></nebu-scenario-panel>
          <nebu-alarm-panel id="appAlarmPanel"></nebu-alarm-panel>
          <nebu-tag-table id="appTagTable"></nebu-tag-table>
        </aside>
      </main>

      <footer class="app-footer mono">
        <div class="footer-left text-dim">
          <span>NebuDCS Deterministic Simulation Core</span>
          <span>•</span>
          <span>Axum / Tokio Protocol Server</span>
          <span>•</span>
          <span>Vanilla TypeScript & Native Web Components</span>
        </div>
        <div class="footer-right">
          <span class="latency-indicator text-dim">LATENCY: <span id="pingVal">< 5ms</span></span>
        </div>
      </footer>
    `;

    // Inject clients into subcomponents
    const simControls = this.querySelector("#appSimControls") as any;
    if (simControls) {
      simControls.wsClient = this.wsClient;
      simControls.apiClient = this.apiClient;
    }

    const faceplate = this.querySelector("#appFaceplate") as any;
    if (faceplate) {
      faceplate.wsClient = this.wsClient;
      faceplate.apiClient = this.apiClient;
    }

    const alarmPanel = this.querySelector("#appAlarmPanel") as any;
    if (alarmPanel) {
      alarmPanel.wsClient = this.wsClient;
      alarmPanel.apiClient = this.apiClient;
    }

    const tagTable = this.querySelector("#appTagTable") as any;
    if (tagTable) {
      tagTable.wsClient = this.wsClient;
      tagTable.apiClient = this.apiClient;
    }

    // Connect WebSocket and subscribe to messages
    this.wsClient.onMessage((msg) => store.handleServerMessage(msg));
    this.wsClient.onStateChange((state) => store.setConnectionState(state));

    this.wsClient.connect();

    // Initial REST Health Check fallback
    this.apiClient.getHealth().then(
      (health) => store.setServerHealth(health),
      (err) => console.warn("[nebudcs-app] Initial health fetch:", err.message)
    );
  }

  disconnectedCallback() {
    this.wsClient.disconnect();
  }
}

customElements.define("nebu-app", NebuApp);
