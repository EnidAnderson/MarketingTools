const state = {
  currentArtifact: null,
  chart: null,
  historyRuns: []
};

const el = {
  profileId: document.getElementById('profileId'),
  startDate: document.getElementById('startDate'),
  endDate: document.getElementById('endDate'),
  campaignFilter: document.getElementById('campaignFilter'),
  adGroupFilter: document.getElementById('adGroupFilter'),
  seed: document.getElementById('seed'),
  includeNarratives: document.getElementById('includeNarratives'),
  runButton: document.getElementById('runButton'),
  refreshButton: document.getElementById('refreshButton'),
  refreshStamp: document.getElementById('refreshStamp'),
  jobStatus: document.getElementById('jobStatus'),
  kpiGrid: document.getElementById('kpiGrid'),
  runIdBadge: document.getElementById('runIdBadge'),
  qualityList: document.getElementById('qualityList'),
  driftList: document.getElementById('driftList'),
  campaignTableBody: document.getElementById('campaignTableBody'),
  narrativeList: document.getElementById('narrativeList'),
  historyList: document.getElementById('historyList')
};

boot();

async function boot() {
  wireEvents();
  await refreshHistoryAndRenderLatest();
  setInterval(() => {
    refreshHistoryOnly().catch(() => {
      /* ignore background polling errors */
    });
  }, 45000);
}

function wireEvents() {
  el.runButton.addEventListener('click', () => generateRun());
  el.refreshButton.addEventListener('click', () => refreshHistoryAndRenderLatest());
}

async function invoke(command, payload = {}) {
  const tauriInvoke = window.__TAURI__?.core?.invoke;
  if (!tauriInvoke) {
    throw new Error('Tauri runtime not available. Launch through the Tauri app to fetch live data.');
  }
  return tauriInvoke(command, payload);
}

function status(text) {
  el.jobStatus.textContent = text;
}

function stampNow(prefix = 'Updated') {
  const now = new Date();
  el.refreshStamp.textContent = `${prefix}: ${now.toLocaleString()}`;
}

function parseOptionalInt(value) {
  if (!value || !String(value).trim()) {
    return null;
  }
  const n = Number(value);
  return Number.isFinite(n) ? Math.trunc(n) : null;
}

async function generateRun() {
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
    status(`Job ${handle.job_id} started. Waiting for completion...`);

    let terminal = null;
    while (!terminal) {
      const snapshot = await invoke('get_tool_job', { jobId: handle.job_id });
      status(`${snapshot.progress_pct}% - ${snapshot.stage} (${snapshot.message || 'running'})`);
      if (['succeeded', 'failed', 'canceled'].includes(snapshot.status)) {
        terminal = snapshot;
        break;
      }
      await sleep(320);
    }

    if (terminal.status !== 'succeeded') {
      status(`Run ${terminal.status}: ${terminal.message || 'error'}`);
      return;
    }

    if (!terminal.output) {
      status('Run succeeded but no output artifact was returned.');
      return;
    }

    state.currentArtifact = terminal.output;
    renderDashboard(state.currentArtifact);
    await refreshHistoryOnly();
    stampNow('Run complete');
    status('Dashboard updated from latest run.');
  } catch (err) {
    status(`Run failed: ${String(err)}`);
  }
}

async function refreshHistoryAndRenderLatest() {
  await refreshHistoryOnly();
  if (state.historyRuns.length > 0) {
    const latest = state.historyRuns[0]?.artifact;
    if (latest) {
      state.currentArtifact = latest;
      renderDashboard(latest);
      stampNow('Loaded');
      status('Loaded latest persisted run.');
      return;
    }
  }
  renderDashboard(fallbackArtifact());
  stampNow('Loaded demo');
  status('No persisted run found yet. Showing local demo dashboard.');
}

async function refreshHistoryOnly() {
  const profile = cleanText(el.profileId.value);
  const history = await invoke('get_mock_analytics_run_history', {
    profileId: profile || null,
    limit: 24
  });
  state.historyRuns = Array.isArray(history) ? history : [];
  renderHistory(state.historyRuns);
}

function renderDashboard(artifact) {
  if (!artifact || !artifact.report || !artifact.report.total_metrics) {
    return;
  }

  const m = artifact.report.total_metrics;
  const hist = artifact.historical_analysis || {};

  el.runIdBadge.textContent = `Run: ${artifact.metadata?.run_id || 'n/a'}`;

  const cards = [
    { label: 'Impressions', value: fmtInt(m.impressions), note: 'Reach volume' },
    { label: 'Clicks', value: fmtInt(m.clicks), note: 'Traffic generated' },
    { label: 'CTR', value: `${fmtNum(m.ctr, 2)}%`, note: 'Click-through rate' },
    { label: 'Spend', value: `$${fmtNum(m.cost, 2)}`, note: 'Total media cost' },
    { label: 'Conversions', value: fmtNum(m.conversions, 2), note: 'Attributed actions' },
    { label: 'ROAS', value: `${fmtNum(m.roas, 2)}x`, note: 'Return on ad spend' },
    { label: 'Quality Health', value: artifact.quality_controls?.is_healthy ? 'Healthy' : 'Attention', note: 'Schema/identity/freshness gates' },
    { label: 'Anomaly Flags', value: String((hist.anomaly_flags || []).length), note: 'Longitudinal watchlist' }
  ];
  el.kpiGrid.innerHTML = cards.map(card => `
    <div class="kpi">
      <div class="kpi-label">${escapeHtml(card.label)}</div>
      <div class="kpi-value">${escapeHtml(card.value)}</div>
      <div class="kpi-note">${escapeHtml(card.note)}</div>
    </div>
  `).join('');

  renderDeltaChart(hist.period_over_period_deltas || []);
  renderQuality(artifact.quality_controls || {});
  renderDrift(hist);
  renderCampaignTable(artifact.report.campaign_data || []);
  renderNarratives(artifact);
}

function renderDeltaChart(deltas) {
  const ctx = document.getElementById('deltaChart');
  if (!ctx || typeof Chart === 'undefined') {
    return;
  }

  const top = deltas
    .filter(d => typeof d.delta_percent === 'number')
    .sort((a, b) => Math.abs(b.delta_percent) - Math.abs(a.delta_percent))
    .slice(0, 6);

  const labels = top.map(d => d.metric_key);
  const values = top.map(d => Number((d.delta_percent * 100).toFixed(2)));

  if (state.chart) {
    state.chart.destroy();
  }

  state.chart = new Chart(ctx, {
    type: 'bar',
    data: {
      labels,
      datasets: [{
        label: 'Delta % vs baseline',
        data: values,
        backgroundColor: values.map(v => (v >= 0 ? 'rgba(11,143,140,0.7)' : 'rgba(211,63,73,0.7)')),
        borderColor: values.map(v => (v >= 0 ? 'rgba(11,143,140,1)' : 'rgba(211,63,73,1)')),
        borderWidth: 1
      }]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      scales: {
        y: {
          ticks: {
            callback: (v) => `${v}%`
          }
        }
      },
      plugins: {
        legend: { display: false }
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
    return `<li class="signal-item ${cls}"><strong>${icon}</strong> ${escapeHtml(check.code)}<br/><span>${escapeHtml(check.observed || check.message || '')}</span></li>`;
  }).join('');
}

function renderDrift(hist) {
  const drift = hist.drift_flags || [];
  const anomalies = hist.anomaly_flags || [];

  if (drift.length === 0 && anomalies.length === 0) {
    el.driftList.innerHTML = '<li class="signal-item ok">No drift or anomaly flags in current baseline window.</li>';
    return;
  }

  const driftItems = drift.map(flag => {
    const sev = flag.severity || 'medium';
    return `<li class="signal-item ${sev === 'high' ? 'bad' : 'warn'}"><strong>Drift</strong> ${escapeHtml(flag.metric_key)} z=${fmtNum(flag.z_score, 2)} (${escapeHtml(sev)})</li>`;
  });
  const anomalyItems = anomalies.map(flag => {
    const sev = flag.severity || 'medium';
    return `<li class="signal-item ${sev === 'high' ? 'bad' : 'warn'}"><strong>Anomaly</strong> ${escapeHtml(flag.metric_key)} - ${escapeHtml(flag.reason)}</li>`;
  });

  el.driftList.innerHTML = [...driftItems, ...anomalyItems].slice(0, 8).join('');
}

function renderCampaignTable(campaignRows) {
  if (!campaignRows.length) {
    el.campaignTableBody.innerHTML = '<tr><td colspan="6">No campaign rows</td></tr>';
    return;
  }

  const sorted = [...campaignRows].sort((a, b) => (b.metrics?.roas || 0) - (a.metrics?.roas || 0));
  el.campaignTableBody.innerHTML = sorted.slice(0, 10).map(row => {
    const m = row.metrics || {};
    return `<tr>
      <td>${escapeHtml(row.campaign_name || row.campaign_id || 'Unknown')}</td>
      <td>${fmtInt(m.impressions || 0)}</td>
      <td>${fmtInt(m.clicks || 0)}</td>
      <td>$${fmtNum(m.cost || 0, 2)}</td>
      <td>${fmtNum(m.ctr || 0, 2)}%</td>
      <td>${fmtNum(m.roas || 0, 2)}x</td>
    </tr>`;
  }).join('');
}

function renderNarratives(artifact) {
  const operator = artifact.operator_summary?.attribution_narratives || [];
  const guidance = artifact.inferred_guidance || [];

  const cards = [];
  for (const item of operator.slice(0, 3)) {
    cards.push(`<div class="narrative-item"><strong>${escapeHtml(item.kpi || 'KPI')}</strong><br/>${escapeHtml(item.narrative || '')}</div>`);
  }
  for (const item of guidance.slice(0, 3)) {
    cards.push(`<div class="narrative-item"><strong>Guidance (${escapeHtml(item.confidence_label || 'n/a')})</strong><br/>${escapeHtml(item.text || '')}</div>`);
  }

  el.narrativeList.innerHTML = cards.length ? cards.join('') : '<div class="narrative-item">No operator narratives yet.</div>';
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
    node.addEventListener('click', () => {
      const runId = node.getAttribute('data-run-id');
      const run = runs.find(item => item.metadata?.run_id === runId);
      if (!run?.artifact) {
        return;
      }
      state.currentArtifact = run.artifact;
      renderDashboard(run.artifact);
      status(`Loaded historical run ${runId}`);
      stampNow('Loaded historical run');
    });
  });
}

function fallbackArtifact() {
  return {
    metadata: { run_id: 'demo-artifact' },
    report: {
      total_metrics: {
        impressions: 8000,
        clicks: 680,
        ctr: 8.5,
        cost: 350,
        conversions: 34,
        roas: 6.29
      },
      campaign_data: [
        { campaign_name: 'New Puppy Essentials', metrics: { impressions: 5100, clicks: 410, cost: 210, ctr: 8.04, roas: 7.38 } },
        { campaign_name: 'Summer Pet Food Promo', metrics: { impressions: 2900, clicks: 270, cost: 140, ctr: 9.31, roas: 4.64 } }
      ]
    },
    quality_controls: {
      is_healthy: true,
      schema_drift_checks: [{ code: 'schema_campaign_required_fields', passed: true, observed: 'stable schema' }],
      identity_resolution_checks: [{ code: 'identity_keyword_linked_to_ad_group', passed: true, observed: 'no orphan entities' }],
      freshness_sla_checks: [{ code: 'freshness_sla_mock', passed: true, observed: '0m freshness' }]
    },
    historical_analysis: {
      period_over_period_deltas: [
        { metric_key: 'roas', delta_percent: 0.19 },
        { metric_key: 'ctr', delta_percent: 0.1 },
        { metric_key: 'cost', delta_percent: -0.03 },
        { metric_key: 'clicks', delta_percent: 0.08 }
      ],
      drift_flags: [],
      anomaly_flags: []
    },
    operator_summary: {
      attribution_narratives: [
        { kpi: 'roas', narrative: 'ROAS is concentrated in New Puppy Essentials; scaling opportunity remains.', confidence_label: 'medium' }
      ]
    },
    inferred_guidance: [
      { text: 'Increase budget share toward higher ROAS campaign cluster.', confidence_label: 'medium' }
    ]
  };
}

function cleanText(value) {
  const v = String(value || '').trim();
  return v.length ? v : null;
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

function fmtNum(v, decimals = 2) {
  const n = Number(v);
  if (!Number.isFinite(n)) {
    return '0.00';
  }
  return n.toFixed(decimals);
}

function fmtInt(v) {
  const n = Number(v);
  if (!Number.isFinite(n)) {
    return '0';
  }
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
