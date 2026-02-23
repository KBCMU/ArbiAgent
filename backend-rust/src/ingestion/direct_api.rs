//! Direct API price fetching — bypasses DomeAPI for price data.
//!
//! Uses Kalshi's public `GET /markets?tickers=...` batch endpoint (20 reads/sec free tier)
//! and Polymarket's `POST /prices` batch endpoint (up to 500 tokens per request).
//!
//! This reduces price refresh from ~7 minutes (350+ sequential DomeAPI calls at 1 req/sec)
//! to ~3 seconds (2-3 batch requests in parallel).
//!
//! The old DomeAPI-based price loop (`dome_poller::run_price_refresh_loop`) is preserved
//! and can be used as a fallback by setting `ENABLE_DIRECT_PRICE_API=false`.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use reqwest::Client;
use serde::Serialize;
use tracing::{error, info, warn};

use crate::models::event::{CanonicalEvent, OutcomePrice};
use crate::models::platform::{KalshiDirectMarket, KalshiDirectMarketsResponse};
use crate::models::ws::{PlatformPriceData, PriceUpdateData, WsMessage};
use crate::AppState;

// ─── Constants ──────────────────────────────────────────────────────

const KALSHI_API_BASE: &str = "https://api.elections.kalshi.com/trade-api/v2";
const POLYMARKET_CLOB_BASE: &str = "https://clob.polymarket.com";

/// Maximum Kalshi tickers per batch request.
/// Each ticker is ~35 chars; at 150 tickers the URL stays safely under 8 KB.
const KALSHI_BATCH_SIZE: usize = 150;

/// Maximum Polymarket tokens per batch request (API hard limit is 500).
const POLYMARKET_BATCH_SIZE: usize = 450;

/// HTTP timeout for direct API calls (shorter than DomeAPI since these are faster).
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

// ─── Reverse Mapping ────────────────────────────────────────────────

/// Maps a platform-specific identifier (ticker / token ID) back to its
/// canonical event and outcome name so we can update the cache after a
/// batch response.
struct TickerMapping {
    event_id: String,
    outcome: String,
}

// ─── Public Entry Point ─────────────────────────────────────────────

/// Background loop: refreshes prices via direct Kalshi + Polymarket batch APIs.
///
/// Drop-in replacement for `dome_poller::run_price_refresh_loop`. Uses the same
/// `price_refresh_interval_secs` config and writes to the same `StateCache`.
///
/// When WebSocket ingesters are enabled (providing sub-second updates), this loop
/// automatically uses a longer interval (60s) to serve as a reconciliation
/// fallback rather than the primary price source.
pub async fn run_direct_price_refresh_loop(state: Arc<AppState>) {
    let client = build_public_client();

    let ws_active = state.config.enable_kalshi_ws || state.config.enable_polymarket_ws;
    let interval_secs = if ws_active {
        let reconciliation_secs = state.config.price_refresh_interval_secs.max(60);
        info!(
            "📡 WebSocket ingesters active — REST batch polling at {}s (reconciliation mode)",
            reconciliation_secs
        );
        reconciliation_secs
    } else {
        state.config.price_refresh_interval_secs
    };
    let interval = tokio::time::Duration::from_secs(interval_secs);

    // Wait for the DomeAPI event discovery loop to populate the cache.
    // Discovery makes ~24 rate-limited requests (~26s) — 35s gives headroom.
    tokio::time::sleep(tokio::time::Duration::from_secs(35)).await;
    info!("📡 Direct API price refresh loop started (interval: {}s)", interval_secs);

    loop {
        if let Err(e) = refresh_all_prices(&client, &state).await {
            error!("❌ Direct price refresh cycle failed: {}", e);
        }

        tokio::time::sleep(interval).await;
    }
}

// ─── Single Refresh Cycle ───────────────────────────────────────────

/// Execute one full price refresh: batch-fetch from both platforms, update cache,
/// broadcast to WebSocket clients.
async fn refresh_all_prices(client: &Client, state: &AppState) -> anyhow::Result<()> {
    let events: Vec<CanonicalEvent> = state
        .cache
        .events
        .iter()
        .map(|e| e.value().clone())
        .collect();

    if events.is_empty() {
        return Ok(());
    }

    // Build reverse maps: ticker/token → (event_id, outcome)
    let (kalshi_map, poly_map) = build_reverse_maps(&events);

    let total_ids = kalshi_map.len() + poly_map.len();
    info!(
        "📡 Direct API: refreshing {} events ({} identifiers)...",
        events.len(),
        total_ids
    );

    // Fetch from both platforms concurrently
    let (kalshi_updates, poly_updates) = tokio::join!(
        fetch_and_apply_kalshi(client, state, &kalshi_map),
        fetch_and_apply_polymarket(client, state, &poly_map),
    );

    // Broadcast price updates to WebSocket clients
    broadcast_price_updates(state, &events);

    info!(
        "📡 Direct API complete: {} Kalshi + {} Polymarket updates",
        kalshi_updates, poly_updates
    );

    Ok(())
}

/// Build reverse lookup maps from the cached events.
///
/// Kalshi: market_ticker → TickerMapping { event_id, outcome }
/// Polymarket: token_id   → TickerMapping { event_id, outcome }
fn build_reverse_maps(
    events: &[CanonicalEvent],
) -> (HashMap<String, TickerMapping>, HashMap<String, TickerMapping>) {
    let mut kalshi_map: HashMap<String, TickerMapping> = HashMap::new();
    let mut poly_map: HashMap<String, TickerMapping> = HashMap::new();

    for event in events {
        for ticker in &event.platform_ids.kalshi_market_tickers {
            let outcome =
                crate::ingestion::dome_poller::extract_kalshi_outcome_pub(ticker);
            kalshi_map.insert(
                ticker.clone(),
                TickerMapping {
                    event_id: event.id.clone(),
                    outcome,
                },
            );
        }

        for (i, token_id) in event.platform_ids.polymarket_token_ids.iter().enumerate() {
            let outcome = event
                .platform_ids
                .polymarket_outcome_labels
                .get(i)
                .filter(|l| !l.is_empty())
                .cloned()
                .unwrap_or_else(|| format!("OUTCOME_{}", i));
            poly_map.insert(
                token_id.clone(),
                TickerMapping {
                    event_id: event.id.clone(),
                    outcome,
                },
            );
        }
    }

    (kalshi_map, poly_map)
}

// ─── Kalshi Batch Fetch ─────────────────────────────────────────────

/// Fetch prices for all Kalshi tickers via `GET /markets?tickers=...`,
/// update cache, and return the count of successful updates.
async fn fetch_and_apply_kalshi(
    client: &Client,
    state: &AppState,
    ticker_map: &HashMap<String, TickerMapping>,
) -> u32 {
    let tickers: Vec<&str> = ticker_map.keys().map(|s| s.as_str()).collect();
    if tickers.is_empty() {
        return 0;
    }

    let mut updates = 0u32;

    for chunk in tickers.chunks(KALSHI_BATCH_SIZE) {
        let csv = chunk.join(",");
        let url = format!(
            "{}/markets?tickers={}&limit={}",
            KALSHI_API_BASE,
            csv,
            chunk.len()
        );

        let req_url = url.clone();
        let resp = match retry_request("Kalshi batch", || client.get(&req_url).send()).await {
            Ok(r) => r,
            Err(e) => {
                warn!("Kalshi batch request failed after retries: {}", e);
                continue;
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!(
                "Kalshi direct API returned {} (chunk of {} tickers): {}",
                status,
                chunk.len(),
                &body[..body.len().min(200)]
            );
            continue;
        }

        let data: KalshiDirectMarketsResponse = match resp.json().await {
            Ok(d) => d,
            Err(e) => {
                warn!("Failed to parse Kalshi batch response: {}", e);
                continue;
            }
        };

        for market in &data.markets {
            if let Some(mapping) = ticker_map.get(&market.ticker) {
                let price = kalshi_market_to_price(market);
                state
                    .cache
                    .update_odds(&mapping.event_id, "kalshi", &mapping.outcome, price);

                // Update canonical event status from Kalshi's authoritative lifecycle
                if let Some(ref status) = market.status {
                    apply_kalshi_status(state, &mapping.event_id, status);
                }

                updates += 1;
            }
        }
    }

    updates
}

/// Convert Kalshi FixedPointDollars fields into an `OutcomePrice`.
///
/// Uses bid/ask midpoint when both sides are available; falls back to
/// last traded price, then to whichever side exists.
fn kalshi_market_to_price(market: &KalshiDirectMarket) -> OutcomePrice {
    let yes_bid = parse_dollars(&market.yes_bid_dollars);
    let yes_ask = parse_dollars(&market.yes_ask_dollars);
    let no_bid = parse_dollars(&market.no_bid_dollars);
    let no_ask = parse_dollars(&market.no_ask_dollars);
    let last_price = parse_dollars(&market.last_price_dollars);

    // Best estimate for YES price: midpoint if spread available, else last trade
    let yes_price = match (yes_bid, yes_ask) {
        (Some(b), Some(a)) => (b + a) / 2.0,
        _ => last_price.or(yes_bid).or(yes_ask).unwrap_or(0.0),
    };

    let no_price = match (no_bid, no_ask) {
        (Some(b), Some(a)) => (b + a) / 2.0,
        _ => 1.0 - yes_price,
    };

    OutcomePrice {
        yes_price,
        no_price,
        best_bid: yes_bid,
        best_ask: yes_ask,
        bid_size: None,
        ask_size: None,
    }
}

/// Parse a FixedPointDollars string (e.g. "0.5600") to f64.
fn parse_dollars(val: &Option<String>) -> Option<f64> {
    val.as_ref().and_then(|s| s.parse::<f64>().ok())
}

/// Map Kalshi's market lifecycle status to our canonical event status.
///
/// Only promotes an event away from "open" — never downgrades. This prevents
/// a single still-active outcome market from reverting a settled event.
///
/// Kalshi lifecycle: initialized → active → closed → determined → finalized
fn apply_kalshi_status(state: &AppState, event_id: &str, kalshi_status: &str) {
    let mapped = match kalshi_status {
        "determined" | "finalized" => "settled",
        "closed" => "closed",
        _ => return, // "active", "initialized", etc. — don't overwrite
    };

    if let Some(mut event) = state.cache.events.get_mut(event_id) {
        // Only escalate status, never demote (e.g. settled → open)
        if event.status == "open" {
            event.status = mapped.to_string();
            event.updated_at = Utc::now();
        }
    }
}

// ─── Polymarket Batch Fetch ─────────────────────────────────────────

/// Request body item for Polymarket `POST /prices`.
#[derive(Serialize)]
struct PolyPriceRequest {
    token_id: String,
    side: String,
}

/// Fetch prices for all Polymarket tokens via `POST /prices`,
/// update cache, and return the count of successful updates.
async fn fetch_and_apply_polymarket(
    client: &Client,
    state: &AppState,
    token_map: &HashMap<String, TickerMapping>,
) -> u32 {
    let token_ids: Vec<&str> = token_map.keys().map(|s| s.as_str()).collect();
    if token_ids.is_empty() {
        return 0;
    }

    let mut updates = 0u32;

    for chunk in token_ids.chunks(POLYMARKET_BATCH_SIZE) {
        let body: Vec<PolyPriceRequest> = chunk
            .iter()
            .map(|tid| PolyPriceRequest {
                token_id: tid.to_string(),
                side: "BUY".to_string(),
            })
            .collect();

        let url = format!("{}/prices", POLYMARKET_CLOB_BASE);

        let resp = match retry_request("Polymarket batch", || {
            client.post(&url).json(&body).send()
        })
        .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("Polymarket batch request failed after retries: {}", e);
                continue;
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            warn!(
                "Polymarket batch API returned {} (chunk of {} tokens): {}",
                status,
                chunk.len(),
                &body_text[..body_text.len().min(200)]
            );
            continue;
        }

        // Response shape: { "token_id": { "BUY": "0.65" }, ... }
        let prices: HashMap<String, HashMap<String, String>> = match resp.json().await {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to parse Polymarket batch response: {}", e);
                continue;
            }
        };

        for (token_id, side_prices) in &prices {
            if let Some(mapping) = token_map.get(token_id.as_str()) {
                if let Some(buy_str) = side_prices.get("BUY") {
                    if let Ok(yes_price) = buy_str.parse::<f64>() {
                        state.cache.update_odds(
                            &mapping.event_id,
                            "polymarket",
                            &mapping.outcome,
                            OutcomePrice {
                                yes_price,
                                no_price: 1.0 - yes_price,
                                best_bid: None,
                                best_ask: None,
                                bid_size: None,
                                ask_size: None,
                            },
                        );
                        updates += 1;
                    }
                }
            }
        }
    }

    updates
}

// ─── WebSocket Broadcast ────────────────────────────────────────────

/// Broadcast updated prices to all connected frontend WebSocket clients.
fn broadcast_price_updates(state: &AppState, events: &[CanonicalEvent]) {
    for event in events {
        let odds_ref = match state.cache.odds.get(&event.id) {
            Some(r) => r,
            None => continue,
        };

        let msg = WsMessage::PriceUpdate(PriceUpdateData {
            canonical_event_id: event.id.clone(),
            kalshi: odds_ref.platform_odds.get("kalshi").map(|outcomes| {
                PlatformPriceData {
                    outcomes: outcomes
                        .iter()
                        .map(|(k, v)| (k.clone(), v.yes_price))
                        .collect(),
                    updated_at: odds_ref.updated_at.to_rfc3339(),
                }
            }),
            polymarket: odds_ref
                .platform_odds
                .get("polymarket")
                .map(|outcomes| PlatformPriceData {
                    outcomes: outcomes
                        .iter()
                        .map(|(k, v)| (k.clone(), v.yes_price))
                        .collect(),
                    updated_at: odds_ref.updated_at.to_rfc3339(),
                }),
        });

        let _ = state.arb_tx.send(msg);
    }
}

// ─── Retry Logic ────────────────────────────────────────────────────

/// Maximum number of retries per HTTP request (3 total attempts).
const MAX_RETRIES: u32 = 2;

/// Base delay for exponential backoff (doubles each retry: 300ms, 600ms).
const RETRY_BASE_DELAY_MS: u64 = 300;

/// Execute an HTTP request with exponential backoff retries.
///
/// Re-invokes the closure on each retry, creating a fresh `RequestBuilder`.
/// Handles transient failures (network errors, 5xx) without retrying 4xx.
async fn retry_request<F, Fut>(
    label: &str,
    operation: F,
) -> Result<reqwest::Response, reqwest::Error>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
{
    let mut last_err = None;
    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            let delay_ms = RETRY_BASE_DELAY_MS * 2u64.pow(attempt - 1);
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
        match operation().await {
            Ok(resp) => {
                // Don't retry client errors (4xx) — only server errors
                if resp.status().is_server_error() && attempt < MAX_RETRIES {
                    warn!(
                        "{}: server error {} on attempt {}/{}, retrying...",
                        label,
                        resp.status(),
                        attempt + 1,
                        MAX_RETRIES + 1
                    );
                    continue;
                }
                return Ok(resp);
            }
            Err(e) => {
                if attempt < MAX_RETRIES {
                    warn!(
                        "{}: attempt {}/{} failed: {}",
                        label,
                        attempt + 1,
                        MAX_RETRIES + 1,
                        e
                    );
                }
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap())
}

// ─── Helpers ────────────────────────────────────────────────────────

/// Build a plain HTTP client for public API reads (no auth required).
fn build_public_client() -> Client {
    Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("TLS backend unavailable — cannot create HTTP client")
}
