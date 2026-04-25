use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VegasOutcomeOdds {
    pub consensus_moneyline: f64,
    pub implied_prob: f64,
    pub fair_prob: f64,
    pub num_books: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VegasOdds {
    pub canonical_event_id: String,
    pub outcomes: HashMap<String, VegasOutcomeOdds>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvOpportunity {
    pub canonical_event_id: String,
    pub event_title: String,
    pub sport: String,
    pub outcome: String,
    pub market_platform: String,
    pub market_price: f64,
    pub vegas_fair_prob: f64,
    pub edge_pct: f64,
    pub consensus_moneyline: f64,
    pub kelly_fraction: Option<f64>,
    pub detected_at: DateTime<Utc>,
}
