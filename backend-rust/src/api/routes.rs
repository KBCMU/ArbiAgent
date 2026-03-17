
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;

use crate::models::event::{is_event_past, is_event_settled_full, CanonicalEvent, EventOdds};
use crate::models::platform::{
    ArbOpportunityResponse, EventResponse, OutcomePriceResponse, PlatformOddsResponse,
};
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(root))
        .route("/api/v2/health", get(health))
        .route("/api/v2/events", get(list_events))
        .route("/api/v2/events/{id}", get(get_event))
        .route("/api/v2/events/{id}/odds-history", get(odds_history))
        .route("/api/v2/arbitrage", get(list_arbitrage))
        .route("/api/v2/arbitrage/history", get(arb_history))
        .route("/api/v2/arbitrage/stats", get(arb_stats))
        .route("/api/v2/matching/stats", get(matching_stats))
        .route("/api/v2/sports/{sport}/today", get(sport_today))
}

async fn root() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "message": "ArbiAgent Rust Backend is running",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "dome_api_configured": !state.config.dome_api_key.is_empty(),
        "supabase_configured": !state.config.supabase_url.is_empty(),
        "tracked_events": state.cache.event_count(),
        "dual_platform_events": state.cache.dual_platform_count(),
        "active_arbs": state.cache.get_all_active_arbs().len(),
    }))
}

#[derive(Deserialize)]
struct ListEventsParams {
    sport: Option<String>,
    status: Option<String>,
    limit: Option<usize>,
}

fn has_binary_outcomes(event: &CanonicalEvent, odds_opt: Option<&EventOdds>) -> bool {
    // Prefer actual odds-map cardinality when available (source of truth for
    // what the frontend will render). Fall back to platform metadata when odds
    // are missing.
    if let Some(odds) = odds_opt {
        if event.sport.is_sport() {
            if let Some(k) = odds.platform_odds.get("kalshi") {
                if !k.is_empty() && k.len() != 2 {
                    return false;
                }
            }
            if let Some(p) = odds.platform_odds.get("polymarket") {
                if !p.is_empty() && p.len() != 2 {
                    return false;
                }
            }

            // Safety net: ensure rendered union is still binary even if platform
            // keys diverge (e.g. stale relabel keys).
            let mut rendered_outcomes = std::collections::HashSet::new();
            if let Some(k) = odds.platform_odds.get("kalshi") {
                for key in k.keys() {
                    rendered_outcomes.insert(key.to_lowercase());
                }
            }
            if let Some(p) = odds.platform_odds.get("polymarket") {
                for key in p.keys() {
                    rendered_outcomes.insert(key.to_lowercase());
                }
            }
            if !rendered_outcomes.is_empty() && rendered_outcomes.len() != 2 {
                return false;
            }
        }
        true
    } else {
        let outcome_count = if !event.platform_ids.kalshi_market_tickers.is_empty() {
            event.platform_ids.kalshi_market_tickers.len()
        } else {
            event.platform_ids.polymarket_token_ids.len()
        };
        outcome_count == 2
    }
}

async fn list_events(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListEventsParams>,
) -> Json<serde_json::Value> {
    let limit = params.limit.unwrap_or(100);
    let events_with_odds = state.cache.get_all_events_with_odds();

    let mut responses: Vec<EventResponse> = events_with_odds
        .into_iter()
        .filter(|(event, odds_opt)| {
            if !has_binary_outcomes(event, odds_opt.as_ref()) {
                return false;
            }
            // For sports events: reject any that still have generic Yes/No labels
            // (these are futures events that slipped through upstream filters)
            if event.sport.is_sport() {
                let labels = &event.platform_ids.polymarket_outcome_labels;
                let all_generic = !labels.is_empty()
                    && labels.iter().all(|l| {
                        l.eq_ignore_ascii_case("yes") || l.eq_ignore_ascii_case("no")
                    });
                if all_generic {
                    return false;
                }
            }
            // Skip events whose game date has already passed
            if is_event_past(&event.id) {
                return false;
            }
            // Skip events whose markets have settled (status or price heuristic)
            if let Some(odds) = odds_opt {
                if is_event_settled_full(&event.status, odds) {
                    return false;
                }
            }
            // Filter by sport
            if let Some(ref sport) = params.sport {
                if event.sport.as_str() != sport.as_str() {
                    return false;
                }
            }
            // Filter by status
            if let Some(ref status) = params.status {
                if event.status != *status {
                    return false;
                }
            }
            true
        })
        .map(|(event, odds)| build_event_response(&event, odds.as_ref(), &state))
        .take(limit)
        .collect();

    // Sort: events with arb opportunities first, then by title
    responses.sort_by(|a, b| {
        let a_has_arb = a.arb_opportunity.is_some();
        let b_has_arb = b.arb_opportunity.is_some();
        b_has_arb.cmp(&a_has_arb).then(a.event_title.cmp(&b.event_title))
    });

    Json(serde_json::json!({
        "events": responses,
        "total": responses.len(),
    }))
}

async fn get_event(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.cache.get_event_with_odds(&id) {
        Some((event, odds)) => {
            let response = build_event_response(&event, odds.as_ref(), &state);
            Json(serde_json::json!(response))
        }
        None => Json(serde_json::json!({
            "error": "Not found",
            "message": format!("Event '{}' not found", id),
        })),
    }
}

async fn list_arbitrage(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let arbs: Vec<_> = state
        .cache
        .get_all_active_arbs()
        .into_iter()
        .filter(|arb| {
            // Skip events whose game date has already passed
            if is_event_past(&arb.canonical_event_id) {
                return false;
            }
            // Skip events whose markets have settled (status or price heuristic)
            if let Some((event, Some(odds))) =
                state.cache.get_event_with_odds(&arb.canonical_event_id)
            {
                if is_event_settled_full(&event.status, &odds) {
                    return false;
                }
            }
            true
        })
        .collect();
    Json(serde_json::json!({
        "opportunities": arbs,
        "total": arbs.len(),
    }))
}

async fn sport_today(
    State(state): State<Arc<AppState>>,
    Path(sport): Path<String>,
) -> Json<serde_json::Value> {
    let events_with_odds = state.cache.get_all_events_with_odds();
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let responses: Vec<EventResponse> = events_with_odds
        .into_iter()
        .filter(|(event, _)| {
            event.sport.as_str() == sport.as_str() && event.id.contains(&today)
        })
        .map(|(event, odds)| build_event_response(&event, odds.as_ref(), &state))
        .collect();

    Json(serde_json::json!({
        "sport": sport,
        "date": today,
        "events": responses,
        "total": responses.len(),
    }))
}

/// Build an EventResponse from a CanonicalEvent and its odds.
fn build_event_response(
    event: &crate::models::event::CanonicalEvent,
    odds: Option<&crate::models::event::EventOdds>,
    state: &AppState,
) -> EventResponse {
    let kalshi = odds.and_then(|o| {
        o.platform_odds.get("kalshi").map(|outcomes| PlatformOddsResponse {
            outcomes: outcomes
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        OutcomePriceResponse {
                            yes_price: v.yes_price,
                            no_price: v.no_price,
                        },
                    )
                })
                .collect(),
            updated_at: Some(o.updated_at.to_rfc3339()),
        })
    });

    let polymarket = odds.and_then(|o| {
        o.platform_odds.get("polymarket").map(|outcomes| PlatformOddsResponse {
            outcomes: outcomes
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        OutcomePriceResponse {
                            yes_price: v.yes_price,
                            no_price: v.no_price,
                        },
                    )
                })
                .collect(),
            updated_at: Some(o.updated_at.to_rfc3339()),
        })
    });

    // Check for active arb on this event
    let arb_opportunity = state
        .cache
        .active_arbs
        .get(&event.id)
        .and_then(|arbs| {
            arbs.first().map(|arb| ArbOpportunityResponse {
                outcome: arb.outcome.clone(),
                buy_platform: arb.buy_platform.clone(),
                buy_price: arb.buy_price,
                sell_platform: arb.sell_platform.clone(),
                sell_price: arb.sell_price,
                margin_pct: arb.margin_pct,
                score: arb.score,
            })
        });

    EventResponse {
        id: event.id.clone(),
        sport: event.sport.as_str().to_string(),
        event_title: event.event_title.clone(),
        game_start_time: event.game_start_time.map(|t| t.to_rfc3339()),
        created_at: event.created_at.to_rfc3339(),
        status: event.status.clone(),
        match_score: event.match_score,
        kalshi,
        polymarket,
        arb_opportunity,
    }
}

// ─── Phase 3: Historical / Analytics endpoints ───────────────────────

#[derive(Deserialize)]
struct OddsHistoryParams {
    hours: Option<i32>,
}

async fn odds_history(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
    Query(params): Query<OddsHistoryParams>,
) -> Json<serde_json::Value> {
    let hours = params.hours.unwrap_or(24);
    match state.db.get_odds_history(&event_id, hours).await {
        Ok(snapshots) => Json(serde_json::json!({
            "event_id": event_id,
            "hours": hours,
            "snapshots": snapshots,
            "total": snapshots.len(),
        })),
        Err(e) => Json(serde_json::json!({
            "error": "Failed to fetch odds history",
            "message": e.to_string(),
        })),
    }
}

#[derive(Deserialize)]
struct ArbHistoryParams {
    event_id: Option<String>,
    limit: Option<i32>,
}

async fn arb_history(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ArbHistoryParams>,
) -> Json<serde_json::Value> {
    let limit = params.limit.unwrap_or(50);
    match state
        .db
        .get_arb_history(params.event_id.as_deref(), limit)
        .await
    {
        Ok(arbs) => Json(serde_json::json!({
            "history": arbs,
            "total": arbs.len(),
        })),
        Err(e) => Json(serde_json::json!({
            "error": "Failed to fetch arb history",
            "message": e.to_string(),
        })),
    }
}

async fn arb_stats(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match state.db.get_arb_stats().await {
        Ok(stats) => Json(stats),
        Err(e) => Json(serde_json::json!({
            "error": "Failed to fetch arb stats",
            "message": e.to_string(),
        })),
    }
}

async fn matching_stats(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let stats = state.cache.matching_stats.read().unwrap().clone();
    match stats {
        Some(s) => Json(serde_json::json!(s)),
        None => Json(serde_json::json!({
            "message": "No matching cycle has run yet",
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use crate::models::event::{OutcomePrice, PlatformIds, Sport};

    fn make_event() -> CanonicalEvent {
        CanonicalEvent {
            id: "nhl-bos-njd-2026-03-15".to_string(),
            sport: Sport::Nhl,
            event_title: "Bruins vs Devils".to_string(),
            game_start_time: None,
            status: "open".to_string(),
            platform_ids: PlatformIds {
                kalshi_event_ticker: None,
                kalshi_market_tickers: vec!["KX1".to_string(), "KX2".to_string()],
                polymarket_market_slug: None,
                polymarket_token_ids: vec!["p1".to_string(), "p2".to_string()],
                polymarket_outcome_labels: vec!["BOS".to_string(), "NJD".to_string()],
            },
            match_score: Some(85.0),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_price() -> OutcomePrice {
        OutcomePrice {
            yes_price: 0.5,
            no_price: 0.5,
            best_bid: None,
            best_ask: None,
            bid_size: None,
            ask_size: None,
        }
    }

    #[test]
    fn test_has_binary_outcomes_rejects_three_rendered_keys() {
        let event = make_event();
        let mut odds = EventOdds {
            canonical_event_id: event.id.clone(),
            platform_odds: HashMap::new(),
            updated_at: Utc::now(),
        };

        let mut poly = HashMap::new();
        poly.insert("BOS".to_string(), make_price());
        poly.insert("NJD".to_string(), make_price());
        poly.insert("bruins".to_string(), make_price());
        odds.platform_odds.insert("polymarket".to_string(), poly);

        assert!(!has_binary_outcomes(&event, Some(&odds)));
    }

    #[test]
    fn test_has_binary_outcomes_accepts_two_keys() {
        let event = make_event();
        let mut odds = EventOdds {
            canonical_event_id: event.id.clone(),
            platform_odds: HashMap::new(),
            updated_at: Utc::now(),
        };

        let mut poly = HashMap::new();
        poly.insert("BOS".to_string(), make_price());
        poly.insert("NJD".to_string(), make_price());
        odds.platform_odds.insert("polymarket".to_string(), poly);

        assert!(has_binary_outcomes(&event, Some(&odds)));
    }
}
