import { store, Store } from "../state/store";
import { WsClient } from "../websocket/client";
import { ApiClient } from "../api/client";

export class NebuTagTable extends HTMLElement {
  private unsubscribe?: () => void;
  public wsClient?: WsClient;
  public apiClient?: ApiClient;

  private tbody!: HTMLElement;
  private filterInput!: HTMLInputElement;
  private currentFilter: string = "";

  connectedCallback() {
    this.innerHTML = `
      <div class="tag-table-card">
        <div class="panel-header">
          <div class="title-group">
            <span class="panel-title">TELEMETRY TAG DICTIONARY</span>
            <span class="badge badge-info tag-count-badge">0 TAGS</span>
          </div>
          <div class="filter-group">
            <input type="text" id="tagFilter" placeholder="Filter tags..." />
          </div>
        </div>

        <div class="tag-table-wrapper">
          <table class="tag-table">
            <thead>
              <tr>
                <th>TAG IDENTIFIER</th>
                <th>VALUE</th>
                <th>QUALITY</th>
                <th>UNIT</th>
                <th>MUTATE</th>
              </tr>
            </thead>
            <tbody id="tagTbody">
              <tr><td colspan="5" class="text-dim">No tags discovered</td></tr>
            </tbody>
          </table>
        </div>
      </div>
    `;

    this.tbody = this.querySelector("#tagTbody")!;
    this.filterInput = this.querySelector("#tagFilter")!;

    this.filterInput.addEventListener("input", () => {
      this.currentFilter = this.filterInput.value.trim().toLowerCase();
      this.render(store);
    });

    this.unsubscribe = store.subscribe((s) => this.render(s));
  }

  disconnectedCallback() {
    if (this.unsubscribe) {
      this.unsubscribe();
    }
  }

  private async writeTag(tag: string, valStr: string) {
    const num = parseFloat(valStr);
    if (isNaN(num)) return;

    try {
      if (this.wsClient && this.wsClient.getState() === "synchronized") {
        await this.wsClient.sendCommand({
          type: "write_tag",
          tag,
          value: num,
        });
      } else if (this.apiClient) {
        await this.apiClient.writeTag(tag, num);
      }
    } catch (err) {
      console.error(`Failed to write tag ${tag}:`, err);
    }
  }

  private render(s: Store) {
    const allTags = Array.from(s.tags.values());
    const badge = this.querySelector(".tag-count-badge");
    if (badge) {
      badge.textContent = `${allTags.length} TAGS`;
    }

    const filtered = this.currentFilter
      ? allTags.filter((t) => t.tag.toLowerCase().includes(this.currentFilter))
      : allTags;

    if (filtered.length === 0) {
      this.tbody.innerHTML = `<tr><td colspan="5" class="text-dim">No matching tags</td></tr>`;
      return;
    }

    this.tbody.innerHTML = "";
    for (const item of filtered) {
      const tr = document.createElement("tr");
      const isGood = item.quality === "GOOD";
      const valDisplay = typeof item.value === "number" ? item.value.toFixed(2) : String(item.value);

      tr.innerHTML = `
        <td class="mono font-bold">${item.tag}</td>
        <td class="mono val-cell">${valDisplay}</td>
        <td><span class="badge ${isGood ? "badge-good" : "badge-warn"}">${item.quality}</span></td>
        <td class="text-dim mono">${item.unit || "—"}</td>
        <td class="write-cell">
          <input type="number" class="write-val-in mono" style="width: 70px;" placeholder="val" />
          <button class="write-btn">SET</button>
        </td>
      `;

      const input = tr.querySelector(".write-val-in") as HTMLInputElement;
      const btn = tr.querySelector(".write-btn") as HTMLButtonElement;

      btn.addEventListener("click", () => {
        if (input.value) {
          this.writeTag(item.tag, input.value);
          input.value = "";
        }
      });

      input.addEventListener("keydown", (e) => {
        if (e.key === "Enter" && input.value) {
          this.writeTag(item.tag, input.value);
          input.value = "";
        }
      });

      this.tbody.appendChild(tr);
    }
  }
}

customElements.define("nebu-tag-table", NebuTagTable);
