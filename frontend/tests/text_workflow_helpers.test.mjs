import test from 'node:test';
import assert from 'node:assert/strict';

import {
  ROUTE_POLICY_PRESETS,
  buildTextWorkflowExportPacketMarkdown,
  buildTextWorkflowGateMarkup,
  buildTextWorkflowRequestFromInputs,
  deriveTextWorkflowGateState,
  runTextWorkflowJobLifecycle
} from '../text_workflow_helpers.mjs';

function sampleResult(overrides = {}) {
  return {
    template_id: 'tpl.email_landing_sequence.v1',
    workflow_kind: 'email_landing_sequence',
    campaign_spine_id: 'spine.default.v1',
    total_estimated_input_tokens: 4200,
    total_estimated_output_tokens: 1700,
    total_estimated_cost_usd: 0.318,
    traces: [
      {
        node_id: 'planner',
        node_kind: 'planner',
        route: { provider: 'local_mock', model: 'local.mock.det.v1' },
        estimated_input_tokens: 900,
        estimated_output_tokens: 260,
        estimated_cost_usd: 0
      }
    ],
    artifact: {
      gate_decision: {
        blocked: false,
        blocking_reasons: [],
        warning_reasons: []
      },
      sections: [
        {
          section_id: 'email_1',
          section_title: 'Email 1',
          content: 'Welcome to Nature\'s Diet.'
        }
      ],
      critique_findings: []
    },
    ...overrides
  };
}

test('buildTextWorkflowRequestFromInputs maps economy policy and strips evidence when disabled', () => {
  const req = buildTextWorkflowRequestFromInputs({
    routePolicyId: 'economy',
    templateId: 'tpl.ad_variant_pack.v1',
    variantCount: 15,
    paidCallsAllowed: false,
    campaignSpineId: 'spine.alpha.v1',
    productName: 'ND Raw',
    offerSummary: 'Save 20%',
    audienceSegments: 'new puppy owners, sensitive stomach',
    positioningStatement: 'Practical raw nutrition',
    bigIdea: 'Fresh confidence in every bowl',
    proofClaim: 'digestibility blend',
    includeEvidence: false
  });

  assert.equal(req.budget.max_cost_per_run_usd, ROUTE_POLICY_PRESETS.economy.max_cost_per_run_usd);
  assert.equal(req.budget.max_total_input_tokens, ROUTE_POLICY_PRESETS.economy.max_total_input_tokens);
  assert.equal(req.budget.hard_daily_cap_usd, 10);
  assert.deepEqual(req.campaign_spine.evidence_refs, []);
  assert.deepEqual(req.campaign_spine.message_house.proof_points[0].evidence_ref_ids, []);
});

test('deriveTextWorkflowGateState blocks export when critical blockers exist', () => {
  const result = sampleResult({
    artifact: {
      gate_decision: {
        blocked: true,
        blocking_reasons: ['unsupported_high_risk_claim: no evidence'],
        warning_reasons: []
      },
      sections: [],
      critique_findings: []
    }
  });
  const gate = deriveTextWorkflowGateState(result);
  assert.equal(gate.blocked, true);
  assert.equal(gate.canExport, false);
  assert.equal(gate.gateStatus, 'blocked');
  assert.match(gate.exportBlockReason, /Export blocked/);
});

test('buildTextWorkflowGateMarkup includes gate status class and summary content', () => {
  const markup = buildTextWorkflowGateMarkup(sampleResult());
  assert.match(markup, /gate-status ready/);
  assert.match(markup, /Template:/);
  assert.match(markup, /Estimated cost/);
});

test('buildTextWorkflowExportPacketMarkdown emits packet for exportable result', () => {
  const markdown = buildTextWorkflowExportPacketMarkdown(sampleResult(), '2026-02-20T12:00:00.000Z');
  assert.match(markdown, /# Text Workflow Export Packet/);
  assert.match(markdown, /Generated At: 2026-02-20T12:00:00.000Z/);
  assert.match(markdown, /## Node Trace/);
  assert.match(markdown, /## Sections/);
  assert.match(markdown, /## Critique Findings/);
});

test('buildTextWorkflowExportPacketMarkdown throws when gate is blocked', () => {
  const blockedResult = sampleResult({
    artifact: {
      gate_decision: {
        blocked: true,
        blocking_reasons: ['policy_violation: unsupported claim'],
        warning_reasons: []
      },
      sections: [],
      critique_findings: []
    }
  });

  assert.throws(
    () => buildTextWorkflowExportPacketMarkdown(blockedResult),
    /Export blocked/
  );
});

test('runTextWorkflowJobLifecycle handles staged progress to success', async () => {
  const snapshots = [
    {
      status: 'running',
      progress_pct: 20,
      stage: 'validating_graph',
      message: 'Validating text workflow graph template'
    },
    {
      status: 'running',
      progress_pct: 78,
      stage: 'generating_artifact',
      message: 'Generating deterministic workflow artifact'
    },
    {
      status: 'succeeded',
      progress_pct: 100,
      stage: 'completed',
      output: sampleResult()
    }
  ];

  let pollIndex = 0;
  const seenStages = [];
  const invoke = async (command, payload) => {
    if (command === 'start_mock_text_workflow_job') {
      assert.ok(payload.request);
      return { job_id: 'job-text-1' };
    }
    if (command === 'get_tool_job') {
      assert.equal(payload.jobId, 'job-text-1');
      const snapshot = snapshots[Math.min(pollIndex, snapshots.length - 1)];
      pollIndex += 1;
      return snapshot;
    }
    throw new Error(`unexpected command ${command}`);
  };

  const lifecycle = await runTextWorkflowJobLifecycle({
    invoke,
    request: buildTextWorkflowRequestFromInputs({ routePolicyId: 'balanced' }),
    pollIntervalMs: 0,
    onSnapshot: (snapshot) => {
      seenStages.push(snapshot.stage);
    }
  });

  assert.equal(lifecycle.status, 'succeeded');
  assert.equal(lifecycle.jobId, 'job-text-1');
  assert.equal(lifecycle.result.template_id, 'tpl.email_landing_sequence.v1');
  assert.deepEqual(seenStages, ['validating_graph', 'generating_artifact', 'completed']);
});
