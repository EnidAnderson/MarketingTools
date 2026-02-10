# ADR Trigger Rules

Effective date: 2026-02-10

## Non-optional trigger conditions

ADR is required before merge when a change:
1. Alters system boundaries/interfaces across subsystems.
2. Changes runtime execution model, job lifecycle, or persistence semantics.
3. Introduces/replaces external providers with architecture impact.
4. Changes security model, trust boundaries, or secret handling strategy.
5. Alters budget enforcement model or release gate semantics.
6. Changes role authority contracts for safety-critical decisions.

## Enforcement

1. If any trigger condition matches and no ADR exists, change gate is red.
2. Red change gate blocks publish.
3. ADR must be created from template in `planning/adrs/ADR_TEMPLATE.md`.

