---
plan: 01-02
phase: 01-foundational-stability
status: complete
completed_at: 2026-03-30
subsystem: matching
tags: [dedup, polymarket, matching, tdd]
dependency_graph:
  requires: []
  provides: [polymarket-double-emit-fix]
  affects: [matching/fetcher_polymarket]
tech_stack:
  added: []
  patterns: [HashSet dedup guard per (slug, MarketTypeBucket) pair]
key_files:
  modified:
    - backend-rust/src/matching/fetcher_polymarket.rs
decisions:
  - Used seen: HashSet<(String, MarketTypeBucket)> scoped to single process_gamma_events call to prevent within-call duplicates; cross-call dedup remains in fetch_polymarket_sports_candidates via seen_slugs
metrics:
  duration: ~5min
  completed_date: 2026-03-30
  tasks_completed: 1
  files_modified: 1
---

# Phase 01 Plan 02: Polymarket Double-Emit Fix Summary

## One-liner

Added `seen: HashSet<(String, MarketTypeBucket)>` guard in `process_gamma_events` to prevent multiple `CandidateEvent` outputs for the same `(slug, market_type_bucket)` pair within a single call.

## What was done

- Added `seen: HashSet<(String, MarketTypeBucket)>` guard at the top of `process_gamma_events`, after `let mut candidates = Vec::new()`
- At the moneyline emit site (~line 245): compute `bucket = market_type.bucket_key()`, then only push if `seen.insert((slug.clone(), bucket))` returns true; otherwise emit `debug!("D-03: skipping duplicate moneyline ...")`
- At the spread/total emit site (~line 272): same pattern — check `seen.insert` before each `candidates.push` in the `for (market_type, tids, labels) in extra` loop

This ensures that even if the same slug is fed into `process_gamma_events` twice (e.g. from duplicate API responses), or if both the moneyline path and spread/total path would emit the same `(slug, Moneyline)` bucket, each unique pair is emitted at most once.

## Tests added

- `test_process_gamma_events_no_double_emit`: feeds the same slug twice (once with a spread market), asserts `moneyline_count <= 1`
- `test_process_gamma_events_dedup_by_slug_market_type`: feeds two identical GammaEvents, then scans all output candidates for duplicate `(polymarket_slug, market_type.bucket_key())` pairs — asserts none exist

## Verification

Full suite: 108 tests, 0 failures.

```
test result: ok. 108 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None.

## Self-Check: PASSED

- `backend-rust/src/matching/fetcher_polymarket.rs` — FOUND and modified
- Commit `8e5dba4` — FOUND (`feat(01-02): add HashSet dedup guard to process_gamma_events`)
- Both test functions present at lines 1129 and 1180
- `seen.insert` appears at lines 245 and 272 inside `process_gamma_events`
