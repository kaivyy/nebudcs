import { store, Store } from "../state/store";
import { WsClient } from "../websocket/client";
import { ApiClient } from "../api/client";

export class NebuAlarmPanel extends HTMLElement {
  private unsubscribe?: () => void;
  public wsClient?: WsClient;
  public apiClient?: ApiClient;

  private unackCountBadge!: HTMLElement;
  private totalCountBadge!: HTMLElement;
  private standingCountBadge!: HTMLElement;
  private staleCountBadge!: HTMLElement;
  private shelvedCountBadge!: HTMLElement;
  private floodBanner!: HTMLElement;
  private floodRateTxt!: HTMLElement;
  private tableBody!: HTMLElement;
  private sevFilterSelect!: HTMLSelectElement;
  private stateFilterSelect!: HTMLSelectElement;

  connectedCallback() {
    this.innerHTML = `
      <div class="alarm-panel-card">
        <div class="alarm-flood-banner mono font-bold" id="alarmFloodBanner" style="display: none;">
          <span class="badge badge-alarm">⚠ ALARM FLOOD ACTIVE (ISA-18.2)</span>
          <span id="floodRateTxt">Rate: 0.0 alarms/10min</span>
        </div>

        <div class="panel-header">
          <div class="title-group">
            <span class="panel-title">ALARM &amp; PROTECTION STATION (ANSI/ISA-18.2)</span>
            <span class="badge badge-alarm unack-badge">0 UNACK</span>
            <span class="badge badge-warn active-badge">0 ACTIVE</span>
            <span class="badge badge-info standing-badge">0 STANDING</span>
            <span class="badge badge-secondary stale-badge">0 STALE</span>
            <span class="badge badge-secondary shelved-badge">0 SHELVED</span>
          </div>
          <div class="panel-actions">
            <button id="ackAllBtn" class="btn-ack-all" title="Acknowledge all active alarms (Annunciator only)">ACK ALL</button>
            <button id="resetTripsBtn" class="btn-reset-trips" title="Reset latched BPCS interlocks">RESET INTERLOCKS</button>
            <button id="resetSisBtn" class="btn-reset-sis btn-alarm" title="Reset Tripped Safety Instrumented Functions (SIS)">RESET SIS</button>
          </div>
        </div>

        <div class="op-separation-banner mono font-small text-dim">
          <span class="text-warn font-bold">OPERATIONAL SEPARATION:</span> Acknowledging alarms only silences visual/audible annunciators. Interlocks &amp; SIS trips require independent physical verification and reset.
        </div>

        <!-- Filter Bar -->
        <div class="alarm-filter-bar mono">
          <div class="filter-group">
            <label for="alarmSevFilter">SEV:</label>
            <select id="alarmSevFilter">
              <option value="ALL" selected>ALL SEVERITIES</option>
              <option value="CRITICAL">CRITICAL</option>
              <option value="WARNING">WARNING / HIGH</option>
              <option value="ADVISORY">ADVISORY / LOW</option>
            </select>
          </div>
          <div class="filter-group">
            <label for="alarmStateFilter">STATE:</label>
            <select id="alarmStateFilter">
              <option value="ALL" selected>ALL STATES</option>
              <option value="UNACK">UNACK ONLY</option>
              <option value="ACTIVE">ACTIVE ONLY</option>
              <option value="SHELVED">SHELVED</option>
            </select>
          </div>
        </div>

        <div class="alarm-table-wrapper">
          <table class="alarm-table">
            <thead>
              <tr>
                <th>SEVERITY</th>
                <th>ALARM NAME</th>
                <th>STATE</th>
                <th>PV</th>
                <th>LIMIT</th>
                <th>ACTIONS</th>
              </tr>
            </thead>
            <tbody id="alarmTbody">
              <tr class="empty-row"><td colspan="6" class="text-dim">No active alarms</td></tr>
            </tbody>
          </table>
        </div>
      </div>
    `;

    this.unackCountBadge = this.querySelector(".unack-badge")!;
    this.totalCountBadge = this.querySelector(".active-badge")!;
    this.standingCountBadge = this.querySelector(".standing-badge")!;
    this.staleCountBadge = this.querySelector(".stale-badge")!;
    this.shelvedCountBadge = this.querySelector(".shelved-badge")!;
    this.floodBanner = this.querySelector("#alarmFloodBanner")!;
    this.floodRateTxt = this.querySelector("#floodRateTxt")!;
    this.tableBody = this.querySelector("#alarmTbody")!;
    this.sevFilterSelect = this.querySelector("#alarmSevFilter")!;
    this.stateFilterSelect = this.querySelector("#alarmStateFilter")!;

    // Filter change listeners
    this.sevFilterSelect.addEventListener("change", () => {
      store.setAlarmSeverityFilter(this.sevFilterSelect.value);
    });

    this.stateFilterSelect.addEventListener("change", () => {
      store.setAlarmStateFilter(this.stateFilterSelect.value);
    });

    // Global Ack All
    this.querySelector("#ackAllBtn")?.addEventListener("click", async () => {
      const alarms = store.getAlarms();
      for (const a of alarms) {
        if (a.state === "ACTIVE_UNACK") {
          await this.ackAlarm(a.alarm_name);
        }
      }
    });

    // Reset Trips Action
    this.querySelector("#resetTripsBtn")?.addEventListener("click", async () => {
      try {
        if (this.apiClient) {
          await this.apiClient.resetProtection();
        }
      } catch (err) {
        console.error("Failed to reset protection trips:", err);
      }
    });

    // Reset SIS Action
    this.querySelector("#resetSisBtn")?.addEventListener("click", async () => {
      try {
        if (this.apiClient) {
          const sifs = await this.apiClient.listSifs();
          for (const sif of sifs) {
            if (sif.state === "Tripped") {
              await this.apiClient.resetSif(sif.id, "Operator");
            }
          }
        }
      } catch (err) {
        console.error("Failed to reset SIS trips:", err);
      }
    });

    this.unsubscribe = store.subscribe((s) => this.render(s));
  }

  disconnectedCallback() {
    if (this.unsubscribe) {
      this.unsubscribe();
    }
  }

  public async ackAlarm(alarmName: string) {
    try {
      if (this.wsClient && this.wsClient.getState() === "synchronized") {
        await this.wsClient.sendCommand({
          type: "acknowledge_alarm",
          alarm_name: alarmName,
        });
      } else if (this.apiClient) {
        await this.apiClient.acknowledgeAlarm(alarmName);
      }
    } catch (err) {
      console.error(`Failed to acknowledge alarm ${alarmName}:`, err);
    }
  }

  public async shelveAlarm(alarmName: string, durationSeconds: number = 60.0) {
    try {
      if (this.wsClient && this.wsClient.getState() === "synchronized") {
        await this.wsClient.sendCommand({
          type: "shelve_alarm",
          alarm_name: alarmName,
          duration_seconds: durationSeconds,
        });
      } else if (this.apiClient) {
        await this.apiClient.shelveAlarm(alarmName, durationSeconds);
      }
    } catch (err) {
      console.error(`Failed to shelve alarm ${alarmName}:`, err);
    }
  }

  /**
   * Navigates to the entity associated with an alarm using the active project model.
   * Looks up the alarm by name in the project alarms list, then resolves its pv_tag
   * to a canonical entity via store.getEntityForAlarm().
   * Shows an unavailable state if the alarm source cannot be found.
   */
  public navigateToEntity(alarmName: string) {
    const target = store.getEntityForAlarm(alarmName);
    if (target) {
      store.openFaceplate(target);
      store.setFocusedEntity(target.id);
    } else {
      // Alarm entity not found in the active project — show a clear unavailable state
      // rather than navigating to an unrelated object
      console.warn(`[alarm-panel] Cannot navigate to alarm '${alarmName}': entity not found in active project`);
      store.openFaceplate({
        kind: "instrument",
        id: alarmName,
        name: alarmName,
        description: "ALARM SOURCE — entity not found in active project",
      });
      store.setFocusedEntity(null);
    }
  }

  private render(s: Store) {
    const rawAlarms = s.getAlarms();
    const unack = rawAlarms.filter((a) => a.state === "ACTIVE_UNACK").length;
    const active = rawAlarms.filter((a) => a.state !== "NORMAL" && a.state !== "CLEARED").length;

    this.unackCountBadge.textContent = `${unack} UNACK`;
    this.totalCountBadge.textContent = `${active} ACTIVE`;

    const flood = s.alarmFloodMetrics;
    if (flood && flood.is_flood) {
      this.floodBanner.style.display = "flex";
      this.floodRateTxt.textContent = `Rate: ${flood.alarm_rate_per_10min.toFixed(1)} / 10min (Threshold: ${flood.flood_threshold_per_10min.toFixed(1)}) — ${flood.total_active} Active, ${flood.unacknowledged} Unack`;
    } else {
      this.floodBanner.style.display = "none";
    }

    if (this.standingCountBadge) {
      this.standingCountBadge.textContent = `${flood?.standing_count || 0} STANDING`;
    }
    if (this.staleCountBadge) {
      this.staleCountBadge.textContent = `${flood?.stale_count || 0} STALE`;
    }
    if (this.shelvedCountBadge) {
      this.shelvedCountBadge.textContent = `${flood?.shelved || 0} SHELVED`;
    }

    this.unackCountBadge.style.display = unack > 0 ? "inline-block" : "none";
    this.totalCountBadge.style.display = active > 0 ? "inline-block" : "none";

    // Apply operator filters
    const filteredAlarms = rawAlarms.filter((a) => {
      // Severity filter
      if (s.alarmSeverityFilter !== "ALL") {
        const sevStr = a.severity as string;
        if (s.alarmSeverityFilter === "CRITICAL" && sevStr !== "CRITICAL") return false;
        if (s.alarmSeverityFilter === "WARNING" && sevStr !== "WARNING" && sevStr !== "HIGH") return false;
        if (s.alarmSeverityFilter === "ADVISORY" && sevStr !== "ADVISORY" && sevStr !== "LOW") return false;
      }
      // State filter
      if (s.alarmStateFilter !== "ALL") {
        if (s.alarmStateFilter === "UNACK" && a.state !== "ACTIVE_UNACK") return false;
        if (s.alarmStateFilter === "ACTIVE" && a.state === "NORMAL") return false;
        if (s.alarmStateFilter === "SHELVED" && a.state !== "SHELVED") return false;
      }
      return true;
    });

    if (filteredAlarms.length === 0) {
      this.tableBody.innerHTML = `<tr class="empty-row"><td colspan="6" class="text-dim">No matching alarms</td></tr>`;
      return;
    }

    this.tableBody.innerHTML = "";
    for (const alm of filteredAlarms) {
      const tr = document.createElement("tr");
      tr.className = `alarm-row state-${alm.state.toLowerCase()}`;

      const isCritical = alm.severity === "CRITICAL";
      const sevBadge = `<span class="badge badge-${isCritical ? "alarm" : "warn"}">${alm.severity}</span>`;
      const isUnack = alm.state === "ACTIVE_UNACK";

      tr.innerHTML = `
        <td>${sevBadge}</td>
        <td class="mono font-bold">${alm.alarm_name}</td>
        <td class="mono"><span class="badge badge-${isUnack ? "alarm" : "good"}">${alm.state}</span></td>
        <td class="mono">${alm.pv.toFixed(1)}</td>
        <td class="mono">${alm.trip_limit.toFixed(1)}</td>
        <td class="actions-cell">
          <button class="goto-btn" title="Focus equipment and open faceplate">GOTO</button>
          <button class="ack-btn" ${!isUnack ? "disabled" : ""}>ACK</button>
          <button class="shelve-btn">SHELVE (60s)</button>
        </td>
      `;

      tr.querySelector(".goto-btn")?.addEventListener("click", () => this.navigateToEntity(alm.alarm_name));
      tr.querySelector(".ack-btn")?.addEventListener("click", () => this.ackAlarm(alm.alarm_name));
      tr.querySelector(".shelve-btn")?.addEventListener("click", () => this.shelveAlarm(alm.alarm_name, 60.0));

      this.tableBody.appendChild(tr);
    }
  }
}

customElements.define("nebu-alarm-panel", NebuAlarmPanel);

