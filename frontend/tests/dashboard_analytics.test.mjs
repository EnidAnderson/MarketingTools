import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildChannelMixChartModel,
  buildDailyRevenueChartModel,
  buildDeltaChartModel,
  buildKpiViewModel
} from '../dashboard_view_models.mjs';
import {
  renderChartSummarySurface,
  renderChartSurfaceMetadata,
  renderKpiSurface
} from '../dashboard_renderers.mjs';
import { executiveFixtureSnapshot } from './dashboard_fixture_data.mjs';

function createElementStub() {
  return {
    innerHTML: '',
    textContent: '',
    className: '',
    dataset: {}
  };
}

test('buildKpiViewModel derives stable keys and formula tooltips', () => {
  const model = buildKpiViewModel(executiveFixtureSnapshot.kpis, executiveFixtureSnapshot);
  const roasCard = model.cards.find((card) => card.key === 'roas');
  const ctrCard = model.cards.find((card) => card.key === 'ctr');

  assert.equal(model.diagnostics.cardCount, 7);
  assert.equal(roasCard.displayValue, '6.55x');
  assert.match(roasCard.tooltipText, /Formula: ROAS = Revenue \/ Spend\./);
  assert.match(roasCard.tooltipText, /Derived from KPI tiles: 4,861\.42 \/ 742\.18 = 6\.55x/);
  assert.match(ctrCard.tooltipText, /Formula: CTR = Clicks \/ Impressions × 100\./);
  assert.match(ctrCard.tooltipText, /From funnel stages: 3,796 \/ 45,200 = 8\.40%/);
});

test('chart models produce deterministic configs and diagnostics', () => {
  const deltaModel = buildDeltaChartModel(
    executiveFixtureSnapshot.historical_analysis.period_over_period_deltas
  );
  const channelMixModel = buildChannelMixChartModel(
    executiveFixtureSnapshot.channel_mix_series,
    executiveFixtureSnapshot.roas_target_band
  );
  const dailyRevenueModel = buildDailyRevenueChartModel(
    executiveFixtureSnapshot.daily_revenue_series
  );

  assert.equal(deltaModel.diagnostics.pointCount, 4);
  assert.equal(deltaModel.config.data.datasets[0].data[0], 15.3);
  assert.deepEqual(deltaModel.diagnostics.labels, ['revenue', 'roas', 'conversions', 'cost']);

  assert.equal(channelMixModel.diagnostics.datasetCount, 4);
  assert.equal(channelMixModel.diagnostics.hasRoasTarget, true);
  assert.equal(channelMixModel.config.data.datasets[3].label, 'ROAS Target');

  assert.equal(dailyRevenueModel.diagnostics.pointCount, 7);
  assert.equal(dailyRevenueModel.diagnostics.totalRevenue, 4861.42);
  assert.match(
    dailyRevenueModel.summaryText,
    /Total in selected window: \$4,861\.42 across 7 day\(s\); 7 day\(s\) recorded non-zero revenue\./
  );
});

test('render helpers emit stable DOM contracts for KPIs and chart metadata', () => {
  const kpiGrid = createElementStub();
  const dailyRevenueSummary = createElementStub();
  const deltaCanvas = createElementStub();
  const viewModel = buildKpiViewModel(executiveFixtureSnapshot.kpis, executiveFixtureSnapshot);
  const dailyRevenueModel = buildDailyRevenueChartModel(
    executiveFixtureSnapshot.daily_revenue_series
  );
  const deltaModel = buildDeltaChartModel(
    executiveFixtureSnapshot.historical_analysis.period_over_period_deltas
  );

  renderKpiSurface({ kpiGrid }, viewModel);
  renderChartSummarySurface(dailyRevenueSummary, dailyRevenueModel);
  renderChartSurfaceMetadata(deltaCanvas, deltaModel.diagnostics);

  assert.equal(kpiGrid.dataset.kpiCount, '7');
  assert.match(kpiGrid.innerHTML, /data-kpi-key="roas"/);
  assert.match(kpiGrid.innerHTML, /data-field="tooltip"/);
  assert.match(kpiGrid.innerHTML, /data-confidence-label="high"/);
  assert.equal(dailyRevenueSummary.dataset.chartKey, 'daily-revenue');
  assert.equal(dailyRevenueSummary.dataset.pointCount, '7');
  assert.match(dailyRevenueSummary.textContent, /\$4,861\.42/);
  assert.equal(deltaCanvas.dataset.chartKey, 'delta');
  assert.equal(deltaCanvas.dataset.pointCount, '4');
  assert.equal(deltaCanvas.dataset.datasetCount, '1');
});
