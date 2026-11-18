//! Front-month OI & volume change tracker.
//!
//! Distinguishes new positioning (volume + rising OI) from day-trading churn
//! (high volume, flat/falling OI) using UW contracts + session stats.

use crate::uw::{FuturesContract, FuturesStats};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct OiRow {
    pub product: String,
    pub front: String,
    pub back: Option<String>,
    pub front_volume: i64,
    pub back_volume: i64,
    pub open_interest: Option<i64>,
    pub session_volume: Option<i64>,
    pub session_high: Option<f64>,
    pub session_low: Option<f64>,
    pub settlement: Option<f64>,
    /// Session volume / OI — high turnover with no OI expansion ≈ churn.
    pub turnover: Option<f64>,
    pub regime: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OiMatrixReport {
    pub rows: Vec<OiRow>,
    pub narrative: String,
    pub methodology: String,
}

pub fn build_oi_matrix(
    contracts: &[FuturesContract],
    stats: &[(String, FuturesStats)],
) -> OiMatrixReport {
    let products = ["ES", "NQ", "RTY"];
    let stats_map: std::collections::HashMap<_, _> = stats
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let mut rows = Vec::new();
    for product in products {
        let mut chain: Vec<&FuturesContract> = contracts
            .iter()
            .filter(|c| c.product.as_deref() == Some(product) && c.is_spread != Some(true))
            .collect();
        chain.sort_by(|a, b| b.volume.unwrap_or(0).cmp(&a.volume.unwrap_or(0)));
        let Some(front) = chain.first() else {
            continue;
        };
        let back = chain.get(1).copied();
        let st = stats_map.get(&front.name);
        let oi = st.and_then(|s| s.open_interest);
        let session_volume = st.and_then(|s| s.session_volume);
        let turnover = match (session_volume, oi) {
            (Some(v), Some(o)) if o > 0 => Some(v as f64 / o as f64),
            _ => None,
        };
        let front_volume = front.volume.unwrap_or(0);
        let back_volume = back.and_then(|c| c.volume).unwrap_or(0);
        let roll_share = if front_volume + back_volume > 0 {
            front_volume as f64 / (front_volume + back_volume) as f64
        } else {
            1.0
        };

        let (regime, note) = classify(turnover, roll_share, session_volume, oi);

        rows.push(OiRow {
            product: product.into(),
            front: front.name.clone(),
            back: back.map(|c| c.name.clone()),
            front_volume,
            back_volume,
            open_interest: oi,
            session_volume,
            session_high: st
                .and_then(|s| s.session_high.as_ref())
                .and_then(|s| s.parse().ok()),
            session_low: st
                .and_then(|s| s.session_low.as_ref())
                .and_then(|s| s.parse().ok()),
            settlement: st
                .and_then(|s| s.settlement.as_ref())
                .and_then(|s| s.parse().ok()),
            turnover,
            regime: regime.into(),
            note: note.into(),
        });
    }

    let narrative = rows
        .iter()
        .map(|r| format!("{}={}", r.product, r.regime))
        .collect::<Vec<_>>()
        .join(" · ");

    OiMatrixReport {
        rows,
        narrative: if narrative.is_empty() {
            "No index futures OI rows available.".into()
        } else {
            format!("Positioning regimes: {narrative}")
        },
        methodology: "Front vs next contract volume from UW futures contracts. Session volume \
+ open interest from /stats. Turnover = session volume ÷ OI. High turnover without OI \
expansion reads as day-trade churn; volume with sticky OI reads as new positioning. \
Classic COT/structure framing traders already use — single-snapshot PoC until Kafka \
history lands."
            .into(),
    }
}

fn classify(
    turnover: Option<f64>,
    roll_share: f64,
    session_volume: Option<i64>,
    oi: Option<i64>,
) -> (&'static str, &'static str) {
    match (turnover, session_volume, oi) {
        (Some(t), Some(v), Some(_)) if t >= 0.35 && v > 500_000 => (
            "churn",
            "Heavy session volume vs OI — looks like short-horizon churn / liquidations, not sticky positioning.",
        ),
        (Some(t), Some(v), Some(_)) if t < 0.15 && v > 200_000 => (
            "positioning",
            "Volume with low turnover vs OI — consistent with new or held institutional positioning.",
        ),
        (Some(t), _, _) if t >= 0.2 => (
            "mixed",
            "Moderate turnover — mix of day flow and inventory adjustment.",
        ),
        _ if roll_share < 0.85 => (
            "roll",
            "Meaningful volume on the back month — watch the roll for false breakouts.",
        ),
        _ => (
            "quiet",
            "Light relative activity — wait for confirmation before treating moves as structural.",
        ),
    }
}
