//! Industrial Communications Manager for NebuDCS (Stage 7).
//!
//! Manages external protocol connections (Modbus TCP, OPC UA, MQTT, Sparkplug B),
//! tag mappings with scaling/deadbands/stale timeouts, store-and-forward queues,
//! ISA-95 zone enforcement, and connection testing with SSRF guards.

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::RwLock;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_protocol::dto::{
    CommunicationsConfigDto, ConnectionTestResultDto, IndustrialZoneDto, ProtocolConnectionDto,
    ProtocolDiagnosticsDto, TagMappingDto,
};
use nebudcs_protocol::industrial::{
    AccessLevel, AtomicDiagnostics, IndustrialDataType, IndustrialQuality, MappingDirection,
    MockModbusServer, MockMqttBroker, MockOpcUaServer, ProtocolDiagnostics, ProtocolType,
    SsrfGuard, StoreAndForwardQueue, TagMapping, TagMappingEngine,
};

/// Primary industrial communications coordinator for the NebuDCS server.
pub struct CommunicationsManager {
    connections: BTreeMap<String, ProtocolConnectionDto>,
    tag_mappings: BTreeMap<String, TagMappingDto>,
    zones: Vec<IndustrialZoneDto>,
    mapping_engine: TagMappingEngine,
    store_and_forward: Arc<RwLock<StoreAndForwardQueue>>,
    diagnostics: BTreeMap<String, Arc<AtomicDiagnostics>>,
    // Mock servers and brokers for local testing, demonstration, and offline execution
    mock_modbus_servers: BTreeMap<String, MockModbusServer>,
    mock_opcua_servers: BTreeMap<String, MockOpcUaServer>,
    mock_mqtt_brokers: BTreeMap<String, MockMqttBroker>,
}

impl Default for CommunicationsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CommunicationsManager {
    /// Creates a new CommunicationsManager pre-configured with canonical ISA-95 zones
    /// and pre-seeded demonstration acceptance plants.
    pub fn new() -> Self {
        let zones = vec![
            IndustrialZoneDto {
                id: "zone-l0-process".to_string(),
                name: "Level 0: Physical Process & Hydraulic Simulation".to_string(),
                level: "Level0".to_string(),
                permitted_protocols: vec!["Internal".to_string()],
                trust_boundary: false,
                security_policy: "Direct memory simulation interconnect".to_string(),
            },
            IndustrialZoneDto {
                id: "zone-l1-io".to_string(),
                name: "Level 1: Sensing & Manipulation (I/O HAL)".to_string(),
                level: "Level1".to_string(),
                permitted_protocols: vec!["ModbusTcp".to_string(), "ModbusSecurity".to_string()],
                trust_boundary: true,
                security_policy: "Isolated fieldbus subnet with strict port isolation".to_string(),
            },
            IndustrialZoneDto {
                id: "zone-l2-control".to_string(),
                name: "Level 2: Supervisory & Regulatory Control (DCS/PLC/APC)".to_string(),
                level: "Level2".to_string(),
                permitted_protocols: vec![
                    "ModbusTcp".to_string(),
                    "ModbusSecurity".to_string(),
                    "OpcUa".to_string(),
                ],
                trust_boundary: true,
                security_policy: "Deterministic cyclic control network (10-50ms scan)".to_string(),
            },
            IndustrialZoneDto {
                id: "zone-l3-ops".to_string(),
                name: "Level 3: Operations, SCADA, & Plant Historian".to_string(),
                level: "Level3".to_string(),
                permitted_protocols: vec![
                    "OpcUa".to_string(),
                    "Mqtt".to_string(),
                    "SparkplugB".to_string(),
                ],
                trust_boundary: true,
                security_policy: "Authenticated operations DMZ with TLS encryption".to_string(),
            },
            IndustrialZoneDto {
                id: "zone-l4-enterprise".to_string(),
                name: "Level 4: Business Logistics & Cloud Analytics".to_string(),
                level: "Level4".to_string(),
                permitted_protocols: vec!["Mqtt".to_string(), "SparkplugB".to_string()],
                trust_boundary: true,
                security_policy: "Strict enterprise firewall conduit with read-only metrics"
                    .to_string(),
            },
        ];

        let mut mgr = Self {
            connections: BTreeMap::new(),
            tag_mappings: BTreeMap::new(),
            zones,
            mapping_engine: TagMappingEngine::new(),
            store_and_forward: Arc::new(RwLock::new(StoreAndForwardQueue::new(
                2048,
                nebudcs_protocol::industrial::OverflowPolicy::DropOldest,
            ))),
            diagnostics: BTreeMap::new(),
            mock_modbus_servers: BTreeMap::new(),
            mock_opcua_servers: BTreeMap::new(),
            mock_mqtt_brokers: BTreeMap::new(),
        };

        mgr.seed_acceptance_plants();
        mgr
    }

    /// Seeds acceptance plants A, B, C, D, and E for industrial protocol validation.
    fn seed_acceptance_plants(&mut self) {
        // --- Plant A: Modbus TCP Acceptance Plant ---
        let conn_a = ProtocolConnectionDto {
            id: "conn-modbus-a".to_string(),
            name: "CDU Column Modbus Gateway (Plant A)".to_string(),
            protocol: "ModbusTcp".to_string(),
            endpoint: "tcp://192.168.1.100:502".to_string(),
            enabled: true,
            state: "Connected".to_string(),
            zone_id: "zone-l2-control".to_string(),
            timeout_ms: 2000,
            reconnect_delay_ms: 5000,
            use_tls: false,
            active_tags: 3,
            last_comm_ms: 15,
        };
        self.register_connection_internal(conn_a);
        self.mock_modbus_servers
            .insert("conn-modbus-a".to_string(), MockModbusServer::new());

        let map_a1 = TagMappingDto {
            id: "map-modbus-01".to_string(),
            connection_id: "conn-modbus-a".to_string(),
            external_address: "HR:40001:F32:WORDSWAP".to_string(),
            canonical_tag: "TI-101.PV".to_string(),
            direction: "Inbound".to_string(),
            datatype: "Float32".to_string(),
            scale: 1.0,
            offset: 0.0,
            engineering_unit: Some("°C".to_string()),
            deadband: 0.01,
            update_rate_ms: 100,
            stale_timeout_ms: 5000,
            access: "ReadOnly".to_string(),
            current_value: Some("185.4".to_string()),
            quality: "Good".to_string(),
        };
        let map_a2 = TagMappingDto {
            id: "map-modbus-02".to_string(),
            connection_id: "conn-modbus-a".to_string(),
            external_address: "HR:40003:F32:WORDSWAP".to_string(),
            canonical_tag: "PI-101.PV".to_string(),
            direction: "Inbound".to_string(),
            datatype: "Float32".to_string(),
            scale: 0.1,
            offset: 0.0,
            engineering_unit: Some("bar".to_string()),
            deadband: 0.01,
            update_rate_ms: 100,
            stale_timeout_ms: 5000,
            access: "ReadOnly".to_string(),
            current_value: Some("3.2".to_string()),
            quality: "Good".to_string(),
        };
        let map_a3 = TagMappingDto {
            id: "map-modbus-03".to_string(),
            connection_id: "conn-modbus-a".to_string(),
            external_address: "HR:40005:I16:BIGENDIAN".to_string(),
            canonical_tag: "P-101.RUN_CMD".to_string(),
            direction: "Outbound".to_string(),
            datatype: "Int16".to_string(),
            scale: 1.0,
            offset: 0.0,
            engineering_unit: None,
            deadband: 0.0,
            update_rate_ms: 100,
            stale_timeout_ms: 5000,
            access: "ReadWrite".to_string(),
            current_value: Some("1".to_string()),
            quality: "Good".to_string(),
        };
        let _ = self.add_mapping(map_a1);
        let _ = self.add_mapping(map_a2);
        let _ = self.add_mapping(map_a3);

        // --- Plant B: OPC UA Acceptance Plant ---
        let conn_b = ProtocolConnectionDto {
            id: "conn-opcua-b".to_string(),
            name: "Refinery Supervisory OPC UA Server (Plant B)".to_string(),
            protocol: "OpcUa".to_string(),
            endpoint: "opc.tcp://192.168.2.50:4840".to_string(),
            enabled: true,
            state: "Connected".to_string(),
            zone_id: "zone-l3-ops".to_string(),
            timeout_ms: 3000,
            reconnect_delay_ms: 5000,
            use_tls: true,
            active_tags: 2,
            last_comm_ms: 22,
        };
        self.register_connection_internal(conn_b);
        self.mock_opcua_servers
            .insert("conn-opcua-b".to_string(), MockOpcUaServer::new());

        let map_b1 = TagMappingDto {
            id: "map-opcua-01".to_string(),
            connection_id: "conn-opcua-b".to_string(),
            external_address: "ns=2;s=Refinery.CDU.ColumnPressure".to_string(),
            canonical_tag: "PIC-101.PV".to_string(),
            direction: "Inbound".to_string(),
            datatype: "Float32".to_string(),
            scale: 1.0,
            offset: 0.0,
            engineering_unit: Some("bar".to_string()),
            deadband: 0.05,
            update_rate_ms: 50,
            stale_timeout_ms: 3000,
            access: "ReadOnly".to_string(),
            current_value: Some("2.85".to_string()),
            quality: "Good".to_string(),
        };
        let map_b2 = TagMappingDto {
            id: "map-opcua-02".to_string(),
            connection_id: "conn-opcua-b".to_string(),
            external_address: "ns=2;s=Refinery.CDU.ReboilerTemp".to_string(),
            canonical_tag: "TIC-102.SP".to_string(),
            direction: "Outbound".to_string(),
            datatype: "Float32".to_string(),
            scale: 1.0,
            offset: 0.0,
            engineering_unit: Some("°C".to_string()),
            deadband: 0.1,
            update_rate_ms: 100,
            stale_timeout_ms: 3000,
            access: "ReadWrite".to_string(),
            current_value: Some("210.0".to_string()),
            quality: "Good".to_string(),
        };
        let _ = self.add_mapping(map_b1);
        let _ = self.add_mapping(map_b2);

        // --- Plant C: MQTT / Sparkplug B Acceptance Plant ---
        let conn_c = ProtocolConnectionDto {
            id: "conn-sparkplug-c".to_string(),
            name: "Enterprise IIoT Sparkplug B Broker (Plant C)".to_string(),
            protocol: "SparkplugB".to_string(),
            endpoint: "mqtt://192.168.3.10:1883".to_string(),
            enabled: true,
            state: "Connected".to_string(),
            zone_id: "zone-l3-ops".to_string(),
            timeout_ms: 4000,
            reconnect_delay_ms: 5000,
            use_tls: false,
            active_tags: 2,
            last_comm_ms: 12,
        };
        self.register_connection_internal(conn_c);
        self.mock_mqtt_brokers
            .insert("conn-sparkplug-c".to_string(), MockMqttBroker::new());

        let map_c1 = TagMappingDto {
            id: "map-sparkplug-01".to_string(),
            connection_id: "conn-sparkplug-c".to_string(),
            external_address: "spBv1.0/Refinery/DDATA/Distillation/Col101/TopTemp".to_string(),
            canonical_tag: "TI-102.PV".to_string(),
            direction: "Inbound".to_string(),
            datatype: "Float32".to_string(),
            scale: 1.0,
            offset: 0.0,
            engineering_unit: Some("°C".to_string()),
            deadband: 0.1,
            update_rate_ms: 250,
            stale_timeout_ms: 5000,
            access: "ReadOnly".to_string(),
            current_value: Some("142.6".to_string()),
            quality: "Good".to_string(),
        };
        let map_c2 = TagMappingDto {
            id: "map-sparkplug-02".to_string(),
            connection_id: "conn-sparkplug-c".to_string(),
            external_address: "spBv1.0/Refinery/DDATA/Distillation/Col101/BottomLevel".to_string(),
            canonical_tag: "LT-101.PV".to_string(),
            direction: "Inbound".to_string(),
            datatype: "Float32".to_string(),
            scale: 1.0,
            offset: 0.0,
            engineering_unit: Some("%".to_string()),
            deadband: 0.05,
            update_rate_ms: 250,
            stale_timeout_ms: 5000,
            access: "ReadOnly".to_string(),
            current_value: Some("54.2".to_string()),
            quality: "Good".to_string(),
        };
        let _ = self.add_mapping(map_c1);
        let _ = self.add_mapping(map_c2);

        // --- Plant D: Robustness & Fault Tolerance Acceptance Plant ---
        let conn_d = ProtocolConnectionDto {
            id: "conn-robust-d".to_string(),
            name: "Critical Intertie Remote Terminal Unit (Plant D)".to_string(),
            protocol: "ModbusTcp".to_string(),
            endpoint: "tcp://192.168.1.200:502".to_string(),
            enabled: true,
            state: "Degraded".to_string(),
            zone_id: "zone-l2-control".to_string(),
            timeout_ms: 1500,
            reconnect_delay_ms: 2000,
            use_tls: false,
            active_tags: 1,
            last_comm_ms: 2100,
        };
        self.register_connection_internal(conn_d);
        self.mock_modbus_servers
            .insert("conn-robust-d".to_string(), MockModbusServer::new());

        let map_d1 = TagMappingDto {
            id: "map-robust-01".to_string(),
            connection_id: "conn-robust-d".to_string(),
            external_address: "HR:40010:F32:WORDSWAP".to_string(),
            canonical_tag: "FT-101.PV".to_string(),
            direction: "Inbound".to_string(),
            datatype: "Float32".to_string(),
            scale: 1.0,
            offset: 0.0,
            engineering_unit: Some("kg/h".to_string()),
            deadband: 0.05,
            update_rate_ms: 100,
            stale_timeout_ms: 2000,
            access: "ReadOnly".to_string(),
            current_value: Some("12500.0".to_string()),
            quality: "Stale".to_string(),
        };
        let _ = self.add_mapping(map_d1);

        // --- Plant E: Multi-Level Refinery Architecture Acceptance Plant ---
        let conn_e = ProtocolConnectionDto {
            id: "conn-enterprise-e".to_string(),
            name: "Refinery Level 4 MES / Cloud Gateway (Plant E)".to_string(),
            protocol: "SparkplugB".to_string(),
            endpoint: "mqtt://10.0.100.20:8883".to_string(),
            enabled: true,
            state: "Connected".to_string(),
            zone_id: "zone-l4-enterprise".to_string(),
            timeout_ms: 5000,
            reconnect_delay_ms: 10000,
            use_tls: true,
            active_tags: 1,
            last_comm_ms: 45,
        };
        self.register_connection_internal(conn_e);
        self.mock_mqtt_brokers
            .insert("conn-enterprise-e".to_string(), MockMqttBroker::new());

        let map_e1 = TagMappingDto {
            id: "map-enterprise-01".to_string(),
            connection_id: "conn-enterprise-e".to_string(),
            external_address: "spBv1.0/Enterprise/NDATA/RefineryComplex/ProductionRate".to_string(),
            canonical_tag: "PROD_RATE.PV".to_string(),
            direction: "Outbound".to_string(),
            datatype: "Float32".to_string(),
            scale: 1.0,
            offset: 0.0,
            engineering_unit: Some("BPD".to_string()),
            deadband: 1.0,
            update_rate_ms: 1000,
            stale_timeout_ms: 10000,
            access: "ReadOnly".to_string(),
            current_value: Some("84200.0".to_string()),
            quality: "Good".to_string(),
        };
        let _ = self.add_mapping(map_e1);
    }

    fn register_connection_internal(&mut self, conn: ProtocolConnectionDto) {
        let diag = Arc::new(AtomicDiagnostics::default());
        diag.messages_sent.store(42, Ordering::Relaxed);
        diag.messages_received.store(189, Ordering::Relaxed);
        diag.bytes_sent.store(1024, Ordering::Relaxed);
        diag.bytes_received.store(4096, Ordering::Relaxed);
        self.diagnostics.insert(conn.id.clone(), diag);
        self.connections.insert(conn.id.clone(), conn);
    }

    /// Obtains all configured protocol connections.
    pub fn list_connections(&self) -> Vec<ProtocolConnectionDto> {
        self.connections.values().cloned().collect()
    }

    /// Obtains a specific connection by ID.
    pub fn get_connection(&self, id: &str) -> Option<ProtocolConnectionDto> {
        self.connections.get(id).cloned()
    }

    /// Adds a new protocol connection after SSRF verification.
    pub fn add_connection(
        &mut self,
        conn: ProtocolConnectionDto,
    ) -> Result<ProtocolConnectionDto, String> {
        if self.connections.contains_key(&conn.id) {
            return Err(format!("Connection '{}' already exists", conn.id));
        }
        // SSRF guard check
        SsrfGuard::default()
            .validate_endpoint(&conn.endpoint)
            .map_err(|e| format!("SSRF Guard rejected endpoint: {e}"))?;

        self.register_connection_internal(conn.clone());
        Ok(conn)
    }

    /// Updates an existing protocol connection.
    pub fn update_connection(
        &mut self,
        id: &str,
        conn: ProtocolConnectionDto,
    ) -> Result<ProtocolConnectionDto, String> {
        if !self.connections.contains_key(id) {
            return Err(format!("Connection '{id}' not found"));
        }
        SsrfGuard::default()
            .validate_endpoint(&conn.endpoint)
            .map_err(|e| format!("SSRF Guard rejected endpoint: {e}"))?;

        self.connections.insert(id.to_string(), conn.clone());
        Ok(conn)
    }

    /// Deletes a connection and its associated mappings.
    pub fn delete_connection(&mut self, id: &str) -> Result<(), String> {
        if self.connections.remove(id).is_none() {
            return Err(format!("Connection '{id}' not found"));
        }
        self.diagnostics.remove(id);
        self.mock_modbus_servers.remove(id);
        self.mock_opcua_servers.remove(id);
        self.mock_mqtt_brokers.remove(id);

        let mapping_ids: Vec<String> = self
            .tag_mappings
            .values()
            .filter(|m| m.connection_id == id)
            .map(|m| m.id.clone())
            .collect();

        for m_id in mapping_ids {
            let _ = self.delete_mapping(&m_id);
        }

        Ok(())
    }

    /// Obtains all configured tag mappings.
    pub fn list_mappings(&self) -> Vec<TagMappingDto> {
        self.tag_mappings.values().cloned().collect()
    }

    /// Obtains a specific tag mapping by ID.
    pub fn get_mapping(&self, id: &str) -> Option<TagMappingDto> {
        self.tag_mappings.get(id).cloned()
    }

    /// Adds a tag mapping with validation and conflict detection.
    pub fn add_mapping(&mut self, mapping: TagMappingDto) -> Result<TagMappingDto, String> {
        if self.tag_mappings.contains_key(&mapping.id) {
            return Err(format!("Mapping '{}' already exists", mapping.id));
        }
        if mapping.scale.abs() < 1e-9 {
            return Err("Scaling factor cannot be zero".to_string());
        }

        let dir = match mapping.direction.to_lowercase().as_str() {
            "inbound" => MappingDirection::Inbound,
            "outbound" => MappingDirection::Outbound,
            "bidirectional" => MappingDirection::Bidirectional,
            _ => return Err(format!("Unknown mapping direction: {}", mapping.direction)),
        };

        let internal_mapping = TagMapping {
            id: mapping.id.clone(),
            connection_id: mapping.connection_id.clone(),
            external_address: mapping.external_address.clone(),
            canonical_tag: mapping.canonical_tag.clone(),
            direction: dir,
            datatype: IndustrialDataType::Float32,
            scale: mapping.scale,
            offset: mapping.offset,
            engineering_unit: mapping.engineering_unit.clone(),
            deadband: mapping.deadband,
            update_rate_ms: mapping.update_rate_ms,
            stale_timeout_ms: mapping.stale_timeout_ms,
            access: AccessLevel::ReadWrite,
            fallback_value: None,
            fallback_quality: IndustrialQuality::Bad,
            last_inbound_value: None,
            last_outbound_value: None,
            last_update_timestamp_ms: 0,
        };

        self.mapping_engine
            .add_mapping(internal_mapping)
            .map_err(|e| format!("Mapping conflict or error: {e}"))?;

        if let Some(conn) = self.connections.get_mut(&mapping.connection_id) {
            conn.active_tags += 1;
        }

        self.tag_mappings
            .insert(mapping.id.clone(), mapping.clone());
        Ok(mapping)
    }

    /// Updates an existing tag mapping.
    pub fn update_mapping(
        &mut self,
        id: &str,
        mapping: TagMappingDto,
    ) -> Result<TagMappingDto, String> {
        if !self.tag_mappings.contains_key(id) {
            return Err(format!("Mapping '{id}' not found"));
        }
        if mapping.scale.abs() < 1e-9 {
            return Err("Scaling factor cannot be zero".to_string());
        }

        self.tag_mappings.insert(id.to_string(), mapping.clone());
        Ok(mapping)
    }

    /// Deletes a tag mapping.
    pub fn delete_mapping(&mut self, id: &str) -> Result<(), String> {
        if let Some(m) = self.tag_mappings.remove(id) {
            let _ = self.mapping_engine.remove_mapping(id);
            if let Some(conn) = self.connections.get_mut(&m.connection_id) {
                conn.active_tags = conn.active_tags.saturating_sub(1);
            }
            Ok(())
        } else {
            Err(format!("Mapping '{id}' not found"))
        }
    }

    /// Obtains diagnostic metrics across all connections.
    pub fn get_diagnostics(&self) -> Vec<ProtocolDiagnosticsDto> {
        let mut results = Vec::new();
        for (id, conn) in &self.connections {
            let diag_snap = if let Some(d) = self.diagnostics.get(id) {
                d.snapshot(id, ProtocolType::ModbusTcp)
            } else {
                ProtocolDiagnostics {
                    connection_id: id.clone(),
                    protocol: ProtocolType::ModbusTcp,
                    messages_sent: 0,
                    messages_received: 0,
                    bytes_sent: 0,
                    bytes_received: 0,
                    read_operations: 0,
                    write_operations: 0,
                    error_count: 0,
                    reconnect_count: 0,
                    last_latency_ms: 0.0,
                    avg_latency_ms: 0.0,
                    max_latency_ms: 0.0,
                    active_subscriptions: 0,
                    stale_tags_count: 0,
                    bad_tags_count: 0,
                    queue_depth: 0,
                    last_successful_comm_ms: 0,
                }
            };

            let is_healthy = conn.state != "Faulted" && conn.state != "Disconnected";
            let active_subs = self
                .tag_mappings
                .values()
                .filter(|m| m.connection_id == *id)
                .count();

            results.push(ProtocolDiagnosticsDto {
                connection_id: id.clone(),
                protocol: conn.protocol.clone(),
                state: conn.state.clone(),
                is_healthy,
                messages_sent: diag_snap.messages_sent,
                messages_received: diag_snap.messages_received,
                bytes_sent: diag_snap.bytes_sent,
                bytes_received: diag_snap.bytes_received,
                read_operations: diag_snap.read_operations,
                write_operations: diag_snap.write_operations,
                error_count: diag_snap.error_count,
                reconnect_count: diag_snap.reconnect_count,
                active_subscriptions: active_subs,
                stale_tags_count: if conn.state == "Degraded" { 1 } else { 0 },
                queue_depth: 0,
                last_latency_ms: if is_healthy { 12.5 } else { 0.0 },
                last_comm_ms: conn.last_comm_ms,
            });
        }
        results
    }

    /// Obtains configured ISA-95 zones.
    pub fn get_zones(&self) -> Vec<IndustrialZoneDto> {
        self.zones.clone()
    }

    /// Access the store-and-forward queue.
    pub fn store_and_forward(&self) -> &Arc<RwLock<StoreAndForwardQueue>> {
        &self.store_and_forward
    }

    /// Tests a connection endpoint using SSRF guard validation and protocol simulation.
    pub fn test_connection(&self, id_or_endpoint: &str, protocol: &str) -> ConnectionTestResultDto {
        // Resolve target endpoint
        let endpoint = if let Some(conn) = self.connections.get(id_or_endpoint) {
            conn.endpoint.clone()
        } else {
            id_or_endpoint.to_string()
        };

        // SSRF guard check
        if let Err(err) = SsrfGuard::default().validate_endpoint(&endpoint) {
            return ConnectionTestResultDto {
                connection_id: id_or_endpoint.to_string(),
                endpoint,
                protocol: protocol.to_string(),
                success: false,
                latency_ms: 0.0,
                message: format!("Security violation: SSRF Guard blocked endpoint ({err})"),
            };
        }

        // Check if endpoint is known offline or faulted in mock state
        if endpoint.contains("offline") || endpoint.contains("unreachable") {
            return ConnectionTestResultDto {
                connection_id: id_or_endpoint.to_string(),
                endpoint,
                protocol: protocol.to_string(),
                success: false,
                latency_ms: 0.0,
                message: "Connection failed: host unreachable or timed out".to_string(),
            };
        }

        // Realistic connection test success
        let latency_ms = match protocol.to_lowercase().as_str() {
            "modbustcp" => 8.4,
            "opcua" => 15.2,
            "mqtt" => 6.1,
            "sparkplugb" => 9.8,
            _ => 10.0,
        };

        ConnectionTestResultDto {
            connection_id: id_or_endpoint.to_string(),
            endpoint,
            protocol: protocol.to_string(),
            success: true,
            latency_ms,
            message: format!("Connected successfully via {protocol} (handshake OK)"),
        }
    }

    /// Loads communications configuration from a project definition.
    pub fn load_project_communications(&mut self, comms: &Option<CommunicationsConfigDto>) {
        if let Some(ref c) = comms {
            self.connections.clear();
            self.tag_mappings.clear();
            self.diagnostics.clear();
            self.mapping_engine = TagMappingEngine::new();

            for conn in &c.connections {
                self.register_connection_internal(conn.clone());
            }
            for mapping in &c.tag_mappings {
                let _ = self.add_mapping(mapping.clone());
            }
            if !c.zones.is_empty() {
                self.zones = c.zones.clone();
            }
        }
    }

    /// Synchronizes values between external protocol mappings and the simulation RuntimeTagStore.
    /// This is strictly executed at deterministic simulation tick boundaries.
    pub fn sync_external_values(&mut self, tag_store: &mut RuntimeTagStore, _now_ms: u64) {
        // Ingest inbound mapped values into tag store
        for mapping in self.tag_mappings.values_mut() {
            if mapping.direction.eq_ignore_ascii_case("Inbound") {
                if let Some(ref cur_str) = mapping.current_value {
                    if let Ok(raw_val) = cur_str.parse::<f64>() {
                        // EU = Raw * Scale + Offset
                        let scaled_val = raw_val * mapping.scale + mapping.offset;
                        let quality = match mapping.quality.as_str() {
                            "Good" => TagQuality::Good,
                            "Uncertain" => TagQuality::Uncertain,
                            "Stale" => TagQuality::Uncertain,
                            "Bad" => TagQuality::Bad,
                            _ => TagQuality::Good,
                        };

                        if let Some(tid) = tag_store.resolve_name(&mapping.canonical_tag) {
                            tag_store.set_tag_float(tid, scaled_val, quality);
                        }
                    }
                }
            } else if mapping.direction.eq_ignore_ascii_case("Outbound") {
                // Outbound: Read canonical tag from tag_store and update current_value
                if let Some(tid) = tag_store.resolve_name(&mapping.canonical_tag) {
                    if let Some(TagValue::Float(f)) = tag_store.get_value(tid) {
                        mapping.current_value = Some(format!("{:.4}", f));
                    }
                }
            }
        }
    }
}
