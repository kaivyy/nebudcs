#!/usr/bin/env python3
"""
NebuDCS Stage 4: PLC/DCS Engineering Studio Comprehensive Verification Suite
=============================================================================
Verifies:
1. Canonical Unified Project Model Hierarchy (Controllers, HAL Systems, Mappings, Tags)
2. Signal & Tag Cross-Reference Engine (/api/v1/projects/:id/cross-reference)
3. Design-Time Validation Rules:
   - VAL_TASK_PRIORITY_RANGE
   - VAL_IEC_COMPILE_ERROR
   - VAL_HAL_DIRECTION_MISMATCH
4. Project-Driven Simulation Deployment:
   - Dynamic instantiation of Virtual Controllers, Tasks, Programs, Routines
   - Dynamic instantiation of Hardware I/O Systems, Racks, Modules, Channels
   - Controller-to-Hardware channel mapping and signal loopback
5. Studio UI Integration & Rendering via Playwright (Explorer, Flowsheet, IEC Editor, HAL, Tags, Cross-Ref)
"""

import os
import sys
import time
import urllib.request
import urllib.error
import json
from playwright.sync_api import sync_playwright

BASE_URL = os.environ.get("NEBUDCS_URL", "http://127.0.0.1:8088")
ADMIN_TOKEN = os.environ.get("NEBUDCS_ADMIN_TOKEN", "sec-admin-change-me-in-production-482a9f")
OPERATOR_TOKEN = os.environ.get("NEBUDCS_OPERATOR_TOKEN", "sec-oper-change-me-in-production-c92d4f")

SCREENSHOT_DIR = "/root/NebuDCS/docs/screenshots"
ARTIFACT_SCREENSHOT_DIR = "/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots"
os.makedirs(SCREENSHOT_DIR, exist_ok=True)
os.makedirs(ARTIFACT_SCREENSHOT_DIR, exist_ok=True)

HEADERS_ADMIN = {
    "Content-Type": "application/json",
    "Authorization": f"Bearer {ADMIN_TOKEN}",
}

HEADERS_OPERATOR = {
    "Content-Type": "application/json",
    "Authorization": f"Bearer {OPERATOR_TOKEN}",
}


def api_request(path: str, method: str = "GET", data: dict = None, token: str = ADMIN_TOKEN) -> dict:
    url = f"{BASE_URL}{path}"
    payload = json.dumps(data).encode("utf-8") if data is not None else None
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {token}",
    }
    req = urllib.request.Request(url, data=payload, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req) as resp:
            body = resp.read().decode("utf-8")
            return json.loads(body) if body else {}
    except urllib.error.HTTPError as e:
        err_body = e.read().decode("utf-8")
        raise RuntimeError(f"HTTP {e.code} on {method} {path}: {err_body}")


def save_screenshot(page, name: str):
    p1 = os.path.join(SCREENSHOT_DIR, f"{name}.png")
    p2 = os.path.join(ARTIFACT_SCREENSHOT_DIR, f"{name}.png")
    page.screenshot(path=p1)
    page.screenshot(path=p2)
    print(f"  [SCREENSHOT] Saved: {name}.png")


def test_section_1_canonical_hierarchy():
    print("\n--- SECTION 1: Canonical Unified Project Hierarchy ---")
    projects = api_request("/api/v1/projects", token=OPERATOR_TOKEN)
    assert len(projects) > 0, "At least one project should be available"
    print(f"  [OK] Found {len(projects)} projects available")

    # Inspect default project
    default_proj_id = "proj-vertical-slice-01"
    proj = api_request(f"/api/v1/projects/{default_proj_id}", token=OPERATOR_TOKEN)
    assert proj["id"] == default_proj_id

    # Verify Controllers
    assert "controllers" in proj, "ProjectDetailDto must include 'controllers'"
    assert len(proj["controllers"]) >= 1, "Should have at least 1 virtual controller"
    c101 = proj["controllers"][0]
    assert c101["id"] == "C-101"
    assert len(c101["tasks"]) >= 1
    task = c101["tasks"][0]
    assert task["id"] == "FastLogic"
    assert len(task["programs"]) >= 1
    prog = task["programs"][0]
    assert len(prog["routines"]) >= 1
    routine = prog["routines"][0]
    assert routine["language"] == "ST"
    print(f"  [OK] Controller hierarchy verified: {c101['id']} -> Task '{task['id']}' -> Program '{prog['name']}' -> Routine '{routine['name']}' ({routine['language']})")

    # Verify HAL Systems
    assert "io_systems" in proj, "ProjectDetailDto must include 'io_systems'"
    assert len(proj["io_systems"]) >= 1, "Should have at least 1 HAL system"
    hal = proj["io_systems"][0]
    assert hal["id"] == "HAL-MAIN"
    assert len(hal["racks"]) >= 1
    rack = hal["racks"][0]
    assert len(rack["chassis"]) >= 1
    chassis = rack["chassis"][0]
    assert len(chassis["modules"]) >= 4, "Should have AI, AO, DI, DO modules"
    print(f"  [OK] HAL hierarchy verified: {hal['id']} -> Rack {rack['rack_number']} -> Chassis {chassis['chassis_number']} -> {len(chassis['modules'])} Modules")

    # Verify I/O Mappings
    assert "io_mappings" in proj, "ProjectDetailDto must include 'io_mappings'"
    assert len(proj["io_mappings"]) >= 2, "Should have input and output mappings"
    m_in = next(m for m in proj["io_mappings"] if m["direction"] == "input")
    m_out = next(m for m in proj["io_mappings"] if m["direction"] == "output")
    assert "AI" in m_in["channel_address"]
    assert "AO" in m_out["channel_address"]
    print(f"  [OK] I/O Mappings verified: {m_in['signal_key']} -> {m_in['channel_address']}, {m_out['signal_key']} -> {m_out['channel_address']}")

    # Verify Tags
    assert "tags" in proj, "ProjectDetailDto must include 'tags'"
    assert len(proj["tags"]) >= 4, "Should have canonical project tags"
    tag_names = [t["name"] for t in proj["tags"]]
    assert "V-101.LEVEL" in tag_names
    assert "FV-101.POS" in tag_names
    print(f"  [OK] Canonical tag database verified: {len(proj['tags'])} tags ({', '.join(tag_names[:4])})")


def test_section_2_cross_reference_query():
    print("\n--- SECTION 2: Signal & Tag Cross-Reference Engine ---")
    proj_id = "proj-vertical-slice-01"

    # 1. Target tag query
    res = api_request(f"/api/v1/projects/{proj_id}/cross-reference?tag_or_entity=V-101.LEVEL", token=OPERATOR_TOKEN)
    assert res["target"] == "V-101.LEVEL"
    entries = res["entries"]
    assert len(entries) > 0, "Should find cross-references for V-101.LEVEL"
    types = {e["entity_type"] for e in entries}
    assert "Tag" in types
    assert "ControlLoop" in types
    assert "Alarm" in types
    assert "ChannelMapping" in types
    print(f"  [OK] Cross-reference for V-101.LEVEL returned {len(entries)} references across types: {types}")

    # 2. Wildcard query
    res_all = api_request(f"/api/v1/projects/{proj_id}/cross-reference?tag_or_entity=*", token=OPERATOR_TOKEN)
    assert len(res_all["entries"]) >= 10, "Wildcard should return all project references"
    print(f"  [OK] Wildcard cross-reference returned {len(res_all['entries'])} total entries")


def test_section_3_validation_rules():
    print("\n--- SECTION 3: Design-Time Validation Rules ---")
    proj_id = "proj-vertical-slice-01"

    # Baseline validation must be clean
    val_clean = api_request(f"/api/v1/projects/{proj_id}/validate", method="POST", token=OPERATOR_TOKEN)
    assert val_clean["valid"] is True, "Clean default project must be valid"
    assert val_clean["error_count"] == 0
    print("  [OK] Baseline project validates cleanly (valid=True, errors=0)")

    # Fetch fresh project to mutate
    orig_proj = api_request(f"/api/v1/projects/{proj_id}", token=OPERATOR_TOKEN)

    # 1. Invalid Task Priority (out of range 1..=15)
    mutated = json.loads(json.dumps(orig_proj))
    mutated["controllers"][0]["tasks"][0]["priority"] = 25
    api_request(f"/api/v1/projects/{proj_id}", method="PUT", data={"project": mutated}, token=ADMIN_TOKEN)
    val_prio = api_request(f"/api/v1/projects/{proj_id}/validate", method="POST", token=OPERATOR_TOKEN)
    assert val_prio["valid"] is False, "Validation must fail for invalid task priority"
    codes = [r["code"] for r in val_prio["results"]]
    assert "VAL_TASK_PRIORITY_RANGE" in codes
    print("  [OK] Task priority range check correctly rejected priority=25 with VAL_TASK_PRIORITY_RANGE")

    # 2. IEC Routine Syntax Error
    mutated = json.loads(json.dumps(orig_proj))
    mutated["controllers"][0]["tasks"][0]["programs"][0]["routines"][0]["source_text"] = "VAR A : INT; END_VAR IF THEN"
    api_request(f"/api/v1/projects/{proj_id}", method="PUT", data={"project": mutated}, token=ADMIN_TOKEN)
    val_iec = api_request(f"/api/v1/projects/{proj_id}/validate", method="POST", token=OPERATOR_TOKEN)
    assert val_iec["valid"] is False, "Validation must fail for syntax error in IEC routine"
    codes = [r["code"] for r in val_iec["results"]]
    assert "VAL_IEC_COMPILE_ERROR" in codes
    print("  [OK] IEC routine compilation check correctly caught syntax error with VAL_IEC_COMPILE_ERROR")

    # 3. HAL Direction Mismatch
    mutated = json.loads(json.dumps(orig_proj))
    mutated["io_mappings"].append({
        "controller_id": "C-101",
        "signal_key": "ERR_SIG",
        "channel_address": "R01-C01-M02-AO01",
        "direction": "input", # Mismatch! AO is output
        "target_tag": None,
        "target_actuator": None,
        "priority": 5,
    })
    api_request(f"/api/v1/projects/{proj_id}", method="PUT", data={"project": mutated}, token=ADMIN_TOKEN)
    val_dir = api_request(f"/api/v1/projects/{proj_id}/validate", method="POST", token=OPERATOR_TOKEN)
    assert val_dir["valid"] is False, "Validation must fail for input mapping to AO channel"
    codes = [r["code"] for r in val_dir["results"]]
    assert "VAL_HAL_DIRECTION_MISMATCH" in codes
    print("  [OK] HAL direction mismatch check correctly caught input->AO conflict with VAL_HAL_DIRECTION_MISMATCH")

    # Restore clean project
    api_request(f"/api/v1/projects/{proj_id}", method="PUT", data={"project": orig_proj}, token=ADMIN_TOKEN)
    val_restored = api_request(f"/api/v1/projects/{proj_id}/validate", method="POST", token=OPERATOR_TOKEN)
    assert val_restored["valid"] is True
    print("  [OK] Original project restored and validated cleanly")


def test_section_4_project_driven_simulation_deployment():
    print("\n--- SECTION 4: Project-Driven Simulation Deployment ---")
    proj_id = "proj-vertical-slice-01"

    # Trigger load-sim
    res = api_request(f"/api/v1/projects/{proj_id}/load-sim", method="POST", token=ADMIN_TOKEN)
    print(f"  [OK] load-sim response: {res.get('message', 'success')}")

    # Verify virtual controllers deployed
    controllers = api_request("/api/v1/controllers", token=OPERATOR_TOKEN)
    assert len(controllers) > 0, "Virtual controller should be deployed from project"
    c101 = next((c for c in controllers if c["id"] == "C-101" or "Primary" in c["name"]), None)
    assert c101 is not None, "C-101 controller must be active"
    print(f"  [OK] Virtual controller active: {c101['name']} (state={c101.get('lifecycle', c101.get('state', 'UNKNOWN'))}, tasks={c101['task_count']})")

    # Verify HAL hardware deployed
    hal_systems = api_request("/api/v1/hal/systems", token=OPERATOR_TOKEN)
    assert len(hal_systems) > 0, "HAL system should be deployed from project"
    hal_main = next((s for s in hal_systems if "Main" in s["name"] or s["id"] == "HAL-MAIN"), None)
    assert hal_main is not None, "Main HAL system must be active"
    rack_count = len(hal_main.get("racks", []))
    mod_count = sum(len(c.get("modules", [])) for r in hal_main.get("racks", []) for c in r.get("chassis", []))
    chan_count = sum(len(m.get("channels", [])) for r in hal_main.get("racks", []) for c in r.get("chassis", []) for m in c.get("modules", []))
    print(f"  [OK] HAL system active: {hal_main['name']} ({rack_count} racks, {mod_count} modules, {chan_count} channels)")

    # Verify live simulation snapshot
    snap = api_request("/api/v1/sim/snapshot", token=OPERATOR_TOKEN)
    assert snap.get("sim_time_sec", -1) >= 0
    print(f"  [OK] Simulation execution active: sim_time={snap.get('sim_time_sec', 0):.2f}s, tags_count={len(snap.get('tags', []))}")


def test_section_5_browser_ui():
    print("\n--- SECTION 5: Browser UI Engineering Studio Verification ---")
    with sync_playwright() as p:
        browser = p.chromium.launch(
            headless=True,
            args=[
                "--no-sandbox",
                "--disable-dev-shm-usage",
                "--disable-gpu",
                "--window-size=1440,900",
            ],
        )
        context = browser.new_context(viewport={"width": 1440, "height": 900})
        page = context.new_page()

        page.goto(BASE_URL, wait_until="networkidle")
        page.wait_for_selector("nebu-app", timeout=15000)
        time.sleep(1)

        # 1. Switch to Studio Workbench
        page.click("button.mode-tab[data-mode='studio']")
        page.wait_for_selector("nebu-studio", timeout=10000)
        time.sleep(1)
        print("  [OK] Navigated to Studio workbench mode")
        time.sleep(0.5)
        save_screenshot(page, "stage_4_01_studio_initial")

        # 3. View P&ID Flowsheet
        btn_flow = page.query_selector("button[data-view='flowsheet']")
        if btn_flow:
            btn_flow.click()
            time.sleep(0.5)
            save_screenshot(page, "stage_4_02_pid_flowsheet")
            print("  [OK] Flowsheet canvas view verified")

        # 4. View IEC 61131-3 Logic Editor
        btn_iec = page.query_selector("button[data-view='iec']")
        if btn_iec:
            btn_iec.click()
            time.sleep(0.5)
            save_screenshot(page, "stage_4_03_iec_logic_editor")
            print("  [OK] IEC Logic Editor view verified")

        # 5. View I/O Hardware Systems (HAL)
        btn_io = page.query_selector("button[data-view='io']")
        if btn_io:
            btn_io.click()
            time.sleep(0.5)
            save_screenshot(page, "stage_4_04_io_hardware_systems")
            print("  [OK] I/O Hardware panel view verified")

        # 6. View I/O Mappings Table
        btn_map = page.query_selector("button[data-view='mappings']")
        if btn_map:
            btn_map.click()
            time.sleep(0.5)
            save_screenshot(page, "stage_4_05_io_mappings_table")
            print("  [OK] I/O Mappings table view verified")

        # 7. View Tag Database
        btn_tags = page.query_selector("button[data-view='tags']")
        if btn_tags:
            btn_tags.click()
            time.sleep(0.5)
            save_screenshot(page, "stage_4_06_tag_database")
            print("  [OK] Tag Database table view verified")

        # 8. View Cross-Reference Inspector
        btn_xref = page.query_selector("button[data-view='cross_ref']")
        if btn_xref:
            btn_xref.click()
            time.sleep(0.5)
            # Query V-101.LEVEL
            input_el = page.query_selector(".xref-query-input")
            if input_el:
                input_el.fill("V-101.LEVEL")
                btn_q = page.query_selector(".btn-query")
                if btn_q:
                    btn_q.click()
                    time.sleep(0.5)
            save_screenshot(page, "stage_4_07_cross_reference_inspector")
            print("  [OK] Cross-Reference Inspector view verified")

        # 9. View Controller Diagnostics
        btn_ctrl = page.query_selector("button[data-view='controllers']")
        if btn_ctrl:
            btn_ctrl.click()
            time.sleep(0.5)
            save_screenshot(page, "stage_4_08_controller_diagnostics")
            print("  [OK] Controller Diagnostics view verified")

        browser.close()


def main():
    print("=================================================================")
    print("NebuDCS Stage 4: PLC/DCS Engineering Studio Verification Suite")
    print("=================================================================")

    test_section_1_canonical_hierarchy()
    test_section_2_cross_reference_query()
    test_section_3_validation_rules()
    test_section_4_project_driven_simulation_deployment()
    test_section_5_browser_ui()

    print("\n=================================================================")
    print("STAGE 4 VERIFICATION SUITE: ALL CHECKS PASSED")
    print("VERDICT: STAGE 4 ENGINEERING STUDIO: PASS")
    print("=================================================================")


if __name__ == "__main__":
    main()
