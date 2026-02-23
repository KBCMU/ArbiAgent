use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::Utc;
use tracing::{info, warn};

use crate::models::arb::ArbitrageOpportunity;
use crate::models::event::{is_event_past, is_event_settled_full, EventOdds};
use crate::models::ws::{ArbUpdateData, WsMessage};
use crate::storage::cache::StateCache;
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
            // Skip events whose game date has passed — no trades possible
            if is_event_past(&event.id) {
                // Clean up any stale arbs still in cache for this event
                state.cache.set_active_arbs(&event.id, vec![]);
                continue;
            }

            if let Some(odds) = odds_opt {
                // Skip events whose markets have settled (Kalshi status or price heuristic)
                if is_event_settled_full(&event.status, odds) {
                    state.cache.set_active_arbs(&event.id, vec![]);
                    continue;
                }

                // Validate and correct Polymarket outcome labels before arb detection.
                // This compares YES prices across platforms and swaps labels if they
                // are inverted — fixing both arb detection and frontend display.
                validate_and_correct_odds_mapping(&event.id, &state.cache);

                // Re-read odds from cache (correction may have swapped Poly keys)
                let corrected_odds = match state.cache.odds.get(&event.id) {
                    Some(o) => o.value().clone(),
                    None => continue,
                };

                let mut arbs = detect_arbitrage(
                    &event.id,
                    &event.event_title,
                    event.sport.as_str(),
                    &corrected_odds,
                    &state,
                );

                // In a 2-outcome market, buying YES on outcome A is the same
                // trade as buying NO on outcome B.  The detector sees both,
                // producing mirror-image duplicates.  Keep only the highest-
                // margin arb per unique platform-pair direction on this event.
                dedup_complementary_arbs(&mut arbs);

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


    let kalshi_fee_rate = state.config.kalshi_fee_pct / 100.0;
    let poly_fee_rate = state.config.polymarket_fee_pct / 100.0;

    for (outcome, kalshi_price) in kalshi_odds {
        if let Some(poly_price) = poly_odds.get(outcome) {
            // === Strategy 1: Buy YES on Kalshi, Buy NO on Polymarket ===
            // Cost = kalshi_yes * (1 + kalshi_fee) + poly_no * (1 + poly_fee)
            let kalshi_cost = kalshi_price.yes_price * (1.0 + kalshi_fee_rate);
            let poly_cost = poly_price.no_price * (1.0 + poly_fee_rate);
            let total_cost_1 = kalshi_cost + poly_cost;
            let raw_cost_1 = kalshi_price.yes_price + poly_price.no_price;

            if total_cost_1 < 1.0 {
                let net_margin = (1.0 - total_cost_1) * 100.0;
                let _gross_margin = (1.0 - raw_cost_1) * 100.0;
                let fees = (total_cost_1 - raw_cost_1) * 100.0;

                if net_margin >= state.config.arb_min_margin_pct {
                    let score =
                        compute_score(net_margin, kalshi_price.bid_size, poly_price.ask_size);
                    opportunities.push(ArbitrageOpportunity {
                        canonical_event_id: event_id.to_string(),
                        event_title: event_title.to_string(),
                        sport: sport.to_string(),
                        outcome: outcome.clone(),
                        buy_platform: "kalshi".to_string(),
                        buy_price: kalshi_price.yes_price,
                        sell_platform: "polymarket".to_string(),
                        sell_price: poly_price.yes_price,
                        margin_pct: net_margin,
                        max_executable_size: None,
                        estimated_fees: Some(fees),
                        estimated_net_profit: Some(net_margin),
                        score,
                        detected_at: Utc::now(),
                    });
                }
            }

            // === Strategy 2: Buy YES on Polymarket, Buy NO on Kalshi ===
            let poly_cost_2 = poly_price.yes_price * (1.0 + poly_fee_rate);
            let kalshi_cost_2 = kalshi_price.no_price * (1.0 + kalshi_fee_rate);
            let total_cost_2 = poly_cost_2 + kalshi_cost_2;
            let raw_cost_2 = poly_price.yes_price + kalshi_price.no_price;

            if total_cost_2 < 1.0 {
                let net_margin = (1.0 - total_cost_2) * 100.0;
                let _gross_margin = (1.0 - raw_cost_2) * 100.0;
                let fees = (total_cost_2 - raw_cost_2) * 100.0;

                if net_margin >= state.config.arb_min_margin_pct {
                    let score =
                        compute_score(net_margin, poly_price.bid_size, kalshi_price.ask_size);
                    opportunities.push(ArbitrageOpportunity {
                        canonical_event_id: event_id.to_string(),
                        event_title: event_title.to_string(),
                        sport: sport.to_string(),
                        outcome: outcome.clone(),
                        buy_platform: "polymarket".to_string(),
                        buy_price: poly_price.yes_price,
                        sell_platform: "kalshi".to_string(),
                        sell_price: kalshi_price.yes_price,
                        margin_pct: net_margin,
                        max_executable_size: None,
                        estimated_fees: Some(fees),
                        estimated_net_profit: Some(net_margin),
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

/// Deduplicate complementary arbs within a single event.
///
/// In a binary market (2 outcomes A and B), buying YES-A on Kalshi +
/// NO-A on Polymarket is the *same economic trade* as buying YES-B on
/// Polymarket + NO-B on Kalshi.  The detector finds both because it
/// iterates outcomes independently.
///
/// To remove the mirror image, we normalize each arb's platform pair
/// (always sorted alphabetically) and keep only the highest-margin
/// arb per normalized pair.
fn dedup_complementary_arbs(arbs: &mut Vec<ArbitrageOpportunity>) {
    if arbs.len() <= 1 {
        return;
    }

    // For each normalized platform pair, track the index of the best arb
    let mut best: HashMap<(String, String), usize> = HashMap::new();
    let mut keep = vec![false; arbs.len()];

    for (i, arb) in arbs.iter().enumerate() {
        // Normalize: always put the alphabetically-smaller platform first
        let key = if arb.buy_platform <= arb.sell_platform {
            (arb.buy_platform.clone(), arb.sell_platform.clone())
        } else {
            (arb.sell_platform.clone(), arb.buy_platform.clone())
        };

        match best.entry(key) {
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(i);
                keep[i] = true;
            }
            std::collections::hash_map::Entry::Occupied(mut e) => {
                let prev_idx = *e.get();
                if arb.margin_pct > arbs[prev_idx].margin_pct {
                    keep[prev_idx] = false;
                    keep[i] = true;
                    e.insert(i);
                }
            }
        }
    }

    let mut idx = 0;
    arbs.retain(|_| {
        let k = keep[idx];
        idx += 1;
        k
    });
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

/// Validate and correct Polymarket outcome-label alignment for binary markets.
///
/// For a 2-outcome event there are exactly two possible token→outcome mappings.
/// We score both by how closely YES prices agree across platforms (Kalshi is
/// the reference) and pick the one with the lower total gap.
///
/// When the "crossed" mapping is materially better (>4¢ total improvement),
/// we swap the Polymarket odds keys in the cache AND the persistent
/// `polymarket_outcome_labels` on the event  so every future price update
/// (WebSocket, direct batch, DomeAPI) lands under the correct outcome name.
///
/// This function is idempotent: if the mapping is already correct, the
/// straight gap will be lower and no swap occurs.
fn validate_and_correct_odds_mapping(event_id: &str, cache: &StateCache) {
    // ── Step 1: read current odds and decide ─────────────────────────
    let swap_info = {
        let entry = match cache.odds.get(event_id) {
            Some(e) => e,
            None => return,
        };
        let odds = entry.value();

        let kalshi = match odds.platform_odds.get("kalshi") {
            Some(k) if k.len() == 2 => k,
            _ => return,
        };
        let poly = match odds.platform_odds.get("polymarket") {
            Some(p) if p.len() == 2 => p,
            _ => return,
        };

        // Both platforms must share the same 2 outcome names
        let shared: Vec<String> = kalshi
            .keys()
            .filter(|k| poly.contains_key(k.as_str()))
            .cloned()
            .collect();
        if shared.len() != 2 {
            return;
        }

        let (a, b) = (&shared[0], &shared[1]);
        let k_a = kalshi[a].yes_price;
        let k_b = kalshi[b].yes_price;
        let p_a = poly[a].yes_price;
        let p_b = poly[b].yes_price;

        // Score both possible mappings
        let straight_gap = (k_a - p_a).abs() + (k_b - p_b).abs();
        let crossed_gap  = (k_a - p_b).abs() + (k_b - p_a).abs();

        // Swap only when crossed mapping is clearly better.
        // 4¢ minimum prevents noise-driven swaps (e.g. exact 50/50).
        if crossed_gap + 0.04 < straight_gap {
            Some((a.clone(), b.clone(), straight_gap, crossed_gap))
        } else {
            None
        }
    };

    // ── Step 2: perform the swap if needed ────────────────────────────
    if let Some((outcome_a, outcome_b, straight, crossed)) = swap_info {
        // 2a. Swap Polymarket odds keys in the cache
        if let Some(mut odds_entry) = cache.odds.get_mut(event_id) {
            if let Some(poly_map) = odds_entry.value_mut().platform_odds.get_mut("polymarket") {
                let pa = poly_map.get(&outcome_a).cloned();
                let pb = poly_map.get(&outcome_b).cloned();
                if let (Some(pa), Some(pb)) = (pa, pb) {
                    poly_map.insert(outcome_a.clone(), pb);
                    poly_map.insert(outcome_b.clone(), pa);
                }
            }
        }

        // 2b. Swap outcome labels on the event so future WS/REST price
        //     updates for these token IDs land under the correct name.
        if let Some(mut event_entry) = cache.events.get_mut(event_id) {
            let labels = &mut event_entry.value_mut().platform_ids.polymarket_outcome_labels;
            if labels.len() == 2 {
                labels.swap(0, 1);
            }
        }

        warn!(
            "🔄 Corrected Polymarket labels for {}: swapped {} ↔ {} (straight_gap={:.3}, crossed_gap={:.3})",
            event_id, outcome_a, outcome_b, straight, crossed
        );
    }
}

