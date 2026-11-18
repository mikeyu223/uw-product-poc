//! Enhanced 0DTE Sweep Cluster Filter.
//!
//! Filters UW market-wide flow alerts down to same-day (0DTE) prints, then
//! clusters sweeps by ticker + time bucket so traders see coordinated
//! aggression — not a flat alert firehose.

use crate::uw::FlowAlert;
use chrono::{NaiveDate, TimeZone, Utc};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct SweepAlert {
    pub id: String,
    pub ticker: String,
    pub option_type: String,
    pub strike: String,
    pub expiry: String,
    pub premium: f64,
    pub size: i64,
    pub side_bias: String,
    pub ask_prem: f64,
    pub bid_prem: f64,
    pub has_sweep: bool,
    pub alert_rule: String,
    pub created_at: String,
    pub iv: Option<f64>,
    pub dte: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SweepCluster {
    pub key: String,
    pub ticker: String,
    pub bucket_start: String,
    pub alert_count: usize,
    pub sweep_count: usize,
    pub total_premium: f64,
    pub net_ask_prem: f64,
    pub net_bid_prem: f64,
    pub dominant_side: String,
    pub call_premium: f64,
    pub put_premium: f64,
    pub cluster_score: u8,
    pub label: String,
    pub alerts: Vec<SweepAlert>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SweepClusterReport {
    pub as_of: String,
    pub session_date: String,
    pub total_alerts_scanned: usize,
    pub zero_dte_alerts: usize,
    pub sweep_alerts: usize,
    pub clusters: Vec<SweepCluster>,
    pub feed: Vec<SweepAlert>,
    pub narrative: String,
    pub methodology: String,
}

/// `session_date` = YYYY-MM-DD for 0DTE (defaults to UTC today).
pub fn build_sweep_clusters(alerts: &[FlowAlert], session_date: Option<&str>) -> SweepClusterReport {
    let today = session_date
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| Utc::now().date_naive());
    let session = today.format("%Y-%m-%d").to_string();

    let mut zero_dte = Vec::new();
    let mut sweep_count = 0usize;

    for (idx, a) in alerts.iter().enumerate() {
        let Some(expiry) = a.expiry.as_ref() else {
            continue;
        };
        let exp = &expiry[..10.min(expiry.len())];
        let Ok(exp_date) = NaiveDate::parse_from_str(exp, "%Y-%m-%d") else {
            continue;
        };
        let dte = (exp_date - today).num_days();
        // 0DTE = expiry today; also keep 1DTE for after-hours / weekend edge.
        if dte < 0 || dte > 1 {
            continue;
        }
        let prem = parse_f64(a.total_premium.as_deref()).unwrap_or(0.0);
        let ask = parse_f64(a.total_ask_side_prem.as_deref()).unwrap_or(0.0);
        let bid = parse_f64(a.total_bid_side_prem.as_deref()).unwrap_or(0.0);
        let has_sweep = a.has_sweep.unwrap_or(false)
            || a.alert_rule
                .as_deref()
                .map(|r| r.to_ascii_lowercase().contains("sweep"))
                .unwrap_or(false);
        if has_sweep {
            sweep_count += 1;
        }
        let side_bias = if ask > bid * 1.15 {
            "ask"
        } else if bid > ask * 1.15 {
            "bid"
        } else {
            "mixed"
        }
        .to_string();

        zero_dte.push(SweepAlert {
            id: a
                .option_chain
                .clone()
                .unwrap_or_else(|| format!("{idx}-{}", a.created_at.as_deref().unwrap_or("?"))),
            ticker: a.ticker.clone().unwrap_or_else(|| "?".into()),
            option_type: a.option_type.clone().unwrap_or_else(|| "?".into()),
            strike: a.strike.clone().unwrap_or_else(|| "?".into()),
            expiry: exp.to_string(),
            premium: prem,
            size: a.total_size.unwrap_or(0),
            side_bias,
            ask_prem: ask,
            bid_prem: bid,
            has_sweep,
            alert_rule: a.alert_rule.clone().unwrap_or_default(),
            created_at: a.created_at.clone().unwrap_or_default(),
            iv: a.iv,
            dte,
        });
    }

    // Prefer sweeps for clustering; fall back to all 0DTE if thin.
    let cluster_src: Vec<&SweepAlert> = {
        let sweeps: Vec<_> = zero_dte.iter().filter(|a| a.has_sweep).collect();
        if sweeps.len() >= 3 {
            sweeps
        } else {
            zero_dte.iter().collect()
        }
    };

    let mut buckets: HashMap<(String, String), Vec<SweepAlert>> = HashMap::new();
    for a in cluster_src {
        let bucket = minute_bucket(&a.created_at, 3);
        buckets
            .entry((a.ticker.clone(), bucket))
            .or_default()
            .push(a.clone());
    }

    let mut clusters: Vec<SweepCluster> = buckets
        .into_iter()
        .map(|((ticker, bucket), mut alerts)| {
            alerts.sort_by(|a, b| {
                b.premium
                    .partial_cmp(&a.premium)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let alert_count = alerts.len();
            let sweep_count = alerts.iter().filter(|a| a.has_sweep).count();
            let total_premium: f64 = alerts.iter().map(|a| a.premium).sum();
            let net_ask: f64 = alerts.iter().map(|a| a.ask_prem).sum();
            let net_bid: f64 = alerts.iter().map(|a| a.bid_prem).sum();
            let call_premium: f64 = alerts
                .iter()
                .filter(|a| a.option_type.eq_ignore_ascii_case("call"))
                .map(|a| a.premium)
                .sum();
            let put_premium: f64 = alerts
                .iter()
                .filter(|a| a.option_type.eq_ignore_ascii_case("put"))
                .map(|a| a.premium)
                .sum();
            let dominant_side = if net_ask >= net_bid {
                "ask"
            } else {
                "bid"
            }
            .to_string();
            let cluster_score =
                cluster_score(alert_count, sweep_count, total_premium, &dominant_side);
            let label = if cluster_score >= 75 {
                "Hot cluster"
            } else if cluster_score >= 50 {
                "Active"
            } else {
                "Watch"
            }
            .to_string();

            SweepCluster {
                key: format!("{ticker}|{bucket}"),
                ticker,
                bucket_start: bucket,
                alert_count,
                sweep_count,
                total_premium,
                net_ask_prem: net_ask,
                net_bid_prem: net_bid,
                dominant_side,
                call_premium,
                put_premium,
                cluster_score,
                label,
                alerts,
            }
        })
        .collect();

    clusters.sort_by(|a, b| {
        b.cluster_score
            .cmp(&a.cluster_score)
            .then_with(|| {
                b.total_premium
                    .partial_cmp(&a.total_premium)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    // Keep multi-hit clusters first; singles still listed lower.
    clusters.sort_by(|a, b| {
        let am = a.alert_count >= 2;
        let bm = b.alert_count >= 2;
        bm.cmp(&am).then(b.cluster_score.cmp(&a.cluster_score))
    });

    let mut feed = zero_dte.clone();
    feed.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let top = clusters.first();
    let narrative = match top {
        Some(c) if c.alert_count >= 2 => format!(
            "Hottest 0DTE cluster: {} · {} alerts / {} sweeps · ${:.0} premium · {} aggression in the {} window.",
            c.ticker,
            c.alert_count,
            c.sweep_count,
            c.total_premium,
            c.dominant_side,
            c.bucket_start
        ),
        Some(c) => format!(
            "Top 0DTE print: {} · ${:.0} premium ({}). Waiting for multi-alert clusters to form.",
            c.ticker, c.total_premium, c.label
        ),
        None => "No 0DTE flow in the current alert window — market may be quiet or after hours."
            .into(),
    };

    SweepClusterReport {
        as_of: Utc::now().to_rfc3339(),
        session_date: session,
        total_alerts_scanned: alerts.len(),
        zero_dte_alerts: zero_dte.len(),
        sweep_alerts: sweep_count,
        clusters,
        feed,
        narrative,
        methodology: "Market-wide UW flow alerts filtered to expiry = session date (0DTE, +1DTE \
buffer). Sweeps clustered by ticker × 3-minute bucket. Score = density × premium × ask/bid \
aggression. Same Live Options Flow data — filtered for the exact pain point: 0DTE sweep storms."
            .into(),
    }
}

fn minute_bucket(ts: &str, minutes: i64) -> String {
    // Parse ISO; floor to N-minute bucket.
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
        let ts = dt.with_timezone(&Utc);
        let mins = ts.timestamp() / 60;
        let floored = (mins / minutes) * minutes;
        let bucket = Utc
            .timestamp_opt(floored * 60, 0)
            .single()
            .unwrap_or(ts);
        return bucket.format("%Y-%m-%dT%H:%M").to_string();
    }
    ts.get(..16).unwrap_or(ts).to_string()
}

fn cluster_score(count: usize, sweeps: usize, premium: f64, side: &str) -> u8 {
    let mut pts = 0.0;
    pts += (count as f64 * 12.0).min(36.0);
    pts += (sweeps as f64 * 14.0).min(28.0);
    if premium >= 1_000_000.0 {
        pts += 25.0;
    } else if premium >= 250_000.0 {
        pts += 18.0;
    } else if premium >= 75_000.0 {
        pts += 10.0;
    } else if premium >= 20_000.0 {
        pts += 5.0;
    }
    if side == "ask" || side == "bid" {
        pts += 8.0;
    }
    pts.round().clamp(0.0, 100.0) as u8
}

fn parse_f64(s: Option<&str>) -> Option<f64> {
    s?.parse().ok()
}
