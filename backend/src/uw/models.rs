//! Response types for the Unusual Whales endpoints used by Risk Watcher.
//!
//! Many UW numeric fields arrive as decimal *strings*; we keep them as `String`
//! (or optional string) at the boundary and parse in the risk engine.

use serde::{Deserialize, Serialize};

/// Standard UW envelope: `{ "data": T }`.
#[derive(Debug, Clone, Deserialize)]
pub struct UwEnvelope<T> {
    pub data: T,
}

// ---------------------------------------------------------------------------
// Ticker info — GET /api/stock/{ticker}/info
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TickerInfo {
    pub symbol: String,
    pub full_name: Option<String>,
    pub short_name: Option<String>,
    pub sector: Option<String>,
    pub issue_type: Option<String>,
    pub has_options: Option<bool>,
    pub marketcap: Option<String>,
    pub avg30_volume: Option<String>,
    pub next_earnings_date: Option<String>,
    pub next_earnings_confirmed: Option<bool>,
    pub beta: Option<String>,
}

// ---------------------------------------------------------------------------
// GEX levels — GET /api/stock/{ticker}/gex-levels
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GexSource {
    Vol,
    Oi,
}

impl GexSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vol => "vol",
            Self::Oi => "oi",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GexLevels {
    pub call_wall: Option<String>,
    pub put_wall: Option<String>,
    pub gamma_magnet: Option<String>,
    pub gamma_flip: Option<String>,
    pub nearby_flips: Option<Vec<String>>,
    pub source: Option<String>,
    pub date: Option<String>,
    pub time: Option<String>,
}

impl GexLevels {
    /// Parse `gamma_flip` as f64 when present.
    pub fn gamma_flip_f64(&self) -> Option<f64> {
        self.gamma_flip.as_ref()?.parse().ok()
    }

    pub fn call_wall_f64(&self) -> Option<f64> {
        self.call_wall.as_ref()?.parse().ok()
    }

    pub fn put_wall_f64(&self) -> Option<f64> {
        self.put_wall.as_ref()?.parse().ok()
    }
}

// ---------------------------------------------------------------------------
// Greek exposure — GET /api/stock/{ticker}/greek-exposure
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GreekExposure {
    pub date: Option<String>,
    pub call_gamma: Option<String>,
    pub put_gamma: Option<String>,
    pub call_delta: Option<String>,
    pub put_delta: Option<String>,
    pub call_vanna: Option<String>,
    pub put_vanna: Option<String>,
    pub call_charm: Option<String>,
    pub put_charm: Option<String>,
}

impl GreekExposure {
    pub fn net_gamma(&self) -> Option<f64> {
        let call: f64 = self.call_gamma.as_ref()?.parse().ok()?;
        let put: f64 = self.put_gamma.as_ref()?.parse().ok()?;
        Some(call + put)
    }

    /// Positive net gamma → dampening regime; negative → amplifying.
    pub fn is_positive_gamma_regime(&self) -> Option<bool> {
        self.net_gamma().map(|g| g > 0.0)
    }
}

// ---------------------------------------------------------------------------
// Flow alerts — GET /api/stock/{ticker}/flow-alerts
//              — GET /api/option-trades/flow-alerts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FlowAlert {
    #[serde(rename = "type")]
    pub option_type: Option<String>,
    pub ticker: Option<String>,
    pub expiry: Option<String>,
    pub strike: Option<String>,
    pub option_chain: Option<String>,
    pub underlying_price: Option<String>,
    pub price: Option<String>,
    pub volume: Option<i64>,
    pub open_interest: Option<i64>,
    pub total_premium: Option<String>,
    pub total_size: Option<i64>,
    pub total_ask_side_prem: Option<String>,
    pub total_bid_side_prem: Option<String>,
    pub volume_oi_ratio: Option<String>,
    pub alert_rule: Option<String>,
    pub has_sweep: Option<bool>,
    pub has_floor: Option<bool>,
    pub has_multileg: Option<bool>,
    pub has_singleleg: Option<bool>,
    pub all_opening_trades: Option<bool>,
    pub delta: Option<f64>,
    pub gamma: Option<f64>,
    pub theta: Option<f64>,
    pub vega: Option<f64>,
    pub iv: Option<f64>,
    pub created_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Stock state — GET /api/stock/{ticker}/stock-state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StockState {
    pub open: Option<String>,
    pub high: Option<String>,
    pub low: Option<String>,
    pub close: Option<String>,
    pub prev_close: Option<String>,
    pub volume: Option<i64>,
    pub total_volume: Option<i64>,
    pub market_time: Option<String>,
    pub tape_time: Option<String>,
}

// ---------------------------------------------------------------------------
// Market tide — GET /api/market/market-tide
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketTidePoint {
    pub timestamp: Option<String>,
    pub date: Option<String>,
    pub net_call_premium: Option<String>,
    pub net_put_premium: Option<String>,
    pub net_volume: Option<i64>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// IV Rank — GET /api/stock/{ticker}/iv-rank
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IvRankPoint {
    pub date: Option<String>,
    pub close: Option<String>,
    pub volatility: Option<String>,
    pub iv_rank_1y: Option<String>,
    pub updated_at: Option<String>,
}

impl IvRankPoint {
    /// Normalize IV rank to 0–100 (UW may send 0–1 or 0–100 strings).
    pub fn iv_rank_pct(&self) -> Option<f64> {
        let raw: f64 = self.iv_rank_1y.as_ref()?.parse().ok()?;
        if raw <= 1.0 {
            Some(raw * 100.0)
        } else {
            Some(raw.clamp(0.0, 100.0))
        }
    }

    pub fn volatility_f64(&self) -> Option<f64> {
        self.volatility.as_ref()?.parse().ok()
    }
}

// ---------------------------------------------------------------------------
// Interpolated IV — GET /api/stock/{ticker}/interpolated-iv
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InterpolatedIvPoint {
    pub date: Option<String>,
    pub days: Option<i64>,
    pub implied_move_perc: Option<String>,
    pub percentile: Option<String>,
    pub volatility: Option<String>,
}

impl InterpolatedIvPoint {
    pub fn percentile_pct(&self) -> Option<f64> {
        let raw: f64 = self.percentile.as_ref()?.parse().ok()?;
        if raw <= 1.0 {
            Some(raw * 100.0)
        } else {
            Some(raw.clamp(0.0, 100.0))
        }
    }

    pub fn volatility_f64(&self) -> Option<f64> {
        self.volatility.as_ref()?.parse().ok()
    }
}

/// Historical 25Δ / 10Δ risk reversal — GET /api/stock/{ticker}/historical-risk-reversal-skew
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskReversalPoint {
    pub date: Option<String>,
    pub ticker: Option<String>,
    pub delta: Option<i64>,
    pub risk_reversal: Option<String>,
}

impl RiskReversalPoint {
    /// Put IV − call IV (UW convention). Positive = put skew rich.
    pub fn risk_reversal_f64(&self) -> Option<f64> {
        self.risk_reversal.as_ref()?.parse().ok()
    }
}

/// Dark pool aggregated price levels — GET /api/darkpool/{ticker}/price-levels
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DarkPoolLevel {
    pub price: Option<String>,
    pub dark_pool_volume: Option<String>,
    pub regular_volume: Option<String>,
}

impl DarkPoolLevel {
    pub fn price_f64(&self) -> Option<f64> {
        self.price.as_ref()?.parse().ok()
    }

    pub fn dark_pool_volume_f64(&self) -> Option<f64> {
        self.dark_pool_volume.as_ref()?.parse().ok()
    }
}

/// One row from option-contract historic (liquidity / NBBO).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OptionContractTape {
    pub date: Option<String>,
    pub volume: Option<i64>,
    pub open_interest: Option<i64>,
    pub nbbo_bid: Option<String>,
    pub nbbo_ask: Option<String>,
    pub last_price: Option<String>,
    pub implied_volatility: Option<String>,
    pub total_premium: Option<String>,
}

impl OptionContractTape {
    pub fn spread_pct(&self) -> Option<f64> {
        let bid: f64 = self.nbbo_bid.as_ref()?.parse().ok()?;
        let ask: f64 = self.nbbo_ask.as_ref()?.parse().ok()?;
        let mid = (bid + ask) / 2.0;
        if mid <= 0.0 {
            return None;
        }
        Some(((ask - bid).abs() / mid) * 100.0)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OptionContractHistoric {
    pub chains: Option<Vec<OptionContractTape>>,
}
