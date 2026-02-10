use std::sync::Arc;

use anyhow::Result;
use tokio::sync::broadcast;
use tracing::info;

mod api;
mod config;
mod ingestion;
mod models;
mod processing;
mod storage;

use config::AppConfig;
use storage::{cache::StateCache, supabase::SupabaseClient};

/// Shared application state passed to all components.
pub struct AppState {
    pub config: AppConfig,
    pub cache: StateCache,
    pub db: SupabaseClient,
    /// Broadcast channel for pushing updates to WebSocket clients.
    pub arb_tx: broadcast::Sender<models::ws::WsMessage>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "arbiagent_backend=info,tower_http=info".into()),
        )
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;
    info!("🚀 ArbiAgent Backend v{}", env!("CARGO_PKG_VERSION"));
    info!("📡 DomeAPI configured: {}", !config.dome_api_key.is_empty());
    info!("🗄️  Supabase configured: {}", !config.supabase_url.is_empty());

    // Initialize Supabase client
    let db = SupabaseClient::new(&config).await?;
    info!("✅ Database connected");

    // Initialize in-memory cache
    let cache = StateCache::new();

    // Broadcast channel for WebSocket push (capacity = 256 messages)
    let (arb_tx, _) = broadcast::channel::<models::ws::WsMessage>(256);

    // Build shared state
    let state = Arc::new(AppState {
        config: config.clone(),
        cache,
        db,
        arb_tx: arb_tx.clone(),
    });

    // Spawn the DomeAPI event discovery poller
    let poller_state = Arc::clone(&state);
    tokio::spawn(async move {
        ingestion::dome_poller::run_event_discovery_loop(poller_state).await;
    });

    // Spawn the price refresh loop (REST-based, Phase 1)
    let price_state = Arc::clone(&state);
    tokio::spawn(async move {
        ingestion::dome_poller::run_price_refresh_loop(price_state).await;
    });

    // Spawn the arbitrage detection loop
    let arb_state = Arc::clone(&state);
    tokio::spawn(async move {
        processing::arb_detector::run_arb_detection_loop(arb_state).await;
    });

    // Spawn the snapshot writer (writes odds to Supabase every 60s)
    let snapshot_state = Arc::clone(&state);
    tokio::spawn(async move {
        storage::supabase::run_snapshot_writer(snapshot_state).await;
    });

    // Spawn real-time WebSocket ingesters (Phase 2)
    if config.enable_kalshi_ws {
        let kalshi_state = Arc::clone(&state);
        tokio::spawn(async move {
            ingestion::kalshi_ws::run_kalshi_ws_ingester(kalshi_state).await;
        });
        info!("🔌 Kalshi WebSocket ingester enabled");
    }

    if config.enable_polymarket_ws {
        let poly_state = Arc::clone(&state);
        tokio::spawn(async move {
            ingestion::polymarket_ws::run_polymarket_ws_ingester(poly_state).await;
        });
        info!("🔌 Polymarket WebSocket ingester enabled");
    }

    // Start the HTTP + WebSocket server
    let addr = format!("{}:{}", config.host, config.port);
    info!("🌐 Listening on http://{}", addr);
    api::serve(state, &addr).await?;

    Ok(())
}
