use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};

use crate::models::event::Sport;
use crate::models::vegas::{VegasOdds, VegasOutcomeOdds};
use crate::AppState;

use super::vegas_matcher;

const BETSTACK_API_BASE: &str = "https://api.betstack.dev/api/v1";
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
const POLL_INTERVAL_SECS: u64 = 60;

/// Maps our Sport enum to BetStack league identifiers.
fn sport_to_betstack_league(sport: &Sport) -> Option<&'static str> {
    match sport {
        Sport::Nfl => Some("nfl"),
        Sport::Nba => Some("nba"),
        Sport::Mlb => Some("mlb"),
        Sport::Nhl => Some("nhl"),
        Sport::Cfb => Some("cfb"),
        Sport::Cbb => Some("cbb"),
        Sport::Ufc => Some("ufc"),
        _ => None,
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct BetStackEvent {
    pub id: Option<String>,
    pub home_team: Option<String>,
    pub away_team: Option<String>,
    pub sport: Option<String>,
    pub league: Option<String>,
    pub commence_time: Option<String>,
    pub money_line_home: Option<f64>,
    pub money_line_away: Option<f64>,
    pub home_spread: Option<f64>,
    pub away_spread: Option<f64>,
    pub total: Option<f64>,
    pub bookmaker_count: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct BetStackResponse {
    #[serde(default)]
    events: Vec<BetStackEvent>,
}

/// Convert American moneyline to implied probability.
pub fn moneyline_to_implied_prob(ml: f64) -> f64 {
    if ml < 0.0 {
        let abs_ml = ml.abs();
        abs_ml / (abs_ml + 100.0)
    } else if ml > 0.0 {
        100.0 / (ml + 100.0)
    } else {
        0.5
    }
}

/// De-vig two implied probabilities using multiplicative normalization.
pub fn devig_multiplicative(prob_a: f64, prob_b: f64) -> (f64, f64) {
    let total = prob_a + prob_b;
    if total <= 0.0 {
        return (0.5, 0.5);
    }
    (prob_a / total, prob_b / total)
}

/// Convert a BetStack event into VegasOdds (not yet matched to a canonical event).
pub fn betstack_event_to_vegas_odds(event: &BetStackEvent) -> Option<VegasOdds> {
    let ml_home = event.money_line_home?;
    let ml_away = event.money_line_away?;
    let home = event.home_team.as_deref()?;
    let away = event.away_team.as_deref()?;

    let implied_home = moneyline_to_implied_prob(ml_home);
    let implied_away = moneyline_to_implied_prob(ml_away);
    let (fair_home, fair_away) = devig_multiplicative(implied_home, implied_away);

    let num_books = event.bookmaker_count.unwrap_or(1);

    let mut outcomes = HashMap::new();
    outcomes.insert(
        home.to_string(),
        VegasOutcomeOdds {
            consensus_moneyline: ml_home,
            implied_prob: implied_home,
            fair_prob: fair_home,
            num_books,
        },
    );
    outcomes.insert(
        away.to_string(),
        VegasOutcomeOdds {
            consensus_moneyline: ml_away,
            implied_prob: implied_away,
            fair_prob: fair_away,
            num_books,
        },
    );

    Some(VegasOdds {
        canonical_event_id: String::new(), // populated by matcher
        outcomes,
        updated_at: Utc::now(),
    })
}

/// Background loop: polls BetStack API for sportsbook consensus odds.
pub async fn run_vegas_polling_loop(state: Arc<AppState>) {
    let api_key = std::env::var("BETSTACK_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        warn!("⚠️ BETSTACK_API_KEY not set — Vegas odds polling disabled");
        return;
    }

    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("Failed to create HTTP client for BetStack");

    let interval = tokio::time::Duration::from_secs(POLL_INTERVAL_SECS);

    // Wait for event discovery to populate the cache
    tokio::time::sleep(tokio::time::Duration::from_secs(45)).await;
    info!("🎰 Vegas odds poller started (interval: {}s)", POLL_INTERVAL_SECS);

    loop {
        let mut total_events = 0usize;
        let mut matched = 0usize;

        for sport in Sport::sports_for_discovery() {
            let league = match sport_to_betstack_league(sport) {
                Some(l) => l,
                None => continue,
            };

            match fetch_betstack_events(&client, &api_key, league).await {
                Ok(events) => {
                    total_events += events.len();
                    for bs_event in &events {
                        if let Some(vegas_odds) = betstack_event_to_vegas_odds(bs_event) {
                            if vegas_matcher::match_and_store(&state, bs_event, vegas_odds, sport) {
                                matched += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("BetStack fetch failed for {}: {}", league, e);
                }
            }

            // Respect rate limits between league requests
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        if total_events > 0 {
            info!(
                "🎰 Vegas poll: {} BetStack events fetched, {} matched to canonical events",
                total_events, matched
            );
        }

        tokio::time::sleep(interval).await;
    }
}

async fn fetch_betstack_events(
    client: &Client,
    api_key: &str,
    league: &str,
) -> anyhow::Result<Vec<BetStackEvent>> {
    let url = format!("{}/events?league={}", BETSTACK_API_BASE, league);
    let resp = client
        .get(&url)
        .header("X-API-Key", api_key)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "BetStack API returned {} for league {}: {}",
            status,
            league,
            &body[..body.len().min(200)]
        );
    }

    let data: BetStackResponse = resp.json().await?;
    Ok(data.events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moneyline_favorite() {
        let prob = moneyline_to_implied_prob(-150.0);
        assert!((prob - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_moneyline_underdog() {
        let prob = moneyline_to_implied_prob(150.0);
        assert!((prob - 0.4).abs() < 0.001);
    }

    #[test]
    fn test_moneyline_even() {
        let prob = moneyline_to_implied_prob(100.0);
        assert!((prob - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_devig() {
        let (fair_a, fair_b) = devig_multiplicative(0.6, 0.5);
        let total = fair_a + fair_b;
        assert!((total - 1.0).abs() < 0.001);
    }
}
