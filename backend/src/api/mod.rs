//! Axum HTTP API surface for the dashboard.
//!
//! - `GET /health`
//! - `GET /api/v1/book` — cached scored book (preferred)
//! - `GET /api/v1/portfolio` — summary derived from cached book
//! - `GET /api/v1/positions` — positions from cached book
//! - `GET /api/v1/uw/probe/:ticker`
//! - `GET /api/v1/methodology`
//!
//! Pass `?refresh=1` to bypass the TTL cache.

use crate::cache::{BookCache, CachedBook};
use crate::config::Config;
use crate::risk::{RiskBucket, ScoredPosition, driver_catalog, family_catalog};
use crate::uw::UwClient;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub uw: UwClient,
    pub book_cache: BookCache,
}

#[derive(Debug, Deserialize)]
pub struct RefreshQuery {
    /// When `1`/`true`, force a live Unusual Whales refresh.
    pub refresh: Option<String>,
}

impl RefreshQuery {
    fn force(&self) -> bool {
        matches!(
            self.refresh.as_deref().map(|s| s.to_ascii_lowercase()).as_deref(),
            Some("1") | Some("true") | Some("yes")
        )
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/book", get(book))
        .route("/api/v1/portfolio", get(portfolio))
        .route("/api/v1/positions", get(positions))
        .route("/api/v1/methodology", get(methodology))
        .route("/api/v1/drivers", get(drivers_catalog))
        .route("/api/v1/uw/probe/{ticker}", get(uw_probe))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state))
}

async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "risk-watcher",
        "cache_ttl_secs": 45
    }))
}

async fn methodology() -> impl IntoResponse {
    Json(json!({
        "audience": "options_traders",
        "score_range": "0-100",
        "display": "Each driver and family shows contribution/max (e.g. 4/20). Aggregate = sum of family points.",
        "buckets": {
            "low": "0-24",
            "moderate": "25-49",
            "elevated": "50-74",
            "high": "75-100"
        },
        "families": family_catalog(),
        "drivers": driver_catalog(),
        "caching": { "ttl_secs": 45 },
        "note": "Hand-tuned mathematical proximity-to-harm drivers (no ML). Flip proximity capped at 20."
    }))
}

async fn drivers_catalog() -> impl IntoResponse {
    Json(json!({
        "families": family_catalog(),
        "drivers": driver_catalog(),
        "score_max": 100,
        "principle": "Every driver is a mathematical distance/intensity of structural harm. Contribution asymptotes to max_points via log/sigmoid/exp curves."
    }))
}

async fn book(
    State(state): State<Arc<AppState>>,
    Query(q): Query<RefreshQuery>,
) -> impl IntoResponse {
    match state.book_cache.get(&state.uw, q.force()).await {
        Ok(cached) => Json(book_payload(&cached)).into_response(),
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

async fn portfolio(
    State(state): State<Arc<AppState>>,
    Query(q): Query<RefreshQuery>,
) -> impl IntoResponse {
    match state.book_cache.get(&state.uw, q.force()).await {
        Ok(cached) => Json(json!({ "summary": summary_from_book(&cached) })).into_response(),
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

async fn positions(
    State(state): State<Arc<AppState>>,
    Query(q): Query<RefreshQuery>,
) -> impl IntoResponse {
    match state.book_cache.get(&state.uw, q.force()).await {
        Ok(cached) => Json(book_payload(&cached)).into_response(),
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

fn book_payload(cached: &CachedBook) -> serde_json::Value {
    json!({
        "positions": cached.positions,
        "mode": "live_uw",
        "cache": {
            "hit": cached.cache_hit,
            "age_secs": cached.age_secs,
            "ttl_secs": cached.ttl_secs,
            "fetched_at": cached.fetched_at.to_rfc3339(),
        },
        "summary": summary_from_book(cached),
        "note": "Sample portfolio strikes; scores from UW with server-side TTL cache"
    })
}

fn summary_from_book(cached: &CachedBook) -> serde_json::Value {
    let positions = &cached.positions;
    let count = positions.len();
    let avg = if count == 0 {
        0.0
    } else {
        positions.iter().map(|p| p.risk.score as f64).sum::<f64>() / count as f64
    };
    let avg_u8 = avg.round().clamp(0.0, 100.0) as u8;
    let bucket = RiskBucket::from_score(avg_u8);
    let high_risk = positions.iter().filter(|p| p.risk.score >= 50).count();

    let mut top_drivers: Vec<(&ScoredPosition, &crate::risk::RiskDriver)> = positions
        .iter()
        .flat_map(|p| p.risk.drivers.iter().map(move |d| (p, d)))
        .collect();
    top_drivers.sort_by(|a, b| {
        b.1.contribution
            .partial_cmp(&a.1.contribution)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top_drivers_json: Vec<_> = top_drivers
        .into_iter()
        .take(5)
        .map(|(p, d)| {
            json!({
                "ticker": p.ticker,
                "label": d.label,
                "contribution": d.contribution,
                "detail": d.detail,
                "family": d.family,
            })
        })
        .collect();

    json!({
        "position_count": count,
        "avg_risk_score": avg_u8,
        "bucket": bucket,
        "bucket_label": bucket.label(),
        "elevated_or_high": high_risk,
        "top_drivers": top_drivers_json,
        "updated_at": cached.fetched_at.to_rfc3339(),
        "mode": "live_uw",
        "cache_hit": cached.cache_hit,
        "cache_age_secs": cached.age_secs,
        "cache_ttl_secs": cached.ttl_secs,
        "methodology": "Dealer GEX (26) + flip (20) + walls (16) + adverse flow (20) + structure (10) + earnings (8)"
    })
}

async fn uw_probe(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
) -> impl IntoResponse {
    match state.uw.probe(&ticker).await {
        Ok(report) => {
            let body = json!({
                "ticker": report.ticker,
                "info": report.info,
                "gex": report.gex,
                "latest_greek": report.latest_greek,
                "recent_alerts": report.recent_alerts,
            });
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(err) => {
            let body = json!({ "error": err.to_string() });
            (StatusCode::BAD_GATEWAY, Json(body)).into_response()
        }
    }
}
