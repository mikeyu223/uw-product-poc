//! Complete catalog of position-harm drivers.
//!
//! Every driver is a **mathematical proximity-to-harm** signal — not a subjective
//! vibe score. Hand-tuned family budgets sum to 100. Flip proximity remains a
//! major microstructure family; IV regime captures crush / expansion hazard
//! that GEX alone misses.
//!
//! The UI should show each active driver as `contribution / max_points` (e.g. 4/20).

use serde::Serialize;

/// Family point budgets (hand-tuned). Must sum to 100.
/// Rebalanced to carve an IV/vega regime family out of GEX + flow, and lift catalyst.
pub const MAX_GAMMA_REGIME: f64 = 20.0;
pub const MAX_FLIP_PROXIMITY: f64 = 18.0;
pub const MAX_WALL_CONFLICT: f64 = 14.0;
pub const MAX_ADVERSE_FLOW: f64 = 15.0;
pub const MAX_IV_REGIME: f64 = 13.0;
pub const MAX_STRUCTURE: f64 = 8.0;
pub const MAX_CATALYST: f64 = 12.0;

pub const TOTAL_MAX: f64 = 100.0;

/// Spec for one possible driver that can hurt an options position.
#[derive(Debug, Clone, Serialize)]
pub struct DriverSpec {
    pub code: &'static str,
    pub family: &'static str,
    pub label: &'static str,
    /// Max points this driver can contribute (within its family budget).
    pub max_points: f64,
    /// What raw input is measured.
    pub metric: &'static str,
    /// How it maps to points (curve).
    pub curve: &'static str,
    /// Why it hurts the position (trader language).
    pub harm: &'static str,
    /// Which structures it typically applies to.
    pub applies_to: &'static str,
}

/// Family budget shown in the breakdown strip.
#[derive(Debug, Clone, Serialize)]
pub struct FamilySpec {
    pub family: &'static str,
    pub label: &'static str,
    pub max_points: f64,
    pub why: &'static str,
}

pub fn family_catalog() -> Vec<FamilySpec> {
    vec![
        FamilySpec {
            family: "gamma_regime",
            label: "Dealer GEX regime",
            max_points: MAX_GAMMA_REGIME,
            why: "Sign and magnitude of net dealer gamma — amplifying vs dampening hedge regime.",
        },
        FamilySpec {
            family: "flip_proximity",
            label: "Zero-gamma flip proximity",
            max_points: MAX_FLIP_PROXIMITY,
            why: "Distance of spot to the dealer zero-gamma flip. Closer = higher regime-change risk.",
        },
        FamilySpec {
            family: "wall_conflict",
            label: "Call/put wall conflict",
            max_points: MAX_WALL_CONFLICT,
            why: "Distance of your strike to dealer call/put walls or gamma magnet.",
        },
        FamilySpec {
            family: "adverse_flow",
            label: "Adverse options flow",
            max_points: MAX_ADVERSE_FLOW,
            why: "Aggressive premium flowing against your side (log-scaled $).",
        },
        FamilySpec {
            family: "iv_regime",
            label: "IV / vega regime",
            max_points: MAX_IV_REGIME,
            why: "IV Rank + term structure backwardation + 25Δ risk-reversal skew. Crush (long) / expansion (short).",
        },
        FamilySpec {
            family: "structure",
            label: "Structure (DTE / moneyness)",
            max_points: MAX_STRUCTURE,
            why: "Contract fragility: short DTE, OTM lottery, high gamma, and bid-ask / OI exit friction.",
        },
        FamilySpec {
            family: "catalyst",
            label: "Earnings / catalyst",
            max_points: MAX_CATALYST,
            why: "Calendar distance to binary events that can gap through your structure.",
        },
    ]
}

/// All drivers the engine can emit (and planned mathematical harm signals).
pub fn driver_catalog() -> Vec<DriverSpec> {
    vec![
        // --- gamma_regime ---
        DriverSpec {
            code: "neg_gex_regime",
            family: "gamma_regime",
            label: "Negative dealer gamma (amplifying)",
            max_points: MAX_GAMMA_REGIME,
            metric: "|net_dealer_gex| when net_gex < 0",
            curve: "log_ratio_cap(|net|, ref=$2M, cap=20)",
            harm: "Dealers hedge with the move — vol expansion / stop-run risk vs directional options.",
            applies_to: "All positions; worse for long premium / fragile structures",
        },
        DriverSpec {
            code: "pos_gex_regime",
            family: "gamma_regime",
            label: "Positive dealer gamma (dampening)",
            max_points: 5.0,
            metric: "net_dealer_gex when net_gex > 0",
            curve: "2 + log_saturate(net, scale=$5M, cap=3) ≤ 5",
            harm: "Lower path risk, but residual pin/complacency risk near magnets.",
            applies_to: "All positions (small residual only)",
        },
        // --- flip_proximity ---
        DriverSpec {
            code: "zero_gamma_flip",
            family: "flip_proximity",
            label: "Near dealer zero-gamma flip",
            max_points: MAX_FLIP_PROXIMITY,
            metric: "|spot - gamma_flip| / spot * 100",
            curve: "sigmoid_proximity(dist%, mid=0.38%, k=8.5, cap=18) — tight; ≤0.15%≈max, 0.5%≈5–6",
            harm: "Crossing the flip can flip dealer hedging from dampening to amplifying (or vice versa).",
            applies_to: "All",
        },
        // --- wall_conflict ---
        DriverSpec {
            code: "call_wall_headwind",
            family: "wall_conflict",
            label: "Call wall headwind",
            max_points: 12.0,
            metric: "|strike - call_wall| / spot",
            curve: "exp_decay_near(dist%, half_life=0.7%, cap=12); family cap 14",
            harm: "Dealer call wall can pin/cap upside into your long calls.",
            applies_to: "Long calls (primary); shorts near wall via magnet driver",
        },
        DriverSpec {
            code: "put_wall_magnet",
            family: "wall_conflict",
            label: "Put wall magnet / support",
            max_points: 10.0,
            metric: "|strike - put_wall| / spot",
            curve: "exp_decay_near(dist%, half_life=0.7%, cap=10); family cap 14",
            harm: "Put wall can magnetize spot or fail as support — path risk vs puts/calls.",
            applies_to: "Puts and structures near put wall",
        },
        DriverSpec {
            code: "gamma_magnet_pull",
            family: "wall_conflict",
            label: "Gamma magnet pull",
            max_points: 8.0,
            metric: "|spot - gamma_magnet| / spot",
            curve: "exp_decay_near(dist%, half_life=0.5%, cap=8); family cap 14",
            harm: "Spot may pin toward magnet — bad for lottery OTM, mixed for shorts.",
            applies_to: "Near-magnet structures",
        },
        // --- adverse_flow ---
        DriverSpec {
            code: "adverse_flow",
            family: "adverse_flow",
            label: "Adverse options flow",
            max_points: MAX_ADVERSE_FLOW,
            metric: "net aggressive ask-side premium against position side ($)",
            curve: "log_saturate(net_adverse, scale=$250k, cap=15)",
            harm: "Tape leaning against you (puts vs long calls, calls vs long puts, etc.).",
            applies_to: "All positions with opposing aggressive flow",
        },
        // --- iv_regime ---
        DriverSpec {
            code: "iv_crush_long",
            family: "iv_regime",
            label: "High IV Rank crush risk (long premium)",
            max_points: MAX_IV_REGIME,
            metric: "IV Rank % + 7d/30d term ratio + 25Δ risk reversal; side=long",
            curve: "sigmoid(IVR) + log(term backwardation) + log(|adverse skew|); earnings≤7d ×1.25",
            harm: "Buying rich vol — peak premium into event; front-month inversion and call/put skew tax amplify crush.",
            applies_to: "Long calls/puts (debit premium)",
        },
        DriverSpec {
            code: "iv_expansion_short",
            family: "iv_regime",
            label: "Low IV Rank expansion risk (short premium)",
            max_points: MAX_IV_REGIME,
            metric: "IV Rank % (1y) + skew; side=short",
            curve: "sigmoid_proximity(iv_rank, mid=35, k=0.08, cap=13) — high when IVR is low",
            harm: "Shorting cheap vol — room for IV expansion / vol-of-vol to blow out short vega.",
            applies_to: "Short calls/puts (credit premium)",
        },
        DriverSpec {
            code: "dark_pool_level",
            family: "wall_conflict",
            label: "Dark pool level magnet / overhead",
            max_points: 6.0,
            metric: "|spot - top nearby dark_pool price_level| / spot",
            curve: "exp_decay_near(dist%, half_life=0.6%, cap=6); long-call overhead boost",
            harm: "Institutional block prints act as structural magnets/ceilings — similar to dealer walls.",
            applies_to: "All; especially long calls under DP supply clusters",
        },
        DriverSpec {
            code: "liquidity_friction",
            family: "structure",
            label: "Contract liquidity / exit friction",
            max_points: 3.5,
            metric: "NBBO spread % of mid + thin open interest/volume",
            curve: "log_saturate(spread%) + thin-OI add; within structure family",
            harm: "Wide spreads / thin OI trap exits when vol spikes — microstructure friction ≠ GEX risk.",
            applies_to: "Any listed option contract",
        },
        // --- structure ---
        DriverSpec {
            code: "short_dte",
            family: "structure",
            label: "Short-dated structure risk",
            max_points: 6.0,
            metric: "calendar days to expiry",
            curve: "sigmoid_proximity(dte, mid=8d, k=0.35, cap=6)",
            harm: "Gamma/charm explode into expiry — small spot moves dominate P&L.",
            applies_to: "Any short-dated option",
        },
        DriverSpec {
            code: "otm_lottery",
            family: "structure",
            label: "Out-of-the-money structure",
            max_points: 2.0,
            metric: "OTM % = |strike-spot|/spot for long direction",
            curve: "log_saturate(otm% * 100, scale=6, cap=2)",
            harm: "Needs a larger move; walls/flip matter more; higher lottery wipe risk.",
            applies_to: "OTM longs",
        },
        DriverSpec {
            code: "high_gamma_cluster",
            family: "structure",
            label: "High gamma into short DTE",
            max_points: 1.5,
            metric: "|position_gamma| and dte ≤ 10",
            curve: "flat +1.5 when gamma≥0.02 and dte≤10 (within structure family)",
            harm: "P&L curvature spikes — microstructure noise becomes position-threatening faster.",
            applies_to: "High-gamma weeklies",
        },
        // --- catalyst ---
        DriverSpec {
            code: "earnings_catalyst",
            family: "catalyst",
            label: "Earnings / binary catalyst",
            max_points: MAX_CATALYST,
            metric: "days until next_earnings_date",
            curve: "sigmoid_proximity(days, mid=7d, k=0.4, cap=12)",
            harm: "Gap/IV crush can move spot through your strike faster than continuous hedging implies.",
            applies_to: "Positions held into earnings window",
        },
        // --- reserved ---
        DriverSpec {
            code: "max_loss_zone",
            family: "wall_conflict",
            label: "Proximity to structural max-loss zone",
            max_points: 0.0,
            metric: "distance of spot to strike-path max-loss boundary",
            curve: "planned: sigmoid_proximity to breakeven/max-loss level",
            harm: "Spot approaching the region where the option is structurally worthless / max loss.",
            applies_to: "All (reserved)",
        },
        DriverSpec {
            code: "flow_vs_14d_baseline",
            family: "adverse_flow",
            label: "Flow divergence vs 14-day baseline",
            max_points: 0.0,
            metric: "today adverse premium / 14d rolling mean",
            curve: "planned: log_saturate(z-score or ratio)",
            harm: "Unusual against-you flow relative to that ticker's own history (noise filter).",
            applies_to: "All (reserved)",
        },
    ]
}

pub fn family_max(family: &str) -> f64 {
    match family {
        "gamma_regime" => MAX_GAMMA_REGIME,
        "flip_proximity" => MAX_FLIP_PROXIMITY,
        "wall_conflict" => MAX_WALL_CONFLICT,
        "adverse_flow" => MAX_ADVERSE_FLOW,
        "iv_regime" => MAX_IV_REGIME,
        "structure" => MAX_STRUCTURE,
        "catalyst" => MAX_CATALYST,
        _ => 0.0,
    }
}

pub fn driver_max(code: &str) -> f64 {
    driver_catalog()
        .into_iter()
        .find(|d| d.code == code)
        .map(|d| d.max_points)
        .unwrap_or(0.0)
}
