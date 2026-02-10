use anyhow::{Context, Result};

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct AppConfig {
    // DomeAPI
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

    // Kalshi direct API (for WebSocket)
    pub kalshi_email: String,
    pub kalshi_password: String,

    // Feature flags
    pub enable_kalshi_ws: bool,
    pub enable_polymarket_ws: bool,
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

        Ok(AppConfig {
            dome_api_key: std::env::var("DOME_API_KEY")
                .context("DOME_API_KEY must be set")?,
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
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
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

            kalshi_email: std::env::var("KALSHI_EMAIL").unwrap_or_default(),
            kalshi_password: std::env::var("KALSHI_PASSWORD").unwrap_or_default(),

            enable_kalshi_ws: std::env::var("ENABLE_KALSHI_WS")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            enable_polymarket_ws: std::env::var("ENABLE_POLYMARKET_WS")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        })
    }
}
