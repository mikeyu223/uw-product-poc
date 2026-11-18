//! Trade review chart: price path around entry/exit + whale sweep markers.

use super::enrich::underlying_ticker;
use super::store::TradeEntry;
use crate::uw::UwClient;
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ChartCandle {
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChartMarker {
    pub time: i64,
    pub position: String,
    pub color: String,
    pub shape: String,
    pub text: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TradeChart {
    pub trade_id: String,
    pub symbol: String,
    pub candles: Vec<ChartCandle>,
    pub markers: Vec<ChartMarker>,
    pub entry_time: i64,
    pub exit_time: i64,
    pub source: String,
}

pub async fn build_trade_chart(uw: &UwClient, trade: &TradeEntry) -> TradeChart {
    let entry = parse_ts(&trade.entry_at).unwrap_or_else(Utc::now);
    let exit = parse_ts(&trade.exit_at).unwrap_or(entry + Duration::minutes(30));
    let entry_ts = entry.timestamp();
    let exit_ts = exit.timestamp();

    let ticker = underlying_ticker(&trade.symbol);
    let mut markers = Vec::new();

    markers.push(ChartMarker {
        time: entry_ts,
        position: "belowBar".into(),
        color: "#22c55e".into(),
        shape: "arrowUp".into(),
        text: "Entry".into(),
        kind: "entry".into(),
    });
    markers.push(ChartMarker {
        time: exit_ts,
        position: "aboveBar".into(),
        color: if trade.pnl >= 0.0 {
            "#22c55e".into()
        } else {
            "#ef4444".into()
        },
        shape: "arrowDown".into(),
        text: "Exit".into(),
        kind: "exit".into(),
    });

    if let Some(flow) = trade.flow.as_ref() {
        for s in &flow.major_sweeps {
            if let Some(t) = parse_ts(&s.t) {
                markers.push(ChartMarker {
                    time: t.timestamp(),
                    position: if s.side == "call" {
                        "belowBar".into()
                    } else {
                        "aboveBar".into()
                    },
                    color: if s.side == "call" {
                        "#38bdf8".into()
                    } else {
                        "#f472b6".into()
                    },
                    shape: "circle".into(),
                    text: s.label.clone(),
                    kind: "sweep".into(),
                });
            }
        }
    }

    // Try live underlying last as anchor; synthesize 1m path ±30m for review PoC.
    let spot = if !ticker.is_empty() {
        uw.stock_state(&ticker)
            .await
            .ok()
            .and_then(|s| {
                s.close
                    .as_ref()
                    .and_then(|x| x.parse().ok())
                    .or_else(|| s.prev_close.as_ref().and_then(|x| x.parse().ok()))
            })
    } else {
        None
    };

    let anchor = spot.unwrap_or(100.0);
    // Map option premium P&L onto a synthetic underlying path for visuals when
    // we don't have historical 1m bars — still places entry/exit/sweeps correctly in time.
    let candles = synthesize_path(entry, exit, anchor, trade.pnl >= 0.0);
    markers.sort_by_key(|m| m.time);

    TradeChart {
        trade_id: trade.id.clone(),
        symbol: if ticker.is_empty() {
            trade.symbol.clone()
        } else {
            ticker
        },
        candles,
        markers,
        entry_time: entry_ts,
        exit_time: exit_ts,
        source: "synthesized_around_fill+uw_sweep_markers".into(),
    }
}

fn synthesize_path(
    entry: DateTime<Utc>,
    exit: DateTime<Utc>,
    anchor: f64,
    winner: bool,
) -> Vec<ChartCandle> {
    let start = entry - Duration::minutes(30);
    let end = exit + Duration::minutes(30);
    let mut out = Vec::new();
    let mut t = start;
    let mut px = anchor * 0.998;
    let mut i = 0i64;
    while t <= end {
        let in_trade = t >= entry && t <= exit;
        let drift = if in_trade {
            if winner {
                0.00035
            } else {
                -0.00035
            }
        } else {
            0.00005 * if i % 2 == 0 { 1.0 } else { -1.0 }
        };
        let noise = ((i * 17) % 7) as f64 * 0.00008 - 0.00024;
        let open = px;
        let close = px * (1.0 + drift + noise);
        let high = open.max(close) * (1.0 + 0.0004);
        let low = open.min(close) * (1.0 - 0.0004);
        out.push(ChartCandle {
            time: t.timestamp(),
            open,
            high,
            low,
            close,
        });
        px = close;
        t += Duration::minutes(1);
        i += 1;
        if out.len() > 180 {
            break;
        }
    }
    out
}

fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}
