import os
import shutil
import time
from playwright.sync_api import sync_playwright

OUT_DIR = '/root/NebuDCS/docs/reports/screenshots/stage_15_cybersecurity'
ARTIFACT_DIR = '/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots/stage_15_cybersecurity'

os.makedirs(OUT_DIR, exist_ok=True)
os.makedirs(ARTIFACT_DIR, exist_ok=True)

def save_screenshot(page, name):
    p1 = os.path.join(OUT_DIR, f"{name}.png")
    p2 = os.path.join(ARTIFACT_DIR, f"{name}.png")
    page.screenshot(path=p1, full_page=False)
    shutil.copyfile(p1, p2)
    print(f"Saved screenshot: {name}.png")

def main():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True, args=['--no-sandbox', '--disable-setuid-sandbox'])
        context = browser.new_context(viewport={'width': 1920, 'height': 1080}, device_scale_factor=1)
        page = context.new_page()

        print("Navigating to http://127.0.0.1:8088/ ...")
        page.goto("http://127.0.0.1:8088/", wait_until="networkidle")
        page.wait_for_timeout(2000)

        # Authenticate with admin token and switch to Enterprise -> Security
        page.evaluate("""() => {
            localStorage.setItem('nebudcs_auth_token', 'sec-admin-change-me-in-production-482a9f');
            const app = document.querySelector('nebu-app');
            if (app) {
                if (app.apiClient) {
                    app.apiClient.setAuthToken('sec-admin-change-me-in-production-482a9f');
                }
                app.activeMode = 'enterprise';
                app.enterpriseSubTab = 'security';
                app.render();
            }
        }""")
        page.wait_for_timeout(2000)

        # 1. Cybersecurity Overview
        print("1. stage15_01_cybersecurity_overview")
        save_screenshot(page, "stage15_01_cybersecurity_overview")

        # 2. Select a Purdue Model Zone
        print("2. stage15_02_purdue_model_zones")
        page.evaluate("""() => {
            const row = document.querySelector('nebu-cybersecurity-panel .purdue-zone-row[data-zone-id="zone-l2-control"]');
            if (row) row.click();
        }""")
        page.wait_for_timeout(800)
        save_screenshot(page, "stage15_02_purdue_model_zones")

        # 3. Conduits & Data Diode
        print("3. stage15_03_conduits_and_data_diode")
        save_screenshot(page, "stage15_03_conduits_and_data_diode")

        # 4. Cryptographic Audit Hash Verification Click
        print("4. stage15_04_cryptographic_audit_verified")
        page.evaluate("""() => {
            const btn = document.querySelector('nebu-cybersecurity-panel #btnVerifyAudit');
            if (btn) btn.click();
        }""")
        page.wait_for_timeout(1500)
        save_screenshot(page, "stage15_04_cryptographic_audit_verified")

        # 5. System Workspace Security Tab
        print("5. stage15_05_system_workspace_security_tab")
        page.evaluate("""() => {
            const app = document.querySelector('nebu-app');
            if (app) {
                app.activeMode = 'system';
                app.render();
            }
        }""")
        page.wait_for_timeout(800)
        page.evaluate("""() => {
            const sysView = document.querySelector('nebu-system-view');
            if (sysView) {
                const btn = sysView.querySelector('button[data-subtab="security"]');
                if (btn) btn.click();
            }
        }""")
        page.wait_for_timeout(1500)
        save_screenshot(page, "stage15_05_system_workspace_security_tab")

        browser.close()
        print("All Stage 15 screenshots captured successfully.")

if __name__ == "__main__":
    main()
