use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;

use crate::models::{
    arb::ArbitrageOpportunity,
    event::{CanonicalEvent, EventOdds, OutcomePrice},
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
}
