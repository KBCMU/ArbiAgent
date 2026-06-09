use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::RwLock;
use tracing::debug;

use crate::models::{
    arb::ArbitrageOpportunity,
    event::{is_canonical_event_past, CanonicalEvent, EventOdds, OutcomePrice},
    vegas::{EvOpportunity, VegasOdds},
};

/// Snapshot of the last native-matching cycle, exposed via `/api/v2/matching/stats`.
#[derive(Debug, Clone, Serialize)]
pub struct MatchingStats {
    pub last_run_at: DateTime<Utc>,
    pub kalshi_candidates: usize,
    pub polymarket_candidates: usize,
    pub matched_pairs: usize,
    pub unmatched_kalshi: usize,
    pub unmatched_polymarket: usize,
    pub avg_match_score: f64,
    pub unmatched: Vec<UnmatchedStat>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnmatchedStat {
    pub event_title: String,
    pub platform: String,
    pub sport: String,
    pub best_rejected_score: Option<f64>,
    pub best_rejected_title: Option<String>,
}

/// Thread-safe in-memory cache for real-time state.
///
/// All reads/writes are lock-free via DashMap (sharded concurrent hashmap).
pub struct StateCache {
    /// Canonical events keyed by event ID.
    pub events: DashMap<String, CanonicalEvent>,
    /// Current odds keyed by event ID.
    pub odds: DashMap<String, EventOdds>,
    /// Active arbitrage opportunities keyed by event ID.
    pub active_arbs: DashMap<String, Vec<ArbitrageOpportunity>>,
    /// Vegas/sportsbook consensus odds keyed by canonical event ID.
    pub vegas_odds: DashMap<String, VegasOdds>,
    /// Active +EV opportunities keyed by canonical event ID.
    pub active_ev: DashMap<String, Vec<EvOpportunity>>,
    /// Latest matching cycle statistics (updated every discovery interval).
    pub matching_stats: RwLock<Option<MatchingStats>>,
}

impl StateCache {
    pub fn new() -> Self {
        Self {
            events: DashMap::new(),
            odds: DashMap::new(),
            active_arbs: DashMap::new(),
            vegas_odds: DashMap::new(),
            active_ev: DashMap::new(),
            matching_stats: RwLock::new(None),
        }
    }

    /// Returns true when the two token-ID sets differ (order-independent).
    fn token_ids_differ(a: &[String], b: &[String]) -> bool {
        if a.len() != b.len() { return true; }
        let mut sorted_a = a.to_vec();
        sorted_a.sort();
        let mut sorted_b = b.to_vec();
        sorted_b.sort();
        sorted_a != sorted_b
    }

    /// Insert or update a canonical event with merge semantics.
    ///
    /// D-04: never downgrade a dual-platform event to single-platform.
    /// D-05: clear stale Polymarket odds when token IDs change between cycles.
    pub fn upsert_event(&self, event: CanonicalEvent) {
        let incoming_is_dual = event.platform_ids.kalshi_event_ticker.is_some()
            && event.platform_ids.polymarket_market_slug.is_some();

        if let Some(cached) = self.events.get(&event.id) {
            let cached_is_dual = cached.platform_ids.kalshi_event_ticker.is_some()
                && cached.platform_ids.polymarket_market_slug.is_some();

            // D-04: never downgrade dual-platform to single-platform.
            if cached_is_dual && !incoming_is_dual {
                debug!(
                    "upsert_event: refusing downgrade for {} (cached=dual, incoming=single)",
                    event.id
                );
                return;
            }

            // D-05: if token IDs changed, clear stale Polymarket odds.
            let should_clear_odds = Self::token_ids_differ(
                &cached.platform_ids.polymarket_token_ids,
                &event.platform_ids.polymarket_token_ids,
            );

            // IMPORTANT: drop the DashMap guard BEFORE any mutable operations to avoid deadlock
            drop(cached);

            if should_clear_odds {
                debug!(
                    "upsert_event: token IDs changed for {} — clearing stale Polymarket odds",
                    event.id
                );
                self.clear_platform_odds(&event.id, "polymarket");
            }
        }

        self.events.insert(event.id.clone(), event);
    }

    /// Update odds for a specific platform + outcome on an event.
    pub fn update_odds(
        &self,
        event_id: &str,
        platform: &str,
        outcome: &str,
        price: OutcomePrice,
    ) {
        let mut entry = self.odds.entry(event_id.to_string()).or_insert_with(|| {
            EventOdds {
                canonical_event_id: event_id.to_string(),
                platform_odds: HashMap::new(),
                updated_at: Utc::now(),
            }
        });

        let platform_map = entry
            .platform_odds
            .entry(platform.to_string())
            .or_insert_with(HashMap::new);
        platform_map.insert(outcome.to_string(), price);
        entry.updated_at = Utc::now();
    }

    /// Get all events with their current odds (snapshot for API responses).
    pub fn get_all_events_with_odds(
        &self,
    ) -> Vec<(CanonicalEvent, Option<EventOdds>)> {
        self.events
            .iter()
            .map(|entry| {
                let event = entry.value().clone();
                let odds = self.odds.get(&event.id).map(|o| o.value().clone());
                (event, odds)
            })
            .collect()
    }

    /// Get a single event with odds.
    pub fn get_event_with_odds(
        &self,
        event_id: &str,
    ) -> Option<(CanonicalEvent, Option<EventOdds>)> {
        self.events.get(event_id).map(|entry| {
            let event = entry.value().clone();
            let odds = self.odds.get(event_id).map(|o| o.value().clone());
            (event, odds)
        })
    }

    /// Clear all odds for a specific platform on an event.
    ///
    /// Used when outcome labels are re-resolved (e.g. "Yes"/"No" → "LAL"/"BOS") to
    /// prevent stale generic keys from accumulating alongside the new real labels.
    pub fn clear_platform_odds(&self, event_id: &str, platform: &str) {
        if let Some(mut entry) = self.odds.get_mut(event_id) {
            entry.platform_odds.remove(platform);
        }
    }

    /// Store active arb opportunities for an event.
    pub fn set_active_arbs(&self, event_id: &str, arbs: Vec<ArbitrageOpportunity>) {
        if arbs.is_empty() {
            self.active_arbs.remove(event_id);
        } else {
            self.active_arbs.insert(event_id.to_string(), arbs);
        }
    }

    /// Get all active arbitrage opportunities across all events.
    pub fn get_all_active_arbs(&self) -> Vec<ArbitrageOpportunity> {
        self.active_arbs
            .iter()
            .flat_map(|entry| entry.value().clone())
            .collect()
    }

    /// Store vegas odds for a canonical event.
    pub fn set_vegas_odds(&self, event_id: &str, odds: VegasOdds) {
        self.vegas_odds.insert(event_id.to_string(), odds);
    }

    /// Store active +EV opportunities for an event.
    pub fn set_active_ev(&self, event_id: &str, opps: Vec<EvOpportunity>) {
        if opps.is_empty() {
            self.active_ev.remove(event_id);
        } else {
            self.active_ev.insert(event_id.to_string(), opps);
        }
    }

    /// Get all active +EV opportunities across all events.
    pub fn get_all_active_ev(&self) -> Vec<EvOpportunity> {
        self.active_ev
            .iter()
            .flat_map(|entry| entry.value().clone())
            .collect()
    }

    /// Get count of tracked events.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Get count of events with odds from both platforms.
    pub fn dual_platform_count(&self) -> usize {
        self.odds
            .iter()
            .filter(|entry| entry.value().platform_odds.len() >= 2)
            .count()
    }

    /// Remove stale events (past date or settled/closed status) from all cache maps.
    ///
    /// Also removes events with no odds update in the last `stale_hours` hours,
    /// which catches orphaned entries from failed discovery cycles.
    ///
    /// Returns the count of evicted events.
    pub fn evict_stale_events(&self, stale_hours: i64) -> usize {
        let cutoff = Utc::now() - chrono::Duration::hours(stale_hours);
        let mut evicted = 0usize;

        let stale_ids: Vec<String> = self
            .events
            .iter()
            .filter(|entry| {
                let event = entry.value();
                // Past date
                if is_canonical_event_past(event) {
                    return true;
                }
                // Settled or closed by platform lifecycle
                if event.status == "settled" || event.status == "closed" {
                    return true;
                }
                // No odds update for `stale_hours` — likely orphaned
                if let Some(odds) = self.odds.get(&event.id) {
                    if odds.updated_at < cutoff {
                        return true;
                    }
                } else if event.updated_at < cutoff {
                    // Also evict no-odds events that have not been refreshed.
                    return true;
                }
                false
            })
            .map(|entry| entry.key().clone())
            .collect();

        for id in &stale_ids {
            self.events.remove(id);
            self.odds.remove(id);
            self.active_arbs.remove(id);
            self.vegas_odds.remove(id);
            self.active_ev.remove(id);
            evicted += 1;
        }

        evicted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use crate::models::event::{PlatformIds, Sport};

    fn make_event(id: &str, sport: Sport, updated_at: chrono::DateTime<Utc>) -> CanonicalEvent {
        CanonicalEvent {
            id: id.to_string(),
            sport,
            event_title: "Test Event".to_string(),
            game_start_time: None,
            status: "open".to_string(),
            platform_ids: PlatformIds {
                kalshi_event_ticker: None,
                kalshi_market_tickers: vec![],
                polymarket_market_slug: None,
                polymarket_token_ids: vec![],
                polymarket_outcome_labels: vec![],
            },
            match_score: None,
            created_at: updated_at,
            updated_at,
        }
    }

    #[test]
    fn test_evict_stale_no_odds_event() {
        let cache = StateCache::new();
        let stale_ts = Utc::now() - Duration::hours(10);
        let id = "nba-bos-lal-nodate";
        cache.upsert_event(make_event(id, Sport::Nba, stale_ts));

        let evicted = cache.evict_stale_events(2);
        assert_eq!(evicted, 1);
        assert!(!cache.events.contains_key(id));
    }

    #[test]
    fn test_keep_fresh_no_odds_event() {
        let cache = StateCache::new();
        let fresh_ts = Utc::now() - Duration::minutes(30);
        let id = "nba-bos-lal-nodate";
        cache.upsert_event(make_event(id, Sport::Nba, fresh_ts));

        let evicted = cache.evict_stale_events(2);
        assert_eq!(evicted, 0);
        assert!(cache.events.contains_key(id));
    }

    #[test]
    fn test_upsert_never_downgrades_dual_to_single() {
        let cache = StateCache::new();
        let now = Utc::now();
        // Insert a dual-platform event
        let dual = CanonicalEvent {
            id: "nba-lal-bos-2026-03-14".to_string(),
            sport: Sport::Nba,
            event_title: "Lakers vs Celtics".to_string(),
            game_start_time: None,
            status: "open".to_string(),
            platform_ids: PlatformIds {
                kalshi_event_ticker: Some("KXNBA-X".to_string()),
                kalshi_market_tickers: vec![],
                polymarket_market_slug: Some("nba-x".to_string()),
                polymarket_token_ids: vec!["t1".to_string(), "t2".to_string()],
                polymarket_outcome_labels: vec![],
            },
            match_score: None,
            created_at: now,
            updated_at: now,
        };
        cache.upsert_event(dual);

        // Now upsert a single-platform event with the same ID
        let single = CanonicalEvent {
            id: "nba-lal-bos-2026-03-14".to_string(),
            sport: Sport::Nba,
            event_title: "Lakers vs Celtics".to_string(),
            game_start_time: None,
            status: "open".to_string(),
            platform_ids: PlatformIds {
                kalshi_event_ticker: None,  // single-platform: no Kalshi
                kalshi_market_tickers: vec![],
                polymarket_market_slug: Some("nba-x".to_string()),
                polymarket_token_ids: vec![],
                polymarket_outcome_labels: vec![],
            },
            match_score: None,
            created_at: now,
            updated_at: now,
        };
        cache.upsert_event(single);

        // Assert cached event still has Kalshi ticker (dual-platform was NOT downgraded)
        let cached = cache.events.get("nba-lal-bos-2026-03-14").unwrap();
        assert_eq!(cached.platform_ids.kalshi_event_ticker, Some("KXNBA-X".to_string()));
    }

    #[test]
    fn test_upsert_upgrades_single_to_dual() {
        let cache = StateCache::new();
        let now = Utc::now();
        // Insert a single-platform event first
        let single = CanonicalEvent {
            id: "nba-lal-bos-2026-03-14".to_string(),
            sport: Sport::Nba,
            event_title: "Lakers vs Celtics".to_string(),
            game_start_time: None,
            status: "open".to_string(),
            platform_ids: PlatformIds {
                kalshi_event_ticker: None,
                kalshi_market_tickers: vec![],
                polymarket_market_slug: Some("nba-x".to_string()),
                polymarket_token_ids: vec![],
                polymarket_outcome_labels: vec![],
            },
            match_score: None,
            created_at: now,
            updated_at: now,
        };
        cache.upsert_event(single);

        // Now upsert a dual-platform event with the same ID
        let dual = CanonicalEvent {
            id: "nba-lal-bos-2026-03-14".to_string(),
            sport: Sport::Nba,
            event_title: "Lakers vs Celtics".to_string(),
            game_start_time: None,
            status: "open".to_string(),
            platform_ids: PlatformIds {
                kalshi_event_ticker: Some("KXNBA-X".to_string()),
                kalshi_market_tickers: vec![],
                polymarket_market_slug: Some("nba-x".to_string()),
                polymarket_token_ids: vec![],
                polymarket_outcome_labels: vec![],
            },
            match_score: None,
            created_at: now,
            updated_at: now,
        };
        cache.upsert_event(dual);

        // Assert cache now has Kalshi ticker
        let cached = cache.events.get("nba-lal-bos-2026-03-14").unwrap();
        assert_eq!(cached.platform_ids.kalshi_event_ticker, Some("KXNBA-X".to_string()));
    }

    #[test]
    fn test_upsert_clears_poly_odds_on_token_change() {
        let cache = StateCache::new();
        let now = Utc::now();
        let event_id = "nba-lal-bos-2026-03-14";

        // Insert dual-platform event with token IDs A, B
        let event_v1 = CanonicalEvent {
            id: event_id.to_string(),
            sport: Sport::Nba,
            event_title: "Lakers vs Celtics".to_string(),
            game_start_time: None,
            status: "open".to_string(),
            platform_ids: PlatformIds {
                kalshi_event_ticker: Some("KXNBA-X".to_string()),
                kalshi_market_tickers: vec![],
                polymarket_market_slug: Some("nba-x".to_string()),
                polymarket_token_ids: vec!["A".to_string(), "B".to_string()],
                polymarket_outcome_labels: vec![],
            },
            match_score: None,
            created_at: now,
            updated_at: now,
        };
        cache.upsert_event(event_v1);

        // Add polymarket odds manually
        use std::collections::HashMap;
        let mut platform_odds = HashMap::new();
        let mut poly_odds = HashMap::new();
        poly_odds.insert("Yes".to_string(), OutcomePrice {
            yes_price: 0.6,
            no_price: 0.4,
            best_bid: None,
            best_ask: None,
            bid_size: None,
            ask_size: None,
        });
        platform_odds.insert("polymarket".to_string(), poly_odds);
        let event_odds = EventOdds {
            canonical_event_id: event_id.to_string(),
            platform_odds,
            updated_at: now,
        };
        cache.odds.insert(event_id.to_string(), event_odds);

        // Now upsert same event with DIFFERENT token IDs (C, D)
        let event_v2 = CanonicalEvent {
            id: event_id.to_string(),
            sport: Sport::Nba,
            event_title: "Lakers vs Celtics".to_string(),
            game_start_time: None,
            status: "open".to_string(),
            platform_ids: PlatformIds {
                kalshi_event_ticker: Some("KXNBA-X".to_string()),
                kalshi_market_tickers: vec![],
                polymarket_market_slug: Some("nba-x".to_string()),
                polymarket_token_ids: vec!["C".to_string(), "D".to_string()],
                polymarket_outcome_labels: vec![],
            },
            match_score: None,
            created_at: now,
            updated_at: now,
        };
        cache.upsert_event(event_v2);

        // Assert event token IDs updated
        let cached = cache.events.get(event_id).unwrap();
        assert_eq!(cached.platform_ids.polymarket_token_ids, vec!["C".to_string(), "D".to_string()]);

        // Assert polymarket odds were cleared
        let poly_odds_present = cache.odds.get(event_id)
            .map_or(false, |odds| odds.platform_odds.contains_key("polymarket"));
        assert!(!poly_odds_present,
            "Polymarket odds should have been cleared after token ID change");
    }
}
