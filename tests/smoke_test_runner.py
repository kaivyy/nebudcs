#!/usr/bin/env python3
"""
NebuDCS Production Deployment & End-to-End Smoke Test Suite
Executes a real production verification against http://127.0.0.1:8088.
"""

import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request

BASE_URL = "http://127.0.0.1:8088"
ADMIN_TOKEN = "sec-admin-change-me-in-production-482a9f"
INSTRUCTOR_TOKEN = "sec-inst-change-me-in-production-b71c3e"
OPERATOR_TOKEN = "sec-oper-change-me-in-production-c92d4f"
TRAINEE_TOKEN = "sec-train-change-me-in-production-e10a8b"

def http_req(method, path, body=None, token=None):
    url = f"{BASE_URL}{path}"
    data = json.dumps(body).encode("utf-8") if body is not None else None
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req) as resp:
            status = resp.getcode()
            res_body = resp.read().decode("utf-8")
            try:
                parsed = json.loads(res_body)
            except Exception:
                parsed = res_body
            return status, parsed
    except urllib.error.HTTPError as e:
        err_body = e.read().decode("utf-8")
        try:
            parsed = json.loads(err_body)
        except Exception:
            parsed = err_body
        return e.code, parsed

def log_test(name, passed, detail=""):
    mark = "PASS" if passed else "FAIL"
    print(f"[{mark}] {name}: {detail}")
    if not passed:
        sys.exit(1)

def run_suite():
    print("=================================================================")
    print("NebuDCS Production Deployment & End-to-End Smoke Test")
    print(f"Target: {BASE_URL}")
    print("=================================================================\n")

    # -----------------------------------------------------------------
    # STEP 1: Health Probes & Static Assets
    # -----------------------------------------------------------------
    print("--- 1. Health Probes & Static Assets ---")
    st, resp = http_req("GET", "/api/v1/health/live")
    log_test("Liveness probe", st == 200 and resp.get("status") == "alive", str(resp))

    st, resp = http_req("GET", "/api/v1/health/ready")
    log_test("Readiness probe", st == 200 and resp.get("status") == "ready", str(resp))

    st, resp = http_req("GET", "/api/v1/metrics")
    log_test("Metrics telemetry probe", st == 200 and "process_memory_bytes" in resp,
             f"RSS={resp.get('process_memory_bytes')} bytes, Uptime={resp.get('uptime_seconds'):.1f}s")

    st, resp = http_req("GET", "/")
    log_test("Frontend index.html served", st == 200 and "<!DOCTYPE html>" in str(resp), "HTML document returned")

    # -----------------------------------------------------------------
    # STEP 2: Security & RBAC Enforcement
    # -----------------------------------------------------------------
    print("\n--- 2. Security & RBAC Enforcement ---")
    # Unauthenticated request on admin backup -> 401
    st, resp = http_req("POST", "/api/v1/admin/backup")
    log_test("Unauthenticated admin access rejected", st == 401, f"HTTP {st}")

    # Trainee token on admin backup -> 403
    st, resp = http_req("POST", "/api/v1/admin/backup", token=TRAINEE_TOKEN)
    log_test("Trainee blocked from admin backup", st == 403, f"HTTP {st} - {resp.get('message')}")

    # Operator token on admin backup -> 403
    st, resp = http_req("POST", "/api/v1/admin/backup", token=OPERATOR_TOKEN)
    log_test("Operator blocked from admin backup", st == 403, f"HTTP {st} - {resp.get('message')}")

    # Trainee token on scenario creation -> 403
    st, resp = http_req("POST", "/api/v1/training/scenarios", body={"id": "scn-test", "name": "Test", "description": "test", "author": "trainee", "version": "1.0.0", "duration_limit_sec": 600.0, "random_seed": 1, "initial_conditions_json": "{}", "timeline_json": "[]", "objectives_json": "[]", "criteria_json": "[]"}, token=TRAINEE_TOKEN)
    log_test("Trainee blocked from scenario creation", st == 403, f"HTTP {st}")

    # Path traversal attack -> 403
    st, resp = http_req("GET", "/api/v1/projects/..%2F..%2Fetc%2Fpasswd", token=ADMIN_TOKEN)
    log_test("Path traversal rejected in projects", st == 403, f"HTTP {st}")

    st, resp = http_req("GET", "/api/v1/training/scenarios/..%2F..%2Fetc%2Fpasswd", token=ADMIN_TOKEN)
    log_test("Path traversal rejected in scenarios", st == 403, f"HTTP {st}")

    # -----------------------------------------------------------------
    # STEP 3: Arbitrary Non-Refinery Plant E2E Smoke Test
    # -----------------------------------------------------------------
    print("\n--- 3. Arbitrary Non-Refinery Plant E2E Smoke Test ---")
    proj_name = "Smoke Test Plant - Generic Water Cooling & Buffer"
    create_payload = {
        "name": proj_name,
        "description": "Arbitrary non-refinery plant smoke test flowsheet with pump, buffer tank, control valve, instruments, PID loop, and alarm",
        "author": "Deployment Verification Engineer"
    }
    st, created_proj = http_req("POST", "/api/v1/projects", body=create_payload, token=ADMIN_TOKEN)
    log_test("Create project", st == 200 and "id" in created_proj, f"Project ID: {created_proj.get('id')}")
    proj_id = created_proj["id"]

    # Configure plant entities
    eq_pump_id = "00000000-0000-0000-00aa-000000000001"
    eq_valve_id = "00000000-0000-0000-00aa-000000000002"
    eq_tank_id = "00000000-0000-0000-00aa-000000000003"

    created_proj["equipment"] = [
        {
            "id": eq_pump_id,
            "definition_id": "standard.pump.centrifugal",
            "definition_version": "1.0.0",
            "name": "PUMP-A1",
            "parameters": {"design_flow": 3000.0, "rated_head": 80.0, "power_kw": 150.0},
            "position": {"x": 100.0, "y": 250.0}
        },
        {
            "id": eq_valve_id,
            "definition_id": "standard.valve.control_globe",
            "definition_version": "1.0.0",
            "name": "VALVE-C3",
            "parameters": {"cv": 50.0, "position": 50.0},
            "position": {"x": 300.0, "y": 250.0}
        },
        {
            "id": eq_tank_id,
            "definition_id": "standard.vessel.vertical_separator",
            "definition_version": "1.0.0",
            "name": "TANK-B7",
            "parameters": {"volume": 25.0, "initial_level": 45.0},
            "position": {"x": 550.0, "y": 250.0}
        }
    ]

    created_proj["streams"] = [
        {
            "id": "00000000-0000-0000-00bb-000000000001",
            "name": "S-901",
            "stream_type": "Material",
            "source_equipment_id": eq_pump_id,
            "source_port_id": "discharge",
            "sink_equipment_id": eq_valve_id,
            "sink_port_id": "inlet",
            "composition": {},
            "route_points": [{"x": 100.0, "y": 250.0}, {"x": 300.0, "y": 250.0}]
        },
        {
            "id": "00000000-0000-0000-00bb-000000000002",
            "name": "S-902",
            "stream_type": "Material",
            "source_equipment_id": eq_valve_id,
            "source_port_id": "outlet",
            "sink_equipment_id": eq_tank_id,
            "sink_port_id": "feed",
            "composition": {},
            "route_points": [{"x": 300.0, "y": 250.0}, {"x": 550.0, "y": 250.0}]
        }
    ]

    created_proj["instruments"] = [
        {
            "id": "00000000-0000-0000-00cc-000000000001",
            "tag": "PT-900",
            "equipment_id": eq_pump_id,
            "port_id": "discharge",
            "measurement_kind": "Pressure",
            "range_min": 0.0,
            "range_max": 40.0,
            "unit": "bar",
            "position": {"x": 160.0, "y": 180.0}
        },
        {
            "id": "00000000-0000-0000-00cc-000000000002",
            "tag": "FT-901",
            "equipment_id": eq_valve_id,
            "port_id": "outlet",
            "measurement_kind": "Flow",
            "range_min": 0.0,
            "range_max": 5000.0,
            "unit": "kg/h",
            "position": {"x": 380.0, "y": 180.0}
        },
        {
            "id": "00000000-0000-0000-00cc-000000000003",
            "tag": "LT-902",
            "equipment_id": eq_tank_id,
            "port_id": None,
            "measurement_kind": "Level",
            "range_min": 0.0,
            "range_max": 100.0,
            "unit": "%",
            "position": {"x": 550.0, "y": 150.0}
        }
    ]

    created_proj["control_loops"] = [
        {
            "id": "00000000-0000-0000-00dd-000000000001",
            "name": "LIC-902",
            "pv_tag": "TANK-B7.LEVEL",
            "sp": 50.0,
            "out_tag": "VALVE-C3.POS",
            "kp": 1.5,
            "ti": 20.0,
            "td": 0.5,
            "reverse_acting": False,
            "mode": "AUTO",
            "output_min": 0.0,
            "output_max": 100.0,
            "position": {"x": 450.0, "y": 120.0}
        }
    ]

    created_proj["alarms"] = [
        {
            "id": "00000000-0000-0000-00ee-000000000001",
            "name": "LAH-902",
            "pv_tag": "TANK-B7.LEVEL",
            "alarm_type": "Hi",
            "severity": "Warning",
            "trip_limit": 80.0,
            "deadband": 1.0,
            "description": "Buffer tank high level alert"
        }
    ]

    created_proj["interlocks"] = [
        {
            "id": "00000000-0000-0000-00ff-000000000001",
            "name": "TRIP-PUMP-A1",
            "source_tag": "TANK-B7.LEVEL",
            "condition": "<",
            "threshold": 5.0,
            "target_action": "TRIP",
            "target_equipment": eq_pump_id,
            "latched": True,
            "description": "Buffer tank low level pump dry run trip"
        }
    ]

    # Save project
    st, saved_resp = http_req("PUT", f"/api/v1/projects/{proj_id}", body={"project": created_proj}, token=ADMIN_TOKEN)
    log_test("Save project flowsheet", st == 200, f"Equipment: {len(created_proj['equipment'])}, Streams: {len(created_proj['streams'])}")

    # Validate project
    st, val_resp = http_req("POST", f"/api/v1/projects/{proj_id}/validate", token=ADMIN_TOKEN)
    log_test("Validate project topology", st == 200 and val_resp.get("valid") is True, f"Validation valid: {val_resp.get('valid')}, Error count: {val_resp.get('error_count')}")

    # Load into authoritative simulation engine
    st, load_sim_resp = http_req("POST", f"/api/v1/projects/{proj_id}/load-sim", token=ADMIN_TOKEN)
    log_test("Load simulation runtime", st == 200 and load_sim_resp.get("status") == "ok", f"Status: {load_sim_resp.get('status')}")

    # Start simulation ticking
    st, play_resp = http_req("POST", "/api/v1/sim/control", body={"action": "play"}, token=OPERATOR_TOKEN)
    log_test("SimControl Play command", st == 200, "Simulation ticking active")

    # Let simulation run for 0.8 seconds to accumulate ticks
    time.sleep(0.8)

    # Verify simulation telemetry & snapshot
    st, snap = http_req("GET", "/api/v1/sim/snapshot", token=OPERATOR_TOKEN)
    log_test("Telemetry snapshot", st == 200 and snap.get("tick_count") > 0 and snap.get("sim_time_sec") > 0,
             f"SimTime: {snap.get('sim_time_sec'):.2f}s, Ticks: {snap.get('tick_count')}, Tag Count: {len(snap.get('tags', []))}")

    # Verify tags populated
    st, tag_list = http_req("GET", "/api/v1/tags", token=OPERATOR_TOKEN)
    tags_dict = {t["tag"]: t["value"] for t in tag_list.get("tags", [])}
    log_test("Runtime tags active", len(tags_dict) > 0,
             f"Total tags: {len(tags_dict)}, sample tags: {list(tags_dict.keys())[:6]}")

    # Change setpoint via faceplate endpoint
    st, sp_resp = http_req("POST", "/api/v1/control/pid/setpoint", body={"loop_name": "LIC-902", "sp": 65.0}, token=OPERATOR_TOKEN)
    log_test("Faceplate Setpoint adjustment (LIC-902 -> 65.0)", st == 200, f"Response: {sp_resp}")

    # Write tank level above alarm threshold (85.0 > 80.0 limit) to trigger alarm
    st, write_resp = http_req("POST", "/api/v1/tags/write", body={"tag": "TANK-B7.LEVEL", "value": 85.0}, token=OPERATOR_TOKEN)
    log_test("Write tag to induce alarm (TANK-B7.LEVEL = 85.0)", st == 200, f"Response: {write_resp}")

    time.sleep(0.3)

    # Check active alarms
    st, alarm_list = http_req("GET", "/api/v1/alarms", token=OPERATOR_TOKEN)
    log_test("Alarm list query", st == 200, f"Active alarm count: {alarm_list.get('count')}")

    # Acknowledge alarm
    st, ack_resp = http_req("POST", "/api/v1/alarms/acknowledge", body={"alarm_name": "LAH-902"}, token=OPERATOR_TOKEN)
    log_test("Acknowledge alarm (LAH-902)", st == 200, f"Response: {ack_resp}")

    # -----------------------------------------------------------------
    # STEP 4: Training Studio E2E Smoke Test
    # -----------------------------------------------------------------
    print("\n--- 4. Training Studio E2E Smoke Test ---")
    # Author scenario with Instructor role
    scn_dto = {
        "id": "scn-smoke-recovery",
        "name": "Plant Cooling Upset & Recovery Drill",
        "description": "Training drill testing operator response to buffer tank level surge and valve setpoint correction",
        "author": "Chief Training Instructor",
        "version": "1.0.0",
        "compatible_project_id": proj_id,
        "duration_limit_sec": 600.0,
        "random_seed": 12345678,
        "initial_conditions_json": json.dumps({
            "tags": [{"tag": "TANK-B7.LEVEL", "value": 45.0}]
        }),
        "timeline_json": json.dumps([
            {
                "id": "EVT-1",
                "priority": 1,
                "trigger": {"type": "time", "elapsed_sec": 2.0},
                "action": {
                    "type": "emit_marker",
                    "label": "DRILL_START",
                    "message": "Cooling upset drill marker"
                }
            }
        ]),
        "objectives_json": json.dumps([
            {
                "id": "OBJ-1",
                "description": "Maintain buffer tank level within range",
                "criteria": {
                    "type": "tag_within_range",
                    "tag": "TANK-B7.LEVEL",
                    "min_val": 20.0,
                    "max_val": 95.0,
                    "hold_duration_sec": 5.0
                },
                "weight": 100,
                "mandatory": True
            }
        ]),
        "criteria_json": "[]"
    }
    st, saved_scn = http_req("POST", "/api/v1/training/scenarios", body=scn_dto, token=INSTRUCTOR_TOKEN)
    log_test("Instructor author scenario (scn-smoke-recovery)", st == 200, f"Scenario ID: {saved_scn.get('id')}")

    # Create training session
    session_req = {
        "name": "Live Smoke Drill Session 01",
        "description": "End-to-end smoke verification session",
        "project_id": proj_id,
        "scenario_id": "scn-smoke-recovery",
        "instructor_id": "INST-LEAD-1",
        "trainee_id": "TRAINEE-OP-9"
    }
    st, session = http_req("POST", "/api/v1/training/sessions", body=session_req, token=INSTRUCTOR_TOKEN)
    log_test("Create training session", st == 200 and "id" in session, f"Session ID: {session.get('id')}")
    sess_id = session["id"]

    # Load project into session
    st, load_sess = http_req("POST", f"/api/v1/training/sessions/{sess_id}/load",
                             body={"project_id": proj_id, "scenario_id": "scn-smoke-recovery"}, token=INSTRUCTOR_TOKEN)
    log_test("Load session project into sim", st == 200, f"Status: {load_sess.get('status')}")

    # Start training session
    st, start_sess = http_req("POST", f"/api/v1/training/sessions/{sess_id}/start", token=INSTRUCTOR_TOKEN)
    log_test("Start training session", st == 200, f"Status: {start_sess.get('status')}")

    # Trainee executes control action: SET_SP on LIC-902 to 52.0
    action_req_1 = {
        "session_id": sess_id,
        "actor": "TRAINEE-OP-9",
        "action_type": "SET_SP",
        "target_entity": "LIC-902",
        "requested_value": 52.0
    }
    st, act_resp_1 = http_req("POST", f"/api/v1/training/sessions/{sess_id}/action", body=action_req_1, token=TRAINEE_TOKEN)
    log_test("Trainee action: SET_SP to 52.0", st == 200 and act_resp_1.get("status") == "Accepted",
             f"Action status: {act_resp_1.get('status')}")

    # Trainee acknowledges alarm: ACK_ALARM
    action_req_2 = {
        "session_id": sess_id,
        "actor": "TRAINEE-OP-9",
        "action_type": "ACK_ALARM",
        "target_entity": "LAH-902"
    }
    st, act_resp_2 = http_req("POST", f"/api/v1/training/sessions/{sess_id}/action", body=action_req_2, token=TRAINEE_TOKEN)
    log_test("Trainee action: ACK_ALARM", st == 200 and act_resp_2.get("status") == "Accepted",
             f"Action status: {act_resp_2.get('status')}")

    # Trainee attempts forbidden action: INJECT_FAULT -> should be rejected by server
    action_req_bad = {
        "session_id": sess_id,
        "actor": "TRAINEE-OP-9",
        "action_type": "INJECT_FAULT",
        "target_entity": "PUMP-A1"
    }
    st, act_resp_bad = http_req("POST", f"/api/v1/training/sessions/{sess_id}/action", body=action_req_bad, token=TRAINEE_TOKEN)
    is_rejected = st == 200 and "Rejected" in act_resp_bad.get("status", "")
    log_test("Trainee forbidden action rejected", is_rejected, f"Status: {act_resp_bad.get('status')}")

    time.sleep(0.5)

    # Complete session and generate evaluation report
    st, eval_report = http_req("POST", f"/api/v1/training/sessions/{sess_id}/complete", token=INSTRUCTOR_TOKEN)
    log_test("Complete session & evaluate", st == 200 and "percentage" in eval_report,
             f"Score: {eval_report.get('percentage')}%, Passed: {eval_report.get('passed')}")

    # Query deterministic evaluation report
    st, eval_get = http_req("GET", f"/api/v1/training/sessions/{sess_id}/evaluation", token=INSTRUCTOR_TOKEN)
    log_test("Fetch evaluation report", st == 200, f"Report ID: {eval_get.get('session_id')}, Passed: {eval_get.get('passed')}")

    # Query replay summary
    st, replay_sum = http_req("GET", f"/api/v1/training/sessions/{sess_id}/replay", token=INSTRUCTOR_TOKEN)
    log_test("Replay summary timeline", st == 200 and "markers" in replay_sum,
             f"Markers count: {len(replay_sum.get('markers', []))}")

    # Query replay frame at sim_time 0.05
    st, replay_frame = http_req("GET", f"/api/v1/training/sessions/{sess_id}/replay/frame?sim_time=0.05", token=INSTRUCTOR_TOKEN)
    log_test("Replay frame scrubbing", st == 200 and "sim_time_sec" in replay_frame,
             f"SimTime: {replay_frame.get('sim_time_sec')}s, Tags: {len(replay_frame.get('tags', {}))}")

    # -----------------------------------------------------------------
    # STEP 5: Persistence & Service Crash Recovery Test
    # -----------------------------------------------------------------
    print("\n--- 5. Persistence & Crash Recovery Test ---")
    data_dir = "/var/lib/nebudcs/data"
    proj_file = os.path.join(data_dir, "projects", f"{proj_id}.json")
    scn_file = os.path.join(data_dir, "scenarios", "scn-smoke-recovery.json")
    sess_file = os.path.join(data_dir, "training", f"{sess_id}.json")
    log_test("Persistence files on disk",
             os.path.isfile(proj_file) and os.path.isfile(scn_file) and os.path.isfile(sess_file),
             f"Project: {os.path.basename(proj_file)}, Scenario: {os.path.basename(scn_file)}, Session: {os.path.basename(sess_file)}")

    print("Restarting nebudcs.service to test crash recovery...")
    res = subprocess.run(["systemctl", "restart", "nebudcs.service"], capture_output=True, text=True)
    log_test("systemctl restart nebudcs.service", res.returncode == 0, res.stderr.strip())

    # Wait for server readiness probe
    reconnected = False
    for _ in range(20):
        time.sleep(0.3)
        try:
            st, ready = http_req("GET", "/api/v1/health/ready")
            if st == 200 and ready.get("status") == "ready":
                reconnected = True
                break
        except Exception:
            pass
    log_test("Server recovered and ready", reconnected, "Readiness probe passed post-restart")

    # Fetch project after restart to prove persistence
    st, recovered_proj = http_req("GET", f"/api/v1/projects/{proj_id}", token=ADMIN_TOKEN)
    log_test("Project survived restart", st == 200 and recovered_proj.get("name") == proj_name,
             f"ID: {recovered_proj.get('id')}, Equipment count: {len(recovered_proj.get('equipment', []))}")

    # Fetch scenario after restart to prove persistence
    st, recovered_scn = http_req("GET", "/api/v1/training/scenarios/scn-smoke-recovery", token=ADMIN_TOKEN)
    log_test("Scenario survived restart", st == 200 and recovered_scn.get("id") == "scn-smoke-recovery",
             f"ID: {recovered_scn.get('id')}, Name: {recovered_scn.get('name')}")

    # Fetch training session after restart
    st, recovered_sess = http_req("GET", f"/api/v1/training/sessions/{sess_id}", token=ADMIN_TOKEN)
    log_test("Training session survived restart", st == 200 and recovered_sess.get("id") == sess_id,
             f"ID: {recovered_sess.get('id')}, Status: {recovered_sess.get('status')}")

    # -----------------------------------------------------------------
    # STEP 6: Cryptographic Backup & Restore Test
    # -----------------------------------------------------------------
    print("\n--- 6. Cryptographic Backup & Restore Test ---")
    # Trigger backup script
    print("Executing /usr/local/bin/nebudcs-backup.sh...")
    res = subprocess.run(["/usr/local/bin/nebudcs-backup.sh"], capture_output=True, text=True)
    log_test("Automated backup script execution", res.returncode == 0, res.stdout.strip())

    backup_dir = "/var/backups/nebudcs"
    backup_files = [f for f in os.listdir(backup_dir) if f.startswith("nebudcs_backup_")]
    log_test("Backup file exists in /var/backups/nebudcs", len(backup_files) > 0, str(backup_files))

    latest_backup = sorted(backup_files)[-1]
    backup_path = os.path.join(backup_dir, latest_backup)
    with open(backup_path, "r", encoding="utf-8") as f:
        bundle = json.load(f)

    log_test("Backup bundle structure",
             "manifest" in bundle and "projects" in bundle and "scenarios" in bundle and "training" in bundle,
             f"Created: {bundle.get('created_at_iso')}, Manifest entries: {len(bundle.get('manifest', []))}, Projects: {len(bundle.get('projects', []))}")

    # Test Restore API
    st, restore_resp = http_req("POST", "/api/v1/admin/restore", body=bundle, token=ADMIN_TOKEN)
    log_test("Admin restore endpoint", st == 200 and restore_resp.get("projects_restored", 0) > 0,
             f"Projects: {restore_resp.get('projects_restored')}, Scenarios: {restore_resp.get('scenarios_restored')}, Training: {restore_resp.get('training_restored')}")

    # Verify restored state via API
    st, verify_proj = http_req("GET", f"/api/v1/projects/{proj_id}", token=ADMIN_TOKEN)
    log_test("Restored project state verified", st == 200 and verify_proj.get("id") == proj_id,
             f"Project: {verify_proj.get('name')}")

    print("\n=================================================================")
    print("ALL PRODUCTION SMOKE TESTS PASSED CLEANLY (100% SUCCESS)")
    print("=================================================================")

if __name__ == "__main__":
    run_suite()
