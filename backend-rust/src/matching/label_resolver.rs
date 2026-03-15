//! Polymarket outcome label resolution via the Gamma API.
//!
//! Extracted from `dome_poller.rs` for reuse by the native matching engine.
//! Maps Polymarket token IDs to Kalshi-compatible outcome abbreviations using
//! a 3-pass strategy: keyword matching → elimination → bipartite scoring.

use std::collections::HashMap;

use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};

use super::team_dictionary;

// ─── Gamma API Types ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GammaEventResponse {
    markets: Option<Vec<GammaMarket>>,
}

#[derive(Debug, Deserialize)]
struct GammaMarket {
    #[serde(rename = "clobTokenIds")]
    clob_token_ids: Option<String>,
    outcomes: Option<String>,
    #[serde(rename = "groupItemTitle")]
    group_item_title: Option<String>,
}

// ─── Public API ─────────────────────────────────────────────────────

/// Resolve Polymarket token IDs to Kalshi-style outcome abbreviations.
///
/// Uses the Gamma API to get human-readable outcome labels, then maps
/// them to Kalshi abbreviations using the shared team dictionary.
///
/// Falls back to Kalshi-ticker-derived labels if the Gamma API is unavailable.
pub async fn resolve_polymarket_labels(
    client: &Client,
    slug: &Option<String>,
    token_ids: &[String],
    kalshi_tickers: &[String],
) -> Vec<String> {
    let slug = match slug {
        Some(s) => s,
        None => {
            return kalshi_tickers
                .iter()
                .map(|t| team_dictionary::extract_kalshi_outcome(t))
                .collect();
        }
    };

    match fetch_gamma_event_metadata(client, slug).await {
        Ok(gamma_map) => {
            let kalshi_outcomes: Vec<String> = kalshi_tickers
                .iter()
                .map(|t| team_dictionary::extract_kalshi_outcome(t))
                .collect();

            // Pass 1: keyword-based matching via team dictionary
            let mut result: Vec<Option<String>> = Vec::with_capacity(token_ids.len());
            let mut used_kalshi: Vec<bool> = vec![false; kalshi_outcomes.len()];

            for token_id in token_ids {
                if let Some(gamma_label) = gamma_map.get(token_id) {
                    if let Some(kalshi_abbrev) =
                        team_dictionary::match_label_to_abbrev(gamma_label, &kalshi_outcomes)
                    {
                        if let Some(idx) = kalshi_outcomes.iter().position(|k| k == &kalshi_abbrev) {
                            used_kalshi[idx] = true;
                        }
                        result.push(Some(kalshi_abbrev));
                    } else {
                        result.push(None);
                    }
                } else {
                    result.push(None);
                }
            }

            // Pass 2: process-of-elimination for exactly one unresolved
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
                    "Elimination match for {}: position {} -> {}",
                    slug, slot, kalshi_label
                );
                result[slot] = Some(kalshi_label.to_string());
            }

            // Pass 3: bipartite string similarity for exactly two unresolved
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

                let score_a = team_dictionary::subsequence_score(&g0, &k0)
                    + team_dictionary::subsequence_score(&g1, &k1);
                let score_b = team_dictionary::subsequence_score(&g0, &k1)
                    + team_dictionary::subsequence_score(&g1, &k0);

                if score_a >= score_b {
                    result[unmatched_indices[0]] = Some(unused_kalshi[0].clone());
                    result[unmatched_indices[1]] = Some(unused_kalshi[1].clone());
                } else {
                    result[unmatched_indices[0]] = Some(unused_kalshi[1].clone());
                    result[unmatched_indices[1]] = Some(unused_kalshi[0].clone());
                }
            }

            // Finalize: convert Option<String> to String
            let result: Vec<String> = result
                .into_iter()
                .enumerate()
                .map(|(i, opt)| {
                    opt.unwrap_or_else(|| {
                        gamma_map
                            .get(&token_ids[i])
                            .map(|l| l.to_uppercase())
                            .unwrap_or_default()
                    })
                })
                .collect();

            // Sanity: no duplicate labels
            if result.len() == 2 && result[0] == result[1] && !result[0].is_empty() {
                warn!(
                    "Duplicate outcome labels for slug {}, using Kalshi fallback",
                    slug
                );
                return kalshi_outcomes;
            }

            info!("Resolved Polymarket labels for {}: {:?}", slug, result);
            result
        }
        Err(e) => {
            warn!(
                "Gamma API lookup failed for {}: {}. Using Kalshi-based fallback.",
                slug, e
            );
            kalshi_tickers
                .iter()
                .map(|t| team_dictionary::extract_kalshi_outcome(t))
                .collect()
        }
    }
}

// ─── Internal ───────────────────────────────────────────────────────

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

    // Find moneyline market
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

    let token_ids: Vec<String> = market
        .clob_token_ids
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let outcomes: Vec<String> = market
        .outcomes
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let mut mapping = HashMap::new();
    for (tid, label) in token_ids.into_iter().zip(outcomes.into_iter()) {
        mapping.insert(tid, label);
    }

    Ok(mapping)
}
