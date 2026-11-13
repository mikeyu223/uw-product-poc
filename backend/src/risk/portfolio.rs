use super::engine::{MarketContext, PositionInput, score_position};
use super::PositionRisk;
use crate::uw::{GexSource, UwClient, UwError};
use serde::Serialize;
use tracing::{info, warn};

/// Sample options book for the PoC (stand-in until brokerage sync).
/// Strikes are chosen near live markets so GEX/flow explanations are meaningful.
pub fn sample_book() -> Vec<PositionInput> {
    vec![
        PositionInput {
            id: "pos-spy-p-740".into(),
            ticker: "SPY".into(),
            option_type: "put".into(),
            strike: 740.0,
            expiry: "2026-11-21".into(),
            quantity: 10,
            side: "long".into(),
            delta: -0.32,
            gamma: 0.014,
            vega: 0.40,
        },
        PositionInput {
            id: "pos-nvda-c-185".into(),
            ticker: "NVDA".into(),
            option_type: "call".into(),
            strike: 185.0,
            expiry: "2026-12-19".into(),
            quantity: 5,
            side: "long".into(),
            delta: 0.48,
            gamma: 0.022,
            vega: 0.55,
        },
        PositionInput {
            id: "pos-qqq-p-660".into(),
            ticker: "QQQ".into(),
            option_type: "put".into(),
            strike: 660.0,
            expiry: "2026-11-28".into(),
            quantity: 8,
            side: "long".into(),
            delta: -0.28,
            gamma: 0.010,
            vega: 0.38,
        },
        PositionInput {
            id: "pos-aapl-c-360".into(),
            ticker: "AAPL".into(),
            option_type: "call".into(),
            strike: 360.0,
            expiry: "2027-01-16".into(),
            quantity: 12,
            side: "long".into(),
            delta: 0.42,
            gamma: 0.013,
            vega: 0.58,
        },
        PositionInput {
            id: "pos-meta-c-650".into(),
            ticker: "META".into(),
            option_type: "call".into(),
            strike: 650.0,
            expiry: "2026-12-05".into(),
            quantity: 3,
            side: "long".into(),
            delta: 0.46,
            gamma: 0.017,
            vega: 0.50,
        },
    ]
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoredPosition {
    pub id: String,
    pub ticker: String,
    pub option_type: String,
    pub strike: f64,
    pub expiry: String,
    pub quantity: i32,
    pub side: String,
    pub underlying_price: f64,
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub gex_snapshot: GexSnapshot,
    #[serde(flatten)]
    pub risk: PositionRisk,
}

#[derive(Debug, Clone, Serialize)]
pub struct GexSnapshot {
    pub gamma_flip: Option<String>,
    pub call_wall: Option<String>,
    pub put_wall: Option<String>,
    pub gamma_magnet: Option<String>,
    pub net_gamma: Option<f64>,
    pub positive_gamma_regime: Option<bool>,
}

/// Pull live UW data per ticker and score the sample book.
/// Position scoring runs concurrently (bounded by UW latency, not sequential waits).
pub async fn score_book(uw: &UwClient) -> Result<Vec<ScoredPosition>, UwError> {
    let book = sample_book();
    let mut handles = Vec::with_capacity(book.len());

    for pos in book {
        let uw = uw.clone();
        handles.push(tokio::spawn(async move {
            let ticker = pos.ticker.clone();
            match score_one(&uw, &pos).await {
                Ok(scored) => Some(scored),
                Err(err) => {
                    warn!(ticker = %ticker, error = %err, "failed to score position — skipping");
                    None
                }
            }
        }));
    }

    let mut out = Vec::with_capacity(handles.len());
    for handle in handles {
        match handle.await {
            Ok(Some(scored)) => out.push(scored),
            Ok(None) => {}
            Err(err) => warn!(error = %err, "score task join failed"),
        }
    }

    // Keep stable sample-book order by id prefix / ticker order from sample_book
    let order: Vec<String> = sample_book().into_iter().map(|p| p.id).collect();
    out.sort_by_key(|p| {
        order
            .iter()
            .position(|id| id == &p.id)
            .unwrap_or(usize::MAX)
    });

    info!(count = out.len(), "scored positions from live UW data");
    Ok(out)
}

async fn score_one(uw: &UwClient, pos: &PositionInput) -> Result<ScoredPosition, UwError> {
    let ticker = &pos.ticker;

    let occ = occ_symbol(ticker, &pos.expiry, &pos.option_type, pos.strike);

    // Parallel UW calls for this ticker (was sequential before).
    let (state_r, gex_r, greek_r, flow_r, info_r, iv_r, iv_curve_r, rr_r, dp_r, tape_r) =
        tokio::join!(
            uw.stock_state(ticker),
            uw.gex_levels(ticker, GexSource::Vol),
            uw.latest_greek_exposure(ticker),
            uw.flow_alerts(ticker, 40),
            uw.ticker_info(ticker),
            uw.latest_iv_rank(ticker),
            uw.interpolated_iv(ticker),
            uw.latest_risk_reversal(ticker),
            uw.darkpool_price_levels(ticker),
            uw.option_contract_latest_tape(&occ),
        );

    let state = state_r?;
    let spot = state
        .close
        .as_ref()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| state.prev_close.as_ref().and_then(|s| s.parse().ok()))
        .unwrap_or(0.0);

    let gex = gex_r?;
    let greek = greek_r?;
    let flow = flow_r.unwrap_or_default();
    let info = info_r.ok();
    let iv_rank = iv_r.ok().flatten();
    let interpolated_iv_curve = iv_curve_r.unwrap_or_default();
    let risk_reversal = rr_r.ok().flatten();
    let dark_pool_levels = dp_r.unwrap_or_default();
    let contract_tape = tape_r.ok().flatten();

    let mkt = MarketContext {
        spot,
        gex: gex.clone(),
        greek: greek.clone(),
        flow,
        info,
        iv_rank,
        interpolated_iv_curve,
        risk_reversal,
        dark_pool_levels,
        contract_tape,
    };

    let risk = score_position(pos, &mkt);

    Ok(ScoredPosition {
        id: pos.id.clone(),
        ticker: pos.ticker.clone(),
        option_type: pos.option_type.clone(),
        strike: pos.strike,
        expiry: pos.expiry.clone(),
        quantity: pos.quantity,
        side: pos.side.clone(),
        underlying_price: spot,
        delta: pos.delta,
        gamma: pos.gamma,
        vega: pos.vega,
        gex_snapshot: GexSnapshot {
            gamma_flip: gex.gamma_flip.clone(),
            call_wall: gex.call_wall.clone(),
            put_wall: gex.put_wall.clone(),
            gamma_magnet: gex.gamma_magnet.clone(),
            net_gamma: greek.as_ref().and_then(|g| g.net_gamma()),
            positive_gamma_regime: greek.as_ref().and_then(|g| g.is_positive_gamma_regime()),
        },
        risk,
    })
}

/// OCC option symbol: `NVDA261218C00185000`.
fn occ_symbol(ticker: &str, expiry: &str, option_type: &str, strike: f64) -> String {
    let date = expiry.get(..10).unwrap_or(expiry);
    let yymmdd = date
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>();
    let yymmdd = if yymmdd.len() >= 8 {
        yymmdd[yymmdd.len() - 6..].to_string()
    } else {
        yymmdd
    };
    let cp = if option_type.eq_ignore_ascii_case("put") {
        'P'
    } else {
        'C'
    };
    let strike_i = (strike * 1000.0).round() as i64;
    format!(
        "{t}{yymmdd}{cp}{strike_i:08}",
        t = ticker.to_ascii_uppercase()
    )
}
