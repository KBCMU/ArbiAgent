use std::sync::Arc;

use chrono::Utc;
use tracing::info;

use crate::models::event::is_event_past;
use crate::models::vegas::EvOpportunity;
use crate::AppState;

const MIN_EV_EDGE_PCT: f64 = 2.0;
const DETECTION_INTERVAL_SECS: u64 = 3;

/// Background loop: scans all events for +EV opportunities
/// by comparing vegas fair probabilities to prediction market prices.
pub async fn run_ev_detection_loop(state: Arc<AppState>) {
    let interval = tokio::time::Duration::from_secs(DETECTION_INTERVAL_SECS);

    // Wait for both vegas odds and prediction market odds to populate
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    info!("📈 +EV detector started (interval: {}s, min_edge: {}%)", DETECTION_INTERVAL_SECS, MIN_EV_EDGE_PCT);

    loop {
        let mut total_ev = 0usize;

        for entry in state.cache.events.iter() {
            let event = entry.value();

            if is_event_past(&event.id) {
                state.cache.set_active_ev(&event.id, vec![]);
                continue;
            }

            let vegas = match state.cache.vegas_odds.get(&event.id) {
                Some(v) => v.value().clone(),
                None => continue,
            };

            let market_odds = match state.cache.odds.get(&event.id) {
                Some(o) => o.value().clone(),
                None => continue,
            };

            let mut opportunities = Vec::new();

            for (outcome, vegas_outcome) in &vegas.outcomes {
                let fair_prob = vegas_outcome.fair_prob;
                if fair_prob <= 0.0 || fair_prob >= 1.0 {
                    continue;
                }

                for platform in &["kalshi", "polymarket"] {
                    let platform_odds = match market_odds.platform_odds.get(*platform) {
                        Some(p) => p,
                        None => continue,
                    };

                    let market_price = match platform_odds.get(outcome) {
                        Some(p) => p.yes_price,
                        None => continue,
                    };

                    if market_price <= 0.0 || market_price >= 1.0 {
                        continue;
                    }

                    let edge = fair_prob - market_price;
                    let edge_pct = edge * 100.0;

                    if edge_pct >= MIN_EV_EDGE_PCT {
                        // Kelly criterion: f = (bp - q) / b
                        // where b = (1/market_price - 1), p = fair_prob, q = 1 - fair_prob
                        let b = (1.0 / market_price) - 1.0;
                        let kelly = if b > 0.0 {
                            let f = (b * fair_prob - (1.0 - fair_prob)) / b;
                            Some(f.max(0.0).min(1.0))
                        } else {
                            None
                        };

                        opportunities.push(EvOpportunity {
                            canonical_event_id: event.id.clone(),
                            event_title: event.event_title.clone(),
                            sport: event.sport.as_str().to_string(),
                            outcome: outcome.clone(),
                            market_platform: platform.to_string(),
                            market_price,
                            vegas_fair_prob: fair_prob,
                            edge_pct,
                            consensus_moneyline: vegas_outcome.consensus_moneyline,
                            kelly_fraction: kelly,
                            detected_at: Utc::now(),
                        });
                    }
                }
            }

            opportunities.sort_by(|a, b| b.edge_pct.partial_cmp(&a.edge_pct).unwrap_or(std::cmp::Ordering::Equal));

            if !opportunities.is_empty() {
                total_ev += opportunities.len();
            }

            state.cache.set_active_ev(&event.id, opportunities);
        }

        if total_ev > 0 {
            info!("📈 Found {} active +EV opportunities!", total_ev);
        }

        tokio::time::sleep(interval).await;
    }
}
