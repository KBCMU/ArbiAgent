# ArbiAgent

## What This Is

A prediction market arbitrage scanner that discovers events across Polymarket and Kalshi, matches corresponding events across platforms, and identifies arbitrage opportunities. Built as a Rust backend (Axum + Tokio) with a Next.js frontend. Currently focused on improving cross-platform sports event matching accuracy.

## Core Value

Accurately match the same sporting event across Polymarket and Kalshi so users can compare odds and spot arbitrage opportunities.

## Requirements

### Validated

- ✓ Fetch market data from Kalshi and Polymarket via direct REST APIs — existing
- ✓ Real-time price updates via WebSocket connections (Kalshi, Polymarket) — existing
- ✓ In-memory cache with DashMap for events, odds, and active arbs — existing
- ✓ Supabase Postgres persistence for events, odds snapshots, and arb history — existing
- ✓ Arbitrage detection loop (2s interval) with fee accounting — existing
- ✓ Frontend markets dashboard with event table, filters, and sport tabs — existing
- ✓ Frontend arbitrage view with real-time WebSocket push — existing
- ✓ Supabase Auth (email/password login, signup, session management) — existing
- ✓ Culture/non-sports event polling and display — existing
- ✓ Odds history snapshots for historical charting — existing

### Active

- [ ] Accurate cross-platform sports event matching — no false negatives (events on both platforms but only showing one platform's odds)
- [ ] No duplicate events — each real-world event appears exactly once
- [ ] Correct event dates — dates shown match actual event dates
- [ ] All sports coverage — any sport on either platform appears in the sports tab
- [ ] Both odds shown when available — if an event is on both platforms, show both sets of odds; if only on one, show that platform's odds

### Out of Scope

- Autonomous agent trading — future milestone, not this work
- Non-sports matching improvements — culture/politics matching not in scope for this milestone
- New platform integrations — only Polymarket and Kalshi for now
- Mobile app — web only

## Context

- Previously used DomeAPI for pre-matched cross-platform events. DomeAPI stopped working, so the team built a native matching engine in Rust (`backend-rust/src/matching/`).
- Current native matcher uses multi-signal scoring (team name similarity, date, sport) with Hungarian algorithm for optimal assignment, but it has significant gaps:
  - Duplicate events appearing in the UI
  - Events listed on both platforms only showing odds from one
  - Incorrect dates on some events
- The matching engine is the largest and most complex part of the backend: `matcher.rs` (37k), `team_dictionary.rs` (58k)
- Fetchers (`fetcher_kalshi.rs`, `fetcher_polymarket.rs`) pull candidate events from each platform's API; matcher scores and pairs them
- Frontend polls `GET /api/v2/events` every 15 seconds

## Constraints

- **Tech stack**: Rust backend (Axum/Tokio), Next.js frontend — preserve existing architecture
- **Data sources**: Kalshi REST API and Polymarket Gamma API — no additional paid data providers
- **Platform**: Both platforms use different naming conventions, date formats, and market structures for the same events

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Replace DomeAPI with native matching | DomeAPI stopped working | — Pending (in progress, quality not yet sufficient) |
| Hungarian algorithm for optimal assignment | Prevents many-to-one matching errors | — Pending |
| Possible full refactor of matching system | Current approach may have fundamental issues vs incremental fixes | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-03-29 after initialization*
