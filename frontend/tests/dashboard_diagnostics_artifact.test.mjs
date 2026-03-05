import test from 'node:test';
import assert from 'node:assert/strict';

import { buildDashboardDiagnosticsArtifact } from '../dashboard_diagnostics_artifact.mjs';
import { executiveFixtureSnapshot } from './dashboard_fixture_data.mjs';

test('dashboard diagnostics artifact is deterministic and captures DOM contracts', () => {
  const artifact = buildDashboardDiagnosticsArtifact(executiveFixtureSnapshot);
  const parsedPayload = JSON.parse(artifact.diagnosticsJson);

  assert.equal(artifact.schemaVersion, 'dashboard_fixture_evidence.v1');
  assert.equal(artifact.runId, executiveFixtureSnapshot.run_id);
  assert.equal(parsedPayload.runId, executiveFixtureSnapshot.run_id);
  assert.equal(
    artifact.payloadElement.dataset.schemaVersion,
    'dashboard_render_diagnostics.v1'
  );
  assert.equal(artifact.domContracts.kpiGrid.dataset.kpiCount, '7');
  assert.equal(artifact.domContracts.kpiGrid.containsTooltipField, true);
  assert.equal(artifact.domContracts.deltaChart.pointCount, '4');
  assert.equal(artifact.domContracts.channelMixChart.datasetCount, '4');
  assert.match(artifact.domContracts.dailyRevenueSummary.textContent, /\$4,861\.42/);
  assert.match(artifact.domContracts.funnelSurvivalSummary.textContent, /Bottleneck: Add To Cart/);
  assert.equal(artifact.domContracts.attributionDeltaTableBody.rowCount, '3');
  assert.equal(
    artifact.diagnostics.highLeverageReports.attributionDelta.dominantCampaign,
    'Puppy Starter Bundle'
  );
});
