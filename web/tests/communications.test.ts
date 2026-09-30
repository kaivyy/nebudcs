import { describe, it, expect, beforeEach, beforeAll, vi } from "vitest";
import { NebuCommunications } from "../src/components/nebu-communications";
import { sharedApiClient } from "../src/api/client";

describe("NebuCommunications Web Component", () => {
  beforeAll(() => {
    if (!window.customElements.get("nebu-communications")) {
      window.customElements.define("nebu-communications", NebuCommunications);
    }
  });

  beforeEach(() => {
    document.body.innerHTML = "";
    vi.restoreAllMocks();
  });

  it("renders architecture overview and ISA-95 zones", async () => {
    vi.spyOn(sharedApiClient, "listProtocolConnections").mockResolvedValue([
      {
        id: "conn-modbus-a",
        name: "CDU Column Modbus Gateway (Plant A)",
        protocol: "ModbusTcp",
        endpoint: "tcp://192.168.1.100:502",
        enabled: true,
        state: "Connected",
        zone_id: "zone-l2-control",
        timeout_ms: 2000,
        reconnect_delay_ms: 5000,
        use_tls: false,
        active_tags: 3,
        last_comm_ms: 15,
      },
    ]);
    vi.spyOn(sharedApiClient, "listTagMappings").mockResolvedValue([
      {
        id: "map-modbus-01",
        connection_id: "conn-modbus-a",
        external_address: "HR:40001:F32:WORDSWAP",
        canonical_tag: "TI-101.PV",
        direction: "Inbound",
        datatype: "Float32",
        scale: 1.0,
        offset: 0.0,
        engineering_unit: "°C",
        deadband: 0.01,
        update_rate_ms: 100,
        stale_timeout_ms: 5000,
        access: "ReadOnly",
        current_value: "185.4",
        quality: "Good",
      },
    ]);
    vi.spyOn(sharedApiClient, "getProtocolDiagnostics").mockResolvedValue([
      {
        connection_id: "conn-modbus-a",
        protocol: "ModbusTcp",
        state: "Connected",
        is_healthy: true,
        messages_sent: 120,
        messages_received: 120,
        bytes_sent: 1440,
        bytes_received: 1440,
        read_operations: 120,
        write_operations: 0,
        error_count: 0,
        reconnect_count: 0,
        active_subscriptions: 0,
        stale_tags_count: 0,
        queue_depth: 0,
        last_latency_ms: 1.2,
        last_comm_ms: 15,
      },
    ]);
    vi.spyOn(sharedApiClient, "getIndustrialArchitecture").mockResolvedValue([
      {
        id: "zone-l0-field",
        name: "Process Simulation Field",
        level: "Level 0",
        permitted_protocols: ["Internal", "DirectMemory"],
        trust_boundary: false,
        security_policy: "IsolatedProcessPhysics",
      },
      {
        id: "zone-l2-control",
        name: "DCS/PLC Control Runtime",
        level: "Level 2",
        permitted_protocols: ["ModbusTcp", "OpcUa"],
        trust_boundary: true,
        security_policy: "DeterministicControlLoop",
      },
    ]);

    const el = document.createElement("nebu-communications") as NebuCommunications;
    document.body.appendChild(el);

    // Wait microtask for async fetch
    await new Promise((r) => setTimeout(r, 50));

    expect(el.innerHTML).toContain("Industrial Communications &amp; Protocol Gateway");
    expect(el.innerHTML).toContain("ISA-95 Levels 0–4");
    expect(el.innerHTML).toContain("Process Simulation Field");
    expect(el.innerHTML).toContain("DCS/PLC Control Runtime");
  });

  it("switches tabs to Connections view and renders connection table", async () => {
    vi.spyOn(sharedApiClient, "listProtocolConnections").mockResolvedValue([
      {
        id: "conn-modbus-a",
        name: "CDU Column Modbus Gateway (Plant A)",
        protocol: "ModbusTcp",
        endpoint: "tcp://192.168.1.100:502",
        enabled: true,
        state: "Connected",
        zone_id: "zone-l2-control",
        timeout_ms: 2000,
        reconnect_delay_ms: 5000,
        use_tls: false,
        active_tags: 3,
        last_comm_ms: 15,
      },
    ]);
    vi.spyOn(sharedApiClient, "listTagMappings").mockResolvedValue([]);
    vi.spyOn(sharedApiClient, "getProtocolDiagnostics").mockResolvedValue([]);
    vi.spyOn(sharedApiClient, "getIndustrialArchitecture").mockResolvedValue([]);

    const el = document.createElement("nebu-communications") as NebuCommunications;
    document.body.appendChild(el);
    await new Promise((r) => setTimeout(r, 50));

    const connTabBtn = el.querySelector("[data-tab='connections']") as HTMLButtonElement;
    expect(connTabBtn).toBeTruthy();
    connTabBtn.click();

    expect(el.innerHTML).toContain("Configured Protocol Connections");
    expect(el.innerHTML).toContain("conn-modbus-a");
    expect(el.innerHTML).toContain("Modbus TCP");
    expect(el.innerHTML).toContain("tcp://192.168.1.100:502");
  });

  it("switches to Security tab and shows SSRF Guard policy", async () => {
    vi.spyOn(sharedApiClient, "listProtocolConnections").mockResolvedValue([]);
    vi.spyOn(sharedApiClient, "listTagMappings").mockResolvedValue([]);
    vi.spyOn(sharedApiClient, "getProtocolDiagnostics").mockResolvedValue([]);
    vi.spyOn(sharedApiClient, "getIndustrialArchitecture").mockResolvedValue([]);

    const el = document.createElement("nebu-communications") as NebuCommunications;
    document.body.appendChild(el);
    await new Promise((r) => setTimeout(r, 50));

    const secTabBtn = el.querySelector("[data-tab='security']") as HTMLButtonElement;
    expect(secTabBtn).toBeTruthy();
    secTabBtn.click();

    expect(el.innerHTML).toContain("Active SSRF &amp; Endpoint Protection Guard");
    expect(el.innerHTML).toContain("169.254.169.254");
    expect(el.innerHTML).toContain("Port 22 (SSH)");
  });
});
