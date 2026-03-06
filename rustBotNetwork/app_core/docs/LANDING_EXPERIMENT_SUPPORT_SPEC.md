# Landing Experiment Support Spec

This spec defines the minimum Rust-side support needed so analytics, experimentation, and content creation stay aligned under low-sample conditions.

## Objective

The Rust analytics layer should expose experiment readiness and insight-permission states as typed, auditable data, not as ad hoc narrative strings.

## Required Typed Concepts

1. `LandingContextV1`
   - `taxonomy_version`
   - `matched_rule_id`
   - `landing_path`
   - `landing_family`
   - `landing_page_group`

2. `InsightPermissionCardV1`
   - what statement is being proposed
   - what decision it affects
   - whether it is allowed, directional-only, insufficient, instrument-first, or blocked
   - what the content pipeline may do with that statement

3. `ExperimentReadinessCardV1`
   - control landing family
   - challenger landing families
   - primary metric
   - baseline value
   - minimum detectable effect
   - observed versus required sample
   - blocking reasons and next actions

## Runtime Invariants

1. No causal landing-page lift claim without:
   - explicit experiment or approved quasi-experiment design
   - typed sample context
   - typed permission state other than `insufficient_evidence`, `instrument_first`, or `blocked`
2. If landing taxonomy coverage fails its gate, landing-family recommendations must downgrade to `instrument_first`.
3. If effect precision is below threshold, the card must emit `insufficient_evidence` even when raw deltas look large.
4. Content workflows may consume `allowed_uses` and `blocked_uses` only from typed cards, not infer policy from summary prose.

## Integration Sequence

1. Add landing taxonomy assignment to normalized GA4/Wix session artifacts.
2. Persist experiment metadata fields:
   - `experiment_id`
   - `variant_id`
   - `landing_family`
   - `ad_creative_id`
   - `campaign_id`
   - `ad_group_id`
3. Build dashboard and content-pipeline surfaces from typed insight cards.
4. Only after that, allow the campaign orchestration layer to request landing-page challenger generation automatically.

## Immediate Product Behavior

- The system may say:
  - "Simply Raw is the current control candidate."
- The system may not yet say:
  - "Bundle landing pages will outperform Simply Raw for the same Google Ads traffic."

That second statement should remain blocked until the typed experiment-readiness contract says otherwise.
