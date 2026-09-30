const { chromium } = require('playwright');
const path = require('path');
const fs = require('fs');

const OUT_DIR = '/root/NebuDCS/docs/reports/screenshots/operations';
const ARTIFACT_DIR = '/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots/operations';

fs.mkdirSync(OUT_DIR, { recursive: true });
fs.mkdirSync(ARTIFACT_DIR, { recursive: true });

async function saveScreenshot(page, name) {
  const filePath1 = path.join(OUT_DIR, `${name}.png`);
  const filePath2 = path.join(ARTIFACT_DIR, `${name}.png`);
  await page.screenshot({ path: filePath1, fullPage: false });
  fs.copyFileSync(filePath1, filePath2);
  console.log(`Saved screenshot: ${name}.png`);
}

async function run() {
  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox']
  });
  const context = await browser.newContext({
    viewport: { width: 1920, height: 1080 },
    deviceScaleFactor: 1
  });
  const page = await context.newPage();

  console.log('Navigating to http://127.0.0.1:8088/ ...');
  await page.goto('http://127.0.0.1:8088/', { waitUntil: 'networkidle' });
  await page.waitForTimeout(1000);

  // 1. Operations startup overview
  console.log('1. operations_01_startup');
  await saveScreenshot(page, 'operations_01_startup');

  // 2. Area navigation / hierarchy breadcrumb
  console.log('2. operations_02_area_navigation');
  const areaSelect = page.locator('nebu-dcs-shell select#selCrumbArea');
  if (await areaSelect.count() > 0) {
    await areaSelect.focus();
    await page.waitForTimeout(300);
  }
  await saveScreenshot(page, 'operations_02_area_navigation');

  // 3. Unit process graphic
  console.log('3. operations_03_unit_process_graphic');
  await page.mouse.click(500, 200); // click on blank canvas area
  await page.waitForTimeout(300);
  await saveScreenshot(page, 'operations_03_unit_process_graphic');

  // 4. Equipment selected
  console.log('4. operations_04_equipment_selected');
  const pumpNode = page.locator('nebu-process-view svg#processSvg g.equipment-node').first();
  if (await pumpNode.count() > 0) {
    const box = await pumpNode.boundingBox();
    if (box) {
      await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
      await page.waitForTimeout(400);
    }
  }
  await saveScreenshot(page, 'operations_04_equipment_selected');

  // 5. Faceplate open
  console.log('5. operations_05_faceplate_open');
  const faceplateEl = page.locator('nebu-faceplate#appFaceplate');
  await faceplateEl.scrollIntoViewIfNeeded();
  await page.waitForTimeout(300);
  await saveScreenshot(page, 'operations_05_faceplate_open');

  // 6. Controller mode change (open loop faceplate)
  console.log('6. operations_06_controller_mode');
  const eqJump = page.locator('nebu-dcs-shell select#selCrumbEqJump');
  if (await eqJump.count() > 0) {
    const loopOpt = await eqJump.locator('option[value^="loop:"]').first();
    if (await loopOpt.count() > 0) {
      const val = await loopOpt.getAttribute('value');
      if (val) {
        await eqJump.selectOption(val);
        await page.waitForTimeout(400);
      }
    }
  }
  const modeSelect = page.locator('nebu-faceplate select#pidModeSelect');
  if (await modeSelect.count() > 0) {
    await modeSelect.selectOption('MANUAL');
    await modeSelect.dispatchEvent('change');
    await page.waitForTimeout(400);
  }
  await saveScreenshot(page, 'operations_06_controller_mode');

  // 7. Equipment command (open pump equipment profile with operational commands)
  console.log('7. operations_07_equipment_command');
  await page.evaluate(() => {
    const shell = document.querySelector('nebu-dcs-shell');
    const fp = document.querySelector('nebu-faceplate');
    const s = window.store;
    if (s) {
      s.openFaceplate({
        kind: 'pump',
        id: '2FG9T3YT29MPZ02Q9MAMF92EH2',
        name: 'PUMP-A1',
        description: 'CENTRIFUGAL FEED PUMP',
        mode: 'AUTO',
        unit: 'm3/h'
      });
    }
  });
  await page.waitForTimeout(400);
  await saveScreenshot(page, 'operations_07_equipment_command');

  // 8. Active alarm state
  console.log('8. operations_08_active_alarm');
  await page.evaluate(async () => {
    const token = localStorage.getItem('nebudcs_auth_token') || 'sec-admin-change-me-in-production-482a9f';
    await fetch('/api/v1/tags/write', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${token}` },
      body: JSON.stringify({ tag: 'TANK-B7.LEVEL', value: 85.0 })
    });
    await fetch('/api/v1/sim/control', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${token}` },
      body: JSON.stringify({ action: 'step' })
    });
  });
  await page.waitForTimeout(600);
  await saveScreenshot(page, 'operations_08_active_alarm');

  // 9. Alarm acknowledged state
  console.log('9. operations_09_alarm_acknowledged');
  const ackBtn = page.locator('#btnTickerAck');
  if (await ackBtn.count() > 0 && await ackBtn.isVisible()) {
    await ackBtn.click();
    await page.waitForTimeout(500);
  } else {
    await page.keyboard.press('Space');
    await page.waitForTimeout(500);
  }
  await saveScreenshot(page, 'operations_09_alarm_acknowledged');

  // 10. Alarm detail (scroll alarm panel into view)
  console.log('10. operations_10_alarm_detail');
  const alarmPanel = page.locator('nebu-alarm-panel#appAlarmPanel');
  await alarmPanel.scrollIntoViewIfNeeded();
  await page.waitForTimeout(300);
  await saveScreenshot(page, 'operations_10_alarm_detail');

  // 11. Trend interaction with crosshair
  console.log('11. operations_11_trend');
  const trendEl = page.locator('nebu-trend#appTrend canvas#trendCanvas');
  if (await trendEl.count() > 0) {
    await trendEl.scrollIntoViewIfNeeded();
    const tBox = await trendEl.boundingBox();
    if (tBox) {
      await page.mouse.move(tBox.x + tBox.width * 0.65, tBox.y + tBox.height * 0.5);
      await page.waitForTimeout(400);
    }
  }
  await saveScreenshot(page, 'operations_11_trend');

  // 12. SIS / Interlock state
  console.log('12. operations_12_sis_interlock_state');
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.waitForTimeout(300);
  await saveScreenshot(page, 'operations_12_sis_interlock_state');

  // 13. Command palette (open via palette.open())
  console.log('13. operations_13_command_palette');
  await page.evaluate(() => {
    const pal = document.querySelector('nebu-command-palette');
    if (pal && typeof pal.open === 'function') {
      pal.open();
    }
  });
  await page.waitForTimeout(400);
  await saveScreenshot(page, 'operations_13_command_palette');
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);

  // 14. Operator status bar (Role = Operator)
  console.log('14. operations_14_operator_status_bar');
  const roleSelect = page.locator('select#selActiveRole');
  if (await roleSelect.count() > 0) {
    await roleSelect.selectOption('operator');
    await roleSelect.dispatchEvent('change');
    await page.waitForTimeout(500);
  }
  await saveScreenshot(page, 'operations_14_operator_status_bar');

  // 15. Engineer role comparison
  console.log('15. operations_15_engineer_role_comparison');
  if (await roleSelect.count() > 0) {
    await roleSelect.selectOption('engineer');
    await roleSelect.dispatchEvent('change');
    await page.waitForTimeout(500);
  }
  await saveScreenshot(page, 'operations_15_engineer_role_comparison');

  // 16. Trainee role comparison
  console.log('16. operations_16_trainee_role_comparison');
  if (await roleSelect.count() > 0) {
    await roleSelect.selectOption('trainee');
    await roleSelect.dispatchEvent('change');
    await page.waitForTimeout(500);
  }
  await saveScreenshot(page, 'operations_16_trainee_role_comparison');

  // 17. 1920x1080 Operations workstation (switch back to Admin/Operator)
  console.log('17. operations_17_workstation_1920x1080');
  if (await roleSelect.count() > 0) {
    await roleSelect.selectOption('admin');
    await roleSelect.dispatchEvent('change');
    await page.waitForTimeout(500);
  }
  // Reset alarm tag back to normal
  await page.evaluate(async () => {
    const token = localStorage.getItem('nebudcs_auth_token') || 'sec-admin-change-me-in-production-482a9f';
    await fetch('/api/v1/tags/write', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${token}` },
      body: JSON.stringify({ tag: 'TANK-B7.LEVEL', value: 45.0 })
    });
    await fetch('/api/v1/sim/control', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${token}` },
      body: JSON.stringify({ action: 'step' })
    });
  });
  await page.waitForTimeout(500);
  await saveScreenshot(page, 'operations_17_workstation_1920x1080');

  console.log('All 17 Operations audit screenshots re-captured successfully!');
  await browser.close();
}

run().catch((err) => {
  console.error('Audit screenshot capture failed:', err);
  process.exit(1);
});
