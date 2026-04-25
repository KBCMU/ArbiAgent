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
\- Deterministic code: primarily \`backend-rust/\` (Axum API, ingestion, processing, storage) and \`frontend/\` (Next.js)  
\- Environment variables and API tokens live in \`.env\`  
\- Reliable, testable; prefer extending existing modules over ad-hoc one-offs  
\- This repo does not use a top-level \`execution/\` Python folder

\*\*Why this works:\*\* if you do everything yourself, errors compound. 90% accuracy per step \= 59% success over 5 steps. The solution is push complexity into deterministic code. That way you just focus on decision-making.

\#\# Operating Principles

\*\*1. Check for tools first\*\*  
Before adding new code, check existing \`backend-rust/src/\` and \`frontend/\` patterns. Only add new modules or scripts when nothing fits.

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

## File Organization

\*\*Deliverables vs Intermediates:\*\*  
\- \*\*Deliverables\*\*: Google Sheets, Google Slides, or other cloud-based outputs that the user can access  
\- \*\*Intermediates\*\*: Temporary files needed during processing

\*\*Directory structure:\*\*  
\- \`.tmp/\` \- Optional intermediate files; never commit if used.  
\- \`backend-rust/\` \- Rust backend (API, ingestion, arb detection, Supabase)  
\- \`frontend/\` \- Next.js UI  
\- \`directives/\` \- SOPs in Markdown (agent instruction set)  
\- \`.env\` \- Environment variables and API keys

\*\*Key principle:\*\* Local files are only for processing. Deliverables live in cloud services (Google Sheets, Slides, etc.) where the user can access them. Everything in \`.tmp/\` can be deleted and regenerated.

\#\# Summary

You sit between human intent (directives) and deterministic execution (Rust/TypeScript in this repo). Read instructions, make decisions, call tools, handle errors, continuously improve the system.

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

A prediction market arbitrage scanner: Rust backend (Axum + Tokio) + Next.js frontend. The **primary operating mode** (when `ENABLE_PREDICTION_API` is set) is **not** in-process cross-platform matching. Instead, **paired events and odds are pulled from an external HTTP Prediction API** (see `backend-rust/src/ingestion/prediction_api.rs`). The backend still runs **arbitrage detection** against the in-memory cache, snapshotting to Supabase, and serves the **REST + WebSocket** API to the frontend.

**Core value:** Surface Kalshi and Polymarket odds and arbitrage opportunities in one UI, with event alignment owned by the external service rather than the `matching/` engine.

### Constraints

- **Tech stack:** Rust (Axum/Tokio), Next.js — preserve clear layering in `backend-rust/src/`.
- **Data source in Prediction API mode:** one REST service (`PREDICTION_API_URL`, optional `PREDICTION_API_TOKEN`); it replaces native matching, direct batch price loops, culture poller, and the optional platform WebSocket ingesters (those are only in the `else` branch of `main.rs`).
- **Legacy code paths** (when `ENABLE_PREDICTION_API` is off): in-repo `matching/` (Hungarian + scoring), optional Dome poller, direct Kalshi/Polymarket APIs, culture poller, optional Kalshi/Polymarket WebSocket ingesters. Treat these as **fallback / self-hosted** operation, not the default story for this deployment.
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

- **Backend:** Rust, Axum, Tokio, `reqwest` for the Prediction API client, in-memory `DashMap` cache (`StateCache`), Supabase/Postgres for history and snapshots.
- **Ingestion (Prediction API mode):** `ingestion::prediction_api` — event loop + price loop calling the external REST API; writes into `storage::cache`.
- **Processing:** `processing::arb_detector` — always-on arb detection (same in both modes).
- **API:** `api::routes` + optional `ws_server` for `/ws/arb-feed`.
- **Frontend:** Next.js (App Router), API client, Supabase Auth.
- **Legacy (optional):** `matching/` (Jaccard, Hungarian, team dictionary, etc.), `ingestion::dome_poller`, `ingestion::direct_api`, `ingestion::culture_poller`, `kalshi_ws` / `polymarket_ws` — only when Prediction API is disabled and the corresponding `config` flags are set.
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

- Prefer **one clear data path**: when extending ingestion, start from `ingestion/prediction_api.rs` for the deployed “external API” mode; only touch `matching/` or Dome-related modules when working on the legacy self-hosted path.
- Keep **env-driven behavior** documented in `backend-rust/src/config.rs` (e.g. `ENABLE_PREDICTION_API`, `PREDICTION_API_URL`).

## Architecture

### Pattern overview

- Single **Axum** process: background tasks + HTTP + (optional) WebSocket share `Arc<AppState>`.
- **State:** `StateCache` (in-memory) + **Supabase** for snapshots and history.
- **Frontend** polls the backend REST API and can subscribe to the WebSocket for arb push.

### Entry point: `backend-rust/src/main.rs`

- **`enable_prediction_api == true` (this project’s default story):** spawns `ingestion::prediction_api::run_prediction_event_loop` and `run_prediction_price_loop` only. Does **not** start native `matching::run_sports_matching_loop`, culture poller, direct batch price API, or Kalshi/Polymarket WebSocket ingesters.
- **`enable_prediction_api == false` (legacy / self-hosted):** native matching or Dome for event discovery, direct batch or Dome for prices, culture poller, optional WebSocket ingesters (all gated by their config flags; see `main.rs` for exact branches).

### Always-on (both modes)

- `processing::arb_detector::run_arb_detection_loop`
- `storage::supabase::run_snapshot_writer`
- Cache eviction and Axum server startup (see `main.rs`).

### Layers (reference)

- **`ingestion/`** — in Prediction API mode, primarily `prediction_api.rs`. Legacy: `direct_api`, `dome_poller`, `culture_poller`, `kalshi_ws`, `polymarket_ws`.
- **`matching/`** — only when native matching is enabled and Prediction API is off.
- **`processing/`** — `arb_detector`.
- **`storage/`** — `cache`, `supabase`.
- **`api/`** — REST and WebSocket.
- **`models/`** — `CanonicalEvent`, `ArbitrageOpportunity`, platform DTOs, WS messages.
- **`frontend/`** — Next.js UI.

### Data flow (Prediction API mode)

1. External REST API → `prediction_api` → updates `StateCache` (events + odds).
2. Arb detector reads cache → writes active arbs + optional Supabase.
3. Frontend reads backend REST (and optional WS).

## Database / storage (high level)

- In-memory: events, odds, active arbs (see `StateCache` in `storage/cache.rs`).
- Supabase: canonical events, odds snapshots, arbitrage history — same as before; RLS and service role usage unchanged.

## Error handling

- Background tasks log errors and continue; API handlers avoid noisy 500s on cache miss where appropriate. Match ingestion and legacy matching notes from older docs still apply to **legacy** paths only.
<!-- GSD:conventions-end -->
