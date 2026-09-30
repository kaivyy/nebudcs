#!/usr/bin/env python3
"""
NebuDCS Post-Implementation Visual Acceptance Capture Suite
Captures key states showing redesigned UI elements:
- Global Command Palette (Ctrl+K)
- Persistent 2-Row ISA-18.2 Alarm Annunciator Bar
- Anchored Flowsheet Centerpiece & Docked Bottom Drawer
- Capability-Driven Equipment Inspector Tabs
- Interactive Operations Breadcrumbs with Equipment Jump
- Strict Trainee Role Containment
- Workspaces: Engineering, Operations, Training, Validation, System, Enterprise
"""

import os
import sys
import time
from playwright.sync_api import sync_playwright

BASE_URL = os.environ.get("NEBUDCS_URL", "http://127.0.0.1:8088")
OUT_DIR = "/root/NebuDCS/docs/screenshots/redesign_post"
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

        print("1. Capturing Redesigned Startup & 2-Row Annunciator Bar...")
        page.goto(BASE_URL)
        time.sleep(2)
        save(page, "redesign_01_startup")

        print("2. Capturing Command Palette (Ctrl+K)...")
        page.keyboard.press("Control+k")
        time.sleep(1)
        # Type search query in palette
        inp = page.query_selector(".cmd-palette-input")
        if inp:
            inp.fill("V-101")
            time.sleep(0.5)
        save(page, "redesign_02_cmd_palette")
        page.keyboard.press("Escape")
        time.sleep(0.5)

        print("3. Capturing Engineering Studio (Anchored Canvas & Readiness Bar)...")
        page.click('.mode-tab[data-mode="engineering"]')
        time.sleep(1.5)
        save(page, "redesign_03_engineering_studio")

        print("4. Capturing Capability-Driven Equipment Inspector...")
        # Click an equipment card or port to focus inspector
        eq_card = page.query_selector(".equipment-card")
        if eq_card:
            eq_card.click()
            time.sleep(0.5)
        save(page, "redesign_04_equipment_inspector_tabs")

        print("5. Capturing Studio Docked Bottom Drawer (IEC/IO/Tags)...")
        btn_drawer_tab = page.query_selector('.drawer-nav-btn[data-tool="iec"]')
        if btn_drawer_tab:
            btn_drawer_tab.click()
            time.sleep(1)
        save(page, "redesign_05_studio_bottom_drawer_open")

        print("6. Capturing Operations Interactive Breadcrumbs...")
        page.click('.mode-tab[data-mode="operations"]')
        time.sleep(1.5)
        save(page, "redesign_06_operations_breadcrumbs")

        print("7. Capturing Operations Faceplate & Annunciator Bar...")
        page.evaluate("""() => {
            const el = document.querySelector('nebu-faceplate');
            if (el && typeof el.selectTag === 'function') {
                el.selectTag('LIC-101.PV');
            }
        }""")
        time.sleep(1)
        save(page, "redesign_07_faceplate_and_annunciator")

        print("8. Capturing Training Trainee Containment (RBAC)...")
        page.select_option("#selActiveRole", "trainee")
        time.sleep(0.5)
        page.click('.mode-tab[data-mode="training"]')
        time.sleep(1.5)
        save(page, "redesign_08_training_trainee_containment")

        # Switch back to engineer/admin
        page.select_option("#selActiveRole", "engineer")
        time.sleep(0.5)

        print("9. Capturing Validation Workspace...")
        page.click('.mode-tab[data-mode="validation"]')
        time.sleep(2)
        save(page, "redesign_09_validation_workspace")

        print("10. Capturing System Observability Workspace...")
        page.click('.mode-tab[data-mode="system"]')
        time.sleep(2)
        save(page, "redesign_10_system_observability")

        print("11. Capturing Enterprise Workspace...")
        page.click('.mode-tab[data-mode="enterprise"]')
        time.sleep(2)
        save(page, "redesign_11_enterprise_workspace")

        print("12. Capturing Operator Role Context & Ticker...")
        page.select_option("#selActiveRole", "operator")
        time.sleep(0.5)
        page.click('.mode-tab[data-mode="operations"]')
        time.sleep(1)
        save(page, "redesign_12_operator_role_context")

        browser.close()
        print("\nAll Redesign Visual Acceptance Screenshots Captured Successfully!")

if __name__ == "__main__":
    run_capture()
