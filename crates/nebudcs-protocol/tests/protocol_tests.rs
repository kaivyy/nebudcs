use nebudcs_protocol::*;

#[test]
fn test_version_compatibility_matrix() {
    assert!(is_compatible_protocol_version("1.0.0"));
    assert!(is_compatible_protocol_version("1.0.1"));
    assert!(is_compatible_protocol_version("1.0.99"));
    assert!(!is_compatible_protocol_version("2.0.0"));
    assert!(!is_compatible_protocol_version("0.9.0"));
    assert!(!is_compatible_protocol_version(""));
    assert!(!is_compatible_protocol_version("1.0"));
    assert!(!is_compatible_protocol_version("alpha"));
}

#[test]
fn test_client_message_serde_roundtrip() {
    let msg1 = ClientMessage::Handshake {
        protocol_version: "1.0.0".to_string(),
        client_id: "client-42".to_string(),
        capabilities: ClientCapabilities::default(),
    };
    let json1 = serde_json::to_string(&msg1).unwrap();
    let de1: ClientMessage = serde_json::from_str(&json1).unwrap();
    assert_eq!(msg1, de1);

    let msg2 = ClientMessage::Subscribe {
        tags: vec!["FIC101.PV".to_string(), "TIC102.SP".to_string()],
        all_tags: false,
        include_alarms: true,
    };
    let json2 = serde_json::to_string(&msg2).unwrap();
    let de2: ClientMessage = serde_json::from_str(&json2).unwrap();
    assert_eq!(msg2, de2);

    let msg3 = ClientMessage::Command {
        command_id: "cmd-001".to_string(),
        command: RuntimeCommand::SetSetpoint {
            loop_name: "PIC101".to_string(),
            sp: 25.5,
        },
    };
    let json3 = serde_json::to_string(&msg3).unwrap();
    let de3: ClientMessage = serde_json::from_str(&json3).unwrap();
    assert_eq!(msg3, de3);

    let msg4 = ClientMessage::Ping { nonce: 12345 };
    let json4 = serde_json::to_string(&msg4).unwrap();
    let de4: ClientMessage = serde_json::from_str(&json4).unwrap();
    assert_eq!(msg4, de4);
}

#[test]
fn test_server_message_serde_roundtrip() {
    let msg1 = ServerMessage::HandshakeAck {
        protocol_version: "1.0.0".to_string(),
        server_version: "0.1.0".to_string(),
        session_id: "sess-abc".to_string(),
        model_version: "1.0.0".to_string(),
    };
    let json1 = serde_json::to_string(&msg1).unwrap();
    let de1: ServerMessage = serde_json::from_str(&json1).unwrap();
    assert_eq!(msg1, de1);

    let msg2 = ServerMessage::Snapshot(StateSnapshotDto {
        snapshot_id: "snap-1".to_string(),
        sim_time_sec: 10.5,
        tick_count: 210,
        tags: vec![TagValueSampleDto {
            tag: "PT101".to_string(),
            value: TagValueDto::Float(12.34),
            quality: TagQualityDto::Good,
            unit: Some("bar".to_string()),
        }],
        active_alarms: vec![AlarmSnapshotDto {
            alarm_name: "PAH101".to_string(),
            state: AlarmStateDto::ActiveUnack,
            severity: AlarmSeverityDto::Warning,
            pv: 12.34,
            trip_limit: 10.0,
        }],
    });
    let json2 = serde_json::to_string(&msg2).unwrap();
    let de2: ServerMessage = serde_json::from_str(&json2).unwrap();
    assert_eq!(msg2, de2);

    let msg3 = ServerMessage::Delta(DeltaUpdateDto {
        seq: 42,
        sim_time_sec: 10.55,
        tick_count: 211,
        changed_tags: vec![TagValueSampleDto {
            tag: "PT101".to_string(),
            value: TagValueDto::Float(12.45),
            quality: TagQualityDto::Good,
            unit: Some("bar".to_string()),
        }],
    });
    let json3 = serde_json::to_string(&msg3).unwrap();
    let de3: ServerMessage = serde_json::from_str(&json3).unwrap();
    assert_eq!(msg3, de3);

    let msg4 = ServerMessage::Error(
        ProtocolError::new(
            ProtocolErrorCode::ProtocolVersionMismatch,
            "Incompatible protocol version",
        )
        .with_correlation_id("conn-1"),
    );
    let json4 = serde_json::to_string(&msg4).unwrap();
    let de4: ServerMessage = serde_json::from_str(&json4).unwrap();
    assert_eq!(msg4, de4);
}

#[test]
fn test_runtime_command_variants() {
    let cmds = vec![
        RuntimeCommand::SimControl(SimControlCommand::Play),
        RuntimeCommand::SimControl(SimControlCommand::Pause),
        RuntimeCommand::SimControl(SimControlCommand::Step),
        RuntimeCommand::SimControl(SimControlCommand::SetSpeedMultiplier { multiplier: 2.5 }),
        RuntimeCommand::WriteTag {
            tag: "VALVE_CMD".to_string(),
            value: TagValueDto::Float(75.0),
        },
        RuntimeCommand::SetSetpoint {
            loop_name: "LIC101".to_string(),
            sp: 5.0,
        },
        RuntimeCommand::SetMode {
            loop_name: "LIC101".to_string(),
            mode: PidModeDto::Manual,
        },
        RuntimeCommand::SetManualOutput {
            loop_name: "LIC101".to_string(),
            output: 45.0,
        },
        RuntimeCommand::AcknowledgeAlarm {
            alarm_name: "LAH101".to_string(),
        },
        RuntimeCommand::ShelveAlarm {
            alarm_name: "LAH101".to_string(),
            duration_seconds: 3600.0,
        },
        RuntimeCommand::ResetProtection {
            block_name: "ESD_PUMP_TRIP".to_string(),
        },
        RuntimeCommand::Scenario(ScenarioCommand::Start),
        RuntimeCommand::Scenario(ScenarioCommand::Pause),
        RuntimeCommand::Scenario(ScenarioCommand::Resume),
        RuntimeCommand::Scenario(ScenarioCommand::Abort {
            reason: "Emergency".into(),
        }),
        RuntimeCommand::Scenario(ScenarioCommand::Reset),
        RuntimeCommand::Scenario(ScenarioCommand::Load {
            scenario_json: "{}".into(),
        }),
        RuntimeCommand::Scenario(ScenarioCommand::InjectFault {
            fault_json: "{}".into(),
        }),
        RuntimeCommand::Scenario(ScenarioCommand::ClearFault {
            fault_id: "FLT-1".into(),
        }),
    ];

    for cmd in cmds {
        let json = serde_json::to_string(&cmd).unwrap();
        let de: RuntimeCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, de);
    }
}

#[test]
fn test_scenario_status_dto_serde_roundtrip() {
    let dto = ScenarioStatusDto {
        scenario_id: Some("SCN-101".into()),
        scenario_name: Some("Startup Training".into()),
        state: "RUNNING".into(),
        elapsed_sec: 42.5,
        duration_limit_sec: Some(300.0),
        active_faults: vec![ActiveFaultDto {
            id: "FLT-1".into(),
            target: "PI-101".into(),
            category: "sensor_bias".into(),
            elapsed_sec: 12.0,
            duration_sec: Some(60.0),
        }],
        active_disturbances_count: 0,
        events_total: 5,
        events_fired: 2,
        score: 95.0,
        mandatory_completed: false,
    };

    let msg = ServerMessage::ScenarioUpdate(dto.clone());
    let json = serde_json::to_string(&msg).unwrap();
    let de: ServerMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, de);
}
