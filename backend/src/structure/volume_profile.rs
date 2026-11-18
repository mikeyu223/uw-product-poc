//! Volume Point of Control + Value Area (Auction Market Theory).
//!
//! POC = price with the most traded volume.
//! Value Area ≈ contiguous range around POC covering ~70% of volume → VAH / VAL.

use crate::uw::FuturesTrade;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub struct ProfileLevel {
    pub price: f64,
    pub volume: i64,
    pub buy_volume: i64,
    pub sell_volume: i64,
    pub in_value_area: bool,
    pub is_poc: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VolumeProfile {
    pub tick_size: f64,
    pub poc: f64,
    pub vah: f64,
    pub val: f64,
    pub total_volume: i64,
    pub value_area_pct: f64,
    pub levels: Vec<ProfileLevel>,
    pub methodology: String,
}

pub fn build_volume_profile(trades: &[FuturesTrade], tick_size: f64) -> VolumeProfile {
    let tick = if tick_size > 0.0 { tick_size } else { 0.25 };
    let mut buckets: BTreeMap<i64, (i64, i64, i64)> = BTreeMap::new(); // buy, sell, total

    for t in trades {
        let price = t.price_f64().unwrap_or(0.0);
        if price <= 0.0 {
            continue;
        }
        let key = (price / tick).round() as i64;
        let size = t.size.max(0);
        let e = buckets.entry(key).or_insert((0, 0, 0));
        e.2 += size;
        if t.side.eq_ignore_ascii_case("buy") {
            e.0 += size;
        } else if t.side.eq_ignore_ascii_case("sell") {
            e.1 += size;
        }
    }

    let total_volume: i64 = buckets.values().map(|(_, _, v)| *v).sum();
    if buckets.is_empty() || total_volume == 0 {
        return VolumeProfile {
            tick_size: tick,
            poc: 0.0,
            vah: 0.0,
            val: 0.0,
            total_volume: 0,
            value_area_pct: 70.0,
            levels: vec![],
            methodology: amt_method(),
        };
    }

    let (poc_key, _) = buckets
        .iter()
        .max_by_key(|(_, (_, _, v))| *v)
        .map(|(k, v)| (*k, *v))
        .unwrap();
    let poc = poc_key as f64 * tick;

    // Expand value area from POC until ≥70% of volume.
    let keys: Vec<i64> = buckets.keys().copied().collect();
    let poc_idx = keys.iter().position(|k| *k == poc_key).unwrap_or(0);
    let target = (total_volume as f64 * 0.70).ceil() as i64;
    let mut covered = buckets.get(&poc_key).map(|(_, _, v)| *v).unwrap_or(0);
    let mut lo = poc_idx;
    let mut hi = poc_idx;
    while covered < target && (lo > 0 || hi + 1 < keys.len()) {
        let next_lo = if lo > 0 {
            buckets.get(&keys[lo - 1]).map(|(_, _, v)| *v).unwrap_or(0)
        } else {
            -1
        };
        let next_hi = if hi + 1 < keys.len() {
            buckets.get(&keys[hi + 1]).map(|(_, _, v)| *v).unwrap_or(0)
        } else {
            -1
        };
        if next_hi >= next_lo {
            hi += 1;
            covered += next_hi.max(0);
        } else {
            lo -= 1;
            covered += next_lo.max(0);
        }
    }

    let val = keys[lo] as f64 * tick;
    let vah = keys[hi] as f64 * tick;
    let in_va: std::collections::HashSet<i64> = keys[lo..=hi].iter().copied().collect();

    let levels: Vec<ProfileLevel> = buckets
        .into_iter()
        .rev()
        .map(|(k, (buy, sell, vol))| ProfileLevel {
            price: k as f64 * tick,
            volume: vol,
            buy_volume: buy,
            sell_volume: sell,
            in_value_area: in_va.contains(&k),
            is_poc: k == poc_key,
        })
        .collect();

    VolumeProfile {
        tick_size: tick,
        poc,
        vah,
        val,
        total_volume,
        value_area_pct: 70.0,
        levels,
        methodology: amt_method(),
    }
}

fn amt_method() -> String {
    "Auction Market Theory volume profile from time & sales. POC = highest-volume \
price. Value Area expands from POC until ~70% of volume is enclosed → VAH / VAL. \
Same levels Sierra Chart / Bookmap traders already mark by hand."
        .into()
}
