use serde::{Deserialize, Serialize};

/// Messages pushed to WebSocket clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    #[serde(rename = "price_update")]
    PriceUpdate(PriceUpdateData),
    #[serde(rename = "arb_update")]
    ArbUpdate(ArbUpdateData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceUpdateData {
    pub canonical_event_id: String,
    pub kalshi: Option<PlatformPriceData>,
    pub polymarket: Option<PlatformPriceData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformPriceData {
    /// outcome_name -> yes_price
    pub outcomes: std::collections::HashMap<String, f64>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbUpdateData {
    pub canonical_event_id: String,
    pub event_title: String,
    pub sport: String,
    pub opportunities: Vec<super::arb::ArbitrageOpportunity>,
}
