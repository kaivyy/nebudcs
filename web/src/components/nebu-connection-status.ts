import { store, Store } from "../state/store";
import { PROTOCOL_VERSION } from "../protocol/types";

export class NebuConnectionStatus extends HTMLElement {
  private unsubscribe?: () => void;
  private statusBadge!: HTMLElement;
  private sessionInfo!: HTMLElement;

  connectedCallback() {
    this.innerHTML = `
      <div class="connection-bar">
        <span class="status-indicator"></span>
        <span class="status-text badge badge-warn">CONNECTING</span>
        <span class="protocol-tag mono">PROTO: v${PROTOCOL_VERSION}</span>
        <span class="session-tag mono text-dim"></span>
      </div>
    `;

    this.statusBadge = this.querySelector(".status-text")!;
    this.sessionInfo = this.querySelector(".session-tag")!;

    this.unsubscribe = store.subscribe((s) => this.render(s));
  }

  disconnectedCallback() {
    if (this.unsubscribe) {
      this.unsubscribe();
    }
  }

  private render(s: Store) {
    const state = s.connectionState;
    this.statusBadge.textContent = state.toUpperCase();

    this.statusBadge.className = "status-text badge ";
    if (state === "synchronized") {
      this.statusBadge.classList.add("badge-good");
    } else if (state === "connected" || state === "resynchronizing") {
      this.statusBadge.classList.add("badge-warn");
    } else {
      this.statusBadge.classList.add("badge-alarm");
    }

    if (s.sessionId) {
      this.sessionInfo.textContent = `SESSION: ${s.sessionId.substring(0, 10)}...`;
    } else {
      this.sessionInfo.textContent = "";
    }
  }
}

customElements.define("nebu-connection-status", NebuConnectionStatus);
