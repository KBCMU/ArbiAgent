use crate::matching::team_dictionary::lookup_team;
use crate::models::event::{CanonicalEvent, Sport};
use crate::models::vegas::VegasOdds;
use crate::AppState;

use super::vegas_poller::BetStackLineEntry;

/// Attempt to match a BetStack line entry to a canonical event in the cache
/// and store the resulting VegasOdds if a match is found.
///
/// Returns `true` if the event was matched and stored.
pub fn match_and_store_from_line(
    state: &AppState,
    line: &BetStackLineEntry,
    mut vegas_odds: VegasOdds,
    sport: &Sport,
) -> bool {
    let home = match line.home_team_name() {
        Some(h) if !h.is_empty() => h,
        _ => return false,
    };
    let away = match line.away_team_name() {
        Some(a) if !a.is_empty() => a,
        _ => return false,
    };

    let home_abbr = lookup_team(home, Some(*sport)).unwrap_or_else(|| normalize_fallback(home));
    let away_abbr = lookup_team(away, Some(*sport)).unwrap_or_else(|| normalize_fallback(away));

    let bs_date = line
        .commence_time()
        .and_then(|t| t.get(..10))
        .unwrap_or("");

    let mut best_match: Option<(String, f64)> = None;

    for entry in state.cache.events.iter() {
        let event: &CanonicalEvent = entry.value();
        if event.sport != *sport {
            continue;
        }

        let score = compute_match_score(event, &home_abbr, &away_abbr, bs_date);
        if score > 0.0 {
            if best_match.as_ref().map_or(true, |(_, s)| score > *s) {
                best_match = Some((event.id.clone(), score));
            }
        }
    }

    if let Some((event_id, _score)) = best_match {
        vegas_odds.canonical_event_id = event_id.clone();

        if let Some(event_ref) = state.cache.events.get(&event_id) {
            remap_outcome_keys(&mut vegas_odds, &event_ref, &home_abbr, &away_abbr);
        }

        state.cache.set_vegas_odds(&event_id, vegas_odds);
        return true;
    }

    false
}

/// Compute a match score between a BetStack event and a canonical event.
fn compute_match_score(
    event: &CanonicalEvent,
    home_abbr: &str,
    away_abbr: &str,
    bs_date: &str,
) -> f64 {
    let id_lower = event.id.to_lowercase();
    let home_lower = home_abbr.to_lowercase();
    let away_lower = away_abbr.to_lowercase();

    let home_in_id = id_lower.contains(&home_lower);
    let away_in_id = id_lower.contains(&away_lower);

    if !home_in_id || !away_in_id {
        return 0.0;
    }

    let mut score = 70.0;

    if !bs_date.is_empty() && id_lower.contains(bs_date) {
        score += 30.0;
    } else if !bs_date.is_empty() {
        if let Ok(bs_naive) = chrono::NaiveDate::parse_from_str(bs_date, "%Y-%m-%d") {
            let id_date = extract_date_from_id(&event.id);
            if let Some(id_naive) = id_date {
                let diff = (bs_naive - id_naive).num_days().abs();
                if diff <= 1 {
                    score += 20.0;
                } else {
                    return 0.0;
                }
            }
        }
    }

    score
}

fn extract_date_from_id(event_id: &str) -> Option<chrono::NaiveDate> {
    let parts: Vec<&str> = event_id.split('-').collect();
    if parts.len() < 4 {
        return None;
    }
    let date_str = format!(
        "{}-{}-{}",
        parts[parts.len() - 3],
        parts[parts.len() - 2],
        parts[parts.len() - 1],
    );
    chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok()
}

/// Remap VegasOdds outcome keys from BetStack team names to the
/// outcome names used in the canonical event's prediction market odds.
fn remap_outcome_keys(
    vegas_odds: &mut VegasOdds,
    event: &CanonicalEvent,
    home_abbr: &str,
    away_abbr: &str,
) {
    let labels = &event.platform_ids.polymarket_outcome_labels;
    if labels.len() != 2 {
        return;
    }

    let label_a = &labels[0];
    let label_b = &labels[1];

    let label_a_lower = label_a.to_lowercase();
    let label_b_lower = label_b.to_lowercase();
    let home_lower = home_abbr.to_lowercase();
    let away_lower = away_abbr.to_lowercase();

    let (home_label, away_label) = if label_a_lower == home_lower || label_a_lower.contains(&home_lower) {
        (label_a.clone(), label_b.clone())
    } else if label_b_lower == home_lower || label_b_lower.contains(&home_lower) {
        (label_b.clone(), label_a.clone())
    } else if label_a_lower == away_lower || label_a_lower.contains(&away_lower) {
        (label_b.clone(), label_a.clone())
    } else {
        return;
    };

    let mut new_outcomes = std::collections::HashMap::new();
    for (key, val) in vegas_odds.outcomes.drain() {
        let key_lower = key.to_lowercase();
        let is_home = key_lower.contains(&home_lower) || home_lower.contains(&key_lower);
        let new_key = if is_home {
            home_label.clone()
        } else {
            away_label.clone()
        };
        new_outcomes.insert(new_key, val);
    }
    vegas_odds.outcomes = new_outcomes;
}

/// Fallback normalization: lowercase + take last word.
fn normalize_fallback(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    let parts: Vec<&str> = cleaned.split_whitespace().collect();
    if let Some(last) = parts.last() {
        last.to_lowercase()
    } else {
        cleaned.to_lowercase()
    }
}
