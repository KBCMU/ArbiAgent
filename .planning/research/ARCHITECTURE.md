---
focus: arch
generated: 2026-03-29
---

# Architecture: Cross-Platform Sports Event Matching Pipeline

**Project:** ArbiAgent — Kalshi / Polymarket sports event matching
**Analysis Date:** 2026-03-29
**Confidence:** HIGH (based on direct codebase inspection of `backend-rust/src/matching/`)

---

## Current Architecture Overview

The matching pipeline lives entirely inside a single Tokio task spawned by `main.rs`
as `run_sports_matching_loop`. It is a synchronous-looking linear pipeline executed
on a configurable interval (default configurable via `event_discovery_interval_secs`).
All stages run to completion before the cache is written.

```
main.rs
  └── tokio::spawn(run_sports_matching_loop)
        │
        ├── Stage 1: FETCH  (concurrent tokio::join!)
        │     ├── fetcher_kalshi::fetch_kalshi_sports_candidates()
        │     └── fetcher_polymarket::fetch_polymarket_sports_candidates()
        │
        ├── Stage 2: NORMALIZE  (inside fetchers)
        │     ├── normalize_title(), extract_date_from_kalshi_ticker()
        │     ├── team_dictionary lookups → team_a / team_b abbreviations
        │     └── MarketType classification (Moneyline / Spread / Total)
        │
        ├── Stage 3: CANDIDATE GENERATION  (in matcher.rs)
        │     └── bucket_by_sport_date_market() → BucketKey=(Sport, Date?, MarketTypeBucket)
        │
        ├── Stage 4: SCORING
        │     └── score_pair() per cross-platform pair
        │           ├── compute_team_score()  (0 / 25 / 50 pts)
        │           ├── jaccard_token_similarity() × 20 pts
        │           ├── date match bonus  (20 / 15 / 0 pts)
        │           └── market type match bonus  (10 pts)
        │
        ├── Stage 5: ASSIGNMENT
        │     ├── Primary: Hungarian max_weight_assignment() per bucket
        │     └── Fallback: cross-bucket retry on (Sport, MarketTypeBucket) key
        │           (same score_pair + Hungarian, relaxes date requirement)
        │
        ├── Stage 6: EVENT CONSTRUCTION
        │     ├── build_matched_event()  → dual-platform CanonicalEvent
        │     └── build_single_platform_event()  → single-platform CanonicalEvent
        │           (unmatched Kalshi and unmatched Poly both become events)
        │
        ├── Stage 7: LABEL RESOLUTION  (async, post-assignment)
        │     └── label_resolver::resolve_polymarket_labels()
        │           ├── Pass 1: keyword match via team_dictionary
        │           ├── Pass 2: elimination (if one unresolved, assign remainder)
        │           └── Pass 3: bipartite scoring fallback
        │
        └── Stage 8: CACHE UPSERT
              ├── StateCache.events.insert() (DashMap, keyed by event ID)
              └── supabase::upsert_canonical_events()
```

---

## Component Boundaries

| Component | File | Responsibility | Input | Output |
|-----------|------|---------------|-------|--------|
| Kalshi Fetcher | `fetcher_kalshi.rs` | Pull raw events from Kalshi REST API, normalize into CandidateEvents | `&[Sport]` | `Vec<CandidateEvent>` |
| Polymarket Fetcher | `fetcher_polymarket.rs` | Pull raw events from Polymarket Gamma API, normalize into CandidateEvents | `&[Sport]` | `Vec<CandidateEvent>` |
| Team Dictionary | `team_dictionary.rs` | Map team name variants → canonical abbreviations; sport-to-series prefix map | raw name string | `Option<String>` abbreviation |
| Candidate Model | `candidate.rs` | Platform-agnostic event representation; title normalization; date extraction utilities | — | `CandidateEvent` |
| Matcher | `matcher.rs` | Bucketing, scoring, assignment, event construction, diagnostics | `Vec<CandidateEvent>` × 2 | `MatchResult` |
| Hungarian Solver | `hungarian.rs` | Optimal bipartite max-weight assignment | weight matrix `Vec<Vec<f64>>` | `Vec<(row, col, weight)>` |
| Label Resolver | `label_resolver.rs` | Resolve Polymarket token IDs → Kalshi-compatible outcome abbreviations via Gamma API | slug, token_ids, kalshi_tickers | `Vec<String>` outcome labels |
| Shadow Comparator | `shadow.rs` | Run DomeAPI in parallel, log overlap/divergence metrics (validation only, no prod writes) | `Arc<AppState>` | log output only |
| Matching Loop | `mod.rs` or `main.rs` | Orchestrate all stages on interval; write results to StateCache + Supabase | `Arc<AppState>` | writes to `StateCache.events` |

**What does NOT touch matching:**
- `processing/arb_detector.rs` — reads events from cache only; never mutates event records
- `ingestion/direct_api.rs` — writes odds to cache but never touches event identity
- `api/routes.rs` — reads cache only; never invokes matching

---

## Data Flow (Information Movement)

```
Kalshi REST API  ──► fetcher_kalshi  ──►┐
                                         ├──► CandidateEvent pool
Polymarket Gamma ──► fetcher_poly    ──►┘       │
                                                 ▼
                              bucket_by_sport_date_market()
                                                 │
                              BucketKey = (Sport, Date?, MarketType)
                                                 │
                                         ┌───────┴──────────┐
                                         │  per-bucket loop  │
                                         │   score_pair()   │
                                         │   Hungarian()    │
                                         └───────┬──────────┘
                                                 │
                                    matched pairs + unmatched lists
                                                 │
                                   build_matched_event()          build_single_platform_event()
                                                 │                              │
                                                 └───────────┬──────────────────┘
                                                             ▼
                                                   Vec<CanonicalEvent>
                                                             │
                                            (async) label_resolver per matched event
                                                             │
                                                   CanonicalEvent with resolved outcome labels
                                                             │
                                                    ┌────────┴────────┐
                                                    ▼                 ▼
                                              StateCache          Supabase
                                          DashMap<id, event>   canonical_events
```

**Key data transformations at each boundary:**

- **Raw API → CandidateEvent:** title string → `normalized_title` (lowercase, stripped punctuation); API ticker/dates → `Option<NaiveDate>`; API strings → `Option<String>` team abbreviations via dictionary lookup
- **CandidateEvent → BucketKey:** 3-tuple `(Sport, Option<NaiveDate>, MarketTypeBucket)` — this is the primary partitioning decision; mismatched dates here cause missed cross-platform matches
- **BucketKey → score matrix:** O(K × P) pairs where K = Kalshi candidates in bucket, P = Polymarket candidates in bucket — in practice K, P < 30 so Hungarian is fast
- **Assignment → CanonicalEvent:** matched pairs get both platforms' IDs in `PlatformIds`; unmatched get only their own platform's IDs (the other platform fields are empty `Vec`/`None`)
- **CanonicalEvent → cache key:** `event_id = "{sport}-{team_a}-{team_b}-{date}"` — deterministic, sorted team names; date source priority is Polymarket > Kalshi

---

## Identified Structural Gaps

These are gaps in the current pipeline that cause the known quality problems:

### Gap 1: No Post-Assignment Deduplication Stage

**What is missing:** After event construction, there is no dedup pass before cache upsert. If two matching cycles produce overlapping events (e.g. one cycle matched "NBA-bos-mia-2026-03-29", a subsequent cycle produces the same pair with a slightly different ID due to date source inconsistency), both entries live in the cache.

**Location of gap:** Between Stage 6 (event construction) and Stage 8 (cache upsert).

**Recommended insertion point:** A `dedup_events(results: Vec<CanonicalEvent>) -> Vec<CanonicalEvent>` stage that:
1. Groups events by normalized `(sport, sorted_team_pair)` within a date window (±1 day)
2. For duplicate groups, keeps the dual-platform event (one with both `kalshi_market_tickers` and `polymarket_token_ids` populated) or highest `match_score`
3. Logs any collapsed duplicates for diagnostics

### Gap 2: Date Extraction Inconsistency Across Sources

**What is missing:** Date is extracted from multiple sources in different components with no single authoritative priority rule enforced end-to-end:
- Kalshi: ticker segment parsing in `candidate.rs::extract_date_from_kalshi_ticker()`
- Kalshi: `close_time` API field (used as proxy for game start)
- Polymarket: `startDate` and `endDate` API fields
- Polymarket: title string parsing via `extract_date_from_title()`
- Event ID construction: prefers Polymarket date over Kalshi date (`build_event_id`)

The bucketing step uses `CandidateEvent.game_date` which may be `None` on one side, placing events in the `(sport, None, market_type)` bucket and bypassing the primary match path entirely (falling through to cross-bucket retry with relaxed date scoring).

**Recommended fix:** Establish a single date resolution hierarchy per platform, resolved in the fetcher before `CandidateEvent` is returned, with explicit `date_confidence: DateConfidence { ApiField, TickerParsed, TitleExtracted, Unknown }` metadata on each candidate.

### Gap 3: Cache Upsert Does Not Merge — It Overwrites

**What is missing:** When `StateCache.events.insert(id, event)` is called, it silently replaces any existing event with that ID. If a new cycle produces a single-platform event (unmatched) for an ID that was previously dual-platform (matched), the dual-platform event and its odds associations are lost. The arb detector then sees no dual-platform events and reports no opportunities.

**Recommended fix:** Cache upsert logic should merge rather than replace: if the incoming event is single-platform but the cached event is dual-platform with both `kalshi_market_tickers` and `polymarket_token_ids`, preserve the existing dual-platform record unless the new record has a higher `match_score`.

### Gap 4: Team Normalization Happens Inside Fetchers, Not as a Shared Pre-Stage

**What is missing:** `team_dictionary` lookups are embedded inside `fetcher_kalshi.rs` and `fetcher_polymarket.rs` separately. This means the same normalization logic is duplicated and can drift. A failure to resolve a team abbreviation in one fetcher silently produces `None` for `team_a`/`team_b`, which zeroes out the team scoring signal and tanks the match score for that event.

**Recommended fix:** Make normalization a distinct, tested pipeline stage with visibility (e.g. `NormalizationResult { candidate: CandidateEvent, unresolved_tokens: Vec<String> }`) so monitoring can track resolution rates.

---

## Recommended Pipeline Architecture

The following adds the missing stages while preserving all existing components:

```
Stage 1: FETCH (parallel)
  fetcher_kalshi::fetch_kalshi_sports_candidates(&sports)
  fetcher_polymarket::fetch_polymarket_sports_candidates(&sports)
  → raw Vec<CandidateEvent> (may have None team fields, None dates)

Stage 2: NORMALIZE (new explicit stage)
  For each candidate:
    - Resolve team names through team_dictionary with fallback chain
    - Assign DateConfidence metadata
    - Tag normalization gaps as NormalizationWarning for diagnostics
  → Vec<NormalizedCandidate>

Stage 3: CANDIDATE GENERATION / BUCKETING
  bucket_by_sport_date_market() → primary buckets on (Sport, Date, MarketType)
  → HashMap<BucketKey, Vec<usize>>

Stage 4: SCORING
  score_pair() per cross-platform pair within bucket
  → weight matrices per bucket

Stage 5: ASSIGNMENT
  Hungarian per bucket (primary)
  Cross-bucket fallback on (Sport, MarketType) for date-unresolved candidates
  → Vec<(kalshi_idx, poly_idx, score)>  +  unmatched sets

Stage 6: EVENT CONSTRUCTION
  build_matched_event() / build_single_platform_event()
  → Vec<CanonicalEvent>

Stage 7: LABEL RESOLUTION (async, parallel across matched events)
  label_resolver::resolve_polymarket_labels() per matched pair
  → Vec<CanonicalEvent> with resolved outcome labels

Stage 8: DEDUPLICATION  ← new stage
  dedup_events():
    - Group by (sport, sorted_team_pair, date_window ±1 day)
    - Keep dual-platform > single-platform
    - Keep higher match_score within same category
    - Log all collapsed duplicates
  → Vec<CanonicalEvent>  (no duplicates)

Stage 9: CACHE UPSERT  ← upgrade from replace to merge
  For each event:
    - If new event is dual-platform: always upsert
    - If new event is single-platform and cache has dual-platform: preserve cache version
    - Otherwise: upsert
  StateCache.events + Supabase canonical_events
```

---

## Component Communication Map

```
fetcher_kalshi ──────────────────────────────────┐
                                                  ├──► matcher::match_candidates()
fetcher_polymarket ───────────────────────────────┘
                                                  │
                              matcher ────────────┼──► label_resolver (async HTTP)
                                                  │
                              label_resolver ─────┼──► Polymarket Gamma API (external)
                                                  │
                              matcher result ──────┼──► StateCache (DashMap write)
                                                  │
                              StateCache ──────────┼──► arb_detector (read-only)
                              StateCache ──────────┼──► API routes (read-only)
                              StateCache ──────────┼──► snapshot_writer (read-only)
                                                  │
                              matcher result ──────┼──► supabase::upsert_canonical_events
                                                  │
                              shadow.rs ───────────┴──► DomeAPI (external, validation only)
```

**Rule:** Only `run_sports_matching_loop` writes to `StateCache.events`. All other components are consumers. This invariant should be preserved in any refactor.

---

## Integration with Existing Rust Architecture

### What Must Be Preserved

| Concern | Current Implementation | Constraint |
|---------|----------------------|------------|
| Tokio task model | `tokio::spawn(run_sports_matching_loop)` | Keep as single spawned task; no reason to split into multiple |
| `Arc<AppState>` sharing | State passed by clone into task | No change needed |
| `DashMap` cache | Lock-free concurrent reads during write cycles | New dedup/merge logic must not hold a DashMap write lock for the entire event list |
| Axum route handlers | Read-only from `StateCache` | No change |
| Supabase upsert | Called after each matching cycle | Dedup must happen before Supabase write (write deduped events only) |
| `MatchingStats` | Written to `RwLock<Option<MatchingStats>>` after each cycle | Add dedup metrics to stats struct |

### Where to Add New Code

| New stage | Location | Pattern |
|-----------|----------|---------|
| Normalization stage | New `matching/normalizer.rs` | Pure function, no async |
| Dedup stage | New `matching/dedup.rs` | Pure function over `Vec<CanonicalEvent>` |
| Cache merge logic | `storage/cache.rs::upsert_event()` | Replace `events.insert()` with merge function |
| DateConfidence type | `matching/candidate.rs` | Add enum and field to `CandidateEvent` |
| Normalization metrics | `models/event.rs::MatchingStats` | Add `unresolved_teams: usize`, `deduped: usize` fields |

### Suggested Build Order

The stages have clean dependencies. Build in this order to avoid circular rework:

1. **`DateConfidence` enum + fetcher date resolution** — foundational; fixes the bucketing miss problem upstream of everything else. Low risk: additive change to `CandidateEvent`.

2. **`matching/dedup.rs`** — pure function with no external dependencies; can be written and unit-tested before touching the cache. Add it as Stage 8 in the matching loop.

3. **Cache merge logic (`cache.rs::upsert_event()`)** — targeted change to one function; replaces the `insert()` call in the upsert stage. Requires dedup to be in place first so merged state is already clean.

4. **Normalization stage extraction** — refactor fetcher team-resolution into a shared `normalizer.rs`; no behavior change, improves testability and adds warning logging for unresolved tokens.

5. **Metrics expansion (`MatchingStats`)** — add dedup counts, unresolved team counts, date-confidence distribution; surfaces pipeline health via `GET /api/v2/matching-stats` (endpoint already exists).

---

## Scalability Notes

The current architecture is appropriate for the scale of this problem (< 200 events per platform per cycle). No architectural changes are needed for scalability. The Hungarian algorithm runs in O(n³) but bucket sizes are < 30 in practice (tested).

The main scalability concern is the label_resolver: it makes one async Gamma API HTTP request per matched pair. With 50 matched pairs per cycle this is 50 sequential HTTP requests if not parallelized. Use `tokio::join_all` or `futures::stream::iter(...).buffer_unordered(10)` for concurrent label resolution.

---

## Sources

- Direct inspection of `backend-rust/src/matching/matcher.rs` (full file)
- Direct inspection of `backend-rust/src/matching/candidate.rs`
- Direct inspection of `backend-rust/src/matching/hungarian.rs`
- Direct inspection of `backend-rust/src/matching/fetcher_kalshi.rs` (header)
- Direct inspection of `backend-rust/src/matching/fetcher_polymarket.rs` (header)
- Direct inspection of `backend-rust/src/matching/label_resolver.rs` (header)
- Direct inspection of `backend-rust/src/matching/shadow.rs` (full file)
- `.planning/codebase/ARCHITECTURE.md` (system-level architecture)
- `.planning/PROJECT.md` (requirements and known gaps)
- `tasks/lessons.md` (historical failure modes)

*All findings are HIGH confidence — derived from current codebase, not training data assumptions.*
