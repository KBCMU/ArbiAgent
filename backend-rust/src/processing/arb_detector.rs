use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use tracing::{info, warn};

use crate::models::arb::ArbitrageOpportunity;
use crate::models::event::EventOdds;
use crate::models::ws::{ArbUpdateData, WsMessage};
use crate::AppState;

/// A unique key identifying an arb window (event + outcome + direction).
#[derive(Hash, Eq, PartialEq, Clone)]
struct ArbKey {
    event_id: String,
    outcome: String,
    buy_platform: String,
}

/// Background loop: scans all events for arbitrage opportunities.
/// Tracks arb window durations — when an arb disappears, it closes the DB record.
pub async fn run_arb_detection_loop(state: Arc<AppState>) {
    let interval = tokio::time::Duration::from_secs(2);

    // Wait for initial price data
    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
    info!("🎯 Arbitrage detector started");

    // Track which arbs were active last cycle so we can detect closures
    let mut prev_arb_keys: HashSet<ArbKey> = HashSet::new();

    loop {
        let events_with_odds = state.cache.get_all_events_with_odds();
        let mut total_arbs = 0usize;
        let mut current_arb_keys: HashSet<ArbKey> = HashSet::new();

        for (event, odds_opt) in &events_with_odds {
            if let Some(odds) = odds_opt {
                let arbs = detect_arbitrage(
                    &event.id,
                    &event.event_title,
                    event.sport.as_str(),
                    odds,
                    &state,
                );

                if !arbs.is_empty() {
                    total_arbs += arbs.len();

                    // Record current arb keys for duration tracking
                    for arb in &arbs {
                        current_arb_keys.insert(ArbKey {
                            event_id: arb.canonical_event_id.clone(),
                            outcome: arb.outcome.clone(),
                            buy_platform: arb.buy_platform.clone(),
                        });
                    }

                    // Store in cache
                    state.cache.set_active_arbs(&event.id, arbs.clone());

                    // Broadcast to WebSocket clients
                    let msg = WsMessage::ArbUpdate(ArbUpdateData {
                        canonical_event_id: event.id.clone(),
                        event_title: event.event_title.clone(),
                        sport: event.sport.as_str().to_string(),
                        opportunities: arbs.clone(),
                    });
                    let _ = state.arb_tx.send(msg);

                    // Log to Supabase (best-effort, only log NEW arbs not in prev cycle)
                    for arb in &arbs {
                        let key = ArbKey {
                            event_id: arb.canonical_event_id.clone(),
                            outcome: arb.outcome.clone(),
                            buy_platform: arb.buy_platform.clone(),
                        };
                        if !prev_arb_keys.contains(&key) {
                            if let Err(e) = state.db.insert_arb_opportunity(arb).await {
                                warn!("Failed to log arb to DB: {}", e);
                            }
                        }
                    }
                } else {
                    // Clear any stale arbs for this event
                    state.cache.set_active_arbs(&event.id, vec![]);
                }
            }
        }

        // === Duration tracking: close arb windows that disappeared ===
        for closed_key in prev_arb_keys.difference(&current_arb_keys) {
            if let Err(e) = state
                .db
                .close_arb_window(
                    &closed_key.event_id,
                    &closed_key.outcome,
                    &closed_key.buy_platform,
                )
                .await
            {
                warn!("Failed to close arb window: {}", e);
            }
        }

        prev_arb_keys = current_arb_keys;

        if total_arbs > 0 {
            info!("🚨 Found {} active arbitrage opportunities!", total_arbs);
        }

        tokio::time::sleep(interval).await;
    }
}

/// Detect arbitrage opportunities for a single event.
fn detect_arbitrage(
    event_id: &str,
    event_title: &str,
    sport: &str,
    odds: &EventOdds,
    state: &AppState,
) -> Vec<ArbitrageOpportunity> {
    let mut opportunities = Vec::new();

    let kalshi_odds = match odds.platform_odds.get("kalshi") {
        Some(o) => o,
        None => return opportunities,
    };
    let poly_odds = match odds.platform_odds.get("polymarket") {
        Some(o) => o,
        None => return opportunities,
    };

    for (outcome, kalshi_price) in kalshi_odds {
        if let Some(poly_price) = poly_odds.get(outcome) {
            // === Strategy 1: Buy YES on Kalshi, Buy NO on Polymarket ===
            let cost_1 = kalshi_price.yes_price + poly_price.no_price;
            if cost_1 < 1.0 {
                let margin = (1.0 - cost_1) * 100.0;
                if margin >= state.config.arb_min_margin_pct {
                    let score =
                        compute_score(margin, kalshi_price.bid_size, poly_price.ask_size);
                    opportunities.push(ArbitrageOpportunity {
                        canonical_event_id: event_id.to_string(),
                        event_title: event_title.to_string(),
                        sport: sport.to_string(),
                        outcome: outcome.clone(),
                        buy_platform: "kalshi".to_string(),
                        buy_price: kalshi_price.yes_price,
                        sell_platform: "polymarket".to_string(),
                        sell_price: poly_price.yes_price,
                        margin_pct: margin,
                        max_executable_size: None,
                        estimated_fees: None,
                        estimated_net_profit: None,
                        score,
                        detected_at: Utc::now(),
                    });
                }
            }

            // === Strategy 2: Buy YES on Polymarket, Buy NO on Kalshi ===
            let cost_2 = poly_price.yes_price + kalshi_price.no_price;
            if cost_2 < 1.0 {
                let margin = (1.0 - cost_2) * 100.0;
                if margin >= state.config.arb_min_margin_pct {
                    let score =
                        compute_score(margin, poly_price.bid_size, kalshi_price.ask_size);
                    opportunities.push(ArbitrageOpportunity {
                        canonical_event_id: event_id.to_string(),
                        event_title: event_title.to_string(),
                        sport: sport.to_string(),
                        outcome: outcome.clone(),
                        buy_platform: "polymarket".to_string(),
                        buy_price: poly_price.yes_price,
                        sell_platform: "kalshi".to_string(),
                        sell_price: kalshi_price.yes_price,
                        margin_pct: margin,
                        max_executable_size: None,
                        estimated_fees: None,
                        estimated_net_profit: None,
                        score,
                        detected_at: Utc::now(),
                    });
                }
            }
        }
    }

    opportunities.sort_by(|a, b| b.margin_pct.partial_cmp(&a.margin_pct).unwrap());
    opportunities
}

/// Compute a 0-100 score for an arbitrage opportunity.
fn compute_score(margin_pct: f64, buy_size: Option<f64>, sell_size: Option<f64>) -> i32 {
    let margin_score = (margin_pct.min(10.0) / 10.0 * 40.0) as i32;

    let liquidity_score = match (buy_size, sell_size) {
        (Some(b), Some(s)) => {
            let min_size = b.min(s);
            (min_size.min(10000.0) / 10000.0 * 25.0) as i32
        }
        _ => 12,
    };

    let freshness_score = 10;
    let base_score = 25;

    (margin_score + liquidity_score + freshness_score + base_score).min(100)
}

