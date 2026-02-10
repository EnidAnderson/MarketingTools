# Pipeline Stage 05: White Output

Append-only.

## Entry template
- run_id:
- timestamp_utc:
- definitions:
- fact_checks:
- terminology_corrections:
- change_requests:
- references:

---

- run_id: run_2026-02-10_001
- timestamp_utc: 2026-02-10T22:08:42Z
- definitions: |
    | term | canonical definition | pass indicator | fail indicator |
    |---|---|---|---|
    | clarity | A message is clear when a first-time pet parent can restate the feeding recommendation, rationale, and next step in plain language without guessing. | Reviewer can answer: what to do, why, and what happens next. | Vague benefit language without concrete recommendation or rationale. |
    | confidence | Confidence is justified user certainty based on transparent reasoning and bounded claims, not emotional pressure. | Claims include scope limits and verifiable rationale. | Copy relies on urgency, fear, or authority without evidence boundaries. |
    | trustworthy guidance | Guidance is trustworthy when it is transparent about evidence limits, avoids implied clinical efficacy, and is reproducible from documented inputs. | Provenance fields present; language stays educational, not therapeutic. | Unbounded health implication or missing lineage metadata. |
- fact_checks:
    - "Trust-building" and "clear guidance" positioning is directionally coherent, but no factual efficacy evidence is cited in current artifacts; treat all benefit wording as non-clinical unless explicitly sourced.
    - Red risk claims about overclaim/judgment drift are consistent with Blue language patterns and are valid as process-risk findings, not external empirical findings.
    - Current pipeline evidence does not include Green/Black outputs; therefore cross-team consistency checks are partial for this run state.
- terminology_corrections:
    - Replace "healthier" with "supports everyday nutrition goals" unless evidence is explicitly referenced.
    - Replace "better food" with "nutrition option with stated ingredient/process rationale" to avoid undefined superiority claims.
    - Replace "guide" with "educational guidance" when adjacent to outcome language.
    - Normalize review labels to `explore`, `draft`, `approved`; reject unlabeled artifacts.
- change_requests:
    - CR-0011: Add a canonical lexical guardrail artifact with allowed/disallowed examples and review fail tests for overclaim, implied efficacy, and judgment framing; target owner `qa_fixer`.
    - CR-0012: Add required provenance metadata fields (prompt/version/input hash/editor/date/confidence label) to review and approval handoff artifacts; target owner `qa_fixer`.
- references:
    - teams/white/prompt.md
    - teams/white/spec.md
    - pipeline/01_blue_output.md
    - pipeline/02_red_output.md
    - pipeline/03_green_output.md
    - pipeline/04_black_output.md

1. Summary (<= 300 words).
White review completed for CR-0005 and CR-0009 scope with available inputs. Canonical definitions were set for clarity, confidence, and trustworthy guidance to reduce semantic drift in copy and review. Terminology was normalized to avoid implicit efficacy and judgment language. Fact check posture: current artifacts contain strategic narrative intent and risk hypotheses, not validated clinical claims; therefore health-outcome wording must stay bounded and educational. Cross-team consistency remains partially blocked because Green and Black stage outputs are still scaffold-only, limiting end-to-end terminology reconciliation across UX and operational constraints.

2. Numbered findings.
1. "Healthier" and "better" are currently ambiguous and can imply superiority without criteria.
2. Blue wording can be read as efficacy-adjacent unless claim boundaries are explicit.
3. Red findings are process-consistent with Blue language and should be converted into lexical fail tests.
4. Confidence labeling is undefined in handoff artifacts, creating approval ambiguity.
5. Provenance requirements are not yet mandatory in stage-level narrative outputs.

3. Open questions (if any).
- Should "trustworthy guidance" explicitly require veterinary-review traceability for any physiology-adjacent claim?
- Should confidence labels be enforced at artifact level only, or also at section/claim level?

4. Explicit non-goals.
- No strategic positioning changes.
- No UX redesign recommendations.
- No code/config/schema/script edits.

---

- run_id: run_2026-02-10_001
- timestamp_utc: 2026-02-10T22:10:04Z
- definitions:
    - amendment: White request IDs corrected to avoid collisions in queue.
- fact_checks:
    - Queue already contains prior CR-0011 and CR-0012 entries from earlier stages.
- terminology_corrections:
    - supersede_references: Use CR-0018 and CR-0019 for White implementation requests in this run.
- change_requests:
    - CR-0018
    - CR-0019
- references:
    - data/team_ops/change_request_queue.csv
    - data/team_ops/handoff_log.csv

1. Summary (<= 300 words).
Correction entry only. White implementation requests were renumbered to unique IDs for queue integrity.

2. Numbered findings.
1. Duplicate request IDs existed in queue before White append.
2. Unique renumbering is required for deterministic downstream execution.

3. Open questions (if any).
- None.

4. Explicit non-goals.
- No change to substantive White definitions or findings.

---

- run_id: run_2026-02-10_001
- timestamp_utc: 2026-02-10T22:30:51Z
- definitions: |
    | domain | canonical definition | pass test | fail test |
    |---|---|---|---|
    | dual_voice.promotional | Persuasive marketing may recommend a product action, but cannot imply diagnosis, treatment, prevention, or certainty beyond cited evidence. | Contains bounded qualifiers + explicit recommendation scope. | Uses therapeutic implication or absolute superiority. |
    | dual_voice.educational | Neutral educational content explains options, tradeoffs, and evidence limits independent of immediate conversion intent. | Reader can restate options and caveats without CTA pressure. | Hidden conversion pressure, comparative absolutes, or selective uncertainty. |
    | confidence.label.explore | Early hypothesis content; non-final and non-claim-binding. | Labeled `explore` and excluded from publish path. | Unlabeled or routed to approval. |
    | confidence.label.draft | Review-ready but not publish-authorized; claims still require lexical + provenance checks. | `draft` + checks pending/recorded. | Treated as approved without full checks. |
    | confidence.label.approved | Publish-eligible only when lexical, evidence, provenance, and mode-consistency checks all pass. | Full gate bundle present and non-red. | Missing any gate artifact. |
- fact_checks:
    - Blue/Red/Green/Black requests are now mutually satisfiable under one White contract: mode taxonomy + lexical boundaries + contradiction checks + citation normalization.
    - All efficacy-adjacent language remains unsourced in pipeline artifacts; therefore only educational/qualified claim classes are allowable.
    - Queue contains historical ID collisions and mixed formats; White now defines canonical citation behavior for deterministic audit lineage.
- terminology_corrections:
    - Allowed evidence qualifiers: `may support`, `is associated with`, `in this context`, `for some pets`, `based on available evidence`.
    - Disallowed implication classes: `treats`, `cures`, `prevents`, `guarantees`, `clinically proven` (unless externally cited and approved by policy).
    - Authority-signal red flags: comparative absolutes (`best`, `only`, `always`), selective uncertainty suppression, fear-pressure adjacency.
    - Contradiction rule: one term => one definition; artifacts must cite canonical definition ID and fail on alternative local redefinition.
    - Superseded-ID citation rule: narrative artifacts must cite `active_id (supersedes legacy_id)` when legacy IDs appear.
- change_requests:
    - CR-WHITE-0001: qa_fixer to implement a versioned White lexicon artifact + adversarial minimal-pair/context-window tests and mutation classes (synonym, reorder, qualifier inversion, authority substitution).
    - CR-WHITE-0002: qa_fixer to implement a review metadata contract enforcing mode/label/evidence/provenance fields and superseded-ID citation normalization checks.
- references:
    - pipeline/01_blue_output.md
    - pipeline/02_red_output.md
    - pipeline/03_green_output.md
    - pipeline/04_black_output.md
    - data/team_ops/change_request_queue.csv
    - teams/white/spec.md
    - teams/shared/HANDOFF_PROTOCOL.md

1. Summary (<= 300 words).
White processed all currently queued White-assigned requests (Blue, Red, and Green originated) in a single canonical package. The package defines dual-voice boundaries, editorial lexical controls, contradiction tests, confidence-label semantics, adversarial drift tests, authority-signal red flags, and superseded-ID citation rules. This resolves duplicate/superseded White tickets by normalizing to one operational contract and converts implementation work into two qa_fixer-executable requests.

2. Numbered findings.
1. Token-level phrase replacement is insufficient; context-window and mutation-resilience checks are required.
2. Educational and promotional voices must be enforced by claim-logic constraints, not style cues.
3. Confidence labels require deterministic gate dependencies to prevent false approvals.
4. ID lineage ambiguity is itself a governance risk; citation normalization is mandatory.

3. Open questions (if any).
- Should mutation-test minimum coverage be fixed at launch (e.g., >=20 cases/class) or adaptive by incident frequency?

4. Explicit non-goals.
- No strategic narrative rewrite.
- No UX flow redesign.
- No executable artifact edits by White.

---

- run_id: run_2026-02-10_001
- timestamp_utc: 2026-02-10T22:38:42Z
- definitions: |
    | fail_state_id | label | canonical definition | journey_stage_map | mode_map | pass_example | fail_example |
    |---|---|---|---|---|---|---|
    | WS-FS-001 | confusion | User cannot restate what action to take, why it is suggested, and what uncertainty remains. | discovery,evaluation,repeat | educational,promotional | "For some pets, this recipe may support digestion; compare ingredient sources, then choose based on your pet's tolerance." | "This is better nutrition." |
    | WS-FS-002 | overclaim | Content implies diagnosis, treatment, prevention, or guaranteed outcomes beyond available evidence boundaries. | discovery,evaluation,repeat | educational,promotional | "May support coat health as part of a complete diet; results vary by pet." | "Prevents long-term illness and guarantees better health." |
    | WS-FS-003 | judgment | Content assigns caretaker blame/shame or implies moral failure for current feeding behavior. | discovery,evaluation,repeat | educational,promotional | "Many owners are comparing options; here is how to evaluate ingredients clearly." | "If you still feed commodity food, you're failing your pet." |
- fact_checks:
    - Green request scope is definitional and within White authority.
    - Journey and mode mappings are now explicit and testable.
- terminology_corrections:
    - Replace certainty absolutes with bounded qualifiers (`may`, `can`, `for some pets`, `based on available evidence`).
    - Remove blame language and moral framing from all caretaker-directed copy.
- change_requests:
    - CR-WHITE-0003: qa_fixer to encode WS-FS-001/002/003 as machine-checkable review checks across mode and stage metadata.
- references:
    - pipeline/03_green_output.md
    - pipeline/05_white_output.md
    - teams/white/spec.md

1. Summary (<= 300 words).
Fulfillment for CR-GREEN-0005: White has published canonical fail-state definitions for confusion, overclaim, and judgment, each mapped across journey stages (discovery, evaluation, repeat) and both voice modes (educational, promotional), with pass/fail examples for deterministic review.

2. Numbered findings.
1. Previous fail-state language existed but was not normalized to journey+mode.
2. Deterministic pass/fail examples are now available for each fail state.
3. This closes White definitional scope; implementation is delegated to QA checks.

3. Open questions (if any).
- None.

4. Explicit non-goals.
- No UX flow changes.
- No strategic copy rewrite.
- No code/config/schema edits by White.

---

- run_id: run_2026-02-10_001
- timestamp_utc: 2026-02-10T23:05:41Z
- definitions: |
    | domain | canonical term | definition | allowed review phrasing | disallowed phrasing |
    |---|---|---|---|---|
    | data_lineage | observed_data | Directly measured platform/first-party events collected from production systems (e.g., Velo/Wix/Google Ads/Analytics telemetry). | "Observed behavior data" | "Customer feedback" when source is synthetic or inferred |
    | data_lineage | scraped_first_party_data | Retrieved first-party published content/state snapshots used for context, not direct behavior measurement. | "Scraped first-party context" | "Measured customer behavior" |
    | data_lineage | simulated_planning_data | Generated scenario/test signal used for planning stress tests only; never evidence of real user behavior. | "Simulated planning signal" | "Observed trend" or "actual customer response" |
    | analytics_architecture | rust_first_typed_analytics | Production decision-path statistics must execute inside typed, compiled Rust contracts with deterministic build semantics. | "Typed-contract analytics path" | "Equivalent ad-hoc script path" |
    | analytics_architecture | script_assisted_integration | Non-Rust connector/webhook edges may exist only at bounded ingestion interfaces with explicit validation and downgrade rules. | "Bounded edge integration" | "Core analytics engine" |
- fact_checks:
    - CR-0035-BLUE, CR-0041-BLUE, CR-0042-BLUE, and CR-0047-BLUE are definitional/terminology requests and within White authority.
    - Mixed-source caveats must be mandatory whenever observed, scraped, and simulated signals co-occur in one decision artifact.
    - Rust-first language now distinguishes decision-path compute from connector-edge implementation to prevent contract-risk masking.
- terminology_corrections:
    - Mandatory label tuple in review artifacts: `source_class` (`observed|scraped_first_party|simulated`) + `analytics_path` (`typed_rust|edge_script_assisted`).
    - Disallow wording that implies script parity with typed production analytics (e.g., "same guarantees as Rust path").
    - Require mixed-signal caveat block whenever >1 source class contributes to one conclusion.
    - Require confidence-label deltas: if simulated data contributes to claim weighting, cap label at `draft` unless explicitly segregated from decision metric.
- change_requests:
    - CR-WHITE-0004: qa_fixer to implement a machine-checkable terminology contract for source_class + analytics_path labels, including disallowed phrase checks.
    - CR-WHITE-0005: qa_fixer to implement confidence-label caveat templates for stats+viz outputs with mixed-signal downgrade enforcement.
- references:
    - planning/BLUE_SYSTEM_SKELETON_2026-02-10_to_2026-02-11.md
    - planning/MARKET_ANALYSIS_SUITE_ARCHITECTURE.md
    - pipeline/01_blue_output.md
    - teams/white/spec.md

1. Summary (<= 300 words).
White fulfilled Blue's immediate priority set: CR-0035-BLUE, CR-0041-BLUE, CR-0042-BLUE, and CR-0047-BLUE. This package defines canonical boundaries for observed vs scraped vs simulated data, establishes Rust-first typed analytics terminology versus script-assisted edge integration, and standardizes confidence-label caveat templates for mixed-source statistical/visual outputs. It prevents category leakage (simulated presented as observed), prevents architecture-risk masking (scripts described as equivalent to typed contracts), and enforces explicit confidence downgrades when simulated signal contributes to decision claims.

2. Numbered findings.
1. Category leakage risk is highest when scraped/simulated inputs are unlabeled in summary narratives.
2. "Rust-first" language must differentiate core typed decision paths from bounded connector edges.
3. Mixed-signal analytics outputs require mandatory caveat templates and label downgrade rules.
4. Stats+viz interpretability fails when caveats are optional or detached from confidence labels.

3. Open questions (if any).
- Should simulated-signal contribution always hard-cap at `draft`, or allow `approved` only when isolated to scenario appendix fields?

4. Explicit non-goals.
- No architecture redesign.
- No connector implementation details.
- No executable artifact edits by White.

---

- run_id: run_2026-02-10_001
- timestamp_utc: 2026-02-10T23:07:34Z
- definitions: |
    | glossary_id | canonical term | definition | disallowed synonym classes |
    |---|---|---|---|
    | WG-001 | data | Collected source records before interpretation; must retain source_class and collection context. | insight, evidence, conclusion |
    | WG-002 | insight | Interpreted pattern derived from data with method + uncertainty attached. | fact, proof, guaranteed outcome |
    | WG-003 | decision | Chosen action based on insights, constraints, and confidence label. | automatic truth, self-evident action |
    | WG-004 | measurement | Post-action observed metrics used to evaluate effects against baseline. | opinion-only signal, simulated outcome |
    | WG-005 | learning | Validated update to playbooks/contracts based on measured outcomes and residual risk notes. | one-off anecdote, unsourced intuition |
    | WG-006 | publication_lane_educational | External-facing educational authority content prioritizing bounded explanation over conversion pressure. | soft-sell disguised as neutral guidance |
    | WG-007 | publication_lane_promotional | External-facing persuasive content allowed only with explicit promotion markers and bounded claims. | neutral editorial guidance when promotional intent exists |
- fact_checks:
    - CR-0028-BLUE scope (canonical glossary) is now satisfied with one-term/one-definition mappings and synonym prohibitions.
    - CR-0029-BLUE lifecycle confidence policy is now explicit across `pre_launch`, `in_flight`, and `post_campaign` stages.
    - CR-0034-BLUE and CR-0039-BLUE publication-lane boundaries now distinguish educational authority from promotion and constrain scraped-update claim drift.
- terminology_corrections:
    - Confidence policy by lifecycle:
      - `pre_launch`: max `draft` unless evidence is observed_data-only and provenance complete.
      - `in_flight`: `approved` allowed only with live measurement linkage + caveat block.
      - `post_campaign`: claims require measured outcome attribution; simulated or scraped-only support cannot elevate label above `draft`.
    - Mandatory caveat templates:
      - "This conclusion uses [source_class]; confidence is [label] because [uncertainty]."
      - "Scraped first-party updates provide context, not measured customer behavior."
      - "Simulated planning data informs scenarios only and is excluded from outcome claims."
    - Publication lane boundary rule:
      - Educational lane must avoid direct conversion pressure, comparative absolutes, and implied clinical outcomes.
      - Promotional lane must disclose promotional intent and retain bounded-claim language.
    - Scrape freshness guardrail:
      - Fresh scraped context may update wording/examples only; it cannot introduce stronger efficacy assertions without observed-data evidence.
- change_requests:
    - CR-WHITE-0006: qa_fixer to implement glossary/term-lint checks enforcing WG-001..WG-007 canonical usage and disallowed synonym detection.
    - CR-WHITE-0007: qa_fixer to implement lifecycle confidence-policy validator with publication-lane boundary and scraped-update drift checks.
- references:
    - planning/BLUE_SYSTEM_SKELETON_2026-02-10_to_2026-02-11.md
    - pipeline/01_blue_output.md
    - pipeline/05_white_output.md
    - teams/white/spec.md

1. Summary (<= 300 words).
White fulfilled CR-0028-BLUE, CR-0029-BLUE, CR-0034-BLUE, and CR-0039-BLUE in a single canonical package. The system glossary now defines one source-of-truth term set for data, insight, decision, measurement, learning, and publication-lane types with disallowed synonym classes. Lifecycle confidence policy is now stage-specific (`pre_launch`, `in_flight`, `post_campaign`) and bounded by source provenance and uncertainty. Publication-lane boundaries explicitly separate educational authority from promotional persuasion, and scraped first-party freshness is constrained to context refresh rather than claim escalation. The package adds deterministic caveat templates and delegates enforcement to QA validators.

2. Numbered findings.
1. Terminology drift risk is concentrated in `data` vs `insight` vs `learning` substitutions.
2. Confidence labels are often treated globally; they must be lifecycle-scoped.
3. Publication authority erodes when promotional intent is hidden in educational lane language.
4. Scraped freshness can silently increase claim strength unless explicitly blocked.

3. Open questions (if any).
- Should publication-lane metadata be mandatory at artifact section level, not only artifact level?

4. Explicit non-goals.
- No channel strategy changes.
- No implementation code edits by White.
- No architectural constraint authorship (Black authority).

---

- run_id: run_2026-02-10_001
- timestamp_utc: 2026-02-10T23:09:58Z
- definitions: |
    | template_id | context | canonical template | required caveat token | pass example | fail example |
    |---|---|---|---|---|---|
    | CT-001 | intelligence_to_action_transition | "We recommend [action] based on [observed_data_scope]. Confidence: [label]. Key uncertainty: [uncertainty]." | `source_class=observed` | "We recommend testing campaign A based on observed Wix checkout trends. Confidence: draft. Key uncertainty: short time window." | "We recommend campaign A because data proves it will win." |
    | CT-002 | mixed_source_transition | "This recommendation combines [observed_data] with [scraped/simulated] context. Decision weighting excludes simulated outcomes from measured-effect claims." | `mixed_source_caveat` | "This recommendation combines observed ad spend performance with scraped message context. Decision weighting excludes simulated outcomes from measured-effect claims." | "All sources confirm this outcome is measured." |
    | CT-003 | connector_advisory_warning | "Connector input is advisory until typed-contract validation passes. Current state: [pass/fail]." | `analytics_path=edge_script_assisted` | "Connector input is advisory until typed-contract validation passes. Current state: fail." | "Connector data is fully trusted immediately." |
    | CT-004 | publication_continuity_note | "This section is educational guidance. Promotional claims, if any, are explicitly marked and bounded." | `publication_lane` | "This section is educational guidance. Promotional claims, if any, are explicitly marked and bounded." | "Educational guidance: this is the best product and guarantees results." |
- fact_checks:
    - CR-GREEN-0008 requests operator/user continuity templates aligned to source-class caveats; templates CT-001..CT-004 satisfy this scope.
    - Template language is compatible with existing White source_class and confidence-label controls.
- terminology_corrections:
    - Mandatory fields in continuity sections: `source_class`, `confidence_label`, `uncertainty_note`, `publication_lane`, `analytics_path` when connector-derived.
    - Disallow unqualified certainty verbs (`proves`, `guarantees`, `always`) in continuity templates.
- change_requests:
    - CR-WHITE-0008: qa_fixer to implement template-presence and token-validation checks for CT-001..CT-004 in review artifacts and operator handoff notes.
- references:
    - pipeline/03_green_output.md
    - pipeline/05_white_output.md
    - teams/white/spec.md

1. Summary (<= 300 words).
White fulfilled CR-GREEN-0008 with canonical continuity templates for intelligence-to-action transitions and connector advisory states. The package includes required caveat tokens, plus pass/fail examples aligned to source-class and confidence-label controls. This reduces continuity drift where operator messaging can otherwise overstate certainty or hide mixed-signal limitations.

2. Numbered findings.
1. Continuity messaging fails most often when connectors are treated as trusted before typed validation.
2. Mixed-source recommendations require explicit segregation language to prevent measured/simulated blending.
3. Publication continuity must preserve educational-vs-promotional lane clarity at section level.

3. Open questions (if any).
- Should CT templates be mandatory for every approved artifact, or only those with connector or mixed-source inputs?

4. Explicit non-goals.
- No UX flow redesign.
- No connector implementation details.
- No executable edits by White.

---

- run_id: run_2026-02-10_001
- timestamp_utc: 2026-02-10T23:11:15Z
- definitions: |
    | causal_term_id | term | constrained meaning | mandatory caveat | disallowed overstatement |
    |---|---|---|---|---|
    | CAUS-001 | attribution_signal | Evidence of association between touchpoints and outcomes, not standalone proof of causal impact. | "Attribution indicates association under current model assumptions." | "This channel caused the outcome." |
    | CAUS-002 | freshness_update | Recency improvement in contextual inputs (including scrape updates) that may refine interpretation but does not itself confirm behavioral change. | "Freshness update changes context recency, not measured customer effect." | "Fresh data proves user behavior changed." |
    | CAUS-003 | publication_trust_signal | External editorial acceptance indicating distribution credibility, not empirical validation of performance claims. | "Publication trust reflects editorial acceptance, not causal efficacy proof." | "Editorial publication proves campaign effectiveness." |
    | CAUS-004 | causal_claim_guard | A claim may be causal only when observed measurement, counterfactual logic, and uncertainty bounds are explicitly present. | "Causal wording allowed only with measured effect, method note, and uncertainty range." | "Clearly caused", "definitely drove", "proved impact" without method/uncertainty |
- fact_checks:
    - CR-0061-BLUE scope is terminological and caveat-based; within White authority.
    - Current artifacts rely heavily on attribution/freshness/publication narratives that are vulnerable to causal overstatement without explicit wording constraints.
- terminology_corrections:
    - Mandatory sentence when attribution appears: "Association, not standalone causation, unless causal guard conditions are met."
    - Mandatory sentence when freshness updates are cited: "Context recency updated; behavioral effect unconfirmed until observed measurement review."
    - Mandatory sentence when publication lane trust is cited: "Editorial acceptance does not elevate confidence in causal performance claims."
    - Prohibited phrasing class: deterministic causal verbs without method + uncertainty (`caused`, `proved`, `guaranteed impact`, `definitively drove`).
- change_requests:
    - CR-WHITE-0009: qa_fixer to implement CAUS-001..CAUS-004 terminology/caveat checks and prohibited-causal-verb detection in review artifacts.
- references:
    - planning/BLUE_MASTER_CONTRACT_MAP_2026-02-11.md
    - pipeline/01_blue_output.md
    - pipeline/05_white_output.md
    - teams/white/spec.md

1. Summary (<= 300 words).
White fulfilled CR-0061-BLUE by defining canonical terminology and required caveats that block causal overstatement across attribution, freshness, and publication-lane narratives. The package distinguishes association from causation, recency from measured behavioral effect, and editorial trust from empirical efficacy. It adds deterministic disallowed phrasing rules and a causal-claim guard requirement for any causal wording.

2. Numbered findings.
1. Attribution language is the largest source of implicit causal drift.
2. Freshness updates are often misread as outcome evidence unless caveated.
3. Publication credibility can be misused to imply efficacy without measured proof.

3. Open questions (if any).
- Should causal-claim guard checks be hard-blocking at `draft`, or only at `approved` transition?

4. Explicit non-goals.
- No causal model design.
- No measurement framework implementation.
- No executable edits by White.

---

- run_id: run_2026-02-10_001
- timestamp_utc: 2026-02-10T23:12:01Z
- definitions: |
    | metric_term_id | term | constrained meaning | mandatory caveat phrase | disallowed overstatement |
    |---|---|---|---|---|
    | MET-001 | lift | Relative change versus baseline under stated window and segmentation; not universal effect size. | "Lift is relative to [baseline/window] and may not generalize beyond this scope." | "Lift proves broad performance superiority." |
    | MET-002 | efficiency | Resource-to-outcome ratio within model assumptions and measurement constraints. | "Efficiency reflects current assumptions, input quality, and channel mix limits." | "This is the most efficient strategy, period." |
    | MET-003 | causal_effect | Causal phrasing allowed only with explicit method, counterfactual framing, and uncertainty interval. | "Causal interpretation uses [method] with uncertainty bounds [range]." | "This caused the result" without method/uncertainty |
    | OP-001 | trust_delta_prompt | Plain-language prompt linking confidence label changes to operator action changes. | "Confidence moved [from->to] because [reason]; next safe action is [action]." | "Confidence changed; proceed as usual." |
- fact_checks:
    - CR-0069-BLUE scope (metric anti-overstatement terminology + caveats) is fully addressed by MET-001..MET-003.
    - CR-GREEN-0011 scope (compact operator caveat templates + trust-delta prompts) is fully addressed by OP-001 template set.
- terminology_corrections:
    - Compact operator caveat templates:
      - "Observed signal supports [action] at [confidence]; uncertainty: [x]."
      - "Mixed-source input present; simulated/scraped signals are advisory, not measured outcomes."
      - "If confidence downgrades, reduce action scope to [safe fallback]."
    - Trust-delta binding rule:
      - Every confidence-label change must include a single-sentence action delta in plain language.
    - Disallow metric absolutism in operator guidance: `proved`, `definitive`, `guaranteed win`, `best possible`.
- change_requests:
    - CR-WHITE-0010: qa_fixer to implement MET-001..MET-003 wording checks and required caveat token presence in metric narrative blocks.
    - CR-WHITE-0011: qa_fixer to implement OP-001 trust-delta prompt enforcement on confidence-label transitions in operator-facing outputs.
- references:
    - planning/BLUE_MASTER_CONTRACT_MAP_2026-02-11.md
    - pipeline/03_green_output.md
    - pipeline/05_white_output.md
    - teams/white/spec.md

1. Summary (<= 300 words).
White fulfilled CR-0069-BLUE and CR-GREEN-0011. This addendum introduces anti-overstatement terminology constraints for metric narratives (`lift`, `efficiency`, and `causal_effect`) with mandatory caveat phrases, and compact operator templates that bind confidence-label shifts to clear action deltas. The goal is to keep operator guidance plain, bounded, and non-inflationary while preserving trust.

2. Numbered findings.
1. Metric narratives overstate most often when baseline/window scope is omitted.
2. Efficiency language frequently hides assumption dependence unless explicitly caveated.
3. Confidence labels are operationally weak unless paired with explicit action-delta prompts.

3. Open questions (if any).
- Should trust-delta prompts be mandatory for all label changes, including `explore -> draft`?

4. Explicit non-goals.
- No metric model redesign.
- No campaign strategy reprioritization.
- No executable edits by White.

---

- run_id: run_2026-02-10_001
- timestamp_utc: 2026-02-10T23:12:53Z
- definitions: |
    | signal_class_id | signal class | operator-handoff wording | mandatory caveat |
    |---|---|---|---|
    | SIG-001 | observed | "Observed signal from production measurement" | "Represents measured behavior within stated window/scope." |
    | SIG-002 | scraped_first_party | "Scraped first-party context signal" | "Context only; not direct customer behavior measurement." |
    | SIG-003 | simulated | "Simulated planning signal" | "Scenario support only; excluded from measured-outcome claims." |
    | SIG-004 | connector_derived | "Connector-derived advisory signal" | "Advisory until typed-contract validation and provenance checks pass." |
- fact_checks:
    - CR-GREY-0004 asks for integration-language addendum distinguishing signal classes with mandatory caveats; SIG-001..SIG-004 satisfy this.
- terminology_corrections:
    - All operator prompts/handoffs must include: `signal_class`, `confidence_label`, `caveat_sentence`, `action_scope`.
    - Prohibit merged unlabeled phrasing such as "data suggests" when multiple signal classes are present.
    - Require per-signal-class caveat if more than one class contributes to the same recommendation.
- change_requests:
    - CR-WHITE-0012: qa_fixer to enforce SIG-001..SIG-004 template presence and mandatory caveat tokens in operator prompts/handoff artifacts.
- references:
    - pipeline/06_grey_output.md
    - pipeline/03_green_output.md
    - pipeline/05_white_output.md
    - teams/white/spec.md

1. Summary (<= 300 words).
White fulfilled CR-GREY-0004 by adding an integration-language addendum for operator prompts and handoffs. The addendum cleanly distinguishes observed, scraped, simulated, and connector-derived signals and requires explicit caveat wording for each class. It also enforces structured fields that make handoffs readable and auditable.

2. Numbered findings.
1. Unlabeled blended signal language is the primary source of operator confusion.
2. Connector-derived signals need explicit advisory status until typed validation passes.
3. Mixed-signal recommendations require class-by-class caveat sentences to avoid inference leakage.

3. Open questions (if any).
- Should prompts hard-fail when any required field is missing, or warn at `draft` and fail at `approved`?

4. Explicit non-goals.
- No workflow ownership changes.
- No connector implementation design.
- No executable edits by White.

---

- run_id: run_2026-02-10_001
- timestamp_utc: 2026-02-10T23:14:58Z
- definitions: |
    | clarity_id | element | canonical meaning | required expression |
    |---|---|---|---|
    | RQ-CL-001 | red_trigger | A trigger is a machine-checkable condition that must include threshold + owner + escalation state. | `if <condition> then <state> by <owner>` |
    | RQ-CL-002 | red_fail_state | Fail state vocabulary is constrained to `blocked`, `limited`, `review_only` only. | One of the three literals per finding |
    | RQ-CL-003 | qa_evidence_unit | Each implemented control must map to one executable check and one persisted artifact row. | `validator_command` + `artifact_path` |
    | RQ-CL-004 | qa_done_boundary | `done` means reproducible pass in current run context, not prose-only rationale. | command output + file evidence |
- fact_checks:
    - Red open workload is large and semantically overlapping; risk findings repeatedly describe the same five control lanes.
    - QA open workload is mostly validator implementation and can be executed faster with a fixed evidence tuple format.
- terminology_corrections:
    - Red findings must avoid mixed wording (`warn/block/fail`) and use only `blocked|limited|review_only` state outcomes.
    - Red triggers must include numerical or boolean thresholds; avoid qualitative-only phrasing such as "high risk".
    - QA logs must use a fixed tuple per request: `request_id`, `validator`, `artifact`, `residual_risk`.
    - Both teams should reference signal classes only as `observed`, `scraped_first_party`, `simulated`, `connector_derived`.
- change_requests:
    - CR-WHITE-0013: Red to publish a normalized trigger catalog for current open Red tickets using `if <condition> then <state> by <owner>` form and the three-state vocabulary.
    - CR-WHITE-0014: qa_fixer to publish an execution matrix mapping each open QA request to exact validator command(s) and required artifact path(s), then use it as completion rubric.
- references:
    - pipeline/02_red_output.md
    - pipeline/07_qa_fix_log.md
    - data/team_ops/change_request_queue.csv
    - teams/white/spec.md

1. Summary (<= 300 words).
Big-picture White clarity pass focused on Red and qa_fixer execution quality. This addendum standardizes trigger language, fail-state vocabulary, and evidence expectations so Red findings convert cleanly into QA-implementable controls. The core simplification is one shared contract: Red emits deterministic trigger statements and state outcomes; QA closes requests only with reproducible validator output plus persisted artifacts.

2. Numbered findings.
1. Red outputs are strong but semantically dense; trigger normalization will reduce duplicate interpretation work.
2. QA throughput is constrained by inconsistent evidence granularity across requests.
3. A shared state vocabulary (`blocked|limited|review_only`) removes downstream ambiguity.
4. Fixed evidence tuples make `done` status auditable and comparable across tickets.

3. Open questions (if any).
- Should QA reject any Red request that does not provide thresholded trigger statements in canonical form?

4. Explicit non-goals.
- No reassignment of authority.
- No implementation code edits by White.
- No strategy changes.

---

- run_id: run_2026-02-10_001
- timestamp_utc: 2026-02-10T23:16:02Z
- definitions: |
    | fallback_id | state communication constraint | required operator copy | forbidden interpretation |
    |---|---|---|---|
    | FB-001 | action_blocked | "Action is blocked pending [specific condition]; no publish/execution permitted." | "Proceed with caution" |
    | FB-002 | action_limited | "Action is limited to [scope] with [explicit caveat]; external publication is not approved." | "Conditionally approved" |
    | FB-003 | action_review_only | "Action is review-only; decision evidence is incomplete and execution is not authorized." | "Soft approval" |
    | FB-004 | trust_delta_binding | Every state change must include one-sentence action delta and one-sentence uncertainty delta. | "State changed" without operational delta |
    | FB-005 | integration_health_indicator | Indicators must be measurable and caveated: fallback_usage_rate, bypass_incidence, trust_delta_compliance_quality. | Raw KPI without caveat context |
- fact_checks:
    - CR-0075-BLUE requires fallback-state language constraints; FB-001..FB-004 satisfy this requirement.
    - CR-0078-BLUE requires measurable integration-health indicators with caveats; FB-005 provides canonical indicator set.
    - CR-GREEN-0014 requires normalized action-state/trust-delta templates with fail examples; this addendum includes both.
- terminology_corrections:
    - Mandatory normalized templates:
      - `action_blocked`: "Blocked because [condition]. Next review trigger: [trigger]."
      - `action_limited`: "Limited to [scope]. Disallowed actions: [list]. Caveat: [uncertainty]."
      - `action_review_only`: "Review only. Missing evidence: [list]. No execution until [trigger]."
      - `trust_delta`: "State moved [from->to] due to [reason]; operator action changes to [action]."
    - Integration-health caveat rule:
      - Any indicator report must include denominator/time-window caveat and one uncertainty note.
- change_requests:
    - CR-WHITE-0015: qa_fixer to enforce fallback-state template compliance (FB-001..FB-004) and health-indicator caveat requirements (FB-005) in operator-facing artifacts.
- references:
    - planning/BLUE_MASTER_CONTRACT_MAP_2026-02-11.md
    - pipeline/03_green_output.md
    - pipeline/05_white_output.md
    - teams/white/spec.md

1. Summary (<= 300 words).
White fulfilled CR-0075-BLUE, CR-0078-BLUE, and CR-GREEN-0014 with a unified fallback-state communication contract. It prevents limited/review-only states from being misread as approved outcomes, standardizes trust-delta language, and introduces measurable integration-health indicators with mandatory caveats. This is designed to reduce operator ambiguity while keeping control boundaries explicit under high-friction workflows.

2. Numbered findings.
1. Most misuse happens when constrained states are described with approval-adjacent wording.
2. Trust-delta prompts without action/uncertainty deltas are operationally non-actionable.
3. Integration-health metrics can mislead unless denominator/window caveats are mandatory.

3. Open questions (if any).
- Should `action_limited` ever allow external publication with override, or remain owned-channel-only by default?

4. Explicit non-goals.
- No fallback-state policy ownership changes.
- No execution-gate implementation by White.
- No strategy rewrite.
