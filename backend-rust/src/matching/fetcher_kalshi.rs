//! Kalshi sports event fetcher — fetches and normalizes sports events
//! from Kalshi's public events API into `CandidateEvent` structs.

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};

use crate::models::event::Sport;

use super::candidate::{
    self, CandidateEvent, MarketType, Platform,
};
use super::team_dictionary;

const KALSHI_API_BASE: &str = "https://api.elections.kalshi.com/trade-api/v2";
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

// ─── API Response Types ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct KalshiEventsResponse {
    events: Vec<KalshiEvent>,
    #[allow(dead_code)]
    cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KalshiEvent {
    event_ticker: String,
    #[allow(dead_code)]
    series_ticker: Option<String>,
    title: String,
    #[allow(dead_code)]
    sub_title: Option<String>,
    #[allow(dead_code)]
    category: Option<String>,
    markets: Option<Vec<KalshiMarketNested>>,
    /// ISO-8601 close time — acts as a proxy for game start
    close_time: Option<String>,
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

// ─── Public API ─────────────────────────────────────────────────────

/// Fetch all sports events from Kalshi for the given sports, returning
/// normalized `CandidateEvent` structs ready for cross-platform matching.
pub async fn fetch_kalshi_sports_candidates(
    sports: &[Sport],
) -> Vec<CandidateEvent> {
    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("TLS backend unavailable");

    let mut all_candidates = Vec::new();

    for &sport in sports {
        let series_prefixes = team_dictionary::sport_to_kalshi_series(sport);
        for &prefix in series_prefixes {
            match fetch_kalshi_events_for_series(&client, prefix).await {
                Ok(events) => {
                    let count = events.len();
                    let candidates: Vec<CandidateEvent> = events
                        .into_iter()
                        .flat_map(|e| kalshi_event_to_candidates(e, sport))
                        .collect();
                    info!(
                        "Kalshi {}/{}: {} events → {} candidates",
                        sport.as_str(), prefix, count, candidates.len()
                    );
                    all_candidates.extend(candidates);
                }
                Err(e) => {
                    warn!("Kalshi fetch failed for {}/{}: {}", sport.as_str(), prefix, e);
                }
            }
        }
    }

    all_candidates
}

// ─── Internal ───────────────────────────────────────────────────────

async fn fetch_kalshi_events_for_series(
    client: &Client,
    series_ticker: &str,
) -> anyhow::Result<Vec<KalshiEvent>> {
    let url = format!(
        "{}/events?status=open&with_nested_markets=true&limit=200&series_ticker={}",
        KALSHI_API_BASE, series_ticker,
    );

    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Kalshi events API returned {}: {}", status, &body[..body.len().min(200)]);
    }

    let data: KalshiEventsResponse = resp.json().await?;
    Ok(data.events)
}

/// Classify a Kalshi market ticker into a market type.
///
/// Patterns observed:
/// - Moneyline: `KXNHLGAME-26MAR15BOSNJD-BOS`  (event_ticker + `-TEAM`)
/// - Spread:    `KXNHLGAME-26MAR15BOSNJD-SPREAD-BOS` (contains `-SPREAD-`)
/// - Total:     `KXNHLGAME-26MAR15BOSNJD-OVER-215` (contains `-OVER-`)
fn classify_kalshi_ticker(ticker: &str) -> MarketType {
    let upper = ticker.to_uppercase();
    if upper.contains("-SPREAD") {
        return MarketType::Spread(extract_line_from_ticker(&upper));
    }
    if upper.contains("-OVER") || upper.contains("-UNDER") || upper.contains("-TOTAL") {
        let line = extract_line_from_ticker(&upper);
        return MarketType::Total(line);
    }
    MarketType::Moneyline
}

/// Try to extract a numeric line from a ticker segment (e.g., 215.5 from `...-OVER-215.5`).
fn extract_line_from_ticker(ticker: &str) -> f64 {
    for seg in ticker.rsplit('-') {
        if let Ok(v) = seg.replace('_', ".").parse::<f64>() {
            return v;
        }
    }
    0.0
}

/// Convert a single Kalshi event into one or more CandidateEvents,
/// emitting separate candidates for moneyline, spread, and total markets.
fn kalshi_event_to_candidates(event: KalshiEvent, sport: Sport) -> Vec<CandidateEvent> {
    use candidate::MarketType;

    let markets = event.markets.unwrap_or_default();

    let active_markets: Vec<&KalshiMarketNested> = markets
        .iter()
        .filter(|m| m.status.as_deref() == Some("active"))
        .collect();

    if active_markets.is_empty() {
        return vec![];
    }

    // Group active tickers by market type
    let mut moneyline_tickers: Vec<String> = Vec::new();
    let mut spread_tickers: Vec<String> = Vec::new();
    let mut total_tickers: Vec<String> = Vec::new();
    let mut spread_line = 0.0_f64;
    let mut total_line = 0.0_f64;

    for m in &active_markets {
        match classify_kalshi_ticker(&m.ticker) {
            MarketType::Moneyline => moneyline_tickers.push(m.ticker.clone()),
            MarketType::Spread(line) => {
                spread_tickers.push(m.ticker.clone());
                if line != 0.0 { spread_line = line; }
            }
            MarketType::Total(line) => {
                total_tickers.push(m.ticker.clone());
                if line != 0.0 { total_line = line; }
            }
        }
    }

    // Shared fields: teams, date, title, start time
    let all_tickers: Vec<String> = active_markets.iter().map(|m| m.ticker.clone()).collect();
    let (team_a_raw, team_b_raw) = candidate::extract_teams_from_kalshi_tickers(
        if !moneyline_tickers.is_empty() { &moneyline_tickers } else { &all_tickers }
    );
    let team_a = team_a_raw.map(|t| team_dictionary::lookup_team_or_passthrough(&t, Some(sport)));
    let team_b = team_b_raw.map(|t| team_dictionary::lookup_team_or_passthrough(&t, Some(sport)));

    let game_date = all_tickers
        .first()
        .and_then(|t| candidate::extract_date_from_kalshi_ticker(t))
        .or_else(|| candidate::extract_date_from_title(&event.title));

    let normalized_title = candidate::normalize_title(&event.title);

    let game_start_time = event
        .close_time
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let mut candidates = Vec::new();

    // Emit moneyline candidate
    if team_a.is_some() && team_b.is_some() && !moneyline_tickers.is_empty() {
        candidates.push(CandidateEvent {
            platform: Platform::Kalshi,
            sport,
            raw_title: event.title.clone(),
            normalized_title: normalized_title.clone(),
            game_date,
            game_start_time,
            team_a: team_a.clone(),
            team_b: team_b.clone(),
            market_type: MarketType::Moneyline,
            kalshi_event_ticker: Some(event.event_ticker.clone()),
            kalshi_market_tickers: moneyline_tickers,
            polymarket_slug: None,
            polymarket_token_ids: vec![],
            polymarket_outcome_labels: vec![],
        });
    }

    // Emit spread candidate
    if !spread_tickers.is_empty() {
        candidates.push(CandidateEvent {
            platform: Platform::Kalshi,
            sport,
            raw_title: event.title.clone(),
            normalized_title: normalized_title.clone(),
            game_date,
            game_start_time,
            team_a: team_a.clone(),
            team_b: team_b.clone(),
            market_type: MarketType::Spread(spread_line),
            kalshi_event_ticker: Some(event.event_ticker.clone()),
            kalshi_market_tickers: spread_tickers,
            polymarket_slug: None,
            polymarket_token_ids: vec![],
            polymarket_outcome_labels: vec![],
        });
    }

    // Emit total candidate
    if !total_tickers.is_empty() {
        candidates.push(CandidateEvent {
            platform: Platform::Kalshi,
            sport,
            raw_title: event.title.clone(),
            normalized_title: normalized_title.clone(),
            game_date,
            game_start_time,
            team_a: team_a.clone(),
            team_b: team_b.clone(),
            market_type: MarketType::Total(total_line),
            kalshi_event_ticker: Some(event.event_ticker.clone()),
            kalshi_market_tickers: total_tickers,
            polymarket_slug: None,
            polymarket_token_ids: vec![],
            polymarket_outcome_labels: vec![],
        });
    }

    // Fallback: if no moneyline tickers but we have teams, still emit one
    if candidates.is_empty() && team_a.is_some() && team_b.is_some() {
        candidates.push(CandidateEvent {
            platform: Platform::Kalshi,
            sport,
            raw_title: event.title.clone(),
            normalized_title: normalized_title.clone(),
            game_date,
            game_start_time,
            team_a,
            team_b,
            market_type: MarketType::Moneyline,
            kalshi_event_ticker: Some(event.event_ticker.clone()),
            kalshi_market_tickers: all_tickers,
            polymarket_slug: None,
            polymarket_token_ids: vec![],
            polymarket_outcome_labels: vec![],
        });
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_market(ticker: &str, status: &str) -> KalshiMarketNested {
        KalshiMarketNested {
            ticker: ticker.to_string(),
            yes_sub_title: None,
            no_sub_title: None,
            status: Some(status.to_string()),
            yes_bid_dollars: None,
            yes_ask_dollars: None,
            last_price_dollars: None,
            volume_24h_fp: None,
        }
    }

    #[test]
    fn test_moneyline_and_spread_emitted_separately() {
        let event = KalshiEvent {
            event_ticker: "KXNHLGAME-26MAR15BOSNJD".to_string(),
            series_ticker: None,
            title: "NHL: Boston Bruins vs New Jersey Devils".to_string(),
            sub_title: None,
            category: None,
            close_time: None,
            markets: Some(vec![
                make_market("KXNHLGAME-26MAR15BOSNJD-BOS", "active"),
                make_market("KXNHLGAME-26MAR15BOSNJD-NJ", "active"),
                make_market("KXNHLGAME-26MAR15BOSNJD-SPREAD-BOS", "active"),
            ]),
        };

        let candidates = kalshi_event_to_candidates(event, Sport::Nhl);
        assert_eq!(candidates.len(), 2, "Should emit moneyline + spread");

        let ml = candidates.iter().find(|c| c.market_type.is_moneyline()).expect("moneyline");
        assert_eq!(ml.team_a.as_deref(), Some("BOS"));
        assert_eq!(ml.team_b.as_deref(), Some("NJD"));
        assert_eq!(ml.kalshi_market_tickers.len(), 2);

        let spread = candidates.iter().find(|c| matches!(c.market_type, MarketType::Spread(_))).expect("spread");
        assert_eq!(spread.kalshi_market_tickers.len(), 1);
    }

    #[test]
    fn test_total_market_detected() {
        let event = KalshiEvent {
            event_ticker: "KXNBAGAME-26MAR15LALBOS".to_string(),
            series_ticker: None,
            title: "NBA: Lakers vs Celtics".to_string(),
            sub_title: None,
            category: None,
            close_time: None,
            markets: Some(vec![
                make_market("KXNBAGAME-26MAR15LALBOS-LAL", "active"),
                make_market("KXNBAGAME-26MAR15LALBOS-BOS", "active"),
                make_market("KXNBAGAME-26MAR15LALBOS-OVER-215", "active"),
                make_market("KXNBAGAME-26MAR15LALBOS-UNDER-215", "active"),
            ]),
        };

        let candidates = kalshi_event_to_candidates(event, Sport::Nba);
        let total = candidates.iter().find(|c| matches!(c.market_type, MarketType::Total(_)));
        assert!(total.is_some(), "Should detect total market");
        if let MarketType::Total(line) = &total.unwrap().market_type {
            assert!((line - 215.0).abs() < 0.1);
        }
    }
}
