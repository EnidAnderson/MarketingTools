# QA Execution Matrix

Date: 2026-02-10
Owner: qa_fixer
Provenance: decision_id=DEC-0001, change_request_id=CR-WHITE-0014

## Scope

Maps open qa_fixer change requests to executable validator commands, required artifact paths, and residual risk output field.

## Matrix

| request_id | validator_command | required_artifact_path | residual_risk_field |
|---|---|---|---|
| CR-WHITE-0004 | `teams/_validation/check_review_artifact_contract.sh` | `data/team_ops/review_artifacts/run_2026-02-10_001_asset_0001.json` | `residual_risk.terminology_contract` |
| CR-WHITE-0005 | `teams/_validation/check_review_artifact_contract.sh` | `data/team_ops/review_artifacts/run_2026-02-10_001_asset_0001.json` | `residual_risk.mixed_source_downgrade` |
| CR-WHITE-0006 | `teams/_validation/check_review_artifact_contract.sh` | `data/team_ops/review_artifacts/run_2026-02-10_001_asset_0001.json` | `residual_risk.glossary_drift` |
| CR-WHITE-0007 | `teams/_validation/check_review_artifact_contract.sh` | `data/team_ops/review_artifacts/run_2026-02-10_001_asset_0001.json` | `residual_risk.lifecycle_policy` |
| CR-WHITE-0008 | `teams/_validation/check_review_artifact_contract.sh` | `data/team_ops/review_artifacts/run_2026-02-10_001_asset_0001.json` | `residual_risk.continuity_template` |
| CR-WHITE-0009 | `teams/_validation/check_review_artifact_contract.sh` | `data/team_ops/review_artifacts/run_2026-02-10_001_asset_0001.json` | `residual_risk.causal_overstatement` |
| CR-WHITE-0010 | `teams/_validation/check_review_artifact_contract.sh` | `data/team_ops/review_artifacts/run_2026-02-10_001_asset_0001.json` | `residual_risk.metric_absolutism` |
| CR-WHITE-0011 | `teams/_validation/check_review_artifact_contract.sh` | `data/team_ops/review_artifacts/run_2026-02-10_001_asset_0001.json` | `residual_risk.trust_delta_transition` |
| CR-WHITE-0012 | `teams/_validation/check_review_artifact_contract.sh` | `data/team_ops/review_artifacts/run_2026-02-10_001_asset_0001.json` | `residual_risk.signal_language` |
| CR-WHITE-0015 | `teams/_validation/check_review_artifact_contract.sh` | `data/team_ops/review_artifacts/run_2026-02-10_001_asset_0001.json` | `residual_risk.fallback_state_clarity` |
| CR-BLACK-0005 | `teams/_validation/check_review_artifact_contract.sh` | `data/team_ops/review_artifacts/run_2026-02-10_001_asset_0001.json` | `residual_risk.high_impact_blocking` |
| CR-BLACK-0006 | `teams/_validation/check_review_artifact_contract.sh` | `data/team_ops/review_artifacts/run_2026-02-10_001_asset_0001.json` | `residual_risk.connector_authenticity` |
| CR-BLACK-0007 | `teams/_validation/check_review_artifact_contract.sh` | `data/team_ops/review_artifacts/run_2026-02-10_001_asset_0001.json` | `residual_risk.social_rollout_gate` |

## Completion Rule

A request is `done` only when:
1. validator command exits 0,
2. required artifact path exists and was validated,
3. residual risk field is documented in QA closeout notes.
