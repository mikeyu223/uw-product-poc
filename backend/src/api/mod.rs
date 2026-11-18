//! Axum HTTP API — futures order-flow suite + options pitches + trading journal.

use crate::config::Config;
use crate::cvd::build_profile;
use crate::journal::{
    JournalFilter, JournalStore, NewTrade, build_analytics_filtered, build_trade_chart,
    enrich_many, enrich_trade, from_new,
};
use crate::options::{build_earnings_iv, build_sweep_clusters, earnings_iv::EarningsInputs};
use crate::structure::{
    FuturesGex, build_futures_gex, build_gex_heatmap, build_oi_matrix, gex_proxies, proxy_scale,
};
use crate::uw::{FuturesContract, GexSource, UwClient};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

const FOCUS_PRODUCTS: &[&str] = &["ES", "NQ", "MES", "MNQ", "RTY"];
const PRIMARY_ORDER: &[&str] = &["ES", "NQ", "RTY", "MES", "MNQ"];
const DEFAULT_EARNINGS_WATCH: &[&str] =
    &["NVDA", "TSLA", "META", "AAPL", "AMD", "AMZN", "MSFT", "GOOGL"];

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub uw: UwClient,
    pub journal: JournalStore,
}

#[derive(Debug, Deserialize)]
pub struct CvdQuery {
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct EarningsQuery {
    /// Comma-separated tickers (default mega-cap watchlist).
    pub tickers: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SweepsQuery {
    pub limit: Option<u32>,
    /// YYYY-MM-DD override for 0DTE session date.
    pub date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CsvImportBody {
    pub csv: String,
}

#[derive(Debug, Deserialize)]
pub struct TradePatchBody {
    pub notes: Option<String>,
    pub emotion: Option<String>,
    pub setup: Option<String>,
    pub tags: Option<Vec<String>>,
    pub lesson: Option<String>,
    pub followed_rules: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct JournalQuery {
    pub account: Option<String>,
    pub setup: Option<String>,
    pub tod: Option<String>,
    pub dow: Option<String>,
    pub strategy: Option<String>,
    pub dte_bucket: Option<String>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/futures/contracts", get(contracts))
        .route("/api/v1/futures/{contract}/cvd", get(cvd_profile))
        .route("/api/v1/futures/oi-matrix", get(oi_matrix))
        .route("/api/v1/options/earnings-iv", get(earnings_iv))
        .route("/api/v1/options/sweep-clusters", get(sweep_clusters))
        .route("/api/v1/journal", get(journal_list).post(journal_add))
        .route("/api/v1/journal/analytics", get(journal_analytics))
        .route("/api/v1/journal/import", post(journal_import))
        .route("/api/v1/journal/{id}/chart", get(journal_chart))
        .route("/api/v1/journal/{id}", post(journal_patch))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state))
}

async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "uw-product-poc",
        "features": [
            "futures_tape_cvd",
            "volume_profile_poc",
            "globex_rth_levels",
            "oi_matrix",
            "futures_gex_zero_gamma",
            "earnings_iv_crush",
            "zero_dte_sweep_clusters",
            "trading_journal"
        ]
    }))
}

async fn contracts(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.uw.futures_contracts().await {
        Ok(all) => {
            let mut focused: Vec<FuturesContract> = all
                .into_iter()
                .filter(|c| {
                    c.is_spread != Some(true)
                        && c.product
                            .as_deref()
                            .map(|p| FOCUS_PRODUCTS.iter().any(|f| *f == p))
                            .unwrap_or(false)
                })
                .collect();
            focused.sort_by(|a, b| {
                b.volume
                    .unwrap_or(0)
                    .cmp(&a.volume.unwrap_or(0))
                    .then_with(|| a.name.cmp(&b.name))
            });
            let mut seen = std::collections::HashSet::new();
            let mut front: Vec<_> = focused
                .into_iter()
                .filter(|c| {
                    let product = c.product.clone().unwrap_or_default();
                    seen.insert(product)
                })
                .collect();
            front.sort_by_key(|c| {
                PRIMARY_ORDER
                    .iter()
                    .position(|p| Some(*p) == c.product.as_deref())
                    .unwrap_or(99)
            });
            Json(json!({ "contracts": front })).into_response()
        }
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

async fn cvd_profile(
    State(state): State<Arc<AppState>>,
    Path(contract): Path<String>,
    Query(q): Query<CvdQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(500).clamp(50, 1000);
    let contract = contract.trim().to_ascii_uppercase();

    let contracts = match state.uw.futures_contracts().await {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": err.to_string() })),
            )
                .into_response();
        }
    };

    let meta = contracts
        .iter()
        .find(|c| c.name.eq_ignore_ascii_case(&contract))
        .cloned()
        .unwrap_or(FuturesContract {
            name: contract.clone(),
            product: None,
            expiry: None,
            security_id: None,
            tick_size: Some("0.25".into()),
            tick_value: None,
            multiplier: None,
            volume: None,
            trades: None,
            high: None,
            low: None,
            last_trade: None,
            is_spread: Some(false),
        });

    let product = meta
        .product
        .clone()
        .unwrap_or_else(|| infer_product(&contract));
    let proxies = gex_proxies(&product);

    let (trades_r, stats_r, candles_r) = tokio::join!(
        state.uw.futures_trades(&contract, limit),
        state.uw.futures_stats(&contract),
        state.uw.futures_candles(&contract, "5m", "1d"),
    );

    let trades = match trades_r {
        Ok(t) => t,
        Err(err) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": format!("trades: {err}") })),
            )
                .into_response();
        }
    };
    let candles = candles_r.unwrap_or_default();
    let stats = stats_r.ok();

    // Futures last from tape, else contract last_trade.
    let futures_last = trades
        .first()
        .and_then(|t| t.price_f64())
        .or_else(|| meta.last_trade.as_ref().and_then(|s| s.parse().ok()));

    let futures_gex =
        load_futures_gex(&state.uw, &product, proxies, futures_last).await;

    let profile = build_profile(&meta, &trades, stats, &candles, futures_gex);
    Json(json!({ "profile": profile })).into_response()
}

async fn load_futures_gex(
    uw: &UwClient,
    product: &str,
    proxies: &[&str],
    futures_last: Option<f64>,
) -> Option<FuturesGex> {
    for proxy in proxies {
        let (gex_r, greek_r, spot_r, expiry_r, strike_r, iv_r) = tokio::join!(
            uw.gex_levels(proxy, GexSource::Vol),
            uw.latest_greek_exposure(proxy),
            uw.stock_state(proxy),
            uw.greek_exposure_by_expiry(proxy),
            uw.greek_exposure_by_strike(proxy),
            uw.interpolated_iv_near_30d(proxy),
        );
        let Ok(gex) = gex_r else {
            continue;
        };
        let proxy_spot = spot_r.ok().and_then(|s| {
            s.close
                .as_ref()
                .and_then(|x| x.parse().ok())
                .or_else(|| s.prev_close.as_ref().and_then(|x| x.parse().ok()))
        });
        let greek = greek_r.ok().flatten();
        let expiry_rows = expiry_r.unwrap_or_default();
        let strike_sum = strike_r.unwrap_or_default();

        // Fetch strike×expiry grids for nearest ~8 expiries.
        let mut near: Vec<_> = expiry_rows
            .iter()
            .filter(|e| e.expiry.is_some())
            .collect();
        near.sort_by_key(|e| e.dte.unwrap_or(999));
        near.truncate(8);

        let mut handles = Vec::new();
        for e in &near {
            let exp = e.expiry_date();
            let dte = e.dte;
            let uw = uw.clone();
            let proxy = (*proxy).to_string();
            handles.push(tokio::spawn(async move {
                let rows = uw
                    .greek_exposure_by_strike_expiry(&proxy, &exp)
                    .await
                    .unwrap_or_default();
                (exp, dte, rows)
            }));
        }
        let mut by_expiry = Vec::new();
        for h in handles {
            if let Ok(row) = h.await {
                by_expiry.push(row);
            }
        }

        let implied_move = iv_r.ok().flatten().and_then(|p| {
            p.implied_move_perc
                .as_ref()
                .and_then(|s| s.parse::<f64>().ok())
                .map(|v| if v <= 1.0 { v * 100.0 } else { v })
        });

        let heatmap = if !strike_sum.is_empty() || !by_expiry.is_empty() {
            Some(build_gex_heatmap(
                proxy,
                proxy_spot.or(futures_last),
                proxy_scale(proxy),
                &expiry_rows,
                &strike_sum,
                &by_expiry,
                implied_move,
            ))
        } else {
            None
        };

        return Some(build_futures_gex(
            product,
            futures_last,
            proxy,
            proxy_spot,
            &gex,
            greek.as_ref(),
            heatmap,
        ));
    }
    None
}

fn infer_product(contract: &str) -> String {
    let c = contract.to_ascii_uppercase();
    for p in ["MES", "MNQ", "M2K", "ES", "NQ", "RTY"] {
        if c.starts_with(p) {
            return p.into();
        }
    }
    c.chars().take(2).collect()
}

async fn oi_matrix(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let contracts = match state.uw.futures_contracts().await {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": err.to_string() })),
            )
                .into_response();
        }
    };

    // Front-month ES/NQ/RTY by volume for stats fetch.
    let mut fronts = Vec::new();
    for product in ["ES", "NQ", "RTY"] {
        if let Some(c) = contracts
            .iter()
            .filter(|c| c.product.as_deref() == Some(product) && c.is_spread != Some(true))
            .max_by_key(|c| c.volume.unwrap_or(0))
        {
            fronts.push(c.name.clone());
        }
    }

    let mut handles = Vec::new();
    for name in fronts {
        let uw = state.uw.clone();
        handles.push(tokio::spawn(async move {
            let stats = uw.futures_stats(&name).await.ok()?;
            Some((name, stats))
        }));
    }

    let mut stats = Vec::new();
    for h in handles {
        if let Ok(Some(row)) = h.await {
            stats.push(row);
        }
    }

    let report = build_oi_matrix(&contracts, &stats);
    Json(json!({ "report": report })).into_response()
}

async fn earnings_iv(
    State(state): State<Arc<AppState>>,
    Query(q): Query<EarningsQuery>,
) -> impl IntoResponse {
    let tickers: Vec<String> = q
        .tickers
        .as_deref()
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_ascii_uppercase())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_EARNINGS_WATCH.iter().map(|s| (*s).to_string()).collect());

    let mut handles = Vec::with_capacity(tickers.len());
    for ticker in tickers {
        let uw = state.uw.clone();
        handles.push(tokio::spawn(async move {
            let (info_r, state_r, term_r, rank_r) = tokio::join!(
                uw.ticker_info(&ticker),
                uw.stock_state(&ticker),
                uw.interpolated_iv(&ticker),
                uw.iv_rank(&ticker, "1y"),
            );
            let info = info_r.ok()?;
            let spot = state_r.ok().and_then(|s| {
                s.close
                    .as_ref()
                    .and_then(|x| x.parse().ok())
                    .or_else(|| s.prev_close.as_ref().and_then(|x| x.parse().ok()))
            });
            Some(EarningsInputs {
                info,
                spot,
                term: term_r.unwrap_or_default(),
                iv_rank_series: rank_r.unwrap_or_default(),
            })
        }));
    }

    let mut rows = Vec::new();
    for h in handles {
        if let Ok(Some(row)) = h.await {
            rows.push(row);
        }
    }
    if rows.is_empty() {
        return (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": "no earnings IV data returned from UW" })),
        )
            .into_response();
    }

    let report = build_earnings_iv(rows);
    Json(json!({ "report": report })).into_response()
}

async fn sweep_clusters(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SweepsQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(200).clamp(50, 500);
    match state.uw.market_flow_alerts(limit).await {
        Ok(alerts) => {
            let report = build_sweep_clusters(&alerts, q.date.as_deref());
            Json(json!({ "report": report })).into_response()
        }
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

async fn journal_list(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let trades = state.journal.list().await;
    Json(json!({ "trades": trades })).into_response()
}

async fn journal_analytics(
    State(state): State<Arc<AppState>>,
    Query(q): Query<JournalQuery>,
) -> impl IntoResponse {
    let trades = state.journal.list().await;
    let filter = JournalFilter {
        account: q.account.filter(|s| !s.is_empty()),
        setup: q.setup.filter(|s| !s.is_empty()),
        tod: q.tod.filter(|s| !s.is_empty()),
        dow: q.dow.filter(|s| !s.is_empty()),
        strategy: q.strategy.filter(|s| !s.is_empty()),
        dte_bucket: q.dte_bucket.filter(|s| !s.is_empty()),
    };
    let analytics = build_analytics_filtered(&trades, &filter);
    Json(json!({ "analytics": analytics })).into_response()
}

async fn journal_add(
    State(state): State<Arc<AppState>>,
    Json(body): Json<NewTrade>,
) -> impl IntoResponse {
    let mut trade = from_new(body);
    enrich_trade(&state.uw, &mut trade).await;
    state.journal.add(trade.clone()).await;
    (StatusCode::CREATED, Json(json!({ "trade": trade }))).into_response()
}

async fn journal_import(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CsvImportBody>,
) -> impl IntoResponse {
    match state.journal.import_csv(&body.csv).await {
        Ok(mut imported) => {
            enrich_many(&state.uw, &mut imported).await;
            let count = imported.len();
            state.journal.add_many(imported).await;
            let trades = state.journal.list().await;
            let analytics = build_analytics_filtered(&trades, &JournalFilter::default());
            Json(json!({ "imported": count, "analytics": analytics, "trades": trades }))
                .into_response()
        }
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": err })),
        )
            .into_response(),
    }
}

async fn journal_chart(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.journal.get(&id).await {
        Some(trade) => {
            let chart = build_trade_chart(&state.uw, &trade).await;
            Json(json!({ "chart": chart })).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "trade not found" })),
        )
            .into_response(),
    }
}

async fn journal_patch(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<TradePatchBody>,
) -> impl IntoResponse {
    match state
        .journal
        .update_notes(
            &id,
            body.notes,
            body.emotion,
            body.setup,
            body.tags,
            body.lesson,
            body.followed_rules,
        )
        .await
    {
        Some(trade) => Json(json!({ "trade": trade })).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "trade not found" })),
        )
            .into_response(),
    }
}
