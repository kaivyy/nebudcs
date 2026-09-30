//! NebuDCS Stage 8: Vendor-Neutral Virtual PLC / DCS Controller Architecture Integration Tests
//!
//! Verifies:
//! 1. REST Endpoints for Hardware Profiles, DCS Control Modules, Redundancy, and Fault Injection.
//! 2. DTO Serialization roundtrip for ProjectController with Stage 8 extensions.
//! 3. 10,000-tick bit-identical determinism with active DCS Control Modules.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use tower::ServiceExt;

use nebudcs_control::pid::{PidConfig, PidDirection};
use nebudcs_control::runtime::{
    ControllerConfig, ControllerTask, CpuProfile, DcsControlModule, FirmwareProfile, MemoryProfile,
    RedundancyRole, RedundancyState, RedundancyStatus, VirtualController,
};
use nebudcs_control::signal::RuntimeTagStore;
use nebudcs_control::system::ControlSystem;
use nebudcs_model::project::{
    ProjectControlModule, ProjectController, ProjectCpuProfile, ProjectFirmwareProfile,
    ProjectMemoryProfile, ProjectRedundancyConfig, ProjectTask,
};
use nebudcs_model::tag::TagQuality;
use nebudcs_protocol::{
    ControlModuleActionRequestDto, ControllerDetailDto, ControllerFaultInjectionRequestDto,
};
use nebudcs_server::project_manager::ProjectManager;
use nebudcs_server::{build_app, HostHandle, ServerConfig, SimulationHost};
use nebudcs_sim::SimulationEngine;
use nebudcs_types::{ControllerId, TaskId};

fn create_stage_8_test_host(
    config: ServerConfig,
) -> (SimulationHost, HostHandle, ControllerId, String) {
    let mut engine = SimulationEngine::new(config.simulation_dt).expect("Failed to create engine");
    engine.clock_mut().play();
    let tag_store = RuntimeTagStore::new();

    let mut ctrl_sys = ControlSystem::new();
    let ctrl_id = ControllerId::new();
    let mut ctrl = VirtualController::new(ctrl_id, "DCS-VCPU-01", ControllerConfig::default());

    // Hardware profiles
    ctrl.set_cpu_profile(CpuProfile {
        execution_budget_us: 25_000,
        max_scan_frequency_hz: 50.0,
        utilization_threshold_pct: 75.0,
        architecture: "Virtual-DCS-VCPU64".to_string(),
    });

    ctrl.set_memory_profile(MemoryProfile {
        program_memory_bytes: 16 * 1024 * 1024,
        data_memory_bytes: 32 * 1024 * 1024,
        retentive_memory_bytes: 4 * 1024 * 1024,
        program_memory_used_bytes: 1024 * 512,
        data_memory_used_bytes: 1024 * 1024,
        retentive_memory_used_bytes: 1024 * 64,
    });

    ctrl.set_firmware_profile(FirmwareProfile {
        version: "v8.2.0-LTS".to_string(),
        build: "2026.09.07".to_string(),
        iec_standard: "IEC 61131-3:2013 Ed.3".to_string(),
    });

    ctrl.set_redundancy(RedundancyState {
        role: RedundancyRole::Primary,
        status: RedundancyStatus::Synchronized,
        partner_id: Some("DCS-VCPU-02-STBY".to_string()),
        sync_frequency_hz: 20.0,
        switchover_timeout_seconds: 0.1,
        last_sync_sim_time: 0.0,
        sync_cycle_count: 142,
    });

    // Control module
    let pid_cfg = PidConfig {
        kp: 2.0,
        ti: 10.0,
        td: 0.0,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Direct,
        sp_tracking_in_manual: true,
        scan_divisor: 1,
    };
    let mod_id = "FIC-101".to_string();
    let mut cm = DcsControlModule::new(
        &mod_id,
        "Furnace Feed Flow Controller",
        "FIT-101.PV",
        "FV-101.OUT",
        50.0,
        pid_cfg,
    );
    cm.add_permissive("PUMP-101.RUNNING");
    cm.add_interlock("ESD-TRIP-HIGH");
    cm.set_safe_position(0.0);

    ctrl.add_control_module(cm);

    // Provide default task
    let task = ControllerTask::new_periodic(TaskId::new(), "FastScan", 0.05, 5);
    ctrl.add_task(task);

    // Seed I/O
    ctrl.io_image_mut()
        .write_input("FIT-101.PV", 50.0, TagQuality::Good, 0.0);
    ctrl.io_image_mut()
        .write_input("PUMP-101.RUNNING", 1.0, TagQuality::Good, 0.0);
    ctrl.io_image_mut()
        .write_input("ESD-TRIP-HIGH", 0.0, TagQuality::Good, 0.0);

    let _ = ctrl.start();

    ctrl_sys.register_controller(ctrl);

    let host = SimulationHost::start(config, engine, tag_store, None, Some(ctrl_sys), None)
        .expect("Failed to start host");

    let handle = host.handle();
    (host, handle, ctrl_id, mod_id)
}

#[tokio::test]
async fn test_stage_8_rest_endpoints_profiles_redundancy_and_modules() {
    let config = ServerConfig {
        simulation_dt: 0.05,
        admin_token: Some("secret_admin".to_string()),
        operator_token: Some("secret_operator".to_string()),
        trainee_token: Some("secret_trainee".to_string()),
        ..Default::default()
    };

    let (_host, handle, ctrl_id, mod_id) = create_stage_8_test_host(config);
    let app = build_app(handle);

    tokio::time::sleep(std::time::Duration::from_millis(60)).await;

    // 1. GET /api/v1/controllers/{id}
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/controllers/{ctrl_id}"))
        .header(header::AUTHORIZATION, "Bearer secret_operator")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let detail: ControllerDetailDto = serde_json::from_slice(&body).unwrap();

    // Verify Profiles
    let cpu = detail.cpu_profile.expect("CPU profile must be populated");
    assert_eq!(cpu.architecture, "Virtual-DCS-VCPU64");
    assert_eq!(cpu.execution_budget_us, 25_000);

    let mem = detail
        .memory_profile
        .expect("Memory profile must be populated");
    assert_eq!(mem.program_memory_bytes, 16 * 1024 * 1024);
    assert_eq!(mem.retentive_memory_bytes, 4 * 1024 * 1024);

    let fw = detail
        .firmware_profile
        .expect("Firmware profile must be populated");
    assert_eq!(fw.version, "v8.2.0-LTS");

    let red = detail
        .redundancy
        .expect("Redundancy status must be populated");
    assert_eq!(red.role, "Primary");
    assert_eq!(red.partner_id, Some("DCS-VCPU-02-STBY".to_string()));

    // Verify Control Modules
    assert_eq!(detail.control_modules.len(), 1);
    let cm = &detail.control_modules[0];
    assert_eq!(cm.id, mod_id);
    assert_eq!(cm.mode, "AUTO");
    assert_eq!(cm.sp, 50.0);
    assert!(cm.permissives_ok);
    assert!(!cm.is_interlocked);

    // 2. POST /api/v1/controllers/{ctrl_id}/modules/{mod_id} -> Change to MAN & set output
    let mod_req = ControlModuleActionRequestDto {
        mode: Some("MAN".to_string()),
        sp: None,
        manual_out: Some(65.0),
    };
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/controllers/{ctrl_id}/modules/{mod_id}"))
        .header(header::AUTHORIZATION, "Bearer secret_operator")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&mod_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. POST /api/v1/controllers/{id}/fault -> Inject Watchdog Timeout
    let fault_req = ControllerFaultInjectionRequestDto {
        fault_type: "watchdog".to_string(),
        description: "Integration test watchdog fault".to_string(),
        recoverable: true,
    };
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/controllers/{ctrl_id}/fault"))
        .header(header::AUTHORIZATION, "Bearer secret_admin")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&fault_req).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. POST /api/v1/controllers/{id}/redundancy/switchover -> Trigger Switchover
    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/controllers/{ctrl_id}/redundancy/switchover"
        ))
        .header(header::AUTHORIZATION, "Bearer secret_admin")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[test]
fn test_stage_8_project_serialization_roundtrip_with_control_modules() {
    let ctrl = ProjectController {
        id: "C-201".to_string(),
        name: "Chemical Reactor Controller".to_string(),
        description: "Continuous stirred-tank reactor virtual controller".to_string(),
        watchdog_timeout_seconds: 0.8,
        enabled: true,
        tasks: vec![ProjectTask {
            id: "RegulatoryScan".to_string(),
            name: "Fast Regulatory Loop".to_string(),
            task_type: "periodic".to_string(),
            period_seconds: 0.05,
            priority: 4,
            watchdog_seconds: 0.5,
            enabled: true,
            programs: vec![],
        }],
        cpu_profile: Some(ProjectCpuProfile {
            execution_budget_us: 30_000,
            max_scan_frequency_hz: 50.0,
            utilization_threshold_pct: 85.0,
            architecture: "DCS-VCPU-R2".to_string(),
        }),
        memory_profile: Some(ProjectMemoryProfile {
            program_memory_bytes: 8 * 1024 * 1024,
            data_memory_bytes: 16 * 1024 * 1024,
            retentive_memory_bytes: 2 * 1024 * 1024,
        }),
        firmware_profile: Some(ProjectFirmwareProfile {
            version: "v8.2.0".to_string(),
            build: "2026.09.07".to_string(),
            iec_standard: "IEC 61131-3:2013 Ed.3".to_string(),
        }),
        redundancy: Some(ProjectRedundancyConfig {
            role: "Primary".to_string(),
            partner_controller_id: Some("C-202".to_string()),
            sync_frequency_hz: 25.0,
            switchover_timeout_seconds: 0.08,
        }),
        control_modules: vec![ProjectControlModule {
            id: "TIC-201".to_string(),
            name: "Reactor Jacket Temperature Controller".to_string(),
            description: "Exothermic reaction cooling control module".to_string(),
            mode: "AUTO".to_string(),
            execution_rate_seconds: 0.05,
            pv_signal: "TT-201.PV".to_string(),
            sp: 85.0,
            out_signal: "TV-201.OUT".to_string(),
            strategy_type: "PID".to_string(),
            kp: 3.5,
            ti: 15.0,
            td: 1.0,
            output_min: 0.0,
            output_max: 100.0,
            permissives: vec!["AGITATOR-201.RUNNING".to_string()],
            interlocks: vec!["ESD-HIGH-TEMP".to_string()],
            safe_position: 100.0, // Fail open cooling
        }],
    };

    // Serialize to DTO and deserialize back
    let dto = ProjectManager::controller_to_dto(&ctrl);
    let roundtrip = ProjectManager::dto_to_controller(&dto);

    assert_eq!(roundtrip.id, ctrl.id);
    assert_eq!(roundtrip.cpu_profile, ctrl.cpu_profile);
    assert_eq!(roundtrip.memory_profile, ctrl.memory_profile);
    assert_eq!(roundtrip.firmware_profile, ctrl.firmware_profile);
    assert_eq!(roundtrip.redundancy, ctrl.redundancy);
    assert_eq!(roundtrip.control_modules.len(), 1);
    assert_eq!(roundtrip.control_modules[0].id, "TIC-201");
    assert_eq!(roundtrip.control_modules[0].safe_position, 100.0);
    assert_eq!(
        roundtrip.control_modules[0].permissives,
        vec!["AGITATOR-201.RUNNING"]
    );
}

#[test]
fn test_stage_8_10k_tick_bit_identical_determinism_with_dcs_modules() {
    let pid_cfg = PidConfig {
        kp: 2.5,
        ti: 8.0,
        td: 0.2,
        derivative_filter_n: 10.0,
        out_min: 0.0,
        out_max: 100.0,
        direction: PidDirection::Direct,
        sp_tracking_in_manual: true,
        scan_divisor: 1,
    };

    let run_sim = || -> Vec<(u64, u64, u64)> {
        let mut ctrl = VirtualController::with_name("DCS-DETERMINISM", ControllerConfig::default());
        let cm = DcsControlModule::new(
            "LIC-301",
            "Drum Level Controller",
            "LT-301.PV",
            "LV-301.OUT",
            50.0,
            pid_cfg.clone(),
        );
        ctrl.add_control_module(cm);
        let _ = ctrl.start();

        let mut checkpoints = Vec::new();
        let dt = 0.05;

        for tick in 1..=10_000 {
            let sim_time = (tick as f64) * dt;

            // Deterministic synthetic process dynamic (sinusoidal disturbance)
            let pv_val = 50.0 + 5.0 * ((tick as f64) * 0.01).sin();
            ctrl.io_image_mut()
                .write_input("LT-301.PV", pv_val, TagQuality::Good, sim_time);

            let _ = ctrl.step(dt, sim_time);

            if tick == 1_000 || tick == 5_000 || tick == 10_000 {
                let cm_ref = ctrl.control_module("LIC-301").unwrap();
                let pv_bits = cm_ref.current_pv().to_bits();
                let out_bits = cm_ref.current_out().to_bits();
                let integral_bits = cm_ref.pid().integral().to_bits();
                checkpoints.push((pv_bits, out_bits, integral_bits));
            }
        }
        checkpoints
    };

    let run1 = run_sim();
    let run2 = run_sim();

    assert_eq!(run1.len(), 3);
    assert_eq!(
        run1, run2,
        "10,000-tick bit-identical determinism violated across identical runs"
    );
}
