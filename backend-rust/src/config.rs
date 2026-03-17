use anyhow::Result;

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct AppConfig {
    // DomeAPI (optional when native matching is enabled)
    pub dome_api_key: String,
    pub dome_api_base_url: String,

    // Supabase
    pub supabase_url: String,
    pub supabase_service_role_key: String,
    pub database_url: String,

    // Server
    pub host: String,
    pub port: u16,

    // Polling intervals (seconds)
    pub event_discovery_interval_secs: u64,
    pub price_refresh_interval_secs: u64,
    pub snapshot_write_interval_secs: u64,

    // Arbitrage thresholds
    pub arb_min_margin_pct: f64,
    pub arb_min_profit_usd: f64,

    // Trading fees (percentage of contract value)
    pub kalshi_fee_pct: f64,
    pub polymarket_fee_pct: f64,

    // Kalshi direct API (for WebSocket)
    pub kalshi_email: String,
    pub kalshi_password: String,

    // Feature flags
    pub enable_kalshi_ws: bool,
    pub enable_polymarket_ws: bool,
    /// Use direct Kalshi/Polymarket batch APIs for price fetching instead of DomeAPI.
    /// Default: true. Set to false to fall back to the old DomeAPI-based price loop.
    pub enable_direct_price_api: bool,

    // Native matching
    /// Use the native matching engine instead of DomeAPI for sports event discovery.
    /// Default: true. Set to false to use DomeAPI (requires DOME_API_KEY).
    pub enable_native_matching: bool,
    /// Minimum score (0-100) to accept a cross-platform match.
    pub match_min_score: f64,
    /// Run both DomeAPI and native matching in parallel for comparison logging.
    pub enable_shadow_matching: bool,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let supabase_url = std::env::var("SUPABASE_URL").unwrap_or_default();
        let supabase_service_role_key =
            std::env::var("SUPABASE_SERVICE_ROLE_KEY").unwrap_or_default();

        // Build database URL from Supabase project ref, or use explicit DATABASE_URL
        let explicit_db_url = std::env::var("DATABASE_URL").unwrap_or_default();
        let database_url = if explicit_db_url.is_empty() {
            // Extract project ref from supabase URL (https://xxx.supabase.co -> xxx)
            let project_ref = supabase_url
                .replace("https://", "")
                .replace(".supabase.co", "");
            let db_password =
                std::env::var("SUPABASE_DB_PASSWORD").unwrap_or_else(|_| "postgres".to_string());
            format!(
                "postgresql://postgres.{}:{}@aws-0-us-west-2.pooler.supabase.com:6543/postgres",
                project_ref, db_password
            )
        } else {
            explicit_db_url
        };

        let enable_native_matching: bool = std::env::var("ENABLE_NATIVE_MATCHING")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        // DOME_API_KEY is only required when native matching is disabled
        let dome_api_key = std::env::var("DOME_API_KEY").unwrap_or_default();
        if !enable_native_matching && dome_api_key.is_empty() {
            anyhow::bail!(
                "DOME_API_KEY must be set when ENABLE_NATIVE_MATCHING=false"
            );
        }

        Ok(AppConfig {
            dome_api_key,
            dome_api_base_url: std::env::var("DOME_API_BASE_URL")
                .unwrap_or_else(|_| "https://api.domeapi.io/v1".to_string()),

            supabase_url,
            supabase_service_role_key,
            database_url,

            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),

            event_discovery_interval_secs: std::env::var("EVENT_DISCOVERY_INTERVAL_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            price_refresh_interval_secs: std::env::var("PRICE_REFRESH_INTERVAL_SECS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            snapshot_write_interval_secs: std::env::var("SNAPSHOT_WRITE_INTERVAL_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),

            arb_min_margin_pct: std::env::var("ARB_MIN_MARGIN_PCT")
                .unwrap_or_else(|_| "1.0".to_string())
                .parse()
                .unwrap_or(1.0),
            arb_min_profit_usd: std::env::var("ARB_MIN_PROFIT_USD")
                .unwrap_or_else(|_| "0.50".to_string())
                .parse()
                .unwrap_or(0.50),

            kalshi_fee_pct: std::env::var("KALSHI_FEE_PCT")
                .unwrap_or_else(|_| "3.0".to_string())
                .parse()
                .unwrap_or(3.0),
            polymarket_fee_pct: std::env::var("POLYMARKET_FEE_PCT")
                .unwrap_or_else(|_| "0.0".to_string())
                .parse()
                .unwrap_or(0.0),

            kalshi_email: std::env::var("KALSHI_EMAIL").unwrap_or_default(),
            kalshi_password: std::env::var("KALSHI_PASSWORD").unwrap_or_default(),

            enable_kalshi_ws: std::env::var("ENABLE_KALSHI_WS")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            enable_polymarket_ws: std::env::var("ENABLE_POLYMARKET_WS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),

            enable_direct_price_api: std::env::var("ENABLE_DIRECT_PRICE_API")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),

            enable_native_matching,
            match_min_score: std::env::var("MATCH_MIN_SCORE")
                .unwrap_or_else(|_| "60.0".to_string())
                .parse()
                .unwrap_or(60.0),
            enable_shadow_matching: std::env::var("ENABLE_SHADOW_MATCHING")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        })
    }
}
