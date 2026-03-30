# Roadmap: ArbiAgent — Sports Event Matching Overhaul

## Overview

The matching pipeline is structurally sound but accumulates three user-visible defects from locatable bugs in its lower stages: duplicate events in the UI, events on both platforms showing only one platform's odds, and wrong game dates. This milestone repairs those bugs in strict dependency order — IDs and deduplication first, then date extraction, then label alignment, then coverage expansion — so each phase builds on a verified, stable baseline from the one before it.

## Phases

- [ ] **Phase 1: Foundational Stability** - Eliminate duplicate events by fixing event ID instability, Polymarket double-emit, and cache overwrite
- [ ] **Phase 2: Date Extraction Reliability** - Fix Kalshi and Polymarket date sources so events land in correct buckets and show correct dates
- [ ] **Phase 3: Label and Market Type Alignment** - Fix sport-context in label resolver, spread-line extraction, and prop/game-winner separation
- [ ] **Phase 4: Coverage and Observability** - Expand college team dictionary, add fuzzy matching, parallelize label resolution, surface rejection diagnostics, recalibrate scoring weights

## Phase Details

### Phase 1: Foundational Stability
**Goal**: Eliminate duplicate events in the UI by making event IDs deterministic, stopping Polymarket's double-emit, adding a post-construction dedup stage, and upgrading cache upsert to merge semantics.
**Depends on**: Nothing (first phase)
**Requirements**: R1, R2, R3
**Success Criteria** (what must be TRUE):
  1. Refreshing the sports tab multiple times shows the same set of events with no growing duplicates between polling cycles
  2. A game that appears on both Polymarket and Kalshi is never shown as two separate entries in the event table
  3. An event that was correctly matched (showing odds from both platforms) is not downgraded to single-platform odds on the next fetch cycle
  4. The same event has the same ID across consecutive matching cycles (verified via matching-stats logs)
**Plans**: 3 plans

Plans:
- [x] 01-01-PLAN.md — Fix build_event_id fallback IDs + post-construction dedup pass (R1)
- [x] 01-02-PLAN.md — Stop Polymarket double-emit in process_gamma_events (R2)
- [ ] 01-03-PLAN.md — Upgrade cache upsert to merge semantics + order-independent token ID comparison (R3)

### Phase 2: Date Extraction Reliability
**Goal**: Ensure every candidate event carries an accurate game date from the correct API field so that cross-platform events land in the same date bucket and displayed dates match the actual game date.
**Depends on**: Phase 1
**Requirements**: R4, R5, R6
**Success Criteria** (what must be TRUE):
  1. Game dates shown in the UI match the actual game date, not the market expiry or trading deadline
  2. Events for the same game on Kalshi and Polymarket land in the same date bucket and are paired (not left as two separate single-platform events)
  3. Any sport active on either platform appears in the sports tab — no events are silently dropped due to a date-bucket miss causing failed sport classification
  4. Volume of events falling through to the date-relaxed cross-bucket fallback path decreases relative to Phase 1 baseline (observable via matching-stats logs)
**Plans**: TBD

### Phase 3: Label and Market Type Alignment
**Goal**: Fix outcome label resolution so team abbreviations are correctly disambiguated by sport, spread lines reflect actual market lines, and prop markets are excluded from game-winner matching.
**Depends on**: Phase 2
**Requirements**: R7, R8, R9
**Success Criteria** (what must be TRUE):
  1. Dual-platform events show team name labels (e.g., "BOS" / "MIA") rather than generic "Yes" / "No" outcome labels in the arbitrage view
  2. Spread markets at different lines (e.g., -3.5 vs -6.5) are not conflated — each line appears as a distinct market entry
  3. Player prop markets (points, rebounds, yards, touchdowns) do not appear alongside or paired with game-winner markets in the sports tab
  4. Multi-sport abbreviations (ATL, CHI, DET, MIA, PHI, CLE, HOU, DAL, WAS) resolve to the correct team within their sport context — confirmed by unit tests
**Plans**: TBD

### Phase 4: Coverage and Observability
**Goal**: Reduce the residual false-negative rate by expanding the college team dictionary, adding fuzzy name matching, parallelizing label resolution, surfacing rejection diagnostics, and recalibrating scoring weights against production data.
**Depends on**: Phase 3
**Requirements**: R10, R11, R12, R13, R14
**Success Criteria** (what must be TRUE):
  1. College basketball and college football events appear in the sports tab when those markets are active on either platform
  2. Team names that are misspelled or use regional variants (e.g., "Los Angeles Lakers" vs "Lakers") are matched rather than dropped — fuzzy fallback fires and is visible in logs
  3. Label resolution latency per matching cycle decreases — concurrent HTTP requests replace the current sequential chain (observable via cycle timing logs)
  4. Unmatched events emit structured rejection logs including best-rejected score, so future prioritization is data-driven rather than guesswork
  5. Scoring weights have been reviewed and adjusted based on production log analysis from the Phase 1-3 baseline
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundational Stability | 2/3 | In Progress|  |
| 2. Date Extraction Reliability | 0/TBD | Not started | - |
| 3. Label and Market Type Alignment | 0/TBD | Not started | - |
| 4. Coverage and Observability | 0/TBD | Not started | - |
