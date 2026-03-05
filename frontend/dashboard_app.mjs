import {
  buildDecisionFeedViewModel,
  buildPublishGateViewModel,
  buildRevenueTruthViewModel
} from './dashboard_view_models.mjs';
import {
  renderDecisionFeedSurface,
  renderPublishGateSurface,
  renderRevenueTruthSurface
} from './dashboard_renderers.mjs';

export function renderDashboardDecisionSurfaces({
  elements,
  snapshot,
  targetWindow = globalThis.window,
  extraDiagnostics = {}
}) {
  const revenueTruth = buildRevenueTruthViewModel(
    snapshot?.high_leverage_reports?.revenue_truth || {}
  );
  const publishGate = buildPublishGateViewModel(snapshot?.publish_export_gate || {});
  const decisionFeed = buildDecisionFeedViewModel(snapshot?.decision_feed || []);

  renderRevenueTruthSurface(elements, revenueTruth);
  renderPublishGateSurface(elements, publishGate);
  renderDecisionFeedSurface(elements, decisionFeed);

  const diagnostics = {
    schemaVersion: 'dashboard_render_diagnostics.v1',
    profileId: snapshot?.profile_id || 'n/a',
    runId: snapshot?.run_id || 'n/a',
    compareWindowRuns: Number(snapshot?.compare_window_runs || 0),
    trustStatus: snapshot?.trust_status || 'unknown',
    revenueTruth: revenueTruth.diagnostics,
    publishGate: publishGate.diagnostics,
    decisionFeed: decisionFeed.diagnostics,
    ...extraDiagnostics
  };
  publishDashboardDiagnostics(targetWindow, diagnostics);
  return diagnostics;
}

export function publishDashboardDiagnostics(targetWindow, diagnostics) {
  if (!targetWindow || !diagnostics) return diagnostics;
  targetWindow.__DASHBOARD_DIAGNOSTICS__ = diagnostics;
  targetWindow.__DASHBOARD_DIAGNOSTICS_JSON__ = JSON.stringify(diagnostics, null, 2);

  const body = targetWindow.document?.body;
  if (body?.dataset) {
    body.dataset.dashboardRunId = diagnostics.runId;
    body.dataset.dashboardGateStatus = diagnostics.publishGate.gateStatus;
    body.dataset.dashboardTrustStatus = diagnostics.trustStatus;
  }

  const payloadElement = targetWindow.document?.getElementById?.('dashboardDiagnosticsPayload');
  if (payloadElement) {
    payloadElement.textContent = targetWindow.__DASHBOARD_DIAGNOSTICS_JSON__;
    if (payloadElement.dataset) {
      payloadElement.dataset.schemaVersion = diagnostics.schemaVersion || 'unknown';
      payloadElement.dataset.runId = diagnostics.runId || 'n/a';
    }
  }

  if (typeof targetWindow.dispatchEvent === 'function') {
    let event = null;
    if (typeof targetWindow.CustomEvent === 'function') {
      event = new targetWindow.CustomEvent('dashboard:rendered', {
        detail: diagnostics
      });
    } else if (typeof CustomEvent === 'function') {
      event = new CustomEvent('dashboard:rendered', {
        detail: diagnostics
      });
    } else {
      event = { type: 'dashboard:rendered', detail: diagnostics };
    }
    targetWindow.dispatchEvent(event);
  }

  return diagnostics;
}
