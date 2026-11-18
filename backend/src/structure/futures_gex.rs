//! Native Futures GEX & Zero-Gamma Flip.
//!
//! Index futures traders already mark SpotGamma / UW flip levels on ES & NQ.
//! We project Unusual Whales options GEX (SPX→ES, NDX→NQ, RUT/IWM→RTY)
//! onto the live futures tape — same dealer-gamma framing, no invented math.

use super::gex_heatmap::GexHeatmap;
use crate::uw::{GexLevels, GreekExposure};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FuturesGexLevel {
    pub name: String,
    pub price: f64,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FuturesGex {
    pub product: String,
    pub proxy_ticker: String,
    pub futures_last: Option<f64>,
    pub proxy_spot: Option<f64>,
    /// Zero-gamma / gamma flip in futures price space.
    pub zero_gamma_flip: Option<f64>,
    pub call_wall: Option<f64>,
    pub put_wall: Option<f64>,
    pub gamma_magnet: Option<f64>,
    pub net_gamma: Option<f64>,
    pub regime: String,
    pub regime_label: String,
    /// Distance from futures last to flip (points).
    pub distance_to_flip: Option<f64>,
    pub distance_to_flip_pct: Option<f64>,
    pub position_vs_flip: String,
    pub levels: Vec<FuturesGexLevel>,
    pub heatmap: Option<GexHeatmap>,
    pub narrative: String,
    pub methodology: String,
}

/// Map futures product → UW options GEX underlyings (preferred first).
pub fn gex_proxies(product: &str) -> &'static [&'static str] {
    match product.to_ascii_uppercase().as_str() {
        "ES" | "MES" => &["SPX", "SPY"],
        "NQ" | "MNQ" => &["NDX", "QQQ"],
        "RTY" | "M2K" => &["RUT", "IWM"],
        _ => &["SPX", "SPY"],
    }
}

/// Scale factor from proxy index/ETF price → futures-comparable points.
/// SPY≈SPX/10; QQQ≈NDX/40; IWM≈RUT roughly 1:1 for level projection when RUT missing.
pub fn proxy_scale(proxy: &str) -> f64 {
    match proxy.to_ascii_uppercase().as_str() {
        "SPY" => 10.0,
        "QQQ" => 40.0,
        "IWM" => 10.0, // rough when only IWM GEX available
        _ => 1.0,      // SPX, NDX, RUT already futures-scale
    }
}

pub fn build_futures_gex(
    product: &str,
    futures_last: Option<f64>,
    proxy_ticker: &str,
    proxy_spot: Option<f64>,
    gex: &GexLevels,
    greek: Option<&GreekExposure>,
    heatmap: Option<GexHeatmap>,
) -> FuturesGex {
    let scale = proxy_scale(proxy_ticker);
    let project = |v: Option<f64>| v.map(|x| x * scale);

    let zero_gamma_flip = project(gex.gamma_flip_f64());
    let call_wall = project(gex.call_wall_f64());
    let put_wall = project(gex.put_wall_f64());
    let gamma_magnet = project(
        gex.gamma_magnet
            .as_ref()
            .and_then(|s| s.parse::<f64>().ok()),
    );
    let net_gamma = greek.and_then(|g| g.net_gamma());

    let (distance_to_flip, distance_to_flip_pct, position_vs_flip) =
        match (futures_last, zero_gamma_flip) {
            (Some(last), Some(flip)) => {
                let dist = last - flip;
                let pct = if flip.abs() > 0.0 {
                    Some((dist / flip) * 100.0)
                } else {
                    None
                };
                let pos = if dist > 0.5 {
                    "above_flip"
                } else if dist < -0.5 {
                    "below_flip"
                } else {
                    "at_flip"
                };
                (Some(dist), pct, pos.to_string())
            }
            _ => (None, None, "n/a".into()),
        };

    // Regime: prefer net dealer gamma; refine with position vs flip when both exist.
    let (regime, regime_label) = match (net_gamma, position_vs_flip.as_str()) {
        (Some(g), _) if g > 0.0 => (
            "positive_gamma".to_string(),
            "Positive dealer gamma — mean-reverting tape (dealers dampen moves)".to_string(),
        ),
        (Some(g), _) if g < 0.0 => (
            "negative_gamma".to_string(),
            "Negative dealer gamma — volatile / amplifying tape".to_string(),
        ),
        (_, "above_flip") => (
            "above_zero_gamma".to_string(),
            "Futures trading above zero-gamma flip — typically calmer dealer hedging".to_string(),
        ),
        (_, "below_flip") => (
            "below_zero_gamma".to_string(),
            "Futures trading below zero-gamma flip — watch for faster, one-way moves".to_string(),
        ),
        (_, "at_flip") => (
            "at_zero_gamma".to_string(),
            "Pinned near zero-gamma flip — regime can flip with a few handles".to_string(),
        ),
        _ => (
            "unknown".to_string(),
            "Dealer GEX unavailable for this proxy".to_string(),
        ),
    };

    let mut levels = Vec::new();
    push_level(&mut levels, "Zero-Γ Flip", zero_gamma_flip, "flip");
    push_level(&mut levels, "Call Wall", call_wall, "wall");
    push_level(&mut levels, "Put Wall", put_wall, "wall");
    push_level(&mut levels, "Γ Magnet", gamma_magnet, "magnet");

    let narrative = match (futures_last, zero_gamma_flip, distance_to_flip) {
        (Some(last), Some(flip), Some(dist)) => format!(
            "{product} last {last:.2} vs zero-γ flip {flip:.2} ({dist:+.2} pts, {}). \
Proxy {proxy_ticker} GEX · {}",
            position_vs_flip.replace('_', " "),
            regime_label
        ),
        (Some(last), None, _) => format!(
            "{product} last {last:.2} · flip level unavailable from {proxy_ticker} GEX. {}",
            regime_label
        ),
        _ => format!("{product}: waiting on futures last + {proxy_ticker} GEX."),
    };

    FuturesGex {
        product: product.to_ascii_uppercase(),
        proxy_ticker: proxy_ticker.to_ascii_uppercase(),
        futures_last,
        proxy_spot,
        zero_gamma_flip,
        call_wall,
        put_wall,
        gamma_magnet,
        net_gamma,
        regime,
        regime_label,
        distance_to_flip,
        distance_to_flip_pct,
        position_vs_flip,
        levels,
        heatmap,
        narrative,
        methodology: "Dealer gamma from Unusual Whales options GEX on the index proxy \
(SPX→ES, NDX→NQ, RUT/IWM→RTY). Zero-gamma flip, call/put walls, magnet, and strike×expiry \
heatmap projected into futures price space. Same UW GEX terminal paradigm — on the live tape."
            .into(),
    }
}

fn push_level(out: &mut Vec<FuturesGexLevel>, name: &str, price: Option<f64>, kind: &str) {
    if let Some(price) = price {
        out.push(FuturesGexLevel {
            name: name.into(),
            price,
            kind: kind.into(),
        });
    }
}

pub fn empty_futures_gex(product: &str) -> FuturesGex {
    FuturesGex {
        product: product.to_ascii_uppercase(),
        proxy_ticker: String::new(),
        futures_last: None,
        proxy_spot: None,
        zero_gamma_flip: None,
        call_wall: None,
        put_wall: None,
        gamma_magnet: None,
        net_gamma: None,
        regime: "unknown".into(),
        regime_label: "No GEX proxy loaded".into(),
        distance_to_flip: None,
        distance_to_flip_pct: None,
        position_vs_flip: "n/a".into(),
        levels: vec![],
        heatmap: None,
        narrative: "Futures GEX not loaded.".into(),
        methodology: String::new(),
    }
}
