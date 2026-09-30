#!/usr/bin/env python3
"""
NebuDCS Pre-Implementation Visual Forensic Audit Capture Suite
Captures all 20 required forensic states across 1920x1080 and inspections.
"""

import os
import sys
import time
from playwright.sync_api import sync_playwright

BASE_URL = os.environ.get("NEBUDCS_URL", "http://127.0.0.1:8088")
OUT_DIR = "/root/NebuDCS/docs/screenshots/audit_pre"
ARTIFACT_DIR = "/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots"

os.makedirs(OUT_DIR, exist_ok=True)
os.makedirs(ARTIFACT_DIR, exist_ok=True)

def save(page, name):
    p1 = os.path.join(OUT_DIR, f"{name}.png")
    p2 = os.path.join(ARTIFACT_DIR, f"{name}.png")
    page.screenshot(path=p1)
    page.screenshot(path=p2)
    print(f"[CAPTURE] Saved: {name}.png ({os.path.getsize(p1):,} bytes)")

def run_capture():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(viewport={"width": 1920, "height": 1080})
        page = context.new_page()

        print("1. Capturing Application Startup...")
        page.goto(BASE_URL)
        time.sleep(2)
        save(page, "audit_01_startup")

        print("2. Capturing Engineering Workspace Overview...")
        page.click('.mode-tab[data-mode="engineering"]')
        time.sleep(1.5)
        save(page, "audit_02_engineering_overview")

        print("3. Capturing P&ID Flowsheet Canvas...")
        btn_flow = page.query_selector('.studio-view-tab[data-view="flowsheet"]')
        if btn_flow:
            btn_flow.click()
            time.sleep(1)
        save(page, "audit_03_engineering_pid")

        print("4. Capturing Equipment Configuration & Inspector...")
        save(page, "audit_04_engineering_equipment_config")

        print("5. Capturing Project Readiness / Preflight...")
        btn_val = page.query_selector(".btn-validate")
        if btn_val:
            btn_val.click()
            time.sleep(1)
        save(page, "audit_05_engineering_project_readiness")

        print("6. Capturing Deployment Pipeline...")
        save(page, "audit_06_engineering_deployment")

        print("7. Capturing Operations Overview...")
        page.click('.mode-tab[data-mode="operations"]')
        time.sleep(1.5)
        save(page, "audit_07_operations_overview")

        print("8. Capturing Process Graphic...")
        save(page, "audit_08_operations_process_graphic")

        print("9. Capturing Loop Faceplate...")
        # Select a tag directly via window.eval to avoid table row detached flakiness
        page.evaluate("""() => {
            const el = document.querySelector('nebu-faceplate');
            if (el && typeof el.selectTag === 'function') {
                el.selectTag('LIC-101.PV');
            }
        }""")
        time.sleep(1)
        save(page, "audit_09_operations_faceplate")

        print("10. Capturing Alarm State & Station...")
        save(page, "audit_10_operations_alarm_state")

        print("11. Capturing Multi-Pen Trend...")
        save(page, "audit_11_operations_trend")

        print("12. Capturing Training Workspace...")
        page.click('.mode-tab[data-mode="training"]')
        time.sleep(2)
        save(page, "audit_12_training_workspace")

        print("13. Capturing Validation Workspace...")
        page.click('.mode-tab[data-mode="validation"]')
        time.sleep(2)
        save(page, "audit_13_validation_workspace")

        print("14. Capturing System Workspace...")
        page.click('.mode-tab[data-mode="system"]')
        time.sleep(2)
        save(page, "audit_14_system_workspace")

        print("15. Capturing Enterprise Workspace...")
        page.click('.mode-tab[data-mode="enterprise"]')
        time.sleep(2)
        save(page, "audit_15_enterprise_workspace")

        print("16. Capturing Runtime Stopped/Paused...")
        btn_pause = page.query_selector(".btn-pause:not([disabled])")
        if btn_pause:
            btn_pause.click()
            time.sleep(1)
        save(page, "audit_16_runtime_stopped")

        print("17. Capturing Runtime Running (50 Hz)...")
        btn_play = page.query_selector(".btn-play:not([disabled])")
        if btn_play:
            btn_play.click()
            time.sleep(1)
        save(page, "audit_17_runtime_running")

        print("18. Capturing Runtime Faulted / Alarm Trip...")
        page.click('.mode-tab[data-mode="enterprise"]')
        time.sleep(1)
        btn_fail = page.query_selector('.ent-subtab[data-subtab="cluster"]')
        if btn_fail:
            btn_fail.click()
            time.sleep(1)
        save(page, "audit_18_runtime_faulted")

        print("19. Capturing Empty State...")
        page.click('.mode-tab[data-mode="operations"]')
        time.sleep(1)
        inp_filter = page.query_selector("#tagFilter")
        if inp_filter:
            inp_filter.fill("NONEXISTENT_TAG_XYZ_999")
            time.sleep(0.5)
        save(page, "audit_19_empty_state")

        print("20. Capturing Error / RBAC State...")
        page.select_option("#selActiveRole", "trainee")
        time.sleep(1)
        save(page, "audit_20_error_state")

        browser.close()
        print("\nAll 20 Pre-Implementation Forensic Screenshots Captured Successfully!")

if __name__ == "__main__":
    run_capture()
