//! Shadow matching — runs DomeAPI discovery in parallel with the native
//! matching engine and logs comparison metrics for validation.
//!
//! When `ENABLE_SHADOW_MATCHING=true` and `DOME_API_KEY` is set, this module
//! fetches DomeAPI's matched events on the same schedule and compares them
//! against the native engine's output without affecting production state.

use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use chrono_tz::US::Eastern;
use reqwest::Client;
use tracing::{info, warn};

use crate::models::event::Sport;
use crate::models::platform::DomeSportsMatchResponse;
use crate::AppState;

const RATE_LIMIT_DELAY: std::time::Duration = std::time::Duration::from_millis(1_100);

/// Background loop: runs DomeAPI discovery for comparison, logging match metrics.
pub async fn run_shadow_comparison_loop(state: Arc<AppState>) {
    let interval = tokio::time::Duration::from_secs(
        state.config.event_discovery_interval_secs,
    );

    // Wait for the native matcher to complete its first cycle
    tokio::time::sleep(tokio::time::Duration::from_secs(45)).await;
    info!("Shadow comparison loop started");

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {}", state.config.dome_api_key).parse().unwrap(),
    );
    let client = Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    loop {
        match run_shadow_cycle(&client, &state).await {
            Ok(metrics) => {
                info!(
                    "Shadow comparison: dome={} events, native={} events, overlap={}, dome_only={}, native_only={}",
                    metrics.dome_events,
                    metrics.native_events,
                    metrics.overlap,
                    metrics.dome_only,
                    metrics.native_only,
                );
            }
            Err(e) => {
                warn!("Shadow comparison cycle failed: {}", e);
            }
        }

        tokio::time::sleep(interval).await;
    }
}

struct ShadowMetrics {
    dome_events: usize,
    native_events: usize,
    overlap: usize,
    dome_only: usize,
    native_only: usize,
}

async fn run_shadow_cycle(
    client: &Client,
    state: &AppState,
) -> anyhow::Result<ShadowMetrics> {
    // Fetch DomeAPI event keys
    let dome_keys = fetch_dome_event_keys(client, &state.config.dome_api_base_url).await?;
    let dome_set: HashSet<&str> = dome_keys.iter().map(|s| s.as_str()).collect();

    // Get native matcher's current event IDs from cache (dual-platform only)
    let native_keys: Vec<String> = state
        .cache
        .events
        .iter()
        .filter(|e| {
            !e.platform_ids.kalshi_market_tickers.is_empty()
                && !e.platform_ids.polymarket_token_ids.is_empty()
                && e.sport.is_sport()
        })
        .map(|e| e.id.clone())
        .collect();
    let native_set: HashSet<&str> = native_keys.iter().map(|s| s.as_str()).collect();

    let overlap = dome_set.intersection(&native_set).count();
    let dome_only = dome_set.difference(&native_set).count();
    let native_only = native_set.difference(&dome_set).count();

    // Log specific mismatches for debugging
    for key in dome_set.difference(&native_set) {
        warn!("Shadow: DomeAPI has '{}' but native matcher does not", key);
    }
    for key in native_set.difference(&dome_set) {
        info!("Shadow: native matcher has '{}' but DomeAPI does not", key);
    }

    Ok(ShadowMetrics {
        dome_events: dome_set.len(),
        native_events: native_set.len(),
        overlap,
        dome_only,
        native_only,
    })
}

/// Fetch all event keys from DomeAPI for comparison.
async fn fetch_dome_event_keys(
    client: &Client,
    base_url: &str,
) -> anyhow::Result<Vec<String>> {
    let now_eastern = Utc::now().with_timezone(&Eastern);
    let yesterday = (now_eastern - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let today = now_eastern.format("%Y-%m-%d").to_string();
    let tomorrow = (now_eastern + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    let dates = [&yesterday, &today, &tomorrow];
    let mut all_keys = Vec::new();
    let mut is_first = true;

    for sport in Sport::sports_for_discovery() {
        for date in &dates {
            if is_first {
                is_first = false;
            } else {
                tokio::time::sleep(RATE_LIMIT_DELAY).await;
            }

            let url = format!(
                "{}/matching-markets/sports/{}?date={}",
                base_url,
                sport.as_str(),
                date
            );

            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(data) = resp.json::<DomeSportsMatchResponse>().await {
                        all_keys.extend(data.markets.keys().cloned());
                    }
                }
                Ok(resp) => {
                    warn!("Shadow DomeAPI returned {} for {}/{}", resp.status(), sport.as_str(), date);
                }
                Err(e) => {
                    warn!("Shadow DomeAPI request failed for {}/{}: {}", sport.as_str(), date, e);
                }
            }
        }
    }

    Ok(all_keys)
}
