---
focus: arch
generated: 2026-03-29
---

# Architecture

**Analysis Date:** 2026-03-29

## Pattern Overview

**Overall:** Two-tier distributed system — a Rust backend service with async background loops + an in-memory cache, fronted by a Next.js SSR/client frontend. Not a microservices architecture; all backend logic runs in a single Rust binary with concurrent Tokio tasks.

**Key Characteristics:**
- Backend is a single Axum process: all background workers, WebSocket server, and REST API share one `Arc<AppState>` instance
- Primary data store is an in-memory `DashMap`-backed cache (`StateCache`); Supabase Postgres is a secondary persistence layer for history and snapshots
- Frontend polls the REST API on a 15-second interval and optionally connects to the WebSocket feed for real-time arb push
- Event discovery and price ingestion are decoupled: discovery runs on a longer interval (configurable), price refresh runs more frequently

## Layers

**Ingestion Layer:**
- Purpose: Fetches raw market data from external platforms
- Location: `backend-rust/src/ingestion/`
- Contains: REST pollers (DomeAPI, direct Kalshi/Polymarket batch APIs), WebSocket ingesters (Kalshi, Polymarket), culture event poller
- Depends on: `storage::cache` (writes odds), `models::event`
- Used by: `main.rs` spawns each ingester as an independent Tokio task

**Matching Layer:**
- Purpose: Correlates events across Kalshi and Polymarket into canonical paired events
- Location: `backend-rust/src/matching/`
- Contains: `matcher.rs` (scoring algorithm), `fetcher_kalshi.rs`, `fetcher_polymarket.rs`, `candidate.rs`, `team_dictionary.rs`, `label_resolver.rs`, `hungarian.rs` (assignment algorithm), `shadow.rs`
- Depends on: External Kalshi API, Polymarket Gamma API; writes to `storage::cache`
- Used by: `main.rs` spawns `run_sports_matching_loop`

**Processing Layer:**
- Purpose: Detects arbitrage opportunities from current odds in the cache
- Location: `backend-rust/src/processing/`
- Contains: `arb_detector.rs` — runs every 2 seconds
- Depends on: `storage::cache` (reads odds, writes active arbs), `models::arb`, `storage::supabase` (persists detected arbs)
- Used by: `main.rs` spawns `run_arb_detection_loop`

**Storage Layer:**
- Purpose: In-memory cache + Supabase persistence
- Location: `backend-rust/src/storage/`
- Contains: `cache.rs` (`StateCache` struct with DashMap for events, odds, active arbs), `supabase.rs` (Postgres client, snapshot writer)
- Depends on: `models::event`, `models::arb`
- Used by: all other backend layers

**API Layer:**
- Purpose: Exposes REST endpoints and WebSocket feed to the frontend
- Location: `backend-rust/src/api/`
- Contains: `routes.rs` (Axum router, all REST handlers), `ws_server.rs` (WebSocket broadcast endpoint at `/ws/arb-feed`), `mod.rs`
- Depends on: `storage::cache` (read-only), `storage::supabase` (history queries)
- Used by: frontend HTTP client (`frontend/lib/api.ts`)

**Models Layer:**
- Purpose: Shared data types used across all backend layers
- Location: `backend-rust/src/models/`
- Contains: `event.rs` (`CanonicalEvent`, `Sport` enum, `EventOdds`), `arb.rs` (`ArbitrageOpportunity`), `platform.rs` (API response structs), `ws.rs` (`WsMessage`)

**Frontend:**
- Purpose: Dashboard UI for viewing matched events, arb opportunities, auth
- Location: `frontend/`
- Contains: Next.js App Router pages, React components, API client
- Depends on: Backend REST API (polled), Supabase Auth

## Data Flow

**Sports Event Discovery:**
1. `matching::run_sports_matching_loop` fires on a configurable interval
2. `fetcher_kalshi` and `fetcher_polymarket` fetch candidates concurrently via `tokio::join!`
3. `matcher::match_candidates` scores all cross-platform pairs using a multi-signal algorithm (team name similarity, date, sport); Hungarian algorithm resolves optimal assignment
4. `label_resolver` resolves Polymarket outcome labels via Gamma API for matched pairs
5. Resulting `CanonicalEvent` records are upserted into `StateCache.events` and written to Supabase `canonical_events` table

**Culture Event Discovery (Non-Sports):**
1. `ingestion::culture_poller::run_culture_discovery_loop` runs independently
2. Fetches from Polymarket Gamma API and Kalshi direct API
3. Upserts into same `StateCache.events` and Supabase

**Price Ingestion:**
1. `ingestion::direct_api::run_direct_price_refresh_loop` polls Kalshi and Polymarket batch APIs
2. Calls `StateCache::update_odds(event_id, platform, outcome, price)`
3. Optionally, `ingestion::kalshi_ws` and `ingestion::polymarket_ws` maintain WebSocket connections for real-time price ticks and write to cache on each tick

**Arbitrage Detection:**
1. `processing::arb_detector::run_arb_detection_loop` runs every 2 seconds
2. Reads all events+odds from cache via `StateCache::get_all_events_with_odds()`
3. For each dual-platform event, validates outcome label alignment (corrects inverted labels)
4. Computes cross-platform margins accounting for platform fees (Kalshi ~3%, Polymarket 0%)
5. Active arbs written to `StateCache.active_arbs`; detected arbs broadcast via `arb_tx` broadcast channel to all WebSocket subscribers; persisted to Supabase `arbitrage_opportunities` table

**Frontend Data Consumption:**
1. `frontend/app/markets/page.tsx` polls `GET /api/v2/events` every 15 seconds via `fetchMatchedEvents` in `frontend/lib/api.ts`
2. `frontend/app/arbitrage/page.tsx` polls `GET /api/v2/arbitrage`
3. WebSocket at `/ws/arb-feed` pushes real-time arb updates; frontend subscribes for live updates
4. On WebSocket connect, backend sends initial state snapshot, then streams `WsMessage` JSON frames on each arb change

**Snapshot Writing:**
1. `storage::supabase::run_snapshot_writer` runs on a configurable interval
2. Iterates all current odds in cache
3. Writes rows to `odds_snapshots` table in Supabase for historical charting via `GET /api/v2/events/{id}/odds-history`

## Entry Points

**Backend Binary:**
- Location: `backend-rust/src/main.rs`
- Triggers: `cargo run` in `backend-rust/`
- Responsibilities: Loads config from env, initializes `StateCache` and `SupabaseClient`, constructs `Arc<AppState>`, spawns all background Tokio tasks, starts Axum HTTP server on `host:port`

**Frontend App:**
- Location: `frontend/app/layout.tsx` (root), `frontend/app/page.tsx` (landing)
- Triggers: `npm run dev` or `npm run build && npm start` in `frontend/`
- Responsibilities: Wraps all routes in `ThemeProvider`, renders landing page, routes to `/markets`, `/arbitrage`, `/bet-tracker`, `/auth/*`, `/pricing`

**Background Task Entry Points (all in `main.rs`):**
- `matching::run_sports_matching_loop` — sports event discovery
- `ingestion::culture_poller::run_culture_discovery_loop` — culture event discovery
- `ingestion::direct_api::run_direct_price_refresh_loop` — price refresh
- `ingestion::kalshi_ws::run_kalshi_ws_ingester` — optional Kalshi WebSocket
- `ingestion::polymarket_ws::run_polymarket_ws_ingester` — optional Polymarket WebSocket
- `processing::arb_detector::run_arb_detection_loop` — arb detection every 2s
- `storage::supabase::run_snapshot_writer` — periodic odds persistence
- `run_cache_eviction_loop` (defined in `main.rs`) — cache cleanup every 30 minutes

## Database / Storage Design

**In-Memory Cache (`StateCache` in `backend-rust/src/storage/cache.rs`):**
- `events: DashMap<String, CanonicalEvent>` — all discovered events keyed by ID (e.g. `"nba-bos-mia-2026-03-29"`)
- `odds: DashMap<String, EventOdds>` — current odds keyed by event ID; `EventOdds.platform_odds` is `HashMap<platform, HashMap<outcome, OutcomePrice>>`
- `active_arbs: DashMap<String, Vec<ArbitrageOpportunity>>` — current detected arbs keyed by event ID
- `matching_stats: RwLock<Option<MatchingStats>>` — last matching cycle statistics

**Supabase Postgres (schema in `backend-rust/migrations/001_initial_schema.sql`):**
- `canonical_events` — persistent record of all discovered events; RLS-enabled with public reads
- `odds_snapshots` — periodic odds snapshots for historical charting; referenced by `GET /api/v2/events/{id}/odds-history`
- `arbitrage_opportunities` — historical arb records with `closed_at` and `duration_ms` for closed windows

**Supabase Auth:**
- Frontend uses Supabase Auth for email/password login via `frontend/lib/supabase/`
- Row Level Security is enforced on all tables; service role key used by backend for writes

## Error Handling

**Strategy:** Best-effort with logging; backend never panics on data path errors

**Patterns:**
- DB writes from ingestion/matching use `if let Err(e) = ... { warn!(...) }` — failures are logged but do not halt the pipeline
- Background loops log errors via `tracing::error!` and continue looping on next interval
- API route handlers return 404/empty arrays rather than 500s when cache misses occur
- WebSocket send errors silently drop the client connection

## Cross-Cutting Concerns

**Logging:** `tracing` crate with `tracing_subscriber::fmt`; log level set via `RUST_LOG` env var, default `arbiagent_backend=info`
**Configuration:** All tunable values in `AppConfig` loaded from environment variables in `backend-rust/src/config.rs`; feature flags as booleans (e.g. `ENABLE_NATIVE_MATCHING`, `ENABLE_KALSHI_WS`)
**Concurrency:** `Arc<AppState>` shared across all Tokio tasks; `DashMap` provides lock-free concurrent access to cache maps; broadcast channel (`tokio::sync::broadcast`) decouples arb detector from WebSocket clients

---

*Architecture analysis: 2026-03-29*
