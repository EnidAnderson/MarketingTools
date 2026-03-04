import {
  buildTextWorkflowExportFilename,
  buildTextWorkflowExportPacketMarkdown,
  buildTextWorkflowGateMarkup,
  buildTextWorkflowRequestFromInputs,
  deriveTextWorkflowGateState,
  resolveRoutePolicyPreset,
  runTextWorkflowJobLifecycle
} from './text_workflow_helpers.mjs';

const state = {
  currentSnapshot: null,
  deltaChart: null,
  channelMixChart: null,
  funnelSurvivalChart: null,
  attributionDeltaChart: null,
  historyRuns: [],
  textWorkflowTemplates: [],
  textWorkflowResult: null
};

const MAX_ANALYTICS_DATE_SPAN_DAYS = 93;
const MAX_PROFILE_ID_LENGTH = 128;
const PROFILE_ID_PATTERN = /^[a-z0-9][a-z0-9_-]{0,127}$/;
const MAX_COMPARE_WINDOW_RUNS = 24;
const TEXT_VARIANT_MIN = 1;
const TEXT_VARIANT_MAX = 30;
const TEXT_MAX_SPINE_ID_LENGTH = 128;
const TEXT_MAX_TEMPLATE_ID_LENGTH = 128;
const TEXT_CAMPAIGN_SPINE_PATTERN = /^spine\.[a-z0-9][a-z0-9._-]{0,111}\.v\d+$/;
const TEXT_TEMPLATE_ID_PATTERN = /^[a-z0-9][a-z0-9._-]{0,127}$/;
const TEXT_ALLOWED_ROUTE_POLICIES = new Set(['economy', 'balanced', 'quality']);
const TEXT_MAX_PRODUCT_NAME_LENGTH = 80;
const TEXT_MAX_OFFER_SUMMARY_LENGTH = 140;
const TEXT_MAX_POSITIONING_LENGTH = 160;
const TEXT_MAX_BIG_IDEA_LENGTH = 120;
const TEXT_MAX_PROOF_CLAIM_LENGTH = 120;
const TEXT_MAX_AUDIENCE_SEGMENTS = 12;
const TEXT_MAX_AUDIENCE_SEGMENT_LENGTH = 64;

const el = {
  profileId: document.getElementById('profileId'),
  startDate: document.getElementById('startDate'),
  endDate: document.getElementById('endDate'),
  campaignFilter: document.getElementById('campaignFilter'),
  adGroupFilter: document.getElementById('adGroupFilter'),
  seed: document.getElementById('seed'),
  compareWindowRuns: document.getElementById('compareWindowRuns'),
  targetRoas: document.getElementById('targetRoas'),
  monthlyRevenueTarget: document.getElementById('monthlyRevenueTarget'),
  includeNarratives: document.getElementById('includeNarratives'),
  analyticsInputErrors: document.getElementById('analyticsInputErrors'),
  runButton: document.getElementById('runButton'),
  refreshButton: document.getElementById('refreshButton'),
  refreshStamp: document.getElementById('refreshStamp'),
  jobStatus: document.getElementById('jobStatus'),
  kpiGrid: document.getElementById('kpiGrid'),
  runIdBadge: document.getElementById('runIdBadge'),
  qualityList: document.getElementById('qualityList'),
  dataQualityPanel: document.getElementById('dataQualityPanel'),
  driftList: document.getElementById('driftList'),
  campaignTableBody: document.getElementById('campaignTableBody'),
  funnelTableBody: document.getElementById('funnelTableBody'),
  storefrontTableBody: document.getElementById('storefrontTableBody'),
  forecastPanel: document.getElementById('forecastPanel'),
  publishGatePanel: document.getElementById('publishGatePanel'),
  decisionFeedList: document.getElementById('decisionFeedList'),
  exportPacketButton: document.getElementById('exportPacketButton'),
  narrativeList: document.getElementById('narrativeList'),
  historyList: document.getElementById('historyList'),
  revenueTruthPanel: document.getElementById('revenueTruthPanel'),
  revenueTruthRiskChip: document.getElementById('revenueTruthRiskChip'),
  funnelSurvivalSummary: document.getElementById('funnelSurvivalSummary'),
  attributionDeltaSummary: document.getElementById('attributionDeltaSummary'),
  attributionDeltaTableBody: document.getElementById('attributionDeltaTableBody'),
  highLeverageScorecardPanel: document.getElementById('highLeverageScorecardPanel'),
  loadTextTemplatesButton: document.getElementById('loadTextTemplatesButton'),
  runTextWorkflowButton: document.getElementById('runTextWorkflowButton'),
  textCampaignSpineId: document.getElementById('textCampaignSpineId'),
  textTemplateSelect: document.getElementById('textTemplateSelect'),
  textVariantCount: document.getElementById('textVariantCount'),
  textProductName: document.getElementById('textProductName'),
  textOfferSummary: document.getElementById('textOfferSummary'),
  textAudienceSegments: document.getElementById('textAudienceSegments'),
  textPositioningStatement: document.getElementById('textPositioningStatement'),
  textBigIdea: document.getElementById('textBigIdea'),
  textProofClaim: document.getElementById('textProofClaim'),
  textRoutePolicy: document.getElementById('textRoutePolicy'),
  textIncludeEvidence: document.getElementById('textIncludeEvidence'),
  textPaidCallsAllowed: document.getElementById('textPaidCallsAllowed'),
  textWorkflowInputErrors: document.getElementById('textWorkflowInputErrors'),
  textBudgetSummary: document.getElementById('textBudgetSummary'),
  textExportPacketButton: document.getElementById('textExportPacketButton'),
  textWorkflowStatus: document.getElementById('textWorkflowStatus'),
  textTemplateSummary: document.getElementById('textTemplateSummary'),
  textWorkflowGatePanel: document.getElementById('textWorkflowGatePanel'),
  textWorkflowTraceBody: document.getElementById('textWorkflowTraceBody'),
  textWorkflowSections: document.getElementById('textWorkflowSections'),
  textWorkflowFindings: document.getElementById('textWorkflowFindings')
};

boot();

async function boot() {
  wireEvents();
  updateTextBudgetSummary();
  await loadTextWorkflowTemplates();
  renderTextWorkflowResult(null);
  await refreshDashboard();
  setInterval(() => {
    refreshHistoryOnly().catch(() => {
      /* no-op */
    });
  }, 45000);
}

function wireEvents() {
  el.runButton.addEventListener('click', () => generateRunAndRefresh());
  el.refreshButton.addEventListener('click', () => refreshDashboard());
  el.loadTextTemplatesButton?.addEventListener('click', () => loadTextWorkflowTemplates());
  el.runTextWorkflowButton?.addEventListener('click', () => runTextWorkflowAndRender());
  el.textExportPacketButton?.addEventListener('click', () => exportTextWorkflowPacket());
  el.textRoutePolicy?.addEventListener('change', () => {
    updateTextBudgetSummary();
    validateTextWorkflowInputs({ showSummary: false, requireTemplate: false });
  });
  wireAnalyticsInputValidation();
  wireTextWorkflowInputValidation();
  el.exportPacketButton?.addEventListener('click', () => {
    status('Export packet is not yet wired to a file command. Gate status is active.');
  });
}

async function invoke(command, payload = {}) {
  const tauriInvoke = window.__TAURI__?.core?.invoke;
  if (!tauriInvoke) {
    throw new Error('Tauri runtime unavailable. Open this through the desktop app.');
  }
  return tauriInvoke(command, payload);
}

function status(text) {
  setStatusTone(el.jobStatus, text, inferToneFromMessage(text));
}

function textWorkflowStatus(text) {
  setStatusTone(el.textWorkflowStatus, text, inferToneFromMessage(text));
}

function setStatusTone(element, text, tone = 'info') {
  if (!element) return;
  element.textContent = text;
  element.classList.remove('is-info', 'is-success', 'is-warn', 'is-danger');
  element.classList.add(`is-${tone}`);
}

function inferToneFromMessage(text) {
  const normalized = String(text || '').toLowerCase();
  if (
    normalized.includes('failed') ||
    normalized.includes('error')
  ) {
    return 'danger';
  }
  if (
    normalized.includes('canceled') ||
    normalized.includes('warning') ||
    normalized.includes('needs review') ||
    normalized.includes('needs attention') ||
    normalized.includes('paused')
  ) {
    return 'warn';
  }
  if (
    normalized.includes('completed') ||
    normalized.includes('loaded') ||
    normalized.includes('ready')
  ) {
    return 'success';
  }
  return 'info';
}

function setButtonBusy(button, busy) {
  if (!button) return;
  if (busy) {
    button.dataset.originalLabel = button.textContent;
    button.disabled = true;
    button.textContent = 'Working...';
    return;
  }
  button.disabled = false;
  if (button.dataset.originalLabel) {
    button.textContent = button.dataset.originalLabel;
    delete button.dataset.originalLabel;
  }
}

function updateTextBudgetSummary() {
  const preset = resolveRoutePolicyPreset(el.textRoutePolicy?.value);
  if (!el.textBudgetSummary) return;
  el.textBudgetSummary.innerHTML = `<div class="text-workflow-meta">
    <strong>Routing mode: ${escapeHtml(preset.label)}</strong><br/>
    Estimated max per run: $${fmtNum(preset.max_cost_per_run_usd, 2)} |
    Input token budget: ${fmtInt(preset.max_total_input_tokens)} |
    Output token budget: ${fmtInt(preset.max_total_output_tokens)}<br/>
    Daily hard cap remains $${fmtNum(preset.hard_daily_cap_usd, 2)} regardless of mode.
  </div>`;
}

function stampNow(prefix = 'Updated') {
  el.refreshStamp.textContent = `${prefix}: ${new Date().toLocaleString()}`;
}

function parseOptionalInt(value) {
  if (!value || !String(value).trim()) return null;
  const n = Number(value);
  return Number.isFinite(n) ? Math.trunc(n) : null;
}

function parseOptionalFloat(value) {
  if (!value || !String(value).trim()) return null;
  const n = Number(value);
  return Number.isFinite(n) ? n : null;
}

function currentPhaseOptions() {
  return {
    compareWindowRuns: parseOptionalInt(el.compareWindowRuns?.value) || 1,
    targetRoas: parseOptionalFloat(el.targetRoas?.value),
    monthlyRevenueTarget: parseOptionalFloat(el.monthlyRevenueTarget?.value)
  };
}

function wireAnalyticsInputValidation() {
  const fields = analyticsValidationFields();
  for (const field of fields) {
    field?.addEventListener('input', () => validateAnalyticsRunInputs({ showSummary: false }));
    field?.addEventListener('change', () => validateAnalyticsRunInputs({ showSummary: false }));
    field?.addEventListener('blur', () => validateAnalyticsRunInputs({ showSummary: false }));
  }
}

function wireTextWorkflowInputValidation() {
  const fields = textWorkflowValidationFields();
  for (const field of fields) {
    field?.addEventListener('input', () => validateTextWorkflowInputs({ showSummary: false, requireTemplate: false }));
    field?.addEventListener('change', () => validateTextWorkflowInputs({ showSummary: false, requireTemplate: false }));
    field?.addEventListener('blur', () => validateTextWorkflowInputs({ showSummary: false, requireTemplate: false }));
  }
}

function textWorkflowValidationFields() {
  return [
    el.textCampaignSpineId,
    el.textTemplateSelect,
    el.textVariantCount,
    el.textRoutePolicy,
    el.textProductName,
    el.textOfferSummary,
    el.textAudienceSegments,
    el.textPositioningStatement,
    el.textBigIdea,
    el.textProofClaim
  ];
}

function analyticsValidationFields() {
  return [
    el.profileId,
    el.startDate,
    el.endDate,
    el.campaignFilter,
    el.adGroupFilter,
    el.seed,
    el.compareWindowRuns,
    el.targetRoas,
    el.monthlyRevenueTarget
  ];
}

function clearAnalyticsInputValidationState() {
  for (const field of analyticsValidationFields()) {
    if (!field) continue;
    field.classList.remove('is-invalid');
    field.removeAttribute('aria-invalid');
  }
}

function clearTextWorkflowInputValidationState() {
  for (const field of textWorkflowValidationFields()) {
    if (!field) continue;
    field.classList.remove('is-invalid');
    field.removeAttribute('aria-invalid');
  }
}

function markFieldInvalid(field, message) {
  if (!field) return;
  field.classList.add('is-invalid');
  field.setAttribute('aria-invalid', 'true');
  field.dataset.validationError = message;
}

function renderAnalyticsInputErrors(errors, { showSummary = true } = {}) {
  if (!el.analyticsInputErrors) return;
  if (!errors.length || !showSummary) {
    el.analyticsInputErrors.classList.remove('is-visible');
    el.analyticsInputErrors.innerHTML = '';
    return;
  }
  el.analyticsInputErrors.classList.add('is-visible');
  el.analyticsInputErrors.innerHTML = `
    <div class="input-errors-title">Please fix the highlighted inputs:</div>
    <ul>
      ${errors.map(err => `<li class="input-error-item">${escapeHtml(err)}</li>`).join('')}
    </ul>
  `;
}

function renderTextWorkflowInputErrors(errors, { showSummary = true } = {}) {
  if (!el.textWorkflowInputErrors) return;
  if (!errors.length || !showSummary) {
    el.textWorkflowInputErrors.classList.remove('is-visible');
    el.textWorkflowInputErrors.innerHTML = '';
    return;
  }
  el.textWorkflowInputErrors.classList.add('is-visible');
  el.textWorkflowInputErrors.innerHTML = `
    <div class="input-errors-title">Please fix the highlighted text workflow inputs:</div>
    <ul>
      ${errors.map(err => `<li class="input-error-item">${escapeHtml(err)}</li>`).join('')}
    </ul>
  `;
}

function parseIsoDateInput(value) {
  const text = String(value || '').trim();
  if (!/^\d{4}-\d{2}-\d{2}$/.test(text)) return null;
  const [yearText, monthText, dayText] = text.split('-');
  const year = Number(yearText);
  const month = Number(monthText);
  const day = Number(dayText);
  if (!Number.isInteger(year) || !Number.isInteger(month) || !Number.isInteger(day)) return null;
  const date = new Date(Date.UTC(year, month - 1, day));
  if (
    date.getUTCFullYear() !== year ||
    date.getUTCMonth() !== month - 1 ||
    date.getUTCDate() !== day
  ) {
    return null;
  }
  return { raw: text, date };
}

function validateAnalyticsRunInputs({ showSummary = true } = {}) {
  const errors = [];
  clearAnalyticsInputValidationState();

  const profileIdRaw = String(el.profileId?.value || '');
  const profileId = profileIdRaw.trim();
  if (!profileId) {
    errors.push('Workspace Profile is required.');
    markFieldInvalid(el.profileId, 'Workspace Profile is required.');
  } else {
    if (profileId.length > MAX_PROFILE_ID_LENGTH) {
      errors.push(`Workspace Profile must be <= ${MAX_PROFILE_ID_LENGTH} characters.`);
      markFieldInvalid(el.profileId, `Workspace Profile must be <= ${MAX_PROFILE_ID_LENGTH} characters.`);
    }
    if (!PROFILE_ID_PATTERN.test(profileId)) {
      errors.push('Workspace Profile must be lowercase slug format (letters, numbers, underscore, hyphen).');
      markFieldInvalid(el.profileId, 'Use lowercase slug format (letters, numbers, underscore, hyphen).');
    }
  }

  const start = parseIsoDateInput(el.startDate?.value);
  const end = parseIsoDateInput(el.endDate?.value);
  if (!start) {
    errors.push('Start Date must use YYYY-MM-DD.');
    markFieldInvalid(el.startDate, 'Start Date must use YYYY-MM-DD.');
  }
  if (!end) {
    errors.push('End Date must use YYYY-MM-DD.');
    markFieldInvalid(el.endDate, 'End Date must use YYYY-MM-DD.');
  }
  if (start && end) {
    if (start.date > end.date) {
      errors.push('Start Date must be on or before End Date.');
      markFieldInvalid(el.startDate, 'Start Date must be on or before End Date.');
      markFieldInvalid(el.endDate, 'End Date must be on or after Start Date.');
    } else {
      const spanDays = Math.round((end.date - start.date) / 86_400_000) + 1;
      if (spanDays > MAX_ANALYTICS_DATE_SPAN_DAYS) {
        errors.push(`Date range cannot exceed ${MAX_ANALYTICS_DATE_SPAN_DAYS} days.`);
        markFieldInvalid(el.startDate, `Date range cannot exceed ${MAX_ANALYTICS_DATE_SPAN_DAYS} days.`);
        markFieldInvalid(el.endDate, `Date range cannot exceed ${MAX_ANALYTICS_DATE_SPAN_DAYS} days.`);
      }
    }
  }

  const campaignFilter = cleanText(el.campaignFilter?.value);
  if (campaignFilter && campaignFilter.length > 128) {
    errors.push('Campaign Filter must be <= 128 characters.');
    markFieldInvalid(el.campaignFilter, 'Campaign Filter must be <= 128 characters.');
  }

  const adGroupFilter = cleanText(el.adGroupFilter?.value);
  if (adGroupFilter && adGroupFilter.length > 128) {
    errors.push('Ad Group Filter must be <= 128 characters.');
    markFieldInvalid(el.adGroupFilter, 'Ad Group Filter must be <= 128 characters.');
  }

  const seedRaw = String(el.seed?.value || '').trim();
  let seed = null;
  if (seedRaw) {
    if (!/^\d+$/.test(seedRaw)) {
      errors.push('Deterministic Seed must be an unsigned integer.');
      markFieldInvalid(el.seed, 'Deterministic Seed must be an unsigned integer.');
    } else {
      seed = Number(seedRaw);
      if (!Number.isSafeInteger(seed) || seed < 0) {
        errors.push(`Deterministic Seed must be between 0 and ${Number.MAX_SAFE_INTEGER}.`);
        markFieldInvalid(el.seed, `Seed must be between 0 and ${Number.MAX_SAFE_INTEGER}.`);
      }
    }
  }

  const compareWindowRaw = String(el.compareWindowRuns?.value || '').trim();
  const compareWindowRuns = /^\d+$/.test(compareWindowRaw)
    ? Number(compareWindowRaw)
    : NaN;
  if (!Number.isInteger(compareWindowRuns) || compareWindowRuns < 1 || compareWindowRuns > MAX_COMPARE_WINDOW_RUNS) {
    errors.push(`Baseline Window must be an integer between 1 and ${MAX_COMPARE_WINDOW_RUNS}.`);
    markFieldInvalid(el.compareWindowRuns, `Use an integer between 1 and ${MAX_COMPARE_WINDOW_RUNS}.`);
  }

  const targetRoas = parseOptionalFloat(el.targetRoas?.value);
  if (targetRoas != null && targetRoas < 0) {
    errors.push('Target ROAS must be a non-negative number.');
    markFieldInvalid(el.targetRoas, 'Target ROAS must be a non-negative number.');
  }

  const monthlyRevenueTarget = parseOptionalFloat(el.monthlyRevenueTarget?.value);
  if (monthlyRevenueTarget != null && monthlyRevenueTarget < 0) {
    errors.push('Monthly Revenue Goal must be a non-negative number.');
    markFieldInvalid(el.monthlyRevenueTarget, 'Monthly Revenue Goal must be a non-negative number.');
  }

  renderAnalyticsInputErrors(errors, { showSummary });
  if (errors.length) return { ok: false, errors };

  return {
    ok: true,
    values: {
      profileId,
      startDate: start.raw,
      endDate: end.raw,
      campaignFilter,
      adGroupFilter,
      seed
    }
  };
}

function validateTextWorkflowTemplateLoadInputs({ showSummary = true } = {}) {
  const errors = [];
  clearTextWorkflowInputValidationState();
  const campaignSpineId = cleanText(el.textCampaignSpineId?.value);

  if (!campaignSpineId) {
    errors.push('Campaign Spine ID is required.');
    markFieldInvalid(el.textCampaignSpineId, 'Campaign Spine ID is required.');
  } else {
    if (campaignSpineId.length > TEXT_MAX_SPINE_ID_LENGTH) {
      errors.push(`Campaign Spine ID must be <= ${TEXT_MAX_SPINE_ID_LENGTH} characters.`);
      markFieldInvalid(el.textCampaignSpineId, `Campaign Spine ID must be <= ${TEXT_MAX_SPINE_ID_LENGTH} characters.`);
    }
    if (!TEXT_CAMPAIGN_SPINE_PATTERN.test(campaignSpineId)) {
      errors.push('Campaign Spine ID must match spine.<name>.v<number> (example: spine.default.v1).');
      markFieldInvalid(el.textCampaignSpineId, 'Use format spine.<name>.v<number>.');
    }
  }

  renderTextWorkflowInputErrors(errors, { showSummary });
  if (errors.length) return { ok: false, errors };
  return { ok: true, values: { campaignSpineId } };
}

function validateTextWorkflowInputs({ showSummary = true, requireTemplate = true } = {}) {
  const errors = [];
  clearTextWorkflowInputValidationState();

  const campaignSpineId = cleanText(el.textCampaignSpineId?.value);
  if (!campaignSpineId) {
    errors.push('Campaign Spine ID is required.');
    markFieldInvalid(el.textCampaignSpineId, 'Campaign Spine ID is required.');
  } else {
    if (campaignSpineId.length > TEXT_MAX_SPINE_ID_LENGTH) {
      errors.push(`Campaign Spine ID must be <= ${TEXT_MAX_SPINE_ID_LENGTH} characters.`);
      markFieldInvalid(el.textCampaignSpineId, `Campaign Spine ID must be <= ${TEXT_MAX_SPINE_ID_LENGTH} characters.`);
    }
    if (!TEXT_CAMPAIGN_SPINE_PATTERN.test(campaignSpineId)) {
      errors.push('Campaign Spine ID must match spine.<name>.v<number> (example: spine.default.v1).');
      markFieldInvalid(el.textCampaignSpineId, 'Use format spine.<name>.v<number>.');
    }
  }

  const templateId = cleanText(el.textTemplateSelect?.value);
  if (requireTemplate) {
    if (!templateId) {
      errors.push('Workflow Template is required. Load templates, then select one.');
      markFieldInvalid(el.textTemplateSelect, 'Workflow Template is required.');
    } else {
      if (templateId.length > TEXT_MAX_TEMPLATE_ID_LENGTH) {
        errors.push(`Workflow Template must be <= ${TEXT_MAX_TEMPLATE_ID_LENGTH} characters.`);
        markFieldInvalid(el.textTemplateSelect, `Workflow Template must be <= ${TEXT_MAX_TEMPLATE_ID_LENGTH} characters.`);
      }
      if (!TEXT_TEMPLATE_ID_PATTERN.test(templateId)) {
        errors.push('Workflow Template must be lowercase slug/dot format (example: tpl.email_landing_sequence.v1).');
        markFieldInvalid(el.textTemplateSelect, 'Use lowercase slug/dot format.');
      }
    }
  }

  const variantRaw = String(el.textVariantCount?.value || '').trim();
  let variantCount = NaN;
  if (!/^\d+$/.test(variantRaw)) {
    errors.push(`Ad Variant Count must be an integer between ${TEXT_VARIANT_MIN} and ${TEXT_VARIANT_MAX}.`);
    markFieldInvalid(el.textVariantCount, `Use an integer between ${TEXT_VARIANT_MIN} and ${TEXT_VARIANT_MAX}.`);
  } else {
    variantCount = Number(variantRaw);
    if (!Number.isInteger(variantCount) || variantCount < TEXT_VARIANT_MIN || variantCount > TEXT_VARIANT_MAX) {
      errors.push(`Ad Variant Count must be an integer between ${TEXT_VARIANT_MIN} and ${TEXT_VARIANT_MAX}.`);
      markFieldInvalid(el.textVariantCount, `Use an integer between ${TEXT_VARIANT_MIN} and ${TEXT_VARIANT_MAX}.`);
    }
  }

  const routePolicyId = String(el.textRoutePolicy?.value || '').trim().toLowerCase();
  if (!TEXT_ALLOWED_ROUTE_POLICIES.has(routePolicyId)) {
    errors.push('Route Policy must be one of: economy, balanced, quality.');
    markFieldInvalid(el.textRoutePolicy, 'Use economy, balanced, or quality.');
  }

  const productName = cleanText(el.textProductName?.value);
  if (!productName) {
    errors.push('Product Name is required.');
    markFieldInvalid(el.textProductName, 'Product Name is required.');
  } else if (productName.length > TEXT_MAX_PRODUCT_NAME_LENGTH) {
    errors.push(`Product Name must be <= ${TEXT_MAX_PRODUCT_NAME_LENGTH} characters.`);
    markFieldInvalid(el.textProductName, `Product Name must be <= ${TEXT_MAX_PRODUCT_NAME_LENGTH} characters.`);
  }

  const offerSummary = cleanText(el.textOfferSummary?.value);
  if (!offerSummary) {
    errors.push('Offer Summary is required.');
    markFieldInvalid(el.textOfferSummary, 'Offer Summary is required.');
  } else if (offerSummary.length > TEXT_MAX_OFFER_SUMMARY_LENGTH) {
    errors.push(`Offer Summary must be <= ${TEXT_MAX_OFFER_SUMMARY_LENGTH} characters.`);
    markFieldInvalid(el.textOfferSummary, `Offer Summary must be <= ${TEXT_MAX_OFFER_SUMMARY_LENGTH} characters.`);
  }

  const positioningStatement = cleanText(el.textPositioningStatement?.value);
  if (!positioningStatement) {
    errors.push('Positioning Statement is required.');
    markFieldInvalid(el.textPositioningStatement, 'Positioning Statement is required.');
  } else if (positioningStatement.length > TEXT_MAX_POSITIONING_LENGTH) {
    errors.push(`Positioning Statement must be <= ${TEXT_MAX_POSITIONING_LENGTH} characters.`);
    markFieldInvalid(el.textPositioningStatement, `Positioning Statement must be <= ${TEXT_MAX_POSITIONING_LENGTH} characters.`);
  }

  const bigIdea = cleanText(el.textBigIdea?.value);
  if (!bigIdea) {
    errors.push('Message Big Idea is required.');
    markFieldInvalid(el.textBigIdea, 'Message Big Idea is required.');
  } else if (bigIdea.length > TEXT_MAX_BIG_IDEA_LENGTH) {
    errors.push(`Message Big Idea must be <= ${TEXT_MAX_BIG_IDEA_LENGTH} characters.`);
    markFieldInvalid(el.textBigIdea, `Message Big Idea must be <= ${TEXT_MAX_BIG_IDEA_LENGTH} characters.`);
  }

  const proofClaim = cleanText(el.textProofClaim?.value);
  if (!proofClaim) {
    errors.push('Primary Proof Claim is required.');
    markFieldInvalid(el.textProofClaim, 'Primary Proof Claim is required.');
  } else if (proofClaim.length > TEXT_MAX_PROOF_CLAIM_LENGTH) {
    errors.push(`Primary Proof Claim must be <= ${TEXT_MAX_PROOF_CLAIM_LENGTH} characters.`);
    markFieldInvalid(el.textProofClaim, `Primary Proof Claim must be <= ${TEXT_MAX_PROOF_CLAIM_LENGTH} characters.`);
  }

  const audienceSegments = String(el.textAudienceSegments?.value || '')
    .split(',')
    .map(segment => segment.trim())
    .filter(Boolean);
  if (!audienceSegments.length) {
    errors.push('Audience Segments requires at least one segment.');
    markFieldInvalid(el.textAudienceSegments, 'Provide at least one comma-separated segment.');
  }
  if (audienceSegments.length > TEXT_MAX_AUDIENCE_SEGMENTS) {
    errors.push(`Audience Segments supports up to ${TEXT_MAX_AUDIENCE_SEGMENTS} segments.`);
    markFieldInvalid(el.textAudienceSegments, `Use ${TEXT_MAX_AUDIENCE_SEGMENTS} segments or fewer.`);
  }
  const oversizedAudienceSegment = audienceSegments.find(segment => segment.length > TEXT_MAX_AUDIENCE_SEGMENT_LENGTH);
  if (oversizedAudienceSegment) {
    errors.push(`Each Audience Segment must be <= ${TEXT_MAX_AUDIENCE_SEGMENT_LENGTH} characters.`);
    markFieldInvalid(el.textAudienceSegments, `Each segment must be <= ${TEXT_MAX_AUDIENCE_SEGMENT_LENGTH} chars.`);
  }

  renderTextWorkflowInputErrors(errors, { showSummary });
  if (errors.length) return { ok: false, errors };

  return {
    ok: true,
    values: {
      campaignSpineId,
      templateId: templateId || 'tpl.email_landing_sequence.v1',
      variantCount,
      routePolicyId,
      productName,
      offerSummary,
      audienceSegments,
      positioningStatement,
      bigIdea,
      proofClaim,
      includeEvidence: !!el.textIncludeEvidence?.checked,
      paidCallsAllowed: !!el.textPaidCallsAllowed?.checked
    }
  };
}

async function generateRunAndRefresh() {
  status('Validating analytics inputs...');
  setButtonBusy(el.runButton, true);
  const validation = validateAnalyticsRunInputs({ showSummary: true });
  if (!validation.ok) {
    status('Input validation failed. Fix highlighted fields and try again.');
    setButtonBusy(el.runButton, false);
    return;
  }

  const profileId = validation.values.profileId;
  status('Submitting analytics snapshot request...');
  const preflight = await runConnectorPreflight(profileId);
  if (!preflight.ok) {
    const reasons = (preflight.blocking_reasons || []).join(' | ') || 'connector preflight failed';
    status(`Run blocked by connector preflight: ${reasons}`);
    setButtonBusy(el.runButton, false);
    return;
  }

  const request = {
    start_date: validation.values.startDate,
    end_date: validation.values.endDate,
    campaign_filter: validation.values.campaignFilter,
    ad_group_filter: validation.values.adGroupFilter,
    seed: validation.values.seed,
    profile_id: profileId,
    include_narratives: el.includeNarratives.checked,
    budget_envelope: defaultBudgetEnvelope()
  };

  try {
    const handle = await invoke('start_mock_analytics_job', { request });
    status(`Snapshot job ${handle.job_id} started.`);

    while (true) {
      const snapshot = await invoke('get_tool_job', { jobId: handle.job_id });
      status(`${snapshot.progress_pct}% • ${snapshot.stage} • ${snapshot.message || 'running'}`);

      if (snapshot.status === 'succeeded') break;
      if (snapshot.status === 'failed' || snapshot.status === 'canceled') {
        const reason = snapshot.message || 'execution failed';
        status(snapshot.status === 'canceled'
          ? `Run canceled: ${reason}`
          : `Run needs attention: ${reason}`);
        return;
      }
      await sleep(350);
    }

    await refreshDashboard();
    status('Snapshot completed and dashboard refreshed.');
    stampNow('Run complete');
  } catch (err) {
    status(`We couldn't complete that run: ${String(err)}`);
  } finally {
    setButtonBusy(el.runButton, false);
  }
}

async function runConnectorPreflight(profileId) {
  try {
    const preflight = await invoke('validate_analytics_connectors_preflight', {});
    if (!preflight?.ok) {
      return { ok: false, blocking_reasons: preflight?.blocking_reasons || [] };
    }
    return { ok: true };
  } catch (err) {
    return { ok: false, blocking_reasons: [`preflight command failed for ${profileId}: ${String(err)}`] };
  }
}

function defaultBudgetEnvelope() {
  return {
    max_retrieval_units: 20000,
    max_analysis_units: 10000,
    max_llm_tokens_in: 15000,
    max_llm_tokens_out: 8000,
    max_total_cost_micros: 50000000,
    policy: 'fail_closed',
    provenance_ref: 'ui.default_budget_envelope.v1'
  };
}

async function loadTextWorkflowTemplates() {
  const validation = validateTextWorkflowTemplateLoadInputs({ showSummary: true });
  if (!validation.ok) {
    textWorkflowStatus('Input validation failed. Fix highlighted fields and reload templates.');
    return;
  }
  const campaignSpineId = validation.values.campaignSpineId;
  textWorkflowStatus('Loading available workflow templates...');
  setButtonBusy(el.loadTextTemplatesButton, true);
  try {
    const templates = await invoke('get_text_workflow_templates', { campaignSpineId });
    const rows = Array.isArray(templates) ? templates : [];
    state.textWorkflowTemplates = rows;
    renderTemplateOptions(rows);
    textWorkflowStatus(`Loaded ${rows.length} templates for ${campaignSpineId}.`);
  } catch (err) {
    const message = String(err || 'Unknown error');
    textWorkflowStatus(`Couldn't load templates yet: ${message}`);
  } finally {
    setButtonBusy(el.loadTextTemplatesButton, false);
  }
}

function renderTemplateOptions(templates) {
  if (!el.textTemplateSelect) return;
  if (!templates.length) {
    el.textTemplateSelect.innerHTML = '<option value="">No templates available</option>';
    el.textTemplateSummary.textContent = 'No templates were found for this campaign spine id. Try loading again or confirm the ID.';
    return;
  }
  el.textTemplateSelect.innerHTML = templates
    .map(t => `<option value="${escapeHtml(t.template_id)}">${escapeHtml(t.title || t.template_id)}</option>`)
    .join('');
  const active = templates[0];
  renderTemplateSummary(active);
  el.textTemplateSelect.onchange = () => {
    const selected = templates.find(t => t.template_id === el.textTemplateSelect.value);
    if (selected) renderTemplateSummary(selected);
  };
}

function renderTemplateSummary(template) {
  if (!template) {
    el.textTemplateSummary.textContent = 'No template selected.';
    return;
  }
  const nodeCount = Array.isArray(template.graph?.nodes) ? template.graph.nodes.length : 0;
  const edgeCount = Array.isArray(template.graph?.edges) ? template.graph.edges.length : 0;
  const workflow = template.workflow_kind || 'unknown';
  el.textTemplateSummary.innerHTML = `<div class="text-workflow-meta">
    <strong>${escapeHtml(template.title || template.template_id)}</strong><br/>
    Template: ${escapeHtml(template.template_id)}<br/>
    Workflow: ${escapeHtml(workflow)} | Graph: ${escapeHtml(template.graph?.graph_id || 'n/a')}<br/>
    Nodes: ${nodeCount} | Edges: ${edgeCount}
  </div>`;
}

async function runTextWorkflowAndRender() {
  const validation = validateTextWorkflowInputs({ showSummary: true, requireTemplate: true });
  if (!validation.ok) {
    textWorkflowStatus('Input validation failed. Fix highlighted text workflow fields and try again.');
    return;
  }
  const request = buildTextWorkflowRequest(validation.values);
  textWorkflowStatus('Starting text workflow run...');
  setButtonBusy(el.runTextWorkflowButton, true);
  if (el.textExportPacketButton) {
    el.textExportPacketButton.disabled = true;
  }
  try {
    const lifecycle = await runTextWorkflowJobLifecycle({
      invoke,
      request,
      pollIntervalMs: 300,
      onSnapshot: (snapshot) => {
        textWorkflowStatus(
          `${snapshot.progress_pct}% • ${snapshot.stage} • ${snapshot.message || 'running'}`
        );
      }
    });

    if (lifecycle.status === 'succeeded') {
      state.textWorkflowResult = lifecycle.result;
      renderTextWorkflowResult(state.textWorkflowResult);
      textWorkflowStatus('Text workflow finished and is ready for review.');
      return;
    }

    const reason = lifecycle.errorMessage || 'execution failed';
    textWorkflowStatus(
      lifecycle.status === 'canceled'
        ? `Workflow canceled: ${reason}`
        : `Workflow needs attention: ${reason}`
    );
  } catch (err) {
    textWorkflowStatus(`Couldn't run workflow: ${String(err)}`);
  } finally {
    setButtonBusy(el.runTextWorkflowButton, false);
  }
}

function buildTextWorkflowRequest(values = null) {
  const resolved = values || {};
  return buildTextWorkflowRequestFromInputs({
    routePolicyId: resolved.routePolicyId || (el.textRoutePolicy?.value || 'balanced'),
    templateId: resolved.templateId || (cleanText(el.textTemplateSelect.value) || 'tpl.email_landing_sequence.v1'),
    variantCount: resolved.variantCount ?? (parseOptionalInt(el.textVariantCount.value) || 12),
    paidCallsAllowed: resolved.paidCallsAllowed ?? !!el.textPaidCallsAllowed.checked,
    campaignSpineId: resolved.campaignSpineId || (cleanText(el.textCampaignSpineId.value) || 'spine.default.v1'),
    productName: resolved.productName || (cleanText(el.textProductName.value) || "Nature's Diet Raw Mix"),
    offerSummary: resolved.offerSummary || (cleanText(el.textOfferSummary.value) || 'Save 20% on first order'),
    audienceSegments: resolved.audienceSegments || (cleanText(el.textAudienceSegments.value) || ''),
    positioningStatement:
      resolved.positioningStatement || (cleanText(el.textPositioningStatement.value) || 'Raw-first nutrition with practical prep'),
    bigIdea: resolved.bigIdea || (cleanText(el.textBigIdea.value) || 'Fresh confidence in every bowl'),
    proofClaim: resolved.proofClaim || (cleanText(el.textProofClaim.value) || 'high digestibility blend'),
    includeEvidence: resolved.includeEvidence ?? !!el.textIncludeEvidence.checked
  });
}

function renderTextWorkflowResult(result) {
  const gatePanel = el.textWorkflowGatePanel;
  const traceBody = el.textWorkflowTraceBody;
  const sectionsPanel = el.textWorkflowSections;
  const findingsPanel = el.textWorkflowFindings;
  if (!gatePanel || !traceBody || !sectionsPanel || !findingsPanel) return;

  if (!result) {
    gatePanel.innerHTML = `
      <div class="gate-card">
        <h3>Status</h3>
        <p>No text workflow run yet.</p>
      </div>`;
    traceBody.innerHTML = '<tr><td colspan="7">No trace rows yet.</td></tr>';
    sectionsPanel.innerHTML = '<div class="narrative-item">No sections generated yet.</div>';
    findingsPanel.innerHTML = '<li class="signal-item neutral">No findings yet.</li>';
    if (el.textExportPacketButton) {
      el.textExportPacketButton.disabled = true;
      el.textExportPacketButton.title = 'No text workflow run available to export.';
    }
    return;
  }

  const gate = deriveTextWorkflowGateState(result);
  gatePanel.innerHTML = buildTextWorkflowGateMarkup(result);
  if (el.textExportPacketButton) {
    el.textExportPacketButton.disabled = !gate.canExport;
    el.textExportPacketButton.title = gate.canExport
      ? 'Export governance packet for this text workflow run.'
      : gate.exportBlockReason;
  }

  const traces = Array.isArray(result.traces) ? result.traces : [];
  traceBody.innerHTML = traces.length
    ? traces.map(trace => `<tr>
      <td>${escapeHtml(trace.node_id || 'n/a')}</td>
      <td>${escapeHtml(trace.node_kind || 'n/a')}</td>
      <td>${escapeHtml(trace.route?.provider || 'n/a')}</td>
      <td>${escapeHtml(trace.route?.model || 'n/a')}</td>
      <td>${fmtInt(trace.estimated_input_tokens || 0)}</td>
      <td>${fmtInt(trace.estimated_output_tokens || 0)}</td>
      <td>$${fmtNum(trace.estimated_cost_usd || 0, 4)}</td>
    </tr>`).join('')
    : '<tr><td colspan="7">No trace rows generated.</td></tr>';

  const sections = Array.isArray(result?.artifact?.sections) ? result.artifact.sections : [];
  sectionsPanel.innerHTML = sections.length
    ? sections.slice(0, 8).map(section => `<div class="narrative-item">
        <strong>${escapeHtml(section.section_title || section.section_id || 'Section')}</strong><br/>
        ${escapeHtml(section.content || '')}
      </div>`).join('')
    : '<div class="narrative-item">No sections generated.</div>';

  const findings = Array.isArray(result?.artifact?.critique_findings) ? result.artifact.critique_findings : [];
  findingsPanel.innerHTML = findings.length
    ? findings.map(finding => {
      const sev = String(finding.severity || '').toLowerCase();
      const cls = sev === 'critical' || sev === 'high' ? 'bad' : (sev === 'medium' ? 'warn' : 'neutral');
      return `<li class="signal-item ${cls}"><strong>${escapeHtml(finding.code || 'finding')}</strong><br/>${escapeHtml(finding.message || '')}</li>`;
    }).join('')
    : '<li class="signal-item ok">No critique findings.</li>';
}

function exportTextWorkflowPacket() {
  try {
    const gate = deriveTextWorkflowGateState(state.textWorkflowResult);
    if (!gate.canExport) {
      textWorkflowStatus(gate.exportBlockReason || 'Export is unavailable until review items are resolved.');
      return;
    }

    const markdown = buildTextWorkflowExportPacketMarkdown(state.textWorkflowResult);
    const filename = buildTextWorkflowExportFilename(state.textWorkflowResult, new Date());
    const blob = new Blob([markdown], { type: 'text/markdown;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.style.display = 'none';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);

    textWorkflowStatus(`Exported review packet: ${filename}`);
  } catch (err) {
    textWorkflowStatus(`We couldn't export that packet: ${String(err)}`);
  }
}

async function refreshDashboard() {
  const profileId = cleanText(el.profileId.value) || 'marketing_default';
  const opts = currentPhaseOptions();
  const productionLike = isProductionLikeProfile(profileId);

  try {
    await refreshHistoryOnly();
    const executive = await invoke('get_executive_dashboard_snapshot', {
      profileId,
      limit: 32,
      compareWindowRuns: opts.compareWindowRuns,
      targetRoas: opts.targetRoas,
      monthlyRevenueTarget: opts.monthlyRevenueTarget
    });
    state.currentSnapshot = executive;
    renderExecutiveDashboard(executive);
    status('Dashboard data loaded.');
    stampNow('Loaded');
  } catch (err) {
    const message = String(err || 'Unknown error');
    if (message.includes('No persisted analytics runs found')) {
      if (productionLike) {
        renderExecutiveDashboard(emptySnapshot(profileId, opts, message));
        status('No live analytics runs are available yet for this production profile.');
        stampNow('No data');
      } else {
        renderExecutiveDashboard(fallbackSnapshot(profileId, opts));
        status('No saved runs yet. Showing a guided demo snapshot.');
        stampNow('Demo');
      }
      return;
    }
    status(`We couldn't refresh the dashboard: ${message}`);
  }
}

async function refreshHistoryOnly() {
  const profileId = cleanText(el.profileId.value) || 'marketing_default';
  const history = await invoke('get_mock_analytics_run_history', {
    profileId,
    limit: 24
  });
  state.historyRuns = Array.isArray(history) ? history : [];
  renderHistory(state.historyRuns);
}

function renderExecutiveDashboard(snapshot) {
  if (!snapshot) return;

  el.runIdBadge.textContent = `Run: ${snapshot.run_id || 'n/a'} | Compare: ${snapshot.compare_window_runs || 1} run(s)`;
  renderKpis(snapshot.kpis || [], snapshot);
  renderHighLeverageReports(snapshot.high_leverage_reports || {}, snapshot);
  renderDeltaChart(snapshot.historical_analysis?.period_over_period_deltas || []);
  renderChannelMixChart(snapshot.channel_mix_series || [], snapshot.roas_target_band);
  renderQuality(snapshot.quality_controls || {});
  renderDataQuality(snapshot.data_quality || {});
  renderDrift(snapshot.historical_analysis || {});
  renderCampaignTable(snapshot.portfolio_rows || [], snapshot.source_coverage || []);
  renderFunnelTable(snapshot.funnel_summary?.stages || []);
  renderStorefrontTable(snapshot.storefront_behavior_summary || {});
  renderForecast(snapshot.forecast_summary || {});
  renderPublishGate(snapshot.publish_export_gate || {});
  renderDecisionFeed(snapshot.decision_feed || []);
  renderNarratives(snapshot.operator_summary?.attribution_narratives || [], snapshot.alerts || []);
}

function renderHighLeverageReports(reports, snapshot) {
  renderRevenueTruthReport(reports?.revenue_truth || {});
  renderFunnelSurvivalReport(reports?.funnel_survival || {});
  renderAttributionDeltaReport(reports?.attribution_delta || {});
  renderHighLeverageScorecard(reports?.data_quality_scorecard || {}, snapshot?.publish_export_gate || {});
}

function renderRevenueTruthReport(report) {
  const risk = String(report?.inflation_risk || 'unknown').toLowerCase();
  const riskClass = risk === 'high' ? 'bad' : risk === 'medium' ? 'warn' : risk === 'low' ? 'good' : 'neutral';
  if (el.revenueTruthRiskChip) {
    el.revenueTruthRiskChip.textContent = `risk: ${risk}`;
    el.revenueTruthRiskChip.className = `pill risk-pill ${riskClass}`;
  }
  if (!el.revenueTruthPanel) return;

  const canonicalRevenue = Number(report?.canonical_revenue || 0);
  const canonicalConversions = Number(report?.canonical_conversions || 0);
  const strictDup = Number(report?.strict_duplicate_ratio || 0);
  const nearDup = Number(report?.near_duplicate_ratio || 0);
  const revenueAtRisk = Number(report?.estimated_revenue_at_risk || 0);
  const summary = cleanText(report?.summary) || 'No revenue-truth summary available for this run.';

  el.revenueTruthPanel.innerHTML = `
    <div class="report-metrics">
      <div class="report-metric">
        <div class="forecast-label">Canonical Revenue</div>
        <div class="forecast-value">$${fmtNum(canonicalRevenue, 2)}</div>
      </div>
      <div class="report-metric">
        <div class="forecast-label">Canonical Conversions</div>
        <div class="forecast-value">${fmtInt(canonicalConversions)}</div>
      </div>
      <div class="report-metric">
        <div class="forecast-label">Strict Duplicate Ratio</div>
        <div class="forecast-value">${fmtNum(strictDup * 100, 2)}%</div>
      </div>
      <div class="report-metric">
        <div class="forecast-label">Near Duplicate Ratio</div>
        <div class="forecast-value">${fmtNum(nearDup * 100, 2)}%</div>
      </div>
      <div class="report-metric">
        <div class="forecast-label">Estimated Revenue At Risk</div>
        <div class="forecast-value">$${fmtNum(revenueAtRisk, 2)}</div>
      </div>
    </div>
    <div class="narrative-item">${escapeHtml(summary)}</div>
  `;
}

function renderFunnelSurvivalReport(report) {
  const points = Array.isArray(report?.points) ? report.points : [];
  const summary = points.length
    ? `Bottleneck: ${report?.bottleneck_stage || 'n/a'} | Survival to final stage: ${fmtNum((points[points.length - 1]?.survival_rate || 0) * 100, 1)}%`
    : 'No funnel survival analysis available in this run.';
  if (el.funnelSurvivalSummary) {
    el.funnelSurvivalSummary.textContent = summary;
  }

  const ctx = document.getElementById('funnelSurvivalChart');
  if (!ctx || typeof Chart === 'undefined') return;
  if (!points.length) {
    if (state.funnelSurvivalChart) {
      state.funnelSurvivalChart.destroy();
      state.funnelSurvivalChart = null;
    }
    return;
  }

  const labels = points.map(point => point.stage);
  const survival = points.map(point => Number((point.survival_rate * 100).toFixed(2)));
  const hazard = points.map(point => Number((point.hazard_rate * 100).toFixed(2)));
  if (state.funnelSurvivalChart) state.funnelSurvivalChart.destroy();
  state.funnelSurvivalChart = new Chart(ctx, {
    data: {
      labels,
      datasets: [
        {
          type: 'line',
          label: 'Survival %',
          data: survival,
          borderColor: 'rgba(11,143,140,1)',
          backgroundColor: 'rgba(11,143,140,0.12)',
          yAxisID: 'y',
          tension: 0.25
        },
        {
          type: 'bar',
          label: 'Hazard %',
          data: hazard,
          borderColor: 'rgba(216,87,42,1)',
          backgroundColor: 'rgba(216,87,42,0.55)',
          yAxisID: 'y1'
        }
      ]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { mode: 'index', intersect: false },
      scales: {
        y: { position: 'left', min: 0, max: 100, ticks: { callback: value => `${value}%` } },
        y1: { position: 'right', min: 0, max: 100, grid: { drawOnChartArea: false }, ticks: { callback: value => `${value}%` } }
      }
    }
  });
}

function renderAttributionDeltaReport(report) {
  const rows = Array.isArray(report?.rows) ? report.rows : [];
  const dominant = cleanText(report?.dominant_last_touch_campaign) || 'n/a';
  const hhi = Number(report?.last_touch_concentration_hhi || 0);
  const summaryText = cleanText(report?.summary) || 'No attribution summary available.';
  if (el.attributionDeltaSummary) {
    el.attributionDeltaSummary.textContent = `${summaryText} Dominant last-touch campaign: ${dominant}. HHI: ${fmtNum(hhi, 4)}.`;
  }
  if (el.attributionDeltaTableBody) {
    el.attributionDeltaTableBody.innerHTML = rows.length
      ? rows.slice(0, 8).map(row => `<tr>
          <td>${escapeHtml(row.campaign || 'n/a')}</td>
          <td>${fmtNum((row.first_touch_proxy_share || 0) * 100, 1)}%</td>
          <td>${fmtNum((row.assist_share || 0) * 100, 1)}%</td>
          <td>${fmtNum((row.last_touch_share || 0) * 100, 1)}%</td>
          <td>${fmtNum((row.delta_first_vs_last || 0) * 100, 1)}%</td>
        </tr>`).join('')
      : '<tr><td colspan="5">No attribution rows available.</td></tr>';
  }

  const ctx = document.getElementById('attributionDeltaChart');
  if (!ctx || typeof Chart === 'undefined') return;
  if (!rows.length) {
    if (state.attributionDeltaChart) {
      state.attributionDeltaChart.destroy();
      state.attributionDeltaChart = null;
    }
    return;
  }
  const top = rows.slice(0, 6);
  const labels = top.map(row => shortLabel(row.campaign || 'n/a', 18));
  const firstTouch = top.map(row => Number(((row.first_touch_proxy_share || 0) * 100).toFixed(2)));
  const lastTouch = top.map(row => Number(((row.last_touch_share || 0) * 100).toFixed(2)));
  const delta = top.map(row => Number(((row.delta_first_vs_last || 0) * 100).toFixed(2)));

  if (state.attributionDeltaChart) state.attributionDeltaChart.destroy();
  state.attributionDeltaChart = new Chart(ctx, {
    data: {
      labels,
      datasets: [
        {
          type: 'bar',
          label: 'First Touch %',
          data: firstTouch,
          backgroundColor: 'rgba(47,110,165,0.65)',
          borderColor: 'rgba(47,110,165,1)',
          yAxisID: 'y'
        },
        {
          type: 'bar',
          label: 'Last Touch %',
          data: lastTouch,
          backgroundColor: 'rgba(11,143,140,0.65)',
          borderColor: 'rgba(11,143,140,1)',
          yAxisID: 'y'
        },
        {
          type: 'line',
          label: 'Delta (First - Last) %',
          data: delta,
          borderColor: 'rgba(211,63,73,1)',
          backgroundColor: 'rgba(211,63,73,0.15)',
          yAxisID: 'y1',
          tension: 0.2
        }
      ]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { mode: 'index', intersect: false },
      scales: {
        y: { position: 'left', ticks: { callback: value => `${value}%` } },
        y1: { position: 'right', grid: { drawOnChartArea: false }, ticks: { callback: value => `${value}%` } }
      }
    }
  });
}

function renderHighLeverageScorecard(scorecard, gate) {
  if (!el.highLeverageScorecardPanel) return;
  const rows = [
    ['Quality Score', scorecard?.quality_score],
    ['Completeness', scorecard?.completeness_ratio],
    ['Freshness Pass', scorecard?.freshness_pass_ratio],
    ['Reconciliation Pass', scorecard?.reconciliation_pass_ratio],
    ['Cross-Source Pass', scorecard?.cross_source_pass_ratio],
    ['Budget Pass', scorecard?.budget_pass_ratio]
  ];
  const failureCount = Number(scorecard?.high_severity_failures || 0);
  const blockingCount = Number(scorecard?.blocking_reasons_count ?? gate?.blocking_reasons?.length ?? 0);
  const warningCount = Number(scorecard?.warning_reasons_count ?? gate?.warning_reasons?.length ?? 0);
  const gateStatus = cleanText(scorecard?.gate_status || gate?.gate_status) || 'unknown';

  const ratioRows = rows.map(([label, value]) => {
    const ratio = typeof value === 'number' ? value : 0;
    const cls = ratio >= 0.99 ? 'good' : ratio >= 0.95 ? 'warn' : 'bad';
    return `<div class="dq-row"><strong>${escapeHtml(label)}</strong>${fmtNum(ratio * 100, 1)}%<span class="dq-badge ${cls}">${cls}</span></div>`;
  });

  ratioRows.push(`<div class="dq-row"><strong>High Severity Failures</strong>${fmtInt(failureCount)}<span class="dq-badge ${failureCount === 0 ? 'good' : 'bad'}">${failureCount === 0 ? 'clear' : 'review'}</span></div>`);
  ratioRows.push(`<div class="dq-row"><strong>Gate Status</strong>${escapeHtml(gateStatus.replaceAll('_', ' '))}<span class="dq-badge ${gateStatus === 'ready' ? 'good' : gateStatus === 'blocked' ? 'bad' : 'warn'}">${gateStatus}</span></div>`);
  ratioRows.push(`<div class="dq-row"><strong>Blocking / Warning</strong>${fmtInt(blockingCount)} / ${fmtInt(warningCount)}<span class="dq-badge ${(blockingCount === 0 && warningCount === 0) ? 'good' : (blockingCount > 0 ? 'bad' : 'warn')}">${blockingCount > 0 ? 'blocked' : warningCount > 0 ? 'warn' : 'clean'}</span></div>`);

  el.highLeverageScorecardPanel.innerHTML = ratioRows.join('');
}

function renderKpis(kpis, snapshot = null) {
  const cards = Array.isArray(kpis) ? kpis : [];
  if (!cards.length) {
    el.kpiGrid.innerHTML = '<div class="kpi"><div class="kpi-label">No KPI data</div><div class="kpi-value">n/a</div><div class="kpi-note">Run the pipeline to populate this panel.</div></div>';
    return;
  }
  const kpiLookup = buildKpiLookup(cards);
  el.kpiGrid.innerHTML = cards.map((kpi, index) => {
    const delta = formatDelta(kpi.delta_percent);
    const targetDelta = formatDelta(kpi.target_delta_percent);
    const targetText = kpi.target_delta_percent == null ? '' : ` | vs target ${targetDelta}`;
    const tooltipId = `kpi-tooltip-${index}`;
    const explanation = buildKpiExplanation(kpi, kpiLookup, snapshot);
    return `<div class="kpi">
      <div class="kpi-head">
        <div class="kpi-label">${escapeHtml(kpi.label)}</div>
        <button
          type="button"
          class="kpi-info"
          aria-label="How ${escapeHtml(kpi.label || 'this KPI')} is calculated"
          aria-describedby="${tooltipId}"
        >?</button>
      </div>
      <div class="kpi-value">${escapeHtml(kpi.formatted_value || fmtNum(kpi.value, 2))}</div>
      <div class="kpi-note">vs baseline ${delta}${targetText}</div>
      <div id="${tooltipId}" class="kpi-tooltip" role="tooltip">${escapeHtml(explanation)}</div>
    </div>`;
  }).join('');
}

function buildKpiLookup(kpis) {
  const map = new Map();
  for (const kpi of kpis) {
    map.set(normalizeKpiKey(kpi?.label), Number(kpi?.value || 0));
  }
  return map;
}

function normalizeKpiKey(label) {
  return String(label || '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '');
}

function findFunnelStageValue(snapshot, stageName) {
  const stages = snapshot?.funnel_summary?.stages || [];
  const row = stages.find(stage => String(stage.stage || '').toLowerCase() === stageName.toLowerCase());
  return row ? Number(row.value || 0) : null;
}

function buildKpiExplanation(kpi, kpiLookup, snapshot) {
  const label = String(kpi?.label || '').trim();
  const key = normalizeKpiKey(label);
  const displayed = kpi?.formatted_value || fmtNum(kpi?.value || 0, 2);
  const spend = kpiLookup.get('spend') || 0;
  const revenue = kpiLookup.get('revenue') || 0;
  const conversions = kpiLookup.get('conversions') || 0;
  const lines = [`Displayed: ${displayed}`];

  if (key === 'spend') {
    lines.unshift('Formula: Spend = sum(ad cost) across selected date range.');
  } else if (key === 'revenue') {
    lines.unshift('Formula: Revenue = canonical conversion value after duplicate-event controls.');
  } else if (key === 'roas') {
    lines.unshift('Formula: ROAS = Revenue / Spend.');
    if (spend > 0) lines.push(`Derived from KPI tiles: ${fmtNum(revenue, 2)} / ${fmtNum(spend, 2)} = ${fmtNum(revenue / spend, 2)}x`);
  } else if (key === 'conversions') {
    lines.unshift('Formula: Conversions = canonical purchase count after dedupe rules.');
  } else if (key === 'ctr') {
    lines.unshift('Formula: CTR = Clicks / Impressions × 100.');
    const impressions = findFunnelStageValue(snapshot, 'Impression');
    const clicks = findFunnelStageValue(snapshot, 'Click');
    if ((impressions || 0) > 0 && (clicks || 0) >= 0) {
      lines.push(`From funnel stages: ${fmtInt(clicks)} / ${fmtInt(impressions)} = ${fmtNum((clicks / impressions) * 100, 2)}%`);
    }
  } else if (key === 'cpa') {
    lines.unshift('Formula: CPA = Spend / Conversions.');
    if (conversions > 0) lines.push(`Derived from KPI tiles: ${fmtNum(spend, 2)} / ${fmtNum(conversions, 2)} = ${fmtNum(spend / conversions, 2)}`);
  } else if (key === 'aov') {
    lines.unshift('Formula: AOV = Revenue / Conversions.');
    if (conversions > 0) lines.push(`Derived from KPI tiles: ${fmtNum(revenue, 2)} / ${fmtNum(conversions, 2)} = ${fmtNum(revenue / conversions, 2)}`);
  } else {
    lines.unshift('Formula: Derived by the analytics pipeline for this profile/date window.');
  }

  lines.push('Source: current analytics artifact and executive snapshot.');
  return lines.join('\n');
}

function renderDeltaChart(deltas) {
  const ctx = document.getElementById('deltaChart');
  if (!ctx || typeof Chart === 'undefined') return;

  const points = deltas.filter(d => typeof d.delta_percent === 'number').slice(0, 8);
  const labels = points.map(d => d.metric_key);
  const values = points.map(d => Number((d.delta_percent * 100).toFixed(2)));

  if (state.deltaChart) state.deltaChart.destroy();
  state.deltaChart = new Chart(ctx, {
    type: 'bar',
    data: {
      labels,
      datasets: [{
        label: 'Delta % vs baseline',
        data: values,
        backgroundColor: values.map(v => v >= 0 ? 'rgba(11,143,140,0.7)' : 'rgba(211,63,73,0.7)'),
        borderColor: values.map(v => v >= 0 ? 'rgba(11,143,140,1)' : 'rgba(211,63,73,1)'),
        borderWidth: 1
      }]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      scales: { y: { ticks: { callback: value => `${value}%` } } },
      plugins: { legend: { display: false } }
    }
  });
}

function renderChannelMixChart(points, roasTarget) {
  const ctx = document.getElementById('channelMixChart');
  if (!ctx || typeof Chart === 'undefined') return;

  const rows = Array.isArray(points) ? points : [];
  if (!rows.length) {
    if (state.channelMixChart) {
      state.channelMixChart.destroy();
      state.channelMixChart = null;
    }
    return;
  }
  const labels = rows.map(p => p.period_label);

  const datasets = [
    {
      label: 'Spend',
      data: rows.map(p => p.spend),
      borderColor: 'rgba(216,87,42,1)',
      backgroundColor: 'rgba(216,87,42,0.12)',
      yAxisID: 'y',
      tension: 0.3
    },
    {
      label: 'Revenue',
      data: rows.map(p => p.revenue),
      borderColor: 'rgba(11,143,140,1)',
      backgroundColor: 'rgba(11,143,140,0.12)',
      yAxisID: 'y',
      tension: 0.3
    },
    {
      label: 'ROAS',
      data: rows.map(p => p.roas),
      borderColor: 'rgba(31,42,53,1)',
      backgroundColor: 'rgba(31,42,53,0.1)',
      yAxisID: 'y1',
      tension: 0.3
    }
  ];

  if (typeof roasTarget === 'number') {
    datasets.push({
      label: 'ROAS Target',
      data: labels.map(() => roasTarget),
      borderColor: 'rgba(224,166,0,1)',
      borderDash: [6, 4],
      yAxisID: 'y1',
      pointRadius: 0,
      tension: 0
    });
  }

  if (state.channelMixChart) state.channelMixChart.destroy();
  state.channelMixChart = new Chart(ctx, {
    type: 'line',
    data: { labels, datasets },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { mode: 'index', intersect: false },
      scales: {
        y: { position: 'left', title: { display: true, text: 'Spend / Revenue' } },
        y1: { position: 'right', grid: { drawOnChartArea: false }, title: { display: true, text: 'ROAS' } }
      }
    }
  });
}

function renderQuality(quality) {
  const allChecks = [
    ...(quality.schema_drift_checks || []),
    ...(quality.identity_resolution_checks || []),
    ...(quality.freshness_sla_checks || []),
    ...(quality.cross_source_checks || []),
    ...(quality.budget_checks || [])
  ];
  if (allChecks.length === 0) {
    el.qualityList.innerHTML = '<li class="signal-item warn">No quality checks available yet.</li>';
    return;
  }
  el.qualityList.innerHTML = allChecks.slice(0, 8).map(check => {
    const applicability = String(check.applicability || 'applies');
    if (applicability === 'not_applicable') {
      return `<li class="signal-item neutral"><strong>N/A</strong> ${escapeHtml(check.code)}<br/><span>${escapeHtml(check.expected || '')}</span></li>`;
    }
    const cls = check.passed ? 'ok' : (check.severity === 'high' ? 'bad' : 'warn');
    const icon = check.passed ? 'PASS' : 'FAIL';
    return `<li class="signal-item ${cls}"><strong>${icon}</strong> ${escapeHtml(check.code)}<br/><span>${escapeHtml(check.observed || '')}</span></li>`;
  }).join('');
}

function renderDrift(historical) {
  const drift = historical.drift_flags || [];
  const anomalies = historical.anomaly_flags || [];
  if (drift.length === 0 && anomalies.length === 0) {
    el.driftList.innerHTML = '<li class="signal-item ok">No drift or anomaly flags in current baseline window.</li>';
    return;
  }
  const items = [
    ...drift.map(d => `<li class="signal-item ${d.severity === 'high' ? 'bad' : 'warn'}"><strong>Drift</strong> ${escapeHtml(d.metric_key)} z=${fmtNum(d.z_score, 2)}</li>`),
    ...anomalies.map(a => `<li class="signal-item ${a.severity === 'high' ? 'bad' : 'warn'}"><strong>Anomaly</strong> ${escapeHtml(a.metric_key)} ${escapeHtml(a.reason)}</li>`)
  ];
  el.driftList.innerHTML = items.slice(0, 10).join('');
}

function renderDataQuality(dq) {
  const rows = [
    ['Completeness', dq.completeness_ratio],
    ['Join Coverage', dq.identity_join_coverage_ratio],
    ['Freshness Pass', dq.freshness_pass_ratio],
    ['Reconciliation Pass', dq.reconciliation_pass_ratio],
    ['Cross-Source Pass', dq.cross_source_pass_ratio],
    ['Budget Pass', dq.budget_pass_ratio],
    ['Composite Score', dq.quality_score]
  ];

  el.dataQualityPanel.innerHTML = rows.map(([label, value]) => {
    const ratio = typeof value === 'number' ? value : 0;
    const pct = `${fmtNum(ratio * 100, 1)}%`;
    const cls = ratio >= 0.99 ? 'good' : ratio >= 0.95 ? 'warn' : 'bad';
    return `<div class="dq-row"><strong>${escapeHtml(label)}</strong>${pct}<span class="dq-badge ${cls}">${cls}</span></div>`;
  }).join('');
}

function renderCampaignTable(rows, sourceCoverage) {
  if (!rows.length) {
    const adsCoverage = Array.isArray(sourceCoverage)
      ? sourceCoverage.find(item => item.source_system === 'google_ads')
      : null;
    const unavailable = adsCoverage && (!adsCoverage.enabled || !adsCoverage.observed);
    const message = unavailable
      ? 'Not available for current source set (Google Ads connector disabled or no observed rows).'
      : 'No campaign rows';
    el.campaignTableBody.innerHTML = `<tr><td colspan="6">${escapeHtml(message)}</td></tr>`;
    return;
  }
  const sorted = [...rows].sort((a, b) => (b.roas || 0) - (a.roas || 0));
  el.campaignTableBody.innerHTML = sorted.slice(0, 10).map(row => `<tr>
      <td>${escapeHtml(row.campaign)}</td>
      <td>${fmtInt(row.conversions * 30)}</td>
      <td>${fmtInt(row.conversions * 5)}</td>
      <td>$${fmtNum(row.spend, 2)}</td>
      <td>${fmtNum(row.ctr, 2)}%</td>
      <td>${fmtNum(row.roas, 2)}x</td>
    </tr>`).join('');
}

function renderFunnelTable(stages) {
  if (!stages.length) {
    el.funnelTableBody.innerHTML = '<tr><td colspan="3">No funnel data</td></tr>';
    return;
  }
  el.funnelTableBody.innerHTML = stages.map(stage => {
    const conv = typeof stage.conversion_from_previous === 'number'
      ? `${fmtNum(stage.conversion_from_previous * 100, 1)}%`
      : 'n/a';
    return `<tr>
      <td>${escapeHtml(stage.stage)}</td>
      <td>${fmtInt(stage.value)}</td>
      <td>${conv}</td>
    </tr>`;
  }).join('');
}

function renderStorefrontTable(summary) {
  const rows = Array.isArray(summary?.rows) ? summary.rows : [];
  if (!rows.length) {
    const source = String(summary?.source_system || '');
    const unavailable = source.includes('not_enabled') || source.includes('no_rows') || source.includes('not_available');
    const message = unavailable
      ? 'Not available for current source set (Wix connector disabled or no observed rows).'
      : 'No storefront behavior data';
    el.storefrontTableBody.innerHTML = `<tr><td colspan="6">${escapeHtml(message)}</td></tr>`;
    return;
  }
  el.storefrontTableBody.innerHTML = rows.map(row => `<tr>
      <td>${escapeHtml(row.segment)}</td>
      <td>${escapeHtml(row.product_or_template)}</td>
      <td>${fmtInt(row.sessions)}</td>
      <td>${fmtNum(row.add_to_cart_rate * 100, 1)}%</td>
      <td>${fmtNum(row.purchase_rate * 100, 1)}%</td>
      <td>$${fmtNum(row.aov, 2)}</td>
    </tr>`).join('');
}

function renderForecast(forecast) {
  const pacing = forecast.pacing_status || 'no_target';
  const pacingClass = ['ahead', 'on_track', 'behind'].includes(pacing) ? pacing : '';
  el.forecastPanel.innerHTML = `
    <div class="forecast-card">
      <div class="forecast-label">Expected Revenue (Next Period)</div>
      <div class="forecast-value">$${fmtNum(forecast.expected_revenue_next_period || 0, 2)}</div>
      <div class="forecast-label">CI: $${fmtNum(forecast.confidence_interval_low || 0, 2)} - $${fmtNum(forecast.confidence_interval_high || 0, 2)}</div>
    </div>
    <div class="forecast-card">
      <div class="forecast-label">Expected ROAS (Next Period)</div>
      <div class="forecast-value">${fmtNum(forecast.expected_roas_next_period || 0, 2)}x</div>
      <div class="forecast-label">Target: ${forecast.target_roas == null ? 'n/a' : `${fmtNum(forecast.target_roas, 2)}x`}</div>
    </div>
    <div class="forecast-card">
      <div class="forecast-label">Month-To-Date Revenue</div>
      <div class="forecast-value">$${fmtNum(forecast.month_to_date_revenue || 0, 2)}</div>
      <div class="forecast-label">Target: ${forecast.monthly_revenue_target == null ? 'n/a' : `$${fmtNum(forecast.monthly_revenue_target, 2)}`}</div>
      <span class="pacing-chip ${pacingClass}">${escapeHtml(pacing.replace('_', ' '))}</span>
    </div>
    <div class="forecast-card">
      <div class="forecast-label">Pacing Ratio</div>
      <div class="forecast-value">${fmtNum((forecast.month_to_date_pacing_ratio || 0) * 100, 1)}%</div>
      <div class="forecast-label">vs monthly target pace</div>
    </div>
  `;
}

function renderPublishGate(gate) {
  const statusValue = gate.gate_status || 'ready';
  const blocking = gate.blocking_reasons || [];
  const warnings = gate.warning_reasons || [];
  const publishReady = gate.publish_ready !== false;
  const exportReady = gate.export_ready !== false;

  if (el.exportPacketButton) {
    el.exportPacketButton.disabled = !exportReady;
    el.exportPacketButton.title = exportReady
      ? 'Export gate is open for this snapshot.'
      : `Blocked: ${blocking.join(' | ') || 'publish/export gate failed'}`;
  }

  el.publishGatePanel.innerHTML = `
    <div class="gate-card">
      <h3>Gate Status</h3>
      <div class="gate-status ${escapeHtml(statusValue)}">${escapeHtml(statusValue.replace('_', ' '))}</div>
      <p>Publish ready: <strong>${publishReady ? 'yes' : 'no'}</strong></p>
      <p>Export ready: <strong>${exportReady ? 'yes' : 'no'}</strong></p>
    </div>
    <div class="gate-card">
      <h3>Blocking Reasons</h3>
      <p>${blocking.length ? escapeHtml(blocking.join(' | ')) : 'None'}</p>
    </div>
    <div class="gate-card">
      <h3>Warnings</h3>
      <p>${warnings.length ? escapeHtml(warnings.join(' | ')) : 'None'}</p>
    </div>
  `;
}

function renderDecisionFeed(cards) {
  if (!cards.length) {
    el.decisionFeedList.innerHTML = '<div class="decision-card low"><h3>No active decision cards</h3><p>Pipeline is stable in this window.</p></div>';
    return;
  }
  el.decisionFeedList.innerHTML = cards.slice(0, 8).map(card => `
    <div class="decision-card ${escapeHtml(card.priority || 'low')}">
      <div class="decision-meta">${escapeHtml(card.priority || 'low')} | ${escapeHtml(card.status || 'monitor')}</div>
      <h3>${escapeHtml(card.title || card.card_id || 'Decision')}</h3>
      <p>${escapeHtml(card.summary || '')}</p>
      <p><strong>Action:</strong> ${escapeHtml(card.recommended_action || 'Monitor')}</p>
    </div>
  `).join('');
}

function renderNarratives(narratives, alerts) {
  const cards = [];
  for (const item of narratives.slice(0, 3)) {
    cards.push(`<div class="narrative-item"><strong>${escapeHtml(item.kpi || 'KPI')}</strong><br/>${escapeHtml(item.narrative || '')}</div>`);
  }
  for (const alert of alerts.slice(0, 3)) {
    cards.push(`<div class="narrative-item"><strong>Alert</strong><br/>${escapeHtml(alert)}</div>`);
  }
  el.narrativeList.innerHTML = cards.length ? cards.join('') : '<div class="narrative-item">No narratives available.</div>';
}

function renderHistory(runs) {
  if (!runs.length) {
    el.historyList.innerHTML = '<div class="history-item">No persisted runs found for this profile.</div>';
    return;
  }

  el.historyList.innerHTML = runs.slice(0, 12).map(run => {
    const rid = run.metadata?.run_id || 'n/a';
    const date = run.stored_at_utc || 'unknown';
    const roas = run.artifact?.report?.total_metrics?.roas;
    return `<div class="history-item" data-run-id="${escapeHtml(rid)}">
      <strong>${escapeHtml(rid)}</strong><br/>
      <span>${escapeHtml(date)}</span><br/>
      <span>ROAS ${fmtNum(roas || 0, 2)}x</span>
    </div>`;
  }).join('');

  Array.from(el.historyList.querySelectorAll('.history-item[data-run-id]')).forEach(node => {
    node.addEventListener('click', async () => {
      const profileId = cleanText(el.profileId.value) || 'marketing_default';
      const opts = currentPhaseOptions();
      const snap = await invoke('get_executive_dashboard_snapshot', {
        profileId,
        limit: 64,
        compareWindowRuns: opts.compareWindowRuns,
        targetRoas: opts.targetRoas,
        monthlyRevenueTarget: opts.monthlyRevenueTarget
      });
      if (!snap) return;

      state.currentSnapshot = snap;
      renderExecutiveDashboard(snap);
      status(`Loaded historical context for profile ${profileId}.`);
      stampNow('Loaded history');
    });
  });
}

function isProductionLikeProfile(profileId) {
  return /prod|production/i.test(String(profileId || ''));
}

function emptySnapshot(profileId, opts, reason) {
  return {
    schema_version: 'executive_dashboard_snapshot.v1',
    profile_id: profileId,
    generated_at_utc: new Date().toISOString(),
    run_id: 'no-data',
    date_range: `${el.startDate?.value || 'n/a'} to ${el.endDate?.value || 'n/a'}`,
    compare_window_runs: opts?.compareWindowRuns || 1,
    roas_target_band: opts?.targetRoas ?? null,
    kpis: [],
    channel_mix_series: [],
    funnel_summary: { stages: [], dropoff_hotspot_stage: 'n/a' },
    storefront_behavior_summary: {
      source_system: 'wix_storefront_not_available',
      identity_confidence: 'not_available',
      rows: []
    },
    portfolio_rows: [],
    quality_controls: {
      schema_drift_checks: [],
      identity_resolution_checks: [],
      freshness_sla_checks: [],
      cross_source_checks: [],
      budget_checks: []
    },
    historical_analysis: {
      period_over_period_deltas: [],
      drift_flags: [],
      anomaly_flags: []
    },
    forecast_summary: {
      expected_revenue_next_period: 0,
      expected_roas_next_period: 0,
      confidence_interval_low: 0,
      confidence_interval_high: 0,
      month_to_date_pacing_ratio: 0,
      month_to_date_revenue: 0,
      monthly_revenue_target: opts?.monthlyRevenueTarget ?? null,
      target_roas: opts?.targetRoas ?? null,
      pacing_status: 'no_data'
    },
    decision_feed: [{
      card_id: 'no-data',
      priority: 'medium',
      status: 'review_required',
      title: 'No data available',
      summary: reason || 'No persisted analytics runs are available for this profile yet.',
      recommended_action: 'Run preflight, execute a GA4 observed read-only job, then refresh this dashboard.'
    }],
    publish_export_gate: {
      publish_ready: false,
      export_ready: false,
      blocking_reasons: [reason || 'No persisted analytics runs found.'],
      warning_reasons: [],
      gate_status: 'blocked'
    },
    data_quality: {
      completeness_ratio: 0,
      identity_join_coverage_ratio: 0,
      freshness_pass_ratio: 0,
      reconciliation_pass_ratio: 0,
      cross_source_pass_ratio: 0,
      budget_pass_ratio: 0,
      quality_score: 0
    },
    high_leverage_reports: {
      revenue_truth: {
        canonical_revenue: 0,
        canonical_conversions: 0,
        strict_duplicate_ratio: 0,
        near_duplicate_ratio: 0,
        inflation_risk: 'unknown',
        estimated_revenue_at_risk: 0,
        summary: reason || 'No revenue-truth report available before first persisted run.'
      },
      funnel_survival: {
        points: [],
        bottleneck_stage: 'none'
      },
      attribution_delta: {
        rows: [],
        dominant_last_touch_campaign: null,
        last_touch_concentration_hhi: 0,
        summary: 'No attribution rows available before first persisted run.'
      },
      data_quality_scorecard: {
        quality_score: 0,
        completeness_ratio: 0,
        freshness_pass_ratio: 0,
        reconciliation_pass_ratio: 0,
        cross_source_pass_ratio: 0,
        budget_pass_ratio: 0,
        high_severity_failures: 0,
        blocking_reasons_count: 1,
        warning_reasons_count: 0,
        gate_status: 'blocked'
      }
    },
    operator_summary: {
      attribution_narratives: []
    },
    alerts: [reason || 'No persisted analytics runs found.'],
    source_coverage: []
  };
}

function fallbackSnapshot(profileId, opts) {
  const targetRoas = opts?.targetRoas ?? 6.0;
  const monthlyRevenueTarget = opts?.monthlyRevenueTarget ?? 3000;
  return {
    schema_version: 'executive_dashboard_snapshot.v1',
    profile_id: profileId,
    generated_at_utc: new Date().toISOString(),
    run_id: 'demo-run',
    date_range: '2026-02-01 to 2026-02-07',
    compare_window_runs: opts?.compareWindowRuns || 1,
    roas_target_band: targetRoas,
    kpis: [
      { label: 'Spend', value: 350, formatted_value: '$350.00', delta_percent: -0.03, confidence_label: 'medium' },
      { label: 'Revenue', value: 2200, formatted_value: '$2200.00', delta_percent: 0.12, confidence_label: 'medium' },
      { label: 'ROAS', value: 6.29, formatted_value: '6.29x', delta_percent: 0.09, target_delta_percent: (6.29 - targetRoas) / targetRoas, confidence_label: 'medium' },
      { label: 'Conversions', value: 34, formatted_value: '34.00', delta_percent: 0.08, confidence_label: 'medium' },
      { label: 'CTR', value: 8.5, formatted_value: '8.50%', delta_percent: 0.04, confidence_label: 'medium' },
      { label: 'CPA', value: 10.29, formatted_value: '$10.29', delta_percent: -0.07, confidence_label: 'medium' },
      { label: 'AOV', value: 64.7, formatted_value: '$64.70', delta_percent: 0.03, confidence_label: 'medium' }
    ],
    channel_mix_series: [
      { period_label: '2026-01-18 -> 2026-01-24', spend: 300, revenue: 1700, roas: 5.67 },
      { period_label: '2026-01-25 -> 2026-01-31', spend: 340, revenue: 2000, roas: 5.88 },
      { period_label: '2026-02-01 -> 2026-02-07', spend: 350, revenue: 2200, roas: 6.29 }
    ],
    funnel_summary: {
      stages: [
        { stage: 'Impression', value: 8000 },
        { stage: 'Click', value: 680, conversion_from_previous: 0.085 },
        { stage: 'Session', value: 620, conversion_from_previous: 0.912 },
        { stage: 'Product View', value: 415, conversion_from_previous: 0.669 },
        { stage: 'Add To Cart', value: 118, conversion_from_previous: 0.284 },
        { stage: 'Checkout', value: 67, conversion_from_previous: 0.568 },
        { stage: 'Purchase', value: 34, conversion_from_previous: 0.507 }
      ]
    },
    storefront_behavior_summary: {
      rows: [
        { segment: 'mobile', product_or_template: 'ready-raw-hero-landing', sessions: 360, add_to_cart_rate: 0.2, purchase_rate: 0.065, aov: 61.2 },
        { segment: 'desktop', product_or_template: 'value-bundle-collection', sessions: 260, add_to_cart_rate: 0.17, purchase_rate: 0.072, aov: 68.1 }
      ]
    },
    portfolio_rows: [
      { campaign: 'New Puppy Essentials', spend: 210, revenue: 1550, roas: 7.38, ctr: 8.04, cpa: 10.0, conversions: 21 },
      { campaign: 'Summer Pet Food Promo', spend: 140, revenue: 650, roas: 4.64, ctr: 9.31, cpa: 10.77, conversions: 13 }
    ],
    quality_controls: {
      schema_drift_checks: [{ code: 'schema_campaign_required_fields', passed: true, severity: 'high', observed: 'stable fields' }],
      identity_resolution_checks: [{ code: 'identity_keyword_linked_to_ad_group', passed: true, severity: 'high', observed: 'join coverage good' }],
      freshness_sla_checks: [{ code: 'freshness_sla_mock', passed: true, severity: 'medium', observed: '0m freshness' }],
      cross_source_checks: [{ code: 'cross_source_attributed_revenue_within_wix_gross', passed: true, severity: 'high', observed: 'revenue aligned' }],
      budget_checks: [{ code: 'budget_no_blocked_spend', passed: true, severity: 'high', observed: 'blocked_events=0' }]
    },
    historical_analysis: {
      period_over_period_deltas: [
        { metric_key: 'roas', delta_percent: 0.09 },
        { metric_key: 'ctr', delta_percent: 0.04 },
        { metric_key: 'cost', delta_percent: -0.03 }
      ],
      drift_flags: [],
      anomaly_flags: []
    },
    forecast_summary: {
      expected_revenue_next_period: 2400,
      expected_roas_next_period: 6.4,
      confidence_interval_low: 2160,
      confidence_interval_high: 2640,
      month_to_date_pacing_ratio: 2200 / monthlyRevenueTarget,
      month_to_date_revenue: 2200,
      monthly_revenue_target: monthlyRevenueTarget,
      target_roas: targetRoas,
      pacing_status: (2200 / monthlyRevenueTarget) >= 0.9 ? 'on_track' : 'behind'
    },
    decision_feed: [
      {
        card_id: 'demo-review',
        priority: 'medium',
        status: 'investigate',
        title: 'ROAS variance near threshold',
        summary: 'ROAS is above target, but one campaign has widening CPA variance.',
        recommended_action: 'Review campaign budget split before weekly publish.'
      }
    ],
    publish_export_gate: {
      publish_ready: true,
      export_ready: true,
      blocking_reasons: [],
      warning_reasons: ['One medium anomaly requires review note in packet.'],
      gate_status: 'review_required'
    },
    data_quality: {
      completeness_ratio: 1.0,
      identity_join_coverage_ratio: 0.99,
      freshness_pass_ratio: 0.96,
      reconciliation_pass_ratio: 1.0,
      cross_source_pass_ratio: 1.0,
      budget_pass_ratio: 1.0,
      quality_score: 0.988
    },
    high_leverage_reports: {
      revenue_truth: {
        canonical_revenue: 2200,
        canonical_conversions: 34,
        strict_duplicate_ratio: 0.011,
        near_duplicate_ratio: 0.024,
        inflation_risk: 'low',
        estimated_revenue_at_risk: 52.8,
        summary: 'Canonical purchase metrics applied with low duplicate inflation risk.'
      },
      funnel_survival: {
        points: [
          { stage: 'Impression', entrants: 8000, survival_rate: 1.0, hazard_rate: 0.0 },
          { stage: 'Click', entrants: 680, survival_rate: 0.085, hazard_rate: 0.915 },
          { stage: 'Session', entrants: 620, survival_rate: 0.0775, hazard_rate: 0.088 },
          { stage: 'Product View', entrants: 415, survival_rate: 0.0518, hazard_rate: 0.331 },
          { stage: 'Add To Cart', entrants: 118, survival_rate: 0.0147, hazard_rate: 0.716 },
          { stage: 'Checkout', entrants: 67, survival_rate: 0.0084, hazard_rate: 0.432 },
          { stage: 'Purchase', entrants: 34, survival_rate: 0.0042, hazard_rate: 0.493 }
        ],
        bottleneck_stage: 'Add To Cart'
      },
      attribution_delta: {
        rows: [
          { campaign: 'New Puppy Essentials', first_touch_proxy_share: 0.61, assist_share: 0.58, last_touch_share: 0.7, delta_first_vs_last: -0.09 },
          { campaign: 'Summer Pet Food Promo', first_touch_proxy_share: 0.39, assist_share: 0.42, last_touch_share: 0.3, delta_first_vs_last: 0.09 }
        ],
        dominant_last_touch_campaign: 'New Puppy Essentials',
        last_touch_concentration_hhi: 0.58,
        summary: 'Last-touch value is concentrated in one campaign; validate assist credit before reallocating budget.'
      },
      data_quality_scorecard: {
        quality_score: 0.988,
        completeness_ratio: 1.0,
        freshness_pass_ratio: 0.96,
        reconciliation_pass_ratio: 1.0,
        cross_source_pass_ratio: 1.0,
        budget_pass_ratio: 1.0,
        high_severity_failures: 0,
        blocking_reasons_count: 0,
        warning_reasons_count: 1,
        gate_status: 'review_required'
      }
    },
    operator_summary: {
      attribution_narratives: [
        { kpi: 'roas', narrative: 'ROAS remains strongest in New Puppy Essentials with clean quality signals.' }
      ]
    },
    alerts: []
  };
}

function cleanText(value) {
  const v = String(value || '').trim();
  return v.length ? v : null;
}

function formatDelta(value) {
  if (typeof value !== 'number') return 'n/a';
  const pct = value * 100;
  return `${pct >= 0 ? '+' : ''}${fmtNum(pct, 1)}%`;
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

function fmtNum(v, decimals = 2) {
  const n = Number(v);
  if (!Number.isFinite(n)) return '0.00';
  return n.toFixed(decimals);
}

function fmtInt(v) {
  const n = Number(v);
  if (!Number.isFinite(n)) return '0';
  return Math.round(n).toLocaleString();
}

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function shortLabel(text, maxLength = 20) {
  const raw = String(text || '');
  if (raw.length <= maxLength) return raw;
  return `${raw.slice(0, maxLength - 1)}...`;
}
