//! Unusual Whales product PoCs:
//! - Futures live tape + CVD + market structure (POC/VAH/VAL, Globex/RTH, OI)
//! - Earnings IV crush visualizer
//! - 0DTE sweep cluster filter
//! - Trading Journal (TradeZella / Tradervue-style retention loop)

pub mod api;
pub mod config;
pub mod cvd;
pub mod journal;
pub mod options;
pub mod structure;
pub mod uw;

pub use config::Config;
