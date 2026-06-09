use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::US::Eastern;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Event category — sports (DomeAPI-matched) and culture (direct API discovery).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sport {
    // Sports (discovered via DomeAPI matching-markets endpoint)
    Nfl,
    Nba,
    Mlb,
    Nhl,
    Cfb,
    Cbb,
    Pga,
    Tennis,
    Ufc,
    // Non-sports categories (discovered via direct Kalshi/Polymarket APIs)
    Politics,
    Crypto,
    Culture,
    Science,
    Economics,
    World,
}

impl Sport {
    /// Sports categories only — used for DomeAPI matching-markets discovery.
    pub fn sports_for_discovery() -> &'static [Sport] {
        &[
            Sport::Nfl,
            Sport::Nba,
            Sport::Mlb,
            Sport::Nhl,
            Sport::Cfb,
            Sport::Cbb,
            Sport::Pga,
            Sport::Tennis,
            Sport::Ufc,
        ]
    }

    pub fn is_sport(&self) -> bool {
        matches!(
            self,
            Sport::Nfl
                | Sport::Nba
                | Sport::Mlb
                | Sport::Nhl
                | Sport::Cfb
                | Sport::Cbb
                | Sport::Pga
                | Sport::Tennis
                | Sport::Ufc
        )
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
            Sport::Ufc => "ufc",
            Sport::Politics => "politics",
            Sport::Crypto => "crypto",
            Sport::Culture => "culture",
            Sport::Science => "science",
            Sport::Economics => "economics",
            Sport::World => "world",
        }
    }

    /// Parse a category string into a Sport variant. Falls back to Culture for unknowns.
    pub fn from_str_loose(s: &str) -> Sport {
        match s.to_lowercase().as_str() {
            "nfl" => Sport::Nfl,
            "nba" => Sport::Nba,
            "mlb" => Sport::Mlb,
            "nhl" => Sport::Nhl,
            "cfb" => Sport::Cfb,
            "cbb" => Sport::Cbb,
            "pga" => Sport::Pga,
            "tennis" => Sport::Tennis,
            "ufc" | "mma" => Sport::Ufc,
            "politics" | "elections" => Sport::Politics,
            "crypto" | "cryptocurrency" => Sport::Crypto,
            "science" | "tech" | "technology" => Sport::Science,
            "economics" | "economy" | "finance" | "financial" => Sport::Economics,
            "world" | "global" | "geopolitics" => Sport::World,
            _ => Sport::Culture,
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
    /// Outcome labels for each polymarket token (from Gamma API metadata).
    /// Same order as polymarket_token_ids.
    pub polymarket_outcome_labels: Vec<String>,
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
    /// Score (0–100) from the cross-platform matcher. `None` for single-platform events.
    pub match_score: Option<f64>,
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

/// Threshold above which an outcome price indicates market settlement.
/// 97¢ YES means the market is effectively decided — no pre-game line
/// reaches this level, but all settled markets do (typically 98-100¢).
const SETTLED_PRICE_THRESHOLD: f64 = 0.97;

/// Check if an event's markets have effectively settled.
///
/// Two independent signals, either sufficient:
/// 1. **Status-based** — the canonical event status has been set to "settled"
///    or "closed" by the Kalshi direct API lifecycle (authoritative).
/// 2. **Price-based** — any outcome on any platform shows YES ≥ 97¢,
///    indicating the market result is known (heuristic fallback when
///    the DomeAPI price loop is used and status is never updated).
pub fn is_event_settled_full(status: &str, odds: &EventOdds) -> bool {
    // Signal 1: Kalshi market lifecycle
    if status == "settled" || status == "closed" {
        return true;
    }
    // Signal 2: price threshold heuristic
    is_event_settled(odds)
}

/// Price-only settlement check (legacy, used when event status is unavailable).
pub fn is_event_settled(odds: &EventOdds) -> bool {
    for outcomes in odds.platform_odds.values() {
        for price in outcomes.values() {
            if price.yes_price >= SETTLED_PRICE_THRESHOLD {
                return true;
            }
        }
    }
    false
}

/// Whether a canonical event's game day is strictly before today (US/Eastern).
///
/// Prefers `game_start_time` when set (PredictionAPI / matched events). Falls back
/// to parsing `YYYY-MM-DD` from the event ID for legacy native-matching IDs.
pub fn is_canonical_event_past(event: &CanonicalEvent) -> bool {
    if let Some(start) = event.game_start_time {
        let today_eastern = Utc::now().with_timezone(&Eastern).date_naive();
        let game_date = start.with_timezone(&Eastern).date_naive();
        return game_date < today_eastern;
    }
    is_event_past(&event.id)
}

/// Check if an event's game date (embedded in its ID) has already passed.
///
/// Event IDs follow the pattern `"sport-team1-team2-YYYY-MM-DD"`.
/// Returns `true` if the extracted date is strictly before today in US/Eastern.
/// Returns `false` if the date can't be parsed (safe default: keep the event).
pub fn is_event_past(event_id: &str) -> bool {
    let parts: Vec<&str> = event_id.split('-').collect();
    // Need at least 4 parts: sport, team(s)..., year, month, day
    if parts.len() < 4 {
        return false;
    }
    // Date is always the last 3 segments: YYYY-MM-DD
    let date_str = format!(
        "{}-{}-{}",
        parts[parts.len() - 3],
        parts[parts.len() - 2],
        parts[parts.len() - 1],
    );
    match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
        Ok(game_date) => {
            let today_eastern = Utc::now().with_timezone(&Eastern).date_naive();
            game_date < today_eastern
        }
        Err(_) => false, // Can't parse → assume not past
    }
}
