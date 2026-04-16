\# Agent Instructions

\> This file is mirrored across CLAUDE.md, AGENTS.md, and GEMINI.md so the same instructions load in any AI environment.

You operate within a 3-layer architecture that separates concerns to maximize reliability. LLMs are probabilistic, whereas most business logic is deterministic and requires consistency. This system fixes that mismatch.

\#\# The 3-Layer Architecture

\*\*Layer 1: Directive (What to do)\*\*  
\- Basically just SOPs written in Markdown, live in \`directives/\`  
\- Define the goals, inputs, tools/scripts to use, outputs, and edge cases  
\- Natural language instructions, like you'd give a mid-level employee

\*\*Layer 2: Orchestration (Decision making)\*\*  
\- This is you. Your job: intelligent routing.  
\- Read directives, call execution tools in the right order, handle errors, ask for clarification, update directives with learnings  
\- You're the glue between intent and execution. E.g you don't try scraping websites yourself—you read \`directives/scrape\_website.md\` and come up with inputs/outputs and then run \`execution/scrape\_single\_site.py\`

\*\*Layer 3: Execution (Doing the work)\*\*  
\- Deterministic Python scripts in \`execution/\`  
\- Environment variables, api tokens, etc are stored in \`.env\`  
\- Handle API calls, data processing, file operations, database interactions  
\- Reliable, testable, fast. Use scripts instead of manual work. Commented well.

\*\*Why this works:\*\* if you do everything yourself, errors compound. 90% accuracy per step \= 59% success over 5 steps. The solution is push complexity into deterministic code. That way you just focus on decision-making.

\#\# Operating Principles

\*\*1. Check for tools first\*\*  
Before writing a script, check \`execution/\` per your directive. Only create new scripts if none exist.

\*\*2. Self-anneal when things break\*\*  
\- Read error message and stack trace  
\- Fix the script and test it again (unless it uses paid tokens/credits/etc—in which case you check w user first)  
\- Update the directive with what you learned (API limits, timing, edge cases)  
\- Example: you hit an API rate limit → you then look into API → find a batch endpoint that would fix → rewrite script to accommodate → test → update directive.

\*\*3. Update directives as you learn\*\*  
Directives are living documents. When you discover API constraints, better approaches, common errors, or timing expectations—update the directive. But don't create or overwrite directives without asking unless explicitly told to. Directives are your instruction set and must be preserved (and improved upon over time, not extemporaneously used and then discarded).

\#\# Self-annealing loop

Errors are learning opportunities. When something breaks:  
1\. Fix it  
2\. Update the tool  
3\. Test tool, make sure it works  
4\. Update directive to include new flow  
5\. System is now stronger

\#\# File Organization

\*\*Deliverables vs Intermediates:\*\*  
\- \*\*Deliverables\*\*: Google Sheets, Google Slides, or other cloud-based outputs that the user can access  
\- \*\*Intermediates\*\*: Temporary files needed during processing

\*\*Directory structure:\*\*  
\- \`.tmp/\` \- All intermediate files (dossiers, scraped data, temp exports). Never commit, always regenerated.  
\- \`execution/\` \- Python scripts (the deterministic tools)  
\- \`directives/\` \- SOPs in Markdown (the instruction set)  
\- \`.env\` \- Environment variables and API keys  
\- \`credentials.json\`, \`token.json\` \- Google OAuth credentials (required files, in \`.gitignore\`)

\*\*Key principle:\*\* Local files are only for processing. Deliverables live in cloud services (Google Sheets, Slides, etc.) where the user can access them. Everything in \`.tmp/\` can be deleted and regenerated.

\#\# Summary

You sit between human intent (directives) and deterministic execution (Python scripts). Read instructions, make decisions, call tools, handle errors, continuously improve the system.

Be pragmatic. Be reliable. Self-anneal.

# SESSION START

1. Read tasks/lessons.md — apply all lessons before touching anything
2. Read tasks/todo.md — understand current state
3. If neither exists, create them before starting

# WORKFLOW

## 1. Plan First
- Enter plan mode for any non-trivial task (3+ steps)
- Write plan to tasks/todo.md before implementing
- If something goes wrong, STOP and re-plan — never push through

## 2. Subagent Strategy
- Use subagents to keep main context clean
- One task per subagent
- Throw more compute at hard problems

## 3. Self-Improvement Loop
- After any correction: update tasks/lessons.md
- Format: [date] | what went wrong | rule to prevent it
- Review lessons at every session start

## 4. Verification Standard
- Never mark complete without proving it works
- Run tests, check logs, diff behavior
- Ask: "Would a staff engineer approve this?"

## 5. Demand Elegance
- For non-trivial changes: is there a more elegant solution?
- If a fix feels hacky: rebuild it properly
- Don't over-engineer simple things

## 6. Autonomous Bug Fixing
- When given a bug: just fix it
- Go to logs, find root cause, resolve it
- No hand-holding needed

# CORE PRINCIPLES
- Simplicity First — touch minimal code
- No Laziness — root causes only, no temp fixes
- Never Assume — verify paths, APIs, variables before using
- Ask Once — one question upfront if unclear, never interrupt mid-task

# TASK MANAGEMENT
1. Plan → tasks/todo.md
2. Verify → confirm before implementing
3. Track → mark complete as you go
4. Explain → high-level summary each step
5. Learn → tasks/lessons.md after corrections

# LEARNED
(Claude fills this in over time)

<!-- GSD:project-start source:PROJECT.md -->
## Project

**ArbiAgent**

A prediction market arbitrage scanner that discovers events across Polymarket and Kalshi, matches corresponding events across platforms, and identifies arbitrage opportunities. Built as a Rust backend (Axum + Tokio) with a Next.js frontend. Currently focused on improving cross-platform sports event matching accuracy.

**Core Value:** Accurately match the same sporting event across Polymarket and Kalshi so users can compare odds and spot arbitrage opportunities.

### Constraints

- **Tech stack**: Rust backend (Axum/Tokio), Next.js frontend — preserve existing architecture
- **Data sources**: Kalshi REST API and Polymarket Gamma API — no additional paid data providers
- **Platform**: Both platforms use different naming conventions, date formats, and market structures for the same events
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

## What This Research Is About
## Current Approach Inventory
| Component | Technique | File |
|-----------|-----------|------|
| Team normalization | Static keyword dictionary → canonical 3-letter abbreviation | `team_dictionary.rs` (58k) |
| Title similarity | Jaccard token overlap on normalized titles | `candidate.rs::jaccard_token_similarity` |
| Date extraction | Ticker segment parsing + regex on titles | `candidate.rs` |
| Assignment | Kuhn-Munkres (Hungarian) O(n³), hand-rolled | `hungarian.rs` |
| Bucketing | (sport, date, market_type) bucket → per-bucket matching | `matcher.rs` |
| Fallback | Cross-bucket retry when date differs | `matcher.rs::match_within_bucket` |
| Score weighting | `WEIGHT_BOTH_TEAMS=50, TITLE_SIM=20, DATE=20, MARKET_TYPE=10` | `matcher.rs` |
## Recommended Stack Changes
### 1. String Similarity: Replace Jaccard with Edit-Distance Metrics
# Cargo.toml
- Pure Rust, no unsafe, zero dependencies — compiles cleanly in the existing workspace
- Version 0.11 (2024) includes Jaro-Winkler, Damerau-Levenshtein, normalized Levenshtein, Hamming
- Confidence: HIGH — this is the canonical string metrics crate in the Rust ecosystem,
- `jaro_winkler` is the right choice here because it rewards common prefixes, which is exactly
- Keep Jaccard for title-level comparison (still useful for multi-word overlap)
- Add Jaro-Winkler as a fallback in `compute_team_score` when exact canonical match fails:
- This handles cases where team extraction succeeds on one side but not the other
- `fuzzy-matcher` (FZF algorithm) — designed for interactive UI filtering, not deterministic
- `rapidfuzz` Python port crates — immature, no stable Rust crate as of 2025
- Custom Levenshtein — `strsim` already implements it correctly and is battle-tested
### 2. Date Resolution: Use Platform API Timestamps, Not Parsed Strings
### 3. Deduplication: Canonical Event ID as Primary Dedup Key
- The cross-bucket fallback at line 111–169 does not check `matched_kalshi`/`matched_poly` early
- More critically: `build_event_id` is called to produce the canonical ID used as the DashMap key.
### 4. Fuzzy Team Matching: Add `edit-distance` Crate for Candidate Extraction
# Same strsim crate, already recommended above
### 5. Sports Data APIs: No New External Providers Needed
| Field | Platform | Currently Used? | Recommendation |
|-------|----------|-----------------|----------------|
| `close_time` | Kalshi | Partial | Use as primary date source |
| `startDate` | Polymarket Gamma | Partial | Use as primary date source |
| `sub_title` | Kalshi | No | Parse for opponent team name |
| `series_ticker` | Kalshi | No | Use as sport/league discriminator |
| `tags[].label` | Polymarket | Partial | Use "nba", "nfl" tags for sport confirmation |
| `tags[].slug` | Polymarket | No | Use slugs like "nba-playoffs" for market type |
### 6. Scoring Architecture: Weight Recalibration
### 7. What NOT to Use
| Approach | Why Not |
|----------|---------|
| **External NLP/ML models** | Overkill for structured data matching; adds latency, deployment complexity, and violates the "no new external providers" constraint; Kalshi/Polymarket data is structured enough for rule-based matching |
| **Elasticsearch/Solr fuzzy search** | Requires a separate service; the dataset is ~50-200 events at any time — not a search scale problem |
| **Rapidfuzz Rust ports** (`fuzzy-string-match`, `fuzzmatch`)| Unmaintained or unstable; `strsim` covers all needed algorithms with a stable API |
| **Pre-trained sports entity resolution models** | No production-ready Rust-native option; Python-based solutions would require a sidecar service, adding latency and operational complexity |
| **The-odds-api / SportsData.io for canonical IDs** | These are paid providers; they also don't return Kalshi/Polymarket-specific IDs, so the mapping problem still exists; the constraint forbids them anyway |
| **Redis for candidate caching between cycles** | Not needed; the matching cycle runs in-process with the full candidates in memory; adding Redis would increase latency without benefit at the current event volume |
| **Probabilistic blocking (LSH/MinHash)** | Appropriate at scale (millions of records); at 50-200 candidates per platform, exhaustive scoring with Hungarian assignment is already O(n²) ≈ 10,000 operations, which is trivially fast |
## Summary: Crate Additions Required
| Crate | Version | Purpose | Confidence |
|-------|---------|---------|------------|
| `strsim` | `0.11` | Jaro-Winkler + normalized Levenshtein for fuzzy team name scoring | HIGH |
# In backend-rust/Cargo.toml [dependencies]
## Sources
- Direct codebase analysis: `backend-rust/src/matching/` (all files)
- Project constraints: `.planning/PROJECT.md`
- Known gaps: `tasks/lessons.md`, `.planning/PROJECT.md#active-requirements`
- `strsim` crate: https://crates.io/crates/strsim (canonical Rust string metrics, used by Cargo)
- Confidence for `strsim` recommendation: HIGH (training data, Rust ecosystem standard since 2015,
- Confidence for architectural patterns (date bucketing, ID canonicalization): HIGH (derived from
- Confidence for weight recalibration numbers: MEDIUM (directionally correct, require empirical
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## Pattern Overview
- Backend is a single Axum process: all background workers, WebSocket server, and REST API share one `Arc<AppState>` instance
- Primary data store is an in-memory `DashMap`-backed cache (`StateCache`); Supabase Postgres is a secondary persistence layer for history and snapshots
- Frontend polls the REST API on a 15-second interval and optionally connects to the WebSocket feed for real-time arb push
- Event discovery and price ingestion are decoupled: discovery runs on a longer interval (configurable), price refresh runs more frequently
## Layers
- Purpose: Fetches raw market data from external platforms
- Location: `backend-rust/src/ingestion/`
- Contains: REST pollers (DomeAPI, direct Kalshi/Polymarket batch APIs), WebSocket ingesters (Kalshi, Polymarket), culture event poller
- Depends on: `storage::cache` (writes odds), `models::event`
- Used by: `main.rs` spawns each ingester as an independent Tokio task
- Purpose: Correlates events across Kalshi and Polymarket into canonical paired events
- Location: `backend-rust/src/matching/`
- Contains: `matcher.rs` (scoring algorithm), `fetcher_kalshi.rs`, `fetcher_polymarket.rs`, `candidate.rs`, `team_dictionary.rs`, `label_resolver.rs`, `hungarian.rs` (assignment algorithm), `shadow.rs`
- Depends on: External Kalshi API, Polymarket Gamma API; writes to `storage::cache`
- Used by: `main.rs` spawns `run_sports_matching_loop`
- Purpose: Detects arbitrage opportunities from current odds in the cache
- Location: `backend-rust/src/processing/`
- Contains: `arb_detector.rs` — runs every 2 seconds
- Depends on: `storage::cache` (reads odds, writes active arbs), `models::arb`, `storage::supabase` (persists detected arbs)
- Used by: `main.rs` spawns `run_arb_detection_loop`
- Purpose: In-memory cache + Supabase persistence
- Location: `backend-rust/src/storage/`
- Contains: `cache.rs` (`StateCache` struct with DashMap for events, odds, active arbs), `supabase.rs` (Postgres client, snapshot writer)
- Depends on: `models::event`, `models::arb`
- Used by: all other backend layers
- Purpose: Exposes REST endpoints and WebSocket feed to the frontend
- Location: `backend-rust/src/api/`
- Contains: `routes.rs` (Axum router, all REST handlers), `ws_server.rs` (WebSocket broadcast endpoint at `/ws/arb-feed`), `mod.rs`
- Depends on: `storage::cache` (read-only), `storage::supabase` (history queries)
- Used by: frontend HTTP client (`frontend/lib/api.ts`)
- Purpose: Shared data types used across all backend layers
- Location: `backend-rust/src/models/`
- Contains: `event.rs` (`CanonicalEvent`, `Sport` enum, `EventOdds`), `arb.rs` (`ArbitrageOpportunity`), `platform.rs` (API response structs), `ws.rs` (`WsMessage`)
- Purpose: Dashboard UI for viewing matched events, arb opportunities, auth
- Location: `frontend/`
- Contains: Next.js App Router pages, React components, API client
- Depends on: Backend REST API (polled), Supabase Auth
## Data Flow
## Entry Points
- Location: `backend-rust/src/main.rs`
- Triggers: `cargo run` in `backend-rust/`
- Responsibilities: Loads config from env, initializes `StateCache` and `SupabaseClient`, constructs `Arc<AppState>`, spawns all background Tokio tasks, starts Axum HTTP server on `host:port`
- Location: `frontend/app/layout.tsx` (root), `frontend/app/page.tsx` (landing)
- Triggers: `npm run dev` or `npm run build && npm start` in `frontend/`
- Responsibilities: Wraps all routes in `ThemeProvider`, renders landing page, routes to `/markets`, `/arbitrage`, `/bet-tracker`, `/auth/*`, `/pricing`
- `matching::run_sports_matching_loop` — sports event discovery
- `ingestion::culture_poller::run_culture_discovery_loop` — culture event discovery
- `ingestion::direct_api::run_direct_price_refresh_loop` — price refresh
- `ingestion::kalshi_ws::run_kalshi_ws_ingester` — optional Kalshi WebSocket
- `ingestion::polymarket_ws::run_polymarket_ws_ingester` — optional Polymarket WebSocket
- `processing::arb_detector::run_arb_detection_loop` — arb detection every 2s
- `storage::supabase::run_snapshot_writer` — periodic odds persistence
- `run_cache_eviction_loop` (defined in `main.rs`) — cache cleanup every 30 minutes
## Database / Storage Design
- `events: DashMap<String, CanonicalEvent>` — all discovered events keyed by ID (e.g. `"nba-bos-mia-2026-03-29"`)
- `odds: DashMap<String, EventOdds>` — current odds keyed by event ID; `EventOdds.platform_odds` is `HashMap<platform, HashMap<outcome, OutcomePrice>>`
- `active_arbs: DashMap<String, Vec<ArbitrageOpportunity>>` — current detected arbs keyed by event ID
- `matching_stats: RwLock<Option<MatchingStats>>` — last matching cycle statistics
- `canonical_events` — persistent record of all discovered events; RLS-enabled with public reads
- `odds_snapshots` — periodic odds snapshots for historical charting; referenced by `GET /api/v2/events/{id}/odds-history`
- `arbitrage_opportunities` — historical arb records with `closed_at` and `duration_ms` for closed windows
- Frontend uses Supabase Auth for email/password login via `frontend/lib/supabase/`
- Row Level Security is enforced on all tables; service role key used by backend for writes
## Error Handling
- DB writes from ingestion/matching use `if let Err(e) = ... { warn!(...) }` — failures are logged but do not halt the pipeline
- Background loops log errors via `tracing::error!` and continue looping on next interval
- API route handlers return 404/empty arrays rather than 500s when cache misses occur
- WebSocket send errors silently drop the client connection
## Cross-Cutting Concerns
<!-- GSD:architecture-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd:quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd:debug` for investigation and bug fixing
- `/gsd:execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd:profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
