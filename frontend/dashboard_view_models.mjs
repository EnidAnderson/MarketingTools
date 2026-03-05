function cleanText(value) {
  return String(value == null ? '' : value).trim();
}

function fmtNum(value, decimals = 2) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '0.00';
  return numeric.toLocaleString('en-US', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals
  });
}

function fmtInt(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '0';
  return Math.round(numeric).toLocaleString('en-US');
}

function formatDelta(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '0.0%';
  const sign = numeric > 0 ? '+' : '';
  return `${sign}${fmtNum(numeric * 100, 1)}%`;
}

function riskTone(risk) {
  if (risk === 'high') return 'bad';
  if (risk === 'medium') return 'warn';
  if (risk === 'low') return 'good';
  return 'neutral';
}

function guardTone(status) {
  if (status === 'guarded_review_required') return 'warn';
  if (status === 'guarded_clean' || status === 'canonical_only') return 'good';
  return 'neutral';
}

function guardLabel(status) {
  if (status === 'guarded_review_required') return 'guard: review required';
  if (status === 'guarded_clean') return 'guard: stable';
  if (status === 'canonical_only') return 'guard: canonical only';
  return 'guard: unknown';
}

export function buildRevenueTruthViewModel(report = {}) {
  const risk = cleanText(report.inflation_risk).toLowerCase() || 'unknown';
  const truthGuardStatus = cleanText(report.truth_guard_status).toLowerCase() || 'unknown';
  const summary =
    cleanText(report.summary) || 'No revenue-truth summary available for this run.';
  const metrics = [
    {
      key: 'canonical_revenue',
      label: 'Canonical Revenue',
      rawValue: Number(report.canonical_revenue || 0),
      displayValue: `$${fmtNum(report.canonical_revenue || 0, 2)}`
    },
    {
      key: 'canonical_conversions',
      label: 'Canonical Conversions',
      rawValue: Number(report.canonical_conversions || 0),
      displayValue: fmtInt(report.canonical_conversions || 0)
    },
    {
      key: 'strict_duplicate_ratio',
      label: 'Strict Duplicate Ratio',
      rawValue: Number(report.strict_duplicate_ratio || 0),
      displayValue: `${fmtNum((report.strict_duplicate_ratio || 0) * 100, 2)}%`
    },
    {
      key: 'near_duplicate_ratio',
      label: 'Near Duplicate Ratio',
      rawValue: Number(report.near_duplicate_ratio || 0),
      displayValue: `${fmtNum((report.near_duplicate_ratio || 0) * 100, 2)}%`
    },
    {
      key: 'estimated_revenue_at_risk',
      label: 'Estimated Revenue At Risk',
      rawValue: Number(report.estimated_revenue_at_risk || 0),
      displayValue: `$${fmtNum(report.estimated_revenue_at_risk || 0, 2)}`
    },
    {
      key: 'custom_purchase_rows',
      label: 'Custom Purchase Rows',
      rawValue: Number(report.custom_purchase_rows || 0),
      displayValue: fmtInt(report.custom_purchase_rows || 0)
    },
    {
      key: 'custom_purchase_overlap_rows',
      label: 'Canonical Match Rows',
      rawValue: Number(report.custom_purchase_overlap_rows || 0),
      displayValue: fmtInt(report.custom_purchase_overlap_rows || 0)
    },
    {
      key: 'custom_purchase_orphan_rows',
      label: 'Orphan Rows',
      rawValue: Number(report.custom_purchase_orphan_rows || 0),
      displayValue: fmtInt(report.custom_purchase_orphan_rows || 0)
    },
    {
      key: 'custom_purchase_overlap_ratio',
      label: 'Custom Overlap Ratio',
      rawValue: Number(report.custom_purchase_overlap_ratio || 0),
      displayValue: `${fmtNum((report.custom_purchase_overlap_ratio || 0) * 100, 2)}%`
    },
    {
      key: 'custom_purchase_orphan_ratio',
      label: 'Custom Orphan Ratio',
      rawValue: Number(report.custom_purchase_orphan_ratio || 0),
      displayValue: `${fmtNum((report.custom_purchase_orphan_ratio || 0) * 100, 2)}%`
    }
  ];

  return {
    panelId: 'revenue-truth',
    risk,
    riskLabel: `risk: ${risk}`,
    riskTone: riskTone(risk),
    guardStatus: truthGuardStatus,
    guardLabel: guardLabel(truthGuardStatus),
    guardTone: guardTone(truthGuardStatus),
    summary,
    metrics,
    diagnostics: {
      risk,
      guardStatus: truthGuardStatus,
      summary,
      metricCount: metrics.length,
      customPurchaseRows: Number(report.custom_purchase_rows || 0),
      customPurchaseOrphanRows: Number(report.custom_purchase_orphan_rows || 0),
      customPurchaseOverlapRatio: Number(report.custom_purchase_overlap_ratio || 0),
      customPurchaseOrphanRatio: Number(report.custom_purchase_orphan_ratio || 0)
    }
  };
}

export function buildPublishGateViewModel(gate = {}) {
  const gateStatus = cleanText(gate.gate_status).toLowerCase() || 'ready';
  const blockingReasons = Array.isArray(gate.blocking_reasons)
    ? gate.blocking_reasons.map(cleanText).filter(Boolean)
    : [];
  const warningReasons = Array.isArray(gate.warning_reasons)
    ? gate.warning_reasons.map(cleanText).filter(Boolean)
    : [];
  const publishReady = gate.publish_ready !== false;
  const exportReady = gate.export_ready !== false;

  return {
    panelId: 'publish-gate',
    gateStatus,
    publishReady,
    exportReady,
    blockingReasons,
    warningReasons,
    sections: [
      {
        key: 'status',
        title: 'Gate Status',
        bodyLines: [
          `Publish ready: ${publishReady ? 'yes' : 'no'}`,
          `Export ready: ${exportReady ? 'yes' : 'no'}`
        ]
      },
      {
        key: 'blocking_reasons',
        title: 'Blocking Reasons',
        bodyLines: [blockingReasons.length ? blockingReasons.join(' | ') : 'None']
      },
      {
        key: 'warning_reasons',
        title: 'Warnings',
        bodyLines: [warningReasons.length ? warningReasons.join(' | ') : 'None']
      }
    ],
    diagnostics: {
      gateStatus,
      publishReady,
      exportReady,
      blockingCount: blockingReasons.length,
      warningCount: warningReasons.length
    }
  };
}

export function buildDecisionFeedViewModel(cards = []) {
  const normalizedCards = Array.isArray(cards)
    ? cards
        .slice(0, 8)
        .map((card, index) => ({
          cardId: cleanText(card.card_id) || `decision-${index + 1}`,
          priority: cleanText(card.priority).toLowerCase() || 'low',
          status: cleanText(card.status).toLowerCase() || 'monitor',
          title: cleanText(card.title) || cleanText(card.card_id) || 'Decision',
          summary: cleanText(card.summary),
          recommendedAction: cleanText(card.recommended_action) || 'Monitor'
        }))
    : [];

  const cardsOrFallback = normalizedCards.length
    ? normalizedCards
    : [
        {
          cardId: 'no-active-cards',
          priority: 'low',
          status: 'monitor',
          title: 'No active decision cards',
          summary: 'Pipeline is stable in this window.',
          recommendedAction: 'Monitor'
        }
      ];

  return {
    panelId: 'decision-feed',
    cards: cardsOrFallback,
    diagnostics: {
      cardCount: normalizedCards.length,
      visibleCardCount: cardsOrFallback.length,
      cardIds: cardsOrFallback.map((card) => card.cardId),
      priorities: cardsOrFallback.map((card) => card.priority),
      statuses: cardsOrFallback.map((card) => card.status)
    }
  };
}

export function normalizeKpiKey(label) {
  return String(label || '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '');
}

export function buildKpiLookup(kpis) {
  const map = new Map();
  for (const kpi of Array.isArray(kpis) ? kpis : []) {
    map.set(normalizeKpiKey(kpi?.label), Number(kpi?.value || 0));
  }
  return map;
}

export function findFunnelStageValue(snapshot, stageName) {
  const stages = snapshot?.funnel_summary?.stages || [];
  const row = stages.find(
    (stage) => String(stage.stage || '').toLowerCase() === String(stageName || '').toLowerCase()
  );
  return row ? Number(row.value || 0) : null;
}

export function buildKpiExplanation(kpi, kpiLookup, snapshot) {
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
    if (spend > 0) {
      lines.push(
        `Derived from KPI tiles: ${fmtNum(revenue, 2)} / ${fmtNum(spend, 2)} = ${fmtNum(
          revenue / spend,
          2
        )}x`
      );
    }
  } else if (key === 'conversions') {
    lines.unshift('Formula: Conversions = canonical purchase count after dedupe rules.');
  } else if (key === 'ctr') {
    lines.unshift('Formula: CTR = Clicks / Impressions × 100.');
    const impressions = findFunnelStageValue(snapshot, 'Impression');
    const clicks = findFunnelStageValue(snapshot, 'Click');
    if ((impressions || 0) > 0 && (clicks || 0) >= 0) {
      lines.push(
        `From funnel stages: ${fmtInt(clicks)} / ${fmtInt(impressions)} = ${fmtNum(
          (clicks / impressions) * 100,
          2
        )}%`
      );
    }
  } else if (key === 'cpa') {
    lines.unshift('Formula: CPA = Spend / Conversions.');
    if (conversions > 0) {
      lines.push(
        `Derived from KPI tiles: ${fmtNum(spend, 2)} / ${fmtNum(conversions, 2)} = ${fmtNum(
          spend / conversions,
          2
        )}`
      );
    }
  } else if (key === 'aov') {
    lines.unshift('Formula: AOV = Revenue / Conversions.');
    if (conversions > 0) {
      lines.push(
        `Derived from KPI tiles: ${fmtNum(revenue, 2)} / ${fmtNum(conversions, 2)} = ${fmtNum(
          revenue / conversions,
          2
        )}`
      );
    }
  } else {
    lines.unshift('Formula: Derived by the analytics pipeline for this profile/date window.');
  }

  lines.push('Source: current analytics artifact and executive snapshot.');
  return lines.join('\n');
}

export function buildKpiViewModel(kpis, snapshot = null) {
  const cards = Array.isArray(kpis) ? kpis : [];
  if (!cards.length) {
    return {
      panelId: 'kpi-grid',
      cards: [
        {
          key: 'no-kpi-data',
          label: 'No KPI data',
          displayValue: 'n/a',
          note: 'Run the pipeline to populate this panel.',
          tooltipId: null,
          tooltipText: ''
        }
      ],
      diagnostics: {
        cardCount: 0,
        visibleCardCount: 1,
        keys: ['no-kpi-data']
      }
    };
  }

  const kpiLookup = buildKpiLookup(cards);
  const normalizedCards = cards.map((kpi, index) => {
    const label = cleanText(kpi?.label) || `KPI ${index + 1}`;
    const key = normalizeKpiKey(label) || `kpi${index + 1}`;
    const delta = formatDelta(kpi?.delta_percent);
    const targetDelta = formatDelta(kpi?.target_delta_percent);
    const targetText =
      kpi?.target_delta_percent == null ? '' : ` | vs target ${targetDelta}`;
    const tooltipId = `kpi-tooltip-${index}`;
    const tooltipText = buildKpiExplanation(kpi, kpiLookup, snapshot);

    return {
      key,
      label,
      displayValue: cleanText(kpi?.formatted_value) || fmtNum(kpi?.value || 0, 2),
      note: `vs baseline ${delta}${targetText}`,
      tooltipId,
      tooltipText,
      confidenceLabel: cleanText(kpi?.confidence_label).toLowerCase() || 'unknown'
    };
  });

  return {
    panelId: 'kpi-grid',
    cards: normalizedCards,
    diagnostics: {
      cardCount: normalizedCards.length,
      visibleCardCount: normalizedCards.length,
      keys: normalizedCards.map((card) => card.key),
      confidenceLabels: normalizedCards.map((card) => card.confidenceLabel)
    }
  };
}

export function buildDeltaChartModel(deltas) {
  const points = Array.isArray(deltas)
    ? deltas.filter((delta) => typeof delta?.delta_percent === 'number').slice(0, 8)
    : [];
  const labels = points.map((delta) => cleanText(delta.metric_key) || 'metric');
  const values = points.map((delta) => Number((delta.delta_percent * 100).toFixed(2)));

  return {
    chartKey: 'delta',
    labels,
    values,
    config: !points.length
      ? null
      : {
          type: 'bar',
          data: {
            labels,
            datasets: [
              {
                label: 'Delta % vs baseline',
                data: values,
                backgroundColor: values.map((value) =>
                  value >= 0 ? 'rgba(11,143,140,0.7)' : 'rgba(211,63,73,0.7)'
                ),
                borderColor: values.map((value) =>
                  value >= 0 ? 'rgba(11,143,140,1)' : 'rgba(211,63,73,1)'
                ),
                borderWidth: 1
              }
            ]
          },
          options: {
            responsive: true,
            maintainAspectRatio: false,
            scales: { y: { ticks: { callback: (value) => `${value}%` } } },
            plugins: { legend: { display: false } }
          }
        },
    diagnostics: {
      chartKey: 'delta',
      pointCount: points.length,
      datasetCount: points.length ? 1 : 0,
      labels
    }
  };
}

export function buildChannelMixChartModel(points, roasTarget) {
  const rows = Array.isArray(points) ? points : [];
  const labels = rows.map((row) => cleanText(row.period_label) || 'period');
  const datasets = rows.length
    ? [
        {
          label: 'Spend',
          data: rows.map((row) => Number(row.spend || 0)),
          borderColor: 'rgba(216,87,42,1)',
          backgroundColor: 'rgba(216,87,42,0.12)',
          yAxisID: 'y',
          tension: 0.3
        },
        {
          label: 'Revenue',
          data: rows.map((row) => Number(row.revenue || 0)),
          borderColor: 'rgba(11,143,140,1)',
          backgroundColor: 'rgba(11,143,140,0.12)',
          yAxisID: 'y',
          tension: 0.3
        },
        {
          label: 'ROAS',
          data: rows.map((row) => Number(row.roas || 0)),
          borderColor: 'rgba(31,42,53,1)',
          backgroundColor: 'rgba(31,42,53,0.1)',
          yAxisID: 'y1',
          tension: 0.3
        }
      ]
    : [];

  if (rows.length && typeof roasTarget === 'number') {
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

  return {
    chartKey: 'channel-mix',
    labels,
    config: !rows.length
      ? null
      : {
          type: 'line',
          data: { labels, datasets },
          options: {
            responsive: true,
            maintainAspectRatio: false,
            interaction: { mode: 'index', intersect: false },
            scales: {
              y: { position: 'left', title: { display: true, text: 'Spend / Revenue' } },
              y1: {
                position: 'right',
                grid: { drawOnChartArea: false },
                title: { display: true, text: 'ROAS' }
              }
            }
          }
        },
    diagnostics: {
      chartKey: 'channel-mix',
      pointCount: rows.length,
      datasetCount: datasets.length,
      labels,
      hasRoasTarget: typeof roasTarget === 'number'
    }
  };
}

export function buildDailyRevenueChartModel(points) {
  const rows = Array.isArray(points) ? points : [];
  const totalRevenue = rows.reduce((sum, row) => sum + Number(row?.revenue || 0), 0);
  const activeDays = rows.filter((row) => Number(row?.revenue || 0) > 0).length;
  const labels = rows.map((row) => cleanText(row.date) || 'n/a');
  const revenue = rows.map((row) => Number((row?.revenue || 0).toFixed(2)));
  const conversions = rows.map((row) => Number((row?.conversions || 0).toFixed(2)));

  return {
    chartKey: 'daily-revenue',
    summaryText: !rows.length
      ? 'No daily revenue series available in this run yet.'
      : `Total in selected window: $${fmtNum(totalRevenue, 2)} across ${rows.length} day(s); ${activeDays} day(s) recorded non-zero revenue.`,
    labels,
    config: !rows.length
      ? null
      : {
          data: {
            labels,
            datasets: [
              {
                type: 'line',
                label: 'Revenue ($)',
                data: revenue,
                borderColor: 'rgba(11,143,140,1)',
                backgroundColor: 'rgba(11,143,140,0.12)',
                yAxisID: 'y',
                tension: 0.25
              },
              {
                type: 'bar',
                label: 'Conversions',
                data: conversions,
                borderColor: 'rgba(47,110,165,1)',
                backgroundColor: 'rgba(47,110,165,0.55)',
                yAxisID: 'y1'
              }
            ]
          },
          options: {
            responsive: true,
            maintainAspectRatio: false,
            interaction: { mode: 'index', intersect: false },
            scales: {
              y: { position: 'left', title: { display: true, text: 'Revenue ($)' } },
              y1: {
                position: 'right',
                grid: { drawOnChartArea: false },
                title: { display: true, text: 'Conversions' }
              }
            }
          }
        },
    diagnostics: {
      chartKey: 'daily-revenue',
      pointCount: rows.length,
      datasetCount: rows.length ? 2 : 0,
      totalRevenue: Number(totalRevenue.toFixed(2)),
      activeDays
    }
  };
}
