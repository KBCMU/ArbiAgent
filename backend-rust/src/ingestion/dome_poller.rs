use std::sync::Arc;

use chrono::Utc;
use reqwest::Client;
use tracing::{error, info, warn};

use crate::models::event::{CanonicalEvent, OutcomePrice, PlatformIds};
use crate::models::platform::{
    DomePlatformMatch, DomeSportsMatchResponse, KalshiPriceResponse, PolymarketPriceResponse,
};
use crate::models::ws::{PlatformPriceData, PriceUpdateData, WsMessage};
use crate::AppState;

/// Builds an authenticated HTTP client for DomeAPI.
fn build_dome_client(api_key: &str) -> Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {}", api_key).parse().unwrap(),
    );
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client")
}

/// Background loop: discovers matched sports events via DomeAPI every N seconds.
pub async fn run_event_discovery_loop(state: Arc<AppState>) {
    let client = build_dome_client(&state.config.dome_api_key);
    let interval =
        tokio::time::Duration::from_secs(state.config.event_discovery_interval_secs);

    // Run immediately on startup, then on interval
    loop {
        match discover_all_sports_events(&client, &state).await {
            Ok(count) => {
                info!(
                    "🔍 Discovered {} matched events across all sports ({} total in cache)",
                    count,
                    state.cache.event_count()
                );
            }
            Err(e) => {
                error!("❌ Event discovery failed: {}", e);
            }
        }

        tokio::time::sleep(interval).await;
    }
}

/// Discovers matched events for all sports for today and tomorrow.
async fn discover_all_sports_events(
    client: &Client,
    state: &AppState,
) -> anyhow::Result<usize> {
    use crate::models::event::Sport;

    let today = Utc::now().format("%Y-%m-%d").to_string();
    let tomorrow = (Utc::now() + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    let mut total_new = 0usize;

    for sport in Sport::all() {
        // Fetch for today
        match fetch_matched_events(client, &state.config.dome_api_base_url, sport, &today).await {
            Ok(events) => {
                for event in events {
                    state.cache.upsert_event(event.clone());
                    // Best-effort write to DB (don't fail if DB isn't configured)
                    if let Err(e) = state.db.upsert_event(&event).await {
                        warn!("DB write skipped for {}: {}", event.id, e);
                    }
                    total_new += 1;
                }
            }
            Err(e) => {
                // Some sports may have no events today - that's fine
                warn!("No {} events for {}: {}", sport.as_str(), today, e);
            }
        }

        // Fetch for tomorrow
        match fetch_matched_events(client, &state.config.dome_api_base_url, sport, &tomorrow).await
        {
            Ok(events) => {
                for event in events {
                    state.cache.upsert_event(event.clone());
                    if let Err(e) = state.db.upsert_event(&event).await {
                        warn!("DB write skipped for {}: {}", event.id, e);
                    }
                    total_new += 1;
                }
            }
            Err(e) => {
                warn!("No {} events for {}: {}", sport.as_str(), tomorrow, e);
            }
        }
    }

    Ok(total_new)
}

/// Fetches matched events for a specific sport and date from DomeAPI.
async fn fetch_matched_events(
    client: &Client,
    base_url: &str,
    sport: &crate::models::event::Sport,
    date: &str,
) -> anyhow::Result<Vec<CanonicalEvent>> {
    let url = format!(
        "{}/matching-markets/sports/{}?date={}",
        base_url,
        sport.as_str(),
        date
    );

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("DomeAPI returned {}: {}", status, body);
    }

    let data: DomeSportsMatchResponse = response.json().await?;
    let now = Utc::now();

    let mut events = Vec::new();

    for (event_key, platform_matches) in data.markets {
        let mut kalshi_event_ticker = None;
        let mut kalshi_market_tickers = Vec::new();
        let mut polymarket_market_slug = None;
        let mut polymarket_token_ids = Vec::new();

        for pm in &platform_matches {
            match pm {
                DomePlatformMatch::Kalshi {
                    event_ticker,
                    market_tickers,
                } => {
                    kalshi_event_ticker = Some(event_ticker.clone());
                    kalshi_market_tickers = market_tickers.clone();
                }
                DomePlatformMatch::Polymarket {
                    market_slug,
                    token_ids,
                } => {
                    polymarket_market_slug = Some(market_slug.clone());
                    polymarket_token_ids = token_ids.clone();
                }
            }
        }

        // Build a human-readable title from the event key
        // e.g. "nfl-ari-den-2025-08-16" -> "NFL: ARI vs DEN (2025-08-16)"
        let title = build_title_from_key(&event_key, sport);

        let event = CanonicalEvent {
            id: event_key.clone(),
            sport: *sport,
            event_title: title,
            game_start_time: None, // DomeAPI doesn't provide exact start time in this endpoint
            status: "open".to_string(),
            platform_ids: PlatformIds {
                kalshi_event_ticker,
                kalshi_market_tickers,
                polymarket_market_slug,
                polymarket_token_ids,
            },
            created_at: now,
            updated_at: now,
        };

        events.push(event);
    }

    Ok(events)
}

/// Build a readable title from a DomeAPI event key.
fn build_title_from_key(key: &str, sport: &crate::models::event::Sport) -> String {
    // Key format: "nfl-ari-den-2025-08-16"
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() >= 4 {
        let sport_str = sport.as_str().to_uppercase();
        let team_a = parts[1].to_uppercase();
        let team_b = parts[2].to_uppercase();
        // Remaining parts are the date
        let date_parts: Vec<&str> = parts[3..].iter().copied().collect();
        let date = date_parts.join("-");
        format!("{}: {} vs {} ({})", sport_str, team_a, team_b, date)
    } else {
        key.to_string()
    }
}

// ──────────────────────────────────────────────────────────────────────
// Price Refresh (REST-based, Phase 1 — will be replaced by WebSockets)
// ──────────────────────────────────────────────────────────────────────

/// Background loop: refreshes prices for all matched events via DomeAPI REST.
pub async fn run_price_refresh_loop(state: Arc<AppState>) {
    let client = build_dome_client(&state.config.dome_api_key);
    let interval =
        tokio::time::Duration::from_secs(state.config.price_refresh_interval_secs);

    // Wait a bit for events to be discovered first
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    loop {
        let events: Vec<CanonicalEvent> = state
            .cache
            .events
            .iter()
            .map(|e| e.value().clone())
            .collect();

        if events.is_empty() {
            tokio::time::sleep(interval).await;
            continue;
        }

        info!("💰 Refreshing prices for {} events...", events.len());

        let mut price_updates = 0u32;
        let mut errors = 0u32;

        for event in &events {
            // Fetch Kalshi prices
            for ticker in &event.platform_ids.kalshi_market_tickers {
                match fetch_kalshi_price(&client, &state.config.dome_api_base_url, ticker).await {
                    Ok(price) => {
                        // Extract outcome name from ticker
                        // e.g. "KXNFLGAME-25AUG16ARIDEN-ARI" -> "ARI"
                        let outcome = extract_kalshi_outcome(ticker);
                        state.cache.update_odds(
                            &event.id,
                            "kalshi",
                            &outcome,
                            OutcomePrice {
                                yes_price: price.yes.price,
                                no_price: price.no.price,
                                best_bid: None,
                                best_ask: None,
                                bid_size: None,
                                ask_size: None,
                            },
                        );
                        price_updates += 1;
                    }
                    Err(e) => {
                        warn!("Failed to fetch Kalshi price for {}: {}", ticker, e);
                        errors += 1;
                    }
                }
            }

            // Fetch Polymarket prices
            for (i, token_id) in event.platform_ids.polymarket_token_ids.iter().enumerate() {
                match fetch_polymarket_price(&client, &state.config.dome_api_base_url, token_id)
                    .await
                {
                    Ok(price_resp) => {
                        // Token IDs correspond to outcomes in order
                        // For a 2-outcome market: token_ids[0] = team A, token_ids[1] = team B
                        let outcome = extract_polymarket_outcome(event, i);
                        state.cache.update_odds(
                            &event.id,
                            "polymarket",
                            &outcome,
                            OutcomePrice {
                                yes_price: price_resp.price,
                                no_price: 1.0 - price_resp.price,
                                best_bid: None,
                                best_ask: None,
                                bid_size: None,
                                ask_size: None,
                            },
                        );
                        price_updates += 1;
                    }
                    Err(e) => {
                        warn!("Failed to fetch Polymarket price for token {}: {}", token_id, e);
                        errors += 1;
                    }
                }
            }

            // Broadcast price update to WebSocket clients
            let odds = state.cache.odds.get(&event.id);
            if let Some(odds) = odds {
                let msg = WsMessage::PriceUpdate(PriceUpdateData {
                    canonical_event_id: event.id.clone(),
                    kalshi: odds.platform_odds.get("kalshi").map(|outcomes| {
                        PlatformPriceData {
                            outcomes: outcomes
                                .iter()
                                .map(|(k, v)| (k.clone(), v.yes_price))
                                .collect(),
                            updated_at: odds.updated_at.to_rfc3339(),
                        }
                    }),
                    polymarket: odds.platform_odds.get("polymarket").map(|outcomes| {
                        PlatformPriceData {
                            outcomes: outcomes
                                .iter()
                                .map(|(k, v)| (k.clone(), v.yes_price))
                                .collect(),
                            updated_at: odds.updated_at.to_rfc3339(),
                        }
                    }),
                });
                // Ignore send errors (no subscribers)
                let _ = state.arb_tx.send(msg);
            }
        }

        info!(
            "💰 Price refresh complete: {} updates, {} errors",
            price_updates, errors
        );

        tokio::time::sleep(interval).await;
    }
}

/// Fetch Kalshi market price via DomeAPI REST.
async fn fetch_kalshi_price(
    client: &Client,
    base_url: &str,
    ticker: &str,
) -> anyhow::Result<KalshiPriceResponse> {
    let url = format!("{}/kalshi/market-price/{}", base_url, ticker);
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        anyhow::bail!("Kalshi price API returned {}", resp.status());
    }

    Ok(resp.json().await?)
}

/// Fetch Polymarket market price via DomeAPI REST.
async fn fetch_polymarket_price(
    client: &Client,
    base_url: &str,
    token_id: &str,
) -> anyhow::Result<PolymarketPriceResponse> {
    let url = format!("{}/polymarket/market-price/{}", base_url, token_id);
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        anyhow::bail!("Polymarket price API returned {}", resp.status());
    }

    Ok(resp.json().await?)
}

/// Extract the outcome name from a Kalshi market ticker.
/// e.g. "KXNFLGAME-25AUG16ARIDEN-ARI" -> "ARI"
fn extract_kalshi_outcome(ticker: &str) -> String {
    ticker
        .rsplit('-')
        .next()
        .unwrap_or("UNKNOWN")
        .to_string()
}

/// Public version of extract_kalshi_outcome for cross-module use.
pub fn extract_kalshi_outcome_pub(ticker: &str) -> String {
    extract_kalshi_outcome(ticker)
}

/// Extract outcome name for a Polymarket token by index.
/// Uses Kalshi market tickers as a reference (they contain team names).
fn extract_polymarket_outcome(event: &CanonicalEvent, token_index: usize) -> String {
    // Polymarket token IDs correspond 1:1 with outcomes
    // We use the Kalshi market tickers to determine outcome names
    if let Some(kalshi_ticker) = event.platform_ids.kalshi_market_tickers.get(token_index) {
        extract_kalshi_outcome(kalshi_ticker)
    } else {
        format!("OUTCOME_{}", token_index)
    }
}
