---
plan: 01-01
phase: 01-foundational-stability
status: complete
completed_at: 2026-03-30
subsystem: matching
tags: [tdd, event-ids, dedup, matching]
dependency_graph:
  requires: []
  provides: [stable-event-ids, dedup-pass]
  affects: [matching/matcher.rs, matching/mod.rs]
tech_stack:
  added: []
  patterns: [slug-parsing, non-lossy-merge, tdd-red-green]
key_files:
  created: []
  modified:
    - backend-rust/src/matching/matcher.rs
    - backend-rust/src/matching/mod.rs
decisions:
  - Use slug_team_segments to extract 2-5 char alphabetic tokens excluding known sport prefixes
  - Use slug_date for YYYY-MM-DD scanning via byte-level pattern matching (zero deps)
  - Fallback strings changed from "unk"/"nodate" to "unknown1"/"unknown2"/"undated" for clarity
  - dedup_events_by_id uses dual-platform coverage as the ranking criterion (both kalshi + poly IDs)
  - Non-lossy merge: backfill None/empty fields from lower-coverage entry so no token IDs lost
metrics:
  duration: ~25min
  completed_date: 2026-03-30
  tasks_completed: 2
  files_modified: 2
---

# Phase 1 Plan 1: Stable Event IDs + Dedup Pass Summary

**One-liner:** Rewrote `build_event_id` to parse team segments and dates from Polymarket slugs/Kalshi tickers when explicit fields are None, and added a non-lossy `dedup_events_by_id` post-construction pass that collapses duplicate IDs by merging platform_ids fields.

## What was done

- Rewrote `build_event_id` in `matcher.rs` to extract team segments and dates from Polymarket slugs and Kalshi tickers when explicit `team_a`/`team_b` fields are `None`
- Added `slug_team_segments` helper: splits slug/ticker on `-`, keeps 2-5 char alphabetic tokens, filters out known sport prefixes (`nba`, `nfl`, `nhl`, etc.)
- Added `slug_date` helper: scans slug/ticker for embedded `YYYY-MM-DD` pattern using byte-level comparison (zero external dependencies)
- Fallback strings changed: `"unk"` replaced by slug-derived abbreviation or `"unknown1"`/`"unknown2"`; `"nodate"` replaced by slug-parsed date or `"undated"`
- Added `dedup_events_by_id` in `mod.rs` with non-lossy merge semantics: dual-platform events win over single-platform regardless of arrival order; backfills `None`/empty `platform_ids` fields from the lower-coverage entry
- Inserted `dedup_events_by_id` call in `discover_and_match_sports` between `let events = match_result.events` and `let total = events.len()`

## Tests added

- `test_event_id_no_teams_uses_slug_fallback` — verifies no "unk" when teams are None but poly slug has segments
- `test_event_id_no_date_uses_slug_fallback` — verifies no "nodate" when game_date is None but poly slug has embedded date
- `test_event_id_unk_never_appears` — verifies "unk" never appears even with sparse inputs
- `test_dedup_pass_collapses_duplicates` — same ID collapses to one event, kalshi_event_ticker preserved
- `test_dedup_pass_keeps_dual_over_single` — dual-platform wins even if single-platform arrives first
- `test_dedup_pass_merges_platform_ids` — kalshi-only + poly-only with same ID get merged into one dual event
- `test_dedup_pass_no_duplicates_passthrough` — 3 distinct IDs pass through unchanged

## Verification

All 112 matching tests pass. Full suite green.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed `Sport::Basketball` in test helper**
- **Found during:** Task 2 RED phase
- **Issue:** Plan's test scaffold used `Sport::Basketball` which doesn't exist in the codebase (enum variant is `Sport::Nba`)
- **Fix:** Changed `Sport::Basketball` to `Sport::Nba` in the test helper
- **Files modified:** `backend-rust/src/matching/mod.rs`
- **Commit:** 8a87e17

## Self-Check: PASSED

Files confirmed:
- FOUND: backend-rust/src/matching/matcher.rs (modified)
- FOUND: backend-rust/src/matching/mod.rs (modified)

Commits confirmed:
- 33c7495: feat(01-01): rewrite build_event_id with slug/ticker fallback + add 3 TDD tests
- 8a87e17: feat(01-01): add dedup_events_by_id with non-lossy merge semantics + 4 TDD tests
