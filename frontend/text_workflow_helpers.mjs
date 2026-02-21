const HARD_DAILY_CAP_USD = 10.0;

export const ROUTE_POLICY_PRESETS = Object.freeze({
  economy: Object.freeze({
    policy_id: 'economy',
    label: 'Economy',
    remaining_daily_budget_usd: HARD_DAILY_CAP_USD,
    max_cost_per_run_usd: 0.75,
    max_total_input_tokens: 12000,
    max_total_output_tokens: 3000,
    hard_daily_cap_usd: HARD_DAILY_CAP_USD
  }),
  balanced: Object.freeze({
    policy_id: 'balanced',
    label: 'Balanced',
    remaining_daily_budget_usd: HARD_DAILY_CAP_USD,
    max_cost_per_run_usd: 2.0,
    max_total_input_tokens: 24000,
    max_total_output_tokens: 8000,
    hard_daily_cap_usd: HARD_DAILY_CAP_USD
  }),
  quality: Object.freeze({
    policy_id: 'quality',
    label: 'Quality',
    remaining_daily_budget_usd: HARD_DAILY_CAP_USD,
    max_cost_per_run_usd: 4.0,
    max_total_input_tokens: 40000,
    max_total_output_tokens: 12000,
    hard_daily_cap_usd: HARD_DAILY_CAP_USD
  })
});

export function resolveRoutePolicyPreset(policyId) {
  const key = String(policyId || 'balanced').trim().toLowerCase();
  return ROUTE_POLICY_PRESETS[key] || ROUTE_POLICY_PRESETS.balanced;
}

export function buildTextWorkflowRequestFromInputs(inputs) {
  const routePolicy = resolveRoutePolicyPreset(inputs.routePolicyId);
  const audiences = normalizeAudienceSegments(inputs.audienceSegments);
  const includeEvidence = !!inputs.includeEvidence;
  const proofClaim = cleanText(inputs.proofClaim) || 'high digestibility blend';

  return {
    template_id: cleanText(inputs.templateId) || 'tpl.email_landing_sequence.v1',
    variant_count: normalizeVariantCount(inputs.variantCount),
    paid_calls_allowed: !!inputs.paidCallsAllowed,
    budget: {
      remaining_daily_budget_usd: routePolicy.remaining_daily_budget_usd,
      max_cost_per_run_usd: routePolicy.max_cost_per_run_usd,
      max_total_input_tokens: routePolicy.max_total_input_tokens,
      max_total_output_tokens: routePolicy.max_total_output_tokens,
      hard_daily_cap_usd: routePolicy.hard_daily_cap_usd
    },
    campaign_spine: {
      campaign_spine_id: cleanText(inputs.campaignSpineId) || 'spine.default.v1',
      product_name: cleanText(inputs.productName) || "Nature's Diet Raw Mix",
      offer_summary: cleanText(inputs.offerSummary) || 'Save 20% on first order',
      audience_segments: audiences.length ? audiences : ['new puppy owners', 'sensitive stomach'],
      positioning_statement:
        cleanText(inputs.positioningStatement) || 'Raw-first nutrition with practical prep',
      message_house: {
        big_idea: cleanText(inputs.bigIdea) || 'Fresh confidence in every bowl',
        pillars: [
          {
            pillar_id: 'p1',
            title: 'Digestive comfort',
            supporting_points: ['gentle proteins']
          }
        ],
        proof_points: [
          {
            claim_id: 'claim1',
            claim_text: proofClaim,
            evidence_ref_ids: includeEvidence ? ['ev1'] : []
          }
        ],
        do_not_say: ['cure claims'],
        tone_guide: ['clear', 'grounded']
      },
      evidence_refs: includeEvidence
        ? [
            {
              evidence_id: 'ev1',
              source_ref: 'internal.digestibility.v1',
              excerpt: 'digestibility improved 11% vs baseline'
            }
          ]
        : []
    }
  };
}

export function deriveTextWorkflowGateState(result) {
  if (!result || !result.artifact || !result.artifact.gate_decision) {
    return {
      blocked: false,
      gateStatus: 'ready',
      blockingReasons: [],
      warningReasons: [],
      canExport: false,
      exportBlockReason: 'No workflow result is available yet.'
    };
  }

  const gate = result.artifact.gate_decision;
  const blocked = gate.blocked === true;
  const blockingReasons = Array.isArray(gate.blocking_reasons) ? gate.blocking_reasons : [];
  const warningReasons = Array.isArray(gate.warning_reasons) ? gate.warning_reasons : [];
  const gateStatus = blocked ? 'blocked' : warningReasons.length ? 'review_required' : 'ready';

  return {
    blocked,
    gateStatus,
    blockingReasons,
    warningReasons,
    canExport: !blocked,
    exportBlockReason: blocked
      ? `Export is paused until review blockers are resolved: ${blockingReasons.join(' | ') || 'gate requires review'}`
      : ''
  };
}

export function buildTextWorkflowGateMarkup(result) {
  const gate = deriveTextWorkflowGateState(result);
  const run = result || {};
  return `
    <div class="gate-card">
      <h3>Run Summary</h3>
      <p>Template: <strong>${escapeHtml(run.template_id || 'n/a')}</strong></p>
      <p>Workflow: <strong>${escapeHtml(run.workflow_kind || 'n/a')}</strong></p>
      <p>Input tokens: <strong>${formatInt(run.total_estimated_input_tokens || 0)}</strong></p>
      <p>Output tokens: <strong>${formatInt(run.total_estimated_output_tokens || 0)}</strong></p>
      <p>Estimated cost: <strong>$${formatNum(run.total_estimated_cost_usd || 0, 4)}</strong></p>
    </div>
    <div class="gate-card">
      <h3>Gate Status</h3>
      <div class="gate-status ${escapeHtml(gate.gateStatus)}">${escapeHtml(gate.gateStatus.replace('_', ' '))}</div>
      <p>Requires review: <strong>${gate.blocked ? 'yes' : 'no'}</strong></p>
    </div>
    <div class="gate-card">
      <h3>Blocking Reasons</h3>
      <p>${gate.blockingReasons.length ? escapeHtml(gate.blockingReasons.join(' | ')) : 'None'}</p>
    </div>
    <div class="gate-card">
      <h3>Warnings</h3>
      <p>${gate.warningReasons.length ? escapeHtml(gate.warningReasons.join(' | ')) : 'None'}</p>
    </div>`;
}

export function buildTextWorkflowExportPacketMarkdown(result, nowIso = new Date().toISOString()) {
  const gate = deriveTextWorkflowGateState(result);
  if (!result) {
    throw new Error('Cannot export packet: workflow result is missing.');
  }
  if (!gate.canExport) {
    throw new Error(gate.exportBlockReason || 'Cannot export packet: export is paused by the review gate.');
  }

  const lines = [];
  lines.push('# Text Workflow Export Packet');
  lines.push('');
  lines.push(`- Generated At: ${nowIso}`);
  lines.push(`- Template: ${result.template_id || 'n/a'}`);
  lines.push(`- Workflow: ${result.workflow_kind || 'n/a'}`);
  lines.push(`- Campaign Spine: ${result.campaign_spine_id || 'n/a'}`);
  lines.push(`- Estimated Cost: $${formatNum(result.total_estimated_cost_usd || 0, 4)}`);
  lines.push(`- Estimated Input Tokens: ${formatInt(result.total_estimated_input_tokens || 0)}`);
  lines.push(`- Estimated Output Tokens: ${formatInt(result.total_estimated_output_tokens || 0)}`);
  lines.push('');
  lines.push('## Gate Decision');
  lines.push(`- Status: ${gate.gateStatus}`);
  lines.push(`- Blocking Reasons: ${gate.blockingReasons.length ? gate.blockingReasons.join(' | ') : 'None'}`);
  lines.push(`- Warning Reasons: ${gate.warningReasons.length ? gate.warningReasons.join(' | ') : 'None'}`);
  lines.push('');
  lines.push('## Node Trace');

  const traces = Array.isArray(result.traces) ? result.traces : [];
  if (!traces.length) {
    lines.push('- No trace rows.');
  } else {
    for (const trace of traces) {
      lines.push(
        `- ${trace.node_id || 'n/a'} (${trace.node_kind || 'n/a'}) :: ${trace.route?.provider || 'n/a'} / ${trace.route?.model || 'n/a'} :: in=${formatInt(trace.estimated_input_tokens || 0)} out=${formatInt(trace.estimated_output_tokens || 0)} cost=$${formatNum(trace.estimated_cost_usd || 0, 4)}`
      );
    }
  }

  lines.push('');
  lines.push('## Sections');
  const sections = Array.isArray(result.artifact?.sections) ? result.artifact.sections : [];
  if (!sections.length) {
    lines.push('- No sections generated.');
  } else {
    for (const section of sections) {
      lines.push(`### ${section.section_title || section.section_id || 'Section'}`);
      lines.push(section.content || '');
      lines.push('');
    }
  }

  lines.push('## Critique Findings');
  const findings = Array.isArray(result.artifact?.critique_findings)
    ? result.artifact.critique_findings
    : [];
  if (!findings.length) {
    lines.push('- No critique findings.');
  } else {
    for (const finding of findings) {
      lines.push(`- [${String(finding.severity || 'unknown').toUpperCase()}] ${finding.code || 'finding'}: ${finding.message || ''}`);
    }
  }

  return lines.join('\n');
}

export function buildTextWorkflowExportFilename(result, now = new Date()) {
  const stamp = now.toISOString().replace(/[:.]/g, '-');
  const template = sanitizeFilePart(result?.template_id || 'text_workflow');
  return `${template}_${stamp}.md`;
}

export async function runTextWorkflowJobLifecycle({
  invoke,
  request,
  onSnapshot = null,
  pollIntervalMs = 250,
  maxPolls = 240,
  sleepFn = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
}) {
  if (typeof invoke !== 'function') {
    throw new Error('invoke function is required');
  }

  const handle = await invoke('start_mock_text_workflow_job', { request });
  if (!handle || !handle.job_id) {
    throw new Error('start_mock_text_workflow_job did not return a valid job handle');
  }

  const snapshots = [];
  for (let attempt = 0; attempt < maxPolls; attempt += 1) {
    const snapshot = await invoke('get_tool_job', { jobId: handle.job_id });
    snapshots.push(snapshot);
    if (typeof onSnapshot === 'function') {
      onSnapshot(snapshot, attempt);
    }

    if (snapshot?.status === 'succeeded') {
      return {
        jobId: handle.job_id,
        status: 'succeeded',
        result: snapshot.output || null,
        terminalSnapshot: snapshot,
        snapshots
      };
    }

    if (snapshot?.status === 'failed' || snapshot?.status === 'canceled') {
      return {
        jobId: handle.job_id,
        status: snapshot.status,
        result: null,
        errorMessage: snapshot?.error?.message || snapshot?.message || 'execution failed',
        terminalSnapshot: snapshot,
        snapshots
      };
    }

    if (pollIntervalMs > 0) {
      await sleepFn(pollIntervalMs);
    }
  }

  throw new Error(`Text workflow job polling exceeded max attempts (${maxPolls})`);
}

function normalizeVariantCount(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return 12;
  }
  const bounded = Math.max(1, Math.min(30, Math.trunc(numeric)));
  return bounded;
}

function normalizeAudienceSegments(value) {
  if (Array.isArray(value)) {
    return value.map((item) => String(item).trim()).filter(Boolean);
  }
  return String(value || '')
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);
}

function cleanText(value) {
  const normalized = String(value || '').trim();
  return normalized || null;
}

function sanitizeFilePart(value) {
  return String(value || '')
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, '_')
    .replace(/^_+|_+$/g, '') || 'text_workflow';
}

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function formatNum(v, decimals = 2) {
  const n = Number(v);
  if (!Number.isFinite(n)) return '0.00';
  return n.toFixed(decimals);
}

function formatInt(v) {
  const n = Number(v);
  if (!Number.isFinite(n)) return '0';
  return Math.round(n).toLocaleString();
}
