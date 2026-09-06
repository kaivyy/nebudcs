import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { WsClient } from '../src/websocket/client';
import { ServerMessage } from '../src/protocol/types';

class MockWebSocket {
  static instances: MockWebSocket[] = [];
  public static readonly CONNECTING = 0;
  public static readonly OPEN = 1;
  public static readonly CLOSING = 2;
  public static readonly CLOSED = 3;

  public url: string;
  public readyState: number = MockWebSocket.CONNECTING;
  public onopen: ((ev: Event) => void) | null = null;
  public onclose: ((ev: CloseEvent) => void) | null = null;
  public onerror: ((ev: Event) => void) | null = null;
  public onmessage: ((ev: MessageEvent) => void) | null = null;
  public sentMessages: string[] = [];

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  public open() {
    this.readyState = MockWebSocket.OPEN;
    if (this.onopen) {
      this.onopen(new Event('open'));
    }
  }

  public send(data: string) {
    this.sentMessages.push(data);
  }

  public close() {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      this.onclose(new CloseEvent('close'));
    }
  }

  public pushServerMessage(msg: ServerMessage) {
    if (this.onmessage) {
      this.onmessage(new MessageEvent('message', { data: JSON.stringify(msg) }));
    }
  }
}

// Assign to global
const originalWebSocket = globalThis.WebSocket;

describe('WsClient Protocol & Lifecycle', () => {
  beforeEach(() => {
    MockWebSocket.instances = [];
    // @ts-ignore
    globalThis.WebSocket = MockWebSocket;
  });

  afterEach(() => {
    globalThis.WebSocket = originalWebSocket;
  });

  it('starts disconnected and sends handshake upon opening', () => {
    const client = new WsClient({ url: 'ws://localhost:8080/ws', autoReconnect: false });
    expect(client.getState()).toBe('disconnected');

    client.connect();
    expect(client.getState()).toBe('connecting');

    const ws = MockWebSocket.instances[0];
    expect(ws).toBeDefined();
    expect(ws.url).toBe('ws://localhost:8080/ws');

    ws.open();
    expect(ws.sentMessages.length).toBe(1);

    const handshake = JSON.parse(ws.sentMessages[0]);
    expect(handshake.type).toBe('handshake');
    expect(handshake.protocol_version).toBe('1.0.0');
    expect(handshake.client_id).toBeDefined();
  });

  it('transitions to connected and sends subscriptions on handshake_ack', () => {
    const client = new WsClient({ autoReconnect: false });
    client.connect();
    const ws = MockWebSocket.instances[0];
    ws.open();

    // Server sends handshake_ack
    ws.pushServerMessage({
      type: 'handshake_ack',
      payload: {
        protocol_version: '1.0.0',
        server_version: '0.1.0',
        session_id: 'sess-test-1',
        model_version: '1.0.0',
      },
    });

    expect(client.getState()).toBe('connected');

    // Should have sent subscriptions (subscribeAll by default)
    expect(ws.sentMessages.length).toBe(2);
    const subMsg = JSON.parse(ws.sentMessages[1]);
    expect(subMsg.type).toBe('subscribe');
    expect(subMsg.all_tags).toBe(true);
  });

  it('disconnects on incompatible protocol version in handshake_ack', () => {
    const client = new WsClient({ autoReconnect: false });
    client.connect();
    const ws = MockWebSocket.instances[0];
    ws.open();

    ws.pushServerMessage({
      type: 'handshake_ack',
      payload: {
        protocol_version: '2.0.0', // incompatible major version
        server_version: '2.0.0',
        session_id: 'sess-bad',
        model_version: '2.0.0',
      },
    });

    expect(client.getState()).toBe('disconnected');
    expect(ws.readyState).toBe(MockWebSocket.CLOSED);
  });

  it('transitions to synchronized on snapshot reception', () => {
    const client = new WsClient({ autoReconnect: false });
    client.connect();
    const ws = MockWebSocket.instances[0];
    ws.open();

    ws.pushServerMessage({
      type: 'snapshot',
      payload: {
        snapshot_id: 'snap-1',
        sim_time_sec: 0,
        tick_count: 10,
        tags: [],
        active_alarms: [],
      },
    });

    expect(client.getState()).toBe('synchronized');
  });

  it('detects sequence gaps and triggers snapshot resynchronization', () => {
    const client = new WsClient({ autoReconnect: false });
    client.connect();
    const ws = MockWebSocket.instances[0];
    ws.open();

    // Snapshot at seq/tick 100
    ws.pushServerMessage({
      type: 'snapshot',
      payload: {
        snapshot_id: 'snap-1',
        sim_time_sec: 5.0,
        tick_count: 100,
        tags: [],
        active_alarms: [],
      },
    });
    expect(client.getState()).toBe('synchronized');

    // Normal contiguous delta seq 101
    ws.pushServerMessage({
      type: 'delta',
      payload: { seq: 101, sim_time_sec: 5.02, tick_count: 101, changed_tags: [] },
    });
    expect(client.getState()).toBe('synchronized');

    // Gap! seq jumps to 110 (missing 102..109)
    ws.pushServerMessage({
      type: 'delta',
      payload: { seq: 110, sim_time_sec: 5.20, tick_count: 110, changed_tags: [] },
    });

    expect(client.getState()).toBe('resynchronizing');
    // Verify client requested snapshot
    const lastSent = JSON.parse(ws.sentMessages[ws.sentMessages.length - 1]);
    expect(lastSent.type).toBe('request_snapshot');
  });

  it('resolves and rejects command promises via command_ack', async () => {
    const client = new WsClient({ autoReconnect: false });
    client.connect();
    const ws = MockWebSocket.instances[0];
    ws.open();

    const cmdPromise = client.sendCommand({
      type: 'sim_control',
      action: 'play',
    });

    // Check sent message
    const lastSent = JSON.parse(ws.sentMessages[ws.sentMessages.length - 1]);
    expect(lastSent.type).toBe('command');
    const cmdId = lastSent.command_id;
    expect(cmdId).toBeDefined();

    // Server acknowledges with success
    ws.pushServerMessage({
      type: 'command_ack',
      payload: {
        command_id: cmdId,
        success: true,
      },
    });

    await expect(cmdPromise).resolves.toBe(true);

    // Now test rejection
    const failPromise = client.sendCommand({
      type: 'set_setpoint',
      loop_name: 'LIC-101.OUT',
      sp: -999, // invalid setpoint
    });
    const failSent = JSON.parse(ws.sentMessages[ws.sentMessages.length - 1]);
    ws.pushServerMessage({
      type: 'command_ack',
      payload: {
        command_id: failSent.command_id,
        success: false,
        message: 'Invalid setpoint value',
      },
    });

    await expect(failPromise).rejects.toThrow('Invalid setpoint value');
  });
});
