use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::config::AppConfig;
use crate::models::arb::ArbitrageOpportunity;
use crate::models::event::CanonicalEvent;
use crate::AppState;

/// Supabase/Postgres client using sqlx connection pool.
pub struct SupabaseClient {
    pool: PgPool,
}

impl SupabaseClient {
    pub async fn new(config: &AppConfig) -> Result<Self> {
        if config.database_url.is_empty() {
            // Return a client that will fail gracefully on DB operations
            warn!("⚠️  No DATABASE_URL configured — database operations will be skipped");
            // Create a minimal pool that won't actually connect
            let pool = PgPoolOptions::new()
                .max_connections(1)
                .connect_lazy(&format!(
                    "postgresql://postgres:postgres@localhost:5432/postgres"
                ))?;
            return Ok(Self { pool });
        }

        info!("🔗 Connecting to database (lazy)...");
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect_lazy(&config.database_url)?;

        Ok(Self { pool })
    }

    /// Insert or update a canonical event.
    pub async fn upsert_event(&self, event: &CanonicalEvent) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO canonical_events (
                id, sport, event_title, game_start_time, status,
                kalshi_event_ticker, kalshi_market_tickers,
                polymarket_market_slug, polymarket_token_ids,
                created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO UPDATE SET
                event_title = EXCLUDED.event_title,
                game_start_time = EXCLUDED.game_start_time,
                status = EXCLUDED.status,
                kalshi_event_ticker = EXCLUDED.kalshi_event_ticker,
                kalshi_market_tickers = EXCLUDED.kalshi_market_tickers,
                polymarket_market_slug = EXCLUDED.polymarket_market_slug,
                polymarket_token_ids = EXCLUDED.polymarket_token_ids,
                updated_at = NOW()
            "#,
        )
        .bind(&event.id)
        .bind(event.sport.as_str())
        .bind(&event.event_title)
        .bind(event.game_start_time)
        .bind(&event.status)
        .bind(&event.platform_ids.kalshi_event_ticker)
        .bind(&event.platform_ids.kalshi_market_tickers)
        .bind(&event.platform_ids.polymarket_market_slug)
        .bind(&event.platform_ids.polymarket_token_ids)
        .bind(event.created_at)
        .bind(event.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert an odds snapshot.
    pub async fn insert_odds_snapshot(
        &self,
        event_id: &str,
        platform: &str,
        outcome: &str,
        yes_price: f64,
        no_price: f64,
        best_bid: Option<f64>,
        best_ask: Option<f64>,
        bid_size: Option<f64>,
        ask_size: Option<f64>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO odds_snapshots (
                canonical_event_id, platform, outcome_name,
                yes_price, no_price, best_bid, best_ask,
                bid_size, ask_size, recorded_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            "#,
        )
        .bind(event_id)
        .bind(platform)
        .bind(outcome)
        .bind(yes_price)
        .bind(no_price)
        .bind(best_bid)
        .bind(best_ask)
        .bind(bid_size)
        .bind(ask_size)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert a detected arbitrage opportunity.
    pub async fn insert_arb_opportunity(&self, arb: &ArbitrageOpportunity) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO arbitrage_opportunities (
                canonical_event_id, outcome_name,
                buy_platform, buy_price, sell_platform, sell_price,
                margin_pct, max_executable_size, estimated_fees,
                estimated_net_profit, score, detected_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(&arb.canonical_event_id)
        .bind(&arb.outcome)
        .bind(&arb.buy_platform)
        .bind(arb.buy_price)
        .bind(&arb.sell_platform)
        .bind(arb.sell_price)
        .bind(arb.margin_pct)
        .bind(arb.max_executable_size)
        .bind(arb.estimated_fees)
        .bind(arb.estimated_net_profit)
        .bind(arb.score)
        .bind(arb.detected_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Fetch recent arb opportunities from the database.
    pub async fn get_recent_arbs(&self, limit: i32) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query_as::<_, (serde_json::Value,)>(
            r#"
            SELECT row_to_json(t) FROM (
                SELECT
                    ao.*,
                    ce.event_title,
                    ce.sport
                FROM arbitrage_opportunities ao
                JOIN canonical_events ce ON ce.id = ao.canonical_event_id
                ORDER BY ao.detected_at DESC
                LIMIT $1
            ) t
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// Fetch odds history for a specific event (for charting).
    pub async fn get_odds_history(
        &self,
        event_id: &str,
        hours: i32,
    ) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query_as::<_, (serde_json::Value,)>(
            r#"
            SELECT row_to_json(t) FROM (
                SELECT
                    platform,
                    outcome_name,
                    yes_price,
                    no_price,
                    best_bid,
                    best_ask,
                    recorded_at
                FROM odds_snapshots
                WHERE canonical_event_id = $1
                  AND recorded_at > NOW() - ($2 || ' hours')::interval
                ORDER BY recorded_at ASC
            ) t
            "#,
        )
        .bind(event_id)
        .bind(hours.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// Fetch arb history for a specific event or all events.
    pub async fn get_arb_history(
        &self,
        event_id: Option<&str>,
        limit: i32,
    ) -> Result<Vec<serde_json::Value>> {
        let rows = if let Some(eid) = event_id {
            sqlx::query_as::<_, (serde_json::Value,)>(
                r#"
                SELECT row_to_json(t) FROM (
                    SELECT
                        ao.*,
                        ce.event_title,
                        ce.sport
                    FROM arbitrage_opportunities ao
                    JOIN canonical_events ce ON ce.id = ao.canonical_event_id
                    WHERE ao.canonical_event_id = $1
                    ORDER BY ao.detected_at DESC
                    LIMIT $2
                ) t
                "#,
            )
            .bind(eid)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, (serde_json::Value,)>(
                r#"
                SELECT row_to_json(t) FROM (
                    SELECT
                        ao.*,
                        ce.event_title,
                        ce.sport
                    FROM arbitrage_opportunities ao
                    JOIN canonical_events ce ON ce.id = ao.canonical_event_id
                    ORDER BY ao.detected_at DESC
                    LIMIT $1
                ) t
                "#,
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// Close an arb window: set closed_at and compute duration_ms.
    pub async fn close_arb_window(
        &self,
        canonical_event_id: &str,
        outcome_name: &str,
        buy_platform: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE arbitrage_opportunities
            SET closed_at = NOW(),
                duration_ms = EXTRACT(EPOCH FROM (NOW() - detected_at)) * 1000
            WHERE canonical_event_id = $1
              AND outcome_name = $2
              AND buy_platform = $3
              AND closed_at IS NULL
            "#,
        )
        .bind(canonical_event_id)
        .bind(outcome_name)
        .bind(buy_platform)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get aggregate arb stats.
    pub async fn get_arb_stats(&self) -> Result<serde_json::Value> {
        let row = sqlx::query_as::<_, (serde_json::Value,)>(
            r#"
            SELECT row_to_json(t) FROM (
                SELECT
                    COUNT(*) as total_arbs,
                    COUNT(CASE WHEN closed_at IS NOT NULL THEN 1 END) as closed_arbs,
                    ROUND(AVG(margin_pct)::numeric, 3) as avg_margin_pct,
                    ROUND(MAX(margin_pct)::numeric, 3) as max_margin_pct,
                    ROUND(AVG(duration_ms)::numeric, 0) as avg_duration_ms,
                    ROUND(AVG(score)::numeric, 0) as avg_score,
                    COUNT(DISTINCT canonical_event_id) as events_with_arbs
                FROM arbitrage_opportunities
            ) t
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.0).unwrap_or(serde_json::json!({})))
    }
}

/// Periodically writes odds snapshots from the in-memory cache to Supabase.
///
/// Tracks `updated_at` per event so only events whose odds have changed since
/// the last snapshot cycle are written, avoiding redundant DB inserts.
pub async fn run_snapshot_writer(state: Arc<AppState>) {
    let interval = tokio::time::Duration::from_secs(state.config.snapshot_write_interval_secs);
    let mut last_snapshot_at: HashMap<String, DateTime<Utc>> = HashMap::new();

    loop {
        tokio::time::sleep(interval).await;

        let events_with_odds = state.cache.get_all_events_with_odds();
        let mut written = 0u32;
        let mut skipped = 0u32;

        for (event, odds_opt) in &events_with_odds {
            if let Some(odds) = odds_opt {
                // Skip events whose odds haven't changed since last snapshot
                if let Some(&prev_ts) = last_snapshot_at.get(&event.id) {
                    if odds.updated_at <= prev_ts {
                        skipped += 1;
                        continue;
                    }
                }

                for (platform, outcomes) in &odds.platform_odds {
                    for (outcome, price) in outcomes {
                        if let Err(e) = state
                            .db
                            .insert_odds_snapshot(
                                &event.id,
                                platform,
                                outcome,
                                price.yes_price,
                                price.no_price,
                                price.best_bid,
                                price.best_ask,
                                price.bid_size,
                                price.ask_size,
                            )
                            .await
                        {
                            error!("Failed to write snapshot for {}: {}", event.id, e);
                        } else {
                            written += 1;
                        }
                    }
                }

                last_snapshot_at.insert(event.id.clone(), Utc::now());
            }
        }

        if written > 0 || skipped > 0 {
            info!(
                "📸 Wrote {} odds snapshots to Supabase ({} events unchanged, skipped)",
                written, skipped
            );
        }
    }
}
