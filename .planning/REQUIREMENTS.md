# Requirements

> Generated: 2026-03-29 | Milestone: Sports Event Matching Overhaul

## Goal

Fix three user-visible defects in cross-platform sports event matching: duplicate events, missed matches (events on both platforms but only showing one platform's odds), and wrong event dates.

## Scope

**In scope:**
- All sports available on Polymarket and/or Kalshi
- Kalshi and Polymarket only (no new platform integrations)
- Backend matching pipeline (`backend-rust/src/matching/`)
- Frontend display correctness (events show correct odds from both platforms)

**Out of scope:**
- Non-sports (culture/politics) matching
- Autonomous trading agent
- New platform integrations
- Mobile / new UI design

---

## Requirements

### P0 — Must Have (blocking correct behavior)

| ID | Requirement | Why |
|----|-------------|-----|
| R1 | Event IDs must be stable across fetch cycles | Non-deterministic IDs cause duplicate cache entries, making dedup impossible downstream |
| R2 | Polymarket must emit exactly one `CandidateEvent` per game | Grouped-market + moneyline paths both fire, causing the same game to appear twice |
| R3 | Cache upsert must use merge semantics, not replace | `.insert()` silently overwrites a correctly-matched dual-platform event with a single-platform result from a bad cycle |
| R4 | Kalshi date must come from ticker, not `close_time` | `close_time` is the trading deadline, not the game time; causes same-day games to land in different date buckets |
| R5 | Polymarket date must prefer `startDate` over `end_date` | `end_date` is market expiry (often 2–3 days after game); causes bucket mismatches |
| R6 | Any sport available on either platform must appear in the sports tab | Events must not be silently dropped due to missing sport classification |

### P1 — Should Have (fixes arb signal quality)

| ID | Requirement | Why |
|----|-------------|-----|
| R7 | Label resolver must receive sport context to disambiguate team abbreviations | `ATL/CHI/DET/MIA/PHI/CLE/HOU` collide across NBA/NFL/MLB without sport context, producing cross-sport false-positive matches |
| R8 | Spread-line extraction must return `Option<f64>`, not silent `0.0` | Parse failures collapse all spread markets into `Spread(0)`, conflating different bet lines |
| R9 | Prop markets and game-winner markets must not be paired with each other | Different market types being matched produces nonsensical arb signals |

### P2 — Nice to Have (coverage + observability)

| ID | Requirement | Why |
|----|-------------|-----|
| R10 | College teams (CBB/CFB) must be in the team dictionary | Currently 0 college entries means team-name score is always 0.0, making matching impossible for college sports |
| R11 | Team name matching must have a fuzzy fallback | Exact dictionary lookup fails for misspellings, new teams, and international name variants |
| R12 | Label resolution must run with concurrency | Currently 50 sequential HTTP calls; `buffer_unordered(10)` would reduce latency significantly |
| R13 | Unmatched events must emit structured rejection logs | Without logging, prioritization within phases is heuristic; structured logs make it data-driven |
| R14 | Scoring weights must be recalibrated against production data | Current weights are initial guesses; tuning after phases 1–3 will close remaining false negatives |

---

## Acceptance Criteria

The milestone is complete when:
- [ ] A sporting event that appears on both Kalshi and Polymarket shows odds from **both platforms** in the sports tab
- [ ] No sporting event appears more than once in the sports tab
- [ ] Displayed event dates match the actual game date (not market expiry or trading deadline)
- [ ] College basketball and college football events appear in the sports tab when active on either platform
- [ ] No props/futures are displayed alongside game-winner markets as if they're the same event

---

## Dependencies

- No new external APIs (Kalshi and Polymarket direct APIs only)
- `strsim = "0.11"` crate (for fuzzy matching, P2 only)
- Existing Rust backend stack — no architectural changes required

---

## Validated From Research

- Root causes confirmed by direct code inspection of `matcher.rs`, `fetcher_kalshi.rs`, `fetcher_polymarket.rs`, `candidate.rs`, `team_dictionary.rs`
- `build_event_id()` instability confirmed: team order and date field presence vary across cycles
- Polymarket grouped-market double-emit confirmed in `process_gamma_events()`
- Kalshi `close_time` vs ticker date confirmed: UTC midnight rollover mismatches documented
- Zero college entries in `team_dictionary.rs` confirmed
- Label resolver missing sport param confirmed in `match_label_to_abbrev()`

---

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| R1 | Phase 1 — Foundational Stability | Pending |
| R2 | Phase 1 — Foundational Stability | Pending |
| R3 | Phase 1 — Foundational Stability | Pending |
| R4 | Phase 2 — Date Extraction Reliability | Pending |
| R5 | Phase 2 — Date Extraction Reliability | Pending |
| R6 | Phase 2 — Date Extraction Reliability | Pending |
| R7 | Phase 3 — Label and Market Type Alignment | Pending |
| R8 | Phase 3 — Label and Market Type Alignment | Pending |
| R9 | Phase 3 — Label and Market Type Alignment | Pending |
| R10 | Phase 4 — Coverage and Observability | Pending |
| R11 | Phase 4 — Coverage and Observability | Pending |
| R12 | Phase 4 — Coverage and Observability | Pending |
| R13 | Phase 4 — Coverage and Observability | Pending |
| R14 | Phase 4 — Coverage and Observability | Pending |
