//! Culture event discovery — fetches non-sports events from Kalshi and Polymarket.
//!
//! Unlike sports events (discovered via DomeAPI's cross-platform matching),
//! culture events are discovered independently from each platform's API
//! and stored as single-platform events. Cross-platform matching for arb
//! detection can be added later.
//!
//! - **Polymarket**: Gamma API (`GET /events`) with tag-based filtering
//! - **Kalshi**: Direct API (`GET /events`) with series_ticker filtering

use std::sync::Arc;

use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};

use crate::models::event::{CanonicalEvent, PlatformIds, Sport};
use crate::AppState;

const POLYMARKET_GAMMA_BASE: &str = "https://gamma-api.polymarket.com";
const KALSHI_API_BASE: &str = "https://api.elections.kalshi.com/trade-api/v2";
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// Tags to exclude from Polymarket discovery (handled by DomeAPI sports matching).
const SPORTS_TAGS: &[&str] = &["sports", "nfl", "nba", "mlb", "nhl", "ncaaf", "ncaab", "pga", "tennis"];

// ─── Polymarket Gamma API Types ─────────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GammaEvent {
    slug: Option<String>,
    title: Option<String>,
    closed: Option<bool>,
    active: Option<bool>,
    volume: Option<f64>,
    markets: Option<Vec<GammaMarket>>,
    tags: Option<Vec<GammaTag>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GammaMarket {
    #[serde(rename = "clobTokenIds")]
    clob_token_ids: Option<String>,
    outcomes: Option<String>,
    #[serde(rename = "outcomePrices")]
    outcome_prices: Option<String>,
    question: Option<String>,
    active: Option<bool>,
    closed: Option<bool>,
    #[serde(rename = "enableOrderBook")]
    enable_order_book: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GammaTag {
    label: Option<String>,
    slug: Option<String>,
}

// ─── Kalshi Direct API Types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct KalshiEventsResponse {
    events: Vec<KalshiEvent>,
    cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct KalshiEvent {
    event_ticker: String,
    series_ticker: Option<String>,
    title: String,
    sub_title: Option<String>,
    category: Option<String>,
    markets: Option<Vec<KalshiMarketNested>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct KalshiMarketNested {
    ticker: String,
    yes_sub_title: Option<String>,
    no_sub_title: Option<String>,
    status: Option<String>,
    yes_bid_dollars: Option<String>,
    yes_ask_dollars: Option<String>,
    last_price_dollars: Option<String>,
    volume_24h_fp: Option<String>,
}

// ─── Public Entry Point ─────────────────────────────────────────────

/// Background loop: discovers culture events from Kalshi and Polymarket.
///
/// Runs every 10 minutes. Creates single-platform CanonicalEvents that the
/// existing price refresh and API layers handle automatically.
pub async fn run_culture_discovery_loop(state: Arc<AppState>) {
    let interval = tokio::time::Duration::from_secs(600); // 10 minutes
    let client = build_client();

    // Initial delay — let DomeAPI sports discovery run first
    tokio::time::sleep(tokio::time::Duration::from_secs(45)).await;
    info!("🌐 Culture event discovery loop started");

    loop {
        let before_count = state.cache.event_count();

        let (poly_count, kalshi_count) = tokio::join!(
            discover_polymarket_culture(&client, &state),
            discover_kalshi_culture(&client, &state),
        );

        let after_count = state.cache.event_count();
        let new_events = after_count.saturating_sub(before_count);

        info!(
            "🌐 Culture discovery: {} Polymarket + {} Kalshi events ({} new, {} total in cache)",
            poly_count, kalshi_count, new_events, after_count
        );

        tokio::time::sleep(interval).await;
    }
}

// ─── Polymarket Culture Discovery ───────────────────────────────────

/// Fetch top culture events from Polymarket's Gamma API.
///
/// Uses the public events endpoint sorted by 24h volume, filtering out
/// sports events (which are handled by DomeAPI). Extracts token IDs
/// from each market for price fetching.
async fn discover_polymarket_culture(client: &Client, state: &AppState) -> usize {
    let url = format!(
        "{}/events?closed=false&limit=100&order=volume24hr&ascending=false",
        POLYMARKET_GAMMA_BASE
    );

    let events: Vec<GammaEvent> = match client.get(&url).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                warn!("Polymarket Gamma API returned {}", resp.status());
                return 0;
            }
            match resp.json().await {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to parse Polymarket Gamma response: {}", e);
                    return 0;
                }
            }
        }
        Err(e) => {
            warn!("Polymarket Gamma API request failed: {}", e);
            return 0;
        }
    };

    let mut count = 0usize;

    for event in &events {
        if event.closed.unwrap_or(false) || !event.active.unwrap_or(true) {
            continue;
        }

        // Skip sports events
        if is_sports_event_poly(event) {
            continue;
        }

        let slug = match &event.slug {
            Some(s) if !s.is_empty() => s.clone(),
            _ => continue,
        };

        let title = match &event.title {
            Some(t) if !t.is_empty() => t.clone(),
            _ => continue,
        };

        let event_id = format!("poly-{}", slug);

        // Skip if already tracked (from DomeAPI or a previous cycle)
        if state.cache.events.contains_key(&event_id) {
            count += 1;
            continue;
        }

        // Extract token IDs and outcome labels from the event's markets
        let (token_ids, outcome_labels) = extract_polymarket_tokens(event);

        if token_ids.is_empty() {
            continue;
        }

        let category = classify_polymarket_event(event);

        let canonical = CanonicalEvent {
            id: event_id,
            sport: category,
            event_title: title,
            game_start_time: None,
            status: "open".to_string(),
            platform_ids: PlatformIds {
                kalshi_event_ticker: None,
                kalshi_market_tickers: vec![],
                polymarket_market_slug: Some(slug),
                polymarket_token_ids: token_ids,
                polymarket_outcome_labels: outcome_labels,
            },
            match_score: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        state.cache.upsert_event(canonical);
        count += 1;
    }

    count
}

/// Extract CLOB token IDs and outcome labels from a Polymarket event.
///
/// The Gamma API returns `clobTokenIds` and `outcomes` as **JSON array strings**
/// (e.g., `'["token1","token2"]'` and `'["Yes","No"]'`), not plain arrays.
fn extract_polymarket_tokens(event: &GammaEvent) -> (Vec<String>, Vec<String>) {
    let mut all_token_ids = Vec::new();
    let mut all_labels = Vec::new();

    let markets = match &event.markets {
        Some(m) => m,
        None => return (all_token_ids, all_labels),
    };

    for market in markets {
        if market.closed.unwrap_or(false) || !market.active.unwrap_or(true) {
            continue;
        }

        let token_ids_str = match &market.clob_token_ids {
            Some(s) if !s.is_empty() => s.clone(),
            _ => continue,
        };

        // clobTokenIds is a JSON array string: '["abc123","def456"]'
        let tokens: Vec<String> = match serde_json::from_str(&token_ids_str) {
            Ok(t) => t,
            Err(_) => {
                // Fallback: try comma-separated (unlikely but defensive)
                token_ids_str
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            }
        };

        let outcome_names: Vec<String> = market
            .outcomes
            .as_ref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default();

        for (i, token) in tokens.into_iter().enumerate() {
            let label = outcome_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("OUTCOME_{}", i));
            all_token_ids.push(token);
            all_labels.push(label);
        }
    }

    (all_token_ids, all_labels)
}

/// Classify a Polymarket event into a category based on its tags.
fn classify_polymarket_event(event: &GammaEvent) -> Sport {
    let tags = match &event.tags {
        Some(t) => t,
        None => return Sport::Culture,
    };

    for tag in tags {
        let label = tag
            .label
            .as_deref()
            .or(tag.slug.as_deref())
            .unwrap_or("")
            .to_lowercase();

        match label.as_str() {
            "politics" | "elections" | "government" => return Sport::Politics,
            "crypto" | "cryptocurrency" | "bitcoin" | "ethereum" => return Sport::Crypto,
            "science" | "technology" | "tech" | "ai" => return Sport::Science,
            "economics" | "economy" | "finance" | "fed" | "interest rates" => {
                return Sport::Economics
            }
            "world" | "global" | "geopolitics" | "international" | "conflict" => {
                return Sport::World
            }
            _ => {}
        }
    }

    Sport::Culture
}

/// Check if a Polymarket event is a sports event (should be skipped).
fn is_sports_event_poly(event: &GammaEvent) -> bool {
    if let Some(tags) = &event.tags {
        for tag in tags {
            let label = tag
                .label
                .as_deref()
                .or(tag.slug.as_deref())
                .unwrap_or("")
                .to_lowercase();
            if SPORTS_TAGS.iter().any(|&s| label.contains(s)) {
                return true;
            }
        }
    }
    false
}

// ─── Kalshi Culture Discovery ───────────────────────────────────────

/// Kalshi series tickers that correspond to sports (skip these).
const KALSHI_SPORTS_SERIES: &[&str] = &[
    "KXNFL", "KXNBA", "KXMLB", "KXNHL", "KXCFB", "KXCBB", "KXPGA", "KXTEN",
    "NFL", "NBA", "MLB", "NHL",
];

/// Fetch culture events from Kalshi's direct events API.
///
/// Fetches open events with nested markets, filtering out sports series.
/// Only includes events with active markets that have meaningful volume.
async fn discover_kalshi_culture(client: &Client, state: &AppState) -> usize {
    let url = format!(
        "{}/events?status=open&with_nested_markets=true&limit=200",
        KALSHI_API_BASE
    );

    let response: KalshiEventsResponse = match client.get(&url).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                warn!("Kalshi events API returned {}", resp.status());
                return 0;
            }
            match resp.json().await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to parse Kalshi events response: {}", e);
                    return 0;
                }
            }
        }
        Err(e) => {
            warn!("Kalshi events API request failed: {}", e);
            return 0;
        }
    };

    let mut count = 0usize;

    for event in &response.events {
        // Skip sports series
        if is_kalshi_sports_event(event) {
            continue;
        }

        let event_id = format!("kalshi-{}", event.event_ticker);

        // Skip if already tracked
        if state.cache.events.contains_key(&event_id) {
            count += 1;
            continue;
        }

        // Extract market tickers from nested markets
        let markets = match &event.markets {
            Some(m) if !m.is_empty() => m,
            _ => continue,
        };

        let tickers: Vec<String> = markets
            .iter()
            .filter(|m| m.status.as_deref() == Some("active"))
            .map(|m| m.ticker.clone())
            .collect();

        if tickers.is_empty() {
            continue;
        }

        let category = classify_kalshi_event(event);

        let canonical = CanonicalEvent {
            id: event_id,
            sport: category,
            event_title: event.title.clone(),
            game_start_time: None,
            status: "open".to_string(),
            platform_ids: PlatformIds {
                kalshi_event_ticker: Some(event.event_ticker.clone()),
                kalshi_market_tickers: tickers,
                polymarket_market_slug: None,
                polymarket_token_ids: vec![],
                polymarket_outcome_labels: vec![],
            },
            match_score: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        state.cache.upsert_event(canonical);
        count += 1;
    }

    count
}

/// Check if a Kalshi event belongs to a sports series.
fn is_kalshi_sports_event(event: &KalshiEvent) -> bool {
    if let Some(series) = &event.series_ticker {
        let upper = series.to_uppercase();
        if KALSHI_SPORTS_SERIES
            .iter()
            .any(|&s| upper.starts_with(s))
        {
            return true;
        }
    }

    // Also check the category field (deprecated but still populated)
    if let Some(cat) = &event.category {
        let lower = cat.to_lowercase();
        if lower.contains("sport") || lower.contains("football") || lower.contains("basketball") {
            return true;
        }
    }

    false
}

/// Classify a Kalshi event into a category.
fn classify_kalshi_event(event: &KalshiEvent) -> Sport {
    let cat = event.category.as_deref().unwrap_or("");
    let title = event.title.to_lowercase();
    let series = event.series_ticker.as_deref().unwrap_or("");

    if cat.contains("politic") || title.contains("election") || title.contains("president") {
        return Sport::Politics;
    }
    if cat.contains("crypto") || title.contains("bitcoin") || title.contains("ethereum") {
        return Sport::Crypto;
    }
    if cat.contains("econ") || title.contains("gdp") || title.contains("inflation") || title.contains("fed ") {
        return Sport::Economics;
    }
    if cat.contains("science") || cat.contains("tech") || title.contains("ai ") {
        return Sport::Science;
    }

    // Fall back to string-based matching
    if !series.is_empty() {
        return Sport::from_str_loose(cat);
    }

    Sport::Culture
}

// ─── Helpers ────────────────────────────────────────────────────────

fn build_client() -> Client {
    Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("TLS backend unavailable")
}
