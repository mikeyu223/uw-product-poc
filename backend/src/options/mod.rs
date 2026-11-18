//! Options-flow product surfaces that sit on data UW already collects.
//!
//! - [`earnings_iv`] — Earnings IV crush visualizer (term structure + IV rank)
//! - [`sweep_clusters`] — Enhanced 0DTE sweep cluster filter on live flow alerts

pub mod earnings_iv;
pub mod sweep_clusters;

pub use earnings_iv::{EarningsIvReport, build_earnings_iv};
pub use sweep_clusters::{SweepClusterReport, build_sweep_clusters};
