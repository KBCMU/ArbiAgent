---
focus: pitfalls
generated: 2026-03-29
domain: cross-platform prediction market sports event matching (Polymarket + Kalshi)
confidence: HIGH — all findings grounded in actual codebase, observed bugs, and lessons.md
---

# Domain Pitfalls: Cross-Platform Sports Event Matching

**Domain:** Polymarket ↔ Kalshi sports event correlation
**Researched:** 2026-03-29
**Confidence:** HIGH — findings drawn from production code, lessons.md, and active bug reports

---

## Critical Pitfalls

Mistakes that cause rewrites or fundamentally wrong behavior.

---

### Pitfall 1: Treating Kalshi `close_time` as Game Date

**What goes wrong:** Kalshi events expose a `close_time` field (when the market closes for trading), which in `fetcher_kalshi.rs` is used as the `game_start_time`. For most games this is close enough, but for games that go to overtime, are postponed, or have market close windows that differ from tip-off/puck-drop, the derived `game_date` is off by one day.

**Why it happens:** Kalshi does not expose an explicit game start time field in its events API. The only temporal anchor is `close_time`. The date is instead extracted from the event ticker itself (e.g., `KXNHLGAME-26MAR15BOSNJD` → March 15, 2026), but if the ticker date and the close_time date disagree (late-night games that close the next day UTC), the bucket key `(sport, close_time_date, market_type)` won't match Polymarket's bucket key which uses `end_date` or `start_date`.

**Consequences:**
- Bucketing by `(Sport, NaiveDate, MarketTypeBucket)` puts same-day games in different buckets because one platform resolves to `2026-03-15` and the other to `2026-03-16` (UTC rollover around midnight ET).
- Events genuinely on both platforms only show one platform's odds.
- Incorrect dates displayed in the UI.

**Warning signs:**
- NBA/NHL games that start at 9–10 PM ET routinely appear unmatched.
- Games in the Western US time zones are disproportionately unmatched.
- `game_date` in `build_event_id()` chooses Polymarket's date, but bucket matching used Kalshi's date — so you get a matched event with a different date than the bucket it was found in.

**Prevention:**
- Always extract game date from the Kalshi ticker string (`26MAR15` → March 15) rather than from `close_time`. The ticker date is the local game date; `close_time` is a trading deadline.
- In the bucket pass, allow ±1 day tolerance: if `(sport, date, market_type)` bucket is empty, also check `(sport, date+1, market_type)` and `(sport, date-1, market_type)`.
- Add an assertion in `build_event_id()` that the chosen date matches the ticker-extracted date; log a warning when they diverge.

**Phase:** Address in the matching accuracy milestone before claiming correct event dates.

---

### Pitfall 2: Polymarket `end_date` Is Market Expiry, Not Game Time

**What goes wrong:** The Gamma API's `end_date` field is when the market expires (resolves), not when the game starts. For same-day NBA games this is usually fine, but for markets that stay open several days after the game ends (e.g., official result disputes), `end_date` can be 2–3 days after the game. Using it as the canonical date makes the event appear in the future or on the wrong day.

**Why it happens:** The Gamma API does not guarantee `startDate` is always populated. The fetcher falls back from slug-parsed date → `end_date` → `start_date` → title extraction. If the slug doesn't encode a date and `startDate` is null, `end_date` is used blindly.

**Consequences:**
- `build_event_id()` preferring Polymarket's date embeds the wrong date in the canonical event ID.
- A game on March 15 gets ID `nba-bos-lac-2026-03-17` because the market doesn't expire until March 17.
- Subsequent matching cycles see a different event ID and re-insert the event as a duplicate.

**Warning signs:**
- Duplicate events in the UI that share team names and sport but have different dates.
- ID mismatches between cycles (cache inserts new row instead of upserting).
- Polymarket slugs that don't contain dates (common for season-long futures).

**Prevention:**
- Prefer `startDate` unconditionally; only fall back to `end_date` when `startDate` is null.
- Parse date from slug first (slug often encodes game date like `nba-lakers-celtics-3-15-2026`).
- When neither is reliable, extract from title before trusting `end_date`.
- Log a `warn!` whenever `end_date` is used as the date source.

**Phase:** Address in matching accuracy milestone.

---

### Pitfall 3: Same Abbreviation, Different Sports (Disambiguation Failure)

**What goes wrong:** Abbreviations like `ATL`, `CHI`, `DET`, `MIA`, `MIN`, `PHI`, `CLE`, `HOU`, `DAL`, `WAS` exist in multiple sports in `team_dictionary.rs`. Without sport context at the lookup site, the dictionary resolves to the first match it finds (determined by entry order in the static array).

**Why it happens:** `team_dictionary::lookup_team()` takes `Option<Sport>` but callers in `fetcher_polymarket.rs` sometimes pass `Some(sport)`, and callers in `label_resolver.rs` use `match_label_to_abbrev()` which doesn't take a sport parameter at all. If a Polymarket event has an ambiguous tag that gets classified as `Sport::Nba`, but the game is actually `Sport::Mlb`, the team abbreviation lookup returns the NBA team — which then fails to match the Kalshi candidate that correctly identified the team as MLB.

**Consequences:**
- False positives: cross-sport matches where "Cubs" (CHI/MLB) matches against "Bulls" (CHI/NBA) because both resolve to `CHI`.
- False negatives: team abbreviation resolution correct on Kalshi but wrong on Polymarket, so team names don't match even though it's the same event.

**Warning signs:**
- CHI, ATL, DET events matching with suspiciously low scores (title similarity low, team score contributed by the abbreviation collision).
- Mismatched events in the UI (e.g., NBA event showing MLB odds).

**Prevention:**
- Always pass sport context when calling `lookup_team()`.
- In `label_resolver.rs`, pass the sport down through `resolve_polymarket_labels()` and use it in `match_label_to_abbrev()`.
- Add a test for every multi-sport abbreviation collision (ATL, CHI, DET, etc.) verifying that sport context correctly disambiguates.

**Phase:** Address early in the matching milestone; this also pollutes the arbitrage detector.

---

### Pitfall 4: Polymarket Grouped Event Structure Emitting Multiple Candidates for One Real Game

**What goes wrong:** Polymarket uses two distinct market structures for the same real event: (a) a single market with `outcomes: ["Lakers","Celtics"]`, and (b) a "grouped event" with one market per team, each with `groupItemTitle: "Lakers"` and `Yes/No` outcomes. The fetcher detects which structure is present, but when a Gamma event has BOTH a moneyline market AND grouped markets, the fetcher can emit two separate `CandidateEvent`s for the same game.

**Why it happens:** `extract_moneyline_market()` checks for the grouped structure and falls back to Yes/No outcomes if no grouped markets are found. But if the API response includes the moneyline market AND spurious grouped markets from a different context, both code paths fire.

**Consequences:**
- Two Polymarket candidates for the same game both match against one Kalshi candidate.
- The Hungarian algorithm assigns one match; the other becomes a single-platform event.
- UI shows the game twice — once with both platforms' odds and once with only Polymarket's odds.
- This is the primary source of the "duplicate events" bug.

**Warning signs:**
- Duplicate entries in the UI where both share the same team names and date.
- One entry shows both platforms' odds; another shows only Polymarket.
- `matched_pairs` count in logs is lower than the number of events with both platforms.

**Prevention:**
- After `process_gamma_events()`, deduplicate candidates by `polymarket_slug` — only keep the first (most complete) candidate per slug.
- Emit only ONE moneyline candidate per slug regardless of whether grouped structure was detected.
- Add an assertion: after deduplication, the count of candidates per slug should be ≤ 1 per market type.

**Phase:** Address as first fix in matching accuracy milestone — highest user-visible impact.

---

### Pitfall 5: Event ID Instability Across Matching Cycles

**What goes wrong:** `build_event_id()` constructs IDs like `nba-bos-lac-2026-03-15`. If any input changes between cycles (team abbreviation resolved differently, date source switches from `end_date` to `start_date`, one team missing and filled with "unk"), a new ID is generated. The cache upsert sees a new ID and inserts a new record, while the old ID remains stale in the cache until eviction.

**Why it happens:** The event ID depends on extracted/resolved fields that can be non-deterministic: which Polymarket date field happens to be non-null, which team abbreviation lookup path fires, fallback to "unk" when team extraction fails. Any flakiness in these lookups causes ID churn.

**Consequences:**
- Each discovery cycle may produce slightly different IDs for the same game.
- Cache eviction (`run_cache_eviction_loop`) doesn't run for 30 minutes, so old IDs accumulate.
- UI shows duplicate events growing over time until eviction.
- Arb detection misses opportunities because odds are attached to one ID but the event is under another.

**Warning signs:**
- Total event count grows across matching cycles without real new games being added.
- Events appearing with "unk" in their ID (team extraction failing silently).
- Log shows "Matching complete: X matched pairs" but UI shows more entries than X.

**Prevention:**
- Derive event ID from a stable, platform-provided identifier, not from extracted fields. Use `kalshi_event_ticker` as the canonical ID anchor (it's deterministic from the API). E.g., `kalshi-KXNHLGAME-26MAR15BOSNJD`.
- Use extracted fields (teams, date) only for matching, not for ID generation.
- When a matched pair's computed ID differs from the previously cached ID for the same ticker, update the cache entry in-place rather than inserting a new one.

**Phase:** Foundational — resolve before other duplicate fixes or they will be masked.

---

## Moderate Pitfalls

Mistakes that cause degraded coverage or incorrect odds, but not full breaks.

---

### Pitfall 6: Min-Score Threshold (60.0) Too Blunt for Date-Unknown Events

**What goes wrong:** When Polymarket's Gamma event has no parseable date (slug has no date, `startDate` null, `end_date` ambiguous), the matching bucket for that candidate uses `game_date: None`. In `bucket_by_sport_date_market()`, a `None` date creates a separate bucket. The fallback cross-bucket pass groups by `(Sport, MarketTypeBucket)` without date — but the max possible score for a dateless pair is 80 (50 both-teams + 20 title + 10 moneyline), not 90. The threshold of 60 should still admit correct matches, but for games where team names are hard to extract (college sports, international players), the score may fall short.

**Why it happens:** Date absence is treated as a partial penalty (no +20 points for date match), but this implicitly disadvantages events from platforms that omit dates in their API responses.

**Prevention:**
- When `game_date` is None on the Polymarket side, compute a "date-missing penalty" score differently: accept a match at score ≥ 50 (just both teams) when the date is missing on exactly one side, as long as the title similarity is above a threshold.
- Or: in the cross-bucket pass, give a small date-plausible bonus when the Kalshi ticker date is within 7 days (current/recent game) rather than requiring exact match.

**Phase:** Second priority after duplicates are fixed.

---

### Pitfall 7: Prop Bets and Player Markets Leaking Into Game Matching

**What goes wrong:** Both Polymarket and Kalshi publish player-level prop markets (e.g., "Will LeBron James score 30+ points?") alongside game-level moneyline markets. These share the same sport tag (`nba`), the same date, and can have partial title similarity. The scorer gives them low but non-zero team scores (if player's team name appears), and they can match against a legitimate moneyline market from the other platform.

**Why it happens:** `is_futures_event()` in the Polymarket fetcher filters out championship futures but doesn't filter player props. Kalshi's fetcher classifies everything non-spread/non-total as `MarketType::Moneyline` including props.

**Consequences:**
- A Kalshi prop market ("Will Tatum score 35+ points?") matched against a Polymarket game market ("Celtics vs Lakers") shows garbled odds.
- Arb detector computes nonsense opportunities from mismatched outcomes.

**Warning signs:**
- Markets with outcome labels like "Yes"/"No" persisting in the matched events despite the fetcher's generic-label-clearing logic.
- Low `match_score` values (60–65) for events that appear in the dual-platform view.

**Prevention:**
- In `fetcher_kalshi.rs`, flag events where the moneyline tickers contain words like "POINTS", "REBOUNDS", "ASSISTS", "YARDS", "TOUCHDOWNS" and emit them with a `MarketType::Prop` variant that is excluded from cross-platform matching entirely.
- In the Polymarket fetcher, extend `is_futures_event()` to also check for player name patterns in the title.
- Add a separate prop bucket that is excluded from the main matching loop.

**Phase:** Address after core duplicates and date issues are resolved.

---

### Pitfall 8: Postseason vs Regular Season Event Confusion

**What goes wrong:** "Celtics vs Nets" appears in both regular season and postseason. Kalshi creates new event tickers for playoff games. Polymarket may keep the same slug format but update the event. If both a March regular season game and an April playoff game are in-flight simultaneously (Kalshi series starts before regular season ends), they can get matched against each other.

**Why it happens:** Playoff event titles are often identical to regular season titles. Both appear as `Sport::Nba`, same teams, dates 1–3 days apart (which gets the 75% date score). Score can be 60–70+ and pass the threshold.

**Prevention:**
- Check for postseason keywords ("playoff", "series", "game 1", "round 1", "conference") in title; exclude from regular-season matching.
- Or: add a `MarketTypeBucket::Playoff` variant and only match playoff events against other playoff events.
- Kalshi ticker prefix often differs for playoff events (check series_ticker prefix).

**Phase:** Relevant during NBA/NHL playoff season (April–June); low priority for current March scope.

---

### Pitfall 9: Stale Cached Events with Live Odds Attached to Dead IDs

**What goes wrong:** When a game is matched and its canonical event ID is recorded in the cache, the WebSocket price ingester (`kalshi_ws`, `polymarket_ws`) updates odds by event ID. If the matching loop produces a new ID for the same game in the next cycle (see Pitfall 5), the new event record has no odds, while the old dead record still has live odds. The arb detector sees a dual-platform event with no odds and produces no arb signals.

**Why it happens:** The cache eviction loop only runs every 30 minutes. The matching loop runs on a shorter `event_discovery_interval_secs` cycle. New IDs accumulate faster than eviction removes stale ones.

**Prevention:**
- When the matching loop writes a `CanonicalEvent` to the cache, also migrate any existing odds keyed under previous IDs for the same `kalshi_event_ticker` or `polymarket_slug`.
- Or: key the odds cache by platform-native ID (`kalshi_event_ticker`, `polymarket_slug`) rather than canonical event ID, and join on demand at read time.

**Phase:** Depends on Pitfall 5 fix — resolve after event ID is stabilized.

---

### Pitfall 10: Kalshi Spread Line Extraction Failing Silently

**What goes wrong:** `extract_line_from_ticker()` in `fetcher_kalshi.rs` uses `rsplit('-')` to find a numeric segment in tickers like `KXNBAGAME-26MAR15LALBOS-SPREAD-BOS-3_5`. The `3_5` (using underscore as decimal) is parsed by replacing `_` with `.`, but the function returns `0.0` on any parse failure. Two spread markets with different lines (e.g., `-3.5` and `-6.5`) both produce `Spread(0.0)`, which means they're in the same `MarketTypeBucket::Spread(0)` and can be incorrectly cross-matched.

**Why it happens:** Silent fallback to `0.0` was convenient for the initial build but masks extraction failures.

**Consequences:**
- Two Kalshi spread markets at different lines both match against the same Polymarket spread candidate.
- Hungarian algorithm picks one; the other becomes single-platform.
- Users see the wrong spread line in the UI.

**Warning signs:**
- Multiple Kalshi spread candidates for the same game collapsing into a single entry.
- Spread line shown as "0.0" in logs or UI.

**Prevention:**
- Return `Option<f64>` from `extract_line_from_ticker()`; skip emitting the spread candidate if the line cannot be parsed.
- Add a unit test: given `KXNBAGAME-26MAR15LALBOS-SPREAD-BOS-3_5`, verify line = 3.5 not 0.0.

**Phase:** Fix alongside market type improvements; small scope, high correctness value.

---

### Pitfall 11: Franchise Renames and Relocations Not Reflected in Team Dictionary

**What goes wrong:** `team_dictionary.rs` is a static file. Franchise changes (like the Utah Hockey Club replacing Arizona Coyotes) require manual updates. Until updated, the old abbreviation maps to the defunct team and the new team appears as unknown, falling through to passthrough mode. Passthrough returns the raw string (e.g., "Utah Hockey Club"), which won't match against Kalshi's canonical ticker abbreviation for the same team.

**Why it happens:** This is the documented "Utah Hockey Club / ARI" lesson from lessons.md. Static dictionaries require human maintenance.

**Consequences:**
- NHL Utah events are unmatched even when both platforms have them.
- Passthrough string is long and won't match abbreviated forms.

**Warning signs:**
- Unmatched events for teams that *should* be on both platforms.
- `team_a` or `team_b` containing a full team name instead of a 2–4 letter abbreviation.

**Prevention:**
- Add a validation pass after `team_dictionary` lookups that warns when a team name was passed through without abbreviation resolution.
- Review dictionary at the start of each sports season (October for NBA/NHL/NFL, April for MLB).
- Consider a secondary lookup that maps recent common aliases (e.g., "Utah HC", "Utah Hockey Club", "UHC") to the canonical NHL abbreviation.

**Phase:** Ongoing maintenance. Add a dictionary validation step to the CI/CD pipeline that asserts all Sport variants have expected team counts.

---

## Minor Pitfalls

---

### Pitfall 12: Jaccard Token Similarity Penalizing Well-Formatted Titles

**What goes wrong:** `jaccard_token_similarity()` computes overlap by tokens. Kalshi titles are terse (e.g., "NHL: BOS vs NJD"), while Polymarket titles are verbose (e.g., "Will the Boston Bruins defeat the New Jersey Devils on March 15?"). After normalization, the Polymarket title has 9 unique tokens; the Kalshi title has 4. Only 2–3 tokens overlap (team abbreviations), so Jaccard similarity is 3/10 = 0.3, contributing only 6 out of 20 possible title score points. For games where team extraction also partially fails, the total score may sit just above 60.

**Prevention:**
- Weight bigram or TF-IDF similarity instead of pure Jaccard, which penalizes longer correct descriptions.
- Or: augment Kalshi titles before scoring by expanding abbreviations to full team names ("BOS" → "boston") using the team dictionary — this increases token overlap.
- The current fallback cross-bucket pass already uses a lower effective score ceiling for dateless events; a title similarity adjustment would help there.

**Phase:** Tuning pass after correctness is established.

---

### Pitfall 13: Polymarket Pagination Using `offset` May Miss High-Volume Events

**What goes wrong:** The Polymarket fetcher paginates using `offset` up to `max_pages=10` (1,000 events). Events are ordered by `volume24hr descending`. Niche sports events (college sports, tennis) have low volume and sort toward the end. If there are more than 1,000 active Polymarket events during peak periods (March Madness brackets), some events are never fetched.

**Prevention:**
- Increase `max_pages` to 20 or fetch sport-specific tags with their own pagination (already done for per-sport tags, but the limit applies there too).
- For NCAA tournament periods, reduce the per-tag limit and increase max-pages.
- Log how many pages were fetched and whether the last page was full (indicating possible truncation).

**Phase:** Relevant during high-volume tournament periods.

---

### Pitfall 14: Hungarian Algorithm Greedy Assignment Missing Better Global Solution in Large Buckets

**What goes wrong:** `match_within_bucket()` builds the full score matrix and calls `hungarian::max_weight_assignment()`. For large buckets (many games on the same sport+date), the algorithm produces the globally optimal assignment. But if buckets are mis-formed (wrong date extracted for some events), some games end up in a large cross-bucket fallback pass. The fallback iterates all unmatched events by `(sport, market_type)` — potentially pairing Game A against Game B's counterpart because both had similar team names extracted.

**Prevention:**
- Tighten date extraction before the Hungarian pass to reduce fallback-bucket size.
- Cap fallback bucket score threshold higher (e.g., 70 instead of 60) because date uncertainty increases false positive risk.

**Phase:** Fine-tuning after date extraction is fixed.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|---|---|---|
| Fix duplicate events | Pitfall 4 (multi-candidate per Polymarket slug) + Pitfall 5 (ID instability) | Deduplicate by slug; stabilize IDs on Kalshi ticker |
| Fix missed matches | Pitfall 1 (Kalshi date from close_time) + Pitfall 2 (Polymarket end_date) + Pitfall 6 (dateless events) | Use ticker-extracted date; prefer startDate; widen date tolerance in bucketing |
| Fix wrong dates in UI | Pitfall 1 + Pitfall 2 | Fix date source priority in both fetchers |
| Fix outcome label alignment | Pitfall 3 (abbreviation disambiguation) + Pitfall 7 (prop market leakage) | Pass sport context through label resolver; add prop detection |
| Maintain over time | Pitfall 11 (franchise renames) | Automated dictionary validation in CI |
| Spread/total correctness | Pitfall 10 (silent line extraction failure) | Return Option<f64>; add unit tests |

---

## Evidence Sources

All findings are HIGH confidence — drawn from:

- `backend-rust/src/matching/matcher.rs` — bucketing logic, scoring, ID construction
- `backend-rust/src/matching/fetcher_kalshi.rs` — date extraction, `close_time` usage, spread line parsing
- `backend-rust/src/matching/fetcher_polymarket.rs` — `end_date` usage, grouped market extraction, deduplication by slug
- `backend-rust/src/matching/candidate.rs` — `extract_date_from_kalshi_ticker()` implementation
- `backend-rust/src/matching/team_dictionary.rs` — multi-sport abbreviation collisions documented in static entries
- `backend-rust/src/matching/label_resolver.rs` — sport context absent from `match_label_to_abbrev()`
- `tasks/lessons.md` — recorded production failures (Utah HC/ARI, Polymarket pagination limit, grouped event structure)
- `.planning/PROJECT.md` — active bug list: duplicates, missed matches, wrong dates
