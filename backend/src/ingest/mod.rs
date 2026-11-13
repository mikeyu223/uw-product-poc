//! Simulated Kafka-like event consumer for tick / GEX updates.
//!
//! Production will consume Unusual Whales Kafka (or websocket) streams.
//! For the PoC we run an in-process loop that polls REST snapshots and
//! publishes [`MarketEvent`]s onto an async channel.

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MarketEvent {
    Tick {
        ticker: String,
        price: f64,
        ts: String,
    },
    GexUpdate {
        ticker: String,
        gamma_flip: Option<f64>,
        call_wall: Option<f64>,
        put_wall: Option<f64>,
        ts: String,
    },
    FlowAlert {
        ticker: String,
        summary: String,
        ts: String,
    },
}

/// Spawn a stub consumer that emits heartbeat ticks for the given symbols.
/// Replace the body with real UW Kafka / WS ingest later.
pub fn spawn_simulated_consumer(
    tickers: Vec<String>,
    tx: mpsc::Sender<MarketEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!(count = tickers.len(), "starting simulated ingest loop");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            for ticker in &tickers {
                let event = MarketEvent::Tick {
                    ticker: ticker.clone(),
                    price: 0.0,
                    ts: chrono::Utc::now().to_rfc3339(),
                };
                debug!(?event, "simulated event");
                if tx.send(event).await.is_err() {
                    info!("ingest channel closed — stopping simulated consumer");
                    return;
                }
            }
        }
    })
}
