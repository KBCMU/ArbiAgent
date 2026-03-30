# Project Research Summary

**Project:** ArbiAgent — Improved Sports Event Matching (Polymarket + Kalshi)
**Domain:** Cross-platform prediction market arbitrage; sports event entity resolution
**Researched:** 2026-03-29
**Confidence:** HIGH (all four research files grounded in direct codebase analysis)

---

## Executive Summary

ArbiAgent's matching pipeline in `backend-rust/src/matching/` is structurally sound. The multi-signal scorer, Hungarian optimal assignment, cross-bucket fallback, and label resolver are all implemented and fundamentally correct. The three user-visible defects — duplicate events, missed cross-platform matches (events on both platforms showing only one platform's odds), and incorrect dates — are not design failures. They are an accumulation of specific, locatable bugs in the pipeline's lower stages that compound into each other. Each symptom has a traceable root cause and a targeted fix.

The recommended approach is surgical repair in strict dependency order. The cascade runs: unstable event IDs produce cache duplicates; bad date extraction causes bucket misses that produce false negatives; missing sport context in the label resolver produces garbled outcome labels that suppress arb signals even on correctly matched events. These failures are not independent — fixing IDs before fixing dates is required because date instability is a primary cause of ID churn. Deduplication logic installed before the cache merge upgrade is in place will not hold across cycles. The order of operations matters as much as the individual fixes.

The primary risk is scope creep. Several improvements are real and worth doing — weight recalibration, college team dictionary expansion, title-similarity tuning, playoff market detection — but none are blocking the three core defects. Building them before the foundational fixes are stable will confound debugging and make it impossible to measure whether coverage is actually improving. The research is unanimous: fix duplicates first, then date extraction, then label alignment, then expand coverage.

---

## Key Findings

### Recommended Stack

No stack changes are needed. One new dependency is warranted: `strsim = "0.11"` (add to `backend-rust/Cargo.toml`) — the canonical Rust string-metrics crate used by Cargo itself for "did you mean?" suggestions. It provides Jaro-Winkler (rewards common prefixes — ideal for "Boston" vs "BOS") and normalized Levenshtein for fuzzy team-name fallback and partial-credit scoring. No external data providers, ML models, or additional services.

Several high-quality signals already in the API response structs are unused and represent the highest-leverage improvements requiring no new dependencies:

- `KalshiMarketNested.yes_sub_title` / `no_sub_title` — currently `#[allow(dead_code)]` in `fetcher_kalshi.rs`; contain human-readable team names ("Los Angeles Lakers", "Boston Celtics") that would eliminate abbreviation-resolution failures in many cases without any dictionary lookup
- `GammaEvent.start_date` (`startDate`) — exists and must be unconditionally preferred over `end_date`; currently not consistently prioritized
- `KalshiEvent.series_ticker` — use as sport/league discriminator in ambiguous cases
- `Polymarket tags[].slug` — underutilized as market-type and sport confirmation signal (e.g., "nba-playoffs" for playoff detection)

**Core technologies:**
- `Rust / Axum / Tokio` — existing backend pipeline, no change
- `strsim = "0.11"` — Jaro-Winkler + normalized Levenshtein for fuzzy scoring
- `Polymarket Gamma API + Kalshi REST API` — sole data sources; paid providers are explicitly out of scope
- `futures::stream::buffer_unordered(10)` — parallelize the label resolver's currently sequential HTTP requests (already available in the Tokio runtime; ~50 sequential requests per cycle is the only real latency hotspot)

**What NOT to add:** no Elasticsearch, no Redis, no probabilistic blocking (LSH/MinHash is overkill at <200 candidates), no pre-trained NLP models, no external sports schedule APIs.

### Expected Features

**Must have — currently broken or incomplete (fix these first):**
- Stable canonical event ID — `build_event_id()` must be deterministic across cycles; current churn is the root of most duplicate bugs
- Authoritative date extraction hierarchy per platform — inconsistent across fetchers; causes bucket key mismatches and wrong dates in the UI
- Per-slug deduplication of Polymarket candidates — `extract_moneyline_market()` can emit two `CandidateEvent`s for one game (moneyline path and grouped-market path both fire); this is the primary source of the "duplicate events" UI bug
- Cache merge logic — `StateCache.events.insert()` silently overwrites; a new cycle that fails to match an event degrades a previously dual-platform event back to single-platform, making arb detection see zero opportunities

**Must have — working but needing hardening:**
- Sport-scoped abbreviation disambiguation — `lookup_team()` accepts `Option<Sport>` but `match_label_to_abbrev()` in `label_resolver.rs` does not pass sport context; causes false cross-sport matches on ATL, CHI, DET, MIA, PHI, CLE, HOU, DAL, WAS
- Prop and futures market filtering — `is_futures_event()` in `fetcher_polymarket.rs` catches championship futures but not player prop markets; props leak into the moneyline matching bucket and produce garbled arb signals
- Spread line extraction correctness — `extract_line_from_ticker()` in `fetcher_kalshi.rs` returns `0.0` on parse failure instead of `Option<f64>`; two spread markets at -3.5 and -6.5 both map to `Spread(0)` and incorrectly collapse into the same bucket

**Should have — implemented, validate coverage:**
- Cross-bucket fallback matching — implemented in `matcher.rs`; validate it catches remaining false-negative class after date fixes land
- `DateConfidence` metadata on `CandidateEvent` — not yet implemented; explicit `{ TickerParsed, ApiStartDate, ApiEndDate, TitleExtracted, Unknown }` enum makes date-source priority explicit, testable, and observable
- `UnmatchedDetail` diagnostics exposed via API — data already exists in `debug!` logs; surface through `GET /api/v2/matching-stats` for production visibility

**Defer:**
- College team dictionary expansion (CBB/CFB) — correct direction, blocked by foundational fixes
- Scoring weight recalibration — direction is right; specific values require empirical validation against clean production logs post-Phase-3
- `match_score` surfaced in frontend — data exists in `CanonicalEvent.match_score`; straightforward once pipeline is stable
- Shadow comparator re-activation (`shadow.rs`) — DomeAPI non-functional; module exists but no reference available

**Feature dependency chain (must follow this order):**
```
Sport classification → team dictionary lookup (sport scopes it)
Team normalization → stable event ID (sorted canonical abbrevs used in ID)
Date extraction → primary bucket key construction (date is bucket component)
Stable event ID → cache deduplication + Supabase upsert correctness
Outcome label alignment → arb detection + frontend odds display
```

### Architecture Approach

The pipeline is a linear 8-stage Tokio task (`run_sports_matching_loop`). Four structural gaps cause the known defects:

1. No post-assignment deduplication stage between Stage 6 (event construction) and Stage 8 (cache upsert)
2. No single authoritative date resolution hierarchy enforced end-to-end; date comes from different fields across fetchers with no `DateConfidence` tracking
3. Cache upsert overwrites rather than merges — silent downgrade from dual-platform to single-platform events
4. Team normalization embedded redundantly in both fetchers with no shared diagnostic layer; `None` from failed resolution silently zeroes out the team scoring signal

Fixing all four requires adding two new modules (`matching/dedup.rs`, `matching/normalizer.rs`), modifying one function in `storage/cache.rs`, and adding a `DateConfidence` enum to `candidate.rs`. No existing module boundaries change.

**Major components and responsibilities:**
1. `fetcher_kalshi.rs` / `fetcher_polymarket.rs` — raw API ingestion and candidate construction; date priority hierarchy must be enforced here before candidates are emitted
2. `matcher.rs` — bucketing, `score_pair()`, Hungarian assignment (`hungarian.rs`), event construction; new `dedup.rs` stage inserted before cache write
3. `label_resolver.rs` — 3-pass Polymarket token → outcome abbreviation resolution; needs sport context threaded through `resolve_polymarket_labels()` → `match_label_to_abbrev()`
4. `storage/cache.rs` (upsert path) — replace `events.insert()` with merge function that preserves higher-quality (dual-platform, higher `match_score`) records
5. `team_dictionary.rs` — static keyword-to-abbreviation map; extend with `sub_title` parsing; seasonal validation needed

**Architectural invariant to preserve:** Only `run_sports_matching_loop` writes to `StateCache.events`. `arb_detector`, `api/routes.rs`, and `snapshot_writer` are read-only consumers. This invariant must not change in any refactor.

### Critical Pitfalls

1. **Event ID instability (Pitfall 5 — foundational, fix first)** — `build_event_id()` depends on extracted fields that flip between cycles (which date field is non-null, which team abbreviation path fires, "unk" fallback on extraction failure). New ID = new cache row = growing duplicates until eviction. Fix: anchor ID on `kalshi_event_ticker` as the deterministic component (e.g., `kalshi-KXNHLGAME-26MAR15BOSNJD`). Must be resolved before all other dedup work or fixes will be masked.

2. **Polymarket multi-candidate slug emission (Pitfall 4 — highest user-visible impact)** — `extract_moneyline_market()` and the grouped-market path can both fire for the same Gamma event, emitting two `CandidateEvent`s for one game. Hungarian assigns one; the other becomes a spurious single-platform event. Fix: deduplicate by `polymarket_slug` after `process_gamma_events()`, one candidate per slug per market type.

3. **Date source confusion (Pitfalls 1 and 2 — largest false-negative class)** — Kalshi `close_time` is a trading deadline (not game time); late-night games (9–10 PM ET) roll into the next UTC day, splitting same-day games across bucket keys. Polymarket `end_date` is market expiry, which can be 2–3 days after the game. Fix: Kalshi — ticker-extracted date is canonical, `close_time` is last resort; Polymarket — `startDate` unconditionally preferred, slug-parsed date second, `end_date` last with `warn!` log.

4. **Cross-sport abbreviation collision in label resolver (Pitfall 3)** — ATL, CHI, DET, MIA, PHI and others exist in multiple sports. `match_label_to_abbrev()` in `label_resolver.rs` has no sport parameter and resolves to the first dictionary entry found, producing cross-sport false matches. Fix: thread `Sport` through `resolve_polymarket_labels()` into `match_label_to_abbrev()`; add unit tests for every multi-sport abbreviation.

5. **Cache overwrite silently degrading dual-platform events (Architecture Gap 3)** — A subsequent matching cycle that fails to pair an event produces a single-platform candidate; `events.insert()` silently replaces the previously dual-platform event. Arb detector then finds no dual-platform events. Fix: merge logic — if incoming is single-platform and cached is dual-platform with higher `match_score`, preserve the cached record.

6. **Spread line silent failure (Pitfall 10 — silent correctness issue)** — `extract_line_from_ticker()` returns `0.0` on parse failure instead of `Option<f64>`. Two spread markets at different lines both map to `MarketTypeBucket::Spread(0)` and incorrectly match against each other. Fix: return `Option<f64>`; skip emitting the spread candidate when the line cannot be parsed; add unit test for underscore-decimal format (`3_5` → 3.5).

---

## Implications for Roadmap

The dependency chain is strict and non-negotiable. Build in this order.

### Phase 1: Foundational Stability — Event IDs and Deduplication

**Rationale:** All duplicate-event symptoms trace to two root causes: ID instability and multi-candidate slug emission. Any other dedup work built on top of unstable IDs will be invisible or counterproductive. Resolving these first gives every subsequent phase a clean, non-growing baseline to verify against.

**Delivers:** Zero duplicate events in the UI; stable IDs across matching cycles; no more cache row proliferation between eviction windows; cache merge prevents dual-platform event downgrade.

**Addresses:** Stable canonical event ID (table stakes), deduplication completeness (table stakes).

**Avoids:** Pitfall 4 (multi-candidate slug), Pitfall 5 (ID instability), Pitfall 9 (stale odds attached to dead IDs).

**Specific changes:**
1. Fix `build_event_id()` — anchor on Kalshi event ticker, not extracted fields
2. Add slug-level deduplication in `fetcher_polymarket.rs` after `process_gamma_events()`, one candidate per slug per market type
3. Add `matching/dedup.rs` — post-construction dedup stage grouping by (sport, sorted-team-pair, ±1 day date window); keep dual-platform over single-platform, higher `match_score` within same category
4. Upgrade `cache.rs::upsert_event()` — merge-not-overwrite; preserve dual-platform and higher `match_score`

**Research flag:** Standard patterns. No additional research needed.

---

### Phase 2: Date Extraction Reliability

**Rationale:** Date is the primary bucket key. Wrong dates cause primary bucket misses, pushing events into the date-relaxed cross-bucket fallback where scores are lower and false positive risk is higher. This phase fixes the upstream cause of the largest class of missed matches. Must come after Phase 1 because date fixes change the inputs to `build_event_id()` — without stable ID logic first, date fixes just produce different unstable IDs.

**Delivers:** Correct game dates in the UI; reduced cross-bucket fallback volume; accurate dates in canonical event IDs; `DateConfidence` metadata enabling observable date-source diagnosis.

**Addresses:** Date extraction reliability (table stakes), `DateConfidence` metadata visibility.

**Avoids:** Pitfall 1 (Kalshi `close_time` as game date), Pitfall 2 (Polymarket `end_date` as game date), Pitfall 6 (dateless events penalized by 20-point date miss).

**Specific changes:**
1. Add `DateConfidence` enum to `candidate.rs` with variants `TickerParsed`, `ApiStartDate`, `ApiEndDate`, `TitleExtracted`, `Unknown`
2. Fix `fetcher_kalshi.rs` — ticker-extracted date canonical (`extract_date_from_kalshi_ticker()`); `close_time` is fallback labeled `ApiEndDate`
3. Fix `fetcher_polymarket.rs` — priority: slug date (`parse_date_from_slug()`) → `startDate` → title extraction → `end_date` with `warn!` log
4. Evaluate removing date from primary bucket key vs. widening to ±1 day tolerance (see Research Flags)
5. Raise cross-bucket fallback score threshold to ~70 — date uncertainty in the fallback pass increases false-positive risk (Pitfall 14)

**Research flag:** MEDIUM — the bucket key change (remove date entirely vs. ±1 day window) requires empirical validation against production logs for back-to-back game false-positive risk (MLB doubleheaders, NBA/NHL playoff series). Validate before committing.

---

### Phase 3: Label and Abbreviation Alignment

**Rationale:** With stable IDs (Phase 1) and accurate dates (Phase 2), label failures become clearly attributable rather than masked by matching failures. Events that are correctly matched but show "Yes/No" instead of team names are now diagnosable as label-resolver failures, not bucket misses. This phase also closes the prop-market leakage and spread-line bugs.

**Delivers:** Outcome labels correctly mapped to team abbreviations in all dual-platform events; no more "Yes"/"No" where team names should appear; arb signals generated correctly; spread lines in the UI reflect actual market lines.

**Addresses:** Outcome label alignment (table stakes), sport-scoped team disambiguation, prop/futures filtering.

**Avoids:** Pitfall 3 (cross-sport abbreviation collision), Pitfall 7 (prop market leakage), Pitfall 10 (spread line silent failure).

**Specific changes:**
1. Thread `Sport` through `resolve_polymarket_labels()` into `match_label_to_abbrev()` in `label_resolver.rs`
2. Parse `yes_sub_title` / `no_sub_title` from Kalshi API response as direct team-name source (currently `#[allow(dead_code)]`)
3. Add prop-market detection in `fetcher_kalshi.rs` — flag tickers containing `POINTS`, `REBOUNDS`, `ASSISTS`, `YARDS`, `TOUCHDOWNS` as `MarketType::Prop`, exclude from cross-platform matching
4. Extend `is_futures_event()` in `fetcher_polymarket.rs` to detect player-name patterns in titles
5. Fix `extract_line_from_ticker()` — return `Option<f64>`, skip spread candidate when line unparseable; add unit test for `3_5` → 3.5
6. Add unit tests for all multi-sport abbreviation collisions: ATL, CHI, DET, MIA, PHI, CLE, HOU, DAL, WAS

**Research flag:** Standard patterns. No additional research needed.

---

### Phase 4: Coverage Expansion and Observability

**Rationale:** With a stable, accurate baseline from Phases 1–3, coverage improvements can be measured cleanly. False-negative rates are now attributable to dictionary gaps and scoring limits rather than upstream bugs. The `strsim` integration goes here — not earlier — so fuzzy thresholds can be tuned against real match data.

**Delivers:** Reduced false-negative rate; college sports (CBB/CFB) partially supported; scoring weights empirically calibrated; matching pipeline health visible via the existing `GET /api/v2/matching-stats` endpoint.

**Addresses:** Differentiator features — `strsim` fuzzy fallback, college team dictionary expansion, `UnmatchedDetail` API exposure, label resolution parallelism.

**Avoids:** Pitfall 11 (franchise renames missed in dictionary), Pitfall 12 (Jaccard penalizing verbose Polymarket titles), Pitfall 13 (Polymarket pagination truncation during high-volume tournaments).

**Specific changes:**
1. Add `strsim = "0.11"` to `backend-rust/Cargo.toml`; integrate `jaro_winkler` as fallback in `compute_team_score()` (threshold ~0.85); add `normalized_levenshtein` soft match in dictionary lookup (threshold ~0.80)
2. Expand `team_dictionary.rs` with NCAA D1 CBB and Power 4 CFB programs (~370 entries); add recent franchise renames (Utah Hockey Club, etc.)
3. Extract team normalization into shared `matching/normalizer.rs` returning `NormalizationResult { candidate, unresolved_tokens }` — eliminates redundant logic in both fetchers, adds diagnostic visibility
4. Recalibrate scoring weights from production log analysis — specifically evaluate `WEIGHT_DATE_MATCH_STRONG = 28.0` for dual-source date confirmation; adjusted cross-bucket threshold (55 or 70)
5. Parallelize label resolver using `futures::stream::iter(...).buffer_unordered(10)` (eliminates ~50 sequential HTTP requests per cycle)
6. Surface `UnmatchedDetail` best-rejected scores via `GET /api/v2/matching-stats`; add `unresolved_teams: usize`, `deduped: usize`, `date_confidence_distribution` to `MatchingStats`
7. Increase Polymarket fetcher `max_pages` to 20 for high-volume tournament periods; log when last page is full (Pitfall 13)
8. Add CI dictionary validation asserting expected team counts per sport in `team_dictionary.rs`

**Research flag:** Fuzzy threshold values (0.85 for Jaro-Winkler, 0.80 for normalized Levenshtein) are MEDIUM confidence — directionally correct but require controlled validation against known-correct and known-incorrect pairs from production logs before deployment. College sports dictionary scope should be confirmed against active markets on both platforms; Power 4 programs appear more frequently than mid-majors.

---

### Phase Ordering Rationale

- **Phase 1 before Phase 2:** Date fixes change inputs to `build_event_id()`. Without stable ID logic first, date fixes just produce differently-unstable IDs and the same cache churn.
- **Phase 2 before Phase 3:** False-negative label failures are indistinguishable from date-bucket misses until dates are reliable. You cannot tell if missing odds is a label failure or a matching failure.
- **Phase 3 before Phase 4:** Weight recalibration and dictionary expansion require a clean, measured baseline. Tuning against a buggy baseline produces wrong weights that will need re-tuning again.

User-visible impact by phase:
- Phase 1: Fixes the most jarring bug — visible duplicates in the UI
- Phase 2: Fixes the most common miss — events on both platforms showing only one platform's odds
- Phase 3: Fixes the silent failure — correct matches with no arb signal due to "Yes/No" label corruption
- Phase 4: Expands coverage and reduces residual false-negative rate

### Research Flags

Needs empirical validation during planning:
- **Phase 2 — Bucket key change:** Removing date from primary bucket key vs. widening to ±1 day window. STACK.md argues for removal (scorer's date signal handles it); PITFALLS.md warns about back-to-back same-sport games producing false positives. Resolve from production log analysis before committing.
- **Phase 4 — Scoring weight recalibration:** Directionally correct per research, but specific numeric values (`WEIGHT_DATE_MATCH_STRONG`, adjusted `MIN_SCORE` for cross-bucket pass) must be tuned against real production data after Phase 3 baseline is established.

Standard patterns — no additional research needed:
- **Phase 1:** Slug dedup, ID anchoring to platform ticker, merge-not-overwrite cache logic are all standard deterministic patterns.
- **Phase 3:** Sport-context threading through label resolver and prop-market filtering are well-scoped refactors with clear test cases.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Single crate addition (`strsim`); all other recommendations are intra-codebase logic changes confirmed against current files |
| Features | HIGH | All findings from direct code analysis and documented production failures in `tasks/lessons.md`; no inference |
| Architecture | HIGH | All structural gaps identified from file inspection; file names, function names, and stage positions confirmed |
| Pitfalls | HIGH | All 14 pitfalls grounded in production code, observed bugs, and `tasks/lessons.md` entries; no speculative risks |

**Overall confidence:** HIGH

### Gaps to Address

- **Bucket key date tolerance (Phase 2):** MEDIUM — whether to remove date from primary bucket key entirely vs. widen to ±1 day needs production log validation for back-to-back same-sport game false-positive risk.

- **Scoring weight recalibration values (Phase 4):** Research correctly identifies direction (`WEIGHT_DATE_MATCH_STRONG`, lower cross-bucket threshold) but specific values require production log analysis after Phase 3 is stable. Do not commit specific numbers upfront.

- **College sports active market scope (Phase 4):** CBB/CFB dictionary expansion is warranted, but confirm which programs are actively traded on both platforms during the milestone window before investing in full NCAA D1 expansion (~370 teams). Power 4 CBB programs are the practical starting point.

- **Kalshi `sub_title` parsing reliability:** `yes_sub_title` / `no_sub_title` in `KalshiMarketNested` are currently dead code. Evaluate actual coverage (what percentage of Kalshi events populate these fields) during Phase 3 implementation before relying on them as the primary team-name source.

---

## Sources

### Primary (HIGH confidence — direct codebase analysis)
- `backend-rust/src/matching/matcher.rs` — scoring weights, bucket strategy, Hungarian assignment, cross-bucket fallback, `build_event_id()`
- `backend-rust/src/matching/candidate.rs` — `CandidateEvent`, date extraction, team extraction, `jaccard_token_similarity`
- `backend-rust/src/matching/fetcher_kalshi.rs` — Kalshi ticker parsing, `close_time` usage, spread line extraction, `sub_title` dead code
- `backend-rust/src/matching/fetcher_polymarket.rs` — Gamma API pagination, grouped market handling, `parse_date_from_slug()`, `is_futures_event()`
- `backend-rust/src/matching/label_resolver.rs` — 3-pass token → label resolution; sport-context absence in `match_label_to_abbrev()`
- `backend-rust/src/matching/team_dictionary.rs` — multi-sport abbreviation collision entries, `lookup_team(phrase, Option<Sport>)`
- `backend-rust/src/matching/hungarian.rs` — assignment algorithm
- `backend-rust/src/matching/shadow.rs` — shadow comparison structure
- `tasks/lessons.md` — recorded production failures (Utah HC/ARI rename, Polymarket pagination limits, grouped event structure, `startDate` usage)
- `.planning/PROJECT.md` — active requirements, known gaps, project constraints

### Secondary (HIGH confidence — ecosystem knowledge)
- `strsim` crate (crates.io) — canonical Rust string metrics library; used by Cargo; maintained through 2025; Jaro-Winkler and normalized Levenshtein confirmed in v0.11

---

*Research completed: 2026-03-29*
*Ready for roadmap: yes*
