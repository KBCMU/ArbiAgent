---
focus: tech
generated: 2026-03-29
---

# Technology Stack: Cross-Platform Sports Event Matching

**Project:** ArbiAgent — prediction market arbitrage scanner
**Milestone:** Improved sports event matching (Polymarket ↔ Kalshi)
**Researched:** 2026-03-29
**Overall confidence:** HIGH (based on direct codebase analysis and Rust ecosystem knowledge as of Aug 2025)

---

## What This Research Is About

The existing matcher in `backend-rust/src/matching/` already uses a multi-signal scoring approach
with Hungarian algorithm assignment. This research documents the **specific techniques, crates,
and approaches** that address the documented gaps: false negatives (events on both platforms
showing only one platform's odds), duplicate events, and incorrect dates. It does not re-evaluate
the Rust/Axum/Next.js stack.

---

## Current Approach Inventory

Before recommending changes, what the system already has:

| Component | Technique | File |
|-----------|-----------|------|
| Team normalization | Static keyword dictionary → canonical 3-letter abbreviation | `team_dictionary.rs` (58k) |
| Title similarity | Jaccard token overlap on normalized titles | `candidate.rs::jaccard_token_similarity` |
| Date extraction | Ticker segment parsing + regex on titles | `candidate.rs` |
| Assignment | Kuhn-Munkres (Hungarian) O(n³), hand-rolled | `hungarian.rs` |
| Bucketing | (sport, date, market_type) bucket → per-bucket matching | `matcher.rs` |
| Fallback | Cross-bucket retry when date differs | `matcher.rs::match_within_bucket` |
| Score weighting | `WEIGHT_BOTH_TEAMS=50, TITLE_SIM=20, DATE=20, MARKET_TYPE=10` | `matcher.rs` |

**Known gaps (from PROJECT.md and lessons.md):**
1. Duplicate events appear in the UI — likely same event emitted from multiple buckets
2. Events on both platforms showing only one platform's odds — false negatives at matching
3. Incorrect dates — Kalshi ticker parsing fails for non-standard ticker formats
4. College sports (CBB/CFB) fail because team dictionary lacks college entries
5. Polymarket grouped market structure (Yes/No per team) requires different parsing path

---

## Recommended Stack Changes

### 1. String Similarity: Replace Jaccard with Edit-Distance Metrics

**Current problem:** `jaccard_token_similarity` treats "Lakers" and "LAL" as entirely different
tokens, scoring 0 even when team_a/team_b correctly canonicalize both to "LAL". Jaccard also
scores "Boston Celtics" and "Celtics" poorly because the shared set is {celtics}/2 = 0.5.

**Recommendation: Add `strsim` crate, use Jaro-Winkler for team name fuzzy matching.**

```toml
# Cargo.toml
strsim = "0.11"
```

**Why `strsim` specifically:**
- Pure Rust, no unsafe, zero dependencies — compiles cleanly in the existing workspace
- Version 0.11 (2024) includes Jaro-Winkler, Damerau-Levenshtein, normalized Levenshtein, Hamming
- Confidence: HIGH — this is the canonical string metrics crate in the Rust ecosystem,
  maintained actively, used by cargo itself for "did you mean?" suggestions
- `jaro_winkler` is the right choice here because it rewards common prefixes, which is exactly
  the pattern in sports names ("Boston" vs "BOS", "Philadelphia" vs "PHI")

**What to replace/augment:**
- Keep Jaccard for title-level comparison (still useful for multi-word overlap)
- Add Jaro-Winkler as a fallback in `compute_team_score` when exact canonical match fails:
  if `jaro_winkler(team_a_raw, poly_team_raw) > 0.85` → award partial credit
- This handles cases where team extraction succeeds on one side but not the other

**What NOT to use:**
- `fuzzy-matcher` (FZF algorithm) — designed for interactive UI filtering, not deterministic
  record linkage; scores are not normalized and vary with query length
- `rapidfuzz` Python port crates — immature, no stable Rust crate as of 2025
- Custom Levenshtein — `strsim` already implements it correctly and is battle-tested

**Confidence:** HIGH

---

### 2. Date Resolution: Use Platform API Timestamps, Not Parsed Strings

**Current problem (from lessons.md):** Polymarket events often have wrong or missing `game_date`
because the system tries to extract dates from title strings. Kalshi ticker parsing handles standard
formats but breaks on edge cases. Date mismatches cause bucket splits — the same event goes into
two different `(sport, date, market_type)` buckets, which prevents matching.

**Recommendation: Use API-provided timestamps as the canonical date source. Parse titles only as
last resort.**

Priority order for `game_date` resolution:

1. **Kalshi `close_time` field** (already in `KalshiEvent.close_time`) — convert to NaiveDate in
   America/New_York timezone. This is the most reliable signal; Kalshi always sets it.
2. **Polymarket `startDate` field** (already in `GammaEvent.start_date`) — convert to NaiveDate
   in Eastern time. This was identified in lessons.md as the correct field to use.
3. **Kalshi ticker segment** (`25AUG16` pattern) — current approach, keep as fallback
4. **Title string regex** — keep as last resort

**Additional fix: Widen the date tolerance in the bucket key.**

The current bucket key is `(Sport, Option<NaiveDate>, MarketTypeBucket)`. A one-day timezone
difference between platforms causes the same event to fall into different buckets and never be
matched. The fix is to use a **date window bucket** rather than exact date:

```rust
// Instead of exact date in bucket key, use ISO week + day-of-week,
// OR bucket on (sport, market_type) only and let the scorer handle date proximity
type BucketKey = (Sport, MarketTypeBucket);  // remove date from primary bucket
// scorer already applies WEIGHT_DATE_MATCH for exact match and 0.75x for ±1 day
```

The scorer's `WEIGHT_DATE_MATCH` logic already handles ±1 day gracefully. The bucket split on
exact date defeats this. Remove date from the bucket key entirely for the primary pass; the
Hungarian algorithm and scoring threshold will reject false matches.

**Why this is safe:** The score threshold is 60.0. Two events for different days of the same sport
would need to match on both teams (50 pts) AND title (up to 20 pts) to reach 70+. The date
mismatch would cost 20 pts. Two different games with the same teams on different days would score
50 (teams) + near-zero title (titles include dates in team abbreviations sometimes) = ~50, which
is below threshold.

**Confidence:** HIGH for using API timestamps. MEDIUM for removing date from bucket key (depends
on how many false-positive pairs exist for back-to-back games, e.g., doubleheaders in MLB or
NBA playoff series).

---

### 3. Deduplication: Canonical Event ID as Primary Dedup Key

**Current problem:** Duplicate events appear in the UI, suggesting that the same matched pair is
being emitted more than once — either from overlapping buckets (primary + cross-bucket fallback
both match the same pair) or from multiple discovery cycles building on stale cache entries.

**Root cause (analysis of `matcher.rs`):**
- The cross-bucket fallback at line 111–169 does not check `matched_kalshi`/`matched_poly` early
  enough — it calls `match_within_bucket` and only checks after assignment, but `match_within_bucket`
  itself can return an already-matched index if the guard condition is wrong
- More critically: `build_event_id` is called to produce the canonical ID used as the DashMap key.
  If `build_event_id` generates different IDs for the same underlying event on two discovery cycles
  (e.g., because `team_a` order flips), two separate cache entries appear for the same game.

**Recommendation: Fix `build_event_id` to be deterministic and order-independent.**

The event ID format should be: `{sport}-{team_min}-{team_max}-{date}` where `team_min`/`team_max`
are the lexicographically sorted canonical abbreviations. This guarantees the same ID regardless
of which team is extracted first from either platform's API.

```rust
fn build_event_id(kalshi: &CandidateEvent, poly: &CandidateEvent) -> String {
    let ta = kalshi.team_a.as_deref().unwrap_or("unk");
    let tb = kalshi.team_b.as_deref().unwrap_or("unk");
    let (lo, hi) = if ta <= tb { (ta, tb) } else { (tb, ta) };  // sort teams
    let date = kalshi.game_date
        .or(poly.game_date)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "nodate".to_string());
    format!("{}-{}-{}-{}", kalshi.sport.as_str(), lo, hi, date)
}
```

**Confidence:** HIGH — the ID instability is a structural issue that the above pattern directly
resolves.

---

### 4. Fuzzy Team Matching: Add `edit-distance` Crate for Candidate Extraction

**Current problem:** College teams (CBB/CFB) and newly relocated/renamed franchises fail because
team extraction depends on exact keyword matches in `team_dictionary.rs`. When a keyword is
missing, `team_a`/`team_b` are `None`, which makes `compute_team_score` return 0.0, which means
the pair relies entirely on Jaccard title similarity (max 20 pts) — always below the 60 pt
threshold.

**Recommendation: Two-part fix.**

**Part A — Expand the dictionary.** College sports require explicit entries; there is no
abbreviation-based fallback that works. Add all NCAA D1 basketball programs for CBB and Power 4
football programs for CFB. These are ~370 teams combined, which is a one-time addition to the
existing `TEAM_ENTRIES` array. No new crate needed.

**Part B — Fuzzy fallback for unknown teams.** When team extraction fails on one side, use
normalized edit distance to attempt a fuzzy match between raw extracted tokens and dictionary
keywords:

```toml
# Same strsim crate, already recommended above
```

The `normalized_levenshtein` function from `strsim` returns a [0,1] similarity. If the best
dictionary keyword match is > 0.80 similarity to a title token, treat it as a soft team match
and award 0.5 × WEIGHT_ONE_TEAM (12.5 pts). This prevents falling to zero on near-misses.

**Confidence:** HIGH for dictionary expansion. MEDIUM for fuzzy fallback (risk: false team
extraction may score wrong pairs higher, need tuning of the 0.80 threshold).

---

### 5. Sports Data APIs: No New External Providers Needed

The PROJECT.md constraint is clear: "no additional paid data providers." The two platform APIs
(Kalshi REST + Polymarket Gamma) are the only data sources.

**What the platforms actually provide that is underutilized:**

| Field | Platform | Currently Used? | Recommendation |
|-------|----------|-----------------|----------------|
| `close_time` | Kalshi | Partial | Use as primary date source |
| `startDate` | Polymarket Gamma | Partial | Use as primary date source |
| `sub_title` | Kalshi | No | Parse for opponent team name |
| `series_ticker` | Kalshi | No | Use as sport/league discriminator |
| `tags[].label` | Polymarket | Partial | Use "nba", "nfl" tags for sport confirmation |
| `tags[].slug` | Polymarket | No | Use slugs like "nba-playoffs" for market type |

**`sub_title` is a particularly valuable untapped signal.** Kalshi's `KalshiMarketNested.yes_sub_title`
and `no_sub_title` often contain the team names as human-readable strings (e.g., "Los Angeles
Lakers", "Boston Celtics") rather than the ticker-encoded abbreviations. Parsing these directly
would eliminate the need for reverse-lookup from abbreviations in many cases.

**Confidence:** HIGH — these are fields already in the API response structs defined in
`fetcher_kalshi.rs` but marked `#[allow(dead_code)]`.

---

### 6. Scoring Architecture: Weight Recalibration

**Current weights:**
```
WEIGHT_BOTH_TEAMS    = 50.0   (both canonical teams match)
WEIGHT_ONE_TEAM      = 25.0   (one canonical team matches)
WEIGHT_TITLE_SIM     = 20.0   (Jaccard title overlap)
WEIGHT_DATE_MATCH    = 20.0   (exact date) / 15.0 (±1 day)
WEIGHT_MARKET_TYPE   = 10.0   (same bucket)
MIN_SCORE            = 60.0
MAX_POSSIBLE         = 100.0
```

**Problem:** With both teams matching (50 pts) + exact date (20 pts) = 70 pts, a match is
accepted even with zero title similarity (0 pts) and wrong market type (0 pts). This is correct
behavior. But when only one team extracts successfully (25 pts) + date match (20 pts) = 45 pts,
the pair fails below threshold even though it might be the correct match — especially for
Polymarket events that encode only one team in the market title.

**Recommendation:** Add a `WEIGHT_DATE_STRONG` signal for cases where both the API timestamp
AND the ticker/title agree on the date (≥2 independent date sources confirming the same date):

```rust
const WEIGHT_DATE_MATCH_STRONG: f64 = 28.0;  // two independent sources confirm date
const WEIGHT_DATE_MATCH: f64 = 20.0;          // single source confirms date
const WEIGHT_DATE_NEAR: f64 = 15.0;           // ±1 day
```

This rewards events where the system has high date confidence, letting a one-team match with
strong date (25 + 28 = 53) combined with minimal title overlap (5) exceed threshold (58 → just
under, so also consider raising ONE_TEAM slightly or lowering threshold to 55 for the cross-bucket
fallback pass).

**Confidence:** MEDIUM — weight recalibration requires empirical testing against production data.
The direction is correct but specific values need tuning.

---

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

---

## Summary: Crate Additions Required

| Crate | Version | Purpose | Confidence |
|-------|---------|---------|------------|
| `strsim` | `0.11` | Jaro-Winkler + normalized Levenshtein for fuzzy team name scoring | HIGH |

That is the only new crate needed. Everything else is algorithm/logic improvements within the
existing code structure.

**Installation:**
```toml
# In backend-rust/Cargo.toml [dependencies]
strsim = "0.11"
```

---

## Sources

- Direct codebase analysis: `backend-rust/src/matching/` (all files)
- Project constraints: `.planning/PROJECT.md`
- Known gaps: `tasks/lessons.md`, `.planning/PROJECT.md#active-requirements`
- `strsim` crate: https://crates.io/crates/strsim (canonical Rust string metrics, used by Cargo)
- Confidence for `strsim` recommendation: HIGH (training data, Rust ecosystem standard since 2015,
  actively maintained through 2025)
- Confidence for architectural patterns (date bucketing, ID canonicalization): HIGH (derived from
  direct code analysis of the bugs described in PROJECT.md)
- Confidence for weight recalibration numbers: MEDIUM (directionally correct, require empirical
  tuning against production logs)
