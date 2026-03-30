# Phase 1: Foundational Stability - Context

**Gathered:** 2026-03-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix the three foundational plumbing bugs that make the event cache unreliable:
1. `build_event_id()` produces non-deterministic IDs when teams/dates are missing → duplicate cache entries
2. `process_gamma_events()` emits multiple `CandidateEvent`s per game (moneyline + spread/total from same slug) → same game appears twice
3. `upsert_event()` blindly replaces cache entries → dual-platform events downgraded to single-platform

No new features, no UI changes, no new APIs. All changes are in `backend-rust/src/matching/` and `backend-rust/src/storage/cache.rs`.

</domain>

<decisions>
## Implementation Decisions

### R1: Event ID stability (build_event_id)
- **D-01:** Fix `build_event_id()` to never produce `"unk"` or `"nodate"` fallbacks in normal operation. When team extraction yields nothing, fall back to poly slug segments or kalshi ticker segments as a tiebreaker to build a stable deterministic ID.
- **D-02:** Add a post-construction dedup pass after `match_candidates()` returns its `Vec<CanonicalEvent>` — collapse any entries with the same (sport, team_a, team_b, date) or same platform slug/ticker into a single entry. Belt + suspenders against edge cases.

### R2: Polymarket double-emit (process_gamma_events)
- **D-03:** Deduplicate within `process_gamma_events()` itself. After building all candidates for a single event, deduplicate by `(polymarket_slug, market_type_bucket)` before appending to `candidates`. Same-game same-type duplicates are collapsed at the source.

### R3: Cache merge semantics (upsert_event)
- **D-04:** Replace `self.events.insert()` with "never downgrade" merge logic: if the cached event has both platforms' data and the incoming event has only one platform, keep the cached version. Only overwrite if the incoming event has equal or more platform coverage.
- **D-05:** On cache upsert, check if `polymarket_token_ids` changed. If they differ from the cached value, clear the odds for that event's Polymarket platform. Prevents stale prices from dead tokens being used for arb detection.

### Claude's Discretion
- Exact data structure for the post-construction dedup pass (HashMap keyed by event slug/ticker vs. sort+dedup)
- Whether `upsert_event()` comparison uses a platform-count heuristic or checks presence of non-empty `kalshi_event_ticker` and `polymarket_market_slug`
- Log verbosity for merge/dedup events (debug vs. info)

</decisions>

<specifics>
## Specific Ideas

- The existing `build_single_platform_event()` function already uses stable IDs (`kalshi-{ticker}`, `poly-{slug}`) — no changes needed there. The instability is isolated to `build_event_id()` for matched pairs only.
- The dedup pass after `match_candidates` should be non-lossy: when collapsing duplicates, merge platform_ids rather than picking one arbitrarily.
- Token ID change detection should compare `Vec<String>` sets (order-independent), not ordered equality, since Polymarket token order can vary.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` — R1, R2, R3 with confirmed root causes and acceptance criteria

### Roadmap
- `.planning/ROADMAP.md` §Phase 1 — Goal statement and success criteria (4 criteria to satisfy)

### Files to modify
- `backend-rust/src/matching/matcher.rs` — `build_event_id()` at line 440, `match_candidates()` (add post-construction dedup), `build_single_platform_event()` at line 405 (reference only, already stable)
- `backend-rust/src/matching/fetcher_polymarket.rs` — `process_gamma_events()` at line 160 (add within-function dedup by slug+market_type_bucket)
- `backend-rust/src/storage/cache.rs` — `upsert_event()` at line 59 (replace `.insert()` with merge logic + token_id change detection)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `build_single_platform_event()` (matcher.rs:405) — already uses stable `kalshi-{ticker}` / `poly-{slug}` IDs; pattern for how to extract stable fallback segments for `build_event_id()`
- `MarketTypeBucket` enum (candidate.rs) — already used for bucketing; use the same `.bucket_key()` method for dedup key in `process_gamma_events`
- `matched_kalshi`/`matched_poly` HashSets in `match_candidates()` — existing dedup pattern; post-construction dedup pass should follow same pattern

### Established Patterns
- Error strategy: `if let Err(e) = ... { warn!(...) }` — log and continue, never panic on data path
- DashMap for cache: all cache ops must be lock-free; no `RwLock<HashMap>` replacements

### Integration Points
- `match_candidates()` return value → `run_sports_matching_loop` calls this → resulting `CanonicalEvent`s are upserted into `StateCache`
- `upsert_event()` is called from both the matching loop and the culture poller — the merge logic must be safe for both callers

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within Phase 1 scope.

</deferred>

---

*Phase: 01-foundational-stability*
*Context gathered: 2026-03-29*
