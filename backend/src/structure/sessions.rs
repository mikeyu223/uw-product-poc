//! Overnight Globex vs RTH range levels — staple morning routine for index futures.

use crate::uw::{FuturesCandle, FuturesStats};
use chrono::{Datelike, FixedOffset, NaiveDateTime, Timelike, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SessionLevels {
    pub globex_high: Option<f64>,
    pub globex_low: Option<f64>,
    pub globex_mid: Option<f64>,
    pub settlement: Option<f64>,
    pub settlement_date: Option<String>,
    pub rth_open: Option<f64>,
    pub rth_high: Option<f64>,
    pub rth_low: Option<f64>,
    pub rth_last: Option<f64>,
    pub opening_range_high: Option<f64>,
    pub opening_range_low: Option<f64>,
    pub in_rth: bool,
    pub bias_vs_globex: String,
    pub levels: Vec<NamedLevel>,
    pub narrative: String,
    pub methodology: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NamedLevel {
    pub name: String,
    pub price: f64,
    pub kind: String,
}

pub fn build_session_levels(candles: &[FuturesCandle], stats: Option<&FuturesStats>) -> SessionLevels {
    let et = FixedOffset::west_opt(5 * 3600).unwrap(); // EST — fine for Nov PoC
    let mut globex_h: Option<f64> = None;
    let mut globex_l: Option<f64> = None;
    let mut rth_h: Option<f64> = None;
    let mut rth_l: Option<f64> = None;
    let mut rth_open: Option<f64> = None;
    let mut rth_last: Option<f64> = None;
    let mut or_h: Option<f64> = None;
    let mut or_l: Option<f64> = None;
    let mut saw_rth = false;

    for c in candles {
        let Some(start) = c.start.as_ref() else {
            continue;
        };
        let Some(dt) = parse_utc(start) else {
            continue;
        };
        let local = dt.with_timezone(&et);
        let mins = local.hour() * 60 + local.minute();
        let is_rth = (9 * 60 + 30..16 * 60).contains(&mins) && local.weekday().number_from_monday() <= 5;
        let high = c.high_f64();
        let low = c.low_f64();
        let open = c.open_f64();
        let close = c.close_f64();

        if is_rth {
            saw_rth = true;
            if rth_open.is_none() {
                rth_open = open.or(close);
            }
            rth_last = close.or(rth_last);
            if let Some(h) = high {
                rth_h = Some(rth_h.map(|x| x.max(h)).unwrap_or(h));
            }
            if let Some(l) = low {
                rth_l = Some(rth_l.map(|x| x.min(l)).unwrap_or(l));
            }
            // Opening range: first 30 minutes (9:30–10:00 ET)
            if mins < 10 * 60 {
                if let Some(h) = high {
                    or_h = Some(or_h.map(|x| x.max(h)).unwrap_or(h));
                }
                if let Some(l) = low {
                    or_l = Some(or_l.map(|x| x.min(l)).unwrap_or(l));
                }
            }
        } else {
            if let Some(h) = high {
                globex_h = Some(globex_h.map(|x| x.max(h)).unwrap_or(h));
            }
            if let Some(l) = low {
                globex_l = Some(globex_l.map(|x| x.min(l)).unwrap_or(l));
            }
        }
    }

    let settlement = stats
        .and_then(|s| s.settlement.as_ref())
        .and_then(|s| s.parse().ok());
    let settlement_date = stats.and_then(|s| s.settlement_date.clone());
    let globex_mid = match (globex_h, globex_l) {
        (Some(h), Some(l)) => Some((h + l) / 2.0),
        _ => None,
    };

    let now_et = Utc::now().with_timezone(&et);
    let now_mins = now_et.hour() * 60 + now_et.minute();
    let in_rth = (9 * 60 + 30..16 * 60).contains(&now_mins)
        && now_et.weekday().number_from_monday() <= 5;

    let bias_vs_globex = match (rth_last.or(rth_open), globex_mid) {
        (Some(px), Some(mid)) if px > mid => "above_globex_mid",
        (Some(px), Some(mid)) if px < mid => "below_globex_mid",
        _ => "n/a",
    }
    .to_string();

    let mut levels = Vec::new();
    push_level(&mut levels, "Globex High", globex_h, "globex");
    push_level(&mut levels, "Globex Mid", globex_mid, "globex");
    push_level(&mut levels, "Globex Low", globex_l, "globex");
    push_level(&mut levels, "Settlement", settlement, "settlement");
    push_level(&mut levels, "OR High", or_h, "opening_range");
    push_level(&mut levels, "OR Low", or_l, "opening_range");
    push_level(&mut levels, "RTH High", rth_h, "rth");
    push_level(&mut levels, "RTH Low", rth_l, "rth");

    let narrative = format!(
        "Globex range {}–{} · settlement {} · RTH open {} · opening range {}–{}. {}",
        fmt(globex_l),
        fmt(globex_h),
        fmt(settlement),
        fmt(rth_open),
        fmt(or_l),
        fmt(or_h),
        if saw_rth {
            "RTH levels updating from live candles."
        } else {
            "Pre-RTH: use Globex balance + settlement for the open."
        }
    );

    SessionLevels {
        globex_high: globex_h,
        globex_low: globex_l,
        globex_mid,
        settlement,
        settlement_date,
        rth_open,
        rth_high: rth_h,
        rth_low: rth_l,
        rth_last,
        opening_range_high: or_h,
        opening_range_low: or_l,
        in_rth,
        bias_vs_globex,
        levels,
        narrative,
        methodology: "Candles split by America/New_York clock: RTH 09:30–16:00, else Globex. \
Opening range = first 30 minutes of RTH. Settlement from UW session stats. Classic \
levels index traders mark every morning — no custom indicators."
            .into(),
    }
}

fn push_level(out: &mut Vec<NamedLevel>, name: &str, price: Option<f64>, kind: &str) {
    if let Some(price) = price {
        out.push(NamedLevel {
            name: name.into(),
            price,
            kind: kind.into(),
        });
    }
}

fn parse_utc(s: &str) -> Option<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
        .or_else(|| {
            NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ")
                .ok()
                .map(|n| n.and_utc())
        })
}

fn fmt(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.2}"))
        .unwrap_or_else(|| "—".into())
}
