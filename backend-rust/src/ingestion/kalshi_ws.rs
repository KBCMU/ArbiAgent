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

const KALSHI_WS_URL: &str = "wss://api.elections.kalshi.com/trade-api/ws/v2";
const KALSHI_LOGIN_URL: &str = "https://api.elections.kalshi.com/trade-api/v2/login";

// ─── Kalshi WS Message Types ────────────────────────────────────────

#[derive(Debug, Serialize)]
struct KalshiLoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct KalshiLoginResponse {
    token: String,
    #[serde(default)]
    member_id: String,
}

#[derive(Debug, Serialize)]
struct KalshiWsCommand {
    id: u64,
    cmd: String,
    params: KalshiWsParams,
}

#[derive(Debug, Serialize)]
struct KalshiWsParams {
    channels: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    market_tickers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct KalshiWsMessage {
    #[serde(rename = "type")]
    msg_type: Option<String>,
    msg: Option<KalshiTickerMsg>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct KalshiTickerMsg {
    market_ticker: Option<String>,
    yes_price: Option<f64>,
    no_price: Option<f64>,
    yes_bid: Option<f64>,
    yes_ask: Option<f64>,
    no_bid: Option<f64>,
    no_ask: Option<f64>,
    volume: Option<f64>,
}

/// Main entry point: runs the Kalshi WebSocket ingester with auto-reconnect.
pub async fn run_kalshi_ws_ingester(state: Arc<AppState>) {
    // Check if Kalshi credentials are configured
    if state.config.kalshi_email.is_empty() || state.config.kalshi_password.is_empty() {
        warn!("⚠️  Kalshi credentials not configured — WebSocket ingester disabled");
        warn!("   Set KALSHI_EMAIL and KALSHI_PASSWORD in .env to enable");
        return;
    }

    info!("🔌 Kalshi WebSocket ingester starting...");

    let mut backoff_secs = 1u64;
    let max_backoff = 60u64;

    loop {
        match run_kalshi_connection(&state).await {
            Ok(()) => {
                info!("Kalshi WS connection closed normally, reconnecting...");
                backoff_secs = 1; // Reset backoff on clean close
            }
            Err(e) => {
                error!("❌ Kalshi WS error: {} — reconnecting in {}s", e, backoff_secs);
            }
        }

        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
        backoff_secs = (backoff_secs * 2).min(max_backoff);
    }
}

/// Single connection attempt to Kalshi WebSocket.
async fn run_kalshi_connection(state: &AppState) -> anyhow::Result<()> {
    // Step 1: Authenticate via REST API
    let token = authenticate_kalshi(
        &state.config.kalshi_email,
        &state.config.kalshi_password,
    )
    .await?;
    info!("✅ Kalshi authenticated");

    // Step 2: Connect WebSocket
    let url = format!("{}?token={}", KALSHI_WS_URL, token);
    let (ws_stream, _) = connect_async(&url).await?;
    info!("✅ Kalshi WS connected");

    let (mut write, mut read) = ws_stream.split();

    // Step 3: Collect all Kalshi market tickers from cache
    let tickers: Vec<String> = state
        .cache
        .events
        .iter()
        .flat_map(|e| e.value().platform_ids.kalshi_market_tickers.clone())
        .collect();

    if tickers.is_empty() {
        warn!("No Kalshi tickers to subscribe to — waiting for event discovery");
        tokio::time::sleep(Duration::from_secs(30)).await;
        return Ok(());
    }

    // Step 4: Subscribe to ticker channel for all market tickers
    // Subscribe in batches of 50 to avoid message size limits
    for (batch_idx, chunk) in tickers.chunks(50).enumerate() {
        let subscribe_msg = KalshiWsCommand {
            id: (batch_idx + 1) as u64,
            cmd: "subscribe".to_string(),
            params: KalshiWsParams {
                channels: vec!["ticker".to_string()],
                market_tickers: chunk.to_vec(),
            },
        };

        let json = serde_json::to_string(&subscribe_msg)?;
        write.send(Message::Text(json.into())).await?;
        info!(
            "📡 Kalshi subscribed to {} tickers (batch {})",
            chunk.len(),
            batch_idx + 1
        );
    }

    // Step 5: Process incoming messages
    let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = process_kalshi_message(&text, state) {
                            warn!("Failed to process Kalshi message: {}", e);
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        write.send(Message::Pong(data)).await?;
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("Kalshi WS received close frame");
                        return Ok(());
                    }
                    Some(Err(e)) => {
                        return Err(e.into());
                    }
                    None => {
                        return Ok(()); // Stream ended
                    }
                    _ => {} // Binary, Pong, etc.
                }
            }
            _ = heartbeat_interval.tick() => {
                // Send a ping to keep the connection alive
                if let Err(e) = write.send(Message::Ping(vec![].into())).await {
                    warn!("Failed to send Kalshi heartbeat: {}", e);
                    return Err(e.into());
                }
            }
        }
    }
}

/// Authenticate with Kalshi REST API to get a session token.
async fn authenticate_kalshi(email: &str, password: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(KALSHI_LOGIN_URL)
        .json(&KalshiLoginRequest {
            email: email.to_string(),
            password: password.to_string(),
        })
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Kalshi login failed ({}): {}", status, body);
    }

    let login_resp: KalshiLoginResponse = resp.json().await?;
    Ok(login_resp.token)
}

/// Process a single Kalshi WebSocket message and update the cache.
fn process_kalshi_message(text: &str, state: &AppState) -> anyhow::Result<()> {
    let msg: KalshiWsMessage = serde_json::from_str(text)?;

    // Only process ticker messages
    if msg.msg_type.as_deref() != Some("ticker") {
        return Ok(());
    }

    let ticker_msg = match msg.msg {
        Some(m) => m,
        None => return Ok(()),
    };

    let market_ticker = match &ticker_msg.market_ticker {
        Some(t) => t.clone(),
        None => return Ok(()),
    };

    // Find which canonical event this ticker belongs to
    let event_id = state
        .cache
        .events
        .iter()
        .find(|e| {
            e.value()
                .platform_ids
                .kalshi_market_tickers
                .contains(&market_ticker)
        })
        .map(|e| e.value().id.clone());

    if let Some(event_id) = event_id {
        let outcome = crate::ingestion::dome_poller::extract_kalshi_outcome_pub(&market_ticker);

        // Kalshi prices are in cents (0-99) or dollars (0.00-1.00) depending on endpoint
        // WebSocket ticker gives prices in cents, convert to dollars
        let yes_price = ticker_msg
            .yes_price
            .map(|p| if p > 1.0 { p / 100.0 } else { p })
            .unwrap_or(0.0);
        let no_price = ticker_msg
            .no_price
            .map(|p| if p > 1.0 { p / 100.0 } else { p })
            .unwrap_or(0.0);

        state.cache.update_odds(
            &event_id,
            "kalshi",
            &outcome,
            OutcomePrice {
                yes_price,
                no_price,
                best_bid: ticker_msg.yes_bid.map(|p| if p > 1.0 { p / 100.0 } else { p }),
                best_ask: ticker_msg.yes_ask.map(|p| if p > 1.0 { p / 100.0 } else { p }),
                bid_size: None,
                ask_size: None,
            },
        );

        // Broadcast update to frontend
        let odds = state.cache.odds.get(&event_id);
        if let Some(odds) = odds {
            let msg = WsMessage::PriceUpdate(PriceUpdateData {
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
            let _ = state.arb_tx.send(msg);
        }
    }

    Ok(())
}
