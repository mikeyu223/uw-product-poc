//! Earnings IV Crush Visualizer.
//!
//! Uses UW ticker info + interpolated IV term structure + IV rank history —
//! data traders already trust — to show how much premium is priced into the
//! front of the curve vs post-event. No invented math: front/back IV ratio,
//! days-to-earnings, IV rank, implied move.

use crate::uw::{InterpolatedIvPoint, IvRankPoint, TickerInfo};
use chrono::{NaiveDate, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TermPoint {
    pub days: i64,
    pub iv: f64,
    pub implied_move_pct: Option<f64>,
    pub percentile: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IvHistoryPoint {
    pub date: String,
    pub iv: f64,
    pub iv_rank: Option<f64>,
    pub close: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EarningsName {
    pub ticker: String,
    pub name: Option<String>,
    pub sector: Option<String>,
    pub earnings_date: Option<String>,
    pub earnings_confirmed: bool,
    pub days_to_earnings: Option<i64>,
    pub spot: Option<f64>,
    pub iv_rank: Option<f64>,
    pub front_iv: Option<f64>,
    pub iv_30d: Option<f64>,
    /// front_iv / iv_30d — elevated ahead of earnings = crush fuel.
    pub crush_ratio: Option<f64>,
    pub implied_move_pct: Option<f64>,
    pub crush_pressure: u8,
    pub crush_label: String,
    pub term: Vec<TermPoint>,
    pub iv_history: Vec<IvHistoryPoint>,
    pub takeaway: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EarningsIvReport {
    pub as_of: String,
    pub names: Vec<EarningsName>,
    pub narrative: String,
    pub methodology: String,
}

pub struct EarningsInputs {
    pub info: TickerInfo,
    pub spot: Option<f64>,
    pub term: Vec<InterpolatedIvPoint>,
    pub iv_rank_series: Vec<IvRankPoint>,
}

pub fn build_earnings_iv(rows: Vec<EarningsInputs>) -> EarningsIvReport {
    let today = Utc::now().date_naive();
    let mut names: Vec<EarningsName> = rows.into_iter().map(|r| score_name(r, today)).collect();
    names.sort_by(|a, b| b.crush_pressure.cmp(&a.crush_pressure));

    let hot = names
        .iter()
        .filter(|n| n.crush_pressure >= 60)
        .map(|n| n.ticker.as_str())
        .take(3)
        .collect::<Vec<_>>();
    let narrative = if hot.is_empty() {
        "No extreme front-end IV crush setups in this watchlist right now — still useful for event prep."
            .into()
    } else {
        format!(
            "Highest crush pressure: {}. Front-month IV is rich vs 30d — classic post-print vol sell setup.",
            hot.join(", ")
        )
    };

    EarningsIvReport {
        as_of: Utc::now().to_rfc3339(),
        names,
        narrative,
        methodology: "UW ticker info (earnings date) + interpolated IV term structure + 1y IV rank. \
Crush ratio = front IV ÷ 30d IV. Pressure blends days-to-event, crush ratio, and IV rank — \
the same inputs desk traders already eyeball; visualized cleanly for the options flow audience."
            .into(),
    }
}

fn score_name(row: EarningsInputs, today: NaiveDate) -> EarningsName {
    let ticker = row.info.symbol.clone();
    let earnings_date = row.info.next_earnings_date.clone();
    let confirmed = row.info.next_earnings_confirmed.unwrap_or(false);
    let dte = earnings_date
        .as_ref()
        .and_then(|s| NaiveDate::parse_from_str(&s[..10.min(s.len())], "%Y-%m-%d").ok())
        .map(|d| (d - today).num_days());

    let mut term: Vec<TermPoint> = row
        .term
        .iter()
        .filter_map(|p| {
            Some(TermPoint {
                days: p.days?,
                iv: p.volatility_f64()?,
                implied_move_pct: p
                    .implied_move_perc
                    .as_ref()
                    .and_then(|s| s.parse().ok()),
                percentile: p.percentile_pct(),
            })
        })
        .collect();
    term.sort_by_key(|t| t.days);

    let front_iv = term
        .iter()
        .find(|t| t.days <= 7)
        .or_else(|| term.first())
        .map(|t| t.iv);
    let iv_30d = term
        .iter()
        .min_by_key(|t| (t.days - 30).abs())
        .map(|t| t.iv);
    let implied_move_pct = term
        .iter()
        .find(|t| t.days <= 7)
        .and_then(|t| t.implied_move_pct)
        .or_else(|| term.first().and_then(|t| t.implied_move_pct));

    let crush_ratio = match (front_iv, iv_30d) {
        (Some(f), Some(b)) if b > 0.0 => Some(f / b),
        _ => None,
    };

    let latest_rank = row.iv_rank_series.last().and_then(|p| p.iv_rank_pct());

    let iv_history: Vec<IvHistoryPoint> = row
        .iv_rank_series
        .iter()
        .rev()
        .take(60)
        .filter_map(|p| {
            Some(IvHistoryPoint {
                date: p.date.clone()?,
                iv: p.volatility_f64()?,
                iv_rank: p.iv_rank_pct(),
                close: p.close.as_ref().and_then(|s| s.parse().ok()),
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let crush_pressure = crush_pressure_score(dte, crush_ratio, latest_rank);
    let crush_label = match crush_pressure {
        0..=24 => "Quiet",
        25..=49 => "Building",
        50..=74 => "Elevated",
        _ => "Crush setup",
    }
    .to_string();

    let takeaway = match (dte, crush_ratio, latest_rank) {
        (Some(d), Some(r), Some(rank)) if d <= 5 && r >= 1.25 => format!(
            "{ticker}: earnings in {d}d, front IV {r:.2}× the 30d wing, IV rank {rank:.0}. Expect post-print crush if realized move ≤ implied."
        ),
        (Some(d), Some(r), _) => format!(
            "{ticker}: {d}d to earnings, front/30d IV ratio {r:.2}. Watch whether the term flattens into the print."
        ),
        (None, Some(r), _) => format!(
            "{ticker}: no dated earnings on file; term still shows front/30d ratio {r:.2}."
        ),
        _ => format!("{ticker}: incomplete IV surface — refresh UW term structure."),
    };

    EarningsName {
        ticker,
        name: row.info.full_name.or(row.info.short_name),
        sector: row.info.sector,
        earnings_date,
        earnings_confirmed: confirmed,
        days_to_earnings: dte,
        spot: row.spot,
        iv_rank: latest_rank,
        front_iv,
        iv_30d,
        crush_ratio,
        implied_move_pct,
        crush_pressure,
        crush_label,
        term,
        iv_history,
        takeaway,
    }
}

fn crush_pressure_score(dte: Option<i64>, ratio: Option<f64>, iv_rank: Option<f64>) -> u8 {
    let mut pts: f64 = 0.0;
    if let Some(d) = dte {
        if (0..=2).contains(&d) {
            pts += 40.0;
        } else if (3..=7).contains(&d) {
            pts += 28.0;
        } else if (8..=21).contains(&d) {
            pts += 14.0;
        } else if d < 0 && d >= -3 {
            pts += 22.0; // just printed — crush in progress
        }
    }
    if let Some(r) = ratio {
        if r >= 1.6 {
            pts += 35.0;
        } else if r >= 1.35 {
            pts += 26.0;
        } else if r >= 1.15 {
            pts += 14.0;
        }
    }
    if let Some(rank) = iv_rank {
        if rank >= 80.0 {
            pts += 25.0;
        } else if rank >= 60.0 {
            pts += 15.0;
        } else if rank >= 40.0 {
            pts += 6.0;
        }
    }
    pts.round().clamp(0.0, 100.0) as u8
}
