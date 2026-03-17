//! CandidateEvent: platform-agnostic normalized representation of a sports
//! event, used as input to the cross-platform matching algorithm.

use chrono::{DateTime, NaiveDate, Utc};

use crate::models::event::Sport;

/// Which platform originated this candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Kalshi,
    Polymarket,
}

/// The type of market for a sports event.
#[derive(Debug, Clone, PartialEq)]
pub enum MarketType {
    /// Winner / moneyline (e.g., "Who will win LAL vs BOS?")
    Moneyline,
    /// Point spread (e.g., "LAL -3.5")
    Spread(f64),
    /// Over/Under total points (e.g., "O/U 215.5")
    Total(f64),
}

impl MarketType {
    /// Bucket-level discriminant: two candidates must share the same
    /// variant *and* the same line (for Spread/Total) to be paired.
    pub fn bucket_key(&self) -> MarketTypeBucket {
        match self {
            MarketType::Moneyline => MarketTypeBucket::Moneyline,
            MarketType::Spread(line) => MarketTypeBucket::Spread(ordered_f64(*line)),
            MarketType::Total(line) => MarketTypeBucket::Total(ordered_f64(*line)),
        }
    }

    #[allow(dead_code)]
    pub fn is_moneyline(&self) -> bool {
        matches!(self, MarketType::Moneyline)
    }
}

/// Hashable bucket key for `MarketType`, where the f64 line is
/// converted to an ordered integer (millicents) for `Hash`/`Eq`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarketTypeBucket {
    Moneyline,
    Spread(i64),
    Total(i64),
}

/// Convert a float line to a hashable integer (multiply by 10 to preserve half-points).
fn ordered_f64(v: f64) -> i64 {
    (v * 10.0).round() as i64
}

/// A normalized sports event from a single platform, ready for matching.
#[derive(Debug, Clone)]
pub struct CandidateEvent {
    pub platform: Platform,
    pub sport: Sport,
    pub raw_title: String,
    pub normalized_title: String,
    pub game_date: Option<NaiveDate>,
    /// Full start time (UTC) when available from platform APIs.
    pub game_start_time: Option<DateTime<Utc>>,
    /// Canonical team abbreviation (e.g., "LAL", "BOS").
    pub team_a: Option<String>,
    pub team_b: Option<String>,
    /// The type of market this candidate represents.
    pub market_type: MarketType,

    // ── Kalshi-specific fields ──────────────────────────────────────
    pub kalshi_event_ticker: Option<String>,
    pub kalshi_market_tickers: Vec<String>,

    // ── Polymarket-specific fields ──────────────────────────────────
    pub polymarket_slug: Option<String>,
    pub polymarket_token_ids: Vec<String>,
    pub polymarket_outcome_labels: Vec<String>,
}

/// Normalize a title for comparison: lowercase, strip punctuation, collapse whitespace.
pub fn normalize_title(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract a date from a Kalshi event ticker.
///
/// Kalshi market tickers encode dates like `KXNFLGAME-25AUG16ARIDEN-ARI`
/// where `25AUG16` = August 16, 2025. The event ticker itself is like
/// `KXNFLGAME-25AUG16ARIDEN`. We parse the first market ticker's date segment.
pub fn extract_date_from_kalshi_ticker(ticker: &str) -> Option<NaiveDate> {
    // Split by '-'; the date is in the second segment: "25AUG16ARIDEN"
    let parts: Vec<&str> = ticker.split('-').collect();
    if parts.len() < 2 {
        return None;
    }
    let seg = parts[1];
    // Extract leading digits (year suffix), then 3-letter month, then 2 digits (day)
    if seg.len() < 7 {
        return None;
    }

    let year_suffix = &seg[..2];
    let month_str = &seg[2..5];
    let day_str = &seg[5..7];

    let year: i32 = year_suffix.parse::<i32>().ok()? + 2000;
    let month: u32 = match month_str.to_uppercase().as_str() {
        "JAN" => 1,  "FEB" => 2,  "MAR" => 3,  "APR" => 4,
        "MAY" => 5,  "JUN" => 6,  "JUL" => 7,  "AUG" => 8,
        "SEP" => 9,  "OCT" => 10, "NOV" => 11, "DEC" => 12,
        _ => return None,
    };
    let day: u32 = day_str.parse().ok()?;

    NaiveDate::from_ymd_opt(year, month, day)
}

/// Extract a date from an event title string.
///
/// Looks for patterns like "(2025-08-16)" or "2025-08-16" in the title.
pub fn extract_date_from_title(title: &str) -> Option<NaiveDate> {
    // Try YYYY-MM-DD anywhere in the string
    for window_start in 0..title.len().saturating_sub(9) {
        let candidate = &title[window_start..window_start + 10];
        if let Ok(date) = NaiveDate::parse_from_str(candidate, "%Y-%m-%d") {
            return Some(date);
        }
    }

    // Try MM/DD/YYYY
    for window_start in 0..title.len().saturating_sub(9) {
        let candidate = &title[window_start..window_start + 10];
        if let Ok(date) = NaiveDate::parse_from_str(candidate, "%m/%d/%Y") {
            return Some(date);
        }
    }

    None
}

/// Extract team abbreviations from a Kalshi event ticker.
///
/// Ticker pattern: `KXNFLGAME-25AUG16ARIDEN-ARI`
/// The date segment encodes both teams after the date digits: "25AUG16**ARI**DEN**"
/// Individual market tickers have the outcome (team) as the last segment.
pub fn extract_teams_from_kalshi_tickers(market_tickers: &[String]) -> (Option<String>, Option<String>) {
    let teams: Vec<String> = market_tickers
        .iter()
        .map(|t| super::team_dictionary::extract_kalshi_outcome(t))
        .collect();

    match teams.len() {
        0 => (None, None),
        1 => (Some(teams[0].clone()), None),
        _ => (Some(teams[0].clone()), Some(teams[1].clone())),
    }
}

/// Extract team names from a title like "Los Angeles Lakers vs Boston Celtics"
/// or "NBA: LAL vs BOS (2025-03-14)" using the team dictionary.
///
/// Also handles Polymarket-style titles like "Will the Lakers beat the Celtics?"
/// by recognizing "beat", "defeat", "over", "cover" as team separators.
pub fn extract_teams_from_title(title: &str, sport: Sport) -> (Option<String>, Option<String>) {
    let lower = title.to_lowercase();

    // Strip common Polymarket preambles so separator detection works
    let cleaned = lower
        .replace("will the ", "")
        .replace("will ", "")
        .replace("who will win ", "")
        .replace("who wins ", "");
    let cleaned = cleaned.trim();

    // Split on "vs", "v.", "at", "@", or Polymarket-style separators
    let separators = [" vs. ", " vs ", " v. ", " at ", " @ ", " beat ", " beats ", " defeat ", " defeats ", " over "];
    for sep in separators {
        if let Some(idx) = cleaned.find(sep) {
            let left = cleaned[..idx].trim();
            let right = cleaned[idx + sep.len()..].trim();
            // Strip trailing punctuation (e.g., "?")
            let right = right.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-');

            let team_a = super::team_dictionary::lookup_team(left, Some(sport));
            let team_b = super::team_dictionary::lookup_team(right, Some(sport));

            if team_a.is_some() || team_b.is_some() {
                return (team_a, team_b);
            }
        }
    }

    // Also try on the original (un-cleaned) lowercase
    for sep in separators {
        if let Some(idx) = lower.find(sep) {
            let left = lower[..idx].trim();
            let right = lower[idx + sep.len()..].trim();
            let right = right.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-');

            let team_a = super::team_dictionary::lookup_team(left, Some(sport));
            let team_b = super::team_dictionary::lookup_team(right, Some(sport));

            if team_a.is_some() || team_b.is_some() {
                return (team_a, team_b);
            }
        }
    }

    // Fallback: try to find any two team names in the full title
    let mut found: Vec<String> = Vec::new();
    let tokens: Vec<&str> = lower.split_whitespace().collect();

    // Try multi-word combinations (longest first)
    for window_size in (1..=5).rev() {
        for window in tokens.windows(window_size) {
            let phrase = window.join(" ");
            if let Some(abbrev) = super::team_dictionary::lookup_team(&phrase, Some(sport)) {
                if !found.contains(&abbrev) {
                    found.push(abbrev);
                    if found.len() == 2 {
                        return (Some(found[0].clone()), Some(found[1].clone()));
                    }
                }
            }
        }
    }

    match found.len() {
        0 => (None, None),
        1 => (Some(found[0].clone()), None),
        _ => (Some(found[0].clone()), Some(found[1].clone())),
    }
}

/// Compute Jaccard similarity between two normalized title strings (token-level).
pub fn jaccard_token_similarity(a: &str, b: &str) -> f64 {
    let set_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let set_b: std::collections::HashSet<&str> = b.split_whitespace().collect();

    if set_a.is_empty() && set_b.is_empty() {
        return 1.0;
    }

    let intersection = set_a.intersection(&set_b).count() as f64;
    let union = set_a.union(&set_b).count() as f64;

    if union == 0.0 { 0.0 } else { intersection / union }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::event::Sport;

    #[test]
    fn test_normalize_title() {
        assert_eq!(normalize_title("NBA: LAL vs BOS (2025-03-14)"), "nba lal vs bos 2025 03 14");
        assert_eq!(normalize_title("  Multiple   Spaces  "), "multiple spaces");
    }

    #[test]
    fn test_extract_date_from_kalshi_ticker() {
        let date = extract_date_from_kalshi_ticker("KXNFLGAME-25AUG16ARIDEN-ARI");
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 8, 16));
    }

    #[test]
    fn test_extract_date_from_kalshi_ticker_march() {
        let date = extract_date_from_kalshi_ticker("KXNBAGAME-26MAR14LALBOS-LAL");
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 3, 14));
    }

    #[test]
    fn test_extract_date_from_title() {
        assert_eq!(
            extract_date_from_title("NBA: LAL vs BOS (2026-03-14)"),
            NaiveDate::from_ymd_opt(2026, 3, 14),
        );
        assert_eq!(extract_date_from_title("No date here"), None);
    }

    #[test]
    fn test_extract_teams_from_kalshi_tickers() {
        let tickers = vec![
            "KXNFLGAME-25AUG16ARIDEN-ARI".to_string(),
            "KXNFLGAME-25AUG16ARIDEN-DEN".to_string(),
        ];
        let (a, b) = extract_teams_from_kalshi_tickers(&tickers);
        assert_eq!(a.as_deref(), Some("ARI"));
        assert_eq!(b.as_deref(), Some("DEN"));
    }

    #[test]
    fn test_jaccard_similarity() {
        let sim = jaccard_token_similarity("nba lal vs bos", "nba lal vs bos");
        assert!((sim - 1.0).abs() < 0.001);

        let sim = jaccard_token_similarity("nba lal vs bos", "nba mia vs den");
        assert!(sim < 0.5, "Different teams should have low similarity: {}", sim);
    }

    #[test]
    fn test_extract_teams_from_title_vs_separator() {
        let (a, b) = extract_teams_from_title("Lakers vs Celtics", Sport::Nba);
        assert_eq!(a.as_deref(), Some("LAL"));
        assert_eq!(b.as_deref(), Some("BOS"));
    }

    #[test]
    fn test_extract_teams_from_title_full_names() {
        let (a, b) = extract_teams_from_title("Los Angeles Lakers vs Boston Celtics", Sport::Nba);
        assert_eq!(a.as_deref(), Some("LAL"));
        assert_eq!(b.as_deref(), Some("BOS"));
    }

    // ── Real-world Polymarket title formats ────────────────────────
    #[test]
    fn test_polymarket_will_beat_pattern() {
        let (a, b) = extract_teams_from_title("Will the Lakers beat the Celtics?", Sport::Nba);
        assert_eq!(a.as_deref(), Some("LAL"));
        assert_eq!(b.as_deref(), Some("BOS"));
    }

    #[test]
    fn test_polymarket_vs_dot_separator() {
        let (a, b) = extract_teams_from_title("Lakers vs. Celtics", Sport::Nba);
        assert_eq!(a.as_deref(), Some("LAL"));
        assert_eq!(b.as_deref(), Some("BOS"));
    }

    #[test]
    fn test_polymarket_nhl_matchup() {
        let (a, b) = extract_teams_from_title("Maple Leafs vs Bruins", Sport::Nhl);
        assert_eq!(a.as_deref(), Some("TOR"));
        assert_eq!(b.as_deref(), Some("BOS"));
    }

    #[test]
    fn test_polymarket_nhl_full_name() {
        let (a, b) = extract_teams_from_title("Toronto Maple Leafs vs Boston Bruins", Sport::Nhl);
        assert_eq!(a.as_deref(), Some("TOR"));
        assert_eq!(b.as_deref(), Some("BOS"));
    }

    #[test]
    fn test_cbb_duke_unc() {
        let (a, b) = extract_teams_from_title("Duke vs North Carolina", Sport::Cbb);
        assert_eq!(a.as_deref(), Some("DUK"));
        assert_eq!(b.as_deref(), Some("UNC"));
    }

    #[test]
    fn test_cbb_full_names() {
        let (a, b) = extract_teams_from_title("Duke Blue Devils vs UNC Tar Heels", Sport::Cbb);
        assert_eq!(a.as_deref(), Some("DUK"));
        assert_eq!(b.as_deref(), Some("UNC"));
    }

    #[test]
    fn test_cbb_gonzaga_vs_alabama() {
        let (a, b) = extract_teams_from_title("Gonzaga Bulldogs vs Alabama Crimson Tide", Sport::Cbb);
        assert_eq!(a.as_deref(), Some("GONZ"));
        assert_eq!(b.as_deref(), Some("ALA"));
    }

    #[test]
    fn test_nfl_full_names() {
        let (a, b) = extract_teams_from_title("Kansas City Chiefs vs San Francisco 49ers", Sport::Nfl);
        assert_eq!(a.as_deref(), Some("KC"));
        assert_eq!(b.as_deref(), Some("SF"));
    }

    #[test]
    fn test_mlb_matchup() {
        let (a, b) = extract_teams_from_title("New York Yankees vs Los Angeles Dodgers", Sport::Mlb);
        assert_eq!(a.as_deref(), Some("NYY"));
        assert_eq!(b.as_deref(), Some("LAD"));
    }

    #[test]
    fn test_nhl_golden_knights_vs_oilers() {
        let (a, b) = extract_teams_from_title("Vegas Golden Knights vs Edmonton Oilers", Sport::Nhl);
        assert_eq!(a.as_deref(), Some("VGK"));
        assert_eq!(b.as_deref(), Some("EDM"));
    }

    #[test]
    fn test_polymarket_at_separator() {
        let (a, b) = extract_teams_from_title("Lakers at Celtics", Sport::Nba);
        assert_eq!(a.as_deref(), Some("LAL"));
        assert_eq!(b.as_deref(), Some("BOS"));
    }

    #[test]
    fn test_title_with_date_suffix() {
        let (a, b) = extract_teams_from_title("Lakers vs Celtics (2026-03-15)", Sport::Nba);
        assert_eq!(a.as_deref(), Some("LAL"));
        assert_eq!(b.as_deref(), Some("BOS"));
    }

    #[test]
    fn test_kalshi_ticker_nba_date() {
        let date = extract_date_from_kalshi_ticker("KXNBAGAME-26MAR15LALBOS-LAL");
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 3, 15));
    }

    #[test]
    fn test_kalshi_ticker_nhl_date() {
        let date = extract_date_from_kalshi_ticker("KXNHLGAME-26MAR15TORTB-TOR");
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 3, 15));
    }

    #[test]
    fn test_kalshi_ticker_cbb_date() {
        let date = extract_date_from_kalshi_ticker("KXCBBGAME-26MAR15DUKUNC-DUK");
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 3, 15));
    }
}
