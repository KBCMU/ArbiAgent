//! PredictionAPI ingestion — polls https://prediction-api-kbcmu.fly.dev for
//! pre-matched events and live odds from both Kalshi and Polymarket.
//!
//! Replaces the native matching engine, culture poller, direct price API,
//! and WebSocket ingesters when `enable_prediction_api = true`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Deserializer};
use tracing::{error, info, warn};

/// PredictionAPI returns prices as JSON strings (e.g. `"0.3300"`) rather than
/// numbers. Accept either form so the struct stays robust to upstream changes.
fn deserialize_price<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrFloat {
        String(String),
        Float(f64),
    }
    match StringOrFloat::deserialize(deserializer)? {
        StringOrFloat::Float(f) => Ok(f),
        StringOrFloat::String(s) => s.parse::<f64>().map_err(D::Error::custom),
    }
}

use crate::models::event::{CanonicalEvent, OutcomePrice, PlatformIds, Sport};
use crate::AppState;

// ─── Response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    data: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct PredictionEvent {
    id: String,
    sport: String,
    participant_a: String,
    participant_b: String,
    start_time: String,
    #[serde(default)]
    is_live: bool,
}

#[derive(Debug, Deserialize)]
struct PredictionMarket {
    event_id: String,
    #[allow(dead_code)]
    market_type: String,
    /// Keyed by exchange name: "kalshi", "polymarket"
    contracts: HashMap<String, PlatformContract>,
}

#[derive(Debug, Deserialize)]
struct PlatformContract {
    #[serde(deserialize_with = "deserialize_price")]
    outcome_a_price: f64,
    #[serde(deserialize_with = "deserialize_price")]
    outcome_b_price: f64,
    /// ISO 8601 timestamp of when this contract's price was last fetched upstream.
    last_fetched: Option<String>,
}

// ─── HTTP helpers ────────────────────────────────────────────────────

fn build_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("failed to build PredictionAPI HTTP client")
}

async fn fetch_events(
    client: &Client,
    base_url: &str,
    token: &str,
) -> anyhow::Result<Vec<PredictionEvent>> {
    let url = format!("{}/events", base_url);
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?
        .error_for_status()?
        .json::<ApiResponse<PredictionEvent>>()
        .await?;
    Ok(resp.data)
}

async fn fetch_markets(
    client: &Client,
    base_url: &str,
    token: &str,
) -> anyhow::Result<Vec<PredictionMarket>> {
    let url = format!("{}/markets", base_url);
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?
        .error_for_status()?
        .json::<ApiResponse<PredictionMarket>>()
        .await?;
    Ok(resp.data)
}

// ─── Staleness check ─────────────────────────────────────────────────

fn is_stale(last_fetched: &Option<String>, threshold_secs: i64) -> bool {
    let Some(ts) = last_fetched else { return false };
    let Ok(dt) = DateTime::parse_from_rfc3339(ts) else { return false };
    let age = Utc::now().signed_duration_since(dt.with_timezone(&Utc));
    age.num_seconds() > threshold_secs
}

// ─── Event discovery loop ─────────────────────────────────────────────

/// Polls `/events`, maps each into a `CanonicalEvent`, and upserts into the cache.
///
/// Runs on `event_discovery_interval_secs` (default 60s). Uses the PredictionAPI's
/// event ID directly — `is_event_past()` treats unparseable IDs as not-past (safe).
pub async fn run_prediction_event_loop(state: Arc<AppState>) {
    let client = build_client();
    let interval = Duration::from_secs(state.config.event_discovery_interval_secs);
    info!(
        "📡 PredictionAPI event loop started (interval: {}s)",
        state.config.event_discovery_interval_secs
    );

    loop {
        match fetch_events(
            &client,
            &state.config.prediction_api_url,
            &state.config.prediction_api_token,
        )
        .await
        {
            Ok(events) => {
                let count = events.len();
                for event in events {
                    let sport = Sport::from_str_loose(&event.sport);
                    let game_start_time = DateTime::parse_from_rfc3339(&event.start_time)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc));
                    let status = if event.is_live { "live" } else { "open" }.to_string();
                    let now = Utc::now();
                    let canonical = CanonicalEvent {
                        id: event.id.clone(),
                        sport,
                        event_title: format!(
                            "{} vs {}",
                            event.participant_a, event.participant_b
                        ),
                        game_start_time,
                        status,
                        platform_ids: PlatformIds {
                            kalshi_event_ticker: None,
                            kalshi_market_tickers: vec![],
                            polymarket_market_slug: None,
                            polymarket_token_ids: vec![],
                            // Participant names become outcome labels — used by routes
                            // to pass the `polymarket_outcome_labels` filter and by the
                            // arb detector to align Kalshi/Polymarket outcomes.
                            polymarket_outcome_labels: vec![
                                event.participant_a.clone(),
                                event.participant_b.clone(),
                            ],
                        },
                        match_score: Some(100.0),
                        created_at: now,
                        updated_at: now,
                    };
                    state.cache.upsert_event(canonical);
                }
                info!("📡 PredictionAPI: discovered {} events", count);
            }
            Err(e) => error!("PredictionAPI event fetch failed: {}", e),
        }

        tokio::time::sleep(interval).await;
    }
}

// ─── Price refresh loop ───────────────────────────────────────────────

/// Polls `/markets` and `/events` (for participant names), then writes odds
/// into the cache keyed by participant name.
///
/// The arb detector checks `platform_odds["kalshi"][participant_a].yes_price`
/// against `platform_odds["polymarket"][participant_a].yes_price` — using
/// participant names as keys ensures cross-platform alignment without
/// additional label-resolution logic.
pub async fn run_prediction_price_loop(state: Arc<AppState>) {
    let client = build_client();
    let interval = Duration::from_secs(state.config.price_refresh_interval_secs);

    // Brief warm-up so the event loop has time to populate the cache first.
    tokio::time::sleep(Duration::from_secs(5)).await;
    info!(
        "📡 PredictionAPI price loop started (interval: {}s)",
        state.config.price_refresh_interval_secs
    );

    loop {
        // Fetch events to resolve participant names used as outcome keys.
        let events_map: HashMap<String, (String, String)> = match fetch_events(
            &client,
            &state.config.prediction_api_url,
            &state.config.prediction_api_token,
        )
        .await
        {
            Ok(events) => events
                .into_iter()
                .map(|e| (e.id, (e.participant_a, e.participant_b)))
                .collect(),
            Err(e) => {
                error!("PredictionAPI events fetch failed in price loop: {}", e);
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        match fetch_markets(
            &client,
            &state.config.prediction_api_url,
            &state.config.prediction_api_token,
        )
        .await
        {
            Ok(markets) => {
                let stale_threshold = state.config.prediction_api_stale_threshold_secs;
                let markets_total = markets.len();
                let mut updated = 0usize;
                let mut skipped_stale = 0usize;
                let mut skipped_missing_contracts = 0usize;
                let mut skipped_unknown_event = 0usize;

                for market in markets {
                    let Some((participant_a, participant_b)) =
                        events_map.get(&market.event_id)
                    else {
                        skipped_unknown_event += 1;
                        continue;
                    };
                    let Some(kalshi) = market.contracts.get("kalshi") else {
                        skipped_missing_contracts += 1;
                        continue;
                    };
                    let Some(poly) = market.contracts.get("polymarket") else {
                        skipped_missing_contracts += 1;
                        continue;
                    };

                    if is_stale(&kalshi.last_fetched, stale_threshold)
                        || is_stale(&poly.last_fetched, stale_threshold)
                    {
                        skipped_stale += 1;
                        continue;
                    }

                    let event_id = &market.event_id;

                    // participant_a outcome: outcome_a_price is the YES price for A winning.
                    // outcome_b_price is the complementary NO price (A losing).
                    state.cache.update_odds(
                        event_id,
                        "kalshi",
                        participant_a,
                        OutcomePrice {
                            yes_price: kalshi.outcome_a_price,
                            no_price: kalshi.outcome_b_price,
                            best_bid: None,
                            best_ask: None,
                            bid_size: None,
                            ask_size: None,
                        },
                    );
                    state.cache.update_odds(
                        event_id,
                        "polymarket",
                        participant_a,
                        OutcomePrice {
                            yes_price: poly.outcome_a_price,
                            no_price: poly.outcome_b_price,
                            best_bid: None,
                            best_ask: None,
                            bid_size: None,
                            ask_size: None,
                        },
                    );

                    // participant_b outcome: prices are flipped — outcome_b_price is YES for B winning.
                    state.cache.update_odds(
                        event_id,
                        "kalshi",
                        participant_b,
                        OutcomePrice {
                            yes_price: kalshi.outcome_b_price,
                            no_price: kalshi.outcome_a_price,
                            best_bid: None,
                            best_ask: None,
                            bid_size: None,
                            ask_size: None,
                        },
                    );
                    state.cache.update_odds(
                        event_id,
                        "polymarket",
                        participant_b,
                        OutcomePrice {
                            yes_price: poly.outcome_b_price,
                            no_price: poly.outcome_a_price,
                            best_bid: None,
                            best_ask: None,
                            bid_size: None,
                            ask_size: None,
                        },
                    );

                    updated += 1;
                }
                if updated > 0 {
                    info!("📡 PredictionAPI: updated odds for {} markets", updated);
                } else if markets_total > 0 {
                    warn!(
                        "PredictionAPI: 0/{} markets got odds (stale={}, no_contracts={}, unknown_event={})",
                        markets_total,
                        skipped_stale,
                        skipped_missing_contracts,
                        skipped_unknown_event,
                    );
                }
            }
            Err(e) => error!("PredictionAPI markets fetch failed: {}", e),
        }

        tokio::time::sleep(interval).await;
    }
}
