import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildAttributionDeltaModel,
  buildChannelMixChartModel,
  buildDailyRevenueChartModel,
  buildDeltaChartModel,
  buildExperimentGovernanceViewModel,
  buildFunnelSurvivalModel,
  buildKpiViewModel
} from '../dashboard_view_models.mjs';
import {
  renderAttributionDeltaTableSurface,
  renderChartSummarySurface,
  renderChartSurfaceMetadata,
  renderExperimentGovernanceSurface,
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

test('high-leverage report models produce deterministic summaries and table rows', () => {
  const funnelModel = buildFunnelSurvivalModel(
    executiveFixtureSnapshot.high_leverage_reports.funnel_survival
  );
  const attributionModel = buildAttributionDeltaModel(
    executiveFixtureSnapshot.high_leverage_reports.attribution_delta
  );
  const experimentGovernanceModel = buildExperimentGovernanceViewModel(
    executiveFixtureSnapshot.high_leverage_reports.experiment_governance
  );

  assert.equal(funnelModel.diagnostics.pointCount, 7);
  assert.equal(funnelModel.diagnostics.bottleneckStage, 'Add To Cart');
  assert.match(funnelModel.summaryText, /Bottleneck: Add To Cart/);

  assert.equal(attributionModel.diagnostics.rowCount, 3);
  assert.equal(attributionModel.diagnostics.dominantCampaign, 'Puppy Starter Bundle');
  assert.equal(attributionModel.rows[0].campaign, 'Puppy Starter Bundle');
  assert.equal(attributionModel.rows[0].lastTouchDisplay, '47.0%');
  assert.match(attributionModel.summaryText, /HHI: 0\.3640\./);

  assert.equal(experimentGovernanceModel.diagnostics.itemCount, 1);
  assert.deepEqual(experimentGovernanceModel.diagnostics.permissionLevels, ['directional_only']);
  assert.equal(experimentGovernanceModel.items[0].metricTiles[0].value, '90.00%');
  assert.equal(experimentGovernanceModel.items[0].metricTiles[5].value, 'assigned sessions only');
});

test('render helpers emit stable DOM contracts for KPIs and chart metadata', () => {
  const kpiGrid = createElementStub();
  const dailyRevenueSummary = createElementStub();
  const deltaCanvas = createElementStub();
  const attributionTableBody = createElementStub();
  const experimentGovernancePanel = createElementStub();
  const viewModel = buildKpiViewModel(executiveFixtureSnapshot.kpis, executiveFixtureSnapshot);
  const dailyRevenueModel = buildDailyRevenueChartModel(
    executiveFixtureSnapshot.daily_revenue_series
  );
  const deltaModel = buildDeltaChartModel(
    executiveFixtureSnapshot.historical_analysis.period_over_period_deltas
  );
  const attributionModel = buildAttributionDeltaModel(
    executiveFixtureSnapshot.high_leverage_reports.attribution_delta
  );
  const experimentGovernanceModel = buildExperimentGovernanceViewModel(
    executiveFixtureSnapshot.high_leverage_reports.experiment_governance
  );

  renderKpiSurface({ kpiGrid }, viewModel);
  renderChartSummarySurface(dailyRevenueSummary, dailyRevenueModel);
  renderChartSurfaceMetadata(deltaCanvas, deltaModel.diagnostics);
  renderAttributionDeltaTableSurface(attributionTableBody, attributionModel);
  renderExperimentGovernanceSurface(experimentGovernancePanel, experimentGovernanceModel);

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
  assert.equal(attributionTableBody.dataset.rowCount, '3');
  assert.match(attributionTableBody.innerHTML, /Puppy Starter Bundle/);
  assert.equal(experimentGovernancePanel.dataset.itemCount, '1');
  assert.equal(
    experimentGovernancePanel.dataset.coverageScope,
    'experiment_id_scoped_observed_sessions'
  );
  assert.match(experimentGovernancePanel.innerHTML, /directional only/);
  assert.match(experimentGovernancePanel.innerHTML, /assigned sessions only/);
});
