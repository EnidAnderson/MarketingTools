function cleanText(value) {
  return String(value == null ? '' : value).trim();
}

function fmtNum(value, decimals = 2) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '0.00';
  return numeric.toLocaleString(undefined, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals
  });
}

function fmtInt(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '0';
  return Math.round(numeric).toLocaleString();
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
