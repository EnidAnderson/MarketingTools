import http from 'node:http';
import path from 'node:path';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createReadStream, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { chromium } from 'playwright';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const frontendRoot = path.resolve(__dirname, '..');

const manifestPathArg = process.argv[2];
if (!manifestPathArg) {
  throw new Error('usage: node frontend/scripts/capture_governed_dashboard_export.mjs <manifest.json>');
}

const manifestPath = path.resolve(process.cwd(), manifestPathArg);
const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
const snapshot = JSON.parse(await readFile(manifest.export_payload_ref, 'utf8'));
const outputDir = manifest.ui_capture_output_dir
  ? path.resolve(manifest.ui_capture_output_dir)
  : path.join(path.dirname(manifestPath), 'images');

await mkdir(outputDir, { recursive: true });

const server = await startStaticServer(frontendRoot);
let browser;

try {
  browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1680, height: 2200 } });
  const page = await context.newPage();

  await page.addInitScript(({ snapshotValue }) => {
    window.__TAURI__ = {
      core: {
        invoke: async (command) => {
          if (command === 'get_executive_dashboard_snapshot') return snapshotValue;
          if (command === 'get_mock_analytics_run_history') return [];
          if (command === 'get_text_workflow_templates') return [];
          if (command === 'validate_analytics_connectors_preflight') {
            return { ok: true, blocking_reasons: [], warning_reasons: [] };
          }
          if (command === 'start_mock_analytics_job') return { job_id: 'not-used' };
          if (command === 'get_tool_job') {
            return { status: 'succeeded', progress_pct: 100, stage: 'completed', message: 'done' };
          }
          return {};
        }
      }
    };
  }, { snapshotValue: snapshot });

  await page.goto(`${server.origin}/index.html`, { waitUntil: 'networkidle' });
  await page.waitForSelector('#revenueTruthPanel .report-metric', { timeout: 20000 });
  await page.waitForSelector('#dateRangeBadge', { timeout: 20000 });
  await page.waitForTimeout(1200);

  const titles = await Promise.all([
    textContent(page, 'article:has-text("Revenue Truth") h2'),
    textContent(page, 'article:has-text("Funnel Survival & Hazard") h2'),
    textContent(page, 'article:has-text("Attribution Delta (First vs Last Touch)") h2'),
    textContent(page, 'article:has-text("Governance Quality Scorecard") h2')
  ]);
  const runIdBadge = await textContent(page, '#runIdBadge');
  const dateRangeBadge = await textContent(page, '#dateRangeBadge');
  const revenueMetricCount = await page.locator('#revenueTruthPanel .report-metric').count();
  const attributionRowCount = await page.locator('#attributionDeltaTableBody tr').count();
  const kpiCount = await page.locator('#kpiGrid .kpi').count();
  const diagnostics = await page.evaluate(() => window.__DASHBOARD_DIAGNOSTICS__ || null);

  const surfaces = [
    {
      key: 'revenue_truth',
      selector: 'article:has-text("Revenue Truth")',
      file: 'report_revenue_truth.png'
    },
    {
      key: 'funnel_survival',
      selector: 'article:has-text("Funnel Survival & Hazard")',
      file: 'report_funnel_survival.png'
    },
    {
      key: 'attribution_delta',
      selector: 'article:has-text("Attribution Delta (First vs Last Touch)")',
      file: 'report_attribution_delta.png'
    },
    {
      key: 'quality_scorecard',
      selector: 'article:has-text("Governance Quality Scorecard")',
      file: 'report_quality_scorecard.png'
    }
  ];

  const layout = {};
  for (const surface of surfaces) {
    const locator = page.locator(surface.selector).first();
    await locator.waitFor({ state: 'visible', timeout: 15000 });
    const box = await locator.boundingBox();
    if (!box || box.width < 240 || box.height < 120 || box.height > 1400) {
      throw new Error(`surface ${surface.key} failed layout stability check`);
    }
    layout[surface.key] = {
      width: Math.round(box.width),
      height: Math.round(box.height)
    };
    await locator.screenshot({ path: path.join(outputDir, surface.file) });
  }

  const fullDashboardPath = path.join(outputDir, 'dashboard_full.png');
  await page.screenshot({ path: fullDashboardPath, fullPage: true });

  const validation = {
    schema_version: 'governed_dashboard_capture_validation.v1',
    source: 'live_stored_analytics_history',
    profile_id: manifest.profile_id,
    run_id: manifest.run_id,
    export_id: manifest.export_id,
    date_range: manifest.date_range,
    titles,
    run_id_badge: runIdBadge,
    date_range_badge: dateRangeBadge,
    revenue_metric_count: revenueMetricCount,
    attribution_row_count: attributionRowCount,
    kpi_count: kpiCount,
    layout,
    diagnostics
  };

  if (dateRangeBadge !== `Range: ${manifest.date_range}`) {
    throw new Error(`date range badge mismatch: expected Range: ${manifest.date_range}, got ${dateRangeBadge}`);
  }
  if (!runIdBadge.includes(manifest.run_id)) {
    throw new Error(`run badge does not include persisted run id ${manifest.run_id}`);
  }
  if (revenueMetricCount < 5) {
    throw new Error('revenue truth surface rendered too few metrics');
  }
  if (attributionRowCount < 1) {
    throw new Error('attribution delta surface rendered no table rows');
  }
  if (kpiCount < 5) {
    throw new Error('kpi strip rendered too few KPI cards');
  }

  await writeFile(
    path.join(path.dirname(manifestPath), 'ui_validation.json'),
    `${JSON.stringify(validation, null, 2)}\n`,
    'utf8'
  );
  await writeFile(
    path.join(path.dirname(manifestPath), 'validation_note.md'),
    [
      '# Live Dashboard Validation',
      '',
      `- Persisted run: \`${manifest.run_id}\``,
      `- Stored history source: \`${manifest.run_store_path}\``,
      `- Governed export payload: \`${manifest.export_payload_ref}\``,
      `- Capture source: live stored analytics history`,
      `- Date range: \`${manifest.date_range}\``,
      `- Exported image directory: \`${outputDir}\``,
      '',
      'These report images were rendered from the governed export payload produced from persisted GA4-backed analytics history, not from fixture-only frontend data.'
    ].join('\n'),
    'utf8'
  );

  console.log(JSON.stringify({
    manifest_path: manifestPath,
    output_dir: outputDir,
    dashboard_full: fullDashboardPath,
    validation_path: path.join(path.dirname(manifestPath), 'ui_validation.json')
  }, null, 2));

  await context.close();
} finally {
  if (browser) {
    await browser.close().catch(() => {});
  }
  await stopStaticServer(server);
}

async function textContent(page, selector) {
  const value = await page.locator(selector).first().textContent();
  return String(value || '').trim();
}

async function startStaticServer(rootDir) {
  const server = http.createServer(async (req, res) => {
    try {
      const urlPath = req.url === '/' ? '/index.html' : req.url;
      const safePath = path.normalize(decodeURIComponent(urlPath)).replace(/^(\.\.[/\\])+/, '');
      const filePath = path.join(rootDir, safePath);
      if (!filePath.startsWith(rootDir) || !existsSync(filePath)) {
        res.writeHead(404);
        res.end('not found');
        return;
      }
      const contentType = mimeTypeFor(filePath);
      res.writeHead(200, { 'Content-Type': contentType });
      createReadStream(filePath).pipe(res);
    } catch (error) {
      res.writeHead(500);
      res.end(String(error));
    }
  });

  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const address = server.address();
  if (!address || typeof address === 'string') {
    throw new Error('failed to bind static server');
  }
  return {
    server,
    origin: `http://127.0.0.1:${address.port}`
  };
}

async function stopStaticServer(handle) {
  await new Promise((resolve) => handle.server.close(resolve));
}

function mimeTypeFor(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  switch (ext) {
    case '.html':
      return 'text/html; charset=utf-8';
    case '.js':
    case '.mjs':
      return 'application/javascript; charset=utf-8';
    case '.css':
      return 'text/css; charset=utf-8';
    case '.json':
      return 'application/json; charset=utf-8';
    default:
      return 'application/octet-stream';
  }
}
