# Data Collection Improvement Playbook

Use this playbook when analysis confidence is limited by data quality or statistical power.

## Priority 1: Event Integrity

- Deduplicate purchase events using transaction-level keys.
- Align canonical event names across all tags.
- Record source tag id and ingestion timestamp per event.

## Priority 2: Identity And Session Quality

- Standardize anonymous/user identifiers across devices where policy allows.
- Capture stable session and campaign identifiers.
- Track consent-mode coverage and resulting missingness.

## Priority 3: Revenue Fidelity

- Store transaction currency and normalized currency values.
- Track refund/cancellation events with linkage to original transaction.
- Reconcile GA4 purchase totals against store backend totals daily.

## Priority 4: Experimentability

- Add experiment id and variant id to user journey events.
- Ensure exposure logging precedes outcome logging.
- Preserve assignment logs for intent-to-treat analysis.

## Priority 5: Statistical Capacity

- Reduce sparse dimensions with controlled taxonomy.
- Predefine decision windows to avoid peeking bias.
- Increase observation horizon when MDE is above business threshold.
