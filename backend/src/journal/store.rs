use super::enrich::demo_flow_context;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepMarker {
    pub t: String,
    pub premium: f64,
    pub side: String,
    pub mins_before_entry: i64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowContext {
    pub alignment: String,
    pub badge: String,
    pub net_call_prem: f64,
    pub net_put_prem: f64,
    pub nearest_sweep_prem: Option<f64>,
    pub nearest_sweep_mins: Option<i64>,
    pub nearest_sweep_side: Option<String>,
    pub gex_regime: String,
    pub gex_label: String,
    pub net_gamma: Option<f64>,
    pub major_sweeps: Vec<SweepMarker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeEntry {
    pub id: String,
    pub account: String,
    pub symbol: String,
    pub asset_class: String,
    pub side: String,
    pub qty: f64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub entry_at: String,
    pub exit_at: String,
    pub commissions: f64,
    pub pnl: f64,
    pub setup: String,
    pub notes: String,
    pub emotion: String,
    pub tags: Vec<String>,
    /// Options-first fields
    pub dte_at_entry: Option<i32>,
    pub iv_rank_at_entry: Option<f64>,
    /// single_leg | vertical | iron_condor | futures | stock
    pub contract_strategy: String,
    pub plan: Option<String>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub risk_amount: Option<f64>,
    pub planned_rr: Option<f64>,
    pub followed_rules: Option<bool>,
    pub lesson: Option<String>,
    pub flow: Option<FlowContext>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewTrade {
    pub account: Option<String>,
    pub symbol: String,
    pub asset_class: Option<String>,
    pub side: String,
    pub qty: f64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub entry_at: Option<String>,
    pub exit_at: Option<String>,
    pub commissions: Option<f64>,
    pub setup: Option<String>,
    pub notes: Option<String>,
    pub emotion: Option<String>,
    pub tags: Option<Vec<String>>,
    pub dte_at_entry: Option<i32>,
    pub iv_rank_at_entry: Option<f64>,
    pub contract_strategy: Option<String>,
    pub plan: Option<String>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub risk_amount: Option<f64>,
    pub planned_rr: Option<f64>,
    pub followed_rules: Option<bool>,
    pub lesson: Option<String>,
}

#[derive(Clone, Default)]
pub struct JournalStore {
    inner: Arc<RwLock<Vec<TradeEntry>>>,
}

impl JournalStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(sample_trades())),
        }
    }

    pub async fn list(&self) -> Vec<TradeEntry> {
        self.inner.read().await.clone()
    }

    pub async fn get(&self, id: &str) -> Option<TradeEntry> {
        self.inner.read().await.iter().find(|t| t.id == id).cloned()
    }

    pub async fn add(&self, trade: TradeEntry) {
        self.inner.write().await.insert(0, trade);
    }

    pub async fn replace(&self, trade: TradeEntry) {
        let mut guard = self.inner.write().await;
        if let Some(idx) = guard.iter().position(|t| t.id == trade.id) {
            guard[idx] = trade;
        } else {
            guard.insert(0, trade);
        }
    }

    pub async fn update_notes(
        &self,
        id: &str,
        notes: Option<String>,
        emotion: Option<String>,
        setup: Option<String>,
        tags: Option<Vec<String>>,
        lesson: Option<String>,
        followed_rules: Option<bool>,
    ) -> Option<TradeEntry> {
        let mut guard = self.inner.write().await;
        let t = guard.iter_mut().find(|t| t.id == id)?;
        if let Some(n) = notes {
            t.notes = n;
        }
        if let Some(e) = emotion {
            t.emotion = e;
        }
        if let Some(s) = setup {
            t.setup = s;
        }
        if let Some(tags) = tags {
            t.tags = tags;
        }
        if let Some(l) = lesson {
            t.lesson = Some(l);
        }
        if let Some(f) = followed_rules {
            t.followed_rules = Some(f);
        }
        Some(t.clone())
    }

    pub async fn import_csv(&self, csv: &str) -> Result<Vec<TradeEntry>, String> {
        let mut imported = Vec::new();
        let mut lines = csv.lines();
        let _header = lines
            .next()
            .ok_or_else(|| "empty CSV".to_string())?
            .to_ascii_lowercase();
        for (i, line) in lines.enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let cols: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if cols.len() < 5 {
                return Err(format!("row {}: need at least symbol,side,qty,entry,exit", i + 2));
            }
            let symbol = cols[0].to_ascii_uppercase();
            let side = cols[1].to_ascii_lowercase();
            let qty: f64 = cols[2]
                .parse()
                .map_err(|_| format!("row {}: bad qty", i + 2))?;
            let entry_price: f64 = cols[3]
                .parse()
                .map_err(|_| format!("row {}: bad entry", i + 2))?;
            let exit_price: f64 = cols[4]
                .parse()
                .map_err(|_| format!("row {}: bad exit", i + 2))?;
            let entry_at = cols
                .get(5)
                .copied()
                .filter(|s| !s.is_empty())
                .unwrap_or("2026-11-17T14:35:00Z")
                .to_string();
            let exit_at = cols
                .get(6)
                .copied()
                .filter(|s| !s.is_empty())
                .unwrap_or("2026-11-17T15:10:00Z")
                .to_string();
            let commissions: f64 = cols.get(7).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let setup = cols.get(8).unwrap_or(&"Imported").to_string();
            let account = cols.get(9).unwrap_or(&"CSV Import").to_string();
            let notes = cols.get(10).unwrap_or(&"").to_string();
            let dte: Option<i32> = cols.get(11).and_then(|s| s.parse().ok());
            let strategy = cols
                .get(12)
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| default_strategy(&symbol, &setup));
            let pnl = signed_pnl(&side, qty, entry_price, exit_price) - commissions;
            let trade = TradeEntry {
                id: Uuid::new_v4().to_string(),
                account,
                symbol: symbol.clone(),
                asset_class: guess_asset(&symbol),
                side,
                qty,
                entry_price,
                exit_price,
                entry_at,
                exit_at,
                commissions,
                pnl,
                setup: setup.clone(),
                notes,
                emotion: String::new(),
                tags: vec!["imported".into()],
                dte_at_entry: dte.or_else(|| infer_dte_from_setup(&setup)),
                iv_rank_at_entry: None,
                contract_strategy: strategy,
                plan: None,
                stop_loss: None,
                take_profit: None,
                risk_amount: None,
                planned_rr: None,
                followed_rules: None,
                lesson: None,
                flow: None,
            };
            imported.push(trade);
        }
        Ok(imported)
    }

    pub async fn add_many(&self, trades: Vec<TradeEntry>) {
        let mut guard = self.inner.write().await;
        for t in trades.into_iter().rev() {
            guard.insert(0, t);
        }
    }
}

pub fn from_new(n: NewTrade) -> TradeEntry {
    let side = n.side.to_ascii_lowercase();
    let commissions = n.commissions.unwrap_or(0.0);
    let pnl = signed_pnl(&side, n.qty, n.entry_price, n.exit_price) - commissions;
    let symbol = n.symbol.to_ascii_uppercase();
    let setup = n.setup.clone().unwrap_or_else(|| "Untagged".into());
    TradeEntry {
        id: Uuid::new_v4().to_string(),
        account: n.account.unwrap_or_else(|| "Demo".into()),
        symbol: symbol.clone(),
        asset_class: n.asset_class.unwrap_or_else(|| guess_asset(&symbol)),
        side,
        qty: n.qty,
        entry_price: n.entry_price,
        exit_price: n.exit_price,
        entry_at: n.entry_at.unwrap_or_else(|| Utc::now().to_rfc3339()),
        exit_at: n.exit_at.unwrap_or_else(|| Utc::now().to_rfc3339()),
        commissions,
        pnl,
        setup: setup.clone(),
        notes: n.notes.unwrap_or_default(),
        emotion: n.emotion.unwrap_or_default(),
        tags: n.tags.unwrap_or_default(),
        dte_at_entry: n.dte_at_entry.or_else(|| infer_dte_from_setup(&setup)),
        iv_rank_at_entry: n.iv_rank_at_entry,
        contract_strategy: n
            .contract_strategy
            .unwrap_or_else(|| default_strategy(&symbol, &setup)),
        plan: n.plan,
        stop_loss: n.stop_loss,
        take_profit: n.take_profit,
        risk_amount: n.risk_amount,
        planned_rr: n.planned_rr,
        followed_rules: n.followed_rules,
        lesson: n.lesson,
        flow: None,
    }
}

fn signed_pnl(side: &str, qty: f64, entry: f64, exit: f64) -> f64 {
    let mult = if side == "short" || side == "sell" {
        -1.0
    } else {
        1.0
    };
    let contract_mult = if entry < 500.0 && exit < 500.0 {
        100.0
    } else {
        1.0
    };
    mult * (exit - entry) * qty * contract_mult
}

fn guess_asset(symbol: &str) -> String {
    let s = symbol.to_ascii_uppercase();
    if s.starts_with("ES") || s.starts_with("NQ") || s.starts_with("/ES") || s.starts_with("/NQ") {
        "futures".into()
    } else {
        "options".into()
    }
}

fn default_strategy(symbol: &str, setup: &str) -> String {
    if guess_asset(symbol) == "futures" {
        return "futures".into();
    }
    let s = setup.to_ascii_lowercase();
    if s.contains("iron") {
        "iron_condor".into()
    } else if s.contains("vertical") || s.contains("spread") {
        "vertical".into()
    } else {
        "single_leg".into()
    }
}

fn infer_dte_from_setup(setup: &str) -> Option<i32> {
    let s = setup.to_ascii_lowercase();
    if s.contains("0dte") {
        Some(0)
    } else if s.contains("earnings") {
        Some(5)
    } else {
        Some(1)
    }
}

fn sample_trades() -> Vec<TradeEntry> {
    let specs = [
        (
            "SPY",
            "long",
            4.0,
            2.15,
            3.40,
            "2026-11-17T14:35:00Z",
            "2026-11-17T15:05:00Z",
            "0DTE Gamma Squeeze",
            "Demo-ToS",
            "Caught opening sweep cluster",
            "focused",
            Some(0_i32),
            Some(62.0_f64),
            "single_leg",
            Some(true),
            "Only take 0DTE when flow aligns within 10 mins",
        ),
        (
            "QQQ",
            "long",
            3.0,
            1.80,
            1.10,
            "2026-11-17T16:10:00Z",
            "2026-11-17T17:40:00Z",
            "Opening Range Breakout",
            "Demo-ToS",
            "Chased mid-day — cut",
            "tilted",
            Some(0),
            Some(55.0),
            "single_leg",
            Some(false),
            "No new risk after 11:30 if first trade was a loser",
        ),
        (
            "NVDA",
            "short",
            2.0,
            8.50,
            5.20,
            "2026-11-14T15:00:00Z",
            "2026-11-14T20:30:00Z",
            "Earnings IV Crush",
            "Demo-IBKR",
            "Sold rich front IV into print",
            "calm",
            Some(3),
            Some(88.0),
            "vertical",
            Some(true),
            "IV rank > 80 into earnings = size up credit",
        ),
        (
            "ESZ6",
            "long",
            1.0,
            7420.0,
            7448.0,
            "2026-11-17T14:32:00Z",
            "2026-11-17T15:15:00Z",
            "Opening Range Breakout",
            "Demo-Tradovate",
            "CVD flip + OR break",
            "focused",
            None,
            None,
            "futures",
            Some(true),
            "Wait for CVD confirmation on OR break",
        ),
        (
            "SPY",
            "long",
            5.0,
            1.25,
            0.55,
            "2026-11-13T17:00:00Z",
            "2026-11-13T18:20:00Z",
            "0DTE Gamma Squeeze",
            "Demo-ToS",
            "Lunch chop — should skip",
            "bored",
            Some(0),
            Some(48.0),
            "single_leg",
            Some(false),
            "Hard rule: flat 11:30–13:00",
        ),
        (
            "META",
            "long",
            2.0,
            4.10,
            6.80,
            "2026-11-12T14:45:00Z",
            "2026-11-12T19:10:00Z",
            "Earnings IV Crush",
            "Demo-IBKR",
            "Held through crush correctly",
            "patient",
            Some(2),
            Some(79.0),
            "vertical",
            Some(true),
            "Hold credit through open when GEX positive",
        ),
        (
            "NQZ6",
            "short",
            1.0,
            27450.0,
            27390.0,
            "2026-11-11T15:00:00Z",
            "2026-11-11T16:30:00Z",
            "Opening Range Breakout",
            "Demo-Tradovate",
            "Failed OR long → flipped",
            "adaptive",
            None,
            None,
            "futures",
            Some(true),
            "Flip only once; no third cut",
        ),
        (
            "IWM",
            "long",
            6.0,
            0.95,
            1.55,
            "2026-11-10T14:40:00Z",
            "2026-11-10T15:55:00Z",
            "0DTE Gamma Squeeze",
            "Demo-ToS",
            "Clean open drive",
            "confident",
            Some(0),
            Some(41.0),
            "single_leg",
            Some(true),
            "Size up only on open + sweep align",
        ),
    ];

    specs
        .into_iter()
        .enumerate()
        .map(
            |(
                i,
                (
                    sym,
                    side,
                    qty,
                    entry,
                    exit,
                    ea,
                    xa,
                    setup,
                    acct,
                    notes,
                    emotion,
                    dte,
                    ivr,
                    strat,
                    followed,
                    lesson,
                ),
            )| {
                let commissions = 2.5 + i as f64;
                let mut pnl = signed_pnl(side, qty, entry, exit) - commissions;
                if sym.starts_with("ES") {
                    pnl = (exit - entry) * qty * 50.0 - commissions;
                } else if sym.starts_with("NQ") {
                    let dir = if side == "short" { -1.0 } else { 1.0 };
                    pnl = dir * (exit - entry) * qty * 20.0 - commissions;
                }
                let mut trade = TradeEntry {
                    id: format!("demo-{i}"),
                    account: acct.into(),
                    symbol: sym.into(),
                    asset_class: guess_asset(sym),
                    side: side.into(),
                    qty,
                    entry_price: entry,
                    exit_price: exit,
                    entry_at: ea.into(),
                    exit_at: xa.into(),
                    commissions,
                    pnl,
                    setup: setup.into(),
                    notes: notes.into(),
                    emotion: emotion.into(),
                    tags: vec![setup.split_whitespace().next().unwrap_or("setup").into()],
                    dte_at_entry: dte,
                    iv_rank_at_entry: ivr,
                    contract_strategy: strat.into(),
                    plan: Some(format!("Playbook: {setup}")),
                    stop_loss: Some(entry * 0.55),
                    take_profit: Some(entry * 1.8),
                    risk_amount: Some((entry * 0.45 * qty * 100.0).abs()),
                    planned_rr: Some(2.0),
                    followed_rules: followed,
                    lesson: Some(lesson.into()),
                    flow: None,
                };
                trade.flow = Some(demo_flow_context(&trade, i));
                trade
            },
        )
        .collect()
}
