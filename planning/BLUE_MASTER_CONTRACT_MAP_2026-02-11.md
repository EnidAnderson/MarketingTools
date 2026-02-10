# Blue Master Contract Map

Date: 2026-02-11 planning target
Run: run_2026-02-10_001
Owner: Blue Team (architecture and intelligence lead)

## Purpose
Provide one high-level contract map that aligns all teams on the same backbone:
data -> intelligence -> decision -> action -> measurement -> learning.

## Contract Spine

1. Data Contract
- Required fields: `source_class`, `provenance`, `freshness`, `collection_context`.
- Allowed `source_class`: `observed`, `scraped_first_party`, `simulated`.
- Rule: simulated signals cannot be used as observed outcome evidence.

2. Intelligence Contract
- Required fields: `insight_statement`, `supporting_evidence_refs`, `uncertainty_note`.
- Rule: no insight may omit uncertainty scope.

3. Decision Contract
- Required fields: `decision_hypothesis`, `input_refs`, `constraints`, `confidence_label`.
- Rule: decisions must declare confidence before launch, not after.

4. Action Contract
- Required fields: `channel`, `voice_mode`, `approved_constraints_ref`.
- Rule: action artifacts must cite upstream decision contract id.

5. Measurement Contract
- Required fields: `baseline`, `expected_signal`, `observed_signal`, `variance_note`.
- Rule: measurement must separate exploratory vs accountability metrics.

6. Learning Contract
- Required fields: `validated_updates`, `invalidated_assumptions`, `next_cycle_changes`.
- Rule: no learning memo without explicit invalidation section.

## Red-Team Pressure Zones (Always-On)

1. Mixed-signal contamination (observed/scraped/simulated blending).
2. Webhook and connector poisoning (identity spoofing, replay, schema mimicry).
3. Attribution laundering via narrative confidence overreach.
4. Confidence-label misuse (scope mismatch, caveat burial).
5. Freshness exploitation (scrape updates outrunning claim normalization).

## White/Black/Grey Dependencies

1. White:
- canonical terms, confidence caveats, publication-lane boundaries.
2. Black:
- hard block/warn thresholds, fail-closed constraints for decision approval.
3. Grey:
- integrated sequencing and unresolved-tradeoff preservation.

## Done Condition For This Map

1. All new Blue requests reference this map and one pipeline artifact.
2. Red has at least one full open package at all times.
3. Each wave includes at least one Black and one Grey synthesis follow-through ticket.
