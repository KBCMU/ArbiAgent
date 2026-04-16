---
focus: arch
generated: 2026-03-29
---

# Codebase Structure

**Analysis Date:** 2026-03-29

## Directory Layout

```
arbiagent/
├── backend-rust/           # Rust backend service (Axum + Tokio)
│   ├── src/
│   │   ├── main.rs         # Binary entry point, task orchestration
│   │   ├── config.rs       # AppConfig loaded from env vars
│   │   ├── api/            # HTTP routes + WebSocket server
│   │   ├── ingestion/      # Market data pollers + WS ingesters
│   │   ├── matching/       # Native cross-platform event matching engine
│   │   ├── models/         # Shared data types
│   │   ├── processing/     # Arbitrage detection engine
│   │   └── storage/        # In-memory cache + Supabase client
│   ├── migrations/
│   │   └── 001_initial_schema.sql   # Supabase Postgres schema
│   └── Cargo.toml
├── frontend/               # Next.js 16 App Router frontend
│   ├── app/                # Next.js app directory (pages + layouts)
│   │   ├── layout.tsx      # Root layout (ThemeProvider, fonts)
│   │   ├── page.tsx        # Landing page
│   │   ├── globals.css     # Global Tailwind CSS
│   │   ├── global-error.tsx
│   │   ├── markets/        # Prediction markets dashboard
│   │   ├── arbitrage/      # Arbitrage opportunities view
│   │   ├── bet-tracker/    # Bet tracking page
│   │   ├── crypto/         # Crypto markets page
│   │   ├── learn/          # Educational content
│   │   ├── pricing/        # Pricing tiers page
│   │   └── auth/           # Login / signup / callback
│   ├── components/         # Shared React components
│   ├── lib/                # API client, utilities, Supabase clients
│   ├── public/             # Static assets (platform logos, icons)
│   ├── next.config.ts      # Next.js configuration
│   ├── tsconfig.json       # TypeScript config
│   ├── package.json        # Node dependencies
│   └── vercel.json         # Vercel deployment config
├── directives/             # Agent SOPs in Markdown
│   ├── orchestration.md    # High-level orchestration directive
│   └── agents/             # Role-specific agent directives (18 files)
├── execution/              # Python execution scripts (currently empty)
├── tasks/                  # Session tracking files
│   ├── todo.md             # Current task plan
│   └── lessons.md          # Accumulated learnings
├── .planning/              # AI planning artifacts
│   └── codebase/           # Codebase analysis docs
├── .tmp/                   # Temporary intermediate files (not committed)
├── .cursor/                # Cursor editor rules
├── CLAUDE.md               # AI agent instructions (primary)
├── AGENTS.md               # Mirror of CLAUDE.md for other agents
├── README.md               # Project overview and setup guide
├── DEPLOYMENT.md           # Deployment instructions
└── .env                    # Root-level environment variables
```

## Directory Purposes

**`backend-rust/src/api/`:**
- Purpose: HTTP REST API and WebSocket server
- Contains: `mod.rs` (module exports), `routes.rs` (all REST handlers, Axum router), `ws_server.rs` (WebSocket broadcast at `/ws/arb-feed`)
- Key files: `backend-rust/src/api/routes.rs` (9 REST endpoints), `backend-rust/src/api/ws_server.rs`

**`backend-rust/src/ingestion/`:**
- Purpose: All data acquisition from external market APIs
- Contains: `dome_poller.rs` (DomeAPI REST poller, legacy), `direct_api.rs` (direct Kalshi+Polymarket batch APIs), `culture_poller.rs` (Polymarket Gamma + Kalshi direct for non-sports), `kalshi_ws.rs` (Kalshi WebSocket), `polymarket_ws.rs` (Polymarket WebSocket), `mod.rs`
- Key files: `backend-rust/src/ingestion/direct_api.rs` (primary price source), `backend-rust/src/ingestion/culture_poller.rs`

**`backend-rust/src/matching/`:**
- Purpose: Native engine for correlating events across Kalshi and Polymarket
- Contains: `matcher.rs` (main scoring + assignment, largest file at 37k), `fetcher_kalshi.rs`, `fetcher_polymarket.rs`, `candidate.rs` (CandidateEvent struct), `team_dictionary.rs` (team name mappings, 58k), `label_resolver.rs`, `hungarian.rs` (optimal assignment), `shadow.rs` (DomeAPI shadow mode), `mod.rs`
- Key files: `backend-rust/src/matching/matcher.rs`, `backend-rust/src/matching/team_dictionary.rs`

**`backend-rust/src/models/`:**
- Purpose: Shared data model types used across all backend modules
- Contains: `event.rs` (`CanonicalEvent`, `Sport` enum, `EventOdds`, `OutcomePrice`), `arb.rs` (`ArbitrageOpportunity`), `platform.rs` (API response shapes), `ws.rs` (`WsMessage`, `ArbUpdateData`), `mod.rs`
- Key files: `backend-rust/src/models/event.rs` (central domain model)

**`backend-rust/src/processing/`:**
- Purpose: Arbitrage detection over current cache state
- Contains: `arb_detector.rs` (2-second detection loop, outcome label validation), `mod.rs`
- Key files: `backend-rust/src/processing/arb_detector.rs`

**`backend-rust/src/storage/`:**
- Purpose: In-memory state cache and Supabase persistence client
- Contains: `cache.rs` (`StateCache` with DashMap collections, `MatchingStats`), `supabase.rs` (Postgres client via `sqlx`, snapshot writer loop), `mod.rs`
- Key files: `backend-rust/src/storage/cache.rs`, `backend-rust/src/storage/supabase.rs`

**`frontend/app/`:**
- Purpose: Next.js App Router pages — each subdirectory is a route segment
- Contains: Route-specific `page.tsx` files; `layout.tsx` files for nested layouts
- Key files: `frontend/app/page.tsx` (landing, 30k), `frontend/app/markets/page.tsx` (main dashboard), `frontend/app/arbitrage/page.tsx`

**`frontend/components/`:**
- Purpose: Shared React components used across multiple pages
- Contains: `Sidebar.tsx`, `Header.tsx`, `FilterBar.tsx`, `EventTable.tsx`, `EventRow.tsx`, `EventCard.tsx`, `ArbitrageTable.tsx`, `ConnectionError.tsx`, `MarketTypeFilter.tsx`, `Logo.tsx`, `ThemeProvider.tsx`
- Key files: `frontend/components/EventTable.tsx`, `frontend/components/EventRow.tsx`, `frontend/components/FilterBar.tsx`

**`frontend/lib/`:**
- Purpose: Frontend utility modules — API client and Supabase auth clients
- Contains: `api.ts` (typed REST client, all `fetchX` functions, frontend TypeScript types), `utils.ts`, `supabase/client.ts` (browser Supabase client), `supabase/server.ts` (server-side Supabase client), `supabase/middleware.ts` (Next.js middleware for auth)
- Key files: `frontend/lib/api.ts` (defines all frontend data types and REST fetch functions)

**`directives/agents/`:**
- Purpose: Role-specific SOP documents for AI agents (18 roles: rust-engineer, typescript-pro, websocket-engineer, etc.)
- Generated: No — authored SOPs
- Committed: Yes

**`execution/`:**
- Purpose: Python execution scripts per the 3-layer architecture pattern
- Currently empty — all backend work is in the Rust binary

**`tasks/`:**
- Purpose: Session-level planning and learning artifacts
- Contains: `todo.md` (current tasks), `lessons.md` (accumulated debugging lessons)

## Key File Locations

**Entry Points:**
- `backend-rust/src/main.rs`: Rust binary main — initializes all state and spawns background tasks
- `frontend/app/layout.tsx`: Next.js root layout wrapping all pages
- `frontend/app/page.tsx`: Landing page (public-facing)
- `frontend/app/markets/page.tsx`: Main authenticated dashboard

**Configuration:**
- `backend-rust/src/config.rs`: All backend env var parsing; feature flag definitions
- `backend-rust/.env` (not tracked): Backend secrets (DOME_API_KEY, SUPABASE_*, KALSHI_*)
- `frontend/.env.local` (not tracked): NEXT_PUBLIC_SUPABASE_URL, NEXT_PUBLIC_SUPABASE_ANON_KEY
- `frontend/next.config.ts`: Next.js build config
- `frontend/tsconfig.json`: TypeScript path aliases (`@/` → project root)

**Core Logic:**
- `backend-rust/src/matching/matcher.rs`: Cross-platform event scoring and assignment
- `backend-rust/src/matching/team_dictionary.rs`: Team name normalization dictionary
- `backend-rust/src/processing/arb_detector.rs`: Arbitrage detection with fee accounting
- `backend-rust/src/storage/cache.rs`: `StateCache` — the central shared state struct
- `frontend/lib/api.ts`: All frontend types and REST API calls

**Database:**
- `backend-rust/migrations/001_initial_schema.sql`: Full Postgres schema for Supabase
- `backend-rust/src/storage/supabase.rs`: All SQL queries and snapshot writer

**Auth:**
- `frontend/lib/supabase/middleware.ts`: Next.js middleware for session refresh
- `frontend/lib/supabase/server.ts`: Server-side Supabase client for RSC/route handlers
- `frontend/lib/supabase/client.ts`: Browser-side Supabase client
- `frontend/app/auth/login/`, `frontend/app/auth/signup/`, `frontend/app/auth/callback/`: Auth pages

## Naming Conventions

**Backend (Rust):**
- Files: `snake_case.rs` for all modules
- Modules map 1:1 to files; module hierarchy matches directory structure
- Async loop functions named `run_X_loop` (e.g. `run_arb_detection_loop`, `run_sports_matching_loop`)
- Struct names: `PascalCase` (e.g. `CanonicalEvent`, `StateCache`, `AppConfig`)
- Background task spawn pattern: clone `Arc<AppState>`, pass to `tokio::spawn`

**Frontend (TypeScript):**
- Files: `PascalCase.tsx` for components (e.g. `FilterBar.tsx`, `EventRow.tsx`), `camelCase.ts` for utilities (e.g. `api.ts`, `utils.ts`)
- Pages: `page.tsx` following Next.js App Router convention
- API fetch functions named `fetchX` (e.g. `fetchMatchedEvents`, `fetchArbitrageOpportunities`)
- Types: `PascalCase` interfaces (e.g. `MatchedEvent`, `ActiveArb`, `OutcomeOdds`)

## Where to Add New Code

**New backend data source / ingester:**
- Implementation: `backend-rust/src/ingestion/your_source.rs`
- Register module in: `backend-rust/src/ingestion/mod.rs`
- Spawn the loop in: `backend-rust/src/main.rs`

**New backend API endpoint:**
- Add handler function and `.route(...)` call in: `backend-rust/src/api/routes.rs`

**New data model:**
- Add struct/enum to: `backend-rust/src/models/event.rs` (domain types) or `backend-rust/src/models/platform.rs` (API response shapes)
- Export from: `backend-rust/src/models/mod.rs`

**New frontend page:**
- Create directory: `frontend/app/your-page/`
- Add file: `frontend/app/your-page/page.tsx`

**New frontend component:**
- Add file: `frontend/components/YourComponent.tsx`
- Use `"use client"` directive if the component needs React state/effects

**New API type or fetch function:**
- Add to: `frontend/lib/api.ts`

**New database table:**
- Create migration: `backend-rust/migrations/00N_description.sql`
- Apply manually against Supabase project

## Special Directories

**`.planning/codebase/`:**
- Purpose: AI-generated codebase analysis documents
- Generated: Yes (by mapping agents)
- Committed: Yes

**`.tmp/`:**
- Purpose: Intermediate files during processing (temporary exports, scraped data)
- Generated: Yes (regenerated as needed)
- Committed: No (in `.gitignore`)

**`backend-rust/target/`:**
- Purpose: Rust build artifacts
- Generated: Yes (`cargo build`)
- Committed: No (in `.gitignore`)

**`frontend/.next/`:**
- Purpose: Next.js build output
- Generated: Yes (`npm run build`)
- Committed: No (in `.gitignore`)

---

*Structure analysis: 2026-03-29*
