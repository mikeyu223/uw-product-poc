use super::store::TradeEntry;
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct JournalFilter {
    pub account: Option<String>,
    pub setup: Option<String>,
    pub tod: Option<String>,
    pub dow: Option<String>,
    pub strategy: Option<String>,
    pub dte_bucket: Option<String>, // "0dte" | "1_5" | "swing"
}

#[derive(Debug, Clone, Serialize)]
pub struct JournalAnalytics {
    pub summary: PerformanceSummary,
    pub accounts: Vec<AccountRow>,
    pub by_setup: Vec<SetupRow>,
    pub by_strategy: Vec<SetupRow>,
    pub time_of_day: Vec<HeatCell>,
    pub day_of_week: Vec<HeatCell>,
    pub recent: Vec<TradeEntry>,
    pub active_filters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerformanceSummary {
    pub trade_count: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub profit_factor: f64,
    pub expectancy: f64,
    pub payoff_ratio: f64,
    pub sharpe_approx: f64,
    pub sortino_approx: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountRow {
    pub account: String,
    pub trades: usize,
    pub pnl: f64,
    pub win_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupRow {
    pub setup: String,
    pub trades: usize,
    pub wins: usize,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub avg_r: f64,
    pub expectancy: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HeatCell {
    pub label: String,
    pub trades: usize,
    pub pnl: f64,
    pub win_rate: f64,
}

pub fn filter_trades<'a>(trades: &'a [TradeEntry], f: &JournalFilter) -> Vec<&'a TradeEntry> {
    trades
        .iter()
        .filter(|t| {
            if let Some(a) = f.account.as_ref() {
                if &t.account != a {
                    return false;
                }
            }
            if let Some(s) = f.setup.as_ref() {
                if &t.setup != s {
                    return false;
                }
            }
            if let Some(st) = f.strategy.as_ref() {
                if &t.contract_strategy != st {
                    return false;
                }
            }
            if let Some(tod) = f.tod.as_ref() {
                if tod_label(&t.entry_at) != tod.as_str() {
                    return false;
                }
            }
            if let Some(dow) = f.dow.as_ref() {
                if dow_label(&t.entry_at) != dow.as_str() {
                    return false;
                }
            }
            if let Some(bucket) = f.dte_bucket.as_ref() {
                let ok = match (bucket.as_str(), t.dte_at_entry) {
                    ("0dte", Some(0)) => true,
                    ("1_5", Some(d)) if (1..=5).contains(&d) => true,
                    ("swing", Some(d)) if d > 5 => true,
                    ("swing", None) if t.asset_class == "futures" => true,
                    _ => false,
                };
                if !ok {
                    return false;
                }
            }
            true
        })
        .collect()
}

pub fn build_analytics(trades: &[TradeEntry]) -> JournalAnalytics {
    build_analytics_filtered(trades, &JournalFilter::default())
}

pub fn build_analytics_filtered(all: &[TradeEntry], f: &JournalFilter) -> JournalAnalytics {
    let filtered = filter_trades(all, f);
    let owned: Vec<TradeEntry> = filtered.into_iter().cloned().collect();
    // Facets always from full book so filters remain clickable.
    let accounts = group_accounts(all);
    let by_setup = group_setups(all);
    let by_strategy = group_strategies(all);
    let time_of_day = group_tod(all);
    let day_of_week = group_dow(all);

    let summary = summarize(&owned);
    let mut recent = owned;
    recent.sort_by(|a, b| b.exit_at.cmp(&a.exit_at));
    recent.truncate(50);

    JournalAnalytics {
        summary,
        accounts,
        by_setup,
        by_strategy,
        time_of_day,
        day_of_week,
        recent,
        active_filters: serde_json::json!({
            "account": f.account,
            "setup": f.setup,
            "tod": f.tod,
            "dow": f.dow,
            "strategy": f.strategy,
            "dte_bucket": f.dte_bucket,
        }),
    }
}

fn tod_label(iso: &str) -> &'static str {
    const LABELS: [&str; 6] = [
        "09:30–10:30",
        "10:30–11:30",
        "11:30–13:00",
        "13:00–14:30",
        "14:30–16:00",
        "Other",
    ];
    LABELS[tod_bucket(iso)]
}

fn dow_label(iso: &str) -> &'static str {
    const LABELS: [&str; 6] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat/Sun"];
    LABELS[dow_bucket(iso)]
}

fn summarize(trades: &[TradeEntry]) -> PerformanceSummary {
    let n = trades.len();
    if n == 0 {
        return PerformanceSummary {
            trade_count: 0,
            wins: 0,
            losses: 0,
            win_rate: 0.0,
            total_pnl: 0.0,
            avg_win: 0.0,
            avg_loss: 0.0,
            profit_factor: 0.0,
            expectancy: 0.0,
            payoff_ratio: 0.0,
            sharpe_approx: 0.0,
            sortino_approx: 0.0,
        };
    }

    let wins: Vec<f64> = trades.iter().map(|t| t.pnl).filter(|p| *p > 0.0).collect();
    let losses: Vec<f64> = trades.iter().map(|t| t.pnl).filter(|p| *p < 0.0).collect();
    let total_pnl: f64 = trades.iter().map(|t| t.pnl).sum();
    let win_rate = wins.len() as f64 / n as f64;
    let avg_win = if wins.is_empty() {
        0.0
    } else {
        wins.iter().sum::<f64>() / wins.len() as f64
    };
    let avg_loss = if losses.is_empty() {
        0.0
    } else {
        losses.iter().sum::<f64>() / losses.len() as f64
    };
    let gross_win: f64 = wins.iter().sum();
    let gross_loss: f64 = losses.iter().map(|l| l.abs()).sum();
    let profit_factor = if gross_loss > 0.0 {
        gross_win / gross_loss
    } else if gross_win > 0.0 {
        99.0
    } else {
        0.0
    };
    let expectancy = total_pnl / n as f64;
    let payoff_ratio = if avg_loss.abs() > 0.0 {
        avg_win / avg_loss.abs()
    } else {
        0.0
    };

    let mean = expectancy;
    let variance = trades
        .iter()
        .map(|t| {
            let d = t.pnl - mean;
            d * d
        })
        .sum::<f64>()
        / n as f64;
    let std = variance.sqrt();
    let sharpe_approx = if std > 0.0 { mean / std } else { 0.0 };

    let downside_var = trades
        .iter()
        .map(|t| {
            let d = (t.pnl - mean).min(0.0);
            d * d
        })
        .sum::<f64>()
        / n as f64;
    let downside = downside_var.sqrt();
    let sortino_approx = if downside > 0.0 {
        mean / downside
    } else {
        0.0
    };

    PerformanceSummary {
        trade_count: n,
        wins: wins.len(),
        losses: losses.len(),
        win_rate: round4(win_rate),
        total_pnl: round2(total_pnl),
        avg_win: round2(avg_win),
        avg_loss: round2(avg_loss),
        profit_factor: round2(profit_factor),
        expectancy: round2(expectancy),
        payoff_ratio: round2(payoff_ratio),
        sharpe_approx: round2(sharpe_approx),
        sortino_approx: round2(sortino_approx),
    }
}

fn group_accounts(trades: &[TradeEntry]) -> Vec<AccountRow> {
    let mut map: BTreeMap<String, Vec<&TradeEntry>> = BTreeMap::new();
    for t in trades {
        map.entry(t.account.clone()).or_default().push(t);
    }
    map.into_iter()
        .map(|(account, rows)| {
            let wins = rows.iter().filter(|t| t.pnl > 0.0).count();
            let pnl: f64 = rows.iter().map(|t| t.pnl).sum();
            AccountRow {
                account,
                trades: rows.len(),
                pnl: round2(pnl),
                win_rate: round4(wins as f64 / rows.len().max(1) as f64),
            }
        })
        .collect()
}

fn group_setups(trades: &[TradeEntry]) -> Vec<SetupRow> {
    group_by_key(trades, |t| t.setup.clone())
}

fn group_strategies(trades: &[TradeEntry]) -> Vec<SetupRow> {
    group_by_key(trades, |t| t.contract_strategy.clone())
}

fn group_by_key<F>(trades: &[TradeEntry], key_fn: F) -> Vec<SetupRow>
where
    F: Fn(&TradeEntry) -> String,
{
    let mut map: BTreeMap<String, Vec<&TradeEntry>> = BTreeMap::new();
    for t in trades {
        map.entry(key_fn(t)).or_default().push(t);
    }
    let mut rows: Vec<SetupRow> = map
        .into_iter()
        .map(|(setup, rows)| {
            let wins = rows.iter().filter(|t| t.pnl > 0.0).count();
            let total_pnl: f64 = rows.iter().map(|t| t.pnl).sum();
            let expectancy = total_pnl / rows.len() as f64;
            let avg_loss = rows
                .iter()
                .filter(|t| t.pnl < 0.0)
                .map(|t| t.pnl.abs())
                .sum::<f64>()
                / rows.iter().filter(|t| t.pnl < 0.0).count().max(1) as f64;
            let risk_unit = if avg_loss > 0.0 { avg_loss } else { 100.0 };
            let avg_r = total_pnl / rows.len() as f64 / risk_unit;
            SetupRow {
                setup,
                trades: rows.len(),
                wins,
                win_rate: round4(wins as f64 / rows.len() as f64),
                total_pnl: round2(total_pnl),
                avg_r: round2(avg_r),
                expectancy: round2(expectancy),
            }
        })
        .collect();
    rows.sort_by(|a, b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap());
    rows
}

fn group_tod(trades: &[TradeEntry]) -> Vec<HeatCell> {
    // Buckets in US/Eastern wall-clock hours (approximate from UTC timestamps)
    let labels = [
        "09:30–10:30",
        "10:30–11:30",
        "11:30–13:00",
        "13:00–14:30",
        "14:30–16:00",
        "Other",
    ];
    let mut buckets: Vec<(usize, f64, usize)> = vec![(0, 0.0, 0); labels.len()];
    for t in trades {
        let idx = tod_bucket(&t.entry_at);
        buckets[idx].0 += 1;
        buckets[idx].1 += t.pnl;
        if t.pnl > 0.0 {
            buckets[idx].2 += 1;
        }
    }
    labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let (n, pnl, wins) = buckets[i];
            HeatCell {
                label: (*label).into(),
                trades: n,
                pnl: round2(pnl),
                win_rate: if n > 0 {
                    round4(wins as f64 / n as f64)
                } else {
                    0.0
                },
            }
        })
        .collect()
}

fn tod_bucket(iso: &str) -> usize {
    let Some(dt) = DateTime::parse_from_rfc3339(iso)
        .ok()
        .map(|d| d.with_timezone(&Utc))
    else {
        return 5;
    };
    // Treat stored demo times as already ET-ish; convert UTC → ET roughly (UTC-5)
    let hour = dt.hour().saturating_sub(5);
    let minute = dt.minute();
    let mins = hour * 60 + minute;
    match mins {
        570..=629 => 0,  // 9:30–10:30
        630..=689 => 1,  // 10:30–11:30
        690..=779 => 2,  // 11:30–13:00
        780..=869 => 3,  // 13:00–14:30
        870..=960 => 4,  // 14:30–16:00
        _ => 5,
    }
}

fn group_dow(trades: &[TradeEntry]) -> Vec<HeatCell> {
    let labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat/Sun"];
    let mut buckets: Vec<(usize, f64, usize)> = vec![(0, 0.0, 0); labels.len()];
    for t in trades {
        let idx = dow_bucket(&t.entry_at);
        buckets[idx].0 += 1;
        buckets[idx].1 += t.pnl;
        if t.pnl > 0.0 {
            buckets[idx].2 += 1;
        }
    }
    labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let (n, pnl, wins) = buckets[i];
            HeatCell {
                label: (*label).into(),
                trades: n,
                pnl: round2(pnl),
                win_rate: if n > 0 {
                    round4(wins as f64 / n as f64)
                } else {
                    0.0
                },
            }
        })
        .collect()
}

fn dow_bucket(iso: &str) -> usize {
    let Some(dt) = DateTime::parse_from_rfc3339(iso)
        .ok()
        .map(|d| d.with_timezone(&Utc))
    else {
        return 5;
    };
    match dt.weekday().num_days_from_monday() {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        _ => 5,
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}
