//! Cross-platform matching engine — scores and pairs Kalshi ↔ Polymarket
//! candidates using a multi-signal algorithm, then emits `CanonicalEvent`s.

use std::collections::HashMap;

use chrono::{NaiveDate, Utc};
use tracing::{debug, info, warn};

use crate::models::event::{CanonicalEvent, PlatformIds, Sport};

use super::candidate::{self, CandidateEvent, MarketTypeBucket, Platform};

// ─── Scoring Weights ────────────────────────────────────────────────

const WEIGHT_BOTH_TEAMS: f64 = 50.0;
const WEIGHT_ONE_TEAM: f64 = 25.0;
const WEIGHT_TITLE_SIMILARITY: f64 = 20.0;
const WEIGHT_DATE_MATCH: f64 = 20.0;
const WEIGHT_MONEYLINE_MATCH: f64 = 10.0;

/// Minimum score to accept a cross-platform match.
const DEFAULT_MIN_SCORE: f64 = 60.0;

/// A scored pair of candidates.
/// Detail about an unmatched candidate, including its best rejected cross-platform pair.
#[derive(Debug, Clone)]
pub struct UnmatchedDetail {
    pub event_title: String,
    pub platform: String,
    pub sport: String,
    pub best_rejected_score: Option<f64>,
    pub best_rejected_title: Option<String>,
}

/// Result of a matching cycle, including events and diagnostics.
pub struct MatchResult {
    pub events: Vec<CanonicalEvent>,
    pub matched_pairs: usize,
    pub unmatched_details: Vec<UnmatchedDetail>,
}

// ─── Public API ─────────────────────────────────────────────────────

/// Match Kalshi and Polymarket candidates, returning `CanonicalEvent`s
/// together with matching diagnostics.
///
/// Matched events contain both platforms' IDs. Unmatched events are returned
/// as single-platform `CanonicalEvent`s (same as culture_poller behavior).
pub fn match_candidates(
    kalshi_candidates: Vec<CandidateEvent>,
    poly_candidates: Vec<CandidateEvent>,
    min_score: Option<f64>,
) -> MatchResult {
    let min_score = min_score.unwrap_or(DEFAULT_MIN_SCORE);
    let now = Utc::now();

    // Log candidate summaries for diagnostics
    for c in &kalshi_candidates {
        debug!(
            "Kalshi candidate: sport={}, teams={:?}/{:?}, date={:?}, mkt={:?}, title={}",
            c.sport.as_str(), c.team_a, c.team_b, c.game_date, c.market_type, c.raw_title
        );
    }
    for c in &poly_candidates {
        debug!(
            "Poly candidate: sport={}, teams={:?}/{:?}, date={:?}, mkt={:?}, title={}",
            c.sport.as_str(), c.team_a, c.team_b, c.game_date, c.market_type, c.raw_title
        );
    }

    // Bucket by (sport, date) for efficient matching
    let kalshi_buckets = bucket_by_sport_date_market(&kalshi_candidates);
    let poly_buckets = bucket_by_sport_date_market(&poly_candidates);

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
        // Group unmatched by (sport, market_type) — dates may differ
        type FallbackKey = (Sport, MarketTypeBucket);
        let mut kalshi_by_key: HashMap<FallbackKey, Vec<usize>> = HashMap::new();
        let mut poly_by_key: HashMap<FallbackKey, Vec<usize>> = HashMap::new();

        for &ki in &unmatched_kalshi {
            let c = &kalshi_candidates[ki];
            kalshi_by_key
                .entry((c.sport, c.market_type.bucket_key()))
                .or_default()
                .push(ki);
        }
        for &pi in &unmatched_poly {
            let c = &poly_candidates[pi];
            poly_by_key
                .entry((c.sport, c.market_type.bucket_key()))
                .or_default()
                .push(pi);
        }

        for (key, ki_list) in &kalshi_by_key {
            if let Some(pi_list) = poly_by_key.get(key) {
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

    // Build unmatched diagnostics + emit single-platform events
    let mut unmatched_details: Vec<UnmatchedDetail> = Vec::new();

    for (i, c) in kalshi_candidates.iter().enumerate() {
        if !matched_kalshi.contains(&i) {
            results.push(build_single_platform_event(c, &now));

            let best = best_rejected_score(c, &poly_candidates, min_score);
            debug!(
                "Unmatched Kalshi: {} (teams={:?}/{:?}, date={:?}) best_cross={:?}",
                c.raw_title, c.team_a, c.team_b, c.game_date,
                best.as_ref().map(|(s, t)| format!("{:.0} vs {}", s, t)),
            );
            unmatched_details.push(UnmatchedDetail {
                event_title: c.raw_title.clone(),
                platform: "kalshi".to_string(),
                sport: c.sport.as_str().to_string(),
                best_rejected_score: best.as_ref().map(|(s, _)| *s),
                best_rejected_title: best.map(|(_, t)| t),
            });
        }
    }
    for (i, c) in poly_candidates.iter().enumerate() {
        if !matched_poly.contains(&i) {
            results.push(build_single_platform_event(c, &now));

            let best = best_rejected_score(c, &kalshi_candidates, min_score);
            debug!(
                "Unmatched Poly: {} (teams={:?}/{:?}, date={:?}) best_cross={:?}",
                c.raw_title, c.team_a, c.team_b, c.game_date,
                best.as_ref().map(|(s, t)| format!("{:.0} vs {}", s, t)),
            );
            unmatched_details.push(UnmatchedDetail {
                event_title: c.raw_title.clone(),
                platform: "polymarket".to_string(),
                sport: c.sport.as_str().to_string(),
                best_rejected_score: best.as_ref().map(|(s, _)| *s),
                best_rejected_title: best.map(|(_, t)| t),
            });
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

    MatchResult {
        events: results,
        matched_pairs: matched_count,
        unmatched_details,
    }
}

/// Find the highest-scoring cross-platform pair for an unmatched candidate,
/// even if that score fell below the threshold. Returns `None` if no
/// opposite-platform candidates exist for the same sport.
fn best_rejected_score(
    candidate: &CandidateEvent,
    opposites: &[CandidateEvent],
    _min_score: f64,
) -> Option<(f64, String)> {
    let mut best: Option<(f64, String)> = None;
    for opp in opposites {
        if opp.sport != candidate.sport {
            continue;
        }
        let s = score_pair(candidate, opp);
        if best.as_ref().map_or(true, |(bs, _)| s > *bs) {
            best = Some((s, opp.raw_title.clone()));
        }
    }
    best
}

// ─── Bucketing ──────────────────────────────────────────────────────

type BucketKey = (Sport, Option<NaiveDate>, MarketTypeBucket);

fn bucket_by_sport_date_market(candidates: &[CandidateEvent]) -> HashMap<BucketKey, Vec<usize>> {
    let mut buckets: HashMap<BucketKey, Vec<usize>> = HashMap::new();
    for (i, c) in candidates.iter().enumerate() {
        buckets
            .entry((c.sport, c.game_date, c.market_type.bucket_key()))
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
    if kalshi_indices.is_empty() || poly_indices.is_empty() {
        return vec![];
    }

    // Build weight matrix: weights[ki_local][pi_local] = score
    let weights: Vec<Vec<f64>> = kalshi_indices
        .iter()
        .map(|&ki| {
            poly_indices
                .iter()
                .map(|&pi| score_pair(&kalshi_all[ki], &poly_all[pi]))
                .collect()
        })
        .collect();

    // Optimal assignment via Hungarian algorithm
    let assigned = super::hungarian::max_weight_assignment(&weights, min_score);

    // Map local indices back to global indices
    assigned
        .into_iter()
        .map(|(ki_local, pi_local, score)| {
            (kalshi_indices[ki_local], poly_indices[pi_local], score)
        })
        .collect()
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

    // Date match bonus: full points for exact, 75% for ±1 day (timezone edge cases)
    if let (Some(kd), Some(pd)) = (kalshi.game_date, poly.game_date) {
        if kd == pd {
            score += WEIGHT_DATE_MATCH;
        } else if let Some(diff) = kd.signed_duration_since(pd).num_days().checked_abs() {
            if diff == 1 {
                score += WEIGHT_DATE_MATCH * 0.75;
            }
        }
    }

    // Market type match bonus (same variant = higher confidence in the match)
    if kalshi.market_type.bucket_key() == poly.market_type.bucket_key() {
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
    score: f64,
) -> CanonicalEvent {
    let now = Utc::now();

    let id = build_event_id(kalshi, poly);

    let title = if !kalshi.raw_title.is_empty() {
        kalshi.raw_title.clone()
    } else {
        poly.raw_title.clone()
    };

    let game_start_time = kalshi.game_start_time.or(poly.game_start_time);

    CanonicalEvent {
        id,
        sport: kalshi.sport,
        event_title: title,
        game_start_time,
        status: "open".to_string(),
        platform_ids: PlatformIds {
            kalshi_event_ticker: kalshi.kalshi_event_ticker.clone(),
            kalshi_market_tickers: kalshi.kalshi_market_tickers.clone(),
            polymarket_market_slug: poly.polymarket_slug.clone(),
            polymarket_token_ids: poly.polymarket_token_ids.clone(),
            polymarket_outcome_labels: poly.polymarket_outcome_labels.clone(),
        },
        match_score: Some(score),
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
        game_start_time: candidate.game_start_time,
        status: "open".to_string(),
        platform_ids: PlatformIds {
            kalshi_event_ticker: candidate.kalshi_event_ticker.clone(),
            kalshi_market_tickers: candidate.kalshi_market_tickers.clone(),
            polymarket_market_slug: candidate.polymarket_slug.clone(),
            polymarket_token_ids: candidate.polymarket_token_ids.clone(),
            polymarket_outcome_labels: candidate.polymarket_outcome_labels.clone(),
        },
        match_score: None,
        created_at: *now,
        updated_at: *now,
    }
}

/// Extract potential team abbreviation segments from a slug or ticker.
///
/// Keeps tokens that are 2-5 alphabetic characters and are not sport prefixes.
fn slug_team_segments(slug: &str) -> Vec<String> {
    let sport_prefixes = ["nba","nfl","nhl","mlb","mls","cfb","cbb","ncaab","ncaaf"];
    slug.split('-')
        .filter(|s| {
            let lower = s.to_lowercase();
            s.chars().all(|c| c.is_alphabetic())
                && s.len() >= 2
                && s.len() <= 5
                && !sport_prefixes.contains(&lower.as_str())
        })
        .map(|s| s.to_lowercase())
        .collect()
}

/// Scan a slug/ticker for an embedded YYYY-MM-DD date pattern.
fn slug_date(slug: &str) -> Option<NaiveDate> {
    let bytes = slug.as_bytes();
    for i in 0..bytes.len().saturating_sub(9) {
        if bytes[i+4] == b'-' && bytes[i+7] == b'-'
            && bytes[i..i+4].iter().all(|b| b.is_ascii_digit())
            && bytes[i+5..i+7].iter().all(|b| b.is_ascii_digit())
            && bytes[i+8..i+10].iter().all(|b| b.is_ascii_digit())
        {
            let y: i32 = slug[i..i+4].parse().ok()?;
            let m: u32 = slug[i+5..i+7].parse().ok()?;
            let d: u32 = slug[i+8..i+10].parse().ok()?;
            return NaiveDate::from_ymd_opt(y, m, d);
        }
    }
    None
}

/// Build a canonical event ID matching DomeAPI's format: "sport-teamA-teamB-YYYY-MM-DD"
fn build_event_id(kalshi: &CandidateEvent, poly: &CandidateEvent) -> String {
    let sport = kalshi.sport.as_str();

    // Extract teams: prefer explicit fields, fall back to slug/ticker segments
    let poly_slug = poly.polymarket_slug.as_deref().unwrap_or("");
    let kalshi_ticker = kalshi.kalshi_event_ticker.as_deref().unwrap_or("");

    let team_a = kalshi.team_a.as_ref()
        .or(poly.team_a.as_ref())
        .map(|s| s.to_lowercase())
        .or_else(|| {
            let segs = slug_team_segments(poly_slug);
            if segs.is_empty() { slug_team_segments(kalshi_ticker).into_iter().next() }
            else { segs.into_iter().next() }
        })
        .unwrap_or_else(|| {
            warn!("build_event_id: could not extract team_a from any source for ticker={} slug={}", kalshi_ticker, poly_slug);
            "unknown1".to_string()
        });

    let team_b = kalshi.team_b.as_ref()
        .or(poly.team_b.as_ref())
        .map(|s| s.to_lowercase())
        .or_else(|| {
            let segs = slug_team_segments(poly_slug);
            if segs.len() >= 2 { Some(segs[1].clone()) }
            else {
                let kalshi_segs = slug_team_segments(kalshi_ticker);
                if kalshi_segs.len() >= 2 { Some(kalshi_segs[1].clone()) } else { None }
            }
        })
        .unwrap_or_else(|| {
            warn!("build_event_id: could not extract team_b from any source for ticker={} slug={}", kalshi_ticker, poly_slug);
            "unknown2".to_string()
        });

    let mut teams = vec![team_a, team_b];
    teams.sort();

    // Extract date: prefer explicit game_date, fall back to slug/ticker parsing
    let date = poly.game_date.or(kalshi.game_date)
        .or_else(|| slug_date(poly_slug))
        .or_else(|| slug_date(kalshi_ticker))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| {
            warn!("build_event_id: could not extract date for ticker={} slug={}", kalshi_ticker, poly_slug);
            "undated".to_string()
        });

    format!("{}-{}-{}-{}", sport, teams[0], teams[1], date)
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::candidate::MarketType;
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
            game_start_time: None,
            team_a: team_a.map(|s| s.to_string()),
            team_b: team_b.map(|s| s.to_string()),
            market_type: MarketType::Moneyline,
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
            game_start_time: None,
            team_a: team_a.map(|s| s.to_string()),
            team_b: team_b.map(|s| s.to_string()),
            market_type: MarketType::Moneyline,
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

        let events = match_candidates(kalshi, poly, Some(60.0)).events;

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

        let events = match_candidates(kalshi, poly, Some(60.0)).events;

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

        let events = match_candidates(kalshi, poly, Some(60.0)).events;
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

        let events = match_candidates(kalshi, poly, Some(60.0)).events;
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

        let events = match_candidates(kalshi, poly, Some(60.0)).events;
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

        let events = match_candidates(kalshi, poly, Some(60.0)).events;
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

        let events = match_candidates(kalshi, poly, Some(60.0)).events;
        let matched: Vec<_> = events.iter().filter(|e| {
            !e.platform_ids.kalshi_market_tickers.is_empty()
                && !e.platform_ids.polymarket_token_ids.is_empty()
        }).collect();
        assert_eq!(matched.len(), 1, "Should match via cross-bucket fallback");
    }

    #[test]
    fn test_adjacent_day_date_tolerance_matches() {
        // Kalshi has date Mar 17, Polymarket has Mar 16 (timezone edge case)
        let kalshi_date = NaiveDate::from_ymd_opt(2026, 3, 17);
        let poly_date = NaiveDate::from_ymd_opt(2026, 3, 16);
        let kalshi = vec![
            make_kalshi("NBA: LAL vs HOU", Sport::Nba, kalshi_date, Some("LAL"), Some("HOU"),
                vec!["KXNBAGAME-26MAR17LALHOU-LAL", "KXNBAGAME-26MAR17LALHOU-HOU"]),
        ];
        let poly = vec![
            make_poly("Lakers vs. Rockets", Sport::Nba, poly_date,
                Some("LAL"), Some("HOU"), "nba-lal-hou-2026-03-16"),
        ];

        let events = match_candidates(kalshi, poly, Some(60.0)).events;
        let matched: Vec<_> = events.iter().filter(|e| {
            !e.platform_ids.kalshi_market_tickers.is_empty()
                && !e.platform_ids.polymarket_token_ids.is_empty()
        }).collect();
        assert_eq!(matched.len(), 1, "±1 day date difference should still match");
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

        let events = match_candidates(kalshi, poly, Some(60.0)).events;
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

        let events = match_candidates(kalshi, poly, Some(60.0)).events;
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
    fn test_event_id_no_teams_uses_slug_fallback() {
        let kalshi = make_kalshi(
            "NBA Lakers vs Celtics",
            Sport::Nba,
            None,
            None,
            None,
            vec!["KXNBA-26MAR14-LALBOS"],
        );
        let poly = make_poly(
            "NBA Lakers vs Celtics",
            Sport::Nba,
            Some(NaiveDate::from_ymd_opt(2026, 3, 14).unwrap()),
            None,
            None,
            "nba-lal-bos-2026-03-14",
        );
        let id = build_event_id(&kalshi, &poly);
        assert!(!id.contains("unk"), "ID contained 'unk': {}", id);
        // Stable: same inputs same output
        let id2 = build_event_id(&kalshi, &poly);
        assert_eq!(id, id2);
    }

    #[test]
    fn test_event_id_no_date_uses_slug_fallback() {
        let kalshi = make_kalshi(
            "NBA Lakers vs Celtics",
            Sport::Nba,
            None,
            None,
            None,
            vec!["KXNBA-LALBOS"],
        );
        let poly = make_poly(
            "NBA Lakers vs Celtics",
            Sport::Nba,
            None,
            None,
            None,
            "nba-lal-bos-2026-03-14",
        );
        let id = build_event_id(&kalshi, &poly);
        assert!(!id.contains("nodate"), "ID contained 'nodate': {}", id);
    }

    #[test]
    fn test_event_id_unk_never_appears() {
        let kalshi = make_kalshi(
            "NBA game",
            Sport::Nba,
            None,
            None,
            None,
            vec!["KXNBA-LALBOS"],
        );
        let poly = make_poly(
            "NBA game",
            Sport::Nba,
            Some(NaiveDate::from_ymd_opt(2026, 3, 14).unwrap()),
            None,
            None,
            "nba-lal-bos-2026-03-14",
        );
        let id = build_event_id(&kalshi, &poly);
        assert!(!id.contains("unk"), "ID contained 'unk': {}", id);
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

        let events = match_candidates(kalshi, poly, Some(60.0)).events;
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
