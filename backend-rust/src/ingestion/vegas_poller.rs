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
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const POLL_INTERVAL_SECS: u64 = 65; // slightly above the 60s rate limit

/// Maps BetStack league key to our Sport enum.
fn league_key_to_sport(key: &str) -> Option<Sport> {
    match key {
        "americanfootball_nfl" => Some(Sport::Nfl),
        "basketball_nba" => Some(Sport::Nba),
        "baseball_mlb" => Some(Sport::Mlb),
        "icehockey_nhl" => Some(Sport::Nhl),
        "americanfootball_ncaaf" => Some(Sport::Cfb),
        "basketball_ncaab" => Some(Sport::Cbb),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
pub struct BetStackLineLeague {
    pub key: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BetStackLineEvent {
    pub id: Option<i64>,
    pub commence_time: Option<String>,
    pub home_team: Option<String>,
    pub away_team: Option<String>,
    pub league: Option<BetStackLineLeague>,
}

#[derive(Debug, Deserialize)]
pub struct BetStackMoneyline {
    pub home: Option<String>,
    pub away: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct BetStackLineEntry {
    pub id: Option<i64>,
    pub event_id: Option<i64>,
    pub event: Option<BetStackLineEvent>,
    pub moneyline: Option<BetStackMoneyline>,
    pub last_updated: Option<String>,
    pub source: Option<String>,
}

impl BetStackLineEntry {
    pub fn home_team_name(&self) -> Option<&str> {
        self.event.as_ref()?.home_team.as_deref()
    }
    pub fn away_team_name(&self) -> Option<&str> {
        self.event.as_ref()?.away_team.as_deref()
    }
    pub fn league_key(&self) -> Option<&str> {
        self.event.as_ref()?.league.as_ref()?.key.as_deref()
    }
    pub fn commence_time(&self) -> Option<&str> {
        self.event.as_ref()?.commence_time.as_deref()
    }
    pub fn ml_home(&self) -> Option<f64> {
        self.moneyline.as_ref()?.home.as_ref()?.parse::<f64>().ok()
    }
    pub fn ml_away(&self) -> Option<f64> {
        self.moneyline.as_ref()?.away.as_ref()?.parse::<f64>().ok()
    }
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

/// Convert a BetStack line entry into VegasOdds (not yet matched to a canonical event).
pub fn line_entry_to_vegas_odds(entry: &BetStackLineEntry) -> Option<VegasOdds> {
    let ml_home = entry.ml_home()?;
    let ml_away = entry.ml_away()?;
    let home = entry.home_team_name()?;
    let away = entry.away_team_name()?;

    let implied_home = moneyline_to_implied_prob(ml_home);
    let implied_away = moneyline_to_implied_prob(ml_away);
    let (fair_home, fair_away) = devig_multiplicative(implied_home, implied_away);

    let mut outcomes = HashMap::new();
    outcomes.insert(
        home.to_string(),
        VegasOutcomeOdds {
            consensus_moneyline: ml_home,
            implied_prob: implied_home,
            fair_prob: fair_home,
            num_books: 1,
        },
    );
    outcomes.insert(
        away.to_string(),
        VegasOutcomeOdds {
            consensus_moneyline: ml_away,
            implied_prob: implied_away,
            fair_prob: fair_away,
            num_books: 1,
        },
    );

    Some(VegasOdds {
        canonical_event_id: String::new(), // populated by matcher
        outcomes,
        updated_at: Utc::now(),
    })
}

/// Background loop: polls BetStack /lines endpoint for consensus odds.
/// A single call returns moneylines for all leagues, respecting the 60s rate limit.
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
        match fetch_all_lines(&client, &api_key).await {
            Ok(lines) => {
                let total = lines.len();
                let mut matched = 0usize;
                let mut with_ml = 0usize;
                let mut by_league: HashMap<String, usize> = HashMap::new();

                for line in &lines {
                    let league_key = line.league_key().unwrap_or("unknown");
                    let sport = match league_key_to_sport(league_key) {
                        Some(s) => s,
                        None => continue,
                    };

                    if let Some(vegas_odds) = line_entry_to_vegas_odds(line) {
                        with_ml += 1;
                        if vegas_matcher::match_and_store_from_line(&state, line, vegas_odds, &sport) {
                            matched += 1;
                            *by_league.entry(league_key.to_string()).or_default() += 1;
                        }
                    }
                }

                info!(
                    "🎰 Vegas poll: {} lines fetched, {} with moneylines, {} matched to canonical events",
                    total, with_ml, matched
                );
                for (league, count) in &by_league {
                    info!("   └─ {}: {} matched", league, count);
                }
            }
            Err(e) => {
                warn!("BetStack /lines fetch failed: {}", e);
            }
        }

        tokio::time::sleep(interval).await;
    }
}

async fn fetch_all_lines(
    client: &Client,
    api_key: &str,
) -> anyhow::Result<Vec<BetStackLineEntry>> {
    let url = format!("{}/lines", BETSTACK_API_BASE);
    let resp = client
        .get(&url)
        .header("X-API-Key", api_key)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "BetStack API returned {}: {}",
            status,
            &body[..body.len().min(300)]
        );
    }

    let lines: Vec<BetStackLineEntry> = resp.json().await?;
    Ok(lines)
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

    #[test]
    fn test_league_key_mapping() {
        assert_eq!(league_key_to_sport("basketball_nba"), Some(Sport::Nba));
        assert_eq!(league_key_to_sport("americanfootball_nfl"), Some(Sport::Nfl));
        assert_eq!(league_key_to_sport("baseball_mlb"), Some(Sport::Mlb));
        assert_eq!(league_key_to_sport("icehockey_nhl"), Some(Sport::Nhl));
        assert_eq!(league_key_to_sport("soccer_epl"), None);
    }
}
