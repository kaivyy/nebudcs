/**
 * NebuCommunications Web Component.
 *
 * Implements Stage 7: Industrial Protocol & Communication Architecture Workbench:
 * - ISA-95 Levels 0–4 Multi-Tier Security Zone Visualization & Conduit Inspection
 * - Industrial Protocol Connection Management (Modbus TCP, OPC UA, MQTT, Sparkplug B)
 * - Tag Mapping Gateway with Scaling, Deadband, Fallbacks & Duplicate Inbound Prevention
 * - Real-Time Throughput Diagnostics & Bounded Store-and-Forward Monitoring
 * - Cybersecurity & SSRF Guard Policy Inspector (IEC 62443 Conformance)
 */

import { sharedApiClient } from "../api/client";
import {
  ConnectionTestResultDto,
  IndustrialZoneDto,
  ProtocolConnectionDto,
  ProtocolDiagnosticsDto,
  TagMappingDto,
} from "../protocol/types";

export class NebuCommunications extends HTMLElement {
  private activeTab: "architecture" | "connections" | "mappings" | "diagnostics" | "security" = "architecture";
  private connections: ProtocolConnectionDto[] = [];
  private mappings: TagMappingDto[] = [];
  private diagnostics: ProtocolDiagnosticsDto[] = [];
  private zones: IndustrialZoneDto[] = [];
  private isLoading = false;
  private autoRefresh = true;
  private timer: number | null = null;
  private statusMessage = "";
  private testResult: ConnectionTestResultDto | null = null;
  private isTesting = false;
  private selectedConnectionFilter = "ALL";
  private mappingSearchQuery = "";

  connectedCallback() {
    this.fetchData();
    if (this.autoRefresh) {
      this.startPolling();
    }
  }

  disconnectedCallback() {
    this.stopPolling();
  }

  private startPolling() {
    this.stopPolling();
    this.timer = window.setInterval(() => {
      if (this.autoRefresh && !this.isLoading) {
        this.fetchDiagnosticsOnly();
      }
    }, 2000);
  }

  private stopPolling() {
    if (this.timer !== null) {
      window.clearInterval(this.timer);
      this.timer = null;
    }
  }

  private async fetchData() {
    this.isLoading = true;
    try {
      const [conns, maps, diags, zones] = await Promise.all([
        sharedApiClient.listProtocolConnections(),
        sharedApiClient.listTagMappings(),
        sharedApiClient.getProtocolDiagnostics(),
        sharedApiClient.getIndustrialArchitecture(),
      ]);
      this.connections = conns;
      this.mappings = maps;
      this.diagnostics = diags;
      this.zones = zones;
      this.statusMessage = `Updated at ${new Date().toLocaleTimeString()}`;
    } catch (err: any) {
      console.error("[nebu-communications] Fetch error:", err);
      this.statusMessage = `Error: ${err.message || "Failed to load communications data"}`;
    } finally {
      this.isLoading = false;
      this.render();
    }
  }

  private async fetchDiagnosticsOnly() {
    try {
      this.diagnostics = await sharedApiClient.getProtocolDiagnostics();
      const el = this.querySelector(".diag-content");
      if (el && this.activeTab === "diagnostics") {
        this.render();
      }
    } catch (err: any) {
      console.warn("[nebu-communications] Poll diagnostics error:", err);
    }
  }

  private async runConnectionTest(connId: string, protocol: string) {
    this.isTesting = true;
    this.testResult = null;
    this.render();
    try {
      const res = await sharedApiClient.testProtocolConnection(connId, protocol);
      this.testResult = res;
    } catch (err: any) {
      this.testResult = {
        connection_id: connId,
        endpoint: "unknown",
        protocol: protocol,
        success: false,
        latency_ms: 0,
        message: err.message || "Connection test failed",
      };
    } finally {
      this.isTesting = false;
      this.render();
    }
  }

  private getProtocolBadge(protocol: string): string {
    switch (protocol) {
      case "ModbusTcp":
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-amber-950/80 text-amber-300 border border-amber-600/50">Modbus TCP</span>`;
      case "OpcUa":
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-cyan-950/80 text-cyan-300 border border-cyan-600/50">OPC UA</span>`;
      case "Mqtt":
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-indigo-950/80 text-indigo-300 border border-indigo-600/50">MQTT</span>`;
      case "SparkplugB":
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-purple-950/80 text-purple-300 border border-purple-600/50">Sparkplug B</span>`;
      default:
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-slate-800 text-slate-300 border border-slate-700">${protocol}</span>`;
    }
  }

  private getStateBadge(state: string): string {
    switch (state) {
      case "CONNECTED":
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-emerald-950/80 text-emerald-300 border border-emerald-600/50 flex items-center gap-1.5"><span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>Online</span>`;
      case "CONNECTING":
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-blue-950/80 text-blue-300 border border-blue-600/50 flex items-center gap-1.5"><span class="w-1.5 h-1.5 rounded-full bg-blue-400 animate-spin"></span>Connecting</span>`;
      case "FAULTED":
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-rose-950/80 text-rose-300 border border-rose-600/50 flex items-center gap-1.5"><span class="w-1.5 h-1.5 rounded-full bg-rose-400"></span>Faulted</span>`;
      case "DEGRADED":
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-amber-950/80 text-amber-300 border border-amber-600/50 flex items-center gap-1.5"><span class="w-1.5 h-1.5 rounded-full bg-amber-400"></span>Degraded</span>`;
      default:
        return `<span class="px-2 py-0.5 text-xs font-semibold rounded bg-slate-800 text-slate-400 border border-slate-700">Disconnected</span>`;
    }
  }

  render() {
    this.innerHTML = `
      <div class="flex flex-col h-full bg-slate-950 text-slate-200 select-none overflow-hidden font-sans">
        <!-- Top Toolbar Header -->
        <header class="flex items-center justify-between px-4 py-2.5 bg-slate-900/90 border-b border-slate-800">
          <div class="flex items-center gap-3">
            <div class="flex items-center gap-2">
              <span class="text-xl">🔌</span>
              <h2 class="text-base font-bold tracking-wide text-slate-100 uppercase">Industrial Communications & Protocol Gateway</h2>
            </div>
            <span class="px-2 py-0.5 text-xs rounded font-mono bg-cyan-950/70 text-cyan-300 border border-cyan-800/60">ISA-95 Levels 0–4</span>
            <span class="px-2 py-0.5 text-xs rounded font-mono bg-emerald-950/70 text-emerald-300 border border-emerald-800/60">IEC 62443 Guard</span>
          </div>
          <div class="flex items-center gap-2">
            <span class="text-xs text-slate-400 mr-2">${this.statusMessage}</span>
            <button class="btn-refresh px-3 py-1 text-xs rounded bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 transition flex items-center gap-1">
              <span>🔄</span> Refresh
            </button>
          </div>
        </header>

        <!-- Tab Navigation Bar -->
        <nav class="flex items-center px-4 bg-slate-900 border-b border-slate-800 gap-1 text-xs font-medium">
          <button class="tab-btn px-4 py-2.5 border-b-2 transition flex items-center gap-1.5 ${this.activeTab === "architecture" ? "border-cyan-500 text-cyan-300 font-semibold bg-cyan-950/20" : "border-transparent text-slate-400 hover:text-slate-200"}" data-tab="architecture">
            <span>🏢</span> ISA-95 Architecture
          </button>
          <button class="tab-btn px-4 py-2.5 border-b-2 transition flex items-center gap-1.5 ${this.activeTab === "connections" ? "border-cyan-500 text-cyan-300 font-semibold bg-cyan-950/20" : "border-transparent text-slate-400 hover:text-slate-200"}" data-tab="connections">
            <span>📡</span> Connections (${this.connections.length})
          </button>
          <button class="tab-btn px-4 py-2.5 border-b-2 transition flex items-center gap-1.5 ${this.activeTab === "mappings" ? "border-cyan-500 text-cyan-300 font-semibold bg-cyan-950/20" : "border-transparent text-slate-400 hover:text-slate-200"}" data-tab="mappings">
            <span>🔀</span> Tag Mappings (${this.mappings.length})
          </button>
          <button class="tab-btn px-4 py-2.5 border-b-2 transition flex items-center gap-1.5 ${this.activeTab === "diagnostics" ? "border-cyan-500 text-cyan-300 font-semibold bg-cyan-950/20" : "border-transparent text-slate-400 hover:text-slate-200"}" data-tab="diagnostics">
            <span>📊</span> Diagnostics & Store-Forward
          </button>
          <button class="tab-btn px-4 py-2.5 border-b-2 transition flex items-center gap-1.5 ${this.activeTab === "security" ? "border-cyan-500 text-cyan-300 font-semibold bg-cyan-950/20" : "border-transparent text-slate-400 hover:text-slate-200"}" data-tab="security">
            <span>🛡️</span> Cybersecurity & SSRF Guard
          </button>
        </nav>

        <!-- Test Connection Result Banner -->
        ${
          this.testResult
            ? `
          <div class="px-4 py-2.5 flex items-center justify-between text-xs border-b ${
            this.testResult.success
              ? "bg-emerald-950/50 border-emerald-800/80 text-emerald-200"
              : "bg-rose-950/50 border-rose-800/80 text-rose-200"
          }">
            <div class="flex items-center gap-2">
              <span class="text-base">${this.testResult.success ? "✅" : "❌"}</span>
              <div>
                <strong>Test Result for ${this.testResult.connection_id} (${this.testResult.endpoint}):</strong>
                <span>${this.testResult.message}</span>
                ${this.testResult.success ? `<span class="ml-2 font-mono">[Latency: ${this.testResult.latency_ms} ms]</span>` : ""}
              </div>
            </div>
            <button class="btn-dismiss-test px-2 py-0.5 rounded bg-black/40 hover:bg-black/60 text-slate-300">Dismiss</button>
          </div>
        `
            : ""
        }

        <!-- Tab Content Area -->
        <main class="flex-1 overflow-y-auto p-4 bg-slate-950/70">
          ${this.renderTabContent()}
        </main>
      </div>
    `;

    this.attachEventListeners();
  }

  private renderTabContent(): string {
    switch (this.activeTab) {
      case "architecture":
        return this.renderArchitectureTab();
      case "connections":
        return this.renderConnectionsTab();
      case "mappings":
        return this.renderMappingsTab();
      case "diagnostics":
        return this.renderDiagnosticsTab();
      case "security":
        return this.renderSecurityTab();
      default:
        return "";
    }
  }

  private renderArchitectureTab(): string {
    return `
      <div class="space-y-6 max-w-7xl mx-auto">
        <!-- ISA-95 Hierarchy Overview -->
        <div class="grid grid-cols-1 md:grid-cols-5 gap-4">
          ${this.zones
            .map(
              (z) => `
            <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 flex flex-col justify-between shadow-lg relative overflow-hidden">
              <div class="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r ${this.getZoneColor(z.level)}"></div>
              <div>
                <div class="flex items-center justify-between mb-2">
                  <span class="px-2 py-0.5 text-xs font-bold font-mono rounded bg-slate-800 text-cyan-300">${z.level}</span>
                  <span class="text-xs text-slate-500 font-mono">${z.id}</span>
                </div>
                <h3 class="text-sm font-bold text-slate-100 mb-1">${z.name}</h3>
                <p class="text-xs text-slate-400 mb-3">${z.trust_boundary ? "🛡️ Security Boundary" : "Internal Zone"}</p>
              </div>
              <div class="space-y-2 pt-2 border-t border-slate-800/80 text-xs">
                <div>
                  <span class="text-slate-500 block mb-1 font-semibold">Protocols:</span>
                  <div class="flex flex-wrap gap-1">
                    ${(z.permitted_protocols || []).map((p) => `<span class="px-1.5 py-0.2 rounded text-[10px] bg-slate-800 text-slate-300 font-mono">${p}</span>`).join("")}
                  </div>
                </div>
                <div>
                  <span class="text-slate-500 block mb-1 font-semibold">Security Policy:</span>
                  <span class="px-1.5 py-0.5 rounded text-[10px] bg-cyan-950/60 text-cyan-400 border border-cyan-800/40 font-mono block truncate">${z.security_policy || "Default"}</span>
                </div>
              </div>
            </div>
          `
            )
            .join("")}
        </div>

        <!-- Visual Multi-Tier Architecture Diagram & Security Flow -->
        <div class="bg-slate-900/60 border border-slate-800 rounded-lg p-5 shadow-lg">
          <h3 class="text-sm font-bold text-slate-200 mb-3 flex items-center gap-2">
            <span>🛡️</span> ISA-95 Levels 0–4 Multi-Tier Data Flow & Security DMZ Boundaries
          </h3>
          <div class="p-4 bg-slate-950 rounded border border-slate-800/80 font-mono text-xs text-slate-300 space-y-3 leading-relaxed">
            <div class="flex items-center gap-3 p-2 rounded bg-slate-900/90 border-l-4 border-purple-500">
              <span class="text-purple-400 font-bold w-20">Level 4:</span>
              <span class="text-slate-300 flex-1">Enterprise MES / Cloud Analytics (HTTPS, MQTT Sparkplug B DMZ)</span>
              <span class="px-2 py-0.5 rounded text-[11px] bg-purple-950 text-purple-300 border border-purple-700">Read-Only Ingress</span>
            </div>
            <div class="text-center text-slate-600">▲ Conduit: TLS 1.3 / Reverse-Proxy Firewall with SSRF Guard ▲</div>
            <div class="flex items-center gap-3 p-2 rounded bg-slate-900/90 border-l-4 border-indigo-500">
              <span class="text-indigo-400 font-bold w-20">Level 3:</span>
              <span class="text-slate-300 flex-1">Site SCADA & Historian Engine (OPC UA, MQTT Sparkplug B, 100k samples/sec Store-and-Forward)</span>
              <span class="px-2 py-0.5 rounded text-[11px] bg-indigo-950 text-indigo-300 border border-indigo-700">Supervisory</span>
            </div>
            <div class="text-center text-slate-600">▲ Conduit: IEC 62443 Security Zone Boundary (Encrypted TCP) ▲</div>
            <div class="flex items-center gap-3 p-2 rounded bg-slate-900/90 border-l-4 border-cyan-500">
              <span class="text-cyan-400 font-bold w-20">Level 2:</span>
              <span class="text-slate-300 flex-1">NebuDCS Deterministic Control Runtime & Field PLC Nodes (Modbus TCP, OPC UA Server)</span>
              <span class="px-2 py-0.5 rounded text-[11px] bg-cyan-950 text-cyan-300 border border-cyan-700">Authoritative Core</span>
            </div>
            <div class="text-center text-slate-600">▲ Conduit: Local Realtime Memory Bus (Microsecond Tick Boundary) ▲</div>
            <div class="flex items-center gap-3 p-2 rounded bg-slate-900/90 border-l-4 border-emerald-500">
              <span class="text-emerald-400 font-bold w-20">Level 1:</span>
              <span class="text-slate-300 flex-1">I/O Hardware Abstraction Layer (HAL) (Fieldbus, Modbus RTU, 4-20mA HART, Profibus)</span>
              <span class="px-2 py-0.5 rounded text-[11px] bg-emerald-950 text-emerald-300 border border-emerald-700">Physical HAL</span>
            </div>
            <div class="text-center text-slate-600">▲ Direct Physical Coupling ▲</div>
            <div class="flex items-center gap-3 p-2 rounded bg-slate-900/90 border-l-4 border-amber-500">
              <span class="text-amber-400 font-bold w-20">Level 0:</span>
              <span class="text-slate-300 flex-1">Process Simulation & Hydraulic Physics (Conservation of Mass, Momentum, Enthalpy)</span>
              <span class="px-2 py-0.5 rounded text-[11px] bg-amber-950 text-amber-300 border border-amber-700">Ground Truth</span>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  private getZoneColor(level: string): string {
    if (level.includes("0")) return "from-amber-500 to-amber-700";
    if (level.includes("1")) return "from-emerald-500 to-emerald-700";
    if (level.includes("2")) return "from-cyan-500 to-cyan-700";
    if (level.includes("3")) return "from-indigo-500 to-indigo-700";
    if (level.includes("4")) return "from-purple-500 to-purple-700";
    return "from-slate-500 to-slate-700";
  }

  private renderConnectionsTab(): string {
    return `
      <div class="space-y-4 max-w-7xl mx-auto">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <h3 class="text-sm font-bold text-slate-100">Configured Protocol Connections</h3>
            <span class="text-xs text-slate-400">(${this.connections.length} active adapters)</span>
          </div>
        </div>

        <div class="bg-slate-900/80 border border-slate-800 rounded-lg overflow-hidden shadow-lg">
          <table class="w-full text-left text-xs border-collapse">
            <thead>
              <tr class="bg-slate-800/80 text-slate-300 border-b border-slate-700">
                <th class="py-2.5 px-3 font-semibold">Connection ID / Name</th>
                <th class="py-2.5 px-3 font-semibold">Protocol</th>
                <th class="py-2.5 px-3 font-semibold">Endpoint Address</th>
                <th class="py-2.5 px-3 font-semibold">ISA-95 Zone</th>
                <th class="py-2.5 px-3 font-semibold">Timeout / Reconnect</th>
                <th class="py-2.5 px-3 font-semibold">Health State</th>
                <th class="py-2.5 px-3 font-semibold text-right">Actions</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-800">
              ${this.connections
                .map((c) => {
                  const diag = this.diagnostics.find((d) => d.connection_id === c.id);
                  const state = diag ? diag.state : c.enabled ? "CONNECTED" : "DISCONNECTED";
                  return `
                  <tr class="hover:bg-slate-800/40 transition">
                    <td class="py-2.5 px-3">
                      <div class="font-medium text-slate-200">${c.name}</div>
                      <div class="text-[11px] font-mono text-slate-400">${c.id}</div>
                    </td>
                    <td class="py-2.5 px-3">${this.getProtocolBadge(c.protocol)}</td>
                    <td class="py-2.5 px-3 font-mono text-slate-300">${c.endpoint}</td>
                    <td class="py-2.5 px-3">
                      <span class="px-2 py-0.5 text-[11px] rounded bg-slate-800 text-cyan-300 font-mono">${c.zone_id}</span>
                    </td>
                    <td class="py-2.5 px-3 font-mono text-slate-300">
                      ${c.timeout_ms} ms / ${c.reconnect_delay_ms} ms
                    </td>
                    <td class="py-2.5 px-3">${this.getStateBadge(state)}</td>
                    <td class="py-2.5 px-3 text-right">
                      <button class="btn-test-conn px-2.5 py-1 text-xs rounded bg-cyan-950/80 hover:bg-cyan-900 text-cyan-300 border border-cyan-700/60 transition" data-id="${c.id}" data-proto="${c.protocol}">
                        ${this.isTesting ? "⏳ Testing..." : "⚡ Test"}
                      </button>
                    </td>
                  </tr>
                `;
                })
                .join("")}
            </tbody>
          </table>
        </div>
      </div>
    `;
  }

  private renderMappingsTab(): string {
    const filteredMappings = this.mappings.filter((m) => {
      if (this.selectedConnectionFilter !== "ALL" && m.connection_id !== this.selectedConnectionFilter) {
        return false;
      }
      if (this.mappingSearchQuery.trim() !== "") {
        const q = this.mappingSearchQuery.toLowerCase();
        return (
          m.canonical_tag.toLowerCase().includes(q) ||
          m.external_address.toLowerCase().includes(q) ||
          m.id.toLowerCase().includes(q)
        );
      }
      return true;
    });

    return `
      <div class="space-y-4 max-w-7xl mx-auto">
        <!-- Filter and Search Bar -->
        <div class="flex flex-wrap items-center justify-between gap-3 bg-slate-900/60 p-3 rounded-lg border border-slate-800">
          <div class="flex items-center gap-3">
            <label class="text-xs text-slate-400">Filter Connection:</label>
            <select class="sel-conn-filter bg-slate-800 border border-slate-700 rounded px-2.5 py-1 text-xs text-slate-200">
              <option value="ALL">All Connections (${this.mappings.length})</option>
              ${this.connections
                .map((c) => `<option value="${c.id}" ${this.selectedConnectionFilter === c.id ? "selected" : ""}>${c.name} (${c.id})</option>`)
                .join("")}
            </select>
          </div>
          <div class="flex items-center gap-2">
            <input type="text" placeholder="Search canonical tag or address..." class="inp-map-search bg-slate-800 border border-slate-700 rounded px-3 py-1 text-xs text-slate-200 w-64 placeholder-slate-500" value="${this.mappingSearchQuery}" />
          </div>
        </div>

        <!-- Tag Mappings Table -->
        <div class="bg-slate-900/80 border border-slate-800 rounded-lg overflow-hidden shadow-lg">
          <table class="w-full text-left text-xs border-collapse">
            <thead>
              <tr class="bg-slate-800/80 text-slate-300 border-b border-slate-700">
                <th class="py-2.5 px-3 font-semibold">Mapping ID</th>
                <th class="py-2.5 px-3 font-semibold">Direction</th>
                <th class="py-2.5 px-3 font-semibold">External Address</th>
                <th class="py-2.5 px-3 font-semibold">Canonical Tag</th>
                <th class="py-2.5 px-3 font-semibold">Data Type</th>
                <th class="py-2.5 px-3 font-semibold">Scaling (EU = Raw * S + O)</th>
                <th class="py-2.5 px-3 font-semibold">Deadband</th>
                <th class="py-2.5 px-3 font-semibold">Stale Timeout</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-800">
              ${filteredMappings
                .map((m) => `
                <tr class="hover:bg-slate-800/40 transition">
                  <td class="py-2.5 px-3 font-mono text-slate-300">${m.id}</td>
                  <td class="py-2.5 px-3">
                    <span class="px-2 py-0.5 text-[11px] font-bold rounded ${
                      m.direction === "INBOUND"
                        ? "bg-emerald-950/80 text-emerald-300 border border-emerald-600/50"
                        : m.direction === "OUTBOUND"
                        ? "bg-blue-950/80 text-blue-300 border border-blue-600/50"
                        : "bg-purple-950/80 text-purple-300 border border-purple-600/50"
                    }">${m.direction}</span>
                  </td>
                  <td class="py-2.5 px-3 font-mono text-cyan-300">${m.external_address}</td>
                  <td class="py-2.5 px-3 font-bold font-mono text-slate-100">${m.canonical_tag}</td>
                  <td class="py-2.5 px-3 font-mono text-slate-400">${m.datatype}</td>
                  <td class="py-2.5 px-3 font-mono text-slate-300">
                    Raw × ${m.scale} + ${m.offset} ${m.engineering_unit ? `<span class="text-slate-400">[${m.engineering_unit}]</span>` : ""}
                  </td>
                  <td class="py-2.5 px-3 font-mono text-slate-300">${m.deadband}</td>
                  <td class="py-2.5 px-3 font-mono text-slate-400">${m.stale_timeout_ms} ms</td>
                </tr>
              `)
                .join("")}
            </tbody>
          </table>
        </div>
      </div>
    `;
  }

  private renderDiagnosticsTab(): string {
    return `
      <div class="space-y-6 max-w-7xl mx-auto diag-content">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-bold text-slate-100">Real-Time Protocol Telemetry & Diagnostics</h3>
          <span class="text-xs text-slate-400">Automatic polling active (2s interval)</span>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          ${this.diagnostics
            .map((d) => `
            <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 shadow-lg flex flex-col justify-between space-y-3">
              <div>
                <div class="flex items-center justify-between mb-2">
                  <span class="font-mono font-bold text-slate-200 text-xs">${d.connection_id}</span>
                  ${this.getStateBadge(d.state)}
                </div>
                <div class="text-xs text-slate-400 mb-1 flex items-center justify-between">
                  <span>Protocol:</span>
                  ${this.getProtocolBadge(d.protocol)}
                </div>
              </div>

              <!-- Metric Grid -->
              <div class="grid grid-cols-2 gap-2 bg-slate-950/60 p-2.5 rounded border border-slate-800/80 text-xs font-mono">
                <div>
                  <span class="text-slate-500 text-[10px] block">Sent / Received:</span>
                  <span class="text-slate-200">${d.messages_sent} / ${d.messages_received}</span>
                </div>
                <div>
                  <span class="text-slate-500 text-[10px] block">Bandwidth (bytes):</span>
                  <span class="text-slate-200">${(d.bytes_sent + d.bytes_received).toLocaleString()}</span>
                </div>
                <div>
                  <span class="text-slate-500 text-[10px] block">Avg Latency:</span>
                  <span class="text-cyan-300">${d.last_latency_ms} ms</span>
                </div>
                <div>
                  <span class="text-slate-500 text-[10px] block">Errors / Reconnects:</span>
                  <span class="${d.error_count > 0 ? "text-rose-400 font-bold" : "text-slate-300"}">${d.error_count} / ${d.reconnect_count}</span>
                </div>
              </div>

              <!-- Store-and-Forward Bounded Queue KPI -->
              <div class="bg-slate-950/60 p-2.5 rounded border border-slate-800/80 text-xs">
                <span class="text-slate-400 text-[10px] uppercase font-bold block mb-1">Queue & Subscription Telemetry:</span>
                <div class="flex items-center justify-between font-mono text-[11px]">
                  <span class="text-amber-300">Queue Depth: ${d.queue_depth}</span>
                  <span class="text-rose-400">Stale Tags: ${d.stale_tags_count}</span>
                  <span class="text-emerald-400">Subscriptions: ${d.active_subscriptions}</span>
                </div>
              </div>
            </div>
          `)
            .join("")}
        </div>
      </div>
    `;
  }

  private renderSecurityTab(): string {
    return `
      <div class="space-y-6 max-w-7xl mx-auto">
        <!-- SSRF Guard Banner -->
        <div class="bg-emerald-950/30 border border-emerald-700/60 rounded-lg p-4 flex items-start gap-3 shadow-lg">
          <span class="text-2xl">🛡️</span>
          <div>
            <h3 class="text-sm font-bold text-emerald-200">Active SSRF & Endpoint Protection Guard</h3>
            <p class="text-xs text-emerald-300/80 mt-1 leading-relaxed">
              All outbound protocol connections undergo strict security validation before network sockets are opened.
              Cloud instance metadata IP (169.254.169.254) and privileged OS ports (SSH 22, Telnet 23, SMTP 25, SMB 445) are unconditionally blocked.
              Sensitive credentials in connection URLs are automatically redacted in telemetry and audit logs.
            </p>
          </div>
        </div>

        <!-- Policy Matrix -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 shadow-lg">
            <h4 class="text-xs font-bold text-slate-200 uppercase tracking-wide mb-3 flex items-center gap-1.5">
              <span>🚫</span> Blocked Addresses & Blacklisted Ports
            </h4>
            <div class="space-y-2 text-xs font-mono">
              <div class="flex items-center justify-between p-2 rounded bg-slate-950 border border-slate-800/80">
                <span class="text-rose-400">169.254.169.254</span>
                <span class="text-slate-500 text-[11px]">Cloud Metadata (AWS/GCP/Azure IMDS)</span>
              </div>
              <div class="flex items-center justify-between p-2 rounded bg-slate-950 border border-slate-800/80">
                <span class="text-rose-400">Port 22 (SSH)</span>
                <span class="text-slate-500 text-[11px]">Host Management Shell</span>
              </div>
              <div class="flex items-center justify-between p-2 rounded bg-slate-950 border border-slate-800/80">
                <span class="text-rose-400">Port 23 (Telnet)</span>
                <span class="text-slate-500 text-[11px]">Insecure Remote Shell</span>
              </div>
              <div class="flex items-center justify-between p-2 rounded bg-slate-950 border border-slate-800/80">
                <span class="text-rose-400">Port 445 (SMB)</span>
                <span class="text-slate-500 text-[11px]">Windows File Sharing</span>
              </div>
            </div>
          </div>

          <div class="bg-slate-900/80 border border-slate-800 rounded-lg p-4 shadow-lg">
            <h4 class="text-xs font-bold text-slate-200 uppercase tracking-wide mb-3 flex items-center gap-1.5">
              <span>✅</span> Permitted Industrial Protocols & Ports
            </h4>
            <div class="space-y-2 text-xs font-mono">
              <div class="flex items-center justify-between p-2 rounded bg-slate-950 border border-slate-800/80">
                <span class="text-emerald-400">Port 502 (Modbus TCP)</span>
                <span class="text-slate-500 text-[11px]">ISA-95 Level 1/2 PLC/RTU</span>
              </div>
              <div class="flex items-center justify-between p-2 rounded bg-slate-950 border border-slate-800/80">
                <span class="text-cyan-400">Port 4840 (OPC UA TCP)</span>
                <span class="text-slate-500 text-[11px]">IEC 62541 Interoperability</span>
              </div>
              <div class="flex items-center justify-between p-2 rounded bg-slate-950 border border-slate-800/80">
                <span class="text-indigo-400">Port 1883 / 8883 (MQTT / MQTTS)</span>
                <span class="text-slate-500 text-[11px]">Eclipse Sparkplug B Edge Node</span>
              </div>
              <div class="flex items-center justify-between p-2 rounded bg-slate-950 border border-slate-800/80">
                <span class="text-purple-400">Credential Redaction</span>
                <span class="text-slate-500 text-[11px]">Masked: ***REDACTED*** in URI</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  private attachEventListeners() {
    this.querySelectorAll(".tab-btn").forEach((btn) => {
      btn.addEventListener("click", (e: any) => {
        const tab = e.currentTarget.dataset.tab;
        if (tab) {
          this.activeTab = tab;
          this.render();
        }
      });
    });

    const refreshBtn = this.querySelector(".btn-refresh");
    if (refreshBtn) {
      refreshBtn.addEventListener("click", () => this.fetchData());
    }

    const dismissTestBtn = this.querySelector(".btn-dismiss-test");
    if (dismissTestBtn) {
      dismissTestBtn.addEventListener("click", () => {
        this.testResult = null;
        this.render();
      });
    }

    this.querySelectorAll(".btn-test-conn").forEach((btn) => {
      btn.addEventListener("click", (e: any) => {
        const id = e.currentTarget.dataset.id;
        const proto = e.currentTarget.dataset.proto;
        if (id && proto) {
          this.runConnectionTest(id, proto);
        }
      });
    });

    const connFilter = this.querySelector(".sel-conn-filter") as HTMLSelectElement;
    if (connFilter) {
      connFilter.addEventListener("change", (e: any) => {
        this.selectedConnectionFilter = e.target.value;
        this.render();
      });
    }

    const mapSearch = this.querySelector(".inp-map-search") as HTMLInputElement;
    if (mapSearch) {
      mapSearch.addEventListener("input", (e: any) => {
        this.mappingSearchQuery = e.target.value;
        this.render();
      });
    }
  }
}

customElements.define("nebu-communications", NebuCommunications);
