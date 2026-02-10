# Review Metadata Contract v1

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=CR-0019`
- `change_request_id=CR-WHITE-0002`
- `change_request_id=CR-BLACK-0003`
- `change_request_id=CR-BLACK-0004`

## Purpose

Define deterministic metadata required before an artifact can be labeled `approved`.

## Required fields

1. `run_id`
2. `artifact_id`
3. `mode_label` (`explore|draft|approved`)
4. `confidence_label` (`low|medium|high`)
5. `prompt_version`
6. `model_version`
7. `source_input_hash`
8. `editor_log`
9. `approval_timestamp_utc`
10. `evidence_caveat_map`
11. `provenance_bundle`
12. `white_lexicon_version`
13. `bounded_claim_class`
14. `prohibited_implication_scan`
15. `superseded_ids`

## Deterministic approval linkage

`approved` is valid only when all are true:
1. White lexical hard-fail count is zero.
2. Evidence-to-caveat map is present and non-empty.
3. Provenance bundle has prompt/model/hash/editor lineage.
4. `mode_label`, `confidence_label`, and claim class are mutually valid.

## External editorial submission constraints

Submission is blocked unless:
1. `bounded_claim_class` is declared.
2. `confidence_label` is declared.
3. `prohibited_implication_scan.hits == 0`.
4. Evidence/caveat map exists for every claim in submission scope.

## Superseded-ID citation rule

If legacy request IDs are cited, include active lineage as:
`<active_id> (supersedes <legacy_id>)`.

## Block states

1. Missing required field.
2. Any lexical hard fail.
3. Any prohibited implication hit.
4. Missing evidence-caveat map.
5. Missing provenance fields.
