# High Impact Action Policy

Effective date: 2026-02-11
Owner roles: Team Lead, Product Steward

## Thresholds

1. `high_impact_action` is true when either condition holds:
- spend >= 1000 USD
- projected audience reach >= 10000

## Hard-fail coupling

1. If contamination/authenticity hard-fail triggers, high-impact actions must be blocked.
2. Overrides require Team Lead + Product Steward and must be append-only logged.

## Required fields in review artifacts

1. `high_impact_action.threshold_spend_usd`
2. `high_impact_action.threshold_reach`
3. `high_impact_action.is_high_impact`
4. `high_impact_action.blocked_when_hard_fail`
