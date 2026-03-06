# Experiment Insight Gating Spec

This spec defines how the data science and content systems distinguish between allowed facts, directional observations, and claims that are not yet justified.

## Permission States

| State | Meaning | Content Pipeline Action |
| --- | --- | --- |
| `allowed_operational_claim` | Safe to use for operational decisions because the statement is descriptive, bounded, and supported by current evidence. | May be used to select control baselines, prioritize implementation work, or frame experiment setup. |
| `directional_only` | Suggestive but not decision-safe as a primary claim. | May inspire challenger concepts, but must not be phrased as a promised lift. |
| `insufficient_evidence` | Not enough data or not enough precision to recommend. | Generate no performance claim; gather more data or run an experiment. |
| `instrument_first` | Instrumentation or taxonomy gaps prevent valid inference. | Block decision usage until tracking/taxonomy work is fixed. |
| `blocked` | Unsafe or logically invalid to use. | Do not route into content planning or executive recommendation. |

## Allowed Operational Facts Today

1. The current Simply Raw paid landing route can serve as the control candidate for future landing experiments.
2. Tablet traffic materially underperforms desktop traffic and is a valid optimization target.
3. Purchase sessions retain landing and source context well enough for experiment setup.

## Not-Yet-Permitted Claims

1. Redirecting the same Google Ads traffic to bundle landing pages will outperform the Simply Raw control.
2. Redirecting the same Google Ads traffic to product-detail pages will outperform the Simply Raw control.
3. Any page-family recommendation framed as causal lift without randomized routing or a defended quasi-experiment.

## Required Data To Unlock Those Claims

1. Stable `landing_family` taxonomy in all traffic analysis outputs.
2. Experiment metadata on the journey:
   - `experiment_id`
   - `variant_id`
   - `landing_family`
   - `ad_creative_id`
   - `campaign_id`
   - `ad_group_id`
3. Predeclared experiment design with:
   - control family
   - challenger family
   - primary metric
   - minimum detectable effect
   - required sample size

## Translation To Content Creation

- `allowed_operational_claim`:
  - Content team may treat the supported page as the current control.
  - Creative team may build challengers around alternative landing families.
- `directional_only`:
  - Content team may produce ideas and drafts.
  - No copy or planning note may imply expected lift as settled fact.
- `insufficient_evidence`:
  - Content team may prepare concepts, but paid-routing decisions stay unchanged.
  - Scientist must specify the minimum additional sample or experiment needed.
- `instrument_first`:
  - Implementation work takes priority over creative iteration.
  - Any downstream insight card must visibly state why it is blocked.

## Rust Support Requirement

The Rust analytics layer should surface these states as typed cards rather than burying them in narrative strings. The content pipeline should consume only the typed permission state, not infer policy from prose.
