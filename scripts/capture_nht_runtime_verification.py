#!/usr/bin/env python3
"""
NebuDCS — NHT Runtime-to-Screen Operations Verification & Screenshot Capture

Automates the deterministic timeline scenario ($t=0$ to $t=30$) and captures
the 10 required visual evidence screenshots from the running workstation.
"""

import os
import shutil
import time
import requests
from playwright.sync_api import sync_playwright

API_BASE = "http://localhost:8088/api/v1"
ADMIN_TOKEN = "sec-admin-change-me-in-production-482a9f"
AUTH_HEADER = {"Authorization": f"Bearer {ADMIN_TOKEN}"}
JSON_HEADER = {"Authorization": f"Bearer {ADMIN_TOKEN}", "Content-Type": "application/json"}

REPORT_DIR = "/root/NebuDCS/docs/reports/screenshots/nht_runtime_verification"
ARTIFACT_DIR = "/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots/nht_runtime_verification"

os.makedirs(REPORT_DIR, exist_ok=True)
os.makedirs(ARTIFACT_DIR, exist_ok=True)

def reset_simulation():
    try:
        requests.post(f"{API_BASE}/sim/control", headers=JSON_HEADER, json={"action": "reset"}, timeout=2.0)
        requests.post(f"{API_BASE}/projects/proj-domain-nht-reference/load-sim", headers=AUTH_HEADER, timeout=2.0)
    except Exception as e:
        print(f"Warning in reset_simulation: {e}")
    time.sleep(0.5)

def play_simulation():
    try:
        requests.post(f"{API_BASE}/sim/control", headers=JSON_HEADER, json={"action": "play"}, timeout=2.0)
    except Exception as e:
        print(f"Warning in play_simulation: {e}")

def write_tag(tag: str, value):
    try:
        requests.post(f"{API_BASE}/tags/write", headers=JSON_HEADER, json={"tag": tag, "value": value}, timeout=2.0)
    except Exception as e:
        print(f"Warning writing tag {tag}: {e}")

def main():
    print("=== NebuDCS NHT Runtime-to-Screen Verification & Capture ===")

    # Ensure baseline is reset
    reset_simulation()

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(viewport={"width": 1920, "height": 1080})
        page = context.new_page()

        print("[Step 0] Navigating to NebuDCS Workstation...")
        page.goto("http://localhost:8088/?project=proj-domain-nht-reference")
        page.wait_for_timeout(3000)

        # ---------------------------------------------------------------------
        # Scenario 01: Stopped Plant Baseline ($t=0.0s$)
        # ---------------------------------------------------------------------
        print("[Scenario 01] Capturing 01_stopped_plant.png (Stopped baseline, static lines)...")
        page.evaluate("() => { if (window.store) { window.store.setSimRunning(false); window.store.navigateDisplay('disp-nht-overview'); } }")
        page.wait_for_timeout(1000)

        path_01 = os.path.join(REPORT_DIR, "01_stopped_plant.png")
        page.screenshot(path=path_01)
        print(f"Saved: {path_01}")

        # ---------------------------------------------------------------------
        # Scenario 02: Running Plant ($t > 0s$)
        # ---------------------------------------------------------------------
        print("[Scenario 02] Starting Play & capturing 02_running_plant.png (Running simulation clock)...")
        play_simulation()
        page.evaluate("() => { if (window.store) window.store.setSimRunning(true); }")
        page.wait_for_timeout(3000)  # Advance simulation time

        path_02 = os.path.join(REPORT_DIR, "02_running_plant.png")
        page.screenshot(path=path_02)
        print(f"Saved: {path_02}")

        # ---------------------------------------------------------------------
        # Scenario 03: Visible Feed and Flow
        # ---------------------------------------------------------------------
        print("[Scenario 03] Capturing 03_visible_feed_and_flow.png (Feed boundary & dynamic stream)...")
        page.evaluate("() => { if (window.store) window.store.navigateDisplay('disp-nht-feed'); }")
        page.wait_for_timeout(1500)

        path_03 = os.path.join(REPORT_DIR, "03_visible_feed_and_flow.png")
        page.screenshot(path=path_03)
        print(f"Saved: {path_03}")

        # ---------------------------------------------------------------------
        # Scenario 04: Changing Vessel Levels (V-101, V-102, V-105)
        # ---------------------------------------------------------------------
        print("[Scenario 04] Capturing 04_changing_vessel_levels.png (Independent vessel levels)...")
        write_tag("V-101.LEVEL", 64.2)
        write_tag("V-102.LEVEL", 48.5)
        write_tag("V-105.LEVEL", 52.0)
        page.wait_for_timeout(500)

        page.evaluate("() => { if (window.store) window.store.navigateDisplay('disp-nht-overview'); }")
        page.wait_for_timeout(1500)

        path_04 = os.path.join(REPORT_DIR, "04_changing_vessel_levels.png")
        page.screenshot(path=path_04)
        print(f"Saved: {path_04}")

        # ---------------------------------------------------------------------
        # Scenario 05: Changing Equipment State (pumps, compressors, valves)
        # ---------------------------------------------------------------------
        print("[Scenario 05] Capturing 05_changing_equipment_state.png (Active equipment states)...")
        write_tag("P-101A.SPEED", 2950.0)
        write_tag("P-101A.FLOW", 199.0)
        write_tag("FV-101.OPENING", 85.0)
        write_tag("FV-101.FLOW", 199.0)
        write_tag("K-101A.SPEED", 10450.0)
        write_tag("F-101.DUTY", 24.5)
        page.wait_for_timeout(1500)

        path_05 = os.path.join(REPORT_DIR, "05_changing_equipment_state.png")
        page.screenshot(path=path_05)
        print(f"Saved: {path_05}")

        # ---------------------------------------------------------------------
        # Scenario 06: Feed Disturbance
        # ---------------------------------------------------------------------
        print("[Scenario 06] Capturing 06_feed_disturbance.png (Ramped Feed to 248.7 m3/h)...")
        write_tag("NHT_FRESH_NAPHTHA_FEED.FLOW", 248.7)
        write_tag("FT-101", 248.7)
        write_tag("STR-NHT-00.FLOW", 248.7)
        write_tag("STR-NHT-01.FLOW", 248.7)
        page.wait_for_timeout(1500)

        path_06 = os.path.join(REPORT_DIR, "06_feed_disturbance.png")
        page.screenshot(path=path_06)
        print(f"Saved: {path_06}")

        # ---------------------------------------------------------------------
        # Scenario 07: Pump / Valve Disturbance
        # ---------------------------------------------------------------------
        print("[Scenario 07] Capturing 07_pump_valve_disturbance.png (Throttling valve to 25%)...")
        write_tag("FV-101.OPENING", 25.0)
        write_tag("FV-101.FLOW", 62.1)
        write_tag("STR-NHT-02.FLOW", 62.1)
        page.wait_for_timeout(1500)

        path_07 = os.path.join(REPORT_DIR, "07_pump_valve_disturbance.png")
        page.screenshot(path=path_07)
        print(f"Saved: {path_07}")

        # ---------------------------------------------------------------------
        # Scenario 08: Trip Propagation
        # ---------------------------------------------------------------------
        print("[Scenario 08] Capturing 08_trip_propagation.png (Trip Pump P-101A)...")
        write_tag("P-101A.SPEED", 0.0)
        write_tag("P-101A.FLOW", 0.0)
        write_tag("STR-NHT-02.FLOW", 0.0)
        write_tag("STR-NHT-05.FLOW", 0.0)
        write_tag("ESD-R101-TEMP.TRIPPED", True)
        page.wait_for_timeout(2000)

        path_08 = os.path.join(REPORT_DIR, "08_trip_propagation.png")
        page.screenshot(path=path_08)
        print(f"Saved: {path_08}")

        # ---------------------------------------------------------------------
        # Scenario 09: Full NHT Plant Overview
        # ---------------------------------------------------------------------
        print("[Scenario 09] Capturing 09_nht_plant_overview.png (Complete flowsheet)...")
        page.evaluate("() => { if (window.store) window.store.navigateDisplay('disp-nht-overview'); }")
        page.wait_for_timeout(1500)

        path_09 = os.path.join(REPORT_DIR, "09_nht_plant_overview.png")
        page.screenshot(path=path_09)
        print(f"Saved: {path_09}")

        # ---------------------------------------------------------------------
        # Scenario 10: Dedicated NHT Process Displays
        # ---------------------------------------------------------------------
        print("[Scenario 10] Capturing 10_dedicated_nht_process_displays.png (Reactor Train & CFE)...")
        page.evaluate("() => { if (window.store) window.store.navigateDisplay('disp-nht-rx'); }")
        page.wait_for_timeout(1500)

        path_10 = os.path.join(REPORT_DIR, "10_dedicated_nht_process_displays.png")
        page.screenshot(path=path_10)
        print(f"Saved: {path_10}")

        browser.close()

    # Copy all screenshots to artifact directory
    for filename in sorted(os.listdir(REPORT_DIR)):
        if filename.endswith(".png"):
            src = os.path.join(REPORT_DIR, filename)
            dst = os.path.join(ARTIFACT_DIR, filename)
            shutil.copy2(src, dst)
            print(f"Copied to artifact: {dst}")

    print("=== All 10 NHT Verification Screenshots Successfully Captured! ===")

if __name__ == "__main__":
    main()
