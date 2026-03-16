//! Cross-platform matching engine — scores and pairs Kalshi ↔ Polymarket
//! candidates using a multi-signal algorithm, then emits `CanonicalEvent`s.

use std::collections::HashMap;

use chrono::{NaiveDate, Utc};
use tracing::info;

use crate::models::event::{CanonicalEvent, PlatformIds, Sport};

use super::candidate::{self, CandidateEvent, Platform};

// ─── Scoring Weights ────────────────────────────────────────────────

const WEIGHT_BOTH_TEAMS: f64 = 50.0;
const WEIGHT_ONE_TEAM: f64 = 25.0;
const WEIGHT_TITLE_SIMILARITY: f64 = 20.0;
const WEIGHT_DATE_MATCH: f64 = 20.0;
const WEIGHT_MONEYLINE_MATCH: f64 = 10.0;

/// Minimum score to accept a cross-platform match.
const DEFAULT_MIN_SCORE: f64 = 60.0;

/// A scored pair of candidates.
struct ScoredPair {
    kalshi_idx: usize,
    poly_idx: usize,
    score: f64,
}

// ─── Public API ─────────────────────────────────────────────────────

/// Match Kalshi and Polymarket candidates, returning `CanonicalEvent`s.
///
/// Matched events contain both platforms' IDs. Unmatched events are returned
/// as single-platform `CanonicalEvent`s (same as culture_poller behavior).
pub fn match_candidates(
    kalshi_candidates: Vec<CandidateEvent>,
    poly_candidates: Vec<CandidateEvent>,
    min_score: Option<f64>,
) -> Vec<CanonicalEvent> {
    let min_score = min_score.unwrap_or(DEFAULT_MIN_SCORE);
    let now = Utc::now();

    // Bucket by (sport, date) for efficient matching
    let kalshi_buckets = bucket_by_sport_date(&kalshi_candidates);
    let poly_buckets = bucket_by_sport_date(&poly_candidates);

    let mut matched_kalshi: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut matched_poly: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut results: Vec<CanonicalEvent> = Vec::new();

    // For each bucket key that exists in both platforms, run the matcher
    for (key, kalshi_indices) in &kalshi_buckets {
        if let Some(poly_indices) = poly_buckets.get(key) {
            let matches = match_within_bucket(
                &kalshi_candidates,
                kalshi_indices,
                &poly_candidates,
                poly_indices,
                min_score,
            );

            for (ki, pi, score) in matches {
                let kalshi = &kalshi_candidates[ki];
                let poly = &poly_candidates[pi];

                let event = build_matched_event(kalshi, poly, score);
                info!(
                    "Matched: {} ↔ {} (score={:.0}, sport={}, date={:?})",
                    kalshi.raw_title,
                    poly.raw_title,
                    score,
                    kalshi.sport.as_str(),
                    kalshi.game_date,
                );

                results.push(event);
                matched_kalshi.insert(ki);
                matched_poly.insert(pi);
            }
        }
    }

    // Cross-bucket matching: try unmatched Kalshi events against all unmatched Poly events
    // with the same sport but possibly different/missing dates
    let unmatched_kalshi: Vec<usize> = (0..kalshi_candidates.len())
        .filter(|i| !matched_kalshi.contains(i))
        .collect();
    let unmatched_poly: Vec<usize> = (0..poly_candidates.len())
        .filter(|i| !matched_poly.contains(i))
        .collect();

    if !unmatched_kalshi.is_empty() && !unmatched_poly.is_empty() {
        // Group unmatched by sport only
        let mut kalshi_by_sport: HashMap<Sport, Vec<usize>> = HashMap::new();
        let mut poly_by_sport: HashMap<Sport, Vec<usize>> = HashMap::new();

        for &ki in &unmatched_kalshi {
            kalshi_by_sport
                .entry(kalshi_candidates[ki].sport)
                .or_default()
                .push(ki);
        }
        for &pi in &unmatched_poly {
            poly_by_sport
                .entry(poly_candidates[pi].sport)
                .or_default()
                .push(pi);
        }

        for (sport, ki_list) in &kalshi_by_sport {
            if let Some(pi_list) = poly_by_sport.get(sport) {
                let matches = match_within_bucket(
                    &kalshi_candidates,
                    ki_list,
                    &poly_candidates,
                    pi_list,
                    min_score,
                );

                for (ki, pi, score) in matches {
                    if matched_kalshi.contains(&ki) || matched_poly.contains(&pi) {
                        continue;
                    }
                    let kalshi = &kalshi_candidates[ki];
                    let poly = &poly_candidates[pi];

                    let event = build_matched_event(kalshi, poly, score);
                    info!(
                        "Cross-bucket match: {} ↔ {} (score={:.0})",
                        kalshi.raw_title, poly.raw_title, score,
                    );

                    results.push(event);
                    matched_kalshi.insert(ki);
                    matched_poly.insert(pi);
                }
            }
        }
    }

    // Emit single-platform events for anything still unmatched
    for (i, c) in kalshi_candidates.iter().enumerate() {
        if !matched_kalshi.contains(&i) {
            results.push(build_single_platform_event(c, &now));
        }
    }
    for (i, c) in poly_candidates.iter().enumerate() {
        if !matched_poly.contains(&i) {
            results.push(build_single_platform_event(c, &now));
        }
    }

    let matched_count = matched_kalshi.len();
    let total = results.len();
    info!(
        "Matching complete: {} matched pairs, {} single-platform, {} total events",
        matched_count,
        total - matched_count,
        total,
    );

    results
}

// ─── Bucketing ──────────────────────────────────────────────────────

type BucketKey = (Sport, Option<NaiveDate>);

fn bucket_by_sport_date(candidates: &[CandidateEvent]) -> HashMap<BucketKey, Vec<usize>> {
    let mut buckets: HashMap<BucketKey, Vec<usize>> = HashMap::new();
    for (i, c) in candidates.iter().enumerate() {
        buckets
            .entry((c.sport, c.game_date))
            .or_default()
            .push(i);
    }
    buckets
}

// ─── Within-Bucket Matching ─────────────────────────────────────────

/// Score all Kalshi×Poly pairs within a bucket and perform greedy assignment.
/// Returns (kalshi_global_idx, poly_global_idx, score) tuples.
fn match_within_bucket(
    kalshi_all: &[CandidateEvent],
    kalshi_indices: &[usize],
    poly_all: &[CandidateEvent],
    poly_indices: &[usize],
    min_score: f64,
) -> Vec<(usize, usize, f64)> {
    // Score every pair
    let mut scored: Vec<ScoredPair> = Vec::new();

    for &ki in kalshi_indices {
        for &pi in poly_indices {
            let score = score_pair(&kalshi_all[ki], &poly_all[pi]);
            if score >= min_score {
                scored.push(ScoredPair {
                    kalshi_idx: ki,
                    poly_idx: pi,
                    score,
                });
            }
        }
    }

    // Sort descending by score for greedy assignment
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Greedy bipartite: each event matches at most once
    let mut used_kalshi = std::collections::HashSet::new();
    let mut used_poly = std::collections::HashSet::new();
    let mut matches = Vec::new();

    for pair in scored {
        if used_kalshi.contains(&pair.kalshi_idx) || used_poly.contains(&pair.poly_idx) {
            continue;
        }
        used_kalshi.insert(pair.kalshi_idx);
        used_poly.insert(pair.poly_idx);
        matches.push((pair.kalshi_idx, pair.poly_idx, pair.score));
    }

    matches
}

// ─── Scoring ────────────────────────────────────────────────────────

fn score_pair(kalshi: &CandidateEvent, poly: &CandidateEvent) -> f64 {
    let mut score = 0.0;

    // Team matching (order-insensitive)
    let team_score = compute_team_score(kalshi, poly);
    score += team_score;

    // Title similarity
    let title_sim = candidate::jaccard_token_similarity(
        &kalshi.normalized_title,
        &poly.normalized_title,
    );
    score += title_sim * WEIGHT_TITLE_SIMILARITY;

    // Date match bonus (only when both have dates and they agree)
    if let (Some(kd), Some(pd)) = (kalshi.game_date, poly.game_date) {
        if kd == pd {
            score += WEIGHT_DATE_MATCH;
        }
    }

    // Moneyline match bonus
    if kalshi.is_moneyline && poly.is_moneyline {
        score += WEIGHT_MONEYLINE_MATCH;
    }

    score
}

fn compute_team_score(kalshi: &CandidateEvent, poly: &CandidateEvent) -> f64 {
    let ka = kalshi.team_a.as_deref();
    let kb = kalshi.team_b.as_deref();
    let pa = poly.team_a.as_deref();
    let pb = poly.team_b.as_deref();

    // If either side has no teams, return 0
    if (ka.is_none() && kb.is_none()) || (pa.is_none() && pb.is_none()) {
        return 0.0;
    }

    let kalshi_teams: Vec<&str> = [ka, kb].iter().filter_map(|t| *t).collect();
    let poly_teams: Vec<&str> = [pa, pb].iter().filter_map(|t| *t).collect();

    let mut matches = 0;
    for kt in &kalshi_teams {
        if poly_teams.iter().any(|pt| kt.eq_ignore_ascii_case(pt)) {
            matches += 1;
        }
    }

    match matches {
        0 => 0.0,
        1 => WEIGHT_ONE_TEAM,
        _ => WEIGHT_BOTH_TEAMS,
    }
}

// ─── Event Construction ─────────────────────────────────────────────

fn build_matched_event(
    kalshi: &CandidateEvent,
    poly: &CandidateEvent,
    _score: f64,
) -> CanonicalEvent {
    let now = Utc::now();

    // Build an event ID in the same format as DomeAPI: "sport-teamA-teamB-YYYY-MM-DD"
    let id = build_event_id(kalshi, poly);

    // Use the Kalshi title (more consistent format), with fallback to Poly
    let title = if !kalshi.raw_title.is_empty() {
        kalshi.raw_title.clone()
    } else {
        poly.raw_title.clone()
    };

    CanonicalEvent {
        id,
        sport: kalshi.sport,
        event_title: title,
        game_start_time: None,
        status: "open".to_string(),
        platform_ids: PlatformIds {
            kalshi_event_ticker: kalshi.kalshi_event_ticker.clone(),
            kalshi_market_tickers: kalshi.kalshi_market_tickers.clone(),
            polymarket_market_slug: poly.polymarket_slug.clone(),
            polymarket_token_ids: poly.polymarket_token_ids.clone(),
            polymarket_outcome_labels: poly.polymarket_outcome_labels.clone(),
        },
        created_at: now,
        updated_at: now,
    }
}

fn build_single_platform_event(
    candidate: &CandidateEvent,
    now: &chrono::DateTime<Utc>,
) -> CanonicalEvent {
    let id = match candidate.platform {
        Platform::Kalshi => format!(
            "kalshi-{}",
            candidate.kalshi_event_ticker.as_deref().unwrap_or("unknown")
        ),
        Platform::Polymarket => format!(
            "poly-{}",
            candidate.polymarket_slug.as_deref().unwrap_or("unknown")
        ),
    };

    CanonicalEvent {
        id,
        sport: candidate.sport,
        event_title: candidate.raw_title.clone(),
        game_start_time: None,
        status: "open".to_string(),
        platform_ids: PlatformIds {
            kalshi_event_ticker: candidate.kalshi_event_ticker.clone(),
            kalshi_market_tickers: candidate.kalshi_market_tickers.clone(),
            polymarket_market_slug: candidate.polymarket_slug.clone(),
            polymarket_token_ids: candidate.polymarket_token_ids.clone(),
            polymarket_outcome_labels: candidate.polymarket_outcome_labels.clone(),
        },
        created_at: *now,
        updated_at: *now,
    }
}

/// Build a canonical event ID matching DomeAPI's format: "sport-teamA-teamB-YYYY-MM-DD"
fn build_event_id(kalshi: &CandidateEvent, poly: &CandidateEvent) -> String {
    let sport = kalshi.sport.as_str();

    // Prefer Kalshi teams (more reliable abbreviations), then canonicalize ordering
    // so `bos-njd` and `njd-bos` produce the same event ID.
    let mut teams = vec![
        kalshi
        .team_a
        .as_ref()
        .or(poly.team_a.as_ref())
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "unk".to_string()),
        kalshi
        .team_b
        .as_ref()
        .or(poly.team_b.as_ref())
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "unk".to_string()),
    ];
    teams.sort();
    let team_a = teams[0].clone();
    let team_b = teams[1].clone();

    // Prefer Polymarket date (slug/date fields are often game-day aligned),
    // then fall back to Kalshi ticker date.
    let date = poly
        .game_date
        .or(kalshi.game_date)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "nodate".to_string());

    format!("{}-{}-{}-{}", sport, team_a, team_b, date)
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_kalshi(
        title: &str,
        sport: Sport,
        date: Option<NaiveDate>,
        team_a: Option<&str>,
        team_b: Option<&str>,
        tickers: Vec<&str>,
    ) -> CandidateEvent {
        CandidateEvent {
            platform: Platform::Kalshi,
            sport,
            raw_title: title.to_string(),
            normalized_title: candidate::normalize_title(title),
            game_date: date,
            team_a: team_a.map(|s| s.to_string()),
            team_b: team_b.map(|s| s.to_string()),
            is_moneyline: tickers.len() == 2,
            kalshi_event_ticker: Some("KXTEST".to_string()),
            kalshi_market_tickers: tickers.into_iter().map(|s| s.to_string()).collect(),
            polymarket_slug: None,
            polymarket_token_ids: vec![],
            polymarket_outcome_labels: vec![],
        }
    }

    fn make_poly(
        title: &str,
        sport: Sport,
        date: Option<NaiveDate>,
        team_a: Option<&str>,
        team_b: Option<&str>,
        slug: &str,
    ) -> CandidateEvent {
        CandidateEvent {
            platform: Platform::Polymarket,
            sport,
            raw_title: title.to_string(),
            normalized_title: candidate::normalize_title(title),
            game_date: date,
            team_a: team_a.map(|s| s.to_string()),
            team_b: team_b.map(|s| s.to_string()),
            is_moneyline: true,
            kalshi_event_ticker: None,
            kalshi_market_tickers: vec![],
            polymarket_slug: Some(slug.to_string()),
            polymarket_token_ids: vec!["tok1".to_string(), "tok2".to_string()],
            polymarket_outcome_labels: vec!["Team A".to_string(), "Team B".to_string()],
        }
    }

    #[test]
    fn test_both_teams_match_high_score() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 14);
        let k = make_kalshi("NBA: LAL vs BOS", Sport::Nba, date, Some("LAL"), Some("BOS"),
            vec!["KXNBA-LAL", "KXNBA-BOS"]);
        let p = make_poly("Los Angeles Lakers vs Boston Celtics", Sport::Nba, date,
            Some("LAL"), Some("BOS"), "lakers-celtics");

        let score = score_pair(&k, &p);
        // 50 (both teams) + 20 (date) + 10 (moneyline) + ~2.2 (title) ≈ 82
        assert!(score >= 80.0, "Both teams + date + moneyline should score >= 80, got {}", score);
    }

    #[test]
    fn test_one_team_match_partial_score() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 14);
        let k = make_kalshi("NBA: LAL vs BOS", Sport::Nba, date, Some("LAL"), Some("BOS"),
            vec!["KXNBA-LAL", "KXNBA-BOS"]);
        let p = make_poly("Los Angeles Lakers vs Miami Heat", Sport::Nba, date,
            Some("LAL"), Some("MIA"), "lakers-heat");

        let score = score_pair(&k, &p);
        assert!(score >= 25.0 && score < 80.0,
            "One team match should score 25-80, got {}", score);
    }

    #[test]
    fn test_no_team_match_low_score() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 14);
        let k = make_kalshi("NBA: LAL vs BOS", Sport::Nba, date, Some("LAL"), Some("BOS"),
            vec!["KXNBA-LAL", "KXNBA-BOS"]);
        let p = make_poly("Miami Heat vs Denver Nuggets", Sport::Nba, date,
            Some("MIA"), Some("DEN"), "heat-nuggets");

        let score = score_pair(&k, &p);
        assert!(score < 60.0, "No team match should score below threshold, got {}", score);
    }

    #[test]
    fn test_teams_order_insensitive() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 14);
        let k = make_kalshi("NBA: LAL vs BOS", Sport::Nba, date, Some("LAL"), Some("BOS"),
            vec!["KXNBA-LAL", "KXNBA-BOS"]);
        let p = make_poly("Boston Celtics vs Los Angeles Lakers", Sport::Nba, date,
            Some("BOS"), Some("LAL"), "celtics-lakers");

        let score = score_pair(&k, &p);
        assert!(score >= 80.0, "Reversed team order should still score high, got {}", score);
    }

    #[test]
    fn test_greedy_assignment_no_double_match() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 14);
        let kalshi = vec![
            make_kalshi("NBA: LAL vs BOS", Sport::Nba, date, Some("LAL"), Some("BOS"),
                vec!["KXNBA-LAL", "KXNBA-BOS"]),
            make_kalshi("NBA: MIA vs DEN", Sport::Nba, date, Some("MIA"), Some("DEN"),
                vec!["KXNBA-MIA", "KXNBA-DEN"]),
        ];
        let poly = vec![
            make_poly("Los Angeles Lakers vs Boston Celtics", Sport::Nba, date,
                Some("LAL"), Some("BOS"), "lakers-celtics"),
            make_poly("Miami Heat vs Denver Nuggets", Sport::Nba, date,
                Some("MIA"), Some("DEN"), "heat-nuggets"),
        ];

        let events = match_candidates(kalshi, poly, Some(60.0));

        // Should have exactly 2 matched events (not 4)
        let matched: Vec<_> = events.iter().filter(|e| {
            !e.platform_ids.kalshi_market_tickers.is_empty()
                && !e.platform_ids.polymarket_token_ids.is_empty()
        }).collect();
        assert_eq!(matched.len(), 2, "Expected 2 matched events, got {}", matched.len());
    }

    #[test]
    fn test_unmatched_become_single_platform() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 14);
        let kalshi = vec![
            make_kalshi("NBA: LAL vs BOS", Sport::Nba, date, Some("LAL"), Some("BOS"),
                vec!["KXNBA-LAL", "KXNBA-BOS"]),
        ];
        let poly = vec![
            make_poly("Miami Heat vs Denver Nuggets", Sport::Nba, date,
                Some("MIA"), Some("DEN"), "heat-nuggets"),
        ];

        let events = match_candidates(kalshi, poly, Some(60.0));

        let matched: Vec<_> = events.iter().filter(|e| {
            !e.platform_ids.kalshi_market_tickers.is_empty()
                && !e.platform_ids.polymarket_token_ids.is_empty()
        }).collect();
        let single: Vec<_> = events.iter().filter(|e| {
            e.platform_ids.kalshi_market_tickers.is_empty()
                || e.platform_ids.polymarket_token_ids.is_empty()
        }).collect();

        assert_eq!(matched.len(), 0, "No matches expected");
        assert_eq!(single.len(), 2, "Expected 2 single-platform events");
    }

    #[test]
    fn test_event_id_format() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 14);
        let k = make_kalshi("NBA: LAL vs BOS", Sport::Nba, date, Some("LAL"), Some("BOS"),
            vec!["KXNBA-LAL", "KXNBA-BOS"]);
        let p = make_poly("Lakers vs Celtics", Sport::Nba, date,
            Some("LAL"), Some("BOS"), "lakers-celtics");

        let id = build_event_id(&k, &p);
        assert_eq!(id, "nba-bos-lal-2026-03-14");
    }

    #[test]
    fn test_event_id_order_invariant() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 14);
        let k = make_kalshi("NBA: LAL vs BOS", Sport::Nba, date, Some("LAL"), Some("BOS"),
            vec!["KXNBA-LAL", "KXNBA-BOS"]);
        let p_forward = make_poly("Lakers vs Celtics", Sport::Nba, date,
            Some("LAL"), Some("BOS"), "lakers-celtics");
        let p_reversed = make_poly("Celtics vs Lakers", Sport::Nba, date,
            Some("BOS"), Some("LAL"), "celtics-lakers");

        let id_forward = build_event_id(&k, &p_forward);
        let id_reversed = build_event_id(&k, &p_reversed);
        assert_eq!(id_forward, id_reversed, "Event ID must not depend on team order");
    }

    #[test]
    fn test_event_id_prefers_polymarket_date() {
        let kalshi_date = NaiveDate::from_ymd_opt(2026, 3, 15);
        let poly_date = NaiveDate::from_ymd_opt(2026, 3, 16);
        let k = make_kalshi("NBA: LAL vs BOS", Sport::Nba, kalshi_date, Some("LAL"), Some("BOS"),
            vec!["KXNBAGAME-26MAR15LALBOS-LAL", "KXNBAGAME-26MAR15LALBOS-BOS"]);
        let p = make_poly("Lakers vs Celtics", Sport::Nba, poly_date,
            Some("LAL"), Some("BOS"), "nba-lal-bos-2026-03-16");

        let id = build_event_id(&k, &p);
        assert_eq!(id, "nba-bos-lal-2026-03-16");
    }

    #[test]
    fn test_cross_sport_no_match() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 14);
        let kalshi = vec![
            make_kalshi("NFL: ARI vs DEN", Sport::Nfl, date, Some("ARI"), Some("DEN"),
                vec!["KXNFL-ARI", "KXNFL-DEN"]),
        ];
        let poly = vec![
            make_poly("Denver Nuggets vs Phoenix Suns", Sport::Nba, date,
                Some("DEN"), Some("PHX"), "nuggets-suns"),
        ];

        let events = match_candidates(kalshi, poly, Some(60.0));
        let matched: Vec<_> = events.iter().filter(|e| {
            !e.platform_ids.kalshi_market_tickers.is_empty()
                && !e.platform_ids.polymarket_token_ids.is_empty()
        }).collect();
        assert_eq!(matched.len(), 0, "Different sports should not match");
    }

    // ── Real-world integration tests ─────────────────────────────

    #[test]
    fn test_nba_full_slate_matching() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 15);
        let kalshi = vec![
            make_kalshi("NBA: LAL vs BOS", Sport::Nba, date, Some("LAL"), Some("BOS"),
                vec!["KXNBAGAME-26MAR15LALBOS-LAL", "KXNBAGAME-26MAR15LALBOS-BOS"]),
            make_kalshi("NBA: MIA vs DEN", Sport::Nba, date, Some("MIA"), Some("DEN"),
                vec!["KXNBAGAME-26MAR15MIADEN-MIA", "KXNBAGAME-26MAR15MIADEN-DEN"]),
            make_kalshi("NBA: GSW vs PHX", Sport::Nba, date, Some("GSW"), Some("PHX"),
                vec!["KXNBAGAME-26MAR15GSWPHX-GSW", "KXNBAGAME-26MAR15GSWPHX-PHX"]),
            make_kalshi("NBA: NYK vs CHI", Sport::Nba, date, Some("NYK"), Some("CHI"),
                vec!["KXNBAGAME-26MAR15NYKCHI-NYK", "KXNBAGAME-26MAR15NYKCHI-CHI"]),
        ];
        let poly = vec![
            make_poly("Los Angeles Lakers vs Boston Celtics", Sport::Nba, date,
                Some("LAL"), Some("BOS"), "lakers-celtics"),
            make_poly("Miami Heat vs Denver Nuggets", Sport::Nba, date,
                Some("MIA"), Some("DEN"), "heat-nuggets"),
            make_poly("Golden State Warriors vs Phoenix Suns", Sport::Nba, date,
                Some("GSW"), Some("PHX"), "warriors-suns"),
            make_poly("New York Knicks vs Chicago Bulls", Sport::Nba, date,
                Some("NYK"), Some("CHI"), "knicks-bulls"),
        ];

        let events = match_candidates(kalshi, poly, Some(60.0));
        let matched: Vec<_> = events.iter().filter(|e| {
            !e.platform_ids.kalshi_market_tickers.is_empty()
                && !e.platform_ids.polymarket_token_ids.is_empty()
        }).collect();
        assert_eq!(matched.len(), 4, "All 4 NBA games should match, got {}", matched.len());
    }

    #[test]
    fn test_nhl_matching_with_date() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 15);
        let kalshi = vec![
            make_kalshi("NHL: TOR vs BOS", Sport::Nhl, date, Some("TOR"), Some("BOS"),
                vec!["KXNHLGAME-26MAR15TORBOS-TOR", "KXNHLGAME-26MAR15TORBOS-BOS"]),
            make_kalshi("NHL: VGK vs EDM", Sport::Nhl, date, Some("VGK"), Some("EDM"),
                vec!["KXNHLGAME-26MAR15VGKEDM-VGK", "KXNHLGAME-26MAR15VGKEDM-EDM"]),
        ];
        let poly = vec![
            make_poly("Toronto Maple Leafs vs Boston Bruins", Sport::Nhl, date,
                Some("TOR"), Some("BOS"), "leafs-bruins"),
            make_poly("Vegas Golden Knights vs Edmonton Oilers", Sport::Nhl, date,
                Some("VGK"), Some("EDM"), "knights-oilers"),
        ];

        let events = match_candidates(kalshi, poly, Some(60.0));
        let matched: Vec<_> = events.iter().filter(|e| {
            !e.platform_ids.kalshi_market_tickers.is_empty()
                && !e.platform_ids.polymarket_token_ids.is_empty()
        }).collect();
        assert_eq!(matched.len(), 2, "Both NHL games should match, got {}", matched.len());
    }

    #[test]
    fn test_cbb_matching() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 15);
        let kalshi = vec![
            make_kalshi("CBB: DUK vs UNC", Sport::Cbb, date, Some("DUK"), Some("UNC"),
                vec!["KXCBBGAME-26MAR15DUKUNC-DUK", "KXCBBGAME-26MAR15DUKUNC-UNC"]),
            make_kalshi("CBB: GONZ vs ALA", Sport::Cbb, date, Some("GONZ"), Some("ALA"),
                vec!["KXCBBGAME-26MAR15GONZALA-GONZ", "KXCBBGAME-26MAR15GONZALA-ALA"]),
        ];
        let poly = vec![
            make_poly("Duke vs North Carolina", Sport::Cbb, date,
                Some("DUK"), Some("UNC"), "duke-unc"),
            make_poly("Gonzaga Bulldogs vs Alabama Crimson Tide", Sport::Cbb, date,
                Some("GONZ"), Some("ALA"), "gonzaga-alabama"),
        ];

        let events = match_candidates(kalshi, poly, Some(60.0));
        let matched: Vec<_> = events.iter().filter(|e| {
            !e.platform_ids.kalshi_market_tickers.is_empty()
                && !e.platform_ids.polymarket_token_ids.is_empty()
        }).collect();
        assert_eq!(matched.len(), 2, "Both CBB games should match, got {}", matched.len());
    }

    #[test]
    fn test_cross_bucket_date_mismatch_still_matches() {
        // Polymarket has no date, Kalshi has date — should match via cross-bucket
        let date = NaiveDate::from_ymd_opt(2026, 3, 15);
        let kalshi = vec![
            make_kalshi("NBA: LAL vs BOS", Sport::Nba, date, Some("LAL"), Some("BOS"),
                vec!["KXNBAGAME-26MAR15LALBOS-LAL", "KXNBAGAME-26MAR15LALBOS-BOS"]),
        ];
        let poly = vec![
            make_poly("Los Angeles Lakers vs Boston Celtics", Sport::Nba, None,
                Some("LAL"), Some("BOS"), "lakers-celtics"),
        ];

        let events = match_candidates(kalshi, poly, Some(60.0));
        let matched: Vec<_> = events.iter().filter(|e| {
            !e.platform_ids.kalshi_market_tickers.is_empty()
                && !e.platform_ids.polymarket_token_ids.is_empty()
        }).collect();
        assert_eq!(matched.len(), 1, "Should match via cross-bucket fallback");
    }

    #[test]
    fn test_reversed_team_order_matches() {
        // Polymarket lists "Celtics vs Lakers" while Kalshi lists "LAL vs BOS"
        let date = NaiveDate::from_ymd_opt(2026, 3, 15);
        let kalshi = vec![
            make_kalshi("NBA: LAL vs BOS", Sport::Nba, date, Some("LAL"), Some("BOS"),
                vec!["KXNBAGAME-26MAR15LALBOS-LAL", "KXNBAGAME-26MAR15LALBOS-BOS"]),
        ];
        let poly = vec![
            make_poly("Boston Celtics vs Los Angeles Lakers", Sport::Nba, date,
                Some("BOS"), Some("LAL"), "celtics-lakers"),
        ];

        let events = match_candidates(kalshi, poly, Some(60.0));
        let matched: Vec<_> = events.iter().filter(|e| {
            !e.platform_ids.kalshi_market_tickers.is_empty()
                && !e.platform_ids.polymarket_token_ids.is_empty()
        }).collect();
        assert_eq!(matched.len(), 1, "Reversed team order should still match");
    }

    #[test]
    fn test_mixed_sports_correct_pairing() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 15);
        let kalshi = vec![
            make_kalshi("NBA: LAL vs BOS", Sport::Nba, date, Some("LAL"), Some("BOS"),
                vec!["KXNBAGAME-26MAR15LALBOS-LAL", "KXNBAGAME-26MAR15LALBOS-BOS"]),
            make_kalshi("NHL: TOR vs BOS", Sport::Nhl, date, Some("TOR"), Some("BOS"),
                vec!["KXNHLGAME-26MAR15TORBOS-TOR", "KXNHLGAME-26MAR15TORBOS-BOS"]),
        ];
        let poly = vec![
            make_poly("Toronto Maple Leafs vs Boston Bruins", Sport::Nhl, date,
                Some("TOR"), Some("BOS"), "leafs-bruins"),
            make_poly("Los Angeles Lakers vs Boston Celtics", Sport::Nba, date,
                Some("LAL"), Some("BOS"), "lakers-celtics"),
        ];

        let events = match_candidates(kalshi, poly, Some(60.0));
        let matched: Vec<_> = events.iter().filter(|e| {
            !e.platform_ids.kalshi_market_tickers.is_empty()
                && !e.platform_ids.polymarket_token_ids.is_empty()
        }).collect();
        assert_eq!(matched.len(), 2, "Both games should match to their correct sport");
        // NBA game should not match with NHL game
        for e in &matched {
            let has_nba_ticker = e.platform_ids.kalshi_market_tickers.iter().any(|t| t.contains("NBA"));
            let has_nhl_ticker = e.platform_ids.kalshi_market_tickers.iter().any(|t| t.contains("NHL"));
            let has_nba_slug = e.platform_ids.polymarket_market_slug.as_deref() == Some("lakers-celtics");
            let has_nhl_slug = e.platform_ids.polymarket_market_slug.as_deref() == Some("leafs-bruins");
            if has_nba_ticker {
                assert!(has_nba_slug, "NBA Kalshi should pair with NBA Polymarket");
            }
            if has_nhl_ticker {
                assert!(has_nhl_slug, "NHL Kalshi should pair with NHL Polymarket");
            }
        }
    }

    #[test]
    fn test_large_slate_no_false_matches() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 15);
        let kalshi = vec![
            make_kalshi("NBA: LAL vs BOS", Sport::Nba, date, Some("LAL"), Some("BOS"),
                vec!["KXNBA-LAL", "KXNBA-BOS"]),
            make_kalshi("NBA: MIA vs DEN", Sport::Nba, date, Some("MIA"), Some("DEN"),
                vec!["KXNBA-MIA", "KXNBA-DEN"]),
            make_kalshi("NBA: GSW vs PHX", Sport::Nba, date, Some("GSW"), Some("PHX"),
                vec!["KXNBA-GSW", "KXNBA-PHX"]),
            make_kalshi("NBA: NYK vs CHI", Sport::Nba, date, Some("NYK"), Some("CHI"),
                vec!["KXNBA-NYK", "KXNBA-CHI"]),
            make_kalshi("NBA: DAL vs MIL", Sport::Nba, date, Some("DAL"), Some("MIL"),
                vec!["KXNBA-DAL", "KXNBA-MIL"]),
        ];
        let poly = vec![
            make_poly("Lakers vs Celtics", Sport::Nba, date, Some("LAL"), Some("BOS"), "lakers-celtics"),
            make_poly("Heat vs Nuggets", Sport::Nba, date, Some("MIA"), Some("DEN"), "heat-nuggets"),
            make_poly("Warriors vs Suns", Sport::Nba, date, Some("GSW"), Some("PHX"), "warriors-suns"),
            make_poly("Knicks vs Bulls", Sport::Nba, date, Some("NYK"), Some("CHI"), "knicks-bulls"),
            make_poly("Mavericks vs Bucks", Sport::Nba, date, Some("DAL"), Some("MIL"), "mavs-bucks"),
        ];

        let events = match_candidates(kalshi, poly, Some(60.0));
        let matched: Vec<_> = events.iter().filter(|e| {
            !e.platform_ids.kalshi_market_tickers.is_empty()
                && !e.platform_ids.polymarket_token_ids.is_empty()
        }).collect();
        assert_eq!(matched.len(), 5, "All 5 NBA games should match correctly");
        // No single-platform events should remain
        let single: Vec<_> = events.iter().filter(|e| {
            e.platform_ids.kalshi_market_tickers.is_empty()
                || e.platform_ids.polymarket_token_ids.is_empty()
        }).collect();
        assert_eq!(single.len(), 0, "No unmatched events should remain");
    }
}
