# White Lexical Contract v1

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=CR-0018`
- `change_request_id=CR-WHITE-0001`

## Purpose

Define machine-checkable lexical boundaries for educational and promotional marketing artifacts.

## Allowed term classes

1. Bounded support qualifiers: `may support`, `can help`, `for some pets`, `based on available evidence`.
2. Scope qualifiers: `in this context`, `when paired with`, `as part of`.
3. Non-judgment framing: `many owners`, `options`, `tradeoffs`, `evaluate`.

## Disallowed term classes

1. Therapeutic implication: `treats`, `cures`, `prevents`, `heals`.
2. Absolute superiority: `best`, `only`, `always`, `guaranteed`.
3. Judgment framing: `failing your pet`, `bad owner`, `neglectful`.

## Authority-signal taxonomy

1. `AS-ABSOLUTE`: absolute language without evidence boundaries.
2. `AS-FEAR`: fear-pressure coupling (`if you do not buy X, harm follows`).
3. `AS-SHAME`: blame language attached to caretaker identity.

## Adversarial minimal-pair tests

1. Overclaim implication:
- pass: `May support digestion for some pets.`
- fail: `Prevents digestive illness.`
2. Efficacy-adjacent wording:
- pass: `Educational guidance based on available evidence.`
- fail: `Clinically proven solution.`
3. Judgment framing:
- pass: `Owners can compare ingredient sourcing.`
- fail: `Owners who do not switch are irresponsible.`

## Context-window fail tests

1. Fail if disallowed term appears within 10 tokens of product recommendation CTA.
2. Fail if authority-signal class appears without an explicit caveat sentence in the same paragraph.
3. Fail if promotional mode contains uncaveated certainty claim.

## Mutation-class coverage thresholds

Minimum test cases per class: `>= 20`.

Mutation classes:
1. Synonym substitution.
2. Word-order reframe.
3. Qualifier inversion (`may` -> `will`).
4. Authority substitution (`supports` -> `clinically proven`).

## Review-state linkage

1. `explore`: lexical warnings allowed; publish blocked.
2. `draft`: fail cases must be resolved or explicitly caveated.
3. `approved`: zero lexical hard fails required.
