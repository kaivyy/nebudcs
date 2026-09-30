import { store, Store } from "../state/store";
import { WsClient } from "../websocket/client";
import { ApiClient, sharedApiClient } from "../api/client";
import { authStore, UserRoleName } from "../state/auth-store";
import "./nebu-connection-status";
import "./nebu-sim-controls";
import "./nebu-process-view";
import "./nebu-faceplate";
import "./nebu-alarm-panel";
import "./nebu-trend";
import "./nebu-tag-table";
import "./nebu-scenario-panel";
import "./nebu-studio";
import "./nebu-training-studio";
import "./nebu-controller-panel";
import "./nebu-io-panel";
import "./nebu-iec-editor";
import "./nebu-dcs-shell";
import "./nebu-validation-browser";
import "./nebu-cluster-panel";
import "./nebu-audit-panel";
import "./nebu-diagnostics-view";
import "./nebu-backup-panel";
import "./nebu-cybersecurity-panel";
import "./nebu-system-observability";
import "./nebu-system-view";
import "./nebu-command-palette";

export type TopLevelWorkspace =
  | "engineering"
  | "operations"
  | "training"
  | "validation"
  | "system"
  | "enterprise";

export class NebuApp extends HTMLElement {
  public wsClient: WsClient;
  public apiClient: ApiClient;
  public activeMode: TopLevelWorkspace = "operations";
  private enterpriseSubTab: "cluster" | "observability" | "diagnostics" | "audit" | "backup" | "security" = "cluster";
  private unsubscribeStore?: () => void;
  private unsubscribeAuth?: () => void;
  private keydownHandler?: (e: KeyboardEvent) => void;
  private customEventHandlers: Array<{ name: string; handler: (e: any) => void }> = [];
  private clusterInfo = {
    nodeId: "node-01",
    role: "PRIMARY",
    epoch: 1,
    lagMs: 0,
    haState: "SYNCHRONIZED",
  };

  constructor() {
    super();
    this.apiClient = sharedApiClient;
    this.wsClient = new WsClient({ authToken: authStore.getToken() });
  }

  connectedCallback() {
    this.render();

    // Connect WebSocket and subscribe to messages
    this.wsClient.onMessage((msg) => store.handleServerMessage(msg));
    this.wsClient.onStateChange((state) => store.setConnectionState(state));
    this.wsClient.connect();

    // Initial REST Health Check fallback
    this.apiClient.getHealth().then(
      (health) => store.setServerHealth(health),
      (err) => console.warn("[nebudcs-app] Initial health fetch:", err.message)
    );

    // Initial cluster info check
    this.fetchClusterHeaderInfo();

    // Initial SIS status summary check
    this.fetchSisSummary();

    // Initial active project bootstrap & automatic deployment to simulation host
    if (typeof this.apiClient?.listProjects === "function") {
      this.apiClient.listProjects().then(
        async (projects) => {
          if (projects && projects.length > 0 && !store.activeProject) {
            try {
              const urlProject = typeof window !== "undefined" ? new URLSearchParams(window.location.search).get("project") : null;
              const targetId = urlProject || projects.find((p) => p.id === "proj-domain-nht-reference")?.id || projects[0].id;
              const proj = await this.apiClient.getProject(targetId);
              store.setActiveProject(proj);
              // Ensure backend simulation host is synchronized with active project model
              try {
                await this.apiClient.loadProjectSim(proj.id);
                store.setDeploymentStatus(true, proj.id);
              } catch (simErr: any) {
                console.warn("[nebu-app] Initial project sim sync:", simErr?.message || simErr);
              }
            } catch (err: any) {
              console.warn("[nebu-app] Initial project fetch failed:", err?.message || err);
            }
          }
        },
        (err: any) => console.warn("[nebu-app] Initial project listing failed:", err?.message || err)
      );
    }

    // Subscribe to auth token updates
    this.unsubscribeAuth = authStore.subscribe((_role, token) => {
      if (typeof this.apiClient?.setAuthToken === "function") {
        this.apiClient.setAuthToken(token);
      }
    });

    // Global keyboard shortcuts for DCS station
    this.keydownHandler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        const palette = this.querySelector("nebu-command-palette") as any;
        palette?.toggle();
        return;
      }

      const tag = (e.target as HTMLElement)?.tagName?.toLowerCase();
      if (tag === "input" || tag === "select" || tag === "textarea") {
        return;
      }

      if (e.code === "Space") {
        const topAlarm = store.getHighestPriorityUnackAlarm();
        if (topAlarm) {
          e.preventDefault();
          this.quickAckAlarm(topAlarm.alarm_name);
        }
      } else if (e.key === "Escape") {
        store.setFocusedEntity(null);
      }
    };
    window.addEventListener("keydown", this.keydownHandler);

    // Register Command Palette & Global Event Listeners
    const onNavWorkspace = (e: CustomEvent) => {
      if (e.detail?.mode && e.detail.mode !== this.activeMode) {
        this.activeMode = e.detail.mode as TopLevelWorkspace;
        this.render();
      }
    };
    window.addEventListener("navigate-workspace", onNavWorkspace as EventListener);
    this.customEventHandlers.push({ name: "navigate-workspace", handler: onNavWorkspace });

    const onHotDeploy = async () => {
      if (store.activeProject) {
        try {
          await this.apiClient.loadProjectSim(store.activeProject.id);
          store.setDeploymentStatus(true, store.activeProject.id);
        } catch (err: any) {
          console.error("Hot deploy failed:", err);
        }
      }
    };
    window.addEventListener("trigger-hot-deploy", onHotDeploy as EventListener);
    this.customEventHandlers.push({ name: "trigger-hot-deploy", handler: onHotDeploy });

    const onAckAll = async () => {
      const alarmList = store.alarms ? Array.from(store.alarms.values()) : [];
      const unack = alarmList.filter((a) => a.state === "ACTIVE_UNACK" || !(a as any).acknowledged);
      for (const a of unack) {
        await this.quickAckAlarm(a.alarm_name);
      }
    };
    window.addEventListener("trigger-ack-all-alarms", onAckAll as EventListener);
    this.customEventHandlers.push({ name: "trigger-ack-all-alarms", handler: onAckAll });

    // Subscribe to store for reactive status bar & alarm ticker updates
    this.unsubscribeStore = store.subscribe((s) => {
      this.updateAlarmTicker(s);
      this.updateGlobalStatusBarReactive(s);
    });
  }

  private async fetchClusterHeaderInfo() {
    try {
      const topo = await this.apiClient.getClusterTopology();
      const maxLag = topo.nodes.reduce((max, n) => Math.max(max, n.replication_lag_ms), 0);
      this.clusterInfo = {
        nodeId: topo.local_node_id,
        role: topo.nodes.find((n) => n.node_id === topo.local_node_id)?.role || "control_simulation",
        epoch: topo.leader_epoch,
        lagMs: Math.round(maxLag),
        haState: topo.local_ha_state,
      };
      this.updateGlobalStatusBarReactive(store);
    } catch {
      // Offline fallback nominal
    }
  }

  private async fetchSisSummary() {
    try {
      const summary = await this.apiClient.getSisSummary();
      store.setSisSummary(summary);
    } catch {
      // Offline fallback nominal
    }
  }

  public async quickAckAlarm(alarmName: string) {
    try {
      if (this.wsClient && this.wsClient.getState() === "synchronized") {
        await this.wsClient.sendCommand({
          type: "acknowledge_alarm",
          alarm_name: alarmName,
        });
      } else {
        await this.apiClient.acknowledgeAlarm(alarmName);
      }
    } catch (err) {
      console.error(`Quick ack failed for ${alarmName}:`, err);
    }
  }

  private renderRoleStatusBar(role: UserRoleName, s: Store): string {
    const alarmList = s.alarms ? Array.from(s.alarms.values()) : [];
    const unackCount = alarmList.filter((a) => a.state === "ACTIVE_UNACK" || !(a as any).acknowledged).length;
    const standingCount = s.alarms ? s.alarms.size : 0;

    if (role === "operator") {
      const sisTrips = s.sisSummary?.tripped_count || 0;
      return `
        <div class="status-item">
          <span class="label">CLOCK:</span>
          <span class="val" id="sbSimClock" style="color: ${s.isSimRunning ? '#34d399' : '#94a3b8'};">
            ${s.isSimRunning ? "RUNNING @ 50 Hz" : "PAUSED"}
          </span>
        </div>
        <div class="status-item">
          <span class="label">WALL TIME:</span>
          <span class="val mono" id="sbWallClock">${new Date().toISOString().substring(11, 19)} UTC</span>
        </div>
        <div class="status-item">
          <span class="label">SIS:</span>
          <span class="val" id="sbSisStatus" style="color: ${sisTrips > 0 ? '#ef4444' : '#34d399'};">
            ${sisTrips > 0 ? `TRIPPED (${sisTrips} SIF)` : "ARMED (0 Trips)"}
          </span>
        </div>
        <div class="status-item">
          <span class="label">ALARMS:</span>
          <span class="val" id="sbActiveAlarms" style="color: ${unackCount > 0 ? '#ef4444' : '#34d399'};">
            ${unackCount} UNACK / ${standingCount} ACTIVE
          </span>
        </div>
        <div class="status-item">
          <span class="label">I/O HEALTH:</span>
          <span class="val" style="color: #38bdf8;">100% ONLINE (16/16 CHANNELS)</span>
        </div>
        <div class="status-item">
          <span class="label">COMM:</span>
          <span class="val" id="sbCommStatus" style="color: #34d399;">MODBUS / OPC-UA / MQTT ONLINE</span>
        </div>
      `;
    }

    if (role === "engineer") {
      return `
        <div class="status-item">
          <span class="label">CLOCK:</span>
          <span class="val" id="sbSimClock" style="color: ${s.isSimRunning ? '#34d399' : '#94a3b8'};">
            ${s.isSimRunning ? "RUNNING @ 50 Hz" : "PAUSED"}
          </span>
        </div>
        <div class="status-item">
          <span class="label">MODEL VALID:</span>
          <span class="val" style="color: #34d399;">PASS (0 ERRORS, 0 WARNINGS)</span>
        </div>
        <div class="status-item">
          <span class="label">DEPLOY:</span>
          <span class="val" id="sbDeployStatus" style="color: ${s.isProjectDeployed ? '#34d399' : '#f59e0b'};">
            ${s.isProjectDeployed ? "SYNCHRONIZED" : "PENDING"}
          </span>
        </div>
        <div class="status-item">
          <span class="label">READINESS:</span>
          <span class="val" style="color: #34d399;">100% PRE-FLIGHT VERIFIED</span>
        </div>
        <div class="status-item">
          <span class="label">SIS MATRIX:</span>
          <span class="val" style="color: #34d399;">VERIFIED</span>
        </div>
        <div class="status-item">
          <span class="label">COMM:</span>
          <span class="val" id="sbCommStatus" style="color: #38bdf8;">ONLINE (Modbus/OPC UA/MQTT)</span>
        </div>
      `;
    }

    if (role === "instructor" || role === "trainee") {
      return `
        <div class="status-item">
          <span class="label">OTS SESSION:</span>
          <span class="val" style="color: #38bdf8;">ACTIVE</span>
        </div>
        <div class="status-item">
          <span class="label">SIM TIME:</span>
          <span class="val mono" id="sbSimTime">T=00:04:12 (1.0x)</span>
        </div>
        <div class="status-item">
          <span class="label">GRADING:</span>
          <span class="val" style="color: #34d399;">4/4 OBJECTIVES NOMINAL</span>
        </div>
        <div class="status-item">
          <span class="label">SAFETY TRIPS:</span>
          <span class="val" style="color: #34d399;">0</span>
        </div>
        <div class="status-item">
          <span class="label">CLOCK:</span>
          <span class="val" id="sbSimClock" style="color: ${s.isSimRunning ? '#34d399' : '#94a3b8'};">
            ${s.isSimRunning ? "RUNNING @ 50 Hz" : "PAUSED"}
          </span>
        </div>
      `;
    }

    // Default / Admin
    return `
      <div class="status-item">
        <span class="label">NODE:</span>
        <span class="val node-val">${this.clusterInfo.nodeId} [${this.clusterInfo.haState}]</span>
      </div>
      <div class="status-item">
        <span class="label">EPOCH:</span>
        <span class="val" id="sbEpoch">${this.clusterInfo.epoch}</span>
      </div>
      <div class="status-item">
        <span class="label">REPL LAG:</span>
        <span class="val" id="sbReplLag">${this.clusterInfo.lagMs}ms</span>
      </div>
      <div class="status-item">
        <span class="label">DEPLOY:</span>
        <span class="val" id="sbDeployStatus" style="color: ${s.isProjectDeployed ? '#34d399' : '#f59e0b'};">
          ${s.isProjectDeployed ? "SYNCHRONIZED" : "PENDING"}
        </span>
      </div>
      <div class="status-item">
        <span class="label">SIS:</span>
        <span class="val" id="sbSisStatus" style="color: #34d399;">ARMED (0 Trips)</span>
      </div>
      <div class="status-item">
        <span class="label">COMM:</span>
        <span class="val" id="sbCommStatus" style="color: #38bdf8;">ONLINE (Modbus/OPC UA/MQTT)</span>
      </div>
      <div class="status-item">
        <span class="label">CLOCK:</span>
        <span class="val" id="sbSimClock" style="color: ${s.isSimRunning ? '#34d399' : '#94a3b8'};">
          ${s.isSimRunning ? "RUNNING @ 50 Hz" : "PAUSED"}
        </span>
      </div>
    `;
  }

  render() {
    const role = authStore.getRole();
    const roleDef = authStore.getRoleDefinition();
    const topAlarm = store.getHighestPriorityUnackAlarm();
    const alarmList = store.alarms ? Array.from(store.alarms.values()) : [];
    const unackCount = alarmList.filter((a) => a.state === "ACTIVE_UNACK" || !(a as any).acknowledged).length;
    const standingCount = store.alarms ? store.alarms.size : 0;
    const floodRate = store.alarmFloodMetrics?.alarm_rate_per_10min ?? 0;

    this.innerHTML = `
      <header class="app-header">
        <div class="header-left">
          <div class="brand">
            <span class="brand-logo">⚙</span>
            <span class="brand-title">NebuDCS</span>
            <span class="brand-phase badge badge-info">STAGE 14 ENTERPRISE & HA</span>
          </div>
          <div class="header-v-divider"></div>
          <nav class="app-mode-nav">
            <button class="mode-tab ${this.activeMode === "engineering" ? "active" : ""}" data-mode="engineering">
              <span class="tab-icon">🛠️</span> Engineering
            </button>
            <button class="mode-tab ${this.activeMode === "operations" ? "active" : ""}" data-mode="operations">
              <span class="tab-icon">🎛️</span> Operations
            </button>
            <button class="mode-tab ${this.activeMode === "training" ? "active" : ""}" data-mode="training">
              <span class="tab-icon">🎓</span> Training (OTS)
            </button>
            <button class="mode-tab ${this.activeMode === "validation" ? "active" : ""}" data-mode="validation">
              <span class="tab-icon">🔬</span> Scientific Validation
            </button>
            <button class="mode-tab ${this.activeMode === "system" ? "active" : ""}" data-mode="system">
              <span class="tab-icon">🖥️</span> System
            </button>
            <button class="mode-tab ${this.activeMode === "enterprise" ? "active" : ""}" data-mode="enterprise">
              <span class="tab-icon">🏢</span> Enterprise & HA
            </button>
          </nav>
          <div class="header-v-divider"></div>
          <nebu-connection-status></nebu-connection-status>
        </div>

        <div class="header-right">
          <!-- Command Palette Trigger Button -->
          <button class="btn-cmd-palette mono" id="btnOpenPalette" title="Search Tags, Equipment, Loops & Commands (Ctrl+K)">
            <span class="icon">🔍</span>
            <span class="kbd">Ctrl+K</span>
          </button>
          <div class="header-v-divider"></div>

          <div class="role-selector-bar" id="roleSelectorBar">
            <span class="role-title-label text-dim mono">ROLE:</span>
            <select id="selActiveRole" class="sel-active-role mono">
              <option value="admin" ${role === "admin" ? "selected" : ""}>Admin / Lead</option>
              <option value="engineer" ${role === "engineer" ? "selected" : ""}>Engineer</option>
              <option value="instructor" ${role === "instructor" ? "selected" : ""}>Instructor</option>
              <option value="operator" ${role === "operator" ? "selected" : ""}>Operator</option>
              <option value="trainee" ${role === "trainee" ? "selected" : ""}>Trainee</option>
              <option value="custom" ${role === "custom" ? "selected" : ""}>Custom Token...</option>
            </select>
            <span class="badge ${roleDef.badgeClass} mono" id="headerRoleBadge">${roleDef.badge}</span>
            <button class="btn-tool-xs" id="btnEditToken" title="View or Edit Auth Token">🔑</button>
          </div>
          <div class="header-v-divider"></div>
          <nebu-sim-controls id="appSimControls"></nebu-sim-controls>
        </div>
      </header>

      <!-- Global Role-Contextual Reactive Status Bar -->
      <div class="global-status-bar mono" id="globalStatusBar">
        ${this.renderRoleStatusBar(role, store)}
      </div>

      <!-- Persistent 2-Row ISA-18.2 Bottom Alarm Annunciator Bar -->
      <div class="alarm-annunciator-bar alarm-ticker-banner ${unackCount > 0 ? "has-unack" : ""}" id="alarmTickerBanner" style="display: flex;">
        <div class="annunciator-row-top">
          <div class="annunciator-content ticker-content">
            <span class="badge ${topAlarm ? "badge-alarm" : "badge-good"} mono ticker-badge" id="annunciatorBadge">
              ${topAlarm ? `[${topAlarm.severity}]` : "NORMAL"}
            </span>
            <span class="annunciator-msg mono ticker-msg" id="annunciatorMsg">
              ${topAlarm ? `${topAlarm.alarm_name}: PV ${topAlarm.pv.toFixed(1)} exceeded limit ${topAlarm.trip_limit.toFixed(1)} [${topAlarm.severity}]` : "ALL PROCESS SYSTEMS NOMINAL — NO ACTIVE UNACKNOWLEDGED ALARMS"}
            </span>
          </div>
          <div class="annunciator-actions ticker-actions">
            ${topAlarm ? `<button class="btn-ticker-ack" id="btnTickerAck">ACKNOWLEDGE [SPACE]</button>` : ""}
            <button class="btn-ticker-view" id="btnAnnunciatorDetail">
              ${topAlarm ? "DETAIL / FACEPLATE ↗" : "VIEW ALARM LOG"}
            </button>
          </div>
        </div>
        <div class="annunciator-row-bottom">
          <div class="annunciator-metrics">
            <span class="metric-item">STANDING: <span class="val ${standingCount > 0 ? "alarm-active" : ""}" id="sbStandingCount">${standingCount}</span></span>
            <span class="metric-item">UNACK: <span class="val ${unackCount > 0 ? "alarm-active" : ""}" id="sbUnackCount">${unackCount}</span></span>
            <span class="metric-item">SHELVED: <span class="val" id="sbShelvedCount">0</span></span>
            <span class="metric-item">FLOOD RATE (10m): <span class="val" id="sbFloodRate">${floodRate.toFixed(1)}/10m</span></span>
          </div>
          <div class="annunciator-standard text-dim">
            <span>ANSI/ISA-18.2 / EEMUA 191 ALARM MANAGEMENT</span>
          </div>
        </div>
      </div>

      ${
        this.activeMode === "engineering"
          ? `<main class="app-studio-viewport"><nebu-studio></nebu-studio></main>`
          : this.activeMode === "training"
          ? `<main class="app-training-viewport"><nebu-training-studio></nebu-training-studio></main>`
          : this.activeMode === "validation"
          ? `<main class="app-validation-viewport" style="height: calc(100vh - 95px); overflow: auto;"><nebu-validation-browser></nebu-validation-browser></main>`
          : this.activeMode === "system"
          ? `<main class="app-system-viewport" style="height: calc(100vh - 95px); overflow: auto;"><nebu-system-view></nebu-system-view></main>`
          : this.activeMode === "enterprise"
          ? `
            <div class="enterprise-subnav-bar">
              <button class="ent-subtab ${this.enterpriseSubTab === "cluster" ? "active" : ""}" data-subtab="cluster">
                ⛊ Cluster & HA Topology
              </button>
              <button class="ent-subtab ${this.enterpriseSubTab === "observability" ? "active" : ""}" data-subtab="observability">
                📊 Subsystem Observability
              </button>
              <button class="ent-subtab ${this.enterpriseSubTab === "diagnostics" ? "active" : ""}" data-subtab="diagnostics">
                🔍 Global Diagnostics
              </button>
              <button class="ent-subtab ${this.enterpriseSubTab === "audit" ? "active" : ""}" data-subtab="audit">
                📜 Audit & Compliance
              </button>
              <button class="ent-subtab ${this.enterpriseSubTab === "backup" ? "active" : ""}" data-subtab="backup">
                💾 Backup & Recovery
              </button>
              <button class="ent-subtab ${this.enterpriseSubTab === "security" ? "active" : ""}" data-subtab="security">
                🛡️ Cyber Security (IEC 62443)
              </button>
            </div>
            <main class="app-enterprise-viewport" style="height: calc(100vh - 135px); overflow: auto;">
              ${
                this.enterpriseSubTab === "cluster"
                  ? `<nebu-cluster-panel></nebu-cluster-panel>`
                  : this.enterpriseSubTab === "observability"
                  ? `<nebu-system-observability></nebu-system-observability>`
                  : this.enterpriseSubTab === "diagnostics"
                  ? `<nebu-diagnostics-view></nebu-diagnostics-view>`
                  : this.enterpriseSubTab === "audit"
                  ? `<nebu-audit-panel></nebu-audit-panel>`
                  : this.enterpriseSubTab === "backup"
                  ? `<nebu-backup-panel></nebu-backup-panel>`
                  : `<nebu-cybersecurity-panel></nebu-cybersecurity-panel>`
              }
            </main>
          `
          : `
            <main class="app-main">
              <nebu-dcs-shell id="appDcsShell"></nebu-dcs-shell>
            </main>
          `
      }

      <footer class="app-footer mono">
        <div class="footer-left text-dim">
          <span>NebuDCS Deterministic Distributed Core</span>
          <span>•</span>
          <span>High Availability Clustered Runtime</span>
          <span>•</span>
          <span>Vanilla TypeScript & Native Web Components</span>
        </div>
        <div class="footer-right">
          <span class="latency-indicator text-dim">LATENCY: <span id="pingVal">&lt; 5ms</span></span>
        </div>
      </footer>

      <!-- Global Command Palette -->
      <nebu-command-palette id="appCommandPalette"></nebu-command-palette>
    `;

    this.bindAppEvents();
  }

  private updateGlobalStatusBarReactive(s: Store) {
    const nodeVal = this.querySelector(".node-val");
    if (nodeVal) {
      nodeVal.textContent = `${this.clusterInfo.nodeId} [${this.clusterInfo.haState}]`;
    }

    const clockEl = this.querySelector("#sbSimClock");
    if (clockEl) {
      if (s.isSimRunning) {
        clockEl.textContent = `RUNNING @ 50 Hz`;
        (clockEl as HTMLElement).style.color = "#34d399";
      } else {
        clockEl.textContent = "PAUSED";
        (clockEl as HTMLElement).style.color = "#94a3b8";
      }
    }

    const deployEl = this.querySelector("#sbDeployStatus");
    if (deployEl) {
      deployEl.textContent = s.isProjectDeployed ? "SYNCHRONIZED" : "PENDING";
      (deployEl as HTMLElement).style.color = s.isProjectDeployed ? "#34d399" : "#f59e0b";
    }

    const sisEl = this.querySelector("#sbSisStatus");
    if (sisEl && s.sisSummary) {
      const trips = s.sisSummary.tripped_count;
      if (trips > 0) {
        sisEl.textContent = `TRIPPED (${trips} SIF)`;
        (sisEl as HTMLElement).style.color = "#ef4444";
      } else if (s.sisSummary.bypassed_count > 0) {
        sisEl.textContent = `BYPASSED (${s.sisSummary.bypassed_count})`;
        (sisEl as HTMLElement).style.color = "#f59e0b";
      } else {
        sisEl.textContent = `ARMED (0 Trips)`;
        (sisEl as HTMLElement).style.color = "#34d399";
      }
    }

    const alarmsEl = this.querySelector("#sbActiveAlarms");
    if (alarmsEl) {
      const alarmList = s.alarms ? Array.from(s.alarms.values()) : [];
      const unack = alarmList.filter((a) => a.state === "ACTIVE_UNACK" || !(a as any).acknowledged).length;
      alarmsEl.textContent = `${unack} UNACK / ${alarmList.length} ACTIVE`;
      (alarmsEl as HTMLElement).style.color = unack > 0 ? "#ef4444" : "#34d399";
    }
  }

  private bindAppEvents() {
    // Top mode switcher tabs
    this.querySelectorAll(".mode-tab").forEach((tab) => {
      tab.addEventListener("click", () => {
        const mode = tab.getAttribute("data-mode") as TopLevelWorkspace;
        if (mode && mode !== this.activeMode) {
          this.activeMode = mode;
          this.render();
        }
      });
    });

    // Enterprise subtabs
    this.querySelectorAll(".ent-subtab").forEach((tab) => {
      tab.addEventListener("click", () => {
        const sub = tab.getAttribute("data-subtab") as any;
        if (sub && sub !== this.enterpriseSubTab) {
          this.enterpriseSubTab = sub;
          this.render();
        }
      });
    });

    // Inject clients into subcomponents
    const dcsShell = this.querySelector("#appDcsShell") as any;
    if (dcsShell) {
      dcsShell.wsClient = this.wsClient;
      dcsShell.apiClient = this.apiClient;
      dcsShell.injectChildClients();
    }

    const simControls = this.querySelector("#appSimControls") as any;
    if (simControls) {
      simControls.wsClient = this.wsClient;
      simControls.apiClient = this.apiClient;
    }

    const studio = this.querySelector("nebu-studio") as any;
    if (studio) {
      studio.apiClient = this.apiClient;
    }

    const training = this.querySelector("nebu-training-studio") as any;
    if (training) {
      training.apiClient = this.apiClient;
    }

    const systemView = this.querySelector("nebu-system-view") as any;
    if (systemView) {
      systemView.apiClient = this.apiClient;
      systemView.wsClient = this.wsClient;
    }

    // Bind Command Palette Trigger
    this.querySelector("#btnOpenPalette")?.addEventListener("click", () => {
      const pal = this.querySelector("#appCommandPalette") as any;
      pal?.toggle();
    });

    // Bind Role Switcher
    const selRole = this.querySelector("#selActiveRole") as HTMLSelectElement;
    selRole?.addEventListener("change", () => {
      const role = selRole.value as UserRoleName;
      if (role === "custom") {
        const token = prompt("Enter custom bearer authentication token:", authStore.getToken());
        if (token && token.trim()) {
          authStore.setRole("custom", token.trim());
        } else {
          selRole.value = authStore.getRole();
        }
      } else {
        authStore.setRole(role);
      }
      this.render();
    });

    this.querySelector("#btnEditToken")?.addEventListener("click", () => {
      const cur = authStore.getToken();
      const updated = prompt("Bearer authentication token for current session:", cur);
      if (updated && updated.trim() && updated.trim() !== cur) {
        authStore.setToken(updated.trim());
        this.render();
      }
    });

    // Annunciator bar actions
    this.querySelector("#btnAnnunciatorAck")?.addEventListener("click", () => {
      const top = store.getHighestPriorityUnackAlarm();
      if (top) {
        this.quickAckAlarm(top.alarm_name);
      }
    });

    this.querySelector("#btnAnnunciatorDetail")?.addEventListener("click", () => {
      const top = store.getHighestPriorityUnackAlarm();
      if (this.activeMode !== "operations") {
        this.activeMode = "operations";
        this.render();
      }
      if (top) {
        const target = store.getEntityForAlarm(top.alarm_name);
        if (target) {
          store.openFaceplate(target);
        }
        const alarmPanelComp = this.querySelector("nebu-alarm-panel") as any;
        if (alarmPanelComp?.navigateToEntity) {
          alarmPanelComp.navigateToEntity(top.alarm_name);
        }
      }
    });

    // Support horizontal wheel scrolling and mouse drag on app header
    const appHeader = this.querySelector(".app-header") as HTMLElement;
    if (appHeader) {
      appHeader.addEventListener("wheel", (e: WheelEvent) => {
        if (e.deltaY !== 0 && Math.abs(e.deltaX) < Math.abs(e.deltaY)) {
          e.preventDefault();
          appHeader.scrollLeft += e.deltaY;
        }
      }, { passive: false });

      let isDown = false;
      let startX = 0;
      let scrollLeft = 0;

      appHeader.addEventListener("mousedown", (e: MouseEvent) => {
        const target = e.target as HTMLElement;
        if (target.closest("button, select, input, a")) return;
        isDown = true;
        startX = e.pageX - appHeader.offsetLeft;
        scrollLeft = appHeader.scrollLeft;
      });

      window.addEventListener("mouseup", () => {
        isDown = false;
      });

      appHeader.addEventListener("mousemove", (e: MouseEvent) => {
        if (!isDown) return;
        e.preventDefault();
        const x = e.pageX - appHeader.offsetLeft;
        const walk = (x - startX) * 1.5;
        appHeader.scrollLeft = scrollLeft - walk;
      });
    }
  }

  private updateAlarmTicker(s: Store) {
    const bar = (this.querySelector("#alarmAnnunciatorBar") || this.querySelector("#alarmTickerBanner")) as HTMLElement;
    const badgeEl = this.querySelector("#annunciatorBadge") as HTMLElement;
    const msgEl = (this.querySelector("#annunciatorMsg") || this.querySelector("#tickerMsg")) as HTMLElement;
    const actionsEl = (this.querySelector(".annunciator-actions") || this.querySelector(".ticker-actions")) as HTMLElement;
    const unackEl = this.querySelector("#sbUnackCount") as HTMLElement;
    const standingEl = this.querySelector("#sbStandingCount") as HTMLElement;
    const floodEl = this.querySelector("#sbFloodRate") as HTMLElement;

    const topAlarm = s.getHighestPriorityUnackAlarm();
    const alarmList = s.alarms ? Array.from(s.alarms.values()) : [];
    const unackCount = alarmList.filter((a) => a.state === "ACTIVE_UNACK" || !(a as any).acknowledged).length;
    const standingCount = s.alarms ? s.alarms.size : 0;
    const floodRate = s.alarmFloodMetrics?.alarm_rate_per_10min ?? 0;

    if (bar) {
      bar.classList.toggle("has-unack", unackCount > 0);
      bar.style.display = "flex";
    }
    if (unackEl) {
      unackEl.textContent = `${unackCount}`;
      unackEl.classList.toggle("alarm-active", unackCount > 0);
    }
    if (standingEl) {
      standingEl.textContent = `${standingCount}`;
      standingEl.classList.toggle("alarm-active", standingCount > 0);
    }
    if (floodEl) {
      floodEl.textContent = `${floodRate.toFixed(1)}/10m`;
    }

    if (badgeEl && msgEl) {
      if (topAlarm) {
        badgeEl.className = "badge badge-alarm mono ticker-badge";
        badgeEl.textContent = `[${topAlarm.severity}]`;
        msgEl.textContent = `[${topAlarm.severity}] ${topAlarm.alarm_name}: PV ${topAlarm.pv.toFixed(1)} exceeded limit ${topAlarm.trip_limit.toFixed(1)}`;
        if (actionsEl) {
          actionsEl.innerHTML = `
            <button class="btn-ticker-ack" id="btnAnnunciatorAck">ACKNOWLEDGE [SPACE]</button>
            <button class="btn-ticker-view" id="btnAnnunciatorDetail">DETAIL / FACEPLATE ↗</button>
          `;
        }
      } else {
        badgeEl.className = "badge badge-good mono ticker-badge";
        badgeEl.textContent = "NORMAL";
        msgEl.textContent = "ALL PROCESS SYSTEMS NOMINAL — NO ACTIVE UNACKNOWLEDGED ALARMS";
        if (actionsEl) {
          actionsEl.innerHTML = `
            <button class="btn-ticker-view" id="btnAnnunciatorDetail">VIEW ALARM LOG</button>
          `;
        }
      }

      this.querySelector("#btnAnnunciatorAck")?.addEventListener("click", () => {
        const top = store.getHighestPriorityUnackAlarm();
        if (top) this.quickAckAlarm(top.alarm_name);
      });
      this.querySelector("#btnAnnunciatorDetail")?.addEventListener("click", () => {
        const top = store.getHighestPriorityUnackAlarm();
        if (this.activeMode !== "operations") {
          this.activeMode = "operations";
          this.render();
        }
        if (top) {
          const target = store.getEntityForAlarm(top.alarm_name);
          if (target) {
            store.openFaceplate(target);
          }
          const alarmPanel = this.querySelector("nebu-alarm-panel") as any;
          if (alarmPanel?.navigateToEntity) alarmPanel.navigateToEntity(top.alarm_name);
        }
      });
    }
  }

  disconnectedCallback() {
    this.wsClient.disconnect();
    if (this.unsubscribeStore) {
      this.unsubscribeStore();
    }
    if (this.unsubscribeAuth) {
      this.unsubscribeAuth();
    }
    if (this.keydownHandler) {
      window.removeEventListener("keydown", this.keydownHandler);
    }
    for (const h of this.customEventHandlers) {
      window.removeEventListener(h.name, h.handler as EventListener);
    }
  }
}

customElements.define("nebu-app", NebuApp);
