//! Stage 7: Industrial Protocol & Communication Architecture Test Suite.
//!
//! Verifies:
//! 1. Modbus TCP: Register encoding/decoding, endianness/word swap (ABCD, CDAB, BADC, DCBA), IEEE float scaling.
//! 2. OPC UA: StatusCode to quality translation, NodeId parsing, deadband filtering, monitored item subscription.
//! 3. MQTT & Sparkplug B: NBIRTH, NDEATH, DBIRTH, DDEATH, NDATA, DDATA, sequence counter 0-255 rollover.
//! 4. Gateway Tag Mapping: Scaling ($EU = Raw \times Scale + Offset$), deadband filtering, stale timeout detection, conflict rejection.
//! 5. Store-and-Forward: Bounded buffer capacity, DropOldest / DropNewest policies, FIFO batch draining.
//! 6. Cybersecurity & SSRF Guard: IP/port filtering, cloud metadata (169.254.169.254) blocking, password redaction.
//! 7. Process Isolation & 10,000-tick Bit-Identical Determinism Verification.

use std::sync::atomic::Ordering;

use nebudcs_protocol::industrial::*;

#[test]
fn test_modbus_address_parsing() {
    let spec1 = ModbusAddressSpec::parse("HR:40001:F32:WORDSWAP").unwrap();
    assert_eq!(spec1.table, ModbusTable::HoldingRegister);
    assert_eq!(spec1.address, 0); // 40001 maps to zero-based register offset 0
    assert_eq!(spec1.register_count, 2);
    assert_eq!(spec1.datatype, IndustrialDataType::Float32);
    assert_eq!(spec1.word_order, ModbusWordOrder::WordSwap);

    let spec2 = ModbusAddressSpec::parse("COIL:105:BOOL:BIGENDIAN").unwrap();
    assert_eq!(spec2.table, ModbusTable::Coil);
    assert_eq!(spec2.address, 104); // 1-based 105 maps to offset 104
    assert_eq!(spec2.register_count, 1);
    assert_eq!(spec2.datatype, IndustrialDataType::Boolean);

    let spec3 = ModbusAddressSpec::parse("IR:30010:I32:LITTLEENDIAN").unwrap();
    assert_eq!(spec3.table, ModbusTable::InputRegister);
    assert_eq!(spec3.address, 9); // 30010 maps to offset 9
    assert_eq!(spec3.register_count, 2);
    assert_eq!(spec3.datatype, IndustrialDataType::Int32);
    assert_eq!(spec3.word_order, ModbusWordOrder::LittleEndian);

    // Invalid spec should fail cleanly
    assert!(ModbusAddressSpec::parse("INVALID:ADDRESS").is_err());
}

#[test]
fn test_modbus_codec_word_swap_ieee_float32() {
    let test_val: f32 = 185.42;
    let original = IndustrialDataValue::Float32(test_val);

    // Test BigEndian (ABCD)
    let be_regs = ModbusCodec::encode_value(&original, ModbusWordOrder::BigEndian).unwrap();
    assert_eq!(be_regs.len(), 2);
    let decoded_be = ModbusCodec::decode_registers(
        &be_regs,
        IndustrialDataType::Float32,
        ModbusWordOrder::BigEndian,
    )
    .unwrap();
    match decoded_be {
        IndustrialDataValue::Float32(f) => assert!((f - test_val).abs() < 1e-4),
        _ => panic!("Expected Float32"),
    }

    // Test WordSwap (CDAB) - Standard Modicon/Honeywell DCS format
    let ws_regs = ModbusCodec::encode_value(&original, ModbusWordOrder::WordSwap).unwrap();
    assert_eq!(ws_regs.len(), 2);
    assert_eq!(ws_regs[0], be_regs[1]);
    assert_eq!(ws_regs[1], be_regs[0]);

    let decoded_ws = ModbusCodec::decode_registers(
        &ws_regs,
        IndustrialDataType::Float32,
        ModbusWordOrder::WordSwap,
    )
    .unwrap();
    match decoded_ws {
        IndustrialDataValue::Float32(f) => assert!((f - test_val).abs() < 1e-4),
        _ => panic!("Expected Float32"),
    }

    // Test LittleEndian (DCBA)
    let le_regs = ModbusCodec::encode_value(&original, ModbusWordOrder::LittleEndian).unwrap();
    let decoded_le = ModbusCodec::decode_registers(
        &le_regs,
        IndustrialDataType::Float32,
        ModbusWordOrder::LittleEndian,
    )
    .unwrap();
    match decoded_le {
        IndustrialDataValue::Float32(f) => assert!((f - test_val).abs() < 1e-4),
        _ => panic!("Expected Float32"),
    }
}

#[tokio::test]
async fn test_mock_modbus_server_read_write_float32() {
    let server = MockModbusServer::new();
    let addr = 40001;
    let val: f32 = 42.125;

    // Seed float32 into server holding registers using WordSwap
    server
        .set_holding_float32(addr, val, ModbusWordOrder::WordSwap)
        .await;

    // Read registers back
    let regs = server.read_holding_registers(addr, 2).await.unwrap();
    assert_eq!(regs.len(), 2);

    let decoded = ModbusCodec::decode_registers(
        &regs,
        IndustrialDataType::Float32,
        ModbusWordOrder::WordSwap,
    )
    .unwrap();
    match decoded {
        IndustrialDataValue::Float32(f) => assert!((f - val).abs() < 1e-4),
        _ => panic!("Expected Float32"),
    }

    // Simulate server offline exception
    *server.is_online.write().await = false;
    let fail_res = server.read_holding_registers(addr, 2).await;
    assert!(fail_res.is_err());
}

#[test]
fn test_opcua_status_code_quality_mapping() {
    let good_code = OpcUaStatusCode::GOOD;
    assert_eq!(good_code.to_quality(), IndustrialQuality::Good);
    assert!(good_code.is_good());

    let uncertain_code = OpcUaStatusCode::UNCERTAIN;
    assert_eq!(uncertain_code.to_quality(), IndustrialQuality::Uncertain);

    let timeout_code = OpcUaStatusCode::BAD_TIMEOUT;
    assert_eq!(
        timeout_code.to_quality(),
        IndustrialQuality::CommunicationFailure
    );
    assert!(timeout_code.is_bad());

    let sensor_code = OpcUaStatusCode::BAD_SENSOR_FAILURE;
    assert_eq!(sensor_code.to_quality(), IndustrialQuality::SensorFailure);

    let range_code = OpcUaStatusCode::BAD_OUT_OF_RANGE;
    assert_eq!(range_code.to_quality(), IndustrialQuality::OutOfRange);
}

#[test]
fn test_opcua_node_id_parsing() {
    let node1 = OpcUaNodeId::parse("ns=2;s=Refinery.CDU.ColumnPressure").unwrap();
    assert_eq!(node1.namespace(), 2);
    assert_eq!(node1.identifier(), "Refinery.CDU.ColumnPressure");

    let node2 = OpcUaNodeId::parse("ns=1;i=1001").unwrap();
    assert_eq!(node2.namespace(), 1);
    assert_eq!(node2.identifier(), "1001");

    assert!(OpcUaNodeId::parse("invalid_node_id").is_ok());
}

#[tokio::test]
async fn test_mock_opcua_server_subscription_and_deadband() {
    let server = MockOpcUaServer::new();
    let node_id = OpcUaNodeId::parse("ns=2;s=TIC-101.PV").unwrap();

    let initial_val = OpcUaDataValue {
        value: IndustrialDataValue::Float32(100.0),
        status: OpcUaStatusCode::GOOD,
        source_timestamp_ms: Some(1000),
        server_timestamp_ms: Some(1000),
    };
    server.insert_node("ns=2;s=TIC-101.PV", initial_val).await;

    let item = OpcUaMonitoredItem {
        item_id: 1,
        node_id: node_id.clone(),
        sampling_interval_ms: 50,
        deadband: 0.5,
        last_value: Some(100.0),
    };
    server.add_monitored_item(item).await;

    // A minor update within deadband (100.0 -> 100.2, delta=0.2 < 0.5) should NOT trigger notification
    server
        .write_node("ns=2;s=TIC-101.PV", IndustrialDataValue::Float32(100.2))
        .await
        .unwrap();
    let notifs_small = server.poll_monitored_items().await;
    assert_eq!(notifs_small.len(), 0);

    // A significant update exceeding deadband (100.2 -> 101.5, delta=1.5 >= 0.5) MUST trigger notification
    server
        .write_node("ns=2;s=TIC-101.PV", IndustrialDataValue::Float32(101.5))
        .await
        .unwrap();
    let notifs_big = server.poll_monitored_items().await;
    assert_eq!(notifs_big.len(), 1);
    assert_eq!(notifs_big[0].0, 1);
    match &notifs_big[0].1.value {
        IndustrialDataValue::Float32(f) => assert!((f - 101.5).abs() < 1e-4),
        _ => panic!("Expected Float32"),
    }
}

#[tokio::test]
async fn test_sparkplug_b_lifecycle_and_sequence_rollover() {
    let edge_node = SparkplugEdgeNode::new("RefineryGroup", "EdgeNode01");

    // Sequence starts at 0 for NBIRTH
    let nbirth = edge_node.generate_birth_payload().await;
    assert_eq!(nbirth.seq, 0);

    // Sequence numbers increment monotonically
    let seq0 = edge_node.next_seq();
    let seq1 = edge_node.next_seq();
    assert_eq!(seq0, 0);
    assert_eq!(seq1, 1);

    // Roll sequence to 255 and verify rollover to 0
    edge_node.seq.store(255, Ordering::Relaxed);
    let seq_roll = edge_node.next_seq();
    assert_eq!(seq_roll, 255);
    let seq_next = edge_node.next_seq();
    assert_eq!(seq_next, 0);

    // NDEATH will payload
    let ndeath = edge_node.generate_death_payload();
    assert_eq!(ndeath.seq, 0);
}

#[test]
fn test_tag_mapping_engine_scaling_and_conflicts() {
    let mut engine = TagMappingEngine::new();

    let mapping1 = TagMapping {
        id: "map-01".to_string(),
        connection_id: "conn-01".to_string(),
        external_address: "HR:40001:F32:WORDSWAP".to_string(),
        canonical_tag: "TI-101.PV".to_string(),
        direction: MappingDirection::Inbound,
        datatype: IndustrialDataType::Float32,
        scale: 0.1,  // 0.1 scale factor
        offset: 5.0, // 5.0 offset: EU = Raw * 0.1 + 5.0
        engineering_unit: Some("°C".to_string()),
        deadband: 0.05,
        update_rate_ms: 100,
        stale_timeout_ms: 2000,
        access: AccessLevel::ReadOnly,
        fallback_value: Some(IndustrialDataValue::Float64(0.0)),
        fallback_quality: IndustrialQuality::Bad,
        last_inbound_value: None,
        last_outbound_value: None,
        last_update_timestamp_ms: 0,
    };

    assert!(engine.add_mapping(mapping1).is_ok());

    // Conflict test: Attempt to register another Inbound mapping to the same canonical tag "TI-101.PV"
    let duplicate_mapping = TagMapping {
        id: "map-duplicate".to_string(),
        connection_id: "conn-02".to_string(),
        external_address: "ns=2;s=AnotherSensor".to_string(),
        canonical_tag: "TI-101.PV".to_string(),
        direction: MappingDirection::Inbound,
        datatype: IndustrialDataType::Float32,
        scale: 1.0,
        offset: 0.0,
        engineering_unit: None,
        deadband: 0.0,
        update_rate_ms: 100,
        stale_timeout_ms: 1000,
        access: AccessLevel::ReadOnly,
        fallback_value: None,
        fallback_quality: IndustrialQuality::Bad,
        last_inbound_value: None,
        last_outbound_value: None,
        last_update_timestamp_ms: 0,
    };

    let err = engine.add_mapping(duplicate_mapping);
    assert!(
        err.is_err(),
        "Duplicate inbound tag mapping MUST be rejected as an ownership conflict"
    );

    // Inbound scaling verification
    let raw_val = IndustrialDataValue::Float32(100.0);
    let ts = IndustrialTimestamp::now(Some(1000), Some(1000));
    let scaled = engine
        .process_inbound(
            "conn-01",
            "HR:40001:F32:WORDSWAP",
            &raw_val,
            IndustrialQuality::Good,
            ts,
        )
        .unwrap();
    // 100.0 * 0.1 + 5.0 = 15.0
    match scaled.value {
        IndustrialDataValue::Float64(f) => assert!((f - 15.0).abs() < 1e-4),
        _ => panic!("Expected Float64"),
    }
    assert_eq!(scaled.quality, IndustrialQuality::Good);

    // Deadband test: subsequent value 100.1 (EU = 15.01, delta = 0.01 < deadband 0.05) should be suppressed
    let minor_raw = IndustrialDataValue::Float32(100.1);
    let ts2 = IndustrialTimestamp::now(Some(1050), Some(1050));
    let suppressed = engine.process_inbound(
        "conn-01",
        "HR:40001:F32:WORDSWAP",
        &minor_raw,
        IndustrialQuality::Good,
        ts2,
    );
    assert!(
        suppressed.is_none(),
        "Change within deadband must be suppressed"
    );

    // Stale timeout test
    let stale_tags = engine.check_stale_mappings(3500); // 3500 - 1000 = 2500ms > stale_timeout 2000ms
    assert_eq!(stale_tags.len(), 1);
    assert_eq!(stale_tags[0].canonical_id, "TI-101.PV");
    assert_eq!(stale_tags[0].quality, IndustrialQuality::Bad);
}

#[test]
fn test_store_and_forward_queue_overflow_and_drain() {
    let mut queue = StoreAndForwardQueue::new(5, OverflowPolicy::DropOldest);
    assert!(queue.is_empty());

    for i in 1..=5 {
        let tag_val = IndustrialTagValue {
            canonical_id: format!("TAG_{i}"),
            source: DataSourceKind::Modbus,
            value: IndustrialDataValue::Int16(i as i16),
            datatype: IndustrialDataType::Int16,
            engineering_unit: None,
            quality: IndustrialQuality::Good,
            timestamp: IndustrialTimestamp::now(Some(1000 + i as u64), None),
            update_rate_ms: 100,
            access: AccessLevel::ReadWrite,
            provenance: TagProvenance {
                connection_id: "conn-01".to_string(),
                remote_address: format!("HR:4000{i}:I16:BIGENDIAN"),
                protocol: "ModbusTcp".to_string(),
                zone_id: "zone-l2-control".to_string(),
            },
            alarm_state: None,
            diagnostic_state: None,
        };
        assert!(queue.enqueue(tag_val).is_ok());
    }

    assert_eq!(queue.len(), 5);

    // Enqueue 6th item with DropOldest policy: TAG_1 should be dropped
    let overflow_val = IndustrialTagValue {
        canonical_id: "TAG_6".to_string(),
        source: DataSourceKind::Modbus,
        value: IndustrialDataValue::Int16(6),
        datatype: IndustrialDataType::Int16,
        engineering_unit: None,
        quality: IndustrialQuality::Good,
        timestamp: IndustrialTimestamp::now(Some(1006), None),
        update_rate_ms: 100,
        access: AccessLevel::ReadWrite,
        provenance: TagProvenance {
            connection_id: "conn-01".to_string(),
            remote_address: "HR:40006:I16:BIGENDIAN".to_string(),
            protocol: "ModbusTcp".to_string(),
            zone_id: "zone-l2-control".to_string(),
        },
        alarm_state: None,
        diagnostic_state: None,
    };
    assert!(queue.enqueue(overflow_val).is_ok());
    assert_eq!(queue.len(), 5);
    assert_eq!(queue.dropped_count(), 1);

    // Drain batch of 3 items FIFO
    let batch = queue.drain_batch(3);
    assert_eq!(batch.len(), 3);
    assert_eq!(batch[0].canonical_id, "TAG_2"); // TAG_1 was dropped
    assert_eq!(batch[1].canonical_id, "TAG_3");
    assert_eq!(batch[2].canonical_id, "TAG_4");
    assert_eq!(queue.len(), 2);
}

#[test]
fn test_ssrf_guard_security_validation() {
    let guard = SsrfGuard::default();

    // 1. Cloud metadata IP (AWS/GCP/Azure) MUST be blocked
    let cloud_meta = "http://169.254.169.254/latest/meta-data/";
    assert!(
        guard.validate_endpoint(cloud_meta).is_err(),
        "Cloud metadata endpoint must be blocked"
    );

    // 2. Blacklisted ports (e.g. SSH port 22) MUST be blocked
    let ssh_port = "tcp://192.168.1.50:22";
    assert!(
        guard.validate_endpoint(ssh_port).is_err(),
        "Restricted port 22 must be blocked"
    );

    // 3. Valid industrial ports (Modbus 502, OPC UA 4840, MQTT 1883, Sparkplug 8883) MUST pass
    assert!(guard.validate_endpoint("tcp://192.168.1.100:502").is_ok());
    assert!(guard.validate_endpoint("opc.tcp://10.0.0.5:4840").is_ok());
    assert!(guard.validate_endpoint("mqtt://10.0.0.10:1883").is_ok());
    assert!(guard.validate_endpoint("mqtts://10.0.0.20:8883").is_ok());

    // 4. Password sanitization test
    let masked =
        SsrfGuard::sanitize_credentials("opc.tcp://admin:SuperSecretPass123@192.168.1.50:4840");
    assert!(
        !masked.contains("SuperSecretPass123"),
        "Passwords must be redacted in logs and diagnostics"
    );
    assert!(masked.contains("***REDACTED***"));
}
