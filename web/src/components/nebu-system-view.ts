/**
 * NebuSystemView Web Component.
 *
 * Implements the "System" Workspace:
 * - Subsystem Observability & Telemetry (10 subsystems)
 * - Virtual Controllers Runtime & Task Scheduling
 * - I/O Hardware Abstraction Layer (HAL racks, chassis, channels)
 * - Fieldbus Communications (Modbus TCP, OPC UA, MQTT)
 * - Historian Storage & Throughput
 * - Global Diagnostics & Issue Correlation
 */

import { ApiClient, sharedApiClient } from "../api/client";
import { WsClient } from "../websocket/client";
import "./nebu-system-observability";
import "./nebu-controller-panel";
import "./nebu-io-panel";
import "./nebu-communications";
import "./nebu-diagnostics-view";
import "./nebu-cybersecurity-panel";

type SystemSubTab = "observability" | "controllers" | "io" | "comms" | "diagnostics" | "security";

export class NebuSystemView extends HTMLElement {
  public apiClient: ApiClient = sharedApiClient;
  public wsClient?: WsClient;
  private activeSubTab: SystemSubTab = "observability";

  connectedCallback() {
    this.render();
  }

  render() {
    this.innerHTML = `
      <div class="system-workspace-shell flex flex-col h-full bg-[#0b0f17] text-[#e2e8f0]">
        <!-- Sub-navigation Bar -->
        <nav class="system-subnav-bar flex items-center px-4 py-2 bg-[#131822] border-b border-[#1e293b] gap-2">
          <button class="sys-subtab btn-subtab ${this.activeSubTab === "observability" ? "active" : ""}" data-subtab="observability">
            📊 Subsystem Telemetry
          </button>
          <button class="sys-subtab btn-subtab ${this.activeSubTab === "controllers" ? "active" : ""}" data-subtab="controllers">
            🖥️ Virtual Controllers
          </button>
          <button class="sys-subtab btn-subtab ${this.activeSubTab === "io" ? "active" : ""}" data-subtab="io">
            🔌 I/O HAL (Racks & Modules)
          </button>
          <button class="sys-subtab btn-subtab ${this.activeSubTab === "comms" ? "active" : ""}" data-subtab="comms">
            🌐 Fieldbus Comms
          </button>
          <button class="sys-subtab btn-subtab ${this.activeSubTab === "diagnostics" ? "active" : ""}" data-subtab="diagnostics">
            🔍 Global Diagnostics
          </button>
          <button class="sys-subtab btn-subtab ${this.activeSubTab === "security" ? "active" : ""}" data-subtab="security">
            🛡️ IEC 62443 Security
          </button>
        </nav>

        <!-- Dynamic Viewport -->
        <main class="system-content-viewport flex-1 overflow-auto p-4">
          ${
            this.activeSubTab === "observability"
              ? `<nebu-system-observability class="block h-full"></nebu-system-observability>`
              : this.activeSubTab === "controllers"
              ? `<nebu-controller-panel class="block h-full"></nebu-controller-panel>`
              : this.activeSubTab === "io"
              ? `<nebu-io-panel class="block h-full"></nebu-io-panel>`
              : this.activeSubTab === "comms"
              ? `<nebu-communications class="block h-full"></nebu-communications>`
              : this.activeSubTab === "diagnostics"
              ? `<nebu-diagnostics-view class="block h-full"></nebu-diagnostics-view>`
              : `<nebu-cybersecurity-panel class="block h-full"></nebu-cybersecurity-panel>`
          }
        </main>
      </div>
    `;

    this.bindEvents();
    this.injectClients();
  }

  private bindEvents() {
    this.querySelectorAll(".sys-subtab").forEach((btn) => {
      btn.addEventListener("click", () => {
        const sub = btn.getAttribute("data-subtab") as SystemSubTab;
        if (sub && sub !== this.activeSubTab) {
          this.activeSubTab = sub;
          this.render();
        }
      });
    });
  }

  private injectClients() {
    const ctrlPanel = this.querySelector("nebu-controller-panel") as any;
    if (ctrlPanel) {
      ctrlPanel.apiClient = this.apiClient;
    }
    const ioPanel = this.querySelector("nebu-io-panel") as any;
    if (ioPanel) {
      ioPanel.apiClient = this.apiClient;
    }
  }
}

customElements.define("nebu-system-view", NebuSystemView);
