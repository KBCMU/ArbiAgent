use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── DomeAPI Response Types ─────────────────────────────────────────

/// Response from GET /matching-markets/sports/{sport}?date=YYYY-MM-DD
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct DomeSportsMatchResponse {
    pub markets: HashMap<String, Vec<DomePlatformMatch>>,
    pub sport: Option<String>,
    pub date: Option<String>,
}

/// A single platform's data within a matched event.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "platform")]
pub enum DomePlatformMatch {
    #[serde(rename = "KALSHI")]
    Kalshi {
        event_ticker: String,
        market_tickers: Vec<String>,
    },
    #[serde(rename = "POLYMARKET")]
    Polymarket {
        market_slug: String,
        token_ids: Vec<String>,
    },
}

/// Response from GET /kalshi/market-price/{ticker}
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct KalshiPriceResponse {
    pub yes: KalshiSidePrice,
    pub no: KalshiSidePrice,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct KalshiSidePrice {
    pub price: f64,
    pub at_time: i64,
}

/// Response from GET /polymarket/market-price/{token_id}
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct PolymarketPriceResponse {
    pub price: f64,
    pub at_time: i64,
}

/// Response from GET /kalshi/markets
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct KalshiMarketsResponse {
    pub markets: Vec<KalshiMarketData>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct KalshiMarketData {
    pub ticker: Option<String>,
    pub event_ticker: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub status: Option<String>,
    pub yes_bid: Option<f64>,
    pub no_bid: Option<f64>,
    pub volume: Option<f64>,
    pub close_time: Option<String>,
    pub image_url: Option<String>,
}

/// Response from GET /polymarket/markets
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct PolymarketMarketsResponse {
    pub markets: Vec<PolymarketMarketData>,
    pub pagination: Option<PolymarketPagination>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct PolymarketMarketData {
    pub market_slug: Option<String>,
    pub event_slug: Option<String>,
    pub condition_id: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub side_a: Option<MarketSide>,
    pub side_b: Option<MarketSide>,
    pub volume_total: Option<f64>,
    pub image: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct MarketSide {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct PolymarketPagination {
    pub limit: Option<i32>,
    pub total: Option<i32>,
    pub has_more: Option<bool>,
    pub pagination_key: Option<String>,
}

// ─── Kalshi Direct API Response Types ───────────────────────────────
// Used by the direct batch price fetcher (ingestion/direct_api.rs).
// These map to https://api.elections.kalshi.com/trade-api/v2/markets

/// Wrapper for `GET /markets` on Kalshi's public API.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct KalshiDirectMarketsResponse {
    pub markets: Vec<KalshiDirectMarket>,
    pub cursor: Option<String>,
}

/// A single market from Kalshi's public `GET /markets` endpoint.
///
/// Prices use the new FixedPointDollars format (string like "0.5600").
/// We deserialize them as `Option<String>` and parse to f64 in application code.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct KalshiDirectMarket {
    pub ticker: String,
    pub event_ticker: Option<String>,
    /// Market lifecycle status: "active", "closed", "determined", "finalized", etc.
    pub status: Option<String>,
    /// Settlement result: "yes", "no", "" (empty until determined).
    pub result: Option<String>,
    /// Prices in FixedPointDollars (e.g. "0.5600" = $0.56).
    pub yes_bid_dollars: Option<String>,
    pub yes_ask_dollars: Option<String>,
    pub no_bid_dollars: Option<String>,
    pub no_ask_dollars: Option<String>,
    pub last_price_dollars: Option<String>,
    /// Open interest (string, e.g. "150.00")
    pub open_interest_fp: Option<String>,
    /// 24h volume (string, e.g. "500.00")
    pub volume_24h_fp: Option<String>,
}

// ─── API Response Types (our endpoints) ────────────────────────────

/// Serialized event for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct EventResponse {
    pub id: String,
    pub sport: String,
    pub event_title: String,
    pub game_start_time: Option<String>,
    pub created_at: String,
    pub status: String,
    /// Cross-platform match quality score (0–100). `None` for single-platform events.
    pub match_score: Option<f64>,
    pub kalshi: Option<PlatformOddsResponse>,
    pub polymarket: Option<PlatformOddsResponse>,
    pub arb_opportunity: Option<ArbOpportunityResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlatformOddsResponse {
    pub outcomes: HashMap<String, OutcomePriceResponse>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutcomePriceResponse {
    pub yes_price: f64,
    pub no_price: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArbOpportunityResponse {
    pub outcome: String,
    pub buy_platform: String,
    pub buy_price: f64,
    pub sell_platform: String,
    pub sell_price: f64,
    pub margin_pct: f64,
    pub score: i32,
}
