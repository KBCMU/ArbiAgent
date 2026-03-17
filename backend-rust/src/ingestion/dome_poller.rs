use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use chrono_tz::US::Eastern;
use reqwest::Client;
use serde::Deserialize;
use tracing::{error, info, warn};

/// Delay between DomeAPI requests to respect the free-tier rate limit (1 req/sec).
const RATE_LIMIT_DELAY: std::time::Duration = std::time::Duration::from_millis(1_100);

use crate::models::event::{CanonicalEvent, OutcomePrice, PlatformIds};
use crate::models::platform::{
    DomePlatformMatch, DomeSportsMatchResponse, KalshiPriceResponse, PolymarketPriceResponse,
};
use crate::models::ws::{PlatformPriceData, PriceUpdateData, WsMessage};
use crate::AppState;

// ─── Gamma API types (Polymarket metadata) ─────────────────────────

/// Response from Polymarket Gamma API: GET /events/slug/{slug}
#[derive(Debug, Deserialize)]
struct GammaEventResponse {
    markets: Option<Vec<GammaMarket>>,
}

#[derive(Debug, Deserialize)]
struct GammaMarket {
    /// JSON-encoded string: '["tokenId1","tokenId2"]'
    #[serde(rename = "clobTokenIds")]
    clob_token_ids: Option<String>,
    /// JSON-encoded string: '["Team A","Team B"]'
    outcomes: Option<String>,
    #[serde(rename = "groupItemTitle")]
    group_item_title: Option<String>,
}

/// Builds an authenticated HTTP client for DomeAPI.
fn build_dome_client(api_key: &str) -> Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {}", api_key).parse().unwrap(),
    );
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client")
}

/// Background loop: discovers matched sports events via DomeAPI every N seconds.
pub async fn run_event_discovery_loop(state: Arc<AppState>) {
    let client = build_dome_client(&state.config.dome_api_key);
    let interval =
        tokio::time::Duration::from_secs(state.config.event_discovery_interval_secs);

    // Run immediately on startup, then on interval
    loop {
        match discover_all_sports_events(&client, &state).await {
            Ok(count) => {
                info!(
                    "🔍 Discovered {} matched events across all sports ({} total in cache)",
                    count,
                    state.cache.event_count()
                );
            }
            Err(e) => {
                error!("❌ Event discovery failed: {}", e);
            }
        }

        tokio::time::sleep(interval).await;
    }
}

/// Discovers matched events for all sports for yesterday, today, and tomorrow.
///
/// Uses US/Eastern timezone for date computation so that game dates align
/// with how DomeAPI indexes sports events (US-local dates, not UTC).
/// Inserts a delay between each request to stay within the free-tier rate limit.
async fn discover_all_sports_events(
    client: &Client,
    state: &AppState,
) -> anyhow::Result<usize> {
    use crate::models::event::Sport;

    // Compute dates in US/Eastern — this is the timezone DomeAPI uses for game dates.
    let now_eastern = Utc::now().with_timezone(&Eastern);
    let yesterday = (now_eastern - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let today = now_eastern.format("%Y-%m-%d").to_string();
    let tomorrow = (now_eastern + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    let dates = [&yesterday, &today, &tomorrow];

    info!(
        "📅 Discovery dates (US/Eastern): yesterday={}, today={}, tomorrow={}",
        yesterday, today, tomorrow
    );

    let mut total_new = 0usize;
    let mut is_first_request = true;

    for sport in Sport::sports_for_discovery() {
        for date in &dates {
            // Rate-limit: sleep before every request except the first
            if is_first_request {
                is_first_request = false;
            } else {
                tokio::time::sleep(RATE_LIMIT_DELAY).await;
            }

            match fetch_matched_events(client, &state.config.dome_api_base_url, sport, date).await {
                Ok(events) => {
                    if events.is_empty() {
                        info!(
                            "📭 {} {} — API returned OK, no matched markets",
                            sport.as_str(),
                            date
                        );
                    } else {
                        info!(
                            "📬 {} {} — found {} matched event(s)",
                            sport.as_str(),
                            date,
                            events.len()
                        );
                    }

                    for event in events {
                        // Only insert truly NEW events — don't overwrite existing ones
                        // (existing events may have corrected polymarket_outcome_labels)
                        if state.cache.events.contains_key(&event.id) {
                            continue;
                        }
                        state.cache.upsert_event(event.clone());
                        // Best-effort write to DB (don't fail if DB isn't configured)
                        if let Err(e) = state.db.upsert_event(&event).await {
                            warn!("DB write skipped for {}: {}", event.id, e);
                        }
                        total_new += 1;
                    }
                }
                Err(e) => {
                    warn!("⚠️  {} {} — {}", sport.as_str(), date, e);
                }
            }
        }
    }

    Ok(total_new)
}

/// Fetches matched events for a specific sport and date from DomeAPI.
async fn fetch_matched_events(
    client: &Client,
    base_url: &str,
    sport: &crate::models::event::Sport,
    date: &str,
) -> anyhow::Result<Vec<CanonicalEvent>> {
    let url = format!(
        "{}/matching-markets/sports/{}?date={}",
        base_url,
        sport.as_str(),
        date
    );

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("DomeAPI returned {}: {}", status, body);
    }

    let data: DomeSportsMatchResponse = response.json().await?;
    let now = Utc::now();

    let mut events = Vec::new();

    for (event_key, platform_matches) in data.markets {
        let mut kalshi_event_ticker = None;
        let mut kalshi_market_tickers = Vec::new();
        let mut polymarket_market_slug = None;
        let mut polymarket_token_ids = Vec::new();

        for pm in &platform_matches {
            match pm {
                DomePlatformMatch::Kalshi {
                    event_ticker,
                    market_tickers,
                } => {
                    kalshi_event_ticker = Some(event_ticker.clone());
                    kalshi_market_tickers = market_tickers.clone();
                }
                DomePlatformMatch::Polymarket {
                    market_slug,
                    token_ids,
                } => {
                    polymarket_market_slug = Some(market_slug.clone());
                    polymarket_token_ids = token_ids.clone();
                }
            }
        }

        // Build a human-readable title from the event key
        let title = build_title_from_key(&event_key, sport);

        // === Fetch Polymarket Gamma API to get correct token→outcome labels ===
        let polymarket_outcome_labels = resolve_polymarket_labels(
            client,
            &polymarket_market_slug,
            &polymarket_token_ids,
            &kalshi_market_tickers,
        )
        .await;

        let event = CanonicalEvent {
            id: event_key.clone(),
            sport: *sport,
            event_title: title,
            game_start_time: None,
            status: "open".to_string(),
            platform_ids: PlatformIds {
                kalshi_event_ticker,
                kalshi_market_tickers,
                polymarket_market_slug,
                polymarket_token_ids,
                polymarket_outcome_labels,
            },
            match_score: None,
            created_at: now,
            updated_at: now,
        };

        events.push(event);
    }

    Ok(events)
}

/// Build a readable title from a DomeAPI event key.
fn build_title_from_key(key: &str, sport: &crate::models::event::Sport) -> String {
    // Key format: "nfl-ari-den-2025-08-16"
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() >= 4 {
        let sport_str = sport.as_str().to_uppercase();
        let team_a = parts[1].to_uppercase();
        let team_b = parts[2].to_uppercase();
        // Remaining parts are the date
        let date_parts: Vec<&str> = parts[3..].iter().copied().collect();
        let date = date_parts.join("-");
        format!("{}: {} vs {} ({})", sport_str, team_a, team_b, date)
    } else {
        key.to_string()
    }
}

// ──────────────────────────────────────────────────────────────────────
// Price Refresh (REST-based, Phase 1 — will be replaced by WebSockets)
// ──────────────────────────────────────────────────────────────────────

/// Background loop: refreshes prices for all matched events via DomeAPI REST.
pub async fn run_price_refresh_loop(state: Arc<AppState>) {
    let client = build_dome_client(&state.config.dome_api_key);
    let interval =
        tokio::time::Duration::from_secs(state.config.price_refresh_interval_secs);

    // Wait for the first event discovery cycle to complete.
    // Discovery makes ~24 rate-limited requests (8 sports x 3 dates x 1.1s each ≈ 26s).
    tokio::time::sleep(tokio::time::Duration::from_secs(35)).await;

    loop {
        let events: Vec<CanonicalEvent> = state
            .cache
            .events
            .iter()
            .map(|e| e.value().clone())
            .collect();

        if events.is_empty() {
            tokio::time::sleep(interval).await;
            continue;
        }

        let total_tickers: usize = events
            .iter()
            .map(|e| {
                e.platform_ids.kalshi_market_tickers.len()
                    + e.platform_ids.polymarket_token_ids.len()
            })
            .sum();
        info!(
            "💰 Refreshing prices for {} events ({} tickers)...",
            events.len(),
            total_tickers
        );

        let mut price_updates = 0u32;
        let mut errors = 0u32;
        let mut is_first_request = true;

        for event in &events {
            // Fetch Kalshi prices
            for ticker in &event.platform_ids.kalshi_market_tickers {
                // Rate-limit: sleep before every request except the first
                if is_first_request {
                    is_first_request = false;
                } else {
                    tokio::time::sleep(RATE_LIMIT_DELAY).await;
                }

                match fetch_kalshi_price(&client, &state.config.dome_api_base_url, ticker).await {
                    Ok(price) => {
                        // Extract outcome name from ticker
                        // e.g. "KXNFLGAME-25AUG16ARIDEN-ARI" -> "ARI"
                        let outcome = extract_kalshi_outcome(ticker);
                        state.cache.update_odds(
                            &event.id,
                            "kalshi",
                            &outcome,
                            OutcomePrice {
                                yes_price: price.yes.price,
                                no_price: price.no.price,
                                best_bid: None,
                                best_ask: None,
                                bid_size: None,
                                ask_size: None,
                            },
                        );
                        price_updates += 1;
                    }
                    Err(e) => {
                        warn!("Failed to fetch Kalshi price for {}: {}", ticker, e);
                        errors += 1;
                    }
                }
            }

            // Fetch Polymarket prices
            for (i, token_id) in event.platform_ids.polymarket_token_ids.iter().enumerate() {
                // Rate-limit
                tokio::time::sleep(RATE_LIMIT_DELAY).await;

                match fetch_polymarket_price(&client, &state.config.dome_api_base_url, token_id)
                    .await
                {
                    Ok(price_resp) => {
                        // Token IDs correspond to outcomes in order
                        // For a 2-outcome market: token_ids[0] = team A, token_ids[1] = team B
                        let outcome = extract_polymarket_outcome(event, i);
                        state.cache.update_odds(
                            &event.id,
                            "polymarket",
                            &outcome,
                            OutcomePrice {
                                yes_price: price_resp.price,
                                no_price: 1.0 - price_resp.price,
                                best_bid: None,
                                best_ask: None,
                                bid_size: None,
                                ask_size: None,
                            },
                        );
                        price_updates += 1;
                    }
                    Err(e) => {
                        warn!("Failed to fetch Polymarket price for token {}: {}", token_id, e);
                        errors += 1;
                    }
                }
            }

            // === Self-correcting price alignment check ===
            // If the same outcome has wildly different YES prices across platforms,
            // the Polymarket outcome labels are inverted. Swap them in the cache.
            correct_flipped_outcomes(&event.id, &state.cache);

            // Broadcast price update to WebSocket clients
            let odds = state.cache.odds.get(&event.id);
            if let Some(odds) = odds {
                let msg = WsMessage::PriceUpdate(PriceUpdateData {
                    canonical_event_id: event.id.clone(),
                    kalshi: odds.platform_odds.get("kalshi").map(|outcomes| {
                        PlatformPriceData {
                            outcomes: outcomes
                                .iter()
                                .map(|(k, v)| (k.clone(), v.yes_price))
                                .collect(),
                            updated_at: odds.updated_at.to_rfc3339(),
                        }
                    }),
                    polymarket: odds.platform_odds.get("polymarket").map(|outcomes| {
                        PlatformPriceData {
                            outcomes: outcomes
                                .iter()
                                .map(|(k, v)| (k.clone(), v.yes_price))
                                .collect(),
                            updated_at: odds.updated_at.to_rfc3339(),
                        }
                    }),
                });
                // Ignore send errors (no subscribers)
                let _ = state.arb_tx.send(msg);
            }
        }

        info!(
            "💰 Price refresh complete: {} updates, {} errors",
            price_updates, errors
        );

        tokio::time::sleep(interval).await;
    }
}

/// Fetch Kalshi market price via DomeAPI REST.
async fn fetch_kalshi_price(
    client: &Client,
    base_url: &str,
    ticker: &str,
) -> anyhow::Result<KalshiPriceResponse> {
    let url = format!("{}/kalshi/market-price/{}", base_url, ticker);
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        anyhow::bail!("Kalshi price API returned {}", resp.status());
    }

    Ok(resp.json().await?)
}

/// Fetch Polymarket market price via DomeAPI REST.
async fn fetch_polymarket_price(
    client: &Client,
    base_url: &str,
    token_id: &str,
) -> anyhow::Result<PolymarketPriceResponse> {
    let url = format!("{}/polymarket/market-price/{}", base_url, token_id);
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        anyhow::bail!("Polymarket price API returned {}", resp.status());
    }

    Ok(resp.json().await?)
}

/// Detect and correct flipped outcomes between Kalshi and Polymarket.
///
/// For binary markets (2 outcomes per platform), if the same outcome name
/// has YES prices that diverge by >30% across platforms, the Polymarket
/// labels are inverted. This function:
///   1. Swaps the Polymarket odds keys in the odds cache (immediate fix)
///   2. Swaps the `polymarket_outcome_labels` on the event (persistent fix
///      so `extract_polymarket_outcome` produces correct labels on future cycles)
fn correct_flipped_outcomes(
    event_id: &str,
    cache: &crate::storage::cache::StateCache,
) {
    // ── Step 1: Check odds for a flip ──────────────────────────────────
    let needs_swap = {
        let entry = match cache.odds.get(event_id) {
            Some(e) => e,
            None => return,
        };
        let odds = entry.value();

        let kalshi = match odds.platform_odds.get("kalshi") {
            Some(k) if k.len() == 2 => k,
            _ => return,
        };
        let poly = match odds.platform_odds.get("polymarket") {
            Some(p) if p.len() == 2 => p,
            _ => return,
        };

        // Collect matching outcome names
        let kalshi_keys: Vec<String> = kalshi.keys().cloned().collect();
        let matching: Vec<String> = kalshi_keys
            .iter()
            .filter(|k| poly.contains_key(k.as_str()))
            .cloned()
            .collect();

        if matching.len() != 2 {
            return;
        }

        let mut total_gap = 0.0;
        for name in &matching {
            let k_price = kalshi[name].yes_price;
            let p_price = poly[name].yes_price;
            total_gap += (k_price - p_price).abs();
        }
        let avg_gap = total_gap / matching.len() as f64;

        if avg_gap > 0.30 {
            Some((matching[0].clone(), matching[1].clone(), avg_gap))
        } else {
            None
        }
    };

    // ── Step 2: Perform the swap if needed ─────────────────────────────
    if let Some((outcome_a, outcome_b, avg_gap)) = needs_swap {
        // 2a. Swap odds cache keys
        if let Some(mut odds_entry) = cache.odds.get_mut(event_id) {
            let odds = odds_entry.value_mut();
            if let Some(poly_map) = odds.platform_odds.get_mut("polymarket") {
                let price_a = poly_map.get(&outcome_a).cloned();
                let price_b = poly_map.get(&outcome_b).cloned();
                if let (Some(pa), Some(pb)) = (price_a, price_b) {
                    poly_map.insert(outcome_a.clone(), pb);
                    poly_map.insert(outcome_b.clone(), pa);
                }
            }
        }

        // 2b. Swap stored polymarket_outcome_labels on the event so
        //     extract_polymarket_outcome() gives correct labels next cycle
        if let Some(mut event_entry) = cache.events.get_mut(event_id) {
            let labels = &mut event_entry.value_mut().platform_ids.polymarket_outcome_labels;
            if labels.len() == 2 {
                labels.swap(0, 1);
            }
        }

        warn!(
            "🔄 Corrected flipped Polymarket outcomes for {}: swapped {} ↔ {} (avg gap {:.1}%)",
            event_id, outcome_a, outcome_b, avg_gap * 100.0
        );
    }
}

/// Extract the outcome name from a Kalshi market ticker.
/// e.g. "KXNFLGAME-25AUG16ARIDEN-ARI" -> "ARI"
fn extract_kalshi_outcome(ticker: &str) -> String {
    ticker
        .rsplit('-')
        .next()
        .unwrap_or("UNKNOWN")
        .to_string()
}

/// Public version of extract_kalshi_outcome for cross-module use.
pub fn extract_kalshi_outcome_pub(ticker: &str) -> String {
    extract_kalshi_outcome(ticker)
}

/// Extract outcome name for a Polymarket token by index.
/// Uses the pre-resolved labels from Gamma API metadata.
fn extract_polymarket_outcome(event: &CanonicalEvent, token_index: usize) -> String {
    // Use pre-resolved labels from Gamma API (correct mapping)
    if let Some(label) = event.platform_ids.polymarket_outcome_labels.get(token_index) {
        if !label.is_empty() {
            return label.clone();
        }
    }
    // Fallback: use Kalshi ticker naming (may be wrong for some events)
    if let Some(kalshi_ticker) = event.platform_ids.kalshi_market_tickers.get(token_index) {
        extract_kalshi_outcome(kalshi_ticker)
    } else {
        format!("OUTCOME_{}", token_index)
    }
}

// ─── Polymarket Gamma API: token→outcome resolution ────────────────

/// Fetches Polymarket event metadata from Gamma API and resolves
/// token_id → Kalshi-style outcome abbreviation.
async fn resolve_polymarket_labels(
    client: &Client,
    slug: &Option<String>,
    token_ids: &[String],
    kalshi_tickers: &[String],
) -> Vec<String> {
    let slug = match slug {
        Some(s) => s,
        None => {
            // No Polymarket slug — fallback to Kalshi naming
            return kalshi_tickers
                .iter()
                .map(|t| extract_kalshi_outcome(t))
                .collect();
        }
    };

    // Fetch the Gamma API to get clobTokenIds → outcome labels
    match fetch_gamma_event_metadata(client, slug).await {
        Ok(gamma_map) => {
            // gamma_map: token_id → Gamma label (e.g. "Spurs", "Lakers")
            // We need to convert Gamma labels → Kalshi abbreviations
            let kalshi_outcomes: Vec<String> = kalshi_tickers
                .iter()
                .map(|t| extract_kalshi_outcome(t))
                .collect();

            // ── Pass 1: fuzzy-match each Gamma label to a Kalshi abbreviation ──
            // Entries that can't be matched are left as None for the elimination pass.
            let mut result: Vec<Option<String>> = Vec::with_capacity(token_ids.len());
            let mut used_kalshi: Vec<bool> = vec![false; kalshi_outcomes.len()];

            for token_id in token_ids {
                if let Some(gamma_label) = gamma_map.get(token_id) {
                    if let Some(kalshi_abbrev) =
                        match_gamma_label_to_kalshi(gamma_label, &kalshi_outcomes)
                    {
                        // Mark this Kalshi outcome as consumed
                        if let Some(idx) = kalshi_outcomes.iter().position(|k| k == &kalshi_abbrev) {
                            used_kalshi[idx] = true;
                        }
                        result.push(Some(kalshi_abbrev));
                    } else {
                        result.push(None); // Unresolved — will attempt elimination
                    }
                } else {
                    result.push(None);
                }
            }

            // ── Pass 2: process-of-elimination for unmatched tokens ──
            // IMPORTANT: only do elimination when there is exactly one unresolved
            // token and one unused Kalshi outcome. Forcing positional assignment
            // across 2+ unresolved outcomes can invert YES/NO side alignment and
            // generate false arbitrage signals.
            let unmatched_indices: Vec<usize> = result
                .iter()
                .enumerate()
                .filter_map(|(i, v)| if v.is_none() { Some(i) } else { None })
                .collect();
            let unused_kalshi: Vec<&String> = kalshi_outcomes
                .iter()
                .zip(used_kalshi.iter())
                .filter_map(|(k, &used)| if !used { Some(k) } else { None })
                .collect();

            if unmatched_indices.len() == 1 && unused_kalshi.len() == 1 {
                let slot = unmatched_indices[0];
                let kalshi_label = unused_kalshi[0];
                info!(
                    "🔗 Elimination match for {}: position {} → {}",
                    slug, slot, kalshi_label
                );
                result[slot] = Some(kalshi_label.to_string());
            }

            // ── Pass 3: bipartite string similarity mapping for 2 remaining ──
            // CBB often fails Pass 1 because exact substrings don't match (e.g. "FAU" not in "FLORIDA ATLANTIC").
            let unmatched_indices: Vec<usize> = result
                .iter()
                .enumerate()
                .filter_map(|(i, v)| if v.is_none() { Some(i) } else { None })
                .collect();
            let unused_kalshi: Vec<&String> = kalshi_outcomes
                .iter()
                .zip(used_kalshi.iter())
                .filter_map(|(k, &used)| if !used { Some(k) } else { None })
                .collect();

            if unmatched_indices.len() == 2 && unused_kalshi.len() == 2 {
                let g0 = gamma_map
                    .get(&token_ids[unmatched_indices[0]])
                    .map(|s| s.as_str())
                    .unwrap_or("")
                    .to_uppercase();
                let g1 = gamma_map
                    .get(&token_ids[unmatched_indices[1]])
                    .map(|s| s.as_str())
                    .unwrap_or("")
                    .to_uppercase();

                let k0 = unused_kalshi[0].to_uppercase();
                let k1 = unused_kalshi[1].to_uppercase();

                let score_a = subsequence_score(&g0, &k0) + subsequence_score(&g1, &k1);
                let score_b = subsequence_score(&g0, &k1) + subsequence_score(&g1, &k0);

                if score_a >= score_b {
                    info!(
                        "🔗 Bipartite match for {}: config A ({}->{}, {}->{})",
                        slug, g0, k0, g1, k1
                    );
                    result[unmatched_indices[0]] = Some(unused_kalshi[0].clone());
                    result[unmatched_indices[1]] = Some(unused_kalshi[1].clone());
                } else {
                    info!(
                        "🔗 Bipartite match for {}: config B ({}->{}, {}->{})",
                        slug, g0, k1, g1, k0
                    );
                    result[unmatched_indices[0]] = Some(unused_kalshi[1].clone());
                    result[unmatched_indices[1]] = Some(unused_kalshi[0].clone());
                }
            }

            // ── Finalize: convert Option<String> to String ──
            let result: Vec<String> = result
                .into_iter()
                .enumerate()
                .map(|(i, opt)| {
                    opt.unwrap_or_else(|| {
                        // Last resort: use Gamma label as-is (will be unique within event)
                        gamma_map
                            .get(&token_ids[i])
                            .map(|l| l.to_uppercase())
                            .unwrap_or_default()
                    })
                })
                .collect();

            // Sanity: verify no duplicates (if all mapped to same name, something is wrong)
            if result.len() == 2 && result[0] == result[1] && !result[0].is_empty() {
                warn!("Duplicate outcome labels detected for slug {}, using Kalshi fallback", slug);
                return kalshi_outcomes;
            }

            info!(
                "✅ Resolved Polymarket labels for {}: {:?}",
                slug, result
            );
            result
        }
        Err(e) => {
            warn!(
                "⚠️  Gamma API lookup failed for {}: {}. Using Kalshi-based fallback.",
                slug, e
            );
            // Fallback to Kalshi ticker ordering (may be inverted!)
            kalshi_tickers
                .iter()
                .map(|t| extract_kalshi_outcome(t))
                .collect()
        }
    }
}

/// Fetch Polymarket event metadata from Gamma API.
/// Returns a map of token_id → outcome label.
async fn fetch_gamma_event_metadata(
    client: &Client,
    slug: &str,
) -> anyhow::Result<HashMap<String, String>> {
    let url = format!(
        "https://gamma-api.polymarket.com/events/slug/{}",
        slug
    );

    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Gamma API returned {}", resp.status());
    }

    let event: GammaEventResponse = resp.json().await?;

    let markets = event.markets.unwrap_or_default();
    // Find the moneyline market (skip O/U, spreads, etc.)
    // Moneyline markets typically have groupItemTitle == None or "Winner"
    let market = markets
        .iter()
        .find(|m| {
            let title = m.group_item_title.as_deref().unwrap_or("").to_lowercase();
            title.is_empty() || title.contains("winner") || title.contains("moneyline")
        })
        .or_else(|| markets.first());

    let market = match market {
        Some(m) => m,
        None => anyhow::bail!("No markets found in Gamma response for {}", slug),
    };

    // Parse the JSON-encoded arrays
    let token_ids_str = market
        .clob_token_ids
        .as_deref()
        .unwrap_or("[]");
    let outcomes_str = market.outcomes.as_deref().unwrap_or("[]");

    let token_ids: Vec<String> = serde_json::from_str(token_ids_str)
        .unwrap_or_default();
    let outcomes: Vec<String> = serde_json::from_str(outcomes_str)
        .unwrap_or_default();

    let mut mapping = HashMap::new();
    for (tid, label) in token_ids.into_iter().zip(outcomes.into_iter()) {
        mapping.insert(tid, label);
    }

    Ok(mapping)
}

/// Match a Gamma API outcome label (e.g. "Spurs", "San Antonio Spurs")
/// to a Kalshi abbreviation (e.g. "SAS") using fuzzy matching.
fn match_gamma_label_to_kalshi(gamma_label: &str, kalshi_outcomes: &[String]) -> Option<String> {
    let gamma_lower = gamma_label.to_lowercase();

    // NBA team name → abbreviation lookup
    let team_map: Vec<(&str, &str)> = vec![
        // NBA
        ("hawks", "ATL"), ("atlanta", "ATL"),
        ("celtics", "BOS"), ("boston", "BOS"),
        ("nets", "BKN"), ("brooklyn", "BKN"),
        ("hornets", "CHA"), ("charlotte", "CHA"),
        ("bulls", "CHI"), ("chicago", "CHI"),
        ("cavaliers", "CLE"), ("cleveland", "CLE"), ("cavs", "CLE"),
        ("mavericks", "DAL"), ("dallas", "DAL"), ("mavs", "DAL"),
        ("nuggets", "DEN"), ("denver", "DEN"),
        ("pistons", "DET"), ("detroit", "DET"),
        ("warriors", "GSW"), ("golden state", "GSW"),
        ("rockets", "HOU"), ("houston", "HOU"),
        ("pacers", "IND"), ("indiana", "IND"),
        ("clippers", "LAC"), ("la clippers", "LAC"),
        ("lakers", "LAL"), ("la lakers", "LAL"),
        ("grizzlies", "MEM"), ("memphis", "MEM"),
        ("heat", "MIA"), ("miami", "MIA"),
        ("bucks", "MIL"), ("milwaukee", "MIL"),
        ("timberwolves", "MIN"), ("minnesota", "MIN"), ("wolves", "MIN"),
        ("pelicans", "NOP"), ("new orleans", "NOP"),
        ("knicks", "NYK"), ("new york", "NYK"),
        ("thunder", "OKC"), ("oklahoma city", "OKC"),
        ("magic", "ORL"), ("orlando", "ORL"),
        ("76ers", "PHI"), ("philadelphia", "PHI"), ("sixers", "PHI"),
        ("suns", "PHX"), ("phoenix", "PHX"),
        ("trail blazers", "POR"), ("portland", "POR"), ("blazers", "POR"),
        ("kings", "SAC"), ("sacramento", "SAC"),
        ("spurs", "SAS"), ("san antonio", "SAS"),
        ("raptors", "TOR"), ("toronto", "TOR"),
        ("jazz", "UTA"), ("utah", "UTA"),
        ("wizards", "WAS"), ("washington", "WAS"),
        // NFL
        ("cardinals", "ARI"), ("arizona", "ARI"),
        ("falcons", "ATL"),
        ("ravens", "BAL"), ("baltimore", "BAL"),
        ("bills", "BUF"), ("buffalo", "BUF"),
        ("panthers", "CAR"), ("carolina", "CAR"),
        ("bears", "CHI"),
        ("bengals", "CIN"), ("cincinnati", "CIN"),
        ("browns", "CLE"),
        ("cowboys", "DAL"),
        ("broncos", "DEN"),
        ("lions", "DET"),
        ("packers", "GB"), ("green bay", "GB"),
        ("texans", "HOU"),
        ("colts", "IND"), ("indianapolis", "IND"),
        ("jaguars", "JAX"), ("jacksonville", "JAX"),
        ("chiefs", "KC"), ("kansas city", "KC"),
        ("chargers", "LAC"), ("la chargers", "LAC"),
        ("rams", "LAR"), ("la rams", "LAR"),
        ("dolphins", "MIA"),
        ("vikings", "MIN"),
        ("patriots", "NE"), ("new england", "NE"),
        ("saints", "NO"),
        ("giants", "NYG"),
        ("jets", "NYJ"),
        ("raiders", "LV"), ("las vegas", "LV"),
        ("eagles", "PHI"),
        ("steelers", "PIT"), ("pittsburgh", "PIT"),
        ("49ers", "SF"), ("san francisco", "SF"), ("niners", "SF"),
        ("seahawks", "SEA"), ("seattle", "SEA"),
        ("buccaneers", "TB"), ("tampa bay", "TB"), ("bucs", "TB"),
        ("titans", "TEN"), ("tennessee", "TEN"),
        ("commanders", "WAS"),
    ];

    // First try the lookup table
    for (keyword, abbrev) in &team_map {
        if gamma_lower.contains(keyword) {
            // Verify this abbreviation exists in kalshi_outcomes
            if kalshi_outcomes.iter().any(|k| k == abbrev) {
                return Some(abbrev.to_string());
            }
        }
    }

    // CBB/College: Gamma often uses full names, Kalshi uses abbreviations
    // Try direct abbreviation match (e.g. Gamma says "OKST" and Kalshi has "OKST")
    let gamma_upper = gamma_label.to_uppercase();
    if kalshi_outcomes.iter().any(|k| k == &gamma_upper) {
        return Some(gamma_upper);
    }

    // Try containment: if any Kalshi abbreviation is a substring of the Gamma label
    for kalshi_out in kalshi_outcomes {
        if gamma_upper.contains(kalshi_out.as_str()) {
            return Some(kalshi_out.clone());
        }
    }

    // Last resort: try each Kalshi abbreviation as a prefix in the Gamma label
    for kalshi_out in kalshi_outcomes {
        if gamma_lower.starts_with(&kalshi_out.to_lowercase()) {
            return Some(kalshi_out.clone());
        }
    }

    None
}

/// Computes a fuzzy score based on how well the characters of `abbr`
/// form an ordered subsequence inside `full_name`. Both must be uppercase.
fn subsequence_score(full_name: &str, abbr: &str) -> i32 {
    let mut score = 0;
    let mut full_chars = full_name.chars();

    for a_char in abbr.chars() {
        if !a_char.is_alphanumeric() {
            continue;
        }
        while let Some(f_char) = full_chars.next() {
            if f_char == a_char {
                score += 1;
                break; // Found match in order, move to next char in abbr
            }
        }
    }

    score
}
