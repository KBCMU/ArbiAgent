---
focus: features
generated: 2026-03-29
domain: cross-platform prediction market sports event matching
system: ArbiAgent (Polymarket + Kalshi)
---

# Feature Landscape: Cross-Platform Sports Event Matching

**Domain:** Prediction market arbitrage — matching the same sporting event across Polymarket and Kalshi
**Researched:** 2026-03-29
**Confidence:** HIGH (based on exhaustive codebase analysis + entity-matching domain knowledge)

---

## Table Stakes

Features the matching pipeline must have. If any of these fails, matches are wrong, missing, or duplicated.

| Feature | Why Expected | Complexity | Current Status | Notes |
|---------|--------------|------------|----------------|-------|
| Team name normalization (abbrev dictionary) | Both platforms use different names for the same team ("Los Angeles Lakers" vs "LAL" vs "lakers"). Without a shared canonical form, team-match scoring returns 0 for identical events. | Medium | Implemented (`team_dictionary.rs`) | Needs to stay current with relocations/renames (e.g., Utah Hockey Club) |
| Sport classification | Events must be bucketed by sport before scoring. A cross-sport match (NBA vs NFL) is always wrong and wastes compute. | Low | Implemented | Kalshi uses series ticker prefixes; Polymarket uses Gamma tags |
| Date extraction from heterogeneous sources | Kalshi encodes dates in tickers (`26MAR29`); Polymarket puts dates in `endDate`/`startDate` API fields and sometimes slugs. Without a reliable date, bucket-key collisions occur across different days' games. | Medium | Implemented, known gaps | Kalshi `close_time` (ISO-8601) is a reliable fallback. Slug date parsing exists. |
| Market type bucketing (moneyline / spread / total) | A Lakers vs Celtics moneyline should never be matched against a Lakers vs Celtics spread. Without market type separation, one match absorbs both and the spread goes unmatched. | Medium | Implemented (`MarketTypeBucket` in `candidate.rs`) | Spread/total lines encoded as millicents for hashable equality |
| Minimum score threshold | Without a floor (currently 60.0), high false-positive matches produce garbage pairs and waste odds ingestion capacity. | Low | Implemented | Threshold is configurable via `min_score` param |
| Single-platform event passthrough | Events that exist on only one platform are still valid markets. They must appear in the UI with one set of odds rather than being silently dropped. | Low | Implemented (`build_single_platform_event`) | Critical for completeness requirement |
| Deduplication within a cycle | The same Kalshi or Polymarket event must not produce two CandidateEvents that both match against the same opponent, yielding duplicate CanonicalEvents in cache. | Medium | Partially implemented (slug deduplication in fetcher_polymarket.rs) | Hungarian algorithm prevents many-to-one but slug-level dedup only covers exact slug repeats |
| Outcome label alignment (Yes/No → team abbrevs) | The arb detector compares `Kalshi[LAL]` vs `Polymarket[LAL]`. If Polymarket stored outcome as "Yes"/"No" the comparison fails and no arb is detected even when one exists. | High | Implemented (`label_resolver.rs`, 3-pass strategy) | Known failure mode: grouped markets still resolve to generic labels in some cases |
| Stable canonical event ID | The event ID (`sport-teamA-teamB-YYYY-MM-DD`) must be deterministic across cycles. Non-deterministic IDs cause a cascade: cache thrash, duplicate Supabase rows, incorrect odds history. | Medium | Implemented (teams sorted, consistent format) | `build_event_id` sorts teams alphabetically to prevent `bos-lal` vs `lal-bos` skew |
| Futures / prop event filtering | Championship winner markets ("Who wins the NBA Finals?") must not enter the moneyline matching pipeline. They have >2 outcomes, incompatible structure, and will corrupt score matrices. | Low | Implemented (`is_futures_event` in fetcher_polymarket.rs) | String heuristic; may miss novel phrasings |

---

## Differentiators

Features beyond the baseline that increase match recall, reduce false negatives, and make the system measurably better than a naive title-similarity approach.

| Feature | Value Proposition | Complexity | Current Status | Notes |
|---------|-------------------|------------|----------------|-------|
| Cross-bucket fallback matching | When a Kalshi event has a valid date and a Polymarket equivalent lacks one (or vice versa), primary bucket lookup (`sport + date + market_type`) misses the pair entirely. Cross-bucket fallback (keyed by `sport + market_type` only, scored by teams + title) catches these. | Medium | Implemented | This is a real differentiator; prevents a large class of false negatives where one platform's date is missing |
| Hungarian algorithm for optimal assignment | Greedy first-match assignment allows an inferior pair to "steal" a candidate from its true match. Hungarian maximizes total assignment weight, which prevents this class of error in dense same-day same-sport buckets (e.g., 10 NBA games on one night). | High | Implemented (`hungarian.rs`) | Critical for multi-game nights |
| Bidirectional team alias coverage | Some platforms use city names ("Boston"), nicknames ("Celtics"), abbreviations ("BOS"), or league-qualified forms ("NBA: BOS"). A keyword list per team covering all known aliases ensures match succeeds regardless of source format. | Medium | Implemented | `team_dictionary.rs` has multi-keyword entries per team |
| Timezone-aware date scoring | A game on 2026-03-29 at 11 PM Eastern is 2026-03-30 UTC. Kalshi tickers use Eastern dates; Polymarket `endDate` is often UTC. A ±1-day partial score (75%) rather than binary match prevents false negatives at date boundaries. | Low | Implemented | Score: exact → +20, ±1 day → +15 |
| Multi-signal confidence score surfaced to UI | Exposing `match_score` in `CanonicalEvent` lets the frontend (and future alerting logic) distinguish high-confidence dual-platform pairs from low-confidence ones. Enables a "suspect match" warning tier. | Low | Data exists (`match_score: Option<f64>`) | Not yet shown in frontend; straightforward to surface |
| Diagnostic unmatched detail logging | `UnmatchedDetail` records each unmatched candidate's best-rejected score and the title of the candidate it almost matched. This turns silent miss-rates into actionable debugging data. | Low | Implemented | Currently only logged to `debug!`; not yet exposed via API |
| Shadow comparison against reference matcher | Running DomeAPI in shadow mode and logging overlap metrics validates the native matcher quality without risking production state. Gives a measurable recall benchmark. | Medium | Implemented (`shadow.rs`) | DomeAPI currently non-functional; module exists for future re-activation |
| Grouped market structure support (Polymarket Yes/No per team) | Polymarket increasingly represents moneylines as grouped markets: one Yes/No sub-market per team. Without explicit grouped-market handling, the token→label mapping fails and teams are stored as "Yes"/"No". | High | Implemented (fetcher_polymarket.rs `extract_moneyline_market`) | The biggest real-world edge case; required to support current Polymarket market formats |
| Slug-embedded date parsing | Polymarket slugs often contain the game date (`celtics-vs-lakers-3-29`) even when API date fields are malformed or in a bad timezone. Parsing the slug provides a higher-reliability date source. | Low | Implemented (`parse_date_from_slug`) | Useful fallback, reduces date-miss rate |
| Per-sport team dictionary with sport-scoped lookup | Generic keyword matching across sports creates false positives (e.g., "Panthers" → NFL Panthers vs NFC Panthers vs NHL Panthers). Sport-scoped lookup eliminates this ambiguity. | Medium | Implemented (`lookup_team(phrase, Some(sport))`) | Requires dictionary completeness per sport |

---

## Anti-Features

Things to deliberately not build in this matching milestone. Each has a reason.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| LLM/embedding-based matching | Adds latency (100ms+ per pair vs <1ms), cost, non-determinism, and a network dependency to every matching cycle. Matching runs on a tight loop; probabilistic matching at the pairing step is an unacceptable tradeoff when deterministic multi-signal scoring works. | Keep multi-signal weighted scoring. Invest in dictionary completeness and signal quality rather than model calls. |
| External sports schedule API integration | Pulls in a paid data dependency (e.g., SportsRadar, ESPN API) that may go down, have rate limits, or require keys to rotate. The project constraint is: data sources only from Kalshi + Polymarket REST APIs. | Derive dates from platform API fields (Kalshi tickers, Polymarket `startDate`/`endDate`). These are already authoritative for the prediction market context. |
| Learned/trained team name expansion | A system that dynamically "learns" new team name aliases from observed titles would require training data, a model, and correctness validation. Given a finite set of professional sports teams, a static dictionary is simpler, faster, and safer. | Maintain the static `team_dictionary.rs`. Add entries when new aliases are discovered in production logs. |
| Fuzzy string matching without canonical normalization | Raw Levenshtein/edit-distance on raw titles is brittle: "NBA: LAL vs BOS" and "Lakers vs. Celtics" have high edit distance but are the same game. Title similarity is a tiebreaker signal, not a primary match signal. | Use title similarity (Jaccard token overlap) only as a secondary signal weighted below team matching. Never gate a match solely on title similarity. |
| Storing matched state across cycles as a cache (cross-cycle ID reuse for pairing) | Reusing a prior cycle's match assignments to "pre-seed" the next cycle's pairing would introduce stale-match drift as markets open/close. | Re-run the full scoring and assignment pass each cycle. Stability comes from deterministic IDs, not cached assignment state. |
| Matching prop bets (player performance, player points, etc.) | Props are not currently on either platform in a structured, matchable form. Even if they were, the matching problem (which player X total-points market on Kalshi is the same as player X points on Polymarket?) is a separate, harder problem with different signals. | Filter out any market whose title matches prop-bet heuristics (player names, "points", "assists", "rebounds") before the candidate pipeline. Document in `is_futures_event`-style filter. |
| Per-event human review queue | A UI for a human to manually confirm or reject uncertain matches would slow the pipeline to human speed. Arbitrage windows can be seconds wide. | Trust the threshold. Tune `DEFAULT_MIN_SCORE` based on observed precision/recall tradeoffs. Log low-confidence matches with `warn!` for post-hoc auditing, not real-time review. |

---

## Feature Dependencies

```
Sport classification
  → Market type bucketing (sport determines valid market types)
  → Team name normalization (sport scopes dictionary lookup)
  → Date extraction (sport determines which API fields carry dates)

Team name normalization
  → Primary bucket key construction (team abbrevs used in event ID)
  → Stable canonical event ID (requires sorted canonical abbrevs)
  → Outcome label alignment (Polymarket token → Kalshi abbrev mapping)

Date extraction
  → Primary bucket matching (date is part of BucketKey)
  ← Cross-bucket fallback (compensates when date extraction fails)

Minimum score threshold
  → Hungarian assignment (threshold gates which pairs are accepted)
  → Cross-bucket fallback (same threshold applied in fallback pass)

Stable canonical event ID
  → Cache deduplication (DashMap keyed by ID)
  → Supabase upsert (upsert key; non-stable ID = duplicate rows)
  → Odds ingestion (price refresh loop references event IDs)

Outcome label alignment
  → Arb detection (arb_detector compares outcomes by label)
  → Odds display (frontend shows "LAL: 0.62" vs "Yes: 0.62")

Single-platform event passthrough
  → Completeness requirement (events on one platform must still appear)
  ← Deduplication (single-platform events must not collide with matched pairs)
```

---

## MVP Recommendation

For the "accurate cross-platform sports event matching" milestone, prioritize fixing table stakes failures before adding new differentiators:

**Priority 1 — Fix existing failures (table stakes not working correctly):**
1. Outcome label alignment — the most common root cause of "event on both platforms showing only one platform's odds"
2. Date extraction reliability — incorrect dates cause bucket misses and wrong event IDs
3. Deduplication completeness — duplicate CanonicalEvents in cache directly produce duplicate rows in UI

**Priority 2 — Recall improvements (reduce false negatives):**
1. Cross-bucket fallback is already implemented; validate it catches the remaining unmatched cases by examining `unmatched_details` logs
2. Audit `team_dictionary.rs` completeness for all sports currently active on both platforms
3. Futures/prop filter coverage — ensure no non-game-outcome markets enter the pipeline

**Defer:**
- Surfacing `match_score` in the frontend UI — useful eventually, not blocking accuracy
- Shadow comparison against DomeAPI — DomeAPI is non-functional; no reference to compare against
- LLM-assisted matching — violates constraints and adds complexity

---

## Confidence Assessment

| Area | Confidence | Basis |
|------|------------|-------|
| Table stakes identification | HIGH | Derived from direct codebase analysis + known production failures documented in `tasks/lessons.md` |
| Differentiator assessment | HIGH | Based on observed implementation choices and entity-matching domain patterns |
| Anti-features | HIGH | Grounded in explicit project constraints (no new paid APIs; Rust backend) and failure modes visible in code |
| Feature dependencies | HIGH | Traced through actual call graph in `matcher.rs`, `candidate.rs`, `label_resolver.rs` |

---

## Sources

- `backend-rust/src/matching/matcher.rs` — scoring weights, bucket strategy, Hungarian assignment, unmatched passthrough
- `backend-rust/src/matching/candidate.rs` — `CandidateEvent` struct, normalization, date extraction, team extraction, Jaccard similarity
- `backend-rust/src/matching/fetcher_kalshi.rs` — Kalshi ticker parsing, market type classification, candidate construction
- `backend-rust/src/matching/fetcher_polymarket.rs` — Gamma API pagination, grouped market handling, slug date parsing
- `backend-rust/src/matching/label_resolver.rs` — 3-pass token → label resolution strategy
- `backend-rust/src/matching/team_dictionary.rs` — keyword dictionary structure, sport-scoped lookup
- `backend-rust/src/matching/shadow.rs` — shadow comparison metrics structure
- `tasks/lessons.md` — documented production failure modes (grouped markets, pagination limits, franchise renames, label resolver scope)
- `backend-rust/src/models/event.rs` — `CanonicalEvent`, `PlatformIds`, `Sport` enum (implied by imports)
- `.planning/PROJECT.md` — project constraints, active requirements, out-of-scope boundaries
