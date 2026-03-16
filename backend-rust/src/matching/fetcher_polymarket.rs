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
    let max_pages = 5;

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

        // Extract token IDs and outcome labels from the moneyline market
        let (token_ids, mut outcome_labels, is_moneyline) = extract_moneyline_market(&event);
        if token_ids.is_empty() {
            continue;
        }

        // Extract teams from title
        let (team_a, team_b) = candidate::extract_teams_from_title(&title, sport);

        // If outcome labels are still generic "Yes"/"No" (single-market fallback),
        // use title-derived team abbreviations so the WS ingester stores odds
        // under team names instead of Yes/No.
        let labels_are_generic = outcome_labels.iter().any(|l| {
            l.eq_ignore_ascii_case("yes") || l.eq_ignore_ascii_case("no")
        });
        if labels_are_generic {
            if let (Some(ref a), Some(ref b)) = (&team_a, &team_b) {
                outcome_labels = vec![a.clone(), b.clone()];
            }
        }

        // Extract date: prefer end_date from Gamma API, fall back to title
        let game_date = parse_gamma_date(event.end_date.as_deref())
            .or_else(|| parse_gamma_date(event.start_date.as_deref()))
            .or_else(|| candidate::extract_date_from_title(&title));

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

    // Strategy 3: fallback — first active market (original behavior)
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

/// Parse an ISO-8601 datetime string from Gamma API into a NaiveDate.
/// Handles formats like "2026-03-15T00:00:00Z" and "2026-03-15".
fn parse_gamma_date(s: Option<&str>) -> Option<chrono::NaiveDate> {
    let s = s?.trim();
    if s.is_empty() {
        return None;
    }
    // Try full ISO-8601 with time
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.date_naive());
    }
    // Try "YYYY-MM-DDT..." by taking first 10 chars
    if s.len() >= 10 {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d") {
            return Some(d);
        }
    }
    None
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
        let (ids, labels, is_ml) = extract_moneyline_market(&event);
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
        // Single market with no groupItemTitle and Yes/No — falls through to fallback
        let event = make_event_with_markets(vec![
            make_market(None, &["Yes", "No"], &["tok_yes", "tok_no"]),
        ]);
        let (ids, labels, _is_ml) = extract_moneyline_market(&event);
        assert_eq!(ids, vec!["tok_yes", "tok_no"]);
        assert_eq!(labels, vec!["Yes", "No"]);
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

        assert_eq!(candidates.len(), 1, "Should produce exactly one candidate");
        let c = &candidates[0];
        assert_eq!(c.sport, Sport::Nba);
        assert_eq!(c.polymarket_slug.as_deref(), Some("nba-gsw-nyk-2026-03-15"));
        assert_eq!(c.polymarket_token_ids, vec!["tok_gsw", "tok_nyk"]);
        assert_eq!(c.polymarket_outcome_labels, vec!["Warriors", "Knicks"]);
        assert!(c.is_moneyline);
        // Outcome labels should NOT be "Yes"/"No"
        assert!(
            !c.polymarket_outcome_labels.iter().any(|l|
                l.eq_ignore_ascii_case("yes") || l.eq_ignore_ascii_case("no")
            ),
            "Labels must not be Yes/No: {:?}",
            c.polymarket_outcome_labels
        );
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
}
