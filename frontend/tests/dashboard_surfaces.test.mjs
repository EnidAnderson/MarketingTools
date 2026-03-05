import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildDecisionFeedViewModel,
  buildPublishGateViewModel,
  buildRevenueTruthViewModel
} from '../dashboard_view_models.mjs';
import { renderDashboardDecisionSurfaces } from '../dashboard_app.mjs';

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
    exportPacketButton: createElementStub()
  };
}

function createWindowStub() {
  const events = [];
  const payloadElement = { textContent: '', dataset: {} };
  return {
    __events: events,
    __payloadElement: payloadElement,
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

test('revenue truth view model normalizes guard and metrics', () => {
  const model = buildRevenueTruthViewModel({
    canonical_revenue: 4861.42,
    canonical_conversions: 71,
    strict_duplicate_ratio: 0.0079,
    near_duplicate_ratio: 0.0212,
    custom_purchase_rows: 6,
    custom_purchase_overlap_rows: 5,
    custom_purchase_orphan_rows: 1,
    custom_purchase_overlap_ratio: 0.8333,
    custom_purchase_orphan_ratio: 0.1667,
    truth_guard_status: 'guarded_review_required',
    inflation_risk: 'low',
    estimated_revenue_at_risk: 103.06,
    summary: 'Canonical purchase metrics enforced.'
  });

  assert.equal(model.guardStatus, 'guarded_review_required');
  assert.equal(model.guardTone, 'warn');
  assert.equal(model.riskTone, 'good');
  assert.equal(model.metrics.length, 10);
  assert.equal(model.metrics[0].key, 'canonical_revenue');
  assert.equal(model.metrics[5].key, 'custom_purchase_rows');
});

test('publish gate and decision feed view models normalize current status', () => {
  const gate = buildPublishGateViewModel({
    gate_status: 'review_required',
    publish_ready: true,
    export_ready: true,
    blocking_reasons: [],
    warning_reasons: ['custom purchase orphan rows detected']
  });
  const feed = buildDecisionFeedViewModel([
    {
      card_id: 'custom-purchase-orphans',
      priority: 'high',
      status: 'investigate',
      title: 'Custom purchase orphan rows detected',
      summary: 'Revenue completeness may be understated.',
      recommended_action: 'Audit checkout tagging.'
    }
  ]);

  assert.equal(gate.diagnostics.gateStatus, 'review_required');
  assert.equal(gate.diagnostics.warningCount, 1);
  assert.equal(feed.diagnostics.cardCount, 1);
  assert.deepEqual(feed.diagnostics.cardIds, ['custom-purchase-orphans']);
});

test('dashboard decision surfaces render stable DOM contracts and diagnostics', () => {
  const elements = createElements();
  const windowStub = createWindowStub();
  const diagnostics = renderDashboardDecisionSurfaces({
    elements,
    targetWindow: windowStub,
    extraDiagnostics: {
      kpis: { cardCount: 7 },
      charts: { delta: { pointCount: 4 } }
    },
    snapshot: {
      profile_id: 'marketing_default',
      run_id: 'run-2026-03-05-001',
      compare_window_runs: 2,
      trust_status: 'degraded',
      high_leverage_reports: {
        revenue_truth: {
          canonical_revenue: 4861.42,
          canonical_conversions: 71,
          strict_duplicate_ratio: 0.0079,
          near_duplicate_ratio: 0.0212,
          custom_purchase_rows: 6,
          custom_purchase_overlap_rows: 5,
          custom_purchase_orphan_rows: 1,
          custom_purchase_overlap_ratio: 0.8333,
          custom_purchase_orphan_ratio: 0.1667,
          truth_guard_status: 'guarded_review_required',
          inflation_risk: 'low',
          estimated_revenue_at_risk: 103.06,
          summary:
            'Canonical purchase metrics enforced. Duplicate custom purchase rows remain excluded.'
        }
      },
      publish_export_gate: {
        gate_status: 'review_required',
        publish_ready: true,
        export_ready: false,
        blocking_reasons: ['Manual export hold'],
        warning_reasons: ['Custom purchase orphan rows detected']
      },
      decision_feed: [
        {
          card_id: 'custom-purchase-overlap',
          priority: 'medium',
          status: 'review_required',
          title: 'Duplicate custom purchase stream still active',
          summary: 'purchase_ndp overlaps canonical purchase.',
          recommended_action: 'Disable redundant event.'
        },
        {
          card_id: 'custom-purchase-orphans',
          priority: 'high',
          status: 'investigate',
          title: 'Custom purchase orphan rows detected',
          summary: 'Revenue completeness may be understated.',
          recommended_action: 'Audit checkout tagging.'
        }
      ]
    }
  });

  assert.equal(diagnostics.runId, 'run-2026-03-05-001');
  assert.equal(diagnostics.publishGate.gateStatus, 'review_required');
  assert.equal(diagnostics.kpis.cardCount, 7);
  assert.equal(diagnostics.charts.delta.pointCount, 4);
  assert.equal(windowStub.__DASHBOARD_DIAGNOSTICS__.decisionFeed.cardCount, 2);
  assert.equal(windowStub.__DASHBOARD_DIAGNOSTICS_JSON__.includes('run-2026-03-05-001'), true);
  assert.match(windowStub.__payloadElement.textContent, /"runId": "run-2026-03-05-001"/);
  assert.equal(windowStub.document.body.dataset.dashboardRunId, 'run-2026-03-05-001');
  assert.equal(windowStub.document.body.dataset.dashboardGateStatus, 'review_required');
  assert.match(elements.revenueTruthPanel.innerHTML, /data-metric-key="custom_purchase_rows"/);
  assert.match(elements.publishGatePanel.innerHTML, /data-gate-status="review_required"/);
  assert.match(elements.decisionFeedList.innerHTML, /data-card-id="custom-purchase-orphans"/);
  assert.equal(elements.revenueTruthGuardChip.dataset.guardStatus, 'guarded_review_required');
  assert.equal(elements.revenueTruthRiskChip.dataset.riskLevel, 'low');
  assert.equal(elements.exportPacketButton.disabled, true);
  assert.equal(elements.exportPacketButton.dataset.exportReady, 'false');
  assert.match(elements.exportPacketButton.title, /Manual export hold/);
  assert.equal(windowStub.__events.length, 1);
  assert.equal(windowStub.__events[0].type, 'dashboard:rendered');
});
