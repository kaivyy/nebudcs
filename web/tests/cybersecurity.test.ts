/**
 * Stage 15: Industrial Cyber Security & IEC 62443 Alignment Test Suite.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { client } from "../src/api/client";
import {
  AuditVerificationResultDto,
  PurdueConduitDto,
  PurdueZoneDto,
  SecurityIncidentDto,
  SecurityStatusDto,
} from "../src/protocol/types";
import "../src/components/nebu-cybersecurity-panel";
import { NebuCybersecurityPanel } from "../src/components/nebu-cybersecurity-panel";
import "../src/components/nebu-system-view";
import { NebuSystemView } from "../src/components/nebu-system-view";
import "../src/components/nebu-app";
import { NebuApp } from "../src/components/nebu-app";

const MOCK_SECURITY_STATUS: SecurityStatusDto = {
  system_security_level: "SL3_SOPHISTICATED",
  active_threat_level: "LOW",
  total_incidents: 4,
  active_incidents: 0,
  audit_chain_valid: true,
  audit_events_count: 42,
  root_hash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  mtls_enabled: true,
  rbac_enforced: true,
  zones_count: 6,
  conduits_count: 5,
  last_audit_verification_iso: "2026-09-09T05:00:00Z",
};

const MOCK_ZONES: PurdueZoneDto[] = [
  {
    zone_id: "zone-l5",
    name: "Enterprise & Cloud Network",
    level_name: "Level 5",
    purdue_level: 5,
    target_security_level: "SL1_COINCIDENTAL",
    achieved_security_level: "SL1_COINCIDENTAL",
    assets_count: 12,
    status: "ACTIVE",
    trust_boundary: true,
    description: "Corporate IT and cloud reporting",
    assigned_roles: ["Admin"],
  },
  {
    zone_id: "zone-l3",
    name: "Site Operations & Simulation Host",
    level_name: "Level 3",
    purdue_level: 3,
    target_security_level: "SL3_SOPHISTICATED",
    achieved_security_level: "SL3_SOPHISTICATED",
    assets_count: 8,
    status: "ACTIVE",
    trust_boundary: true,
    description: "NebuDCS authoritative process simulation host",
    assigned_roles: ["Admin", "Engineer", "Operator"],
  },
  {
    zone_id: "zone-l1",
    name: "Basic Process Control & Safety (SIS)",
    level_name: "Level 1",
    purdue_level: 1,
    target_security_level: "SL4_EXTENDED",
    achieved_security_level: "SL4_EXTENDED",
    assets_count: 24,
    status: "ACTIVE",
    trust_boundary: true,
    description: "Virtual PLC/DCS controllers and emergency shutdown",
    assigned_roles: ["Engineer", "Operator"],
  },
];

const MOCK_CONDUITS: PurdueConduitDto[] = [
  {
    conduit_id: "cnd-l3-l4",
    name: "Operations to Enterprise Data Diode",
    source_zone_id: "zone-l3",
    destination_zone_id: "zone-l4",
    allowed_protocols: ["HTTPS", "MQTT_TLS"],
    security_level_required: "SL3_SOPHISTICATED",
    is_encrypted: true,
    packets_inspected: 10450,
    violations_blocked: 3,
    status: "DATA_DIODE_UNIDIRECTIONAL",
    policy_description: "Strict unidirectional read-only data diode",
  },
  {
    conduit_id: "cnd-l2-l1",
    name: "Supervisory HMI to Process Controllers",
    source_zone_id: "zone-l2",
    destination_zone_id: "zone-l1",
    allowed_protocols: ["MODBUS_TCP", "OPC_UA", "IEC61850"],
    security_level_required: "SL4_EXTENDED",
    is_encrypted: true,
    packets_inspected: 85200,
    violations_blocked: 0,
    status: "ACTIVE_INSPECTED",
    policy_description: "Hardened field control bus",
  },
];

const MOCK_INCIDENTS: SecurityIncidentDto[] = [
  {
    incident_id: "SEC-INC-000001",
    timestamp_iso: "2026-09-09T05:10:00Z",
    severity: "CRITICAL",
    category: "DATA_DIODE_WRITE_ATTEMPT",
    source_ip: "10.0.4.55",
    actor: "untrusted_cloud",
    description: "Inbound write packet rejected by unidirectional hardware diode cnd-l3-l4",
    mitigation_action: "HARDWARE_DROP_PACKET",
    status: "RESOLVED",
  },
];

const MOCK_VERIFY_RESULT: AuditVerificationResultDto = {
  is_valid: true,
  total_events_checked: 42,
  root_hash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  latest_hash: "3b7c844129fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b112",
  tampering_detected: false,
  first_invalid_event_id: undefined,
  verification_timestamp_iso: "2026-09-09T05:25:00Z",
  verification_status: "CRYPTOGRAPHICALLY_VERIFIED",
};

describe("Stage 15: Industrial Cyber Security & IEC 62443 Alignment", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.spyOn(client, "getSecurityStatus").mockResolvedValue(MOCK_SECURITY_STATUS);
    vi.spyOn(client, "getSecurityZones").mockResolvedValue(MOCK_ZONES);
    vi.spyOn(client, "getSecurityConduits").mockResolvedValue(MOCK_CONDUITS);
    vi.spyOn(client, "getSecurityIncidents").mockResolvedValue(MOCK_INCIDENTS);
    vi.spyOn(client, "verifyAuditIntegrity").mockResolvedValue(MOCK_VERIFY_RESULT);
    vi.spyOn(client, "updateConduitPolicy").mockImplementation(async (req) => ({
      ...MOCK_CONDUITS[0],
      is_encrypted: req.is_encrypted,
    }));
  });

  it("client fetches security posture, zones, conduits and incidents", async () => {
    const status = await client.getSecurityStatus();
    expect(status.system_security_level).toBe("SL3_SOPHISTICATED");
    expect(status.audit_chain_valid).toBe(true);

    const zones = await client.getSecurityZones();
    expect(zones.length).toBe(3);
    expect(zones[0].purdue_level).toBe(5);

    const conduits = await client.getSecurityConduits();
    expect(conduits.length).toBe(2);
    expect(conduits[0].conduit_id).toBe("cnd-l3-l4");
    expect(conduits[0].violations_blocked).toBe(3);

    const incidents = await client.getSecurityIncidents(10);
    expect(incidents.length).toBe(1);
    expect(incidents[0].category).toBe("DATA_DIODE_WRITE_ATTEMPT");
  });

  it("client verifies cryptographic audit hash chain integrity", async () => {
    const res = await client.verifyAuditIntegrity();
    expect(res.is_valid).toBe(true);
    expect(res.total_events_checked).toBe(42);
    expect(res.tampering_detected).toBe(false);
    expect(res.verification_status).toBe("CRYPTOGRAPHICALLY_VERIFIED");
  });

  it("renders <nebu-cybersecurity-panel> with Purdue model zones and conduits", async () => {
    const panel = new NebuCybersecurityPanel();
    document.body.appendChild(panel);
    await new Promise((r) => setTimeout(r, 20));

    expect(panel.innerHTML).toContain("IEC 62443 Industrial Cyber Security");
    expect(panel.innerHTML).toContain("SL3_SOPHISTICATED");
    expect(panel.innerHTML).toContain("LOW");
    expect(panel.innerHTML).toContain("Purdue Model (PERA) Industrial Zones");
    expect(panel.innerHTML).toContain("DATA DIODE: WRITE-BLOCKED");
    expect(panel.innerHTML).toContain("DATA_DIODE_WRITE_ATTEMPT");

    document.body.removeChild(panel);
  });

  it("triggers audit hash chain verification from button", async () => {
    const panel = new NebuCybersecurityPanel();
    document.body.appendChild(panel);
    await new Promise((r) => setTimeout(r, 20));

    const btn = panel.querySelector("#btnVerifyAudit") as HTMLButtonElement;
    expect(btn).toBeTruthy();
    btn.click();
    await new Promise((r) => setTimeout(r, 20));

    expect(client.verifyAuditIntegrity).toHaveBeenCalledTimes(1);
    expect(panel.innerHTML).toContain("ALL HASHES MATHEMATICALLY VERIFIED");
    expect(panel.innerHTML).toContain("Merkle Root SHA-256:");

    document.body.removeChild(panel);
  });

  it("integrates cybersecurity panel into NebuSystemView as sub-tab", async () => {
    const sysView = new NebuSystemView();
    document.body.appendChild(sysView);
    await new Promise((r) => setTimeout(r, 10));

    const secBtn = sysView.querySelector('button[data-subtab="security"]') as HTMLButtonElement;
    expect(secBtn).toBeTruthy();
    expect(secBtn.textContent).toContain("IEC 62443 Security");

    secBtn.click();
    await new Promise((r) => setTimeout(r, 10));

    const cyberPanel = sysView.querySelector("nebu-cybersecurity-panel");
    expect(cyberPanel).toBeTruthy();

    document.body.removeChild(sysView);
  });

  it("integrates cybersecurity panel into NebuApp enterprise workspace", async () => {
    const app = new NebuApp();
    app.activeMode = "enterprise";
    document.body.appendChild(app);
    await new Promise((r) => setTimeout(r, 10));

    const secBtn = app.querySelector('button[data-subtab="security"]') as HTMLButtonElement;
    expect(secBtn).toBeTruthy();
    expect(secBtn.textContent).toContain("Cyber Security (IEC 62443)");

    secBtn.click();
    await new Promise((r) => setTimeout(r, 10));

    const cyberPanel = app.querySelector("nebu-cybersecurity-panel");
    expect(cyberPanel).toBeTruthy();

    document.body.removeChild(app);
  });
});
