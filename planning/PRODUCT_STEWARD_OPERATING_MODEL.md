# Product Steward Operating Model (Marketing Systems)

Last updated: 2026-02-10  
Owner: Platform Architecture + Marketing Operations

## 1. Role Definition
Internal title:
1. Product Steward, Marketing Systems

Mission:
1. Ensure marketing asset generation is reliable, traceable, and production-safe.
2. Convert design intent into explicit constraints executable by tools and agents.
3. Protect brand trust by preventing silent geometry/content failures.

## 2. Core Principle
Every asset is a compilable artifact:
1. Inputs are versioned.
2. Prompts/specs are pinned.
3. Outputs are reproducible.
4. Failures are explicit and auditable.

## 3. Responsibilities
1. Define and maintain asset contract:
- required inputs
- geometry/material constraints
- acceptance checklist
- output metadata schema
2. Separate exploration and production workflows:
- exploration: rapid, cheap, reversible
- production: locked references and deterministic settings
3. Govern failure modes:
- safe degradation preferred over plausible-but-wrong output
- reject outputs that violate geometry or claim constraints
4. Own cross-functional quality loop:
- designers (visual correctness)
- marketers (campaign intent)
- engineers (tooling and runtime integrity)

## 4. Decision Rights
1. Can block promotion of assets that fail production-safety checks.
2. Can require prompt/spec revision before reruns.
3. Can enforce migration/testing gates for tooling changes affecting output trust.
4. Escalates business trade-offs when schedule pressure conflicts with quality gates.

## 5. Required Skill Profile
1. Structural design literacy (geometry, layout, packaging realities).
2. Marketing judgment (exploration vs approval vs production use).
3. ML systems skepticism (constraint-first prompting and failure containment).
4. Engineering/audit fluency (versioning, artifact lineage, reproducibility).

## 6. Artifact Contract (Minimum)
For each generated asset, store:
1. `input/` references (source labels, images, specs).
2. `spec/` machine-readable constraints and human-readable brief.
3. `run/` execution metadata:
- tool name/version
- prompt version/hash
- model/provider
- timestamp
- run id
4. `output/` generated assets and review derivatives.
5. `decision/` approval or rejection rationale.

## 7. Quality Gates
1. Reproducibility gate:
- rerun with same inputs produces materially equivalent output class.
2. Geometry gate:
- no impossible packaging geometry or label wrap errors.
3. Brand/trust gate:
- no hallucinated claims, logos, or misleading content.
4. Audit gate:
- lineage can be reconstructed from filesystem artifacts only.

## 8. Workflow States
1. `explore`:
- loose constraints
- low-cost iterations
- non-final assets
2. `candidate`:
- selected concepts with tightened constraints
- partial checklist enforced
3. `production`:
- frozen references/specs
- full checklist enforced
- approval record required

## 9. Failure Taxonomy (Operational)
1. Spec bug: ambiguous or missing constraints.
2. Prompt bug: under-constrained instructions causing drift.
3. Tool bug: parsing/runtime defect in Rust/Tauri tooling.
4. Model limitation: known inability requiring alternate approach.
5. Process bug: output promoted without required gate checks.

## 10. Integration with Rust/Tauri Platform
1. `app_core` tools must expose structured outputs that include evidence/metadata.
2. Tauri job runtime must preserve run IDs and terminal state details.
3. Registry metadata must be complete so GUI workflows are schema-driven.
4. Campaign filesystem conventions become part of acceptance criteria.

## 11. Metrics
1. First-pass acceptance rate for production candidates.
2. Rerun reproducibility success rate.
3. Rate of rejected outputs due to geometry/claim violations.
4. Mean time from concept to approved production asset.
5. Percentage of assets with complete audit trail.

## 12. Anti-Patterns
1. “Looks good enough” without checklist evidence.
2. Prompt-only fixes when root issue is missing spec contract.
3. Hidden manual edits that bypass artifact lineage.
4. Tool-specific frontend hacks that bypass schema contracts.

## 13. Implementation Steps
1. Add this operating model to planning references and onboarding.
2. Encode checklist fields into output metadata schema.
3. Add run artifact manifest generation in Rust tools.
4. Add UI panels for lineage, constraints, and gate status.
