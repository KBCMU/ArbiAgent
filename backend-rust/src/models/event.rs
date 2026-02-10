use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported sports (matching DomeAPI's enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sport {
    Nfl,
    Nba,
    Mlb,
    Nhl,
    Cfb,
    Cbb,
    Pga,
    Tennis,
}

impl Sport {
    pub fn all() -> &'static [Sport] {
        &[
            Sport::Nfl,
            Sport::Nba,
            Sport::Mlb,
            Sport::Nhl,
            Sport::Cfb,
            Sport::Cbb,
            Sport::Pga,
            Sport::Tennis,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Sport::Nfl => "nfl",
            Sport::Nba => "nba",
            Sport::Mlb => "mlb",
            Sport::Nhl => "nhl",
            Sport::Cfb => "cfb",
            Sport::Cbb => "cbb",
            Sport::Pga => "pga",
            Sport::Tennis => "tennis",
        }
    }
}

/// Platform identifiers for a single canonical event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformIds {
    pub kalshi_event_ticker: Option<String>,
    pub kalshi_market_tickers: Vec<String>,
    pub polymarket_market_slug: Option<String>,
    pub polymarket_token_ids: Vec<String>,
}

/// A canonical event that may exist on multiple platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalEvent {
    /// Unique ID, e.g. "nfl-ari-den-2025-08-16"
    pub id: String,
    pub sport: Sport,
    pub event_title: String,
    pub game_start_time: Option<DateTime<Utc>>,
    pub status: String,
    pub platform_ids: PlatformIds,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Price data for a single outcome on a single platform.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OutcomePrice {
    /// Normalized price 0.0 - 1.0 (dollars)
    pub yes_price: f64,
    pub no_price: f64,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub bid_size: Option<f64>,
    pub ask_size: Option<f64>,
}

/// All odds for a canonical event, keyed by platform then outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventOdds {
    pub canonical_event_id: String,
    /// platform -> (outcome_name -> price)
    pub platform_odds: HashMap<String, HashMap<String, OutcomePrice>>,
    pub updated_at: DateTime<Utc>,
}
