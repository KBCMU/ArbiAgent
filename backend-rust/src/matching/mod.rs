//! Native sports market matching engine.
//!
//! Replaces DomeAPI's `matching-markets` endpoint with a local matching engine
//! that independently fetches events from Kalshi and Polymarket, then correlates
//! them using a multi-signal scoring algorithm.
//!
//! ## Module structure
//!
//! - `team_dictionary` — shared team name ↔ abbreviation mappings
//! - `candidate` — `CandidateEvent` struct and normalization utilities
//! - `fetcher_kalshi` — Kalshi events API fetcher
//! - `fetcher_polymarket` — Polymarket Gamma API fetcher
//! - `matcher` — cross-platform scoring and assignment
//! - `label_resolver` — Polymarket outcome label resolution (Gamma API)

pub mod candidate;
pub mod fetcher_kalshi;
pub mod fetcher_polymarket;
pub mod hungarian;
pub mod label_resolver;
pub mod matcher;
pub mod shadow;
pub mod team_dictionary;

use std::collections::HashSet;
use std::sync::Arc;

use reqwest::Client;
use tracing::{error, info, warn};

use crate::models::event::Sport;
use crate::AppState;

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::event::{CanonicalEvent, PlatformIds, Sport};
    use chrono::Utc;

    fn make_event_with_ids(id: &str, kalshi_ticker: Option<&str>, poly_slug: Option<&str>, poly_token_ids: Vec<&str>) -> CanonicalEvent {
        let now = Utc::now();
        CanonicalEvent {
            id: id.to_string(),
            sport: Sport::Nba,
            event_title: "Test".to_string(),
            game_start_time: None,
            status: "open".to_string(),
            platform_ids: PlatformIds {
                kalshi_event_ticker: kalshi_ticker.map(|s| s.to_string()),
                kalshi_market_tickers: vec![],
                polymarket_market_slug: poly_slug.map(|s| s.to_string()),
                polymarket_token_ids: poly_token_ids.iter().map(|s| s.to_string()).collect(),
                polymarket_outcome_labels: vec![],
            },
            match_score: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_dedup_pass_collapses_duplicates() {
        let a = make_event_with_ids("nba-lal-bos-2026-03-14", Some("KXNBA-X"), Some("nba-x"), vec!["tok1"]);
        let b = make_event_with_ids("nba-lal-bos-2026-03-14", None, Some("nba-x"), vec![]);
        let result = dedup_events_by_id(vec![a, b]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].platform_ids.kalshi_event_ticker, Some("KXNBA-X".to_string()));
        assert_eq!(result[0].platform_ids.polymarket_token_ids, vec!["tok1".to_string()]);
    }

    #[test]
    fn test_dedup_pass_keeps_dual_over_single() {
        // B (single) arrives before A (dual) — dual must still win
        let a = make_event_with_ids("nba-lal-bos-2026-03-14", Some("KXNBA-X"), Some("nba-x"), vec!["tok1"]);
        let b = make_event_with_ids("nba-lal-bos-2026-03-14", None, Some("nba-x"), vec![]);
        let result = dedup_events_by_id(vec![b, a]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].platform_ids.kalshi_event_ticker, Some("KXNBA-X".to_string()));
    }

    #[test]
    fn test_dedup_pass_merges_platform_ids() {
        // A is kalshi-only, B is polymarket-only — result must have both
        let a = make_event_with_ids("nba-lal-bos-2026-03-14", Some("KXNBA-X"), None, vec![]);
        let b = make_event_with_ids("nba-lal-bos-2026-03-14", None, Some("nba-x"), vec!["tok1"]);
        let result = dedup_events_by_id(vec![a, b]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].platform_ids.kalshi_event_ticker, Some("KXNBA-X".to_string()));
        assert_eq!(result[0].platform_ids.polymarket_token_ids, vec!["tok1".to_string()]);
    }

    #[test]
    fn test_dedup_pass_no_duplicates_passthrough() {
        let a = make_event_with_ids("event-a", Some("K1"), Some("slug-a"), vec![]);
        let b = make_event_with_ids("event-b", Some("K2"), Some("slug-b"), vec![]);
        let c = make_event_with_ids("event-c", Some("K3"), Some("slug-c"), vec![]);
        let result = dedup_events_by_id(vec![a, b, c]);
        assert_eq!(result.len(), 3);
    }
}

// ─── Public Entry Point ─────────────────────────────────────────────

/// D-02: Post-construction dedup pass.
/// Collapses CanonicalEvents that share the same ID.
/// Non-lossy: keeps the highest-coverage entry as the base, then backfills
/// any None/empty platform_ids fields from the lower-coverage entry so no
/// token IDs or market tickers are silently discarded.
fn dedup_events_by_id(events: Vec<crate::models::event::CanonicalEvent>) -> Vec<crate::models::event::CanonicalEvent> {
    use std::collections::HashMap;
    let mut seen: HashMap<String, crate::models::event::CanonicalEvent> = HashMap::new();
    for event in events {
        if let Some(entry) = seen.get_mut(&event.id) {
            let stored_is_dual = entry.platform_ids.kalshi_event_ticker.is_some()
                && entry.platform_ids.polymarket_market_slug.is_some();
            let incoming_is_dual = event.platform_ids.kalshi_event_ticker.is_some()
                && event.platform_ids.polymarket_market_slug.is_some();
            if incoming_is_dual && !stored_is_dual {
                // Incoming has more coverage — use it as the base.
                *entry = event;
            } else {
                // Stored entry wins (or equal coverage). Backfill any None/empty
                // fields from incoming so we don't lose data from either side.
                if entry.platform_ids.kalshi_event_ticker.is_none() {
                    entry.platform_ids.kalshi_event_ticker =
                        event.platform_ids.kalshi_event_ticker.clone();
                }
                if entry.platform_ids.kalshi_market_tickers.is_empty() {
                    entry.platform_ids.kalshi_market_tickers =
                        event.platform_ids.kalshi_market_tickers.clone();
                }
                if entry.platform_ids.polymarket_market_slug.is_none() {
                    entry.platform_ids.polymarket_market_slug =
                        event.platform_ids.polymarket_market_slug.clone();
                }
                if entry.platform_ids.polymarket_token_ids.is_empty() {
                    entry.platform_ids.polymarket_token_ids =
                        event.platform_ids.polymarket_token_ids.clone();
                }
                if entry.platform_ids.polymarket_outcome_labels.is_empty() {
                    entry.platform_ids.polymarket_outcome_labels =
                        event.platform_ids.polymarket_outcome_labels.clone();
                }
            }
        } else {
            seen.insert(event.id.clone(), event);
        }
    }
    seen.into_values().collect()
}

/// Background loop: discovers and matches sports events natively every N seconds.
///
/// Drop-in replacement for `dome_poller::run_event_discovery_loop`.
pub async fn run_sports_matching_loop(state: Arc<AppState>) {
    let interval = tokio::time::Duration::from_secs(
        state.config.event_discovery_interval_secs,
    );

    info!("Native sports matching loop started (interval: {}s)", interval.as_secs());

    loop {
        match discover_and_match_sports(&state).await {
            Ok((matched, total)) => {
                info!(
                    "Native matching: {} matched pairs, {} total events ({} in cache)",
                    matched,
                    total,
                    state.cache.event_count(),
                );
            }
            Err(e) => {
                error!("Native sports matching failed: {}", e);
            }
        }

        tokio::time::sleep(interval).await;
    }
}

/// Single discovery + matching cycle. Returns (matched_pairs, total_events).
async fn discover_and_match_sports(state: &AppState) -> anyhow::Result<(usize, usize)> {
    let sports = Sport::sports_for_discovery().to_vec();

    // Fetch from both platforms concurrently
    let (kalshi_candidates, poly_candidates) = tokio::join!(
        fetcher_kalshi::fetch_kalshi_sports_candidates(&sports),
        fetcher_polymarket::fetch_polymarket_sports_candidates(&sports),
    );

    info!(
        "Fetched {} Kalshi + {} Polymarket candidates",
        kalshi_candidates.len(),
        poly_candidates.len(),
    );

    let kalshi_count = kalshi_candidates.len();
    let poly_count = poly_candidates.len();

    // Run the matching algorithm
    let min_score = state.config.match_min_score;
    let match_result = matcher::match_candidates(
        kalshi_candidates,
        poly_candidates,
        Some(min_score),
    );

    let matched = match_result.matched_pairs;
    let events = match_result.events;
    let unmatched_details = match_result.unmatched_details;
    // D-02: collapse any duplicate IDs before proceeding
    let events = dedup_events_by_id(events);
    let total = events.len();

    // Store matching stats for the /api/v2/matching/stats endpoint
    let avg_score = {
        let scores: Vec<f64> = events
            .iter()
            .filter_map(|e| e.match_score)
            .collect();
        if scores.is_empty() {
            0.0
        } else {
            scores.iter().sum::<f64>() / scores.len() as f64
        }
    };

    let unmatched_kalshi_count = unmatched_details
        .iter()
        .filter(|d| d.platform == "kalshi")
        .count();
    let unmatched_poly_count = unmatched_details
        .iter()
        .filter(|d| d.platform == "polymarket")
        .count();

    {
        use crate::storage::cache::{MatchingStats, UnmatchedStat};
        let stats = MatchingStats {
            last_run_at: chrono::Utc::now(),
            kalshi_candidates: kalshi_count,
            polymarket_candidates: poly_count,
            matched_pairs: matched,
            unmatched_kalshi: unmatched_kalshi_count,
            unmatched_polymarket: unmatched_poly_count,
            avg_match_score: avg_score,
            unmatched: unmatched_details
                .into_iter()
                .map(|d| UnmatchedStat {
                    event_title: d.event_title,
                    platform: d.platform,
                    sport: d.sport,
                    best_rejected_score: d.best_rejected_score,
                    best_rejected_title: d.best_rejected_title,
                })
                .collect(),
        };
        *state.cache.matching_stats.write().unwrap() = Some(stats);
    }

    // Snapshot discovered sports event IDs for stale-ID reconciliation later.
    let discovered_sports_ids: HashSet<String> = events
        .iter()
        .map(|e| e.id.clone())
        .collect();

    // Resolve Polymarket outcome labels for matched events using Gamma API
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let mut new_count = 0usize;
    let mut relabeled_count = 0usize;

    for mut event in events {
        let is_cached = state.cache.events.contains_key(&event.id);

        // For dual-platform events, check if labels need resolution.
        // Only resolve if labels are generic "Yes"/"No"/empty — NOT if the fetcher
        // already populated real team names (e.g. "Warriors", "Knicks").
        let current_labels = &event.platform_ids.polymarket_outcome_labels;
        let current_labels_are_generic = current_labels.is_empty()
            || current_labels.iter().any(|l| {
                let u = l.to_uppercase();
                u == "YES" || u == "NO" || l.is_empty()
            });

        let needs_label_resolution = !event.platform_ids.kalshi_market_tickers.is_empty()
            && !event.platform_ids.polymarket_token_ids.is_empty()
            && {
                if is_cached {
                    // Check the cached event's labels
                    state.cache.events.get(&event.id).map_or(true, |cached| {
                        cached.platform_ids.polymarket_outcome_labels.iter().any(|l| {
                            let u = l.to_uppercase();
                            u == "YES" || u == "NO" || l.is_empty()
                        })
                    })
                } else {
                    // New event — only resolve if current labels are generic
                    current_labels_are_generic
                }
            };

        // Detect if cached event's token_ids are stale (e.g., from the grouped
        // market fix — old code grabbed one team's Yes/No tokens instead of
        // each team's Yes token).
        let token_ids_changed = if is_cached {
            state.cache.events.get(&event.id).map_or(false, |cached| {
                cached.platform_ids.polymarket_token_ids
                    != event.platform_ids.polymarket_token_ids
            })
        } else {
            false
        };

        if token_ids_changed {
            info!(
                "Token IDs changed for {} — clearing stale Polymarket odds",
                event.id
            );
            state.cache.clear_platform_odds(&event.id, "polymarket");
        }

        if needs_label_resolution {
            let resolved = label_resolver::resolve_polymarket_labels(
                &client,
                &event.platform_ids.polymarket_market_slug,
                &event.platform_ids.polymarket_token_ids,
                &event.platform_ids.kalshi_market_tickers,
            )
            .await;

            // Only update if we got real team labels (not still Yes/No)
            let resolved_are_real = resolved.iter().any(|l| {
                let u = l.to_uppercase();
                u != "YES" && u != "NO" && !l.is_empty()
            });

            if resolved_are_real {
                // Validate alignment: resolved Polymarket labels should match
                // the set of Kalshi outcomes. If they don't, log a warning.
                let kalshi_outcomes: HashSet<String> = event
                    .platform_ids
                    .kalshi_market_tickers
                    .iter()
                    .map(|t| team_dictionary::extract_kalshi_outcome(t))
                    .collect();
                let poly_outcomes: HashSet<String> = resolved.iter().cloned().collect();

                if !kalshi_outcomes.is_empty()
                    && !poly_outcomes.is_empty()
                    && kalshi_outcomes != poly_outcomes
                {
                    warn!(
                        "Label mismatch for {}: Kalshi={:?} vs Polymarket={:?}",
                        event.id, kalshi_outcomes, poly_outcomes
                    );
                }

                event.platform_ids.polymarket_outcome_labels = resolved;
                if is_cached {
                    state.cache.clear_platform_odds(&event.id, "polymarket");
                    relabeled_count += 1;
                }
            } else if !is_cached {
                event.platform_ids.polymarket_outcome_labels = resolved;
            }
        } else if is_cached && !token_ids_changed {
            // Labels are good AND token_ids unchanged — but check if the fetcher
            // has normalized labels (e.g. "Bruins" → "BOS") that differ from cache.
            let cached_labels = state.cache.events.get(&event.id)
                .map(|c| c.platform_ids.polymarket_outcome_labels.clone())
                .unwrap_or_default();
            if cached_labels != event.platform_ids.polymarket_outcome_labels {
                // Labels changed even though token IDs are stable. Clear existing
                // polymarket odds to avoid mixed old/new outcome keys (3-outcome bug).
                state.cache.clear_platform_odds(&event.id, "polymarket");
                state.cache.upsert_event(event.clone());
                if let Err(e) = state.db.upsert_event(&event).await {
                    warn!("DB write skipped for {}: {}", event.id, e);
                }
                relabeled_count += 1;
            }
            continue;
        }

        state.cache.upsert_event(event.clone());

        // Best-effort DB write
        if let Err(e) = state.db.upsert_event(&event).await {
            warn!("DB write skipped for {}: {}", event.id, e);
        }
        new_count += 1;
    }

    // Reconcile superseded sports IDs only when both platforms returned data.
    // This avoids deleting valid cache entries during transient single-platform outages.
    if kalshi_count > 0 && poly_count > 0 {
        let stale_sports_ids: Vec<String> = state
            .cache
            .events
            .iter()
            .filter(|entry| {
                let event = entry.value();
                event.sport.is_sport() && !discovered_sports_ids.contains(entry.key())
            })
            .map(|entry| entry.key().clone())
            .collect();

        for id in stale_sports_ids {
            state.cache.events.remove(&id);
            state.cache.odds.remove(&id);
            state.cache.active_arbs.remove(&id);
        }
    }

    info!(
        "Inserted {} new events, re-labeled {} cached events with generic Yes/No outcomes",
        new_count, relabeled_count
    );

    Ok((matched, total))
}
