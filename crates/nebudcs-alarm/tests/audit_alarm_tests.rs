use nebudcs_alarm::block::{AlarmBlock, AlarmBlockConfig};
use nebudcs_alarm::system::AlarmSystem;
use nebudcs_alarm::types::{AlarmSeverity, AlarmState, AlarmType};
use nebudcs_model::tag::{TagQuality, TagValue};
use nebudcs_sim::{EquipmentState, SimulationEngine, StateBuffer};
use nebudcs_types::{EquipmentId, TagId};

#[test]
fn test_alarm_config_validation() {
    // 1. NaN limit
    let cfg = AlarmBlockConfig {
        name: "ALM_NAN".to_string(),
        tag_id: TagId::new(),
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Warning,
        limit: f64::NAN,
        deadband: 2.0,
        default_shelve_duration: 60.0,
    };
    assert!(cfg.validate().is_err(), "NaN limit must be rejected");

    // 2. Negative deadband
    let cfg = AlarmBlockConfig {
        name: "ALM_NEG_DB".to_string(),
        tag_id: TagId::new(),
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Warning,
        limit: 100.0,
        deadband: -1.0,
        default_shelve_duration: 60.0,
    };
    assert!(
        cfg.validate().is_err(),
        "Negative deadband must be rejected"
    );

    // 3. Negative shelving duration
    let cfg = AlarmBlockConfig {
        name: "ALM_NEG_SHELVE".to_string(),
        tag_id: TagId::new(),
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Warning,
        limit: 100.0,
        deadband: 2.0,
        default_shelve_duration: -10.0,
    };
    assert!(
        cfg.validate().is_err(),
        "Negative shelve duration must be rejected"
    );

    // 4. Valid config
    let valid_cfg = AlarmBlockConfig {
        name: "ALM_OK".to_string(),
        tag_id: TagId::new(),
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Critical,
        limit: 100.0,
        deadband: 2.0,
        default_shelve_duration: 60.0,
    };
    assert!(valid_cfg.validate().is_ok());
}

#[test]
fn test_alarm_full_isa_18_2_lifecycle() {
    let cfg = AlarmBlockConfig {
        name: "TI_101_HI".to_string(),
        tag_id: TagId::new(),
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Warning,
        limit: 80.0,
        deadband: 5.0, // Clears below 75.0
        default_shelve_duration: 30.0,
    };
    let mut block = AlarmBlock::new(cfg);
    assert_eq!(block.state(), AlarmState::Normal);

    // 1. Normal -> ActiveUnack (PV reaches 80.0)
    block.step(80.5, TagQuality::Good, 0.1);
    assert_eq!(block.state(), AlarmState::ActiveUnack);
    assert!(block.is_unack());
    assert!(block.is_active());

    // 2. ActiveUnack -> ActiveAck (Operator acknowledges)
    block.acknowledge();
    assert_eq!(block.state(), AlarmState::ActiveAck);
    assert!(!block.is_unack());
    assert!(block.is_active());

    // 3. ActiveAck with PV still above limit - deadband (e.g. 77.0): stays ActiveAck
    block.step(77.0, TagQuality::Good, 0.1);
    assert_eq!(block.state(), AlarmState::ActiveAck);

    // 4. ActiveAck -> Normal (PV drops below cleared threshold: 74.0 < 75.0)
    block.step(74.0, TagQuality::Good, 0.1);
    assert_eq!(block.state(), AlarmState::Normal);
    assert!(!block.is_active());

    // 5. Unacknowledged clear path: Normal -> ActiveUnack -> Cleared -> Normal
    block.step(85.0, TagQuality::Good, 0.1);
    assert_eq!(block.state(), AlarmState::ActiveUnack);

    // Process variable drops below deadband BEFORE operator acknowledges
    block.step(73.0, TagQuality::Good, 0.1);
    assert_eq!(
        block.state(),
        AlarmState::Cleared,
        "Alarm must transition to Cleared if condition clears before acknowledgement"
    );

    // Now operator acknowledges the cleared alarm
    block.acknowledge();
    assert_eq!(block.state(), AlarmState::Normal);
}

#[test]
fn test_alarm_shelving_and_automatic_unshelving() {
    let cfg = AlarmBlockConfig {
        name: "PI_102_HI".to_string(),
        tag_id: TagId::new(),
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Warning,
        limit: 100.0,
        deadband: 2.0,
        default_shelve_duration: 1.0, // 1 second
    };
    let mut block = AlarmBlock::new(cfg);

    // Trigger alarm
    block.step(105.0, TagQuality::Good, 0.1);
    assert_eq!(block.state(), AlarmState::ActiveUnack);

    // Shelve for 1.0 second
    block.shelve(1.0);
    assert_eq!(block.state(), AlarmState::Shelved);

    // Step 0.5s: still shelved
    block.step(105.0, TagQuality::Good, 0.5);
    assert_eq!(block.state(), AlarmState::Shelved);

    // Step 0.6s (total elapsed 1.1s > 1.0s): shelving expired!
    // Since condition is STILL violated (105.0 >= 100.0), it MUST transition back to ActiveUnack!
    block.step(105.0, TagQuality::Good, 0.6);
    assert_eq!(
        block.state(),
        AlarmState::ActiveUnack,
        "Expired shelved alarm must reactivate to ActiveUnack if condition persists"
    );
}

#[test]
fn test_alarm_types_lo_deviation_roc_bad_quality() {
    let tag = TagId::new();

    // 1. Low Alarm (Lo)
    let lo_cfg = AlarmBlockConfig {
        name: "LO_ALARM".to_string(),
        tag_id: tag,
        alarm_type: AlarmType::Lo,
        severity: AlarmSeverity::Warning,
        limit: 20.0,
        deadband: 2.0, // Clears above 22.0
        default_shelve_duration: 60.0,
    };
    let mut lo_block = AlarmBlock::new(lo_cfg);
    lo_block.step(25.0, TagQuality::Good, 0.1);
    assert_eq!(lo_block.state(), AlarmState::Normal);
    lo_block.step(19.5, TagQuality::Good, 0.1);
    assert_eq!(lo_block.state(), AlarmState::ActiveUnack);
    lo_block.step(21.0, TagQuality::Good, 0.1); // Inside deadband (20+2=22) -> remains active
    assert_eq!(lo_block.state(), AlarmState::ActiveUnack);
    lo_block.step(22.5, TagQuality::Good, 0.1); // Cleared
    assert_eq!(lo_block.state(), AlarmState::Cleared);

    // 2. BadQuality Alarm
    let bq_cfg = AlarmBlockConfig {
        name: "BQ_ALARM".to_string(),
        tag_id: tag,
        alarm_type: AlarmType::BadQuality,
        severity: AlarmSeverity::Critical,
        limit: 0.0,
        deadband: 0.0,
        default_shelve_duration: 60.0,
    };
    let mut bq_block = AlarmBlock::new(bq_cfg);
    bq_block.step(50.0, TagQuality::Good, 0.1);
    assert_eq!(bq_block.state(), AlarmState::Normal);
    bq_block.step(50.0, TagQuality::Bad, 0.1);
    assert_eq!(bq_block.state(), AlarmState::ActiveUnack);
}

#[test]
fn test_alarm_system_non_interference_invariant() {
    let tag_id = TagId::new();
    let eq_id = EquipmentId::new();

    let mut alarm_sys = AlarmSystem::new();
    alarm_sys.add_alarm(AlarmBlockConfig {
        name: "TEST_ALM".to_string(),
        tag_id,
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Critical,
        limit: 50.0,
        deadband: 2.0,
        default_shelve_duration: 60.0,
    });

    let mut initial_state = StateBuffer::new();
    let mut eq = EquipmentState::new("running");
    eq.set_var("temperature", 75.0);
    initial_state.equipment.insert(eq_id, eq);
    initial_state.tags.insert(tag_id, TagValue::Float(75.0));
    initial_state.qualities.insert(tag_id, TagQuality::Good);

    let mut engine = SimulationEngine::new(0.1).unwrap();
    engine.add_system(alarm_sys);
    engine.initialize(initial_state);

    // Step the simulation
    engine.step().unwrap();

    // Verify:
    // 1. Alarm activated
    let comm_buf = engine.committed_buffer();
    assert_eq!(
        comm_buf
            .get_equipment(eq_id)
            .unwrap()
            .get_var("temperature")
            .unwrap(),
        75.0,
        "AlarmSystem must NEVER mutate physical equipment state"
    );
    assert_eq!(
        comm_buf.get_tag(tag_id).unwrap(),
        &TagValue::Float(75.0),
        "AlarmSystem must NEVER mutate process tags"
    );
}
