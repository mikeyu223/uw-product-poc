//! Non-linear scoring curves.
//!
//! Hard step ladders ($2M → 20 pts, $15M still 20) saturate too early and
//! jump discontinuously. These helpers map raw metrics through smooth curves
//! that ramp quickly in the "normal" zone then asymptote toward a point cap.

/// Logarithmic saturation for positive magnitudes (flow $, |GEX|, etc.).
///
/// `scale` is the characteristic magnitude where the curve is ~63% of `cap`
/// (because 1 - e^{-1} ≈ 0.63). Larger outliers approach `cap` but never explode.
///
/// Equivalent form: `cap * (1 - exp(-ln(1+x/scale)))` smoothed via ln1p ratio.
pub fn log_saturate(x: f64, scale: f64, cap: f64) -> f64 {
    if x <= 0.0 || scale <= 0.0 || cap <= 0.0 {
        return 0.0;
    }
    let t = (1.0 + x / scale).ln() / (1.0 + 1.0_f64).ln(); // normalize so x=scale → 1.0 before soft clamp
    // Soft approach to cap: 1 - e^{-t} keeps diminishing returns past scale.
    let unit = 1.0 - (-t).exp();
    (cap * unit).clamp(0.0, cap)
}

/// Alternate log map: fraction of `cap` from `ln(1+x) / ln(1+ref)` capped at 1.
/// Good when you want `x == ref_at_full` to nearly hit the ceiling.
pub fn log_ratio_cap(x: f64, ref_at_full: f64, cap: f64) -> f64 {
    if x <= 0.0 || ref_at_full <= 0.0 || cap <= 0.0 {
        return 0.0;
    }
    let unit = ((1.0 + x).ln() / (1.0 + ref_at_full).ln()).clamp(0.0, 1.0);
    cap * unit
}

/// Sigmoid proximity: high score when `distance` is small.
///
/// `mid` = distance where score ≈ 50% of cap.
/// `k` = steepness (larger → sharper cliff).
pub fn sigmoid_proximity(distance: f64, mid: f64, k: f64, cap: f64) -> f64 {
    if cap <= 0.0 {
        return 0.0;
    }
    let d = distance.max(0.0);
    let m = mid.max(1e-9);
    let unit = 1.0 / (1.0 + (k * (d - m)).exp());
    (cap * unit).clamp(0.0, cap)
}

/// Exponential decay with distance (walls): full risk at dist=0, fades toward 0.
pub fn exp_decay_near(distance: f64, half_life: f64, cap: f64) -> f64 {
    if cap <= 0.0 || half_life <= 0.0 {
        return 0.0;
    }
    let d = distance.max(0.0);
    // At d=half_life → ~50% of cap
    let unit = (-d * std::f64::consts::LN_2 / half_life).exp();
    (cap * unit).clamp(0.0, cap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_flow_ramps_then_flattens() {
        let cap = 20.0;
        let scale = 250_000.0;
        let a = log_saturate(100_000.0, scale, cap);
        let b = log_saturate(1_000_000.0, scale, cap);
        let c = log_saturate(10_000_000.0, scale, cap);
        let d = log_saturate(20_000_000.0, scale, cap);
        assert!(a < b && b < c);
        assert!(c < cap);
        // Diminishing: 10M→20M adds less than 100k→1M
        assert!((d - c) < (b - a));
    }

    #[test]
    fn sigmoid_closer_is_riskier() {
        let near = sigmoid_proximity(0.1, 0.5, 6.0, 20.0);
        let far = sigmoid_proximity(2.0, 0.5, 6.0, 20.0);
        assert!(near > far);
    }

    #[test]
    fn flip_curve_is_tight_not_hot() {
        // Production flip calibration: mid=0.38%, k=8.5, cap=20
        let on_flip = sigmoid_proximity(0.10, 0.38, 8.5, 20.0);
        let near = sigmoid_proximity(0.15, 0.38, 8.5, 20.0);
        let half_pct = sigmoid_proximity(0.50, 0.38, 8.5, 20.0);
        let calm = sigmoid_proximity(1.00, 0.38, 8.5, 20.0);
        assert!(on_flip >= 17.0, "on-flip should be near max, got {on_flip}");
        assert!(near >= 16.0, "≤0.15% should stay hot, got {near}");
        assert!(half_pct >= 4.5 && half_pct <= 7.0, "0.5% should be modest 5–6, got {half_pct}");
        assert!(calm < 3.0, "1% away should be noise, got {calm}");
        // Old hot curve (mid=0.55, k=5) would score 0.36% around 14+; new must be cooler
        let old_trigger_zone = sigmoid_proximity(0.36, 0.38, 8.5, 20.0);
        assert!(old_trigger_zone < 12.0, "0.36% must not cry wolf, got {old_trigger_zone}");
    }
}
