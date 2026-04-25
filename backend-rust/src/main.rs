use std::sync::Arc;

use anyhow::Result;
use tokio::sync::broadcast;
use tracing::info;

mod api;
mod config;
mod ingestion;
mod matching;
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
    info!("📡 Native matching: {}", config.enable_native_matching);
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

    // ── Background Tasks ────────────────────────────────────────────

    if config.enable_prediction_api {
        // PredictionAPI mode — replaces all native ingestion paths.
        info!("📡 Data source: PredictionAPI ({})", config.prediction_api_url);

        let event_state = Arc::clone(&state);
        tokio::spawn(async move {
            ingestion::prediction_api::run_prediction_event_loop(event_state).await;
        });

        let price_state = Arc::clone(&state);
        tokio::spawn(async move {
            ingestion::prediction_api::run_prediction_price_loop(price_state).await;
        });
    } else {
        // Native ingestion — matching engine, direct price APIs, culture poller, WebSockets.

        // Event discovery — native matching (default) or DomeAPI fallback
        if config.enable_native_matching {
            let matching_state = Arc::clone(&state);
            tokio::spawn(async move {
                matching::run_sports_matching_loop(matching_state).await;
            });
            info!("🔍 Event discovery: native matching engine");

            // Shadow mode: also run DomeAPI for comparison logging
            if config.enable_shadow_matching && !config.dome_api_key.is_empty() {
                let shadow_state = Arc::clone(&state);
                tokio::spawn(async move {
                    matching::shadow::run_shadow_comparison_loop(shadow_state).await;
                });
                info!("👥 Shadow matching: DomeAPI running in parallel for comparison");
            }
        } else {
            let poller_state = Arc::clone(&state);
            tokio::spawn(async move {
                ingestion::dome_poller::run_event_discovery_loop(poller_state).await;
            });
            info!("🔍 Event discovery: DomeAPI (legacy)");
        }

        // Price refresh — direct batch APIs (default) or DomeAPI fallback
        if config.enable_direct_price_api {
            let price_state = Arc::clone(&state);
            tokio::spawn(async move {
                ingestion::direct_api::run_direct_price_refresh_loop(price_state).await;
            });
            info!("📡 Price fetching: direct Kalshi + Polymarket batch APIs");
        } else {
            let price_state = Arc::clone(&state);
            tokio::spawn(async move {
                ingestion::dome_poller::run_price_refresh_loop(price_state).await;
            });
            info!("📡 Price fetching: DomeAPI (legacy fallback)");
        }

        // Culture event discovery (Polymarket Gamma API + Kalshi direct API)
        let culture_state = Arc::clone(&state);
        tokio::spawn(async move {
            ingestion::culture_poller::run_culture_discovery_loop(culture_state).await;
        });

        // Real-time WebSocket ingesters
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
    }

    // Arbitrage detection (always runs regardless of data source)
    let arb_state = Arc::clone(&state);
    tokio::spawn(async move {
        processing::arb_detector::run_arb_detection_loop(arb_state).await;
    });

    // Vegas odds polling (BetStack API) — gracefully skips if BETSTACK_API_KEY not set
    let vegas_state = Arc::clone(&state);
    tokio::spawn(async move {
        ingestion::vegas_poller::run_vegas_polling_loop(vegas_state).await;
    });

    // +EV detection (compares vegas fair probs to prediction market prices)
    let ev_state = Arc::clone(&state);
    tokio::spawn(async move {
        processing::ev_detector::run_ev_detection_loop(ev_state).await;
    });

    // Snapshot writer (writes odds to Supabase periodically)
    let snapshot_state = Arc::clone(&state);
    tokio::spawn(async move {
        storage::supabase::run_snapshot_writer(snapshot_state).await;
    });

    // Cache eviction — periodically removes stale/settled events to bound memory
    let eviction_state = Arc::clone(&state);
    tokio::spawn(async move {
        run_cache_eviction_loop(eviction_state).await;
    });

    // ── HTTP Server with Graceful Shutdown ───────────────────────────

    let addr = format!("{}:{}", config.host, config.port);
    info!("🌐 Listening on http://{}", addr);

    let server_state = Arc::clone(&state);
    tokio::select! {
        result = api::serve(server_state, &addr) => {
            result?;
        }
        _ = tokio::signal::ctrl_c() => {
            info!("⏹️  Received Ctrl+C — shutting down gracefully");
            info!(
                "📊 Final cache stats: {} events, {} with dual-platform odds",
                state.cache.event_count(),
                state.cache.dual_platform_count()
            );
        }
    }

    Ok(())
}

/// Periodically evicts stale events from the in-memory cache.
///
/// Runs every 30 minutes, removing events that are past their game date,
/// settled/closed by platform lifecycle, or have had no odds update for 24 hours.
async fn run_cache_eviction_loop(state: Arc<AppState>) {
    const EVICTION_INTERVAL_SECS: u64 = 30 * 60; // 30 minutes
    const STALE_HOURS: i64 = 24;

    // Initial delay — let the system warm up before evicting
    tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;

    loop {
        let before = state.cache.event_count();
        let evicted = state.cache.evict_stale_events(STALE_HOURS);
        if evicted > 0 {
            info!(
                "🧹 Cache eviction: removed {} stale events ({} → {} remaining)",
                evicted,
                before,
                state.cache.event_count()
            );
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(EVICTION_INTERVAL_SECS)).await;
    }
}
