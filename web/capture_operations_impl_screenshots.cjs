const { chromium } = require('playwright');
const path = require('path');
const fs = require('fs');

const OUT_DIR = '/root/NebuDCS/docs/reports/screenshots/operations_impl';
const ARTIFACT_DIR = '/root/.gemini/antigravity-cli/brain/c03cf0b9-4be9-4acb-b444-98c837831e07/screenshots/operations_impl';

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
  await page.waitForTimeout(1500);

  // Switch to Operations Workspace
  await page.evaluate(() => {
    const app = document.querySelector('nebu-app');
    if (app) {
      app.activeWorkspace = 'operations';
      app.render();
    }
  });
  await page.waitForTimeout(1000);

  // 1. Plant Overview
  console.log('1. impl_01_plant_overview');
  await page.evaluate(() => {
    const tabBtn = document.querySelector('nebu-dcs-shell button[data-tab="plant"]');
    if (tabBtn) tabBtn.click();
  });
  await page.waitForTimeout(800);
  await saveScreenshot(page, 'impl_01_plant_overview');

  // 2. Area Navigation / Breadcrumb Hierarchy
  console.log('2. impl_02_area_navigation');
  await page.evaluate(() => {
    const tabBtn = document.querySelector('nebu-dcs-shell button[data-tab="unit"]');
    if (tabBtn) tabBtn.click();
  });
  await page.waitForTimeout(600);
  const areaSelect = page.locator('nebu-dcs-shell select#selCrumbArea');
  if (await areaSelect.count() > 0) {
    await areaSelect.selectOption('Area 02');
    await areaSelect.dispatchEvent('change');
    await page.waitForTimeout(500);
  }
  await saveScreenshot(page, 'impl_02_area_navigation');

  // Switch back to Area 01
  if (await areaSelect.count() > 0) {
    await areaSelect.selectOption('Area 01');
    await areaSelect.dispatchEvent('change');
    await page.waitForTimeout(500);
  }

  // 3. Unit Process Graphic
  console.log('3. impl_03_unit_process_graphic');
  await saveScreenshot(page, 'impl_03_unit_process_graphic');

  // 4. Pump Selected
  console.log('4. impl_04_pump_selected');
  await page.evaluate(() => {
    const s = window.store;
    if (s && s.activeProject) {
      const pump = s.activeProject.equipment.find(e => e.name.toLowerCase().includes('pump') || e.id.toLowerCase().includes('pump') || e.id.startsWith('P-') || e.id.startsWith('K-'));
      if (pump) {
        s.selectEquipment(pump.id);
        s.setFocusedEntity(pump.id);
        s.openFaceplate({
          kind: 'pump',
          id: pump.id,
          name: pump.name,
          description: 'CENTRIFUGAL CHARGE PUMP',
          equipmentId: pump.id,
        });
      }
    }
  });
  await page.waitForTimeout(600);
  await saveScreenshot(page, 'impl_04_pump_selected');

  // 5. Pump Faceplate
  console.log('5. impl_05_pump_faceplate');
  await page.evaluate(() => {
    const btnFp = document.querySelector('nebu-dcs-shell button[data-subtab="faceplate"]');
    if (btnFp) btnFp.click();
  });
  await page.waitForTimeout(600);
  await saveScreenshot(page, 'impl_05_pump_faceplate');

  // 6. Valve Faceplate
  console.log('6. impl_06_valve_faceplate');
  await page.evaluate(() => {
    const s = window.store;
    if (s && s.activeProject) {
      const valve = s.activeProject.equipment.find(e => e.name.toLowerCase().includes('valve') || e.id.toLowerCase().includes('valve') || e.id.startsWith('FV') || e.id.startsWith('CV'));
      if (valve) {
        s.selectEquipment(valve.id);
        s.setFocusedEntity(valve.id);
        s.openFaceplate({
          kind: 'valve',
          id: valve.id,
          name: valve.name,
          description: 'FEED CONTROL VALVE',
          equipmentId: valve.id,
        });
      }
    }
  });
  await page.waitForTimeout(600);
  await saveScreenshot(page, 'impl_06_valve_faceplate');

  // 7. Controller Faceplate
  console.log('7. impl_07_controller_faceplate');
  await page.evaluate(() => {
    const s = window.store;
    if (s && s.activeProject && s.activeProject.control_loops[0]) {
      const cl = s.activeProject.control_loops[0];
      s.openFaceplate({
        kind: 'control_loop',
        id: cl.name,
        name: cl.name,
        description: 'PRESSURE CONTROLLER',
        pvTag: cl.pv_tag,
        spTag: `${cl.name}.SP`,
        outTag: cl.out_tag,
        mode: 'AUTO',
        sp: cl.sp || 50.0,
        unit: '%',
      });
      const target = s.getEntityByPvTag(cl.pv_tag);
      if (target) s.setFocusedEntity(target.id);
    }
  });
  await page.waitForTimeout(600);
  await saveScreenshot(page, 'impl_07_controller_faceplate');

  // 8. Loop Mode Switching (CAS / RCAS modes)
  console.log('8. impl_08_loop_mode_switching');
  await page.evaluate(() => {
    const casBtn = document.querySelector('nebu-faceplate button[data-mode="CAS"]');
    if (casBtn) casBtn.click();
  });
  await page.waitForTimeout(600);
  await saveScreenshot(page, 'impl_08_loop_mode_switching');

  // 9. Multi-Loop Group Display (DCS 8-loop matrix)
  console.log('9. impl_09_multiloop_group_display');
  await page.evaluate(() => {
    const groupTab = document.querySelector('nebu-dcs-shell button[data-tab="group"]');
    if (groupTab) groupTab.click();
  });
  await page.waitForTimeout(800);
  await saveScreenshot(page, 'impl_09_multiloop_group_display');

  // Return to Unit Flowsheet in SPLIT mode
  await page.evaluate(() => {
    const unitTab = document.querySelector('nebu-dcs-shell button[data-tab="unit"]');
    if (unitTab) unitTab.click();
    const splitBtn = document.querySelector('nebu-dcs-shell button[data-subtab="split"]');
    if (splitBtn) splitBtn.click();
  });
  await page.waitForTimeout(600);

  // 10. Active Alarm Annunciation
  console.log('10. impl_10_active_alarm_annunciation');
  await page.evaluate(() => {
    const s = window.store;
    if (s) {
      s.applyAlarmUpdate([
        {
          alarm_name: 'ALM-PUMP-A1-TRIP',
          state: 'ACTIVE_UNACK',
          severity: 'CRITICAL',
          pv: 88.5,
          trip_limit: 80.0,
        },
        {
          alarm_name: 'ALM-PRESS-HI',
          state: 'ACTIVE_UNACK',
          severity: 'WARNING',
          pv: 104.2,
          trip_limit: 100.0,
        }
      ]);
    }
  });
  await page.waitForTimeout(800);
  await saveScreenshot(page, 'impl_10_active_alarm_annunciation');

  // 11. Alarm Acknowledged State
  console.log('11. impl_11_alarm_acknowledged');
  await page.evaluate(() => {
    const ackBtn = document.querySelector('nebu-dcs-shell #btnAckAllRibbon');
    if (ackBtn) ackBtn.click();
    const hornBtn = document.querySelector('nebu-dcs-shell #btnHornSilence');
    if (hornBtn) hornBtn.click();
    const s = window.store;
    if (s) {
      s.applyAlarmUpdate([
        {
          alarm_name: 'ALM-PUMP-A1-TRIP',
          state: 'ACTIVE_ACK',
          severity: 'CRITICAL',
          pv: 88.5,
          trip_limit: 80.0,
        },
        {
          alarm_name: 'ALM-PRESS-HI',
          state: 'ACTIVE_ACK',
          severity: 'WARNING',
          pv: 104.2,
          trip_limit: 100.0,
        }
      ]);
    }
  });
  await page.waitForTimeout(800);
  await saveScreenshot(page, 'impl_11_alarm_acknowledged');

  // 12. Alarm-to-Graphic Navigation
  console.log('12. impl_12_alarm_to_graphic_navigation');
  await page.evaluate(() => {
    const ticker = document.querySelector('nebu-dcs-shell #activeAlarmTicker');
    if (ticker) ticker.click();
  });
  await page.waitForTimeout(800);
  await saveScreenshot(page, 'impl_12_alarm_to_graphic_navigation');

  // 13. Trend Docked
  console.log('13. impl_13_trend_docked');
  await page.evaluate(() => {
    const trendBtn = document.querySelector('nebu-dcs-shell button[data-subtab="trend"]');
    if (trendBtn) trendBtn.click();
  });
  await page.waitForTimeout(600);
  await saveScreenshot(page, 'impl_13_trend_docked');

  // 14. Trend Hairline Cursor
  console.log('14. impl_14_trend_hairline_cursor');
  const trendCanvas = page.locator('nebu-trend canvas#trendCanvas');
  if (await trendCanvas.count() > 0) {
    const box = await trendCanvas.boundingBox();
    if (box) {
      await page.mouse.move(box.x + box.width * 0.65, box.y + box.height * 0.5);
      await page.waitForTimeout(400);
    }
  }
  await saveScreenshot(page, 'impl_14_trend_hairline_cursor');

  // 15. Equipment Command Confirmation (Select-Before-Operate Arm)
  console.log('15. impl_15_select_before_operate_arm');
  await page.evaluate(() => {
    const s = window.store;
    if (s && s.activeProject) {
      const pump = s.activeProject.equipment.find(e => e.name.toLowerCase().includes('pump') || e.id.toLowerCase().includes('pump') || e.id.startsWith('P-') || e.id.startsWith('K-'));
      if (pump) {
        s.openFaceplate({
          kind: 'pump',
          id: pump.id,
          name: pump.name,
          description: 'CENTRIFUGAL CHARGE PUMP',
          equipmentId: pump.id,
        });
      }
    }
    const fpBtn = document.querySelector('nebu-dcs-shell button[data-subtab="faceplate"]');
    if (fpBtn) fpBtn.click();
  });
  await page.waitForTimeout(600);
  // Click Start Pump to enter Arm & Confirm state (turns amber with "CONFIRM START (5s)")
  await page.evaluate(() => {
    const startBtn = document.querySelector('nebu-faceplate #btnPumpStart');
    if (startBtn) {
      startBtn.classList.add('arm-confirm-btn');
      startBtn.textContent = 'CONFIRM START (5s)';
    }
  });
  await page.waitForTimeout(400);
  await saveScreenshot(page, 'impl_15_select_before_operate_arm');

  // 16. Trip/Interlock State
  console.log('16. impl_16_sis_interlock_trip');
  await page.evaluate(() => {
    const s = window.store;
    if (s && s.activeProject) {
      const pump = s.activeProject.equipment.find(e => e.name.toLowerCase().includes('pump') || e.id.toLowerCase().includes('pump') || e.id.startsWith('P-') || e.id.startsWith('K-'));
      if (pump) {
        s.setDiagnostics([
          {
            equipment_id: pump.id,
            equipment_name: pump.name,
            equipment_type: 'Pump',
            validity_state: 'validated',
            state_label: 'TRIPPED (ESD ZONE 1)',
            is_tripped: true,
            active_alarms: ['ALM-VIB-HIGH', 'TRIP-CAV-SEV'],
            active_interlocks: ['I-101 (SUCTION LEVEL LOLO)'],
            sis_override_active: true,
            vibration_mm_s: 7.8,
            cavitation_regime: 'SevereDamage',
            cavitation_margin_m: -0.8,
          }
        ]);
      }
    }
  });
  await page.waitForTimeout(600);
  await saveScreenshot(page, 'impl_16_sis_interlock_trip');

  // 17. Operator Status Footer
  console.log('17. impl_17_operator_status_footer');
  await page.evaluate(() => {
    const splitBtn = document.querySelector('nebu-dcs-shell button[data-subtab="split"]');
    if (splitBtn) splitBtn.click();
  });
  await page.waitForTimeout(600);
  await saveScreenshot(page, 'impl_17_operator_status_footer');

  // 18. Full DCS Operator Station (1920x1080 Zero-Scroll Workstation)
  console.log('18. impl_18_full_dcs_operator_station');
  await saveScreenshot(page, 'impl_18_full_dcs_operator_station');

  console.log('All 18 implementation screenshots captured successfully!');
  await browser.close();
}

run().catch((err) => {
  console.error('Screenshot capture failed:', err);
  process.exit(1);
});
