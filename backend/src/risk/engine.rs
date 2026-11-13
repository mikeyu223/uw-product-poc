use super::catalog::{
    MAX_ADVERSE_FLOW, MAX_CATALYST, MAX_FLIP_PROXIMITY, MAX_GAMMA_REGIME, MAX_IV_REGIME,
    MAX_STRUCTURE, MAX_WALL_CONFLICT, TOTAL_MAX, driver_max,
};
use super::curves::{exp_decay_near, log_ratio_cap, log_saturate, sigmoid_proximity};
use super::{PositionRisk, RiskBucket, RiskDriver, ScoreBreakdown};
use crate::uw::{FlowAlert, GexLevels, GreekExposure, InterpolatedIvPoint, IvRankPoint, TickerInfo};
use chrono::{NaiveDate, Utc};

/// User options position (book side) — strikes/qty are portfolio truth;
/// market fields come from live UW.
#[derive(Debug, Clone)]
pub struct PositionInput {
    pub id: String,
    pub ticker: String,
    pub option_type: String, // "call" | "put"
    pub strike: f64,
    pub expiry: String, // YYYY-MM-DD
    pub quantity: i32,
    pub side: String, // "long" | "short"
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
}

/// Live market microstructure used for scoring.
#[derive(Debug, Clone)]
pub struct MarketContext {
    pub spot: f64,
    pub gex: GexLevels,
    pub greek: Option<GreekExposure>,
    pub flow: Vec<FlowAlert>,
    pub info: Option<TickerInfo>,
    pub iv_rank: Option<IvRankPoint>,
    pub interpolated_iv: Option<InterpolatedIvPoint>,
}

const METHODOLOGY: &str = "Hand-tuned 0–100 aggregate from mathematical proximity-to-harm drivers \
(no ML). Family caps: GEX 20, flip 18, walls 14, flow 15, IV 13, structure 8, catalyst 12. \
Each driver shows as contribution/max (e.g. 4/20). Non-linear curves asymptote to caps.";

pub fn score_position(pos: &PositionInput, mkt: &MarketContext) -> PositionRisk {
    let mut breakdown = ScoreBreakdown::default();
    let mut drivers: Vec<RiskDriver> = Vec::new();

    let is_call = pos.option_type.eq_ignore_ascii_case("call");
    let is_long = !pos.side.eq_ignore_ascii_case("short");
    let spot = mkt.spot;

    breakdown.gamma_regime = score_gamma_regime(mkt, &mut drivers);
    breakdown.flip_proximity = score_flip_proximity(spot, &mkt.gex, &mut drivers);
    breakdown.wall_conflict =
        score_wall_conflict(pos, spot, is_call, is_long, &mkt.gex, &mut drivers);
    breakdown.adverse_flow = score_adverse_flow(pos, is_call, is_long, &mkt.flow, &mut drivers);
    breakdown.iv_regime = score_iv_regime(pos, is_long, mkt, &mut drivers);
    breakdown.structure = score_structure(pos, spot, is_call, &mut drivers);
    breakdown.catalyst = score_catalyst(mkt.info.as_ref(), &mut drivers);

    breakdown.gamma_regime = round1(breakdown.gamma_regime);
    breakdown.flip_proximity = round1(breakdown.flip_proximity);
    breakdown.wall_conflict = round1(breakdown.wall_conflict);
    breakdown.adverse_flow = round1(breakdown.adverse_flow);
    breakdown.iv_regime = round1(breakdown.iv_regime);
    breakdown.structure = round1(breakdown.structure);
    breakdown.catalyst = round1(breakdown.catalyst);

    let raw = breakdown.total().clamp(0.0, TOTAL_MAX);
    let score = raw.round() as u8;
    let bucket = RiskBucket::from_score(score);
    let family_scores = breakdown.family_scores();

    drivers.sort_by(|a, b| {
        b.contribution
            .partial_cmp(&a.contribution)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // Keep active drivers; UI shows X/max per row
    drivers.retain(|d| d.contribution > 0.05);
    drivers.truncate(8);

    let data_quality = if mkt.greek.is_some() && !mkt.flow.is_empty() && mkt.gex.gamma_flip.is_some()
    {
        "live_uw_gex_flow_greeks".into()
    } else if mkt.gex.gamma_flip.is_some() {
        "live_uw_partial".into()
    } else {
        "degraded".into()
    };

    PositionRisk {
        position_id: pos.id.clone(),
        score,
        score_max: TOTAL_MAX as u8,
        bucket,
        drivers,
        breakdown,
        family_scores,
        methodology: METHODOLOGY.into(),
        data_quality,
    }
}

fn push(
    drivers: &mut Vec<RiskDriver>,
    code: &str,
    label: &str,
    pts: f64,
    family: &str,
    detail: String,
) {
    let max = driver_max(code).max(1.0);
    drivers.push(RiskDriver::new(code, label, pts, max, family, Some(detail)));
}

fn score_gamma_regime(mkt: &MarketContext, drivers: &mut Vec<RiskDriver>) -> f64 {
    let Some(greek) = &mkt.greek else {
        return 0.0;
    };
    let Some(net) = greek.net_gamma() else {
        return 0.0;
    };

    if net < 0.0 {
        let mag = (-net).max(0.0);
        let pts = log_ratio_cap(mag, 2_000_000.0, MAX_GAMMA_REGIME).clamp(8.0, MAX_GAMMA_REGIME);
        push(
            drivers,
            "neg_gex_regime",
            "Negative dealer gamma (amplifying)",
            pts,
            "gamma_regime",
            format!(
                "Metric: |net GEX|={mag:.0} (net={net:.0}). Log curve → {pts:.1}/{max:.0}. Amplifying hedge regime.",
                max = MAX_GAMMA_REGIME
            ),
        );
        pts
    } else {
        let pts = (2.0 + log_saturate(net, 5_000_000.0, 3.0)).min(5.0);
        push(
            drivers,
            "pos_gex_regime",
            "Positive dealer gamma (dampening)",
            pts,
            "gamma_regime",
            format!(
                "Metric: net GEX={net:.0} > 0. Residual dampening-regime points {pts:.1}/5 (pin/complacency)."
            ),
        );
        pts
    }
}

fn score_flip_proximity(spot: f64, gex: &GexLevels, drivers: &mut Vec<RiskDriver>) -> f64 {
    let Some(flip) = gex.gamma_flip_f64() else {
        return 0.0;
    };
    if spot <= 0.0 {
        return 0.0;
    }
    let dist_pct = ((spot - flip).abs() / spot) * 100.0;
    // Tight calibration (was mid=0.55%, k=5 — too hot; everything looked "near"):
    //   ≤~0.15% → ~17–20/20
    //   ~0.5%   → ~5–6/20
    //   ≥1%     → noise floor
    const FLIP_MID_PCT: f64 = 0.38; // 50% of cap at 0.38% distance
    const FLIP_K: f64 = 8.5; // steeper drop-off away from the line
    let pts = sigmoid_proximity(dist_pct, FLIP_MID_PCT, FLIP_K, MAX_FLIP_PROXIMITY);

    let side = if spot >= flip { "above" } else { "below" };
    push(
        drivers,
        "zero_gamma_flip",
        "Near dealer zero-gamma flip",
        pts,
        "flip_proximity",
        format!(
            "Metric: |spot−flip|/spot = {dist_pct:.2}% ({side} {flip:.2}). Tight sigmoid (mid={FLIP_MID_PCT}%, k={FLIP_K}) → {pts:.1}/{max:.0}. High points only when truly on the flip.",
            max = MAX_FLIP_PROXIMITY
        ),
    );
    pts
}

fn score_wall_conflict(
    pos: &PositionInput,
    spot: f64,
    is_call: bool,
    is_long: bool,
    gex: &GexLevels,
    drivers: &mut Vec<RiskDriver>,
) -> f64 {
    let call_wall = gex.call_wall_f64();
    let put_wall = gex.put_wall_f64();
    let magnet = gex.gamma_magnet.as_ref().and_then(|s| s.parse::<f64>().ok());

    let mut pts = 0.0;

    if is_call && is_long {
        if let Some(cw) = call_wall {
            let dist = ((pos.strike - cw).abs() / spot.max(1.0)) * 100.0;
            if dist <= 3.0 || (pos.strike >= cw && (pos.strike - cw) / spot <= 0.04) {
                let add = exp_decay_near(dist, 0.7, 12.0);
                if add >= 1.0 {
                    pts += add;
                    push(
                        drivers,
                        "call_wall_headwind",
                        "Call wall headwind",
                        add,
                        "wall_conflict",
                        format!(
                            "Metric: |strike−call_wall|/spot = {dist:.2}% (strike {strike:.0}, wall {cw:.0}). Exp-decay → {add:.1}/12.",
                            strike = pos.strike
                        ),
                    );
                }
            }
        }
    }

    if !is_call && is_long {
        if let Some(pw) = put_wall {
            let dist = ((pos.strike - pw).abs() / spot.max(1.0)) * 100.0;
            if dist <= 3.0 {
                let add = exp_decay_near(dist, 0.7, 10.0);
                if add >= 1.0 {
                    pts += add;
                    push(
                        drivers,
                        "put_wall_magnet",
                        "Put wall magnet / support",
                        add,
                        "wall_conflict",
                        format!(
                            "Metric: |strike−put_wall|/spot = {dist:.2}% (strike {strike:.0}, wall {pw:.0}). Exp-decay → {add:.1}/10.",
                            strike = pos.strike
                        ),
                    );
                }
            }
        }
    }

    if !is_long {
        if let Some(m) = magnet {
            let dist = ((pos.strike - m).abs() / spot.max(1.0)) * 100.0;
            if dist <= 2.0 {
                let add = exp_decay_near(dist, 0.5, 8.0);
                if add >= 1.0 {
                    pts += add;
                    push(
                        drivers,
                        "gamma_magnet_pin",
                        "Gamma magnet pin risk",
                        add,
                        "wall_conflict",
                        format!(
                            "Metric: |strike−magnet|/spot = {dist:.2}% (strike {strike:.0}, magnet {m:.0}). Exp-decay → {add:.1}/8.",
                            strike = pos.strike
                        ),
                    );
                }
            }
        }
    }

    pts.min(MAX_WALL_CONFLICT)
}

fn score_adverse_flow(
    pos: &PositionInput,
    is_call: bool,
    is_long: bool,
    flow: &[FlowAlert],
    drivers: &mut Vec<RiskDriver>,
) -> f64 {
    if flow.is_empty() {
        return 0.0;
    }

    let mut adverse_prem = 0.0_f64;
    let mut support_prem = 0.0_f64;
    let mut sweep_hits = 0_u32;
    let mut near_strike_hits = 0_u32;

    for a in flow {
        let prem: f64 = a
            .total_premium
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let ask_prem: f64 = a
            .total_ask_side_prem
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let typ = a.option_type.as_deref().unwrap_or("").to_ascii_lowercase();
        let strike: f64 = a.strike.as_ref().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let near = (strike - pos.strike).abs() <= pos.strike * 0.03 + 2.0;

        if a.has_sweep == Some(true) {
            sweep_hits += 1;
        }
        if near {
            near_strike_hits += 1;
        }

        let aggressive = ask_prem > prem * 0.55 || ask_prem > 50_000.0;
        let flow_is_call = typ == "call";
        let adverse = match (is_long, is_call, flow_is_call) {
            (true, true, false) => aggressive,
            (true, false, true) => aggressive,
            (false, true, true) => aggressive,
            (false, false, false) => aggressive,
            _ => false,
        };
        let support = match (is_long, is_call, flow_is_call) {
            (true, true, true) => aggressive,
            (true, false, false) => aggressive,
            _ => false,
        };

        if adverse {
            adverse_prem += ask_prem.max(prem * 0.5);
        }
        if support {
            support_prem += ask_prem.max(prem * 0.5);
        }
    }

    let net_adverse = (adverse_prem - support_prem * 0.35).max(0.0);
    let mut pts = log_saturate(net_adverse, 250_000.0, MAX_ADVERSE_FLOW);

    let headroom = (MAX_ADVERSE_FLOW - pts).max(0.0);
    if sweep_hits >= 2 {
        pts += (headroom * 0.15).min(2.5);
    }
    if near_strike_hits >= 2 {
        pts += ((MAX_ADVERSE_FLOW - pts).max(0.0) * 0.10).min(1.5);
    }
    pts = pts.min(MAX_ADVERSE_FLOW);

    let direction = if is_call { "calls" } else { "puts" };
    let against = if is_call { "puts" } else { "calls" };
    let label = if support_prem > adverse_prem * 1.25 && support_prem > 50_000.0 {
        format!("Supportive {direction} flow (offsets risk)")
    } else if net_adverse > 25_000.0 {
        format!("Adverse {against} flow vs your {direction}")
    } else {
        "Options flow relatively balanced".into()
    };

    if support_prem > adverse_prem * 1.25 {
        pts = pts.min(8.0);
    }

    push(
        drivers,
        "adverse_flow",
        &label,
        pts,
        "adverse_flow",
        format!(
            "Metric: net adverse ${net_adverse:.0} (against ${adverse_prem:.0}, support ${support_prem:.0}). Log scale $250k → {pts:.1}/{max:.0}. Sweeps={sweep_hits}, near-strike={near_strike_hits}.",
            max = MAX_ADVERSE_FLOW
        ),
    );

    pts
}

fn score_structure(
    pos: &PositionInput,
    spot: f64,
    is_call: bool,
    drivers: &mut Vec<RiskDriver>,
) -> f64 {
    let dte = days_to_expiry(&pos.expiry).unwrap_or(30) as f64;
    let moneyness = if is_call {
        (pos.strike - spot) / spot
    } else {
        (spot - pos.strike) / spot
    };

    let dte_pts = sigmoid_proximity(dte, 8.0, 0.35, 6.0);
    let mut pts = dte_pts;

    if dte_pts >= 2.0 {
        push(
            drivers,
            "short_dte",
            if dte <= 3.0 {
                "Very short-dated structure"
            } else {
                "Short-dated structure risk"
            },
            dte_pts,
            "structure",
            format!("Metric: DTE={dte:.0}. Sigmoid(mid=8d) → {dte_pts:.1}/6. Gamma/charm rise into expiry."),
        );
    }

    if moneyness > 0.0 {
        let otm_pts = log_saturate(moneyness * 100.0, 6.0, 2.0);
        if otm_pts >= 0.8 {
            pts += otm_pts;
            push(
                drivers,
                "otm_lottery",
                "Out-of-the-money structure",
                otm_pts,
                "structure",
                format!(
                    "Metric: OTM={pct:.1}% (strike {strike:.0}, spot {spot:.2}). Log → {otm_pts:.1}/2.",
                    strike = pos.strike,
                    pct = moneyness * 100.0
                ),
            );
        }
    }

    if pos.gamma.abs() >= 0.02 && dte <= 10.0 {
        let g_pts = 1.5_f64;
        pts += g_pts;
        push(
            drivers,
            "high_gamma_cluster",
            "High gamma into short DTE",
            g_pts,
            "structure",
            format!(
                "Metric: |γ|={g:.3} with DTE={dte:.0}≤10. Flat add {g_pts:.1}/2 within structure family.",
                g = pos.gamma.abs()
            ),
        );
    }

    pts.min(MAX_STRUCTURE)
}

fn score_iv_regime(
    pos: &PositionInput,
    is_long: bool,
    mkt: &MarketContext,
    drivers: &mut Vec<RiskDriver>,
) -> f64 {
    let iv_rank = mkt.iv_rank.as_ref().and_then(|r| r.iv_rank_pct());
    let iv_vol = mkt.iv_rank.as_ref().and_then(|r| r.volatility_f64());
    let iv30_pct = mkt.interpolated_iv.as_ref().and_then(|r| r.percentile_pct());
    let iv30_vol = mkt.interpolated_iv.as_ref().and_then(|r| r.volatility_f64());

    // Prefer 1y IV Rank; fall back to 30d interpolated IV percentile.
    let Some(rank) = iv_rank.or(iv30_pct) else {
        return 0.0;
    };

    let days_earn = mkt
        .info
        .as_ref()
        .and_then(|i| i.next_earnings_date.as_ref())
        .and_then(|d| days_until_date(d));

    let near_earnings = matches!(days_earn, Some(d) if (0..=7).contains(&d));

    // Blend in 30d percentile when both exist (mild lift toward the hotter reading).
    let effective = match (iv_rank, iv30_pct) {
        (Some(a), Some(b)) => a.max(b) * 0.7 + a.min(b) * 0.3,
        _ => rank,
    };

    let (mut pts, code, label, harm_side) = if is_long {
        // Long premium: high IVR = crush / overpaid vega hazard.
        // Distance-from-safe uses (100 - IVR) so high IVR → low distance → high points.
        let dist_from_cheap = (100.0 - effective).max(0.0);
        let pts = sigmoid_proximity(dist_from_cheap, 35.0, 0.08, MAX_IV_REGIME);
        (
            pts,
            "iv_crush_long",
            "High IV Rank crush risk (long premium)",
            "long",
        )
    } else {
        // Short premium: low IVR = expansion hazard (selling cheap vol).
        let pts = sigmoid_proximity(effective, 35.0, 0.08, MAX_IV_REGIME);
        (
            pts,
            "iv_expansion_short",
            "Low IV Rank expansion risk (short premium)",
            "short",
        )
    };

    // Interaction: long + earnings ≤7d + rich IV → crush trap boost.
    let mut boost_note = String::new();
    if is_long && near_earnings && effective >= 80.0 {
        let boosted = (pts * 1.25).min(MAX_IV_REGIME);
        if boosted > pts {
            boost_note = format!(
                " Earnings≤7d + IVR≥80% boost ×1.25 → {boosted:.1}/{max:.0}.",
                max = MAX_IV_REGIME
            );
            pts = boosted;
        }
    }

    if pts < 1.0 {
        return 0.0;
    }

    let fmt_iv = |v: f64| -> String {
        // UW may send 0.63 or 63.0 depending on endpoint.
        if v <= 3.0 {
            format!("{:.1}%", v * 100.0)
        } else {
            format!("{v:.1}%")
        }
    };
    let vol_bits = match (iv_vol.or(iv30_vol), iv30_pct) {
        (Some(v), Some(p30)) => format!(" IV≈{} · 30d IV pctile={p30:.1}%.", fmt_iv(v)),
        (Some(v), None) => format!(" IV≈{}.", fmt_iv(v)),
        (None, Some(p30)) => format!(" 30d IV pctile={p30:.1}%."),
        _ => String::new(),
    };

    push(
        drivers,
        code,
        label,
        pts,
        "iv_regime",
        format!(
            "Metric: IV Rank={effective:.1}% ({harm_side} {side}). Position-aware sigmoid → {pts:.1}/{max:.0}.{vol_bits}{boost_note}",
            side = pos.side,
            max = MAX_IV_REGIME
        ),
    );
    pts.min(MAX_IV_REGIME)
}

fn score_catalyst(info: Option<&TickerInfo>, drivers: &mut Vec<RiskDriver>) -> f64 {
    let Some(info) = info else {
        return 0.0;
    };
    let Some(date_s) = info.next_earnings_date.as_ref() else {
        return 0.0;
    };
    let Some(days) = days_until_date(date_s) else {
        return 0.0;
    };
    if days < 0 {
        return 0.0;
    }

    let pts = sigmoid_proximity(days as f64, 7.0, 0.4, MAX_CATALYST);
    if pts < 1.0 {
        return 0.0;
    }

    let confirmed = info.next_earnings_confirmed.unwrap_or(false);
    let label = if days <= 2 {
        "Earnings within 2 sessions"
    } else if days <= 7 {
        "Earnings inside a week"
    } else {
        "Earnings on the horizon"
    };

    push(
        drivers,
        "earnings_catalyst",
        label,
        pts,
        "catalyst",
        format!(
            "Metric: days_to_earnings={days}. Sigmoid(mid=7d) → {pts:.1}/{max:.0}. Date {date_s}{}.",
            if confirmed { " (confirmed)" } else { " (estimate)" },
            max = MAX_CATALYST
        ),
    );
    pts
}

fn days_to_expiry(expiry: &str) -> Option<i64> {
    days_until_date(expiry)
}

fn days_until_date(date_s: &str) -> Option<i64> {
    let date = NaiveDate::parse_from_str(&date_s[..10.min(date_s.len())], "%Y-%m-%d").ok()?;
    let today = Utc::now().date_naive();
    Some((date - today).num_days())
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}
