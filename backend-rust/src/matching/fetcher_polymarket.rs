//! Polymarket sports event fetcher — fetches and normalizes sports events
//! from the Gamma API into `CandidateEvent` structs.

use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};
use chrono_tz::US::Eastern;

use crate::models::event::Sport;

use super::candidate::{
    self, CandidateEvent, MarketType, Platform,
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
    #[serde(rename = "endDate")]
    end_date: Option<String>,
    #[serde(rename = "startDate")]
    start_date: Option<String>,
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
    let mut all_events = Vec::new();
    let page_size = 100;
    let max_pages = 10;

    for page in 0..max_pages {
        let offset = page * page_size;
        let url = format!(
            "{}/events?tag={}&active=true&closed=false&limit={}&offset={}&order=volume24hr&ascending=false",
            POLYMARKET_GAMMA_BASE, tag, page_size, offset,
        );

        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("Gamma API returned {}", resp.status());
        }

        let events: Vec<GammaEvent> = resp.json().await?;
        let count = events.len();
        all_events.extend(events);

        if count < page_size {
            break;
        }
    }

    Ok(all_events)
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

        // Skip championship/futures events — they have >2 real outcomes
        // but can slip through as binary Yes/No markets
        if is_futures_event(&title) {
            continue;
        }

        // Extract token IDs and outcome labels from the moneyline market
        let (token_ids, mut outcome_labels, is_moneyline) = extract_moneyline_market(&event);

        // Extract teams from title
        let (team_a, team_b) = candidate::extract_teams_from_title(&title, sport);

        // Extract date: prefer end_date from Gamma API, fall back to title
        let game_date = parse_date_from_slug(&slug)
            .or_else(|| parse_gamma_date(event.end_date.as_deref()))
            .or_else(|| parse_gamma_date(event.start_date.as_deref()))
            .or_else(|| candidate::extract_date_from_title(&title));

        let game_start_time = parse_gamma_datetime(event.start_date.as_deref())
            .or_else(|| parse_gamma_datetime(event.end_date.as_deref()));

        let normalized_title = candidate::normalize_title(&title);

        // Emit moneyline candidate
        if !token_ids.is_empty() {
            // If outcome labels are still generic "Yes"/"No" (single-market fallback),
            // use title-derived team abbreviations so the WS ingester stores odds
            // under team names instead of Yes/No.
            let labels_are_generic = outcome_labels.iter().any(|l| {
                l.eq_ignore_ascii_case("yes") || l.eq_ignore_ascii_case("no")
            });
            if labels_are_generic {
                if let (Some(ref a), Some(ref b)) = (&team_a, &team_b) {
                    outcome_labels = vec![a.clone(), b.clone()];
                } else {
                    // Ambiguous Yes/No market with no resolvable teams in title.
                    // Don't emit a moneyline candidate for this event, but still
                    // check for spread/total below.
                    outcome_labels.clear();
                }
            }

            if !outcome_labels.is_empty() {
                // Normalize ALL outcome labels to canonical abbreviations
                outcome_labels = outcome_labels
                    .iter()
                    .map(|label| {
                        team_dictionary::lookup_team(label, Some(sport))
                            .unwrap_or_else(|| label.clone())
                    })
                    .collect();

                let market_type = if is_moneyline {
                    MarketType::Moneyline
                } else {
                    MarketType::Moneyline // conservative: treat unknown as moneyline
                };

                candidates.push(CandidateEvent {
                    platform: Platform::Polymarket,
                    sport,
                    raw_title: title.clone(),
                    normalized_title: normalized_title.clone(),
                    game_date,
                    game_start_time,
                    team_a: team_a.clone(),
                    team_b: team_b.clone(),
                    market_type,
                    kalshi_event_ticker: None,
                    kalshi_market_tickers: vec![],
                    polymarket_slug: Some(slug.clone()),
                    polymarket_token_ids: token_ids,
                    polymarket_outcome_labels: outcome_labels,
                });
            }
        }

        // Emit spread/total candidates from grouped markets
        let extra = extract_spread_total_markets(&event, sport);
        for (market_type, tids, labels) in extra {
            candidates.push(CandidateEvent {
                platform: Platform::Polymarket,
                sport,
                raw_title: title.clone(),
                normalized_title: normalized_title.clone(),
                game_date,
                game_start_time,
                team_a: team_a.clone(),
                team_b: team_b.clone(),
                market_type,
                kalshi_event_ticker: None,
                kalshi_market_tickers: vec![],
                polymarket_slug: Some(slug.clone()),
                polymarket_token_ids: tids,
                polymarket_outcome_labels: labels,
            });
        }
    }

    candidates
}

/// Classify a Polymarket event into a Sport based on its tags.
///
/// Two-pass approach: first collect all tag labels, then check specific tags
/// before generic ones (e.g. "ncaab" before "basketball") to avoid misclassification.
fn classify_sport_from_tags(event: &GammaEvent) -> Sport {
    let tags = match &event.tags {
        Some(t) => t,
        None => return Sport::Culture,
    };

    let labels: Vec<String> = tags
        .iter()
        .filter_map(|t| t.label.as_deref().or(t.slug.as_deref()).map(|s| s.to_lowercase()))
        .collect();

    // Pass 1: check specific/narrow tags first to prevent misclassification
    // (e.g. an NCAA Basketball event has both "basketball" and "ncaab" tags;
    //  we must match "ncaab" before "basketball")
    for label in &labels {
        match label.as_str() {
            "ncaaf" | "college football" | "cfb" | "ncaa football" => return Sport::Cfb,
            "ncaab" | "college basketball" | "cbb" | "ncaa basketball" | "ncaa" => return Sport::Cbb,
            "cwbb" => return Sport::Cbb,
            _ => {}
        }
    }

    // Pass 2: check broad sport tags
    for label in &labels {
        match label.as_str() {
            "nfl" | "football" => return Sport::Nfl,
            "nba" => return Sport::Nba,
            "mlb" | "baseball" => return Sport::Mlb,
            "nhl" | "hockey" => return Sport::Nhl,
            "pga" | "golf" => return Sport::Pga,
            "tennis" | "atp" | "wta" => return Sport::Tennis,
            "basketball" => return Sport::Nba,
            _ => {}
        }
    }

    // Pass 3: has a "sports" tag but no specific sport — try title
    if labels.iter().any(|l| l == "sports") {
        if let Some(title) = &event.title {
            let lower = title.to_lowercase();
            if lower.contains("ncaa") && lower.contains("football") { return Sport::Cfb; }
            if lower.contains("ncaa") && lower.contains("basketball") { return Sport::Cbb; }
            if lower.contains("nfl") || lower.contains("football") { return Sport::Nfl; }
            if lower.contains("nba") { return Sport::Nba; }
            if lower.contains("mlb") || lower.contains("baseball") { return Sport::Mlb; }
            if lower.contains("nhl") || lower.contains("hockey") { return Sport::Nhl; }
            if lower.contains("pga") || lower.contains("golf") { return Sport::Pga; }
            if lower.contains("tennis") { return Sport::Tennis; }
        }
    }

    Sport::Culture
}

/// Extract the moneyline/winner market tokens and labels from a Gamma event.
///
/// Handles two Polymarket structures:
/// 1. **Single market** with team-name outcomes: `outcomes=["Lakers","Celtics"]`
/// 2. **Grouped event** with per-team markets: each market has `groupItemTitle`
///    set to a team name and `outcomes=["Yes","No"]`. We extract the "Yes" token
///    from each team's market to reconstruct a moneyline view.
fn extract_moneyline_market(event: &GammaEvent) -> (Vec<String>, Vec<String>, bool) {
    let markets = match &event.markets {
        Some(m) => m,
        None => return (vec![], vec![], false),
    };

    let active_markets: Vec<&GammaMarket> = markets
        .iter()
        .filter(|m| !m.closed.unwrap_or(false) && m.active.unwrap_or(true))
        .collect();

    if active_markets.is_empty() {
        return (vec![], vec![], false);
    }

    // Strategy 1: single market with real team-name outcomes (not Yes/No)
    let direct_moneyline = active_markets.iter().find(|m| {
        let title = m.group_item_title.as_deref().unwrap_or("").to_lowercase();
        let is_moneyline_slot = title.is_empty()
            || title.contains("winner")
            || title.contains("moneyline");
        if !is_moneyline_slot {
            return false;
        }
        let outcomes: Vec<String> = m
            .outcomes
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        outcomes.len() == 2
            && !outcomes
                .iter()
                .any(|o| o.eq_ignore_ascii_case("yes") || o.eq_ignore_ascii_case("no"))
    });

    if let Some(market) = direct_moneyline {
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
        return (token_ids, outcome_labels, true);
    }

    // Strategy 2: grouped event — each market is a team with Yes/No outcomes.
    // Extract the "Yes" token from each team market.
    let team_markets: Vec<&&GammaMarket> = active_markets
        .iter()
        .filter(|m| {
            let title = m.group_item_title.as_deref().unwrap_or("");
            if title.is_empty() {
                return false;
            }
            let lower = title.to_lowercase();
            if lower.contains("winner") || lower.contains("moneyline") || lower.contains("spread")
                || lower.contains("over") || lower.contains("under") || lower.contains("total")
                || lower.contains("o/u")
            {
                return false;
            }
            let outcomes: Vec<String> = m
                .outcomes
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            outcomes.len() == 2
                && outcomes
                    .iter()
                    .any(|o| o.eq_ignore_ascii_case("yes"))
        })
        .collect();

    if team_markets.len() == 2 {
        let mut token_ids = Vec::new();
        let mut outcome_labels = Vec::new();

        for market in &team_markets {
            let tids: Vec<String> = market
                .clob_token_ids
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            let outcomes: Vec<String> = market
                .outcomes
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();

            let yes_idx = outcomes
                .iter()
                .position(|o| o.eq_ignore_ascii_case("yes"))
                .unwrap_or(0);
            if let Some(tid) = tids.get(yes_idx) {
                token_ids.push(tid.clone());
                outcome_labels
                    .push(market.group_item_title.clone().unwrap_or_default());
            }
        }

        if token_ids.len() == 2 {
            return (token_ids, outcome_labels, true);
        }
    }

    // Before Strategy 3 fallback: count how many team-like markets exist.
    // If >2, this is a multi-outcome event (e.g. "Who wins the NBA Championship")
    // that should NOT be treated as a binary market.
    let all_team_market_count = active_markets
        .iter()
        .filter(|m| {
            let title = m.group_item_title.as_deref().unwrap_or("");
            if title.is_empty() {
                return false;
            }
            let lower = title.to_lowercase();
            !(lower.contains("winner") || lower.contains("moneyline") || lower.contains("spread")
                || lower.contains("over") || lower.contains("under") || lower.contains("total")
                || lower.contains("o/u"))
        })
        .count();
    if all_team_market_count > 2 {
        return (vec![], vec![], false);
    }

    // Strategy 3: fallback — first active market (only for genuine binary events)
    let market = active_markets[0];
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
    if token_ids.len() != 2 || outcome_labels.len() != 2 {
        return (vec![], vec![], false);
    }

    let is_moneyline = {
        let title = market
            .group_item_title
            .as_deref()
            .unwrap_or("")
            .to_lowercase();
        title.is_empty() || title.contains("winner") || title.contains("moneyline")
    };

    (token_ids, outcome_labels, is_moneyline)
}

/// Extract spread and total (over/under) markets from a Polymarket grouped event.
///
/// Returns a Vec of `(MarketType, token_ids, outcome_labels)` for each
/// spread/total market found in the event's grouped sub-markets.
fn extract_spread_total_markets(
    event: &GammaEvent,
    sport: Sport,
) -> Vec<(MarketType, Vec<String>, Vec<String>)> {
    let markets = match &event.markets {
        Some(m) => m,
        None => return vec![],
    };

    let mut results = Vec::new();

    for market in markets {
        if market.closed.unwrap_or(false) || !market.active.unwrap_or(true) {
            continue;
        }

        let group_title = market.group_item_title.as_deref().unwrap_or("");
        let lower_title = group_title.to_lowercase();

        let market_type = if lower_title.contains("spread") {
            let line = extract_line_from_text(&lower_title);
            MarketType::Spread(line)
        } else if lower_title.contains("over") || lower_title.contains("under")
            || lower_title.contains("total") || lower_title.contains("o/u")
        {
            let line = extract_line_from_text(&lower_title);
            MarketType::Total(line)
        } else {
            continue;
        };

        let token_ids: Vec<String> = market
            .clob_token_ids
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        let mut outcome_labels: Vec<String> = market
            .outcomes
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        if token_ids.len() != 2 || outcome_labels.len() != 2 {
            continue;
        }

        // Normalize labels
        outcome_labels = outcome_labels
            .iter()
            .map(|label| {
                team_dictionary::lookup_team(label, Some(sport))
                    .unwrap_or_else(|| label.clone())
            })
            .collect();

        results.push((market_type, token_ids, outcome_labels));
    }

    results
}

/// Try to extract a numeric line from free text (e.g., "spread -3.5" → 3.5).
fn extract_line_from_text(text: &str) -> f64 {
    for word in text.split_whitespace() {
        let cleaned = word.trim_matches(|c: char| !c.is_ascii_digit() && c != '.' && c != '-');
        if let Ok(v) = cleaned.parse::<f64>() {
            return v;
        }
    }
    0.0
}

/// Returns true if the event title indicates a championship, award, or other
/// futures market that inherently has more than 2 real outcomes.
fn is_futures_event(title: &str) -> bool {
    let lower = title.to_lowercase();
    lower.contains("champion") || lower.contains("mvp")
        || lower.contains("win the 20") // "win the 2026 ..."
        || lower.contains("make the playoffs")
        || lower.contains("win the series")
        || lower.contains("award")
        || lower.contains("rookie of")
        || lower.contains("defensive player")
        || lower.contains("most valuable")
        || lower.contains("cy young")
        || lower.contains("heisman")
}

/// Parse an ISO-8601 datetime string from Gamma API into a NaiveDate.
/// Handles formats like "2026-03-15T00:00:00Z" and "2026-03-15".
fn parse_gamma_datetime(s: Option<&str>) -> Option<chrono::DateTime<chrono::Utc>> {
    let s = s?.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&chrono::Utc));
    }
    None
}

fn parse_gamma_date(s: Option<&str>) -> Option<chrono::NaiveDate> {
    let s = s?.trim();
    if s.is_empty() {
        return None;
    }
    // Try full ISO-8601 with time
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Eastern).date_naive());
    }
    // Try "YYYY-MM-DDT..." by taking first 10 chars
    if s.len() >= 10 {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d") {
            return Some(d);
        }
    }
    None
}

/// Extract YYYY-MM-DD date from the end of a polymarket slug such as:
/// `nba-cle-mil-2026-03-16` -> 2026-03-16
fn parse_date_from_slug(slug: &str) -> Option<chrono::NaiveDate> {
    let parts: Vec<&str> = slug.rsplitn(4, '-').collect();
    if parts.len() != 4 {
        return None;
    }
    // rsplitn gives: [DD, MM, YYYY, ...]
    let candidate = format!("{}-{}-{}", parts[2], parts[1], parts[0]);
    chrono::NaiveDate::parse_from_str(&candidate, "%Y-%m-%d").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gamma_date_rfc3339() {
        let d = parse_gamma_date(Some("2026-03-15T23:30:00Z"));
        assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2026, 3, 15));
    }

    #[test]
    fn test_parse_gamma_date_date_only() {
        let d = parse_gamma_date(Some("2026-03-15"));
        assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2026, 3, 15));
    }

    #[test]
    fn test_parse_gamma_date_with_offset() {
        let d = parse_gamma_date(Some("2026-03-15T19:00:00-04:00"));
        assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2026, 3, 15));
    }

    #[test]
    fn test_parse_gamma_date_uses_eastern_day_boundary() {
        // 00:30 UTC is still previous evening in US/Eastern.
        let d = parse_gamma_date(Some("2026-03-16T00:30:00Z"));
        assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2026, 3, 15));
    }

    #[test]
    fn test_parse_date_from_slug() {
        let d = parse_date_from_slug("nba-cle-mil-2026-03-16");
        assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2026, 3, 16));
        assert_eq!(parse_date_from_slug("nba-cle-mil"), None);
    }

    #[test]
    fn test_parse_gamma_date_none() {
        assert_eq!(parse_gamma_date(None), None);
        assert_eq!(parse_gamma_date(Some("")), None);
    }

    #[test]
    fn test_classify_sport_nba() {
        let event = GammaEvent {
            slug: Some("lakers-celtics".into()),
            title: Some("Lakers vs Celtics".into()),
            closed: Some(false),
            active: Some(true),
            volume: None,
            markets: None,
            tags: Some(vec![GammaTag {
                label: Some("NBA".into()),
                slug: Some("nba".into()),
            }]),
            end_date: None,
            start_date: None,
        };
        assert_eq!(classify_sport_from_tags(&event), Sport::Nba);
    }

    #[test]
    fn test_classify_sport_from_title_fallback() {
        let event = GammaEvent {
            slug: Some("lakers-celtics".into()),
            title: Some("NBA: Lakers vs Celtics".into()),
            closed: Some(false),
            active: Some(true),
            volume: None,
            markets: None,
            tags: Some(vec![GammaTag {
                label: Some("Sports".into()),
                slug: Some("sports".into()),
            }]),
            end_date: None,
            start_date: None,
        };
        assert_eq!(classify_sport_from_tags(&event), Sport::Nba);
    }

    // ── extract_moneyline_market tests ──────────────────────────

    fn make_market(
        group_title: Option<&str>,
        outcomes: &[&str],
        token_ids: &[&str],
    ) -> GammaMarket {
        GammaMarket {
            clob_token_ids: Some(serde_json::to_string(
                &token_ids.iter().map(|s| s.to_string()).collect::<Vec<_>>()
            ).unwrap()),
            outcomes: Some(serde_json::to_string(
                &outcomes.iter().map(|s| s.to_string()).collect::<Vec<_>>()
            ).unwrap()),
            outcome_prices: None,
            question: None,
            active: Some(true),
            closed: Some(false),
            group_item_title: group_title.map(|s| s.to_string()),
            enable_order_book: None,
        }
    }

    fn make_event_with_markets(markets: Vec<GammaMarket>) -> GammaEvent {
        GammaEvent {
            slug: Some("test-event".into()),
            title: Some("Test Event".into()),
            closed: Some(false),
            active: Some(true),
            volume: None,
            markets: Some(markets),
            tags: None,
            end_date: None,
            start_date: None,
        }
    }

    #[test]
    fn test_extract_direct_moneyline_team_names() {
        let event = make_event_with_markets(vec![
            make_market(None, &["Lakers", "Celtics"], &["tok_lal", "tok_bos"]),
        ]);
        let (ids, labels, is_ml) = extract_moneyline_market(&event);
        assert_eq!(ids, vec!["tok_lal", "tok_bos"]);
        assert_eq!(labels, vec!["Lakers", "Celtics"]);
        assert!(is_ml);
    }

    #[test]
    fn test_extract_grouped_per_team_markets() {
        let event = make_event_with_markets(vec![
            make_market(
                Some("Los Angeles Lakers"),
                &["Yes", "No"],
                &["tok_lal_yes", "tok_lal_no"],
            ),
            make_market(
                Some("Boston Celtics"),
                &["Yes", "No"],
                &["tok_bos_yes", "tok_bos_no"],
            ),
        ]);
        let (ids, labels, is_ml) = extract_moneyline_market(&event);
        assert_eq!(ids, vec!["tok_lal_yes", "tok_bos_yes"]);
        assert_eq!(labels, vec!["Los Angeles Lakers", "Boston Celtics"]);
        assert!(is_ml);
    }

    #[test]
    fn test_extract_grouped_ignores_spread_markets() {
        let event = make_event_with_markets(vec![
            make_market(
                Some("Los Angeles Lakers"),
                &["Yes", "No"],
                &["tok_lal_yes", "tok_lal_no"],
            ),
            make_market(
                Some("Boston Celtics"),
                &["Yes", "No"],
                &["tok_bos_yes", "tok_bos_no"],
            ),
            make_market(
                Some("Spread -3.5"),
                &["Yes", "No"],
                &["tok_spread_yes", "tok_spread_no"],
            ),
            make_market(
                Some("Over 215.5"),
                &["Yes", "No"],
                &["tok_over_yes", "tok_over_no"],
            ),
        ]);
        let (ids, labels, _is_ml) = extract_moneyline_market(&event);
        assert_eq!(ids.len(), 2);
        assert_eq!(labels, vec!["Los Angeles Lakers", "Boston Celtics"]);
    }

    #[test]
    fn test_extract_winner_market_with_team_outcomes() {
        let event = make_event_with_markets(vec![
            make_market(
                Some("Winner"),
                &["Los Angeles Lakers", "Boston Celtics"],
                &["tok_lal", "tok_bos"],
            ),
        ]);
        let (ids, labels, is_ml) = extract_moneyline_market(&event);
        assert_eq!(ids, vec!["tok_lal", "tok_bos"]);
        assert_eq!(labels, vec!["Los Angeles Lakers", "Boston Celtics"]);
        assert!(is_ml);
    }

    #[test]
    fn test_extract_single_yes_no_fallback() {
        // Single market with no groupItemTitle and Yes/No — falls through to fallback.
        let event = make_event_with_markets(vec![
            make_market(None, &["Yes", "No"], &["tok_yes", "tok_no"]),
        ]);
        let (ids, labels, is_ml) = extract_moneyline_market(&event);
        assert_eq!(ids, vec!["tok_yes", "tok_no"]);
        assert_eq!(labels, vec!["Yes", "No"]);
        assert!(is_ml);
    }

    #[test]
    fn test_extract_skips_closed_markets() {
        let mut closed_market = make_market(
            None,
            &["Lakers", "Celtics"],
            &["tok_old1", "tok_old2"],
        );
        closed_market.closed = Some(true);

        let event = make_event_with_markets(vec![
            closed_market,
            make_market(
                Some("Los Angeles Lakers"),
                &["Yes", "No"],
                &["tok_lal_yes", "tok_lal_no"],
            ),
            make_market(
                Some("Boston Celtics"),
                &["Yes", "No"],
                &["tok_bos_yes", "tok_bos_no"],
            ),
        ]);
        let (ids, labels, _) = extract_moneyline_market(&event);
        assert_eq!(ids, vec!["tok_lal_yes", "tok_bos_yes"]);
        assert_eq!(labels, vec!["Los Angeles Lakers", "Boston Celtics"]);
    }

    /// Realistic NBA event test: mirrors EXACT structure from the Gamma API
    /// for a real event like "Trail Blazers vs. 76ers" (slug: nba-por-phi-2026-03-15).
    /// Market 0 = moneyline with team names, Markets 1+ = spreads/O-U/player props.
    #[test]
    fn test_realistic_nba_event_structure() {
        let event = GammaEvent {
            slug: Some("nba-por-phi-2026-03-15".into()),
            title: Some("Trail Blazers vs. 76ers".into()),
            closed: Some(false),
            active: Some(true),
            volume: Some(150000.0),
            markets: Some(vec![
                // Market 0: moneyline with team names (the real structure)
                make_market(None, &["Trail Blazers", "76ers"], &["tok_por", "tok_phi"]),
                // Market 1: spread
                make_market(Some("Spread -10.5"), &["76ers", "Trail Blazers"], &["tok_s1", "tok_s2"]),
                // Market 2: O/U
                make_market(Some("O/U 214.5"), &["Over", "Under"], &["tok_o1", "tok_o2"]),
                // Market 3: player prop
                make_market(Some("VJ Edgecombe: Assists O/U 3.5"), &["Yes", "No"], &["tok_p1", "tok_p2"]),
                // Market 4: player prop
                make_market(Some("Cameron Payne: Assists O/U 3.5"), &["Yes", "No"], &["tok_p3", "tok_p4"]),
            ]),
            tags: Some(vec![
                GammaTag { label: Some("Sports".into()), slug: Some("sports".into()) },
                GammaTag { label: Some("NBA".into()), slug: Some("nba".into()) },
                GammaTag { label: Some("Games".into()), slug: Some("games".into()) },
                GammaTag { label: Some("Basketball".into()), slug: Some("basketball".into()) },
            ]),
            end_date: Some("2026-03-15T22:00:00Z".into()),
            start_date: Some("2026-03-09T14:03:29.983806Z".into()),
        };

        // Verify sport classification
        assert_eq!(classify_sport_from_tags(&event), Sport::Nba);

        // Verify moneyline extraction selects Market 0 with team names
        let (ids, labels, is_ml) = extract_moneyline_market(&event);
        assert_eq!(ids, vec!["tok_por", "tok_phi"]);
        assert_eq!(labels, vec!["Trail Blazers", "76ers"]);
        assert!(is_ml, "Should identify as moneyline");
    }

    /// Verify process_gamma_events produces correct CandidateEvent for a real NBA event.
    #[test]
    fn test_process_real_nba_event() {
        let event = GammaEvent {
            slug: Some("nba-gsw-nyk-2026-03-15".into()),
            title: Some("Warriors vs. Knicks".into()),
            closed: Some(false),
            active: Some(true),
            volume: Some(200000.0),
            markets: Some(vec![
                make_market(None, &["Warriors", "Knicks"], &["tok_gsw", "tok_nyk"]),
                make_market(Some("Spread -5.5"), &["Knicks", "Warriors"], &["tok_s1", "tok_s2"]),
                make_market(Some("O/U 222.5"), &["Over", "Under"], &["tok_o1", "tok_o2"]),
            ]),
            tags: Some(vec![
                GammaTag { label: Some("Sports".into()), slug: Some("sports".into()) },
                GammaTag { label: Some("NBA".into()), slug: Some("nba".into()) },
                GammaTag { label: Some("Games".into()), slug: Some("games".into()) },
                GammaTag { label: Some("Basketball".into()), slug: Some("basketball".into()) },
            ]),
            end_date: Some("2026-03-15T23:00:00Z".into()),
            start_date: None,
        };

        let sports = vec![Sport::Nba, Sport::Nhl];
        let candidates = process_gamma_events(vec![event], &sports);

        assert_eq!(candidates.len(), 3, "Should produce moneyline + spread + total");

        let ml = candidates.iter().find(|c| c.market_type.is_moneyline())
            .expect("moneyline candidate");
        assert_eq!(ml.sport, Sport::Nba);
        assert_eq!(ml.polymarket_slug.as_deref(), Some("nba-gsw-nyk-2026-03-15"));
        assert_eq!(ml.polymarket_token_ids, vec!["tok_gsw", "tok_nyk"]);
        assert_eq!(ml.polymarket_outcome_labels, vec!["GSW", "NYK"]);
        assert!(
            !ml.polymarket_outcome_labels.iter().any(|l|
                l.eq_ignore_ascii_case("yes") || l.eq_ignore_ascii_case("no")
            ),
            "Labels must not be Yes/No: {:?}",
            ml.polymarket_outcome_labels
        );

        let spread = candidates.iter().find(|c| matches!(c.market_type, MarketType::Spread(_)))
            .expect("spread candidate");
        assert_eq!(spread.polymarket_token_ids, vec!["tok_s1", "tok_s2"]);

        let total = candidates.iter().find(|c| matches!(c.market_type, MarketType::Total(_)))
            .expect("total candidate");
        assert_eq!(total.polymarket_token_ids, vec!["tok_o1", "tok_o2"]);
    }

    #[test]
    fn test_process_prefers_slug_date_when_present() {
        let event = GammaEvent {
            slug: Some("nba-cle-mil-2026-03-16".into()),
            title: Some("Cavaliers vs. Bucks".into()),
            closed: Some(false),
            active: Some(true),
            volume: None,
            markets: Some(vec![
                make_market(None, &["Cavaliers", "Bucks"], &["tok_cle", "tok_mil"]),
            ]),
            tags: Some(vec![
                GammaTag { label: Some("NBA".into()), slug: Some("nba".into()) },
            ]),
            // This UTC timestamp would map to 03/15 in Eastern, but slug carries 03/16.
            end_date: Some("2026-03-16T00:30:00Z".into()),
            start_date: None,
        };

        let sports = vec![Sport::Nba];
        let candidates = process_gamma_events(vec![event], &sports);
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].game_date,
            chrono::NaiveDate::from_ymd_opt(2026, 3, 16),
            "Slug date should win over timezone-shifted end_date"
        );
    }

    #[test]
    fn test_process_skips_generic_yes_no_without_resolved_teams() {
        let event = GammaEvent {
            slug: Some("nba-ambiguous-market".into()),
            title: Some("Will this happen?".into()),
            closed: Some(false),
            active: Some(true),
            volume: None,
            markets: Some(vec![
                make_market(None, &["Yes", "No"], &["tok_y", "tok_n"]),
            ]),
            tags: Some(vec![
                GammaTag { label: Some("NBA".into()), slug: Some("nba".into()) },
            ]),
            end_date: None,
            start_date: None,
        };

        let sports = vec![Sport::Nba];
        let candidates = process_gamma_events(vec![event], &sports);
        assert_eq!(candidates.len(), 0, "Ambiguous Yes/No markets should be skipped");
    }

    /// Verify that non-NBA events are filtered out when only NBA is requested.
    #[test]
    fn test_process_filters_non_sports() {
        let nba_event = GammaEvent {
            slug: Some("nba-lal-bos-2026-03-15".into()),
            title: Some("Lakers vs. Celtics".into()),
            closed: Some(false),
            active: Some(true),
            volume: None,
            markets: Some(vec![
                make_market(None, &["Lakers", "Celtics"], &["tok_a", "tok_b"]),
            ]),
            tags: Some(vec![
                GammaTag { label: Some("NBA".into()), slug: Some("nba".into()) },
            ]),
            end_date: None,
            start_date: None,
        };

        let politics_event = GammaEvent {
            slug: Some("fed-decision-in-march".into()),
            title: Some("Fed decision in March?".into()),
            closed: Some(false),
            active: Some(true),
            volume: None,
            markets: Some(vec![
                make_market(None, &["Yes", "No"], &["tok_y", "tok_n"]),
            ]),
            tags: Some(vec![
                GammaTag { label: Some("Economy".into()), slug: Some("economy".into()) },
            ]),
            end_date: None,
            start_date: None,
        };

        let sports = vec![Sport::Nba];
        let candidates = process_gamma_events(vec![nba_event, politics_event], &sports);

        assert_eq!(candidates.len(), 1, "Should only include the NBA event");
        assert_eq!(candidates[0].sport, Sport::Nba);
    }

    #[test]
    fn test_is_futures_event() {
        assert!(is_futures_event("Who will win the 2026 NBA Championship?"));
        assert!(is_futures_event("NBA MVP Award 2026"));
        assert!(is_futures_event("Will the Lakers make the playoffs?"));
        assert!(is_futures_event("Cy Young Award Winner"));
        assert!(is_futures_event("Heisman Trophy 2026"));
        assert!(!is_futures_event("Lakers vs. Celtics"));
        assert!(!is_futures_event("Warriors vs. Knicks"));
        assert!(!is_futures_event("Bruins vs. Devils"));
    }

    #[test]
    fn test_futures_event_filtered_out() {
        let futures_event = GammaEvent {
            slug: Some("nba-champion-2026".into()),
            title: Some("Who will win the 2026 NBA Championship?".into()),
            closed: Some(false),
            active: Some(true),
            volume: None,
            markets: Some(vec![
                make_market(Some("Los Angeles Lakers"), &["Yes", "No"], &["tok1_y", "tok1_n"]),
                make_market(Some("Boston Celtics"), &["Yes", "No"], &["tok2_y", "tok2_n"]),
                make_market(Some("Denver Nuggets"), &["Yes", "No"], &["tok3_y", "tok3_n"]),
                make_market(Some("Milwaukee Bucks"), &["Yes", "No"], &["tok4_y", "tok4_n"]),
            ]),
            tags: Some(vec![
                GammaTag { label: Some("NBA".into()), slug: Some("nba".into()) },
            ]),
            end_date: None,
            start_date: None,
        };

        let sports = vec![Sport::Nba];
        let candidates = process_gamma_events(vec![futures_event], &sports);
        assert_eq!(candidates.len(), 0, "Championship events must be filtered out");
    }

    #[test]
    fn test_multi_team_market_rejected_by_strategy3() {
        let multi_event = make_event_with_markets(vec![
            make_market(Some("Team A"), &["Yes", "No"], &["tok_a_y", "tok_a_n"]),
            make_market(Some("Team B"), &["Yes", "No"], &["tok_b_y", "tok_b_n"]),
            make_market(Some("Team C"), &["Yes", "No"], &["tok_c_y", "tok_c_n"]),
            make_market(Some("Team D"), &["Yes", "No"], &["tok_d_y", "tok_d_n"]),
        ]);
        let (ids, _labels, _is_ml) = extract_moneyline_market(&multi_event);
        assert!(ids.is_empty(), "Events with >2 team markets should be rejected");
    }

    #[test]
    fn test_outcome_labels_normalized_to_abbreviations() {
        let event = GammaEvent {
            slug: Some("nhl-bos-njd-2026-03-15".into()),
            title: Some("Bruins vs. Devils".into()),
            closed: Some(false),
            active: Some(true),
            volume: None,
            markets: Some(vec![
                make_market(None, &["Bruins", "Devils"], &["tok_bos", "tok_njd"]),
            ]),
            tags: Some(vec![
                GammaTag { label: Some("NHL".into()), slug: Some("nhl".into()) },
            ]),
            end_date: Some("2026-03-15T23:00:00Z".into()),
            start_date: None,
        };

        let sports = vec![Sport::Nhl];
        let candidates = process_gamma_events(vec![event], &sports);
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].polymarket_outcome_labels,
            vec!["BOS", "NJD"],
            "Outcome labels must be canonical abbreviations"
        );
    }
}
