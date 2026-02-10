# Subsystem Skeleton Overview

Last updated: 2026-02-10

## Purpose
Define stable folder/module boundaries for long-term maintainability as the project scales from prototype to production infrastructure.

## Rust Domain Modules
Located in `rustBotNetwork/app_core/src/subsystems/`:
1. `marketing_data_analysis/`
2. `campaign_orchestration/`
3. `artifact_governance/`
4. `review_and_compliance/`
5. `provider_platform/`

Each module currently contains:
1. baseline domain types
2. placeholder service trait
3. NDOC contract comments and invariants

## Planning Mirrors
Located in `planning/subsystems/`:
1. `marketing_data_analysis/`
2. `campaign_orchestration/`
3. `artifact_governance/`
4. `review_and_compliance/`
5. `provider_platform/`

Each folder is intended to hold:
1. architecture notes
2. ADRs
3. implementation backlog
4. test strategy

## Why This Matters
1. Reduces ad-hoc sprawl in `tools/`.
2. Creates explicit ownership surfaces for teams.
3. Makes future refactors safer through module-level contracts.
