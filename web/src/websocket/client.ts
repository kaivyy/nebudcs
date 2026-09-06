/**
 * NebuDCS WebSocket Client.
 * Manages connection lifecycle, handshake, sequence tracking, subscriptions, and dispatching.
 */

import {
  ClientMessage,
  isCompatibleProtocol,
  PROTOCOL_VERSION,
  RuntimeCommand,
  ServerMessage,
} from "../protocol/types";

export type ConnectionState =
  | "disconnected"
  | "connecting"
  | "connected"
  | "reconnecting"
  | "synchronized"
  | "resynchronizing";

export interface WsClientOptions {
  url?: string;
  clientId?: string;
  authToken?: string;
  autoReconnect?: boolean;
  reconnectDelayMs?: number;
  maxReconnectDelayMs?: number;
}

export type MessageHandler = (message: ServerMessage) => void;
export type StateHandler = (state: ConnectionState) => void;

export class WsClient {
  private ws: WebSocket | null = null;
  private url: string;
  private clientId: string;
  private authToken?: string;
  private autoReconnect: boolean;
  private reconnectDelayMs: number;
  private maxReconnectDelayMs: number;
  private currentReconnectDelay: number;
  private reconnectTimer: number | null = null;

  private state: ConnectionState = "disconnected";
  private lastSeq: number = 0;
  private pendingCommands: Map<
    string,
    { resolve: (success: boolean) => void; reject: (err: Error) => void }
  > = new Map();

  private messageHandlers: Set<MessageHandler> = new Set();
  private stateHandlers: Set<StateHandler> = new Set();

  private subscriptions: Set<string> = new Set();
  private subscribeAll: boolean = true;
  private includeAlarms: boolean = true;

  constructor(options: WsClientOptions = {}) {
    this.clientId = options.clientId || `web-${Math.random().toString(36).substring(2, 9)}`;
    this.authToken = options.authToken;
    this.autoReconnect = options.autoReconnect ?? true;
    this.reconnectDelayMs = options.reconnectDelayMs ?? 1000;
    this.maxReconnectDelayMs = options.maxReconnectDelayMs ?? 15000;
    this.currentReconnectDelay = this.reconnectDelayMs;

    if (options.url) {
      this.url = options.url;
    } else if (typeof window !== "undefined" && window.location) {
      const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
      this.url = `${proto}//${window.location.host}/ws`;
    } else {
      this.url = "ws://127.0.0.1:8080/ws";
    }
  }

  public getState(): ConnectionState {
    return this.state;
  }

  public onMessage(handler: MessageHandler): () => void {
    this.messageHandlers.add(handler);
    return () => this.messageHandlers.delete(handler);
  }

  public onStateChange(handler: StateHandler): () => void {
    this.stateHandlers.add(handler);
    return () => this.stateHandlers.delete(handler);
  }

  private setState(newState: ConnectionState) {
    if (this.state !== newState) {
      this.state = newState;
      this.stateHandlers.forEach((h) => h(newState));
    }
  }

  public connect() {
    if (this.ws && (this.ws.readyState === WebSocket.CONNECTING || this.ws.readyState === WebSocket.OPEN)) {
      return;
    }

    this.setState(this.state === "disconnected" ? "connecting" : "reconnecting");

    let finalUrl = this.url;
    if (this.authToken) {
      const delim = finalUrl.includes("?") ? "&" : "?";
      finalUrl = `${finalUrl}${delim}token=${encodeURIComponent(this.authToken)}`;
    }

    try {
      this.ws = new WebSocket(finalUrl);
    } catch (err) {
      this.handleClose();
      return;
    }

    this.ws.onopen = () => {
      this.currentReconnectDelay = this.reconnectDelayMs;
      this.sendHandshake();
    };

    this.ws.onmessage = (event: MessageEvent) => {
      try {
        const data = JSON.parse(event.data) as ServerMessage;
        this.processServerMessage(data);
      } catch (err) {
        console.error("[nebudcs-ws] Failed to parse message:", err, event.data);
      }
    };

    this.ws.onerror = (err) => {
      console.warn("[nebudcs-ws] Socket error:", err);
    };

    this.ws.onclose = () => {
      this.handleClose();
    };
  }

  private sendHandshake() {
    const handshake: ClientMessage = {
      type: "handshake",
      protocol_version: PROTOCOL_VERSION,
      client_id: this.clientId,
      capabilities: {
        supports_binary: false,
        max_batch_size: 1000,
      },
    };
    this.send(handshake);
  }

  private processServerMessage(msg: ServerMessage) {
    switch (msg.type) {
      case "handshake_ack": {
        const { protocol_version } = msg.payload;
        if (!isCompatibleProtocol(protocol_version)) {
          console.error(`[nebudcs-ws] Incompatible server protocol: ${protocol_version}`);
          this.disconnect();
          return;
        }
        this.setState("connected");
        // Re-establish subscriptions
        this.sendSubscriptions();
        break;
      }

      case "snapshot": {
        this.lastSeq = msg.payload.tick_count;
        this.setState("synchronized");
        break;
      }

      case "delta": {
        const { seq } = msg.payload;
        if (this.state === "synchronized") {
          if (seq > this.lastSeq + 1 && this.lastSeq > 0) {
            console.warn(`[nebudcs-ws] Sequence gap detected (expected ${this.lastSeq + 1}, got ${seq}); requesting snapshot resync`);
            this.setState("resynchronizing");
            this.requestSnapshot();
          } else {
            this.lastSeq = seq;
          }
        }
        break;
      }

      case "command_ack": {
        const { command_id, success, message } = msg.payload;
        const pending = this.pendingCommands.get(command_id);
        if (pending) {
          if (success) {
            pending.resolve(true);
          } else {
            pending.reject(new Error(message || "Command execution rejected by server"));
          }
          this.pendingCommands.delete(command_id);
        }
        break;
      }

      case "pong":
        break;

      case "error":
        console.error("[nebudcs-ws] Server error envelope:", msg.payload);
        break;
    }

    // Broadcast to registered observers
    this.messageHandlers.forEach((h) => h(msg));
  }

  public subscribe(tags: string[], includeAlarms: boolean = true) {
    tags.forEach((t) => this.subscriptions.add(t));
    this.subscribeAll = false;
    this.includeAlarms = includeAlarms;
    if (this.state === "synchronized" || this.state === "connected") {
      this.sendSubscriptions();
    }
  }

  public subscribeAllTags(includeAlarms: boolean = true) {
    this.subscribeAll = true;
    this.includeAlarms = includeAlarms;
    if (this.state === "synchronized" || this.state === "connected") {
      this.sendSubscriptions();
    }
  }

  private sendSubscriptions() {
    const subMsg: ClientMessage = {
      type: "subscribe",
      tags: Array.from(this.subscriptions),
      all_tags: this.subscribeAll,
      include_alarms: this.includeAlarms,
    };
    this.send(subMsg);
  }

  public requestSnapshot() {
    const req: ClientMessage = { type: "request_snapshot" };
    this.send(req);
  }

  public sendCommand(command: RuntimeCommand): Promise<boolean> {
    const command_id = `cmd-${Date.now()}-${Math.random().toString(36).substring(2, 7)}`;
    const msg: ClientMessage = {
      type: "command",
      command_id,
      command,
    };

    return new Promise<boolean>((resolve, reject) => {
      this.pendingCommands.set(command_id, { resolve, reject });
      const sent = this.send(msg);
      if (!sent) {
        this.pendingCommands.delete(command_id);
        reject(new Error("WebSocket not connected"));
      }

      // Timeout command if server doesn't respond within 5s
      setTimeout(() => {
        if (this.pendingCommands.has(command_id)) {
          this.pendingCommands.delete(command_id);
          reject(new Error("Command acknowledgment timeout"));
        }
      }, 5000);
    });
  }

  public ping(nonce: number = Date.now()) {
    const pingMsg: ClientMessage = { type: "ping", nonce };
    this.send(pingMsg);
  }

  public send(msg: ClientMessage): boolean {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
      return true;
    }
    return false;
  }

  private handleClose() {
    this.setState("disconnected");
    this.ws = null;

    // Fail any pending commands
    this.pendingCommands.forEach(({ reject }) => {
      reject(new Error("Connection closed"));
    });
    this.pendingCommands.clear();

    if (this.autoReconnect) {
      if (this.reconnectTimer) {
        clearTimeout(this.reconnectTimer);
      }
      this.reconnectTimer = window.setTimeout(() => {
        this.currentReconnectDelay = Math.min(
          this.currentReconnectDelay * 1.5,
          this.maxReconnectDelayMs
        );
        this.connect();
      }, this.currentReconnectDelay);
    }
  }

  public disconnect() {
    this.autoReconnect = false;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.setState("disconnected");
  }
}
