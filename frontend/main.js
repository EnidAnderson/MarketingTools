const state = {
  currentSnapshot: null,
  deltaChart: null,
  channelMixChart: null,
  historyRuns: []
};

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
  historyList: document.getElementById('historyList')
};

boot();

async function boot() {
  wireEvents();
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
  el.jobStatus.textContent = text;
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

async function generateRunAndRefresh() {
  status('Submitting analytics run...');
  const request = {
    start_date: el.startDate.value,
    end_date: el.endDate.value,
    campaign_filter: cleanText(el.campaignFilter.value),
    ad_group_filter: cleanText(el.adGroupFilter.value),
    seed: parseOptionalInt(el.seed.value),
    profile_id: cleanText(el.profileId.value) || 'marketing_default',
    include_narratives: el.includeNarratives.checked
  };

  try {
    const handle = await invoke('start_mock_analytics_job', { request });
    status(`Job ${handle.job_id} started...`);

    while (true) {
      const snapshot = await invoke('get_tool_job', { jobId: handle.job_id });
      status(`${snapshot.progress_pct}% - ${snapshot.stage} (${snapshot.message || 'running'})`);

      if (snapshot.status === 'succeeded') break;
      if (snapshot.status === 'failed' || snapshot.status === 'canceled') {
        status(`${snapshot.status.toUpperCase()}: ${snapshot.message || 'execution failed'}`);
        return;
      }
      await sleep(350);
    }

    await refreshDashboard();
    status('Run completed and dashboard refreshed.');
    stampNow('Run complete');
  } catch (err) {
    status(`Run failed: ${String(err)}`);
  }
}

async function refreshDashboard() {
  const profileId = cleanText(el.profileId.value) || 'marketing_default';
  const opts = currentPhaseOptions();

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
    status('Loaded executive snapshot.');
    stampNow('Loaded');
  } catch (err) {
    const message = String(err || 'Unknown error');
    if (message.includes('No persisted analytics runs found')) {
      renderExecutiveDashboard(fallbackSnapshot(profileId, opts));
      status('No persisted runs yet. Showing demo snapshot.');
      stampNow('Demo');
      return;
    }
    status(`Refresh failed: ${message}`);
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
  renderKpis(snapshot.kpis || []);
  renderDeltaChart(snapshot.historical_analysis?.period_over_period_deltas || []);
  renderChannelMixChart(snapshot.channel_mix_series || [], snapshot.roas_target_band);
  renderQuality(snapshot.quality_controls || {});
  renderDataQuality(snapshot.data_quality || {});
  renderDrift(snapshot.historical_analysis || {});
  renderCampaignTable(snapshot.portfolio_rows || []);
  renderFunnelTable(snapshot.funnel_summary?.stages || []);
  renderStorefrontTable(snapshot.storefront_behavior_summary?.rows || []);
  renderForecast(snapshot.forecast_summary || {});
  renderPublishGate(snapshot.publish_export_gate || {});
  renderDecisionFeed(snapshot.decision_feed || []);
  renderNarratives(snapshot.operator_summary?.attribution_narratives || [], snapshot.alerts || []);
}

function renderKpis(kpis) {
  const cards = kpis.length ? kpis : fallbackSnapshot('demo', currentPhaseOptions()).kpis;
  el.kpiGrid.innerHTML = cards.map(kpi => {
    const delta = formatDelta(kpi.delta_percent);
    const targetDelta = formatDelta(kpi.target_delta_percent);
    const targetText = kpi.target_delta_percent == null ? '' : ` | vs target ${targetDelta}`;
    return `<div class="kpi">
      <div class="kpi-label">${escapeHtml(kpi.label)}</div>
      <div class="kpi-value">${escapeHtml(kpi.formatted_value || fmtNum(kpi.value, 2))}</div>
      <div class="kpi-note">vs baseline ${delta}${targetText}</div>
    </div>`;
  }).join('');
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

  const rows = points.length ? points : fallbackSnapshot('demo', currentPhaseOptions()).channel_mix_series;
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
    ...(quality.freshness_sla_checks || [])
  ];
  if (allChecks.length === 0) {
    el.qualityList.innerHTML = '<li class="signal-item warn">No quality checks available yet.</li>';
    return;
  }
  el.qualityList.innerHTML = allChecks.slice(0, 8).map(check => {
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
    ['Composite Score', dq.quality_score]
  ];

  el.dataQualityPanel.innerHTML = rows.map(([label, value]) => {
    const ratio = typeof value === 'number' ? value : 0;
    const pct = `${fmtNum(ratio * 100, 1)}%`;
    const cls = ratio >= 0.99 ? 'good' : ratio >= 0.95 ? 'warn' : 'bad';
    return `<div class="dq-row"><strong>${escapeHtml(label)}</strong>${pct}<span class="dq-badge ${cls}">${cls}</span></div>`;
  }).join('');
}

function renderCampaignTable(rows) {
  if (!rows.length) {
    el.campaignTableBody.innerHTML = '<tr><td colspan="6">No campaign rows</td></tr>';
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

function renderStorefrontTable(rows) {
  if (!rows.length) {
    el.storefrontTableBody.innerHTML = '<tr><td colspan="6">No storefront behavior data</td></tr>';
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
      freshness_sla_checks: [{ code: 'freshness_sla_mock', passed: true, severity: 'medium', observed: '0m freshness' }]
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
      quality_score: 0.988
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
