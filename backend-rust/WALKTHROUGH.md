# ArbiAgent Architecture Walkthrough

> For AI agents picking up this codebase. Read this first.

## What ArbiAgent Does

ArbiAgent scans for arbitrage opportunities across prediction markets (Kalshi and Polymarket). It finds the same event on both platforms, compares prices, and surfaces profitable discrepancies.

## Three-Layer Stack

```
Frontend (Next.js 16)  →  Backend (Rust/Actix)  →  Platform APIs (Kalshi, Polymarket)
   :3000                     :8080
```

- **Frontend**: `frontend/` — App Router, React 19, Tailwind 4. Polls `GET /api/v2/events` every 15s. Prices arrive as 0-1 floats, displayed as 0-100 cents.
- **Backend**: `backend-rust/` — Actix-web, Tokio async. All business logic lives here.
- **Orchestration**: `directives/` for SOPs, `execution/` for deterministic scripts (currently empty).

## Backend Data Flow

```
Event Discovery → Price Fetching → Arb Detection → API / WebSocket → Frontend
```

### 1. Event Discovery (how markets get matched)

**Native matching engine** (`src/matching/`) — the primary path (`ENABLE_NATIVE_MATCHING=true`, default).

- `fetcher_kalshi.rs` fetches sports events via `GET /events?series_ticker=KX{SPORT}`
- `fetcher_polymarket.rs` fetches via Gamma API `GET /events?tag={sport}`
- Both normalize into `CandidateEvent` structs (team abbreviations, dates, titles)
- `matcher.rs` scores every Kalshi×Polymarket pair (teams=50pts, date=20pts, title=20pts, moneyline=10pts), accepts matches ≥60
- `label_resolver.rs` resolves Polymarket token IDs to Kalshi-style outcome abbreviations via Gamma API
- Output: `CanonicalEvent` with both platforms' IDs, inserted into `StateCache`

**DomeAPI fallback** (`src/ingestion/dome_poller.rs`) — legacy path when `ENABLE_NATIVE_MATCHING=false`. Requires `DOME_API_KEY`. Rate-limited to 1 req/sec.

**Culture events** (`src/ingestion/culture_poller.rs`) — non-sports events (politics, crypto, etc.) discovered independently per platform. No cross-platform matching yet.

### 2. Price Fetching

- **Primary**: `src/ingestion/direct_api.rs` — batch fetches from Kalshi `GET /markets?tickers=...` and Polymarket `POST /prices` concurrently. Runs every 10s.
- **WebSockets**: `kalshi_ws.rs` and `polymarket_ws.rs` provide sub-second updates when enabled.
- **DomeAPI fallback**: `dome_poller.rs` price refresh (slow, 1 req/sec). Only used when `ENABLE_DIRECT_PRICE_API=false`.

### 3. Arb Detection

`src/processing/arb_detector.rs` — runs every 2 seconds. For each event with odds on both platforms:
- Computes cost of buying YES on one platform + NO on the other
- If total cost < 1.0 minus fees, it's an arb
- Deduplicates complementary arbs (YES-A/NO-B is the same trade as YES-B/NO-A)
- Broadcasts via WebSocket, logs to Supabase

### 4. API Layer

`src/api/routes.rs` — REST endpoints under `/api/v2/`:
- `GET /events` — list events (filterable by sport, status)
- `GET /arbitrage` — active arb opportunities
- `GET /arbitrage/stats` — aggregate statistics
- WebSocket endpoint for real-time push

## Key Data Types

- **`CanonicalEvent`** (`models/event.rs`): The core entity. Has an ID like `nba-lal-bos-2026-03-14`, a `Sport` enum, and `PlatformIds` containing Kalshi tickers + Polymarket token IDs.
- **`OutcomePrice`**: `yes_price` / `no_price` as 0.0-1.0 floats.
- **`EventOdds`**: `platform → outcome → OutcomePrice` nested map.
- **`ArbitrageOpportunity`** (`models/arb.rs`): Buy/sell legs, margin, score.

## Key Config (env vars)

| Var | Default | What it does |
|-----|---------|-------------|
| `ENABLE_NATIVE_MATCHING` | `true` | Use native matcher vs DomeAPI |
| `ENABLE_DIRECT_PRICE_API` | `true` | Batch price APIs vs DomeAPI prices |
| `ENABLE_KALSHI_WS` | `false` | Real-time Kalshi WebSocket |
| `ENABLE_POLYMARKET_WS` | `true` | Real-time Polymarket WebSocket |
| `ENABLE_SHADOW_MATCHING` | `false` | Run DomeAPI in parallel for comparison |
| `MATCH_MIN_SCORE` | `60.0` | Minimum score to accept a cross-platform match |
| `ARB_MIN_MARGIN_PCT` | `1.0` | Minimum arb margin to surface |
| `DOME_API_KEY` | (empty) | Only needed when native matching is off |

## Module Map

```
backend-rust/src/
  main.rs              — server bootstrap, spawns all background tasks
  config.rs            — env var loading
  matching/            — native cross-platform event matching (NEW)
    mod.rs             — discovery loop entry point
    fetcher_kalshi.rs  — Kalshi events API fetcher
    fetcher_polymarket.rs — Gamma API fetcher
    matcher.rs         — scoring algorithm + bipartite assignment
    team_dictionary.rs — 120+ teams across NBA/NFL/MLB/NHL
    candidate.rs       — CandidateEvent normalization
    label_resolver.rs  — Polymarket outcome label resolution
    shadow.rs          — DomeAPI comparison mode
  ingestion/
    dome_poller.rs     — DomeAPI discovery + price refresh (legacy fallback)
    direct_api.rs      — batch price fetching (primary)
    kalshi_ws.rs       — Kalshi WebSocket ingester
    polymarket_ws.rs   — Polymarket WebSocket ingester
    culture_poller.rs  — non-sports event discovery
  processing/
    arb_detector.rs    — arbitrage detection loop
  models/              — shared data types
  storage/             — Supabase client + in-memory cache
  api/                 — REST routes + WebSocket handler

frontend/
  app/markets/page.tsx — main markets page
  components/          — EventCard, EventRow, EventTable
  lib/api.ts           — API client, transforms 0-1 prices to 0-100 cents
```

## Common Tasks

**Add a new sport**: Add team entries to `matching/team_dictionary.rs`, add the series ticker to `fetcher_kalshi.rs`, add the tag to `fetcher_polymarket.rs`. No other changes needed.

**Tune matching sensitivity**: Adjust `MATCH_MIN_SCORE` env var (higher = stricter, lower = more matches).

**Debug a bad match**: Enable `ENABLE_SHADOW_MATCHING=true` with a valid `DOME_API_KEY` to compare native vs DomeAPI output in logs.

**Switch back to DomeAPI**: Set `ENABLE_NATIVE_MATCHING=false` and provide `DOME_API_KEY`.
