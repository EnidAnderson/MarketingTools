export const executiveFixtureSnapshot = {
  profile_id: 'marketing_default',
  run_id: 'run-2026-03-05-smoke-001',
  compare_window_runs: 2,
  trust_status: 'degraded',
  roas_target_band: 6.0,
  kpis: [
    {
      label: 'Spend',
      value: 742.18,
      formatted_value: '$742.18',
      delta_percent: 0.08,
      confidence_label: 'high'
    },
    {
      label: 'Revenue',
      value: 4861.42,
      formatted_value: '$4,861.42',
      delta_percent: 0.15,
      confidence_label: 'high'
    },
    {
      label: 'ROAS',
      value: 6.55,
      formatted_value: '6.55x',
      delta_percent: 0.06,
      target_delta_percent: 0.0917,
      confidence_label: 'high'
    },
    {
      label: 'Conversions',
      value: 71,
      formatted_value: '71',
      delta_percent: 0.09,
      confidence_label: 'high'
    },
    {
      label: 'CTR',
      value: 0.084,
      formatted_value: '8.40%',
      delta_percent: 0.03,
      confidence_label: 'medium'
    },
    {
      label: 'CPA',
      value: 10.45,
      formatted_value: '$10.45',
      delta_percent: -0.04,
      confidence_label: 'medium'
    },
    {
      label: 'AOV',
      value: 68.47,
      formatted_value: '$68.47',
      delta_percent: 0.02,
      confidence_label: 'high'
    }
  ],
  funnel_summary: {
    stages: [
      { stage: 'Impression', value: 45200 },
      { stage: 'Click', value: 3796, conversion_from_previous: 0.084 },
      { stage: 'Session', value: 3389, conversion_from_previous: 0.893 },
      { stage: 'Product View', value: 1713, conversion_from_previous: 0.505 },
      { stage: 'Add To Cart', value: 441, conversion_from_previous: 0.257 },
      { stage: 'Checkout', value: 193, conversion_from_previous: 0.438 },
      { stage: 'Purchase', value: 71, conversion_from_previous: 0.368 }
    ]
  },
  historical_analysis: {
    period_over_period_deltas: [
      { metric_key: 'revenue', delta_percent: 0.153 },
      { metric_key: 'roas', delta_percent: 0.064 },
      { metric_key: 'conversions', delta_percent: 0.089 },
      { metric_key: 'cost', delta_percent: 0.077 }
    ]
  },
  channel_mix_series: [
    {
      period_label: '2026-02-13 -> 2026-02-19',
      spend: 610.11,
      revenue: 3642.12,
      roas: 5.97
    },
    {
      period_label: '2026-02-20 -> 2026-02-26',
      spend: 688.77,
      revenue: 4215.84,
      roas: 6.12
    },
    {
      period_label: '2026-02-27 -> 2026-03-03',
      spend: 742.18,
      revenue: 4861.42,
      roas: 6.55
    }
  ],
  daily_revenue_series: [
    { date: '2026-02-26', revenue: 650.12, conversions: 9 },
    { date: '2026-02-27', revenue: 700.33, conversions: 10 },
    { date: '2026-02-28', revenue: 725.5, conversions: 11 },
    { date: '2026-03-01', revenue: 610.1, conversions: 8 },
    { date: '2026-03-02', revenue: 689.22, conversions: 10 },
    { date: '2026-03-03', revenue: 722.05, conversions: 11 },
    { date: '2026-03-04', revenue: 764.1, conversions: 12 }
  ],
  high_leverage_reports: {
    revenue_truth: {
      canonical_revenue: 4861.42,
      canonical_conversions: 71,
      strict_duplicate_ratio: 0.0079,
      near_duplicate_ratio: 0.0212,
      truth_guard_status: 'guarded_review_required',
      inflation_risk: 'low',
      estimated_revenue_at_risk: 103.06,
      custom_purchase_rows: 6,
      custom_purchase_overlap_rows: 5,
      custom_purchase_orphan_rows: 1,
      custom_purchase_overlap_ratio: 0.8333,
      custom_purchase_orphan_ratio: 0.1667,
      summary:
        'Canonical purchase metrics enforced. Duplicate custom purchase rows remain excluded.'
    },
    funnel_survival: {
      points: [
        { stage: 'Impression', entrants: 45200, survival_rate: 1.0, hazard_rate: 0.0 },
        { stage: 'Click', entrants: 3796, survival_rate: 0.084, hazard_rate: 0.916 },
        { stage: 'Session', entrants: 3389, survival_rate: 0.075, hazard_rate: 0.107 },
        { stage: 'Product View', entrants: 1713, survival_rate: 0.038, hazard_rate: 0.495 },
        { stage: 'Add To Cart', entrants: 441, survival_rate: 0.01, hazard_rate: 0.743 },
        { stage: 'Checkout', entrants: 193, survival_rate: 0.004, hazard_rate: 0.562 },
        { stage: 'Purchase', entrants: 71, survival_rate: 0.002, hazard_rate: 0.632 }
      ],
      bottleneck_stage: 'Add To Cart'
    },
    attribution_delta: {
      rows: [
        {
          campaign: 'Puppy Starter Bundle',
          first_touch_proxy_share: 0.43,
          assist_share: 0.41,
          last_touch_share: 0.47,
          delta_first_vs_last: -0.04
        },
        {
          campaign: 'Sensitive Stomach Retarget',
          first_touch_proxy_share: 0.35,
          assist_share: 0.36,
          last_touch_share: 0.31,
          delta_first_vs_last: 0.04
        },
        {
          campaign: 'Subscription Winback',
          first_touch_proxy_share: 0.22,
          assist_share: 0.23,
          last_touch_share: 0.22,
          delta_first_vs_last: 0
        }
      ],
      dominant_last_touch_campaign: 'Puppy Starter Bundle',
      last_touch_concentration_hhi: 0.364,
      summary: 'Last-touch revenue concentration is moderate.'
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
  ],
  operator_summary: {
    attribution_narratives: [
      {
        kpi: 'ROAS',
        narrative:
          'ROAS improved while duplicate custom purchase rows remained excluded from truth KPIs.'
      }
    ]
  },
  alerts: []
};

export const executiveFixtureHistory = [
  {
    metadata: { run_id: executiveFixtureSnapshot.run_id },
    stored_at_utc: '2026-03-05T19:12:44Z',
    artifact: { report: { total_metrics: { roas: 6.55 } } }
  },
  {
    metadata: { run_id: 'run-2026-03-04-smoke-001' },
    stored_at_utc: '2026-03-04T19:12:44Z',
    artifact: { report: { total_metrics: { roas: 6.12 } } }
  }
];
