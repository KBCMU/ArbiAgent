use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A detected arbitrage opportunity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub canonical_event_id: String,
    pub event_title: String,
    pub sport: String,
    pub outcome: String,
    pub buy_platform: String,
    pub buy_price: f64,
    pub sell_platform: String,
    pub sell_price: f64,
    /// Raw margin before fees: 1.0 - buy_price - (1.0 - sell_price)
    pub margin_pct: f64,
    pub max_executable_size: Option<f64>,
    pub estimated_fees: Option<f64>,
    pub estimated_net_profit: Option<f64>,
    pub score: i32,
    pub detected_at: DateTime<Utc>,
}
