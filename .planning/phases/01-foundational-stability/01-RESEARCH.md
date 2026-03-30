# Phase 1: Foundational Stability — Research

**Researched:** 2026-03-29
**Domain:** Rust backend matching pipeline (`backend-rust/src/matching/`, `backend-rust/src/storage/cache.rs`)
**Confidence:** HIGH — all findings are from direct code inspection, no external lookups required

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Fix `build_event_id()` to never produce `"unk"` or `"nodate"` fallbacks in normal operation. When team extraction yields nothing, fall back to poly slug segments or kalshi ticker segments as a tiebreaker to build a stable deterministic ID.
- **D-02:** Add a post-construction dedup pass after `match_candidates()` returns its `Vec<CanonicalEvent>` — collapse any entries with the same (sport, team_a, team_b, date) or same platform slug/ticker into a single entry. Belt + suspenders against edge cases.
- **D-03:** Deduplicate within `process_gamma_events()` itself. After building all candidates for a single event, deduplicate by `(polymarket_slug, market_type_bucket)` before appending to `candidates`. Same-game same-type duplicates are collapsed at the source.
- **D-04:** Replace `self.events.insert()` with "never downgrade" merge logic: if the cached event has both platforms' data and the incoming event has only one platform, keep the cached version. Only overwrite if the incoming event has equal or more platform coverage.
- **D-05:** On cache upsert, check if `polymarket_token_ids` changed. If they differ from the cached value, clear the odds for that event's Polymarket platform. Prevents stale prices from dead tokens being used for arb detection.

### Claude's Discretion
- Exact data structure for the post-construction dedup pass (HashMap keyed by event slug/ticker vs. sort+dedup)
- Whether `upsert_event()` comparison uses a platform-count heuristic or checks presence of non-empty `kalshi_event_ticker` and `polymarket_market_slug`
- Log verbosity for merge/dedup events (debug vs. info)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within Phase 1 scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| R1 | Event IDs must be stable across fetch cycles | D-01 + D-02 address this; slug/ticker segments are already available in CandidateEvent fields |
| R2 | Polymarket must emit exactly one CandidateEvent per game | D-03 addresses this; `MarketTypeBucket` + `polymarket_slug` are the correct dedup key |
| R3 | Cache upsert must use merge semantics, not replace | D-04 + D-05 address this; platform coverage measured via non-empty ticker/slug fields in PlatformIds |
</phase_requirements>

---

## Q1: Test Coverage

### Existing tests for the three target functions

**`build_event_id` — `backend-rust/src/matching/matcher.rs`**

Three dedicated tests exist in the `#[cfg(test)]` block at the bottom of `matcher.rs`:
- `test_event_id_format` (line 638) — asserts `nba-bos-lal-2026-03-14` format with teams present
- `test_event_id_order_invariant` (line 650) — asserts same ID regardless of team order in poly candidate
- `test_event_id_prefers_polymarket_date` (line 665) — asserts poly date wins when kalshi and poly dates differ

All three tests use candidates that have both teams and dates populated. **None of them test the fallback path** (missing team_a/team_b or missing game_date). The `"unk"`/`"nodate"` code paths are unexercised.

**Tests that will break from D-01 fix:** None of the three existing tests use the `"unk"` or `"nodate"` path. They all pass in candidates with real teams and dates. They should continue passing after the fix, assuming slug-fallback IDs still produce the same ID for matched pairs. However `test_event_id_format`, `test_event_id_order_invariant`, and `test_event_id_prefers_polymarket_date` all assert specific ID strings — they will only break if the fix changes how teams/dates are formatted when both are present (it should not).

**New tests needed for D-01:**
- `test_event_id_no_teams_uses_slug_fallback` — kalshi and poly candidates with `team_a=None, team_b=None`; assert the resulting ID contains slug/ticker segments instead of "unk"
- `test_event_id_no_date_uses_slug_fallback` — candidates with `game_date=None`; assert ID contains a date-like segment from slug instead of "nodate"
- `test_event_id_unk_never_appears` — assert that the string "unk" is absent from any ID produced when slug/ticker data is available

**`process_gamma_events` — `backend-rust/src/matching/fetcher_polymarket.rs`**

Tests exist in the `#[cfg(test)]` block at the bottom of `fetcher_polymarket.rs`:
- `test_parse_gamma_date_rfc3339`
- `test_parse_gamma_date_date_only`
- `test_parse_gamma_date_with_offset`
- `test_parse_gamma_date_uses_eastern_day_boundary`
- `test_parse_date_from_slug`
- `test_parse_gamma_date_none`
- `test_classify_sport_nba` (and presumably more classify tests beyond what was read)

**None of these test `process_gamma_events` itself** — they only test the helper functions it calls. The double-emit bug is completely untested.

**Tests that will break from D-03 fix:** None — no test currently exercises `process_gamma_events` directly.

**New tests needed for D-03:**
- `test_process_gamma_events_no_double_emit` — a `GammaEvent` with both a moneyline market and a spread sub-market; assert `process_gamma_events` returns exactly one moneyline candidate and one spread candidate (not two moneylines)
- `test_process_gamma_events_dedup_by_slug_market_type` — two `GammaEvent`s with same slug but different markets; assert no duplicates by `(slug, MarketTypeBucket)`

**`upsert_event` — `backend-rust/src/storage/cache.rs`**

Two tests exist in `cache.rs`:
- `test_evict_stale_no_odds_event` (line 228) — calls `upsert_event` as setup, tests eviction
- `test_keep_fresh_no_odds_event` (line 240) — calls `upsert_event` as setup, tests eviction

Both tests only use `upsert_event` as a data-insertion step. **Neither tests merge semantics.** The tests call `upsert_event` once per event and check eviction logic, not replacement behavior.

**Tests that will break from D-04/D-05 fix:**
- `test_evict_stale_no_odds_event` and `test_keep_fresh_no_odds_event` use `make_event` which creates a single-platform event (all `platform_ids` fields are empty Vecs/None). The new never-downgrade logic only triggers when the cached event is dual-platform. These tests insert single-platform events and then call `upsert_event` once — there is no overwrite scenario. Both tests should continue passing unchanged.

**New tests needed for D-04/D-05:**
- `test_upsert_never_downgrades_dual_to_single` — insert a dual-platform event, then upsert a single-platform event with the same ID; assert the cache still has both platforms' data
- `test_upsert_upgrades_single_to_dual` — insert a single-platform event, then upsert a dual-platform event with same ID; assert dual-platform data is now in cache
- `test_upsert_clears_poly_odds_on_token_change` — insert event with token_ids `["A","B"]`, add polymarket odds, then upsert same ID with token_ids `["C","D"]`; assert polymarket odds are cleared and kalshi odds are preserved

---

## Q2: All Callers of upsert_event

`state.cache.upsert_event(...)` is called from four locations outside of the cache's own test module:

| File | Line(s) | Call Site | Context | Merge Safety |
|------|---------|-----------|---------|--------------|
| `src/matching/mod.rs` | 261, 270 | `discover_and_match_sports()` | Sports matching loop; upserts both dual-platform matched events and single-platform unmatched events | **Primary caller — merge logic must be correct here** |
| `src/ingestion/culture_poller.rs` | 220 | `process_polymarket_culture_events()` | Culture events from Polymarket Gamma API; always single-platform (polymarket_slug set, no kalshi fields) | Safe: always single-platform, no dual-platform collision risk |
| `src/ingestion/culture_poller.rs` | 422 | `process_kalshi_culture_events()` | Culture events from Kalshi API; always single-platform (kalshi fields set, no polymarket fields) | Safe: always single-platform, no dual-platform collision risk |
| `src/ingestion/dome_poller.rs` | 145 | Legacy DomeAPI poller (only active when `enable_native_matching=false`) | Not active in normal operation; uses its own event construction | Safe: not run concurrently with native matcher |

**Key finding:** The culture poller callers are always single-platform — `kalshi_event_ticker=None, polymarket_token_ids=[]` or vice versa. The never-downgrade logic in D-04 must not block single-platform culture events from being inserted or refreshed. The correct check is: only refuse the downgrade if **the cached event is dual-platform** (has both `kalshi_event_ticker.is_some()` AND `polymarket_market_slug.is_some()`). A culture event upsert on a culture event ID will never collide with a sports matching ID (different ID namespaces: `poly-{slug}` vs `nba-{a}-{b}-{date}`), so there is no cross-caller collision risk.

**`matching/mod.rs` line 261 context:** This upsert is inside the label-resolution branch — it fires specifically when labels changed but token IDs are stable. The D-04 never-downgrade logic must not block this label-update path. The solution: check platform coverage before deciding whether to downgrade, but always allow label-only updates on already-cached dual-platform events.

---

## Q3: extract_spread_total_markets Return Shape

**Signature:** `fn extract_spread_total_markets(event: &GammaEvent, sport: Sport) -> Vec<(MarketType, Vec<String>, Vec<String>)>`

**Return tuple fields:**
1. `MarketType` — either `MarketType::Spread(f64)` or `MarketType::Total(f64)` (the `f64` line value, defaulting to `0.0` if no numeric line found in `group_item_title`)
2. `Vec<String>` — CLOB token IDs, always length 2 (guarded by `if token_ids.len() != 2 ... { continue }`)
3. `Vec<String>` — outcome labels, always length 2 (same guard), normalized through `team_dictionary::lookup_team`

**How spread/total candidates differ from moneyline:**
- `market_type` field: moneyline candidates use `MarketType::Moneyline`; spread/total use `MarketType::Spread(line)` or `MarketType::Total(line)`
- `polymarket_token_ids`: spread/total markets use their own per-market CLOB token IDs, distinct from the moneyline's token IDs (each sub-market has its own tokens)
- `polymarket_outcome_labels`: spread = team names or "Over"/"Under"; total = "Over"/"Under"
- `polymarket_slug`: **same** as the moneyline candidate — both spread/total and moneyline candidates share the parent event's slug

**The dedup key for D-03 is `(polymarket_slug, MarketTypeBucket)`:**
- A moneyline candidate: `(slug="nba-lal-bos-2026-03-16", MarketTypeBucket::Moneyline)`
- A spread candidate: `(slug="nba-lal-bos-2026-03-16", MarketTypeBucket::Spread(ordered_f64(-3.5)))`
- A total candidate: `(slug="nba-lal-bos-2026-03-16", MarketTypeBucket::Total(ordered_f64(215.5)))`

These are distinct keys. The double-emit bug that D-03 fixes is that the **moneyline emit path** (line 242 in `fetcher_polymarket.rs`) and the **spread/total emit path** (the `for (market_type, tids, labels) in extra` loop at line 263) can both fire for the same slug. For a spread-only grouped event, `extract_moneyline_market` may still return token IDs via Strategy 3 fallback, causing both a `Moneyline` candidate and a `Spread` candidate to be emitted for the same game — both with `MarketTypeBucket::Moneyline` and `MarketTypeBucket::Spread(X)` respectively. The dedup key `(slug, market_type_bucket)` is the correct discriminant to collapse same-game same-type duplicates.

**`MarketTypeBucket` is already `Hash + Eq + Clone + Copy`** (line 45 in `candidate.rs`) — it can be used directly as a `HashMap` key.

---

## Q4: match_candidates → upsert Flow

### Call Chain

```
main.rs
  tokio::spawn -> matching::run_sports_matching_loop(state)
                    loop:
                      discover_and_match_sports(&state)
                        |
                        +-- fetcher_kalshi::fetch_kalshi_sports_candidates(&sports)  [concurrent]
                        +-- fetcher_polymarket::fetch_polymarket_sports_candidates(&sports)  [concurrent]
                        |
                        +-- matcher::match_candidates(kalshi_candidates, poly_candidates, min_score)
                        |     returns MatchResult { events: Vec<CanonicalEvent>, matched_pairs, unmatched_details }
                        |
                        |  [stats update, label resolution loop]
                        |
                        for mut event in events:  ← THIS is where upsert happens
                          state.cache.upsert_event(event)    ← line 270
                          state.db.upsert_event(&event)
```

### Exact Insertion Point for Post-Construction Dedup Pass (D-02)

The dedup pass should be inserted in `discover_and_match_sports()` in `matching/mod.rs`, **after** `matcher::match_candidates()` returns but **before** the `discovered_sports_ids` snapshot and the label resolution loop.

Specifically, after line 95 (`let total = events.len();`), insert:

```rust
// D-02: post-construction dedup pass — collapse any duplicate IDs
// that build_event_id() produced from different input paths
let events = dedup_events_by_id(events);
```

This ensures:
1. The `discovered_sports_ids` HashSet is built from the already-deduped list (no phantom IDs)
2. The label resolution loop iterates over the deduped list (no double-resolution)
3. `upsert_event` is called once per canonical event ID (no blind overwrite from a duplicate)

The dedup pass can be a simple sort-by-platform-coverage-then-dedup-by-id pattern, or a `HashMap<String, CanonicalEvent>` keyed by event ID that keeps the highest-coverage entry.

### Label-Update Path (line 261) Interaction

The label-update upsert at line 261 is inside the `for mut event in events` loop. It executes only when `cached_labels != event.platform_ids.polymarket_outcome_labels` and `token_ids_changed == false`. This is an update to an already-cached event. The D-04 merge logic must allow this path through — it is updating an existing dual-platform event's labels, not downgrading platform coverage.

---

## Q5: Token ID Type

**In `CandidateEvent` (`candidate.rs` line 79):**
```rust
pub polymarket_token_ids: Vec<String>,
```

**In `PlatformIds` (`models/event.rs` line 103):**
```rust
pub polymarket_token_ids: Vec<String>,
```

**In `CanonicalEvent`:** stored inside `platform_ids: PlatformIds`.

**Current comparison in `matching/mod.rs` (line 192):**
```rust
cached.platform_ids.polymarket_token_ids != event.platform_ids.polymarket_token_ids
```
This is an **ordered** `Vec<String>` equality check. The CONTEXT.md note under Specifics calls for order-independent set comparison.

**Implementation for D-05 order-independent comparison:**
```rust
fn token_ids_differ(a: &[String], b: &[String]) -> bool {
    if a.len() != b.len() { return true; }
    let mut sorted_a = a.to_vec(); sorted_a.sort();
    let mut sorted_b = b.to_vec(); sorted_b.sort();
    sorted_a != sorted_b
}
```
No new dependencies needed — `Vec::sort()` is in std. No need for `HashSet` unless token IDs can repeat (they cannot — each token ID is a unique CLOB contract identifier).

**Note:** The existing ordered comparison at line 192 is already doing token change detection and clearing odds. D-05 upgrades this to order-independent comparison. The existing logic at lines 199–205 (clear polymarket odds on token change) is already partially correct; D-05 ensures it fires even when token IDs are the same set in different order.

---

## Q6: Cargo.toml / Utilities

**dashmap version:** `dashmap = "6"` (line 30 in `Cargo.toml`)

**No new crates required for Phase 1.** All three fixes use only std library features:
- `HashMap` (std) — for the post-construction dedup pass in D-02
- `HashSet` (std) — already used in `match_candidates()` for `matched_kalshi`/`matched_poly`
- `Vec::sort()` + `Vec::dedup()` (std) — for token ID set comparison in D-05
- `MarketTypeBucket` (already `Hash + Eq`) — for dedup key in D-03

**Existing dedup utilities in codebase:**
- `matched_kalshi: HashSet<usize>` and `matched_poly: HashSet<usize>` in `matcher.rs::match_candidates()` — the established pattern for tracking already-processed indices. The D-02 post-construction dedup should follow this same HashSet pattern.
- `seen_slugs: HashSet<String>` in `fetcher_polymarket.rs::fetch_polymarket_sports_candidates()` (line 87) — dedup by slug across tag fetches. This is the exact pattern D-03 should use inside `process_gamma_events`, scoped to `(slug, market_type_bucket)` instead of just `slug`.

**`MarketTypeBucket` hashability:** Confirmed `Hash + Eq + Clone + Copy` at `candidate.rs` line 45 — usable directly as a `HashMap` or `HashSet` key.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[cfg(test)]` / `cargo test` |
| Config file | None — standard Rust inline tests |
| Quick run command | `cd backend-rust && cargo test matching 2>&1` |
| Full suite command | `cd backend-rust && cargo test 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| R1 | `build_event_id` never produces "unk"/"nodate" when slug/ticker data is present | unit | `cargo test test_event_id_no_teams_uses_slug_fallback` | No — Wave 0 gap |
| R1 | `build_event_id` order-invariant (existing) | unit | `cargo test test_event_id_order_invariant` | Yes |
| R1 | Post-construction dedup collapses same-ID duplicates | unit | `cargo test test_dedup_pass_collapses_duplicates` | No — Wave 0 gap |
| R2 | `process_gamma_events` emits exactly one candidate per (slug, market_type) | unit | `cargo test test_process_gamma_events_no_double_emit` | No — Wave 0 gap |
| R3 | `upsert_event` never downgrades dual-platform to single-platform | unit | `cargo test test_upsert_never_downgrades_dual_to_single` | No — Wave 0 gap |
| R3 | `upsert_event` upgrades single-platform to dual-platform | unit | `cargo test test_upsert_upgrades_single_to_dual` | No — Wave 0 gap |
| R3 | Token ID change clears polymarket odds (order-independent) | unit | `cargo test test_upsert_clears_poly_odds_on_token_change` | No — Wave 0 gap |

### Wave 0 Gaps
- [ ] `test_event_id_no_teams_uses_slug_fallback` — in `matcher.rs` tests — covers R1 fallback path
- [ ] `test_event_id_unk_never_appears` — in `matcher.rs` tests — R1 regression guard
- [ ] `test_dedup_pass_collapses_duplicates` — in `matching/mod.rs` or new `matching/mod.rs` test block — covers D-02
- [ ] `test_process_gamma_events_no_double_emit` — in `fetcher_polymarket.rs` tests — covers R2
- [ ] `test_upsert_never_downgrades_dual_to_single` — in `cache.rs` tests — covers R3/D-04
- [ ] `test_upsert_upgrades_single_to_dual` — in `cache.rs` tests — covers R3/D-04
- [ ] `test_upsert_clears_poly_odds_on_token_change` — in `cache.rs` tests — covers R3/D-05

---

## Implementation Notes

1. **D-01 tiebreaker material is already present.** `CandidateEvent` carries `kalshi_event_ticker: Option<String>` and `polymarket_slug: Option<String>`. When teams are absent, `build_event_id` can extract slug segments directly: for `nba-lal-bos-2026-03-16` the slug itself encodes sport + teams + date, making it a reliable stable fallback ID source.

2. **D-02 dedup pass location is `matching/mod.rs` lines ~93–95**, between `match_candidates()` return and `discovered_sports_ids` snapshot. The pass should merge platform_ids when collapsing: if two entries have the same ID but one is dual and one is single, keep the dual.

3. **D-03 dedup scope is inside `process_gamma_events`.** The existing `seen_slugs` dedup in `fetch_polymarket_sports_candidates` only deduplicates by slug (not by market type). D-03 adds a finer-grained dedup key `(slug, MarketTypeBucket)` at the level of the inner loop in `process_gamma_events`. A `HashSet<(String, MarketTypeBucket)>` initialized before the `for event in events` loop and checked before each `candidates.push(...)` call is the minimal change.

4. **D-04 platform coverage check.** The correct check for "is this event dual-platform?" is:
   - Cached: `event.platform_ids.kalshi_event_ticker.is_some() && event.platform_ids.polymarket_market_slug.is_some()`
   - Incoming: same check on the new event
   - Logic: if `cached_is_dual && !incoming_is_dual` → skip the insert (keep cached). All other combinations → allow insert.

5. **D-05 interacts with existing token-change detection at `matching/mod.rs` line 192.** That code already detects token changes and clears odds — but uses ordered equality. The fix in `upsert_event` itself (D-05) adds a second line of defense at the cache layer, using sorted-Vec comparison. The `clear_platform_odds` method already exists at `cache.rs` line 117 and does exactly what D-05 needs.

6. **Never-downgrade logic must allow label-only updates.** At `matching/mod.rs` line 261, `upsert_event` is called on an already-cached dual-platform event to update its outcome labels. The D-04 logic must not block this. Since the event being upserted in that branch still has both platforms' data (it was fetched as a matched pair), the platform coverage check (`incoming_is_dual`) will be true and the upsert will proceed. No special case needed.

7. **dome_poller caller is inactive.** `ingestion/dome_poller.rs` line 145 calls `upsert_event` only when `enable_native_matching=false`. In normal operation this code does not run. No special handling needed, but the merge logic must not break it if someone re-enables the legacy path.

8. **`process_gamma_events` has no test for the emit logic itself** — only its helper functions are tested. The Wave 0 test gap for R2 is the highest-priority new test to write before implementing D-03, as it will directly validate the fix.

---

## Sources

All findings are from direct inspection of the following files. Confidence: HIGH for all — no external lookups required.

- `backend-rust/src/matching/matcher.rs` — `build_event_id`, `match_candidates`, test suite
- `backend-rust/src/matching/fetcher_polymarket.rs` — `process_gamma_events`, `extract_spread_total_markets`, test suite
- `backend-rust/src/storage/cache.rs` — `upsert_event`, `clear_platform_odds`, test suite
- `backend-rust/src/matching/candidate.rs` — `CandidateEvent`, `MarketTypeBucket`
- `backend-rust/src/matching/mod.rs` — `run_sports_matching_loop`, `discover_and_match_sports`, full upsert call chain
- `backend-rust/src/ingestion/culture_poller.rs` — secondary upsert callers (culture events)
- `backend-rust/src/models/event.rs` — `PlatformIds`, `CanonicalEvent` struct definitions
- `backend-rust/Cargo.toml` — dependency versions
- `backend-rust/src/main.rs` — task spawn topology, confirming dome_poller is the only inactive caller
