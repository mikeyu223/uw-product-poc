//! Risk Watcher backend library.
//!
//! Modules:
//! - [`uw`] — Unusual Whales REST client
//! - [`risk`] — composite risk scoring & explanation (V1)
//! - [`ingest`] — simulated Kafka-like tick / GEX consumer
//! - [`api`] — Axum HTTP routes for the dashboard

pub mod api;
pub mod cache;
pub mod config;
pub mod ingest;
pub mod risk;
pub mod uw;

pub use cache::BookCache;
pub use config::Config;
