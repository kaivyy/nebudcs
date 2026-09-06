import { store, Store } from "../state/store";
import { WsClient } from "../websocket/client";
import { ApiClient } from "../api/client";

export class NebuAlarmPanel extends HTMLElement {
  private unsubscribe?: () => void;
  public wsClient?: WsClient;
  public apiClient?: ApiClient;

  private unackCountBadge!: HTMLElement;
  private totalCountBadge!: HTMLElement;
  private tableBody!: HTMLElement;

  connectedCallback() {
    this.innerHTML = `
      <div class="alarm-panel-card">
        <div class="panel-header">
          <div class="title-group">
            <span class="panel-title">ALARM & PROTECTION LOG (ANSI/ISA-18.2)</span>
            <span class="badge badge-alarm unack-badge">0 UNACK</span>
            <span class="badge badge-warn active-badge">0 ACTIVE</span>
          </div>
          <div class="panel-actions">
            <button id="ackAllBtn" title="Acknowledge all active alarms">ACK ALL</button>
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
    this.tableBody = this.querySelector("#alarmTbody")!;

    this.querySelector("#ackAllBtn")?.addEventListener("click", async () => {
      const alarms = store.getAlarms();
      for (const a of alarms) {
        if (a.state === "ACTIVE_UNACK") {
          await this.ackAlarm(a.alarm_name);
        }
      }
    });

    this.unsubscribe = store.subscribe((s) => this.render(s));
  }

  disconnectedCallback() {
    if (this.unsubscribe) {
      this.unsubscribe();
    }
  }

  private async ackAlarm(alarmName: string) {
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

  private async shelveAlarm(alarmName: string, durationSeconds: number = 60.0) {
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

  private render(s: Store) {
    const alarms = s.getAlarms();
    const unack = alarms.filter((a) => a.state === "ACTIVE_UNACK").length;
    const active = alarms.filter((a) => a.state !== "NORMAL" && a.state !== "CLEARED").length;

    this.unackCountBadge.textContent = `${unack} UNACK`;
    this.totalCountBadge.textContent = `${active} ACTIVE`;

    this.unackCountBadge.style.display = unack > 0 ? "inline-block" : "none";
    this.totalCountBadge.style.display = active > 0 ? "inline-block" : "none";

    if (alarms.length === 0) {
      this.tableBody.innerHTML = `<tr class="empty-row"><td colspan="6" class="text-dim">No active alarms</td></tr>`;
      return;
    }

    this.tableBody.innerHTML = "";
    for (const alm of alarms) {
      const tr = document.createElement("tr");
      tr.className = `alarm-row state-${alm.state.toLowerCase()}`;

      const sevBadge = `<span class="badge badge-${alm.severity === "CRITICAL" ? "alarm" : "warn"}">${alm.severity}</span>`;
      const isUnack = alm.state === "ACTIVE_UNACK";

      tr.innerHTML = `
        <td>${sevBadge}</td>
        <td class="mono font-bold">${alm.alarm_name}</td>
        <td class="mono"><span class="badge badge-${isUnack ? "alarm" : "good"}">${alm.state}</span></td>
        <td class="mono">${alm.pv.toFixed(1)}</td>
        <td class="mono">${alm.trip_limit.toFixed(1)}</td>
        <td class="actions-cell">
          <button class="ack-btn" ${!isUnack ? "disabled" : ""}>ACK</button>
          <button class="shelve-btn">SHELVE (60s)</button>
        </td>
      `;

      tr.querySelector(".ack-btn")?.addEventListener("click", () => this.ackAlarm(alm.alarm_name));
      tr.querySelector(".shelve-btn")?.addEventListener("click", () => this.shelveAlarm(alm.alarm_name, 60.0));

      this.tableBody.appendChild(tr);
    }
  }
}

customElements.define("nebu-alarm-panel", NebuAlarmPanel);
