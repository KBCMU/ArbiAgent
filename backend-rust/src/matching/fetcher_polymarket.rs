//! Polymarket sports event fetcher — fetches and normalizes sports events
//! from the Gamma API into `CandidateEvent` structs.

use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};

use crate::models::event::Sport;

use super::candidate::{
    self, CandidateEvent, Platform,
};
use super::team_dictionary;

const POLYMARKET_GAMMA_BASE: &str = "https://gamma-api.polymarket.com";
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

// ─── Gamma API Response Types ───────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GammaEvent {
    slug: Option<String>,
    title: Option<String>,
    closed: Option<bool>,
    active: Option<bool>,
    #[allow(dead_code)]
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
    #[serde(rename = "groupItemTitle")]
    group_item_title: Option<String>,
    #[serde(rename = "enableOrderBook")]
    enable_order_book: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GammaTag {
    label: Option<String>,
    slug: Option<String>,
}

// ─── Public API ─────────────────────────────────────────────────────

/// Fetch all sports events from Polymarket for the given sports, returning
/// normalized `CandidateEvent` structs ready for cross-platform matching.
pub async fn fetch_polymarket_sports_candidates(
    sports: &[Sport],
) -> Vec<CandidateEvent> {
    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("TLS backend unavailable");

    let mut all_candidates = Vec::new();

    // Fetch the general "sports" tag first (catches most events)
    match fetch_gamma_events_by_tag(&client, "sports").await {
        Ok(events) => {
            let candidates = process_gamma_events(events, sports);
            info!("Polymarket sports (general): {} candidates", candidates.len());
            all_candidates.extend(candidates);
        }
        Err(e) => {
            warn!("Polymarket Gamma fetch failed for 'sports' tag: {}", e);
        }
    }

    // Also fetch per-sport tags for broader coverage
    let mut seen_slugs: std::collections::HashSet<String> = all_candidates
        .iter()
        .filter_map(|c| c.polymarket_slug.clone())
        .collect();

    for &sport in sports {
        let tags = team_dictionary::sport_to_polymarket_tags(sport);
        for &tag in tags {
            match fetch_gamma_events_by_tag(&client, tag).await {
                Ok(events) => {
                    let candidates = process_gamma_events(events, sports);
                    let new_candidates: Vec<CandidateEvent> = candidates
                        .into_iter()
                        .filter(|c| {
                            c.polymarket_slug
                                .as_ref()
                                .map_or(true, |s| seen_slugs.insert(s.clone()))
                        })
                        .collect();
                    if !new_candidates.is_empty() {
                        info!(
                            "Polymarket {}/{}: {} new candidates",
                            sport.as_str(), tag, new_candidates.len()
                        );
                    }
                    all_candidates.extend(new_candidates);
                }
                Err(e) => {
                    warn!("Polymarket Gamma fetch failed for '{}' tag: {}", tag, e);
                }
            }
        }
    }

    all_candidates
}

// ─── Internal ───────────────────────────────────────────────────────

async fn fetch_gamma_events_by_tag(
    client: &Client,
    tag: &str,
) -> anyhow::Result<Vec<GammaEvent>> {
    let url = format!(
        "{}/events?tag={}&closed=false&limit=100&order=volume24hr&ascending=false",
        POLYMARKET_GAMMA_BASE, tag,
    );

    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Gamma API returned {}", resp.status());
    }

    let events: Vec<GammaEvent> = resp.json().await?;
    Ok(events)
}

/// Process a batch of Gamma events into CandidateEvents, filtering to the
/// requested sports and skipping closed/inactive events.
fn process_gamma_events(events: Vec<GammaEvent>, sports: &[Sport]) -> Vec<CandidateEvent> {
    let mut candidates = Vec::new();

    for event in events {
        if event.closed.unwrap_or(false) || !event.active.unwrap_or(true) {
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

        // Classify the sport from tags
        let sport = classify_sport_from_tags(&event);
        if !sports.contains(&sport) {
            continue;
        }

        // Extract token IDs and outcome labels from the moneyline market
        let (token_ids, outcome_labels, is_moneyline) = extract_moneyline_market(&event);
        if token_ids.is_empty() {
            continue;
        }

        // Extract teams from title
        let (team_a, team_b) = candidate::extract_teams_from_title(&title, sport);

        // Extract date from title
        let game_date = candidate::extract_date_from_title(&title);

        let normalized_title = candidate::normalize_title(&title);

        candidates.push(CandidateEvent {
            platform: Platform::Polymarket,
            sport,
            raw_title: title,
            normalized_title,
            game_date,
            team_a,
            team_b,
            is_moneyline,
            kalshi_event_ticker: None,
            kalshi_market_tickers: vec![],
            polymarket_slug: Some(slug),
            polymarket_token_ids: token_ids,
            polymarket_outcome_labels: outcome_labels,
        });
    }

    candidates
}

/// Classify a Polymarket event into a Sport based on its tags.
fn classify_sport_from_tags(event: &GammaEvent) -> Sport {
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
            "nfl" | "football" => return Sport::Nfl,
            "nba" | "basketball" => return Sport::Nba,
            "mlb" | "baseball" => return Sport::Mlb,
            "nhl" | "hockey" => return Sport::Nhl,
            "ncaaf" | "college football" | "cfb" => return Sport::Cfb,
            "ncaab" | "college basketball" | "cbb" => return Sport::Cbb,
            "pga" | "golf" => return Sport::Pga,
            "tennis" | "atp" | "wta" => return Sport::Tennis,
            _ => {}
        }

        // Partial match for "sports" tag — continue looking for a specific sport
        if label == "sports" {
            continue;
        }
    }

    // Has a "sports" tag but no specific sport — fall back to Culture
    // (the matcher can still try to match by title)
    if tags.iter().any(|t| {
        t.label.as_deref().or(t.slug.as_deref()).unwrap_or("").to_lowercase() == "sports"
    }) {
        // Generic sports — try to classify from title
        if let Some(title) = &event.title {
            let lower = title.to_lowercase();
            if lower.contains("nfl") || lower.contains("football") { return Sport::Nfl; }
            if lower.contains("nba") || lower.contains("basketball") { return Sport::Nba; }
            if lower.contains("mlb") || lower.contains("baseball") { return Sport::Mlb; }
            if lower.contains("nhl") || lower.contains("hockey") { return Sport::Nhl; }
            if lower.contains("ncaa") && lower.contains("football") { return Sport::Cfb; }
            if lower.contains("ncaa") && lower.contains("basketball") { return Sport::Cbb; }
            if lower.contains("pga") || lower.contains("golf") { return Sport::Pga; }
            if lower.contains("tennis") { return Sport::Tennis; }
        }
    }

    Sport::Culture
}

/// Extract the moneyline/winner market tokens and labels from a Gamma event.
///
/// Moneyline markets typically have `groupItemTitle` that is None, "Winner",
/// or "Moneyline". We prefer these over spread/O-U/prop markets.
fn extract_moneyline_market(event: &GammaEvent) -> (Vec<String>, Vec<String>, bool) {
    let markets = match &event.markets {
        Some(m) => m,
        None => return (vec![], vec![], false),
    };

    // First pass: find the moneyline market
    let moneyline = markets.iter().find(|m| {
        if m.closed.unwrap_or(false) || !m.active.unwrap_or(true) {
            return false;
        }
        let title = m.group_item_title.as_deref().unwrap_or("").to_lowercase();
        title.is_empty() || title.contains("winner") || title.contains("moneyline")
    });

    // Fallback: first active market
    let market = moneyline.or_else(|| {
        markets.iter().find(|m| {
            !m.closed.unwrap_or(false) && m.active.unwrap_or(true)
        })
    });

    let market = match market {
        Some(m) => m,
        None => return (vec![], vec![], false),
    };

    let is_moneyline = moneyline.is_some();

    // Parse JSON-encoded arrays
    let token_ids: Vec<String> = market
        .clob_token_ids
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let outcome_labels: Vec<String> = market
        .outcomes
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    (token_ids, outcome_labels, is_moneyline)
}
