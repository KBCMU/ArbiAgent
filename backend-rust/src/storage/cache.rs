use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;

use crate::models::{
    arb::ArbitrageOpportunity,
    event::{is_event_past, CanonicalEvent, EventOdds, OutcomePrice},
};

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
}

impl StateCache {
    pub fn new() -> Self {
        Self {
            events: DashMap::new(),
            odds: DashMap::new(),
            active_arbs: DashMap::new(),
        }
    }

    /// Insert or update a canonical event.
    pub fn upsert_event(&self, event: CanonicalEvent) {
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
                if is_event_past(&event.id) {
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
}
