//! Auto-tag fills with Unusual Whales flow + dealer GEX at entry.
//! No invented scores — just connects execution timestamps to UW's data moat.

use super::store::{FlowContext, SweepMarker, TradeEntry};
use crate::uw::{FlowAlert, GreekExposure, UwClient};
use chrono::{DateTime, Utc};

pub async fn enrich_trade(uw: &UwClient, trade: &mut TradeEntry) {
    let ticker = underlying_ticker(&trade.symbol);
    if ticker.is_empty() {
        return;
    }

    let (alerts_r, greek_r, iv_r) = tokio::join!(
        uw.flow_alerts(&ticker, 100),
        uw.latest_greek_exposure(&ticker),
        uw.latest_iv_rank(&ticker),
    );

    if let Ok(Some(iv)) = iv_r {
        trade.iv_rank_at_entry = iv.iv_rank_pct();
    }

    let greek = greek_r.ok().flatten();
    let alerts = alerts_r.unwrap_or_default();
    trade.flow = Some(build_flow_context(trade, &alerts, greek.as_ref()));

    if trade.dte_at_entry.is_none() {
        trade.dte_at_entry = infer_dte(&trade.setup, &alerts, &trade.entry_at);
    }
    if trade.contract_strategy.is_empty() || trade.contract_strategy == "unknown" {
        trade.contract_strategy = infer_strategy(trade, &alerts);
    }
}

pub async fn enrich_many(uw: &UwClient, trades: &mut [TradeEntry]) {
    for trade in trades.iter_mut() {
        enrich_trade(uw, trade).await;
    }
}

fn build_flow_context(
    trade: &TradeEntry,
    alerts: &[FlowAlert],
    greek: Option<&GreekExposure>,
) -> FlowContext {
    let entry = parse_ts(&trade.entry_at).unwrap_or_else(Utc::now);

    let mut window: Vec<&FlowAlert> = alerts
        .iter()
        .filter(|a| {
            a.created_at
                .as_ref()
                .and_then(|t| parse_ts(t))
                .map(|t| {
                    let mins = (entry - t).num_minutes();
                    (-5..=60).contains(&mins)
                })
                .unwrap_or(false)
        })
        .collect();
    window.sort_by_key(|a| {
        a.created_at
            .as_ref()
            .and_then(|t| parse_ts(t))
            .map(|t| (entry - t).num_seconds().abs())
            .unwrap_or(i64::MAX)
    });

    let (net_call, net_put) = window_net_premium(&window);
    let mut major_sweeps = Vec::new();

    for a in &window {
        let prem = prem_f64(a);
        let is_sweep = a.has_sweep.unwrap_or(false)
            || a.alert_rule
                .as_deref()
                .map(|r| r.to_ascii_lowercase().contains("sweep"))
                .unwrap_or(false);
        if !is_sweep || prem < 50_000.0 {
            continue;
        }
        let is_call = is_call_alert(a);
        let mins = a
            .created_at
            .as_ref()
            .and_then(|t| parse_ts(t))
            .map(|t| (entry - t).num_minutes())
            .unwrap_or(0);
        let side = if is_call { "call" } else { "put" };
        major_sweeps.push(SweepMarker {
            t: a.created_at.clone().unwrap_or_default(),
            premium: prem,
            side: side.into(),
            mins_before_entry: mins,
            label: format!(
                "${:.1}M {} Sweep",
                prem / 1_000_000.0,
                if is_call { "Call" } else { "Put" }
            ),
        });
    }

    let bullish_flow = net_call - net_put;
    let trade_bullish = trade.side == "long" || trade.side == "buy";
    let (alignment, badge) = if window.is_empty() {
        (
            "unknown".into(),
            "No recent UW flow alerts near entry — context unavailable.".into(),
        )
    } else if let Some(top) = major_sweeps.first() {
        let aligned = (trade_bullish && top.side == "call") || (!trade_bullish && top.side == "put");
        if aligned {
            (
                "with".into(),
                format!(
                    "Aligned with {} {} prior",
                    top.label,
                    format_mins(top.mins_before_entry)
                ),
            )
        } else {
            (
                "against".into(),
                format!(
                    "Traded against {} {} prior",
                    top.label,
                    format_mins(top.mins_before_entry)
                ),
            )
        }
    } else if bullish_flow.abs() < 100_000.0 {
        (
            "neutral".into(),
            format!(
                "Mixed flow near entry (calls ${:.1}M / puts ${:.1}M)",
                net_call.abs() / 1_000_000.0,
                net_put.abs() / 1_000_000.0
            ),
        )
    } else if (trade_bullish && bullish_flow > 0.0) || (!trade_bullish && bullish_flow < 0.0) {
        (
            "with".into(),
            if bullish_flow > 0.0 {
                format!(
                    "Aligned with Net Call Flow (+${:.1}M)",
                    bullish_flow / 1_000_000.0
                )
            } else {
                format!(
                    "Aligned with Net Put Flow (−${:.1}M)",
                    bullish_flow.abs() / 1_000_000.0
                )
            },
        )
    } else {
        (
            "against".into(),
            if bullish_flow > 0.0 {
                format!(
                    "Traded against Net Call Flow (+${:.1}M)",
                    bullish_flow / 1_000_000.0
                )
            } else {
                format!(
                    "Traded against Net Put Flow (−${:.1}M)",
                    bullish_flow.abs() / 1_000_000.0
                )
            },
        )
    };

    let (gex_regime, gex_label, net_gamma) = match greek.and_then(|g| g.net_gamma()) {
        Some(g) if g > 0.0 => (
            "positive_gamma".into(),
            "Entered in Positive Gamma (mean-reverting)".into(),
            Some(g),
        ),
        Some(g) => (
            "negative_gamma".into(),
            "Entered in Negative Gamma (volatile)".into(),
            Some(g),
        ),
        None => (
            "unknown".into(),
            "Dealer GEX at entry unavailable".into(),
            None,
        ),
    };

    FlowContext {
        alignment,
        badge,
        net_call_prem: net_call,
        net_put_prem: net_put,
        nearest_sweep_prem: major_sweeps.first().map(|s| s.premium),
        nearest_sweep_mins: major_sweeps.first().map(|s| s.mins_before_entry),
        nearest_sweep_side: major_sweeps.first().map(|s| s.side.clone()),
        gex_regime,
        gex_label,
        net_gamma,
        major_sweeps,
    }
}

fn window_net_premium(window: &[&FlowAlert]) -> (f64, f64) {
    let mut call = 0.0;
    let mut put = 0.0;
    for a in window {
        let ask = ask_prem(a);
        let bid = bid_prem(a);
        let prem = prem_f64(a);
        let magnitude = prem.max((ask - bid).abs());
        if magnitude <= 0.0 {
            continue;
        }
        let bullish = ask >= bid;
        if is_call_alert(a) {
            call += if bullish { magnitude } else { -magnitude };
        } else {
            put += if bullish { magnitude } else { -magnitude };
        }
    }
    (call, put)
}

fn infer_dte(setup: &str, alerts: &[FlowAlert], entry_at: &str) -> Option<i32> {
    let s = setup.to_ascii_lowercase();
    if s.contains("0dte") {
        return Some(0);
    }
    let entry = parse_ts(entry_at)?;
    for a in alerts.iter().take(20) {
        if let Some(exp) = a.expiry.as_ref() {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(exp, "%Y-%m-%d") {
                let days = (d - entry.date_naive()).num_days();
                if (0..365).contains(&days) {
                    return Some(days as i32);
                }
            }
        }
    }
    if s.contains("earnings") {
        Some(5)
    } else {
        Some(1)
    }
}

fn infer_strategy(trade: &TradeEntry, alerts: &[FlowAlert]) -> String {
    if trade.asset_class == "futures" {
        return "futures".into();
    }
    let s = trade.setup.to_ascii_lowercase();
    if s.contains("iron") {
        return "iron_condor".into();
    }
    if s.contains("vertical") || s.contains("spread") {
        return "vertical".into();
    }
    if alerts.iter().any(|a| a.has_multileg == Some(true)) {
        "vertical".into()
    } else {
        "single_leg".into()
    }
}

pub fn underlying_ticker(symbol: &str) -> String {
    let s = symbol.to_ascii_uppercase();
    if (s.starts_with("ES") || s.starts_with("NQ") || s.starts_with("RTY") || s.starts_with("MES") || s.starts_with("MNQ"))
        && s.len() <= 5
    {
        return String::new();
    }
    if s.len() > 6 && s.chars().any(|c| c.is_ascii_digit()) {
        let alpha: String = s.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
        if (1..=5).contains(&alpha.len()) {
            return alpha;
        }
    }
    s
}

pub fn demo_flow_context(trade: &TradeEntry, i: usize) -> FlowContext {
    let demos = [
        (
            "with",
            "Aligned with $4.2M Call Sweep 4 mins prior",
            4_200_000.0,
            800_000.0,
            "positive_gamma",
            "Entered in Positive Gamma (mean-reverting)",
            "call",
            4_i64,
        ),
        (
            "against",
            "Traded against Net Put Flow (−$18.0M)",
            2_100_000.0,
            18_000_000.0,
            "negative_gamma",
            "Entered in Negative Gamma (volatile)",
            "put",
            12,
        ),
        (
            "with",
            "Aligned with $6.8M Put Sweep 9 mins prior",
            1_200_000.0,
            6_800_000.0,
            "positive_gamma",
            "Entered in Positive Gamma (mean-reverting)",
            "put",
            9,
        ),
        (
            "with",
            "Aligned with Net Call Flow (+$2.4M)",
            2_400_000.0,
            400_000.0,
            "negative_gamma",
            "Entered in Negative Gamma (volatile)",
            "call",
            3,
        ),
        (
            "against",
            "Traded against $3.1M Call Sweep 7 mins prior",
            3_100_000.0,
            900_000.0,
            "positive_gamma",
            "Entered in Positive Gamma (mean-reverting)",
            "call",
            7,
        ),
        (
            "with",
            "Aligned with $5.5M Call Sweep 2 mins prior",
            5_500_000.0,
            1_100_000.0,
            "negative_gamma",
            "Entered in Negative Gamma (volatile)",
            "call",
            2,
        ),
        (
            "neutral",
            "Mixed flow near entry (calls $1.2M / puts $1.4M)",
            1_200_000.0,
            1_400_000.0,
            "positive_gamma",
            "Entered in Positive Gamma (mean-reverting)",
            "call",
            15,
        ),
        (
            "with",
            "Aligned with $2.9M Call Sweep 6 mins prior",
            2_900_000.0,
            600_000.0,
            "negative_gamma",
            "Entered in Negative Gamma (volatile)",
            "call",
            6,
        ),
    ];
    let d = demos[i % demos.len()];
    let entry = parse_ts(&trade.entry_at).unwrap_or_else(Utc::now);
    let sweep_t = entry - chrono::Duration::minutes(d.7);
    let sweep_prem = if d.6 == "call" { d.2 } else { d.3 };
    FlowContext {
        alignment: d.0.into(),
        badge: d.1.into(),
        net_call_prem: d.2,
        net_put_prem: d.3,
        nearest_sweep_prem: Some(sweep_prem),
        nearest_sweep_mins: Some(d.7),
        nearest_sweep_side: Some(d.6.into()),
        gex_regime: d.4.into(),
        gex_label: d.5.into(),
        net_gamma: Some(if d.4 == "positive_gamma" {
            1.2e9
        } else {
            -8.4e8
        }),
        major_sweeps: vec![SweepMarker {
            t: sweep_t.to_rfc3339(),
            premium: sweep_prem,
            side: d.6.into(),
            mins_before_entry: d.7,
            label: format!(
                "${:.1}M {} Sweep",
                sweep_prem / 1_000_000.0,
                if d.6 == "call" { "Call" } else { "Put" }
            ),
        }],
    }
}

fn is_call_alert(a: &FlowAlert) -> bool {
    a.option_type
        .as_deref()
        .map(|t| t.to_ascii_lowercase().starts_with('c'))
        .unwrap_or(false)
}

fn prem_f64(a: &FlowAlert) -> f64 {
    a.total_premium
        .as_ref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

fn ask_prem(a: &FlowAlert) -> f64 {
    a.total_ask_side_prem
        .as_ref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

fn bid_prem(a: &FlowAlert) -> f64 {
    a.total_bid_side_prem
        .as_ref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|n| n.and_utc())
        })
}

fn format_mins(m: i64) -> String {
    if m <= 0 {
        "at entry".into()
    } else if m == 1 {
        "1 min".into()
    } else {
        format!("{m} mins")
    }
}
