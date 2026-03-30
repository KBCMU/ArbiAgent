---
plan: 01-03
phase: 01-foundational-stability
status: complete
completed_at: 2026-03-30
subsystem: storage/cache + matching/mod
tags: [cache, merge-semantics, token-ids, dedup, tdd]
dependency_graph:
  requires: [01-01]
  provides: [D-04-no-downgrade, D-05-token-change-detection]
  affects: [upsert_event, token_ids_changed]
tech_stack:
  added: []
  patterns: [DashMap guard release before mutation, order-independent set comparison]
key_files:
  created: []
  modified:
    - backend-rust/src/storage/cache.rs
    - backend-rust/src/matching/mod.rs
decisions:
  - Used Self::token_ids_differ as private impl method rather than a free function to keep it co-located with upsert_event
  - Used sort-then-compare instead of HashSet for order-independent comparison (avoids allocation overhead; slices are small)
  - Dropped DashMap guard to a bool (should_clear_odds) before any mutable operation to prevent deadlock
metrics:
  duration: 15min
  completed_date: 2026-03-30
  tasks_completed: 2
  files_modified: 2
---

# Phase 1 Plan 3: Cache Merge Semantics + Order-Independent Token Comparison Summary

One-liner: Merge semantics in upsert_event (dual-platform never downgraded, stale Polymarket odds cleared on token change) plus sort-then-compare token ID fix in mod.rs.

## What was done

### Task 1: cache.rs upsert_event
- Added `token_ids_differ` private helper method (order-independent set comparison via sort-then-compare)
- Replaced one-line blind `insert()` with merge semantics:
  - Never-downgrade guard (D-04): dual-platform events are never overwritten by single-platform events; returns early with debug log
  - Token change detection (D-05): Polymarket odds are cleared via `clear_platform_odds` when token IDs change between cycles
- DashMap guard extracted to a bool (`should_clear_odds`) and dropped before mutable operations to avoid deadlock

### Task 2: mod.rs token_ids_changed
- Replaced ordered `!=` comparison with sort-then-compare block
- Token ID sets that arrive in different order now correctly compare as equal (no spurious odds clearing in the matching loop)

## Tests added
- `test_upsert_never_downgrades_dual_to_single`
- `test_upsert_upgrades_single_to_dual`
- `test_upsert_clears_poly_odds_on_token_change`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Adapted test code for actual EventOdds/OutcomePrice struct fields**
- **Found during:** Task 1 RED phase
- **Issue:** Plan's test code used `event_id` and `price`/`token_id` fields that do not exist; actual struct uses `canonical_event_id`, `yes_price`, `no_price`
- **Fix:** Updated test to use correct field names matching `backend-rust/src/models/event.rs`
- **Files modified:** `backend-rust/src/storage/cache.rs`
- **Commit:** 69c3532

**2. [Rule 1 - Bug] DashMap borrow lifetime error in test assertion**
- **Found during:** Task 1 RED phase compilation
- **Issue:** `if let Some(odds) = cache.odds.get(event_id)` held a Ref past end of scope, triggering E0597
- **Fix:** Extracted to `let poly_odds_present = cache.odds.get(...).map_or(...)` so the temporary Ref is dropped immediately
- **Files modified:** `backend-rust/src/storage/cache.rs`
- **Commit:** 69c3532

**3. [Rule 2 - Missing import] Added `use tracing::debug` import**
- **Found during:** Task 1 GREEN phase
- **Issue:** `debug!` macro used in upsert_event without import
- **Fix:** Added `use tracing::debug` to top of cache.rs
- **Files modified:** `backend-rust/src/storage/cache.rs`
- **Commit:** 69c3532

## Commits
- `69c3532` — feat(01-03): upsert_event merge semantics (D-04 no-downgrade + D-05 token-change)
- `7a4bd51` — fix(01-03): order-independent token ID comparison in mod.rs (D-05)

## Verification
Full suite: 115 passed, 0 failed.

## Self-Check: PASSED
- SUMMARY.md: FOUND
- Commit 69c3532: FOUND
- Commit 7a4bd51: FOUND
