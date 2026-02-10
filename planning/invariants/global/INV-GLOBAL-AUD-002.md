# INV-GLOBAL-AUD-002

## Invariant Statement
Every publishable artifact must have reconstructable lineage (inputs, spec, run metadata, decision).

## Rationale
Ensures post-hoc investigation and compliance replay.

## Scope
Marketing artifacts and campaign deliverables.

## Enforcement Points
`planning/PRODUCT_STEWARD_OPERATING_MODEL.md`, artifact manifests, run summaries.

## Evidence of Compliance
Lineage record can reconstruct artifact decisions from filesystem.

## Failure State
Artifact cannot be traced to source inputs and decisions.

## Owner Role
Platform Architect.

## Test Requirements
Lineage completeness checks fail on missing required lineage fields.
