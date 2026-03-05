import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import {
  executiveFixtureHistory,
  executiveFixtureSnapshot
} from './dashboard_fixture_data.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const FRONTEND_ROOT = path.resolve(__dirname, '..');
const DASHBOARD_URL = pathToFileURL(path.resolve(FRONTEND_ROOT, 'index.html')).href;

test('dashboard smoke renders diagnostics, KPI formulas, and chart metadata', { timeout: 30000 }, async (t) => {
  let playwright;
  try {
    playwright = await import('playwright');
  } catch {
    t.skip('playwright is not installed in this environment');
    return;
  }
  const playwrightApi = playwright.default || playwright;

  let browser;
  try {
    browser = await playwrightApi.chromium.launch({ headless: true });
  } catch (error) {
    t.skip(`playwright browser unavailable: ${error.message}`);
    return;
  }

  const context = await browser.newContext({ viewport: { width: 1600, height: 1400 } });
  const page = await context.newPage();
  t.after(async () => {
    await context.close();
    await browser.close();
  });

  await page.addInitScript(
    ({ snapshot, history }) => {
      window.__DASHBOARD_RENDER_EVENTS__ = [];
      window.addEventListener('dashboard:rendered', (event) => {
        window.__DASHBOARD_RENDER_EVENTS__.push(event.detail);
      });

      window.__TAURI__ = {
        core: {
          invoke: async (command) => {
            if (command === 'get_text_workflow_templates') return [];
            if (command === 'get_mock_analytics_run_history') return history;
            if (command === 'get_executive_dashboard_snapshot') return snapshot;
            return {};
          }
        }
      };
    },
    {
      snapshot: executiveFixtureSnapshot,
      history: executiveFixtureHistory
    }
  );

  await page.goto(DASHBOARD_URL, { waitUntil: 'load' });
  await page.waitForFunction(
    (runId) =>
      window.__DASHBOARD_DIAGNOSTICS__?.runId === runId &&
      window.__DASHBOARD_RENDER_EVENTS__?.length > 0,
    executiveFixtureSnapshot.run_id
  );

  const diagnostics = await page.evaluate(() => window.__DASHBOARD_DIAGNOSTICS__);
  assert.equal(diagnostics.kpis.cardCount, 7);
  assert.equal(diagnostics.charts.delta.pointCount, 4);
  assert.equal(diagnostics.charts.channelMix.datasetCount, 4);
  assert.equal(diagnostics.charts.dailyRevenue.totalRevenue, 4861.42);

  const roasInfo = page.locator('[data-kpi-key="roas"] [data-field="info-button"]');
  await roasInfo.hover();
  await page.waitForFunction(() => {
    const tooltip = document.querySelector('[data-kpi-key="roas"] [data-field="tooltip"]');
    return tooltip && window.getComputedStyle(tooltip).opacity === '1';
  });

  const roasTooltipText = await page
    .locator('[data-kpi-key="roas"] [data-field="tooltip"]')
    .innerText();
  assert.match(roasTooltipText, /Formula: ROAS = Revenue \/ Spend\./);
  assert.match(roasTooltipText, /Derived from KPI tiles: 4,861\.42 \/ 742\.18 = 6\.55x/);

  assert.equal(
    await page.locator('#exportPacketButton').getAttribute('data-export-ready'),
    'false'
  );
  assert.equal(
    await page.locator('#deltaChart').getAttribute('data-point-count'),
    '4'
  );
  assert.equal(
    await page.locator('#channelMixChart').getAttribute('data-dataset-count'),
    '4'
  );

  const dailySummaryText = await page.locator('#dailyRevenueSummary').innerText();
  assert.match(dailySummaryText, /\$4,861\.42/);

  const eventCount = await page.evaluate(() => window.__DASHBOARD_RENDER_EVENTS__.length);
  assert.equal(eventCount, 1);
});
