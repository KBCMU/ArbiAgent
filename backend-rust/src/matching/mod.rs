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
pub mod label_resolver;
pub mod matcher;
pub mod shadow;
pub mod team_dictionary;

use std::sync::Arc;

use reqwest::Client;
use tracing::{error, info, warn};

use crate::models::event::Sport;
use crate::AppState;

// ─── Public Entry Point ─────────────────────────────────────────────

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

    let _kalshi_count = kalshi_candidates.len();
    let _poly_count = poly_candidates.len();

    // Run the matching algorithm
    let min_score = state.config.match_min_score;
    let events = matcher::match_candidates(
        kalshi_candidates,
        poly_candidates,
        Some(min_score),
    );

    // Count matched (dual-platform) events
    let matched = events
        .iter()
        .filter(|e| {
            !e.platform_ids.kalshi_market_tickers.is_empty()
                && !e.platform_ids.polymarket_token_ids.is_empty()
        })
        .count();

    let total = events.len();

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

    info!(
        "Inserted {} new events, re-labeled {} cached events with generic Yes/No outcomes",
        new_count, relabeled_count
    );

    Ok((matched, total))
}
