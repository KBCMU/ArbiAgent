//! Kalshi sports event fetcher — fetches and normalizes sports events
//! from Kalshi's public events API into `CandidateEvent` structs.

use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};

use crate::models::event::Sport;

use super::candidate::{
    self, CandidateEvent, Platform,
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
                        .filter_map(|e| kalshi_event_to_candidate(e, sport))
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

fn kalshi_event_to_candidate(event: KalshiEvent, sport: Sport) -> Option<CandidateEvent> {
    let markets = event.markets.unwrap_or_default();

    // Only include events with active markets
    let active_tickers: Vec<String> = markets
        .iter()
        .filter(|m| m.status.as_deref() == Some("active"))
        .map(|m| m.ticker.clone())
        .collect();

    if active_tickers.is_empty() {
        return None;
    }

    // Extract teams from market tickers (most reliable for Kalshi),
    // then normalize through the team dictionary so Kalshi abbreviations
    // like "NJ" become canonical "NJD" that match Polymarket labels.
    let (team_a_raw, team_b_raw) = candidate::extract_teams_from_kalshi_tickers(&active_tickers);
    // Some Kalshi events include additional active nested markets (spread/O-U),
    // so `len()==2` is too strict and causes moneyline false-negatives.
    let is_moneyline = team_a_raw.is_some() && team_b_raw.is_some();
    let team_a = team_a_raw.map(|t| {
        team_dictionary::lookup_team(&t, Some(sport)).unwrap_or(t)
    });
    let team_b = team_b_raw.map(|t| {
        team_dictionary::lookup_team(&t, Some(sport)).unwrap_or(t)
    });

    // Extract date from ticker or title
    let game_date = active_tickers
        .first()
        .and_then(|t| candidate::extract_date_from_kalshi_ticker(t))
        .or_else(|| candidate::extract_date_from_title(&event.title));

    let normalized_title = candidate::normalize_title(&event.title);

    Some(CandidateEvent {
        platform: Platform::Kalshi,
        sport,
        raw_title: event.title,
        normalized_title,
        game_date,
        team_a,
        team_b,
        is_moneyline,
        kalshi_event_ticker: Some(event.event_ticker),
        kalshi_market_tickers: active_tickers,
        polymarket_slug: None,
        polymarket_token_ids: vec![],
        polymarket_outcome_labels: vec![],
    })
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
    fn test_moneyline_detected_with_extra_active_markets() {
        // Real-world shape: winner + spread/total can all be active at once.
        let event = KalshiEvent {
            event_ticker: "KXNHLGAME-26MAR15BOSNJD".to_string(),
            series_ticker: None,
            title: "NHL: Boston Bruins vs New Jersey Devils".to_string(),
            sub_title: None,
            category: None,
            markets: Some(vec![
                make_market("KXNHLGAME-26MAR15BOSNJD-BOS", "active"),
                make_market("KXNHLGAME-26MAR15BOSNJD-NJ", "active"),
                make_market("KXNHLGAME-26MAR15BOSNJD-SPREAD-BOS", "active"),
            ]),
        };

        let candidate = kalshi_event_to_candidate(event, Sport::Nhl).expect("candidate");
        assert!(candidate.is_moneyline, "Should still be moneyline despite extra active markets");
        assert_eq!(candidate.team_a.as_deref(), Some("BOS"));
        assert_eq!(candidate.team_b.as_deref(), Some("NJD"));
    }
}
