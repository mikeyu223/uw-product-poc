//! Trading Journal PoC — TradeZella / Tradervue-style retention loop,
//! enriched with Unusual Whales flow + GEX context at fill time.

mod analytics;
mod chart;
mod enrich;
mod store;

pub use analytics::{
    JournalAnalytics, JournalFilter, build_analytics, build_analytics_filtered, filter_trades,
};
pub use chart::{TradeChart, build_trade_chart};
pub use enrich::{enrich_many, enrich_trade};
pub use store::{FlowContext, JournalStore, NewTrade, TradeEntry, from_new};
