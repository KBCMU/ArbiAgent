use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

use crate::models::event::OutcomePrice;
use crate::models::ws::{PlatformPriceData, PriceUpdateData, WsMessage};
use crate::AppState;

const POLYMARKET_WS_URL: &str = "wss://ws-subscriptions-clob.polymarket.com/ws/market";

// ─── Polymarket WS Message Types ────────────────────────────────────

/// Subscription message sent to Polymarket.
#[derive(Debug, Serialize)]
struct PolySubscribeMsg {
    assets_id: Vec<String>,
    #[serde(rename = "type")]
    msg_type: String,
}

/// Incoming Polymarket WebSocket message.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PolyWsMessage {
    event_type: Option<String>,
    asset_id: Option<String>,
    price: Option<f64>,
    /// For book updates
    market: Option<String>,
    bids: Option<Vec<PolyBookLevel>>,
    asks: Option<Vec<PolyBookLevel>>,
    /// For last_trade_price
    last_trade_price: Option<f64>,
    /// For best_bid_ask
    best_bid: Option<f64>,
    best_ask: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PolyBookLevel {
    price: String,
    size: String,
}

/// Main entry point: runs the Polymarket WebSocket ingester with auto-reconnect.
pub async fn run_polymarket_ws_ingester(state: Arc<AppState>) {
    info!("🔌 Polymarket WebSocket ingester starting...");

    // Wait for event discovery to populate cache first
    tokio::time::sleep(Duration::from_secs(10)).await;

    let mut backoff_secs = 1u64;
    let max_backoff = 60u64;

    loop {
        match run_polymarket_connection(&state).await {
            Ok(()) => {
                info!("Polymarket WS connection closed normally, reconnecting...");
                backoff_secs = 1;
            }
            Err(e) => {
                error!(
                    "❌ Polymarket WS error: {} — reconnecting in {}s",
                    e, backoff_secs
                );
            }
        }

        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
        backoff_secs = (backoff_secs * 2).min(max_backoff);
    }
}

/// Single connection attempt to Polymarket WebSocket.
async fn run_polymarket_connection(state: &AppState) -> anyhow::Result<()> {
    // Collect all Polymarket token IDs from cache
    let token_ids: Vec<String> = state
        .cache
        .events
        .iter()
        .flat_map(|e| e.value().platform_ids.polymarket_token_ids.clone())
        .collect();

    if token_ids.is_empty() {
        warn!("No Polymarket token IDs to subscribe to — waiting for event discovery");
        tokio::time::sleep(Duration::from_secs(30)).await;
        return Ok(());
    }

    info!(
        "📡 Connecting to Polymarket WS with {} token IDs...",
        token_ids.len()
    );

    // Connect to WebSocket (no auth required for market data)
    let (ws_stream, _) = connect_async(POLYMARKET_WS_URL).await?;
    info!("✅ Polymarket WS connected");

    let (mut write, mut read) = ws_stream.split();

    // Subscribe to all token IDs
    // Polymarket WS accepts batches via assets_id array
    for chunk in token_ids.chunks(100) {
        let subscribe_msg = PolySubscribeMsg {
            assets_id: chunk.to_vec(),
            msg_type: "market".to_string(),
        };

        let json = serde_json::to_string(&subscribe_msg)?;
        write.send(Message::Text(json.into())).await?;
        info!(
            "📡 Polymarket subscribed to {} tokens",
            chunk.len()
        );

        // Small delay between subscription batches
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Process incoming messages
    let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(25));

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Polymarket sometimes sends arrays of messages
                        if text.starts_with('[') {
                            if let Ok(msgs) = serde_json::from_str::<Vec<PolyWsMessage>>(&text) {
                                for m in msgs {
                                    if let Err(e) = process_polymarket_message(m, state) {
                                        warn!("Failed to process Polymarket msg: {}", e);
                                    }
                                }
                            }
                        } else {
                            match serde_json::from_str::<PolyWsMessage>(&text) {
                                Ok(m) => {
                                    if let Err(e) = process_polymarket_message(m, state) {
                                        warn!("Failed to process Polymarket msg: {}", e);
                                    }
                                }
                                Err(_) => {
                                    // Might be a control message or ack, ignore
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        write.send(Message::Pong(data)).await?;
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("Polymarket WS received close frame");
                        return Ok(());
                    }
                    Some(Err(e)) => {
                        return Err(e.into());
                    }
                    None => {
                        return Ok(());
                    }
                    _ => {}
                }
            }
            _ = heartbeat_interval.tick() => {
                // Send ping to keep connection alive
                if let Err(e) = write.send(Message::Ping(vec![].into())).await {
                    warn!("Failed to send Polymarket heartbeat: {}", e);
                    return Err(e.into());
                }
            }
        }
    }
}

/// Process a single Polymarket WebSocket message and update the cache.
fn process_polymarket_message(msg: PolyWsMessage, state: &AppState) -> anyhow::Result<()> {
    let token_id = match &msg.asset_id {
        Some(id) => id.clone(),
        None => return Ok(()), // Not a price message
    };

    // Find which canonical event this token belongs to, and its index
    let event_info: Option<(String, usize)> = state.cache.events.iter().find_map(|e| {
        let event = e.value();
        event
            .platform_ids
            .polymarket_token_ids
            .iter()
            .position(|t| t == &token_id)
            .map(|idx| (event.id.clone(), idx))
    });

    let (event_id, token_index) = match event_info {
        Some(info) => info,
        None => return Ok(()), // Token not in any tracked event
    };

    // Determine the outcome name from the token index
    let outcome = {
        let event = state.cache.events.get(&event_id);
        match event {
            Some(e) => {
                if let Some(kalshi_ticker) =
                    e.value().platform_ids.kalshi_market_tickers.get(token_index)
                {
                    crate::ingestion::dome_poller::extract_kalshi_outcome_pub(kalshi_ticker)
                } else {
                    format!("OUTCOME_{}", token_index)
                }
            }
            None => format!("OUTCOME_{}", token_index),
        }
    };

    // Extract price from the message
    let yes_price = msg
        .price
        .or(msg.last_trade_price)
        .unwrap_or(0.0);

    if yes_price <= 0.0 || yes_price >= 1.0 {
        return Ok(()); // Invalid price, skip
    }

    let best_bid = msg.best_bid;
    let best_ask = msg.best_ask;

    // Determine bid/ask sizes from book data if available
    let (bid_size, ask_size) = if let (Some(bids), Some(asks)) = (&msg.bids, &msg.asks) {
        let total_bid: f64 = bids
            .iter()
            .filter_map(|l| l.size.parse::<f64>().ok())
            .sum();
        let total_ask: f64 = asks
            .iter()
            .filter_map(|l| l.size.parse::<f64>().ok())
            .sum();
        (Some(total_bid), Some(total_ask))
    } else {
        (None, None)
    };

    state.cache.update_odds(
        &event_id,
        "polymarket",
        &outcome,
        OutcomePrice {
            yes_price,
            no_price: 1.0 - yes_price,
            best_bid,
            best_ask,
            bid_size,
            ask_size,
        },
    );

    // Broadcast update to frontend
    let odds = state.cache.odds.get(&event_id);
    if let Some(odds) = odds {
        let ws_msg = WsMessage::PriceUpdate(PriceUpdateData {
            canonical_event_id: event_id.clone(),
            kalshi: odds.platform_odds.get("kalshi").map(|outcomes| {
                PlatformPriceData {
                    outcomes: outcomes
                        .iter()
                        .map(|(k, v)| (k.clone(), v.yes_price))
                        .collect(),
                    updated_at: Utc::now().to_rfc3339(),
                }
            }),
            polymarket: odds.platform_odds.get("polymarket").map(|outcomes| {
                PlatformPriceData {
                    outcomes: outcomes
                        .iter()
                        .map(|(k, v)| (k.clone(), v.yes_price))
                        .collect(),
                    updated_at: Utc::now().to_rfc3339(),
                }
            }),
        });
        let _ = state.arb_tx.send(ws_msg);
    }

    Ok(())
}
