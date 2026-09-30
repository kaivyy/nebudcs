//! Stage 14 Naphtha Hydrotreater (NHT) Dynamic Physics & Disturbance Acceptance Tests.
//!
//! Validates deterministic causal physics and DCS safety responses under 6 industrial disturbance scenarios:
//! - Scenario A: Feed Step Change (Charge Pump / Valve disturbance)
//! - Scenario B: Furnace COT / Reactor Exothermic Runaway & High-High Trip (TAHH-101 / ESD-R101-TEMP at 343.3 C)
//! - Scenario C: Hydrogen Starvation / Recycle Gas Flow Reduction
//! - Scenario D: HP Separator High-High Level Liquid Carryover Trip (LAHH-101 / ESD-V102-LAHH at 85.0%)
//! - Scenario E: Combined Feed Exchanger Train Thermal Disturbance & Heat Transfer
//! - Scenario F: Wash Water Loss & Ammonium Salt Deposition Warning (FAL-103 at 4.5 m3/h)

use std::collections::BTreeMap;
use std::time::Duration;

use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_protocol::{RuntimeCommand, TagValueDto};
use nebudcs_server::{
    nht_seed::PROJ_NHT_REFERENCE, HostHandle, ProjectManager, ServerConfig, SimulationHost,
};
use nebudcs_sim::SimulationEngine;

fn create_test_host() -> (SimulationHost, HostHandle) {
    let config = ServerConfig {
        simulation_dt: 0.05,
        speed_multiplier: 1.0,
        ..Default::default()
    };
    let mut engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");
    engine.clock_mut().play();
    let tag_store = RuntimeTagStore::new();

    let host = SimulationHost::start(
        config,
        engine,
        tag_store,
        Some(nebudcs_alarm::AlarmSystem::new()),
        Some(nebudcs_control::ControlSystem::new()),
        None,
    )
    .expect("Failed to start simulation host");

    let handle = host.handle();
    (host, handle)
}

#[tokio::test]
async fn test_nht_dynamic_physics_and_disturbance_scenarios() {
    let (_host, handle) = create_test_host();

    // 1. Load authoritative NHT reference plant project
    let pm = ProjectManager::new();
    let nht_dto = pm
        .get_project(PROJ_NHT_REFERENCE)
        .expect("NHT Reference Plant must be registered in ProjectManager");

    handle
        .dispatch_command(RuntimeCommand::LoadProject {
            project: Box::new(nht_dto),
        })
        .await
        .expect("Failed to load NHT reference project");

    // Advance simulation
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 2. Verify baseline physics and DCS telemetry
    let snap = handle.get_snapshot().await;
    let tag_map: BTreeMap<String, f64> = snap
        .tags
        .iter()
        .filter_map(|t| t.value.as_f64().map(|v| (t.tag.clone(), v)))
        .collect();

    assert!(tag_map.contains_key("FT-101"), "Feed Flow FT-101 must exist");
    assert!(tag_map.contains_key("TT-101"), "Furnace COT TT-101 must exist");
    assert!(tag_map.contains_key("TT-102"), "Reactor Effluent TT-102 must exist");
    assert!(tag_map.contains_key("PT-102"), "HP Separator Pressure PT-102 must exist");
    assert!(tag_map.contains_key("LT-101"), "HP Separator Level LT-101 must exist");
    assert!(tag_map.contains_key("FT-102"), "Recycle Gas Flow FT-102 must exist");
    assert!(tag_map.contains_key("FT-103"), "Wash Water Flow FT-103 must exist");

    // -----------------------------------------------------------------------
    // SCENARIO A: NHT Charge Feed Step Change
    // -----------------------------------------------------------------------
    let initial_feed = tag_map.get("FT-101").copied().unwrap_or(199.0);
    // Increase feed flow setpoint / tag to 220 m3/h (+10.5% turndown increase)
    handle
        .dispatch_command(RuntimeCommand::WriteTag {
            tag: "FT-101".to_string(),
            value: TagValueDto::Float(220.0),
        })
        .await
        .expect("Failed to write feed flow");

    tokio::time::sleep(Duration::from_millis(150)).await;

    let snap_feed = handle.get_snapshot().await;
    let updated_feed = snap_feed
        .tags
        .iter()
        .find(|t| t.tag == "FT-101")
        .and_then(|t| t.value.as_f64())
        .unwrap_or(0.0);
    assert!(
        updated_feed >= initial_feed,
        "Feed flow FT-101 must reflect step increase: {updated_feed} >= {initial_feed}"
    );

    // -----------------------------------------------------------------------
    // SCENARIO B: Furnace COT / Reactor Exothermic Runaway & High-High Trip (343.3 C)
    // -----------------------------------------------------------------------
    // Verify ESD-R101-TEMP initially normal
    let esd_rx_tripped_before = snap_feed
        .tags
        .iter()
        .find(|t| t.tag == "ESD-R101-TEMP.TRIPPED")
        .and_then(|t| match t.value {
            TagValueDto::Bool(b) => Some(b),
            _ => None,
        })
        .unwrap_or(false);
    assert!(!esd_rx_tripped_before, "ESD-R101-TEMP must be healthy initially");

    // Step 1: Rise to 341.0 C -> High Alarm TAH-101 (threshold 340.0 C)
    handle
        .dispatch_command(RuntimeCommand::WriteTag {
            tag: "TT-102".to_string(),
            value: TagValueDto::Float(341.0),
        })
        .await
        .expect("Write TT-102");
    tokio::time::sleep(Duration::from_millis(150)).await;

    let snap_tah = handle.get_snapshot().await;
    let tah_active = snap_tah
        .active_alarms
        .iter()
        .any(|a| a.alarm_name == "TAH-101");
    assert!(tah_active, "TAH-101 high temperature alarm must be active at 341.0 C");

    // Step 2: Rise to 344.5 C (> 343.3 C EIL / HPCL Visakh design trip limit)
    handle
        .dispatch_command(RuntimeCommand::WriteTag {
            tag: "TT-102".to_string(),
            value: TagValueDto::Float(344.5),
        })
        .await
        .expect("Write TT-102 runaway");
    tokio::time::sleep(Duration::from_millis(150)).await;

    let snap_tahh = handle.get_snapshot().await;
    let tahh_active = snap_tahh
        .active_alarms
        .iter()
        .any(|a| a.alarm_name == "TAHH-101");
    assert!(tahh_active, "TAHH-101 emergency alarm must be active at 344.5 C");

    for t in &snap_tahh.tags {
        if t.tag.contains("TT-102") || t.tag.contains("ESD") || t.tag.contains("R101") {
            println!("TAG: {} = {:?}", t.tag, t.value);
        }
    }
    println!("Active alarms: {:?}", snap_tahh.active_alarms);
    let esd_rx_tripped = snap_tahh
        .tags
        .iter()
        .find(|t| t.tag == "ESD-R101-TEMP.TRIPPED")
        .and_then(|t| match t.value {
            TagValueDto::Bool(b) => Some(b),
            _ => None,
        })
        .unwrap_or(false);
    assert!(
        esd_rx_tripped,
        "ESD-R101-TEMP must trip when TT-102 exceeds 343.3 C"
    );

    // Step 3: Test Reset Refusal while temperature remains above 343.3 C
    handle
        .dispatch_command(RuntimeCommand::ResetProtection {
            block_name: "ESD-R101-TEMP".to_string(),
        })
        .await
        .expect("Dispatch reset");
    tokio::time::sleep(Duration::from_millis(100)).await;

    let snap_refused = handle.get_snapshot().await;
    let esd_rx_still_tripped = snap_refused
        .tags
        .iter()
        .find(|t| t.tag == "ESD-R101-TEMP.TRIPPED")
        .and_then(|t| match t.value {
            TagValueDto::Bool(b) => Some(b),
            _ => None,
        })
        .unwrap_or(false);
    assert!(
        esd_rx_still_tripped,
        "ESD-R101-TEMP must refuse reset while TT-102 is above trip threshold"
    );

    // Step 4: Cool reactor to safe operating condition (330.0 C) and reset
    handle
        .dispatch_command(RuntimeCommand::WriteTag {
            tag: "TT-102".to_string(),
            value: TagValueDto::Float(330.0),
        })
        .await
        .expect("Cool reactor");
    tokio::time::sleep(Duration::from_millis(150)).await;

    handle
        .dispatch_command(RuntimeCommand::ResetProtection {
            block_name: "ESD-R101-TEMP".to_string(),
        })
        .await
        .expect("Dispatch reset");
    tokio::time::sleep(Duration::from_millis(150)).await;

    let snap_cleared = handle.get_snapshot().await;
    let esd_rx_cleared = snap_cleared
        .tags
        .iter()
        .find(|t| t.tag == "ESD-R101-TEMP.TRIPPED")
        .and_then(|t| match t.value {
            TagValueDto::Bool(b) => Some(b),
            _ => None,
        })
        .unwrap_or(true);
    assert!(!esd_rx_cleared, "ESD-R101-TEMP must clear after safe reset");

    // -----------------------------------------------------------------------
    // SCENARIO C: Hydrogen Starvation / Recycle Gas Flow Reduction
    // -----------------------------------------------------------------------
    let initial_h2 = snap_cleared
        .tags
        .iter()
        .find(|t| t.tag == "FT-102")
        .and_then(|t| t.value.as_f64())
        .unwrap_or(10.0);

    // Reduce recycle compressor speed from 10450 rpm to 4500 rpm
    handle
        .dispatch_command(RuntimeCommand::WriteTag {
            tag: "K-101A.SPEED".to_string(),
            value: TagValueDto::Float(4500.0),
        })
        .await
        .expect("Reduce compressor speed");
    tokio::time::sleep(Duration::from_millis(150)).await;

    let snap_h2 = handle.get_snapshot().await;
    let h2_flow = snap_h2
        .tags
        .iter()
        .find(|t| t.tag == "FT-102")
        .and_then(|t| t.value.as_f64())
        .unwrap_or(0.0);
    println!("Recycle gas flow: initial={initial_h2}, after reduction={h2_flow}");
    assert!(
        h2_flow <= initial_h2,
        "Recycle gas flow FT-102 must reflect reduction with compressor speed: {h2_flow} <= {initial_h2}"
    );

    // -----------------------------------------------------------------------
    // SCENARIO D: HP Separator High Level & Compressor Trip (85.0%)
    // -----------------------------------------------------------------------
    let esd_v102_tripped_before = snap_h2
        .tags
        .iter()
        .find(|t| t.tag == "ESD-V102-LAHH.TRIPPED")
        .and_then(|t| match t.value {
            TagValueDto::Bool(b) => Some(b),
            _ => None,
        })
        .unwrap_or(false);
    assert!(!esd_v102_tripped_before, "ESD-V102-LAHH must not be tripped initially");

    // Level rises to 0.88 (> 0.85 LAHH limit)
    handle
        .dispatch_command(RuntimeCommand::WriteTag {
            tag: "LT-101".to_string(),
            value: TagValueDto::Float(0.88),
        })
        .await
        .expect("Raise separator level");
    tokio::time::sleep(Duration::from_millis(150)).await;

    let snap_lahh = handle.get_snapshot().await;
    let lahh_active = snap_lahh
        .active_alarms
        .iter()
        .any(|a| a.alarm_name == "LAHH-101");
    assert!(lahh_active, "LAHH-101 alarm must trigger at 88% separator level");

    let esd_v102_tripped = snap_lahh
        .tags
        .iter()
        .find(|t| t.tag == "ESD-V102-LAHH.TRIPPED")
        .and_then(|t| match t.value {
            TagValueDto::Bool(b) => Some(b),
            _ => None,
        })
        .unwrap_or(false);
    assert!(
        esd_v102_tripped,
        "ESD-V102-LAHH must trip compressor K-101A to prevent liquid carryover"
    );

    // -----------------------------------------------------------------------
    // SCENARIO E: Combined Feed Exchanger Thermal Duty Disturbance
    // -----------------------------------------------------------------------
    // Adjust furnace inlet temperature TT-101 to simulate exchanger fouling / duty drop
    handle
        .dispatch_command(RuntimeCommand::WriteTag {
            tag: "TT-101".to_string(),
            value: TagValueDto::Float(310.0),
        })
        .await
        .expect("Simulate exchanger fouling");
    tokio::time::sleep(Duration::from_millis(150)).await;

    let snap_cfx = handle.get_snapshot().await;
    let cfx_temp = snap_cfx
        .tags
        .iter()
        .find(|t| t.tag == "TT-101")
        .and_then(|t| t.value.as_f64())
        .unwrap_or(0.0);
    assert!(
        (cfx_temp - 310.0).abs() < 1.0,
        "Furnace inlet temperature TT-101 must reflect thermal fouling (310.0 C), got {cfx_temp}"
    );

    // -----------------------------------------------------------------------
    // SCENARIO F: Wash Water Loss & Ammonium Salt Deposition Warning (4.5 m3/h)
    // -----------------------------------------------------------------------
    handle
        .dispatch_command(RuntimeCommand::WriteTag {
            tag: "FT-103".to_string(),
            value: TagValueDto::Float(3.2),
        })
        .await
        .expect("Cut wash water flow");
    tokio::time::sleep(Duration::from_millis(150)).await;

    let snap_water = handle.get_snapshot().await;
    let fal_active = snap_water
        .active_alarms
        .iter()
        .any(|a| a.alarm_name == "FAL-103");
    assert!(
        fal_active,
        "FAL-103 wash water low flow alarm must activate when flow drops below 4.5 m3/h"
    );
}
