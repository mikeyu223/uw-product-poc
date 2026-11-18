//! UW-style GEX heatmap: strike × expiry net gamma grid.
//! Built from `/greek-exposure/expiry`, `/strike`, and `/strike-expiry`.

use crate::uw::{GreekExposureExpiry, GreekExposureStrike, GreekExposureStrikeExpiry};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize)]
pub struct GexExpiryColumn {
    pub expiry: String,
    pub dte: Option<i64>,
    pub label: String,
    pub net_gex: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GexHeatRow {
    pub strike: f64,
    /// Cell values aligned to `expiries` order, then last cell = Σ all-expiry.
    pub values: Vec<f64>,
    /// −1..+1 color intensity per cell.
    pub intensities: Vec<f64>,
    pub sum_net: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GexHeatmap {
    pub proxy_ticker: String,
    pub spot: Option<f64>,
    pub move_high: Option<f64>,
    pub move_low: Option<f64>,
    pub expiries: Vec<GexExpiryColumn>,
    pub rows: Vec<GexHeatRow>,
    pub max_abs: f64,
    pub strike_min: f64,
    pub strike_max: f64,
    pub methodology: String,
}

pub fn build_gex_heatmap(
    proxy_ticker: &str,
    spot: Option<f64>,
    scale: f64,
    expiry_rows: &[GreekExposureExpiry],
    strike_sum: &[GreekExposureStrike],
    by_expiry: &[(String, Option<i64>, Vec<GreekExposureStrikeExpiry>)],
    implied_move_pct: Option<f64>,
) -> GexHeatmap {
    // Near-term columns (skip empty nets), cap at 10.
    let mut cols: Vec<GexExpiryColumn> = expiry_rows
        .iter()
        .filter(|e| e.expiry.is_some())
        .map(|e| {
            let expiry = e.expiry_date();
            let dte = e.dte;
            let label = match dte {
                Some(0) => format!("{} (0d)", short_exp(&expiry)),
                Some(d) => format!("{} ({}d)", short_exp(&expiry), d),
                None => short_exp(&expiry),
            };
            GexExpiryColumn {
                expiry,
                dte,
                label,
                net_gex: e.net_gex(),
            }
        })
        .collect();
    cols.sort_by(|a, b| a.dte.unwrap_or(999).cmp(&b.dte.unwrap_or(999)));
    cols.truncate(10);

    // Prefer the expiries we successfully fetched strike grids for.
    let fetched: BTreeSet<String> = by_expiry.iter().map(|(e, _, _)| e.clone()).collect();
    if !fetched.is_empty() {
        cols.retain(|c| fetched.contains(&c.expiry));
        // Keep order by dte; append any fetched missing from expiry list.
        for (exp, dte, _) in by_expiry {
            if !cols.iter().any(|c| &c.expiry == exp) {
                cols.push(GexExpiryColumn {
                    expiry: exp.clone(),
                    dte: *dte,
                    label: match dte {
                        Some(0) => format!("{} (0d)", short_exp(exp)),
                        Some(d) => format!("{} ({}d)", short_exp(exp), d),
                        None => short_exp(exp),
                    },
                    net_gex: 0.0,
                });
            }
        }
        cols.sort_by(|a, b| a.dte.unwrap_or(999).cmp(&b.dte.unwrap_or(999)));
        cols.truncate(10);
    }

    let spot_px = spot.unwrap_or_else(|| {
        strike_sum
            .iter()
            .filter_map(|s| s.strike_f64())
            .nth(strike_sum.len() / 2)
            .unwrap_or(100.0)
    });

    // Window around spot (futures-comparable after scale).
    let window = (spot_px * 0.04).clamp(40.0, 400.0);
    let lo = spot_px - window;
    let hi = spot_px + window;

    // Strike universe from Σ + per-expiry grids.
    let mut strikes: BTreeSet<i64> = BTreeSet::new();
    for s in strike_sum {
        if let Some(k) = s.strike_f64() {
            if (lo..=hi).contains(&k) {
                strikes.insert(price_key(k));
            }
        }
    }
    for (_, _, rows) in by_expiry {
        for r in rows {
            if let Some(k) = r.strike_f64() {
                if (lo..=hi).contains(&k) {
                    strikes.insert(price_key(k));
                }
            }
        }
    }

    // Maps: expiry -> strike_key -> net
    let mut grids: BTreeMap<String, BTreeMap<i64, f64>> = BTreeMap::new();
    for (exp, _, rows) in by_expiry {
        let mut m = BTreeMap::new();
        for r in rows {
            if let Some(k) = r.strike_f64() {
                m.insert(price_key(k), r.net_gex());
            }
        }
        // Refresh column net from cells if expiry list had 0.
        if let Some(col) = cols.iter_mut().find(|c| &c.expiry == exp) {
            if col.net_gex == 0.0 {
                col.net_gex = m.values().sum();
            }
        }
        grids.insert(exp.clone(), m);
    }

    let mut sum_map: BTreeMap<i64, f64> = BTreeMap::new();
    for s in strike_sum {
        if let Some(k) = s.strike_f64() {
            sum_map.insert(price_key(k), s.net_gex());
        }
    }

    // Max abs for color scale (near-spot cells only).
    let mut max_abs = 1e-9_f64;
    for &key in &strikes {
        for c in &cols {
            if let Some(v) = grids.get(&c.expiry).and_then(|m| m.get(&key)) {
                max_abs = max_abs.max(v.abs());
            }
        }
        if let Some(v) = sum_map.get(&key) {
            max_abs = max_abs.max(v.abs());
        }
    }

    // Cap rows for UI density (~40), prefer strikes closest to spot.
    let mut strike_list: Vec<i64> = strikes.into_iter().collect();
    if strike_list.len() > 40 {
        let spot_key = price_key(spot_px);
        strike_list.sort_by_key(|k| (k - spot_key).abs());
        strike_list.truncate(40);
        strike_list.sort();
    }

    let rows: Vec<GexHeatRow> = strike_list
        .iter()
        .rev() // high strikes on top (UW style)
        .map(|&key| {
            let strike = key as f64;
            let mut values = Vec::with_capacity(cols.len() + 1);
            for c in &cols {
                let v = grids
                    .get(&c.expiry)
                    .and_then(|m| m.get(&key))
                    .copied()
                    .unwrap_or(0.0);
                values.push(v);
            }
            let sum_net = sum_map.get(&key).copied().unwrap_or_else(|| values.iter().sum());
            values.push(sum_net);
            let intensities: Vec<f64> = values
                .iter()
                .map(|v| (v / max_abs).clamp(-1.0, 1.0))
                .collect();
            GexHeatRow {
                strike: strike * scale,
                values,
                intensities,
                sum_net,
            }
        })
        .collect();

    let move_pct = implied_move_pct.unwrap_or(1.5);
    let (move_high, move_low) = (
        Some(spot_px * scale * (1.0 + move_pct / 100.0)),
        Some(spot_px * scale * (1.0 - move_pct / 100.0)),
    );

    GexHeatmap {
        proxy_ticker: proxy_ticker.to_ascii_uppercase(),
        spot: Some(spot_px * scale),
        move_high,
        move_low,
        expiries: cols,
        rows,
        max_abs,
        strike_min: lo * scale,
        strike_max: hi * scale,
        methodology: "Strike × expiry net GEX from Unusual Whales greek-exposure \
(call_gex + put_gex). Σ column = all-expiry by strike. Window ≈ ±4% around spot. \
Same heatmap paradigm as the UW GEX terminal, projected onto the futures product."
            .into(),
    }
}

fn short_exp(expiry: &str) -> String {
    // YYYY-MM-DD → Mon DD
    if expiry.len() >= 10 {
        let m: u32 = expiry[5..7].parse().unwrap_or(1);
        let d: u32 = expiry[8..10].parse().unwrap_or(1);
        let months = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        format!("{} {}", months[(m.saturating_sub(1) as usize).min(11)], d)
    } else {
        expiry.to_string()
    }
}

fn price_key(p: f64) -> i64 {
    p.round() as i64
}
