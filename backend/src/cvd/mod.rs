//! Live Futures Tape + Net Delta / CVD order-flow profile.
//!
//! Same paradigm as Unusual Whales Live Options Flow — but for CME futures
//! time & sales. Aggressive buy/sell size drives CVD and net delta.
//! Designed to sit on a Rust → Kafka → ClickHouse futures tape pipeline.

use crate::structure::{
    FuturesGex, SessionLevels, VolumeProfile, build_session_levels, build_volume_profile,
    empty_futures_gex,
};
use crate::uw::{FuturesCandle, FuturesContract, FuturesStats, FuturesTrade};
use serde::Serialize;
use std::collections::BTreeMap;

/// Large print threshold (contracts) — highlighted like options "whale" rows.
const LARGE_PRINT: i64 = 25;
const BLOCK_HINT: i64 = 100;

#[derive(Debug, Clone, Serialize)]
pub struct TapePrint {
    pub trade_id: Option<i64>,
    pub t: String,
    pub side: String,
    pub size: i64,
    pub price: f64,
    pub price_display: String,
    pub is_block: bool,
    pub is_large: bool,
    pub is_spread: bool,
    pub signed_size: i64,
    /// CVD after this print (session window).
    pub cvd: i64,
    pub nbbo_bid: Option<f64>,
    pub nbbo_ask: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CvdPoint {
    pub t: String,
    pub price: f64,
    pub signed_size: i64,
    pub buy_size: i64,
    pub sell_size: i64,
    pub cvd: i64,
    pub net_delta: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PriceLevelDelta {
    pub price: f64,
    pub buy_size: i64,
    pub sell_size: i64,
    pub net_delta: i64,
    pub total_size: i64,
    pub intensity: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderFlowStats {
    pub buy_volume: i64,
    pub sell_volume: i64,
    pub buy_pct: f64,
    pub sell_pct: f64,
    pub large_prints: usize,
    pub block_prints: usize,
    pub max_print_size: i64,
    pub aggression: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CvdSummary {
    pub trade_count: usize,
    pub buy_volume: i64,
    pub sell_volume: i64,
    pub net_delta: i64,
    pub cvd_high: i64,
    pub cvd_low: i64,
    pub cvd_last: i64,
    pub last_price: Option<f64>,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
    pub tick_size: f64,
    pub bias: String,
    pub narrative: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CvdProfile {
    pub contract: String,
    pub product: String,
    pub summary: CvdSummary,
    pub order_flow: OrderFlowStats,
    /// Newest-first live tape (UW options-flow style).
    pub tape: Vec<TapePrint>,
    pub series: Vec<CvdPoint>,
    pub levels: Vec<PriceLevelDelta>,
    pub volume_profile: VolumeProfile,
    pub sessions: SessionLevels,
    pub futures_gex: FuturesGex,
    pub stats: Option<FuturesStats>,
    pub methodology: String,
}

/// Build live tape + CVD + net-delta price profile from UW trades (newest-first).
pub fn build_profile(
    contract: &FuturesContract,
    trades: &[FuturesTrade],
    stats: Option<FuturesStats>,
    candles: &[FuturesCandle],
    futures_gex: Option<FuturesGex>,
) -> CvdProfile {
    let tick = contract
        .tick_size
        .as_ref()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|t| *t > 0.0)
        .unwrap_or(0.25);

    let mut chrono: Vec<&FuturesTrade> = trades.iter().collect();
    chrono.sort_by(|a, b| a.executed_at.cmp(&b.executed_at));

    let mut series = Vec::with_capacity(chrono.len());
    let mut tape_chrono = Vec::with_capacity(chrono.len());
    let mut cvd: i64 = 0;
    let mut buy_volume: i64 = 0;
    let mut sell_volume: i64 = 0;
    let mut cvd_high: i64 = 0;
    let mut cvd_low: i64 = 0;
    let mut large_prints = 0usize;
    let mut block_prints = 0usize;
    let mut max_print_size: i64 = 0;
    let mut levels: BTreeMap<i64, (i64, i64)> = BTreeMap::new();

    for t in &chrono {
        let size = t.size.max(0) as i64;
        let side = t.side.to_ascii_lowercase();
        let (buy, sell, signed) = match side.as_str() {
            "buy" => (size, 0, size),
            "sell" => (0, size, -size),
            _ => (0, 0, 0),
        };
        buy_volume += buy;
        sell_volume += sell;
        cvd += signed;
        cvd_high = cvd_high.max(cvd);
        cvd_low = cvd_low.min(cvd);
        max_print_size = max_print_size.max(size);

        let is_block = t.is_block.unwrap_or(false) || size >= BLOCK_HINT;
        let is_large = size >= LARGE_PRINT;
        if is_large {
            large_prints += 1;
        }
        if is_block {
            block_prints += 1;
        }

        let price = t.price_f64().unwrap_or(0.0);
        let bucket = price_bucket(price, tick);
        let entry = levels.entry(bucket).or_insert((0, 0));
        entry.0 += buy;
        entry.1 += sell;

        series.push(CvdPoint {
            t: t.executed_at.clone(),
            price,
            signed_size: signed,
            buy_size: buy,
            sell_size: sell,
            cvd,
            net_delta: buy_volume - sell_volume,
        });

        tape_chrono.push(TapePrint {
            trade_id: t.trade_id,
            t: t.executed_at.clone(),
            side: side.clone(),
            size,
            price,
            price_display: t
                .price_display
                .clone()
                .unwrap_or_else(|| format!("{price:.2}")),
            is_block,
            is_large,
            is_spread: t.is_spread.unwrap_or(false),
            signed_size: signed,
            cvd,
            nbbo_bid: t.nbbo_bid.as_ref().and_then(|s| s.parse().ok()),
            nbbo_ask: t.nbbo_ask.as_ref().and_then(|s| s.parse().ok()),
        });
    }

    // Tape: newest first (matches UW Live Flow).
    let mut tape = tape_chrono;
    tape.reverse();

    let max_abs_level = levels
        .values()
        .map(|(b, s)| (b - s).abs())
        .max()
        .unwrap_or(1)
        .max(1) as f64;

    let levels: Vec<PriceLevelDelta> = levels
        .into_iter()
        .rev()
        .map(|(bucket, (buy, sell))| {
            let net = buy - sell;
            PriceLevelDelta {
                price: bucket as f64 * tick,
                buy_size: buy,
                sell_size: sell,
                net_delta: net,
                total_size: buy + sell,
                intensity: (net.abs() as f64 / max_abs_level).clamp(0.0, 1.0),
            }
        })
        .collect();

    let net_delta = buy_volume - sell_volume;
    let total = (buy_volume + sell_volume).max(1) as f64;
    let buy_pct = (buy_volume as f64 / total) * 100.0;
    let sell_pct = (sell_volume as f64 / total) * 100.0;
    let last_price = series.last().map(|p| p.price);
    let bias = if net_delta > 0 {
        "buyers"
    } else if net_delta < 0 {
        "sellers"
    } else {
        "balanced"
    }
    .to_string();
    let aggression = if buy_pct >= 58.0 {
        "aggressive_buy"
    } else if sell_pct >= 58.0 {
        "aggressive_sell"
    } else {
        "two_way"
    }
    .to_string();

    let narrative = format!(
        "{} live tape: net delta {net_delta:+} · CVD {cvd} (range {cvd_low}…{cvd_high}). \
         Aggressive flow {buy_pct:.0}% buy / {sell_pct:.0}% sell · {large_prints} large · {block_prints} block.",
        contract.name
    );

    let volume_profile = build_volume_profile(trades, tick);
    let sessions = build_session_levels(candles, stats.as_ref());
    let product = contract.product.clone().unwrap_or_default();
    let futures_gex = futures_gex.unwrap_or_else(|| empty_futures_gex(&product));

    CvdProfile {
        contract: contract.name.clone(),
        product: product.clone(),
        summary: CvdSummary {
            trade_count: series.len(),
            buy_volume,
            sell_volume,
            net_delta,
            cvd_high,
            cvd_low,
            cvd_last: cvd,
            last_price,
            first_ts: series.first().map(|p| p.t.clone()),
            last_ts: series.last().map(|p| p.t.clone()),
            tick_size: tick,
            bias,
            narrative,
        },
        order_flow: OrderFlowStats {
            buy_volume,
            sell_volume,
            buy_pct: round1(buy_pct),
            sell_pct: round1(sell_pct),
            large_prints,
            block_prints,
            max_print_size,
            aggression,
        },
        tape,
        series,
        levels,
        volume_profile,
        sessions,
        futures_gex,
        stats,
        methodology: "Live futures time & sales from Unusual Whales. Buy aggressor = +size, \
sell = −size. CVD = cumulative signed volume (Sierra/Bookmap-style). Net delta = buy − sell. \
Includes Auction Market Theory POC/VAH/VAL, Globex vs RTH, and native Futures GEX / \
zero-gamma flip from index options proxies — the same toolkit traders already use on \
Sierra/Bookmap/SpotGamma, fed by a Rust/Kafka/ClickHouse-ready tape."
            .into(),
    }
}

fn price_bucket(price: f64, tick: f64) -> i64 {
    if tick <= 0.0 || !price.is_finite() {
        return 0;
    }
    (price / tick).round() as i64
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}
