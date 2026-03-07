import { renderDashboardDecisionSurfaces } from './dashboard_app.mjs';
import {
  buildAttributionDeltaModel,
  buildChannelMixChartModel,
  buildDailyRevenueChartModel,
  buildDeltaChartModel,
  buildExperimentGovernanceViewModel,
  buildFunnelSurvivalModel,
  buildKpiViewModel
} from './dashboard_view_models.mjs';
import {
  renderAttributionDeltaTableSurface,
  renderChartSummarySurface,
  renderChartSurfaceMetadata,
  renderExperimentGovernanceSurface,
  renderKpiSurface
} from './dashboard_renderers.mjs';

function createElementStub() {
  return {
    innerHTML: '',
    textContent: '',
    className: '',
    dataset: {}
  };
}

function createElements() {
  return {
    revenueTruthPanel: createElementStub(),
    revenueTruthGuardChip: createElementStub(),
    revenueTruthRiskChip: createElementStub(),
    publishGatePanel: createElementStub(),
    decisionFeedList: createElementStub(),
    exportPacketButton: createElementStub(),
    kpiGrid: createElementStub(),
    deltaChart: createElementStub(),
    channelMixChart: createElementStub(),
    dailyRevenueChart: createElementStub(),
    dailyRevenueSummary: createElementStub(),
    funnelSurvivalChart: createElementStub(),
    funnelSurvivalSummary: createElementStub(),
    attributionDeltaChart: createElementStub(),
    attributionDeltaSummary: createElementStub(),
    attributionDeltaTableBody: createElementStub(),
    experimentGovernancePanel: createElementStub()
  };
}

function createWindowStub() {
  const payloadElement = createElementStub();
  const events = [];
  return {
    __events: events,
    document: {
      body: {
        dataset: {}
      },
      getElementById(id) {
        return id === 'dashboardDiagnosticsPayload' ? payloadElement : null;
      }
    },
    CustomEvent: class {
      constructor(type, init = {}) {
        this.type = type;
        this.detail = init.detail;
      }
    },
    dispatchEvent(event) {
      events.push(event);
      return true;
    }
  };
}

function cloneDataset(element) {
  return { ...(element?.dataset || {}) };
}

export function buildDashboardDiagnosticsArtifact(snapshot) {
  const elements = createElements();
  const targetWindow = createWindowStub();

  const kpiModel = buildKpiViewModel(snapshot?.kpis || [], snapshot);
  const deltaChartModel = buildDeltaChartModel(
    snapshot?.historical_analysis?.period_over_period_deltas || []
  );
  const channelMixChartModel = buildChannelMixChartModel(
    snapshot?.channel_mix_series || [],
    snapshot?.roas_target_band
  );
  const dailyRevenueChartModel = buildDailyRevenueChartModel(
    snapshot?.daily_revenue_series || []
  );
  const funnelSurvivalModel = buildFunnelSurvivalModel(
    snapshot?.high_leverage_reports?.funnel_survival || {}
  );
  const attributionDeltaModel = buildAttributionDeltaModel(
    snapshot?.high_leverage_reports?.attribution_delta || {}
  );
  const experimentGovernanceModel = buildExperimentGovernanceViewModel(
    snapshot?.high_leverage_reports?.experiment_governance || {}
  );

  renderKpiSurface(elements, kpiModel);
  renderChartSurfaceMetadata(elements.deltaChart, deltaChartModel.diagnostics);
  renderChartSurfaceMetadata(elements.channelMixChart, channelMixChartModel.diagnostics);
  renderChartSummarySurface(elements.dailyRevenueSummary, dailyRevenueChartModel);
  renderChartSurfaceMetadata(elements.dailyRevenueChart, dailyRevenueChartModel.diagnostics);
  renderChartSummarySurface(elements.funnelSurvivalSummary, funnelSurvivalModel);
  renderChartSurfaceMetadata(elements.funnelSurvivalChart, funnelSurvivalModel.diagnostics);
  renderChartSummarySurface(elements.attributionDeltaSummary, attributionDeltaModel);
  renderChartSurfaceMetadata(elements.attributionDeltaChart, attributionDeltaModel.diagnostics);
  renderAttributionDeltaTableSurface(elements.attributionDeltaTableBody, attributionDeltaModel);
  renderExperimentGovernanceSurface(elements.experimentGovernancePanel, experimentGovernanceModel);

  const diagnostics = renderDashboardDecisionSurfaces({
    elements,
    snapshot,
    targetWindow,
    extraDiagnostics: {
      kpis: kpiModel.diagnostics,
      charts: {
        delta: deltaChartModel.diagnostics,
        channelMix: channelMixChartModel.diagnostics,
        dailyRevenue: dailyRevenueChartModel.diagnostics
      },
      highLeverageReports: {
        funnelSurvival: funnelSurvivalModel.diagnostics,
        attributionDelta: attributionDeltaModel.diagnostics,
        experimentGovernance: experimentGovernanceModel.diagnostics
      }
    }
  });

  return {
    schemaVersion: 'dashboard_fixture_evidence.v1',
    runId: diagnostics.runId,
    profileId: diagnostics.profileId,
    diagnostics,
    diagnosticsJson: targetWindow.__DASHBOARD_DIAGNOSTICS_JSON__,
    payloadElement: {
      dataset: cloneDataset(targetWindow.document.getElementById('dashboardDiagnosticsPayload')),
      textContent: targetWindow.document.getElementById('dashboardDiagnosticsPayload').textContent
    },
    domContracts: {
      kpiGrid: {
        dataset: cloneDataset(elements.kpiGrid),
        containsTooltipField: elements.kpiGrid.innerHTML.includes('data-field="tooltip"')
      },
      revenueTruthPanel: cloneDataset(elements.revenueTruthPanel),
      publishGatePanel: cloneDataset(elements.publishGatePanel),
      decisionFeedList: cloneDataset(elements.decisionFeedList),
      exportPacketButton: cloneDataset(elements.exportPacketButton),
      deltaChart: cloneDataset(elements.deltaChart),
      channelMixChart: cloneDataset(elements.channelMixChart),
      dailyRevenueSummary: {
        dataset: cloneDataset(elements.dailyRevenueSummary),
        textContent: elements.dailyRevenueSummary.textContent
      },
      dailyRevenueChart: cloneDataset(elements.dailyRevenueChart),
      funnelSurvivalSummary: {
        dataset: cloneDataset(elements.funnelSurvivalSummary),
        textContent: elements.funnelSurvivalSummary.textContent
      },
      funnelSurvivalChart: cloneDataset(elements.funnelSurvivalChart),
      attributionDeltaSummary: {
        dataset: cloneDataset(elements.attributionDeltaSummary),
        textContent: elements.attributionDeltaSummary.textContent
      },
      attributionDeltaChart: cloneDataset(elements.attributionDeltaChart),
      attributionDeltaTableBody: cloneDataset(elements.attributionDeltaTableBody),
      experimentGovernancePanel: cloneDataset(elements.experimentGovernancePanel)
    }
  };
}
