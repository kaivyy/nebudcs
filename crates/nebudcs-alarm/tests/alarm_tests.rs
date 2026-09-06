use nebudcs_alarm::block::{AlarmBlock, AlarmBlockConfig};
use nebudcs_alarm::engine::AlarmEngine;
use nebudcs_alarm::types::{AlarmSeverity, AlarmState, AlarmType};
use nebudcs_model::tag::TagQuality;
use nebudcs_types::TagId;

#[test]
fn test_alarm_state_transitions_isa18_2() {
    let tag = TagId::new();
    let config = AlarmBlockConfig {
        name: "PT101_HI".to_string(),
        tag_id: tag,
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Warning,
        limit: 100.0,
        deadband: 2.0,
        default_shelve_duration: 60.0,
    };
    let mut block = AlarmBlock::new(config);
    assert_eq!(block.state(), AlarmState::Normal);

    // PV crosses limit (105.0 >= 100.0) -> transitions to ActiveUnack
    block.step(105.0, TagQuality::Good, 1.0);
    assert_eq!(block.state(), AlarmState::ActiveUnack);

    // Operator acknowledges alarm -> transitions to ActiveAck
    block.acknowledge();
    assert_eq!(block.state(), AlarmState::ActiveAck);

    // PV returns within normal range (97.0 <= 100.0 - 2.0 deadband) -> transitions to Normal
    block.step(97.0, TagQuality::Good, 1.0);
    assert_eq!(block.state(), AlarmState::Normal);
}

#[test]
fn test_alarm_deadband_hysteresis() {
    let tag = TagId::new();
    let config = AlarmBlockConfig {
        name: "PT101_HI".to_string(),
        tag_id: tag,
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Warning,
        limit: 100.0,
        deadband: 2.0, // deadband = 2.0, so clear threshold is 98.0
        default_shelve_duration: 60.0,
    };
    let mut block = AlarmBlock::new(config);

    // Step 1: PV exceeds limit -> ActiveUnack
    block.step(101.0, TagQuality::Good, 1.0);
    assert_eq!(block.state(), AlarmState::ActiveUnack);
    block.acknowledge();
    assert_eq!(block.state(), AlarmState::ActiveAck);

    // Step 2: PV drops below 100.0 to 99.0, but still > 98.0 (within deadband)
    block.step(99.0, TagQuality::Good, 1.0);
    assert_eq!(
        block.state(),
        AlarmState::ActiveAck,
        "Alarm must NOT clear within deadband margin (99.0 > 98.0)"
    );

    // Step 3: PV drops to 97.9 (< 98.0) -> clears deadband -> transitions to Normal
    block.step(97.9, TagQuality::Good, 1.0);
    assert_eq!(
        block.state(),
        AlarmState::Normal,
        "Alarm must clear once process drops below limit - deadband"
    );
}

#[test]
fn test_alarm_shelving_lifecycle() {
    let tag = TagId::new();
    let config = AlarmBlockConfig {
        name: "TT101_HI".to_string(),
        tag_id: tag,
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Warning,
        limit: 80.0,
        deadband: 1.0,
        default_shelve_duration: 10.0, // 10 seconds shelving
    };
    let mut block = AlarmBlock::new(config);

    // Trip alarm: PV = 85.0 -> ActiveUnack -> Ack
    block.step(85.0, TagQuality::Good, 1.0);
    block.acknowledge();
    assert_eq!(block.state(), AlarmState::ActiveAck);

    // Operator shelves alarm for 10 seconds
    block.shelve(10.0);
    assert_eq!(block.state(), AlarmState::Shelved);

    // Step 9 seconds: still shelved
    block.step(85.0, TagQuality::Good, 9.0);
    assert_eq!(
        block.state(),
        AlarmState::Shelved,
        "Alarm must remain shelved before timer expiration"
    );

    // Step 2 more seconds (total 11s > 10s): timer expires, PV still 85.0 (> 80.0) -> Re-alarms as ActiveUnack!
    block.step(85.0, TagQuality::Good, 2.0);
    assert_eq!(
        block.state(),
        AlarmState::ActiveUnack,
        "Shelved alarm must automatically re-activate as ActiveUnack upon shelf timer expiry if process is still in alarm"
    );
}

#[test]
fn test_alarm_reactivation_on_higher_severity() {
    let tag = TagId::new();
    let mut engine = AlarmEngine::new();

    // Configure HI alarm (Warning at 80.0)
    let hi_cfg = AlarmBlockConfig {
        name: "TI101_HI".to_string(),
        tag_id: tag,
        alarm_type: AlarmType::Hi,
        severity: AlarmSeverity::Warning,
        limit: 80.0,
        deadband: 1.0,
        default_shelve_duration: 60.0,
    };
    // Configure HIHI alarm (Critical at 95.0)
    let hihi_cfg = AlarmBlockConfig {
        name: "TI101_HIHI".to_string(),
        tag_id: tag,
        alarm_type: AlarmType::HiHi,
        severity: AlarmSeverity::Critical,
        limit: 95.0,
        deadband: 1.0,
        default_shelve_duration: 60.0,
    };

    engine.add_alarm(hi_cfg);
    engine.add_alarm(hihi_cfg);

    // Process reaches 85.0: HI alarm activates
    engine.evaluate_tag(tag, 85.0, TagQuality::Good, 1.0);
    let hi = engine.get_alarm("TI101_HI").unwrap();
    assert_eq!(hi.state(), AlarmState::ActiveUnack);

    // Operator shelves the HI alarm
    engine.shelve_alarm("TI101_HI", 300.0);
    assert_eq!(
        engine.get_alarm("TI101_HI").unwrap().state(),
        AlarmState::Shelved
    );

    // Process continues rising: reaches 98.0 (> 95.0 HIHI threshold)
    engine.evaluate_tag(tag, 98.0, TagQuality::Good, 1.0);

    // HIHI alarm must immediately fire as ActiveUnack, bypassing the shelved status of HI!
    let hihi = engine.get_alarm("TI101_HIHI").unwrap();
    assert_eq!(
        hihi.state(),
        AlarmState::ActiveUnack,
        "HIHI alarm must fire immediately as ActiveUnack even when lower-severity HI alarm is shelved"
    );
}
