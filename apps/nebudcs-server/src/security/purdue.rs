//! Purdue Enterprise Reference Architecture (PERA) & IEC 62443-3-2 Zones & Conduits Engine.
//!
//! Enforces network segregation, conduit packet inspection, and protocol white-listing
//! across Level 0 (Process) through Level 5 (Enterprise).

use nebudcs_protocol::{PurdueConduitDto, PurdueZoneDto, UpdateConduitPolicyRequestDto};
use std::collections::HashMap;

/// Purdue Model Zone and Conduit security policy engine.
#[derive(Debug, Clone)]
pub struct PurdueSecurityEngine {
    zones: HashMap<String, PurdueZoneDto>,
    conduits: HashMap<String, PurdueConduitDto>,
}

impl Default for PurdueSecurityEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PurdueSecurityEngine {
    pub fn new() -> Self {
        let mut zones = HashMap::new();
        let mut conduits = HashMap::new();

        // Level 0: Physical Process
        zones.insert(
            "zone-l0-process".to_string(),
            PurdueZoneDto {
                zone_id: "zone-l0-process".to_string(),
                name: "Level 0: Physical Process".to_string(),
                level_name: "Level 0".to_string(),
                purdue_level: 0,
                target_security_level: "SL-1".to_string(),
                achieved_security_level: "SL-1".to_string(),
                assets_count: 12,
                status: "SECURE".to_string(),
                trust_boundary: false,
                description: "Physical process units: distillation columns, vessels, reactors, pumps, valves, fired heaters".to_string(),
                assigned_roles: vec!["Engineer".to_string()],
            },
        );

        // Level 1: Field Sensing & Manipulation
        zones.insert(
            "zone-l1-io".to_string(),
            PurdueZoneDto {
                zone_id: "zone-l1-io".to_string(),
                name: "Level 1: Basic Sensing & Actuation (I/O)".to_string(),
                level_name: "Level 1".to_string(),
                purdue_level: 1,
                target_security_level: "SL-2".to_string(),
                achieved_security_level: "SL-2".to_string(),
                assets_count: 32,
                status: "SECURE".to_string(),
                trust_boundary: true,
                description: "Field instruments: transmitters (FT, PT, TT, LT), valve positioners, remote I/O chassis".to_string(),
                assigned_roles: vec!["Engineer".to_string()],
            },
        );

        // Level 2: Realtime Control & SIS
        zones.insert(
            "zone-l2-control".to_string(),
            PurdueZoneDto {
                zone_id: "zone-l2-control".to_string(),
                name: "Level 2: Supervisory Control & Safety".to_string(),
                level_name: "Level 2".to_string(),
                purdue_level: 2,
                target_security_level: "SL-3".to_string(),
                achieved_security_level: "SL-3".to_string(),
                assets_count: 8,
                status: "SECURE".to_string(),
                trust_boundary: true,
                description: "Authoritative deterministic execution core: Virtual DCS controllers, SIS safety matrix, IEC 61131-3 logic".to_string(),
                assigned_roles: vec!["Engineer".to_string(), "Operator".to_string(), "Trainee".to_string()],
            },
        );

        // Level 3: Operations & SCADA
        zones.insert(
            "zone-l3-ops".to_string(),
            PurdueZoneDto {
                zone_id: "zone-l3-ops".to_string(),
                name: "Level 3: Operations & SCADA".to_string(),
                level_name: "Level 3".to_string(),
                purdue_level: 3,
                target_security_level: "SL-3".to_string(),
                achieved_security_level: "SL-3".to_string(),
                assets_count: 6,
                status: "SECURE".to_string(),
                trust_boundary: true,
                description: "Human-Machine Interfaces: High-Performance DCS Operator Station, Engineering Studio, SQLite WAL Historian, OTS Trainer".to_string(),
                assigned_roles: vec!["Admin".to_string(), "Engineer".to_string(), "Instructor".to_string(), "Operator".to_string(), "Trainee".to_string()],
            },
        );

        // Level 3.5: Industrial DMZ
        zones.insert(
            "zone-l35-idmz".to_string(),
            PurdueZoneDto {
                zone_id: "zone-l35-idmz".to_string(),
                name: "Level 3.5: Industrial DMZ".to_string(),
                level_name: "Level 3.5".to_string(),
                purdue_level: 4,
                target_security_level: "SL-3".to_string(),
                achieved_security_level: "SL-3".to_string(),
                assets_count: 3,
                status: "SECURE".to_string(),
                trust_boundary: true,
                description: "Security perimeter: Reverse proxy, TLS 1.3 terminator, token-bucket rate limiter, SSRF firewall".to_string(),
                assigned_roles: vec!["Admin".to_string()],
            },
        );

        // Level 4/5: Enterprise Business & Cloud
        zones.insert(
            "zone-l4-enterprise".to_string(),
            PurdueZoneDto {
                zone_id: "zone-l4-enterprise".to_string(),
                name: "Level 4/5: Enterprise & Cloud Analytics".to_string(),
                level_name: "Level 4/5".to_string(),
                purdue_level: 5,
                target_security_level: "SL-2".to_string(),
                achieved_security_level: "SL-2".to_string(),
                assets_count: 4,
                status: "SECURE".to_string(),
                trust_boundary: true,
                description: "External enterprise systems: ERP, cloud time-series historians, remote analytics (Read-only access)".to_string(),
                assigned_roles: vec!["Admin".to_string()],
            },
        );

        // Conduits connecting the zones
        conduits.insert(
            "cnd-l35-l3".to_string(),
            PurdueConduitDto {
                conduit_id: "cnd-l35-l3".to_string(),
                name: "DMZ to Operations Console Conduit".to_string(),
                source_zone_id: "zone-l35-idmz".to_string(),
                destination_zone_id: "zone-l3-ops".to_string(),
                allowed_protocols: vec!["HTTPS".to_string(), "WSS".to_string()],
                security_level_required: "SL-3".to_string(),
                is_encrypted: true,
                packets_inspected: 14250,
                violations_blocked: 0,
                status: "ACTIVE".to_string(),
                policy_description: "Encrypted WebSocket state streaming and REST administrative requests".to_string(),
            },
        );

        conduits.insert(
            "cnd-l3-l2".to_string(),
            PurdueConduitDto {
                conduit_id: "cnd-l3-l2".to_string(),
                name: "Operations to Control Core Conduit".to_string(),
                source_zone_id: "zone-l3-ops".to_string(),
                destination_zone_id: "zone-l2-control".to_string(),
                allowed_protocols: vec!["INTERNAL_DISPATCH".to_string(), "OPC_UA".to_string()],
                security_level_required: "SL-3".to_string(),
                is_encrypted: true,
                packets_inspected: 48920,
                violations_blocked: 0,
                status: "ACTIVE".to_string(),
                policy_description: "Deterministic runtime command queue with strict SBO confirmation and role checking".to_string(),
            },
        );

        conduits.insert(
            "cnd-l2-l1".to_string(),
            PurdueConduitDto {
                conduit_id: "cnd-l2-l1".to_string(),
                name: "Control to Field I/O Chassis Conduit".to_string(),
                source_zone_id: "zone-l2-control".to_string(),
                destination_zone_id: "zone-l1-io".to_string(),
                allowed_protocols: vec!["IO_BUS".to_string(), "MODBUS_TCP".to_string()],
                security_level_required: "SL-2".to_string(),
                is_encrypted: false,
                packets_inspected: 124800,
                violations_blocked: 0,
                status: "ACTIVE".to_string(),
                policy_description: "Real-time cyclic input/output memory mapping with NAMUR NE43 wire-break diagnostics".to_string(),
            },
        );

        conduits.insert(
            "cnd-l1-l0".to_string(),
            PurdueConduitDto {
                conduit_id: "cnd-l1-l0".to_string(),
                name: "Field I/O to Physical Process Medium".to_string(),
                source_zone_id: "zone-l1-io".to_string(),
                destination_zone_id: "zone-l0-process".to_string(),
                allowed_protocols: vec!["PHYSICAL_SIGNAL".to_string()],
                security_level_required: "SL-1".to_string(),
                is_encrypted: false,
                packets_inspected: 249600,
                violations_blocked: 0,
                status: "ACTIVE".to_string(),
                policy_description: "Physical coupling between transmitters, control valves, and process fluid dynamics".to_string(),
            },
        );

        conduits.insert(
            "cnd-l3-l4".to_string(),
            PurdueConduitDto {
                conduit_id: "cnd-l3-l4".to_string(),
                name: "Operations to Enterprise Historian Diode".to_string(),
                source_zone_id: "zone-l3-ops".to_string(),
                destination_zone_id: "zone-l4-enterprise".to_string(),
                allowed_protocols: vec!["MQTT_SPARKPLUG_B".to_string(), "HTTPS".to_string()],
                security_level_required: "SL-2".to_string(),
                is_encrypted: true,
                packets_inspected: 8120,
                violations_blocked: 0,
                status: "ACTIVE".to_string(),
                policy_description: "Unidirectional read-only telemetry replication; zero inbound actuation commands permitted".to_string(),
            },
        );

        Self { zones, conduits }
    }

    pub fn get_zones(&self) -> Vec<PurdueZoneDto> {
        let mut list: Vec<_> = self.zones.values().cloned().collect();
        list.sort_by_key(|z| z.purdue_level);
        list
    }

    pub fn get_conduits(&self) -> Vec<PurdueConduitDto> {
        let mut list: Vec<_> = self.conduits.values().cloned().collect();
        list.sort_by(|a, b| a.conduit_id.cmp(&b.conduit_id));
        list
    }

    pub fn update_conduit_policy(&mut self, req: UpdateConduitPolicyRequestDto) -> bool {
        if let Some(cnd) = self.conduits.get_mut(&req.conduit_id) {
            cnd.allowed_protocols = req.allowed_protocols;
            cnd.is_encrypted = req.is_encrypted;
            cnd.status = req.status;
            true
        } else {
            false
        }
    }

    /// Inspects packet traversal across a conduit.
    /// Returns Ok if permitted, or Err with violation reason.
    pub fn inspect_traffic(
        &mut self,
        conduit_id: &str,
        protocol: &str,
        is_write: bool,
    ) -> Result<(), String> {
        let cnd = self
            .conduits
            .get_mut(conduit_id)
            .ok_or_else(|| format!("Conduit '{conduit_id}' not found in IEC 62443 topology"))?;

        if cnd.status != "ACTIVE" {
            cnd.violations_blocked += 1;
            return Err(format!("Conduit '{}' is not in ACTIVE state (current: {})", conduit_id, cnd.status));
        }

        let proto_upper = protocol.to_uppercase();
        let allowed = cnd.allowed_protocols.iter().any(|p| p.to_uppercase() == proto_upper);
        if !allowed {
            cnd.violations_blocked += 1;
            return Err(format!(
                "Protocol '{}' is prohibited on conduit '{}'. Permitted: {:?}",
                protocol, conduit_id, cnd.allowed_protocols
            ));
        }

        // Data diode rule: Level 3 to Level 4 conduit is strictly unidirectional read-only
        if conduit_id == "cnd-l3-l4" && is_write {
            cnd.violations_blocked += 1;
            return Err("Conduit 'cnd-l3-l4' enforces unidirectional data diode semantics; write commands are strictly blocked".to_string());
        }

        cnd.packets_inspected += 1;
        Ok(())
    }
}
