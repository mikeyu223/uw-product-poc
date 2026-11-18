use anyhow::{Context, Result};
use risk_watcher::api::{self, AppState};
use risk_watcher::config::Config;
use risk_watcher::journal::JournalStore;
use risk_watcher::uw::UwClient;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let config = Config::from_env()?;
    let uw = UwClient::new(&config.uw_base_url, &config.uw_api_key)
        .context("failed to build Unusual Whales client")?;

    // `cargo run -- --probe ESZ6` — futures CVD smoke check without HTTP.
    if let Some(contract) = parse_probe_contract(&args) {
        info!(%contract, "probing futures CVD");
        match uw.futures_trades(&contract, 100).await {
            Ok(trades) => {
                let buy: i64 = trades
                    .iter()
                    .filter(|t| t.side.eq_ignore_ascii_case("buy"))
                    .map(|t| t.size)
                    .sum();
                let sell: i64 = trades
                    .iter()
                    .filter(|t| t.side.eq_ignore_ascii_case("sell"))
                    .map(|t| t.size)
                    .sum();
                println!(
                    "futures probe {contract}: trades={} buy={} sell={} net_delta={:+}",
                    trades.len(),
                    buy,
                    sell,
                    buy - sell
                );
                return Ok(());
            }
            Err(err) => {
                error!(error = %err, "futures probe failed");
                anyhow::bail!(err);
            }
        }
    }

    let state = AppState {
        config: config.clone(),
        uw,
        journal: JournalStore::new(),
    };

    let app = api::router(state);
    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .with_context(|| format!("bind {}", config.bind_addr))?;

    info!(addr = %config.bind_addr, "futures-cvd listening");
    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("risk_watcher=info,tower_http=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

fn parse_probe_contract(args: &[String]) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--probe" {
            return iter.next().cloned().or(Some("ESZ6".to_string()));
        }
        if let Some(c) = arg.strip_prefix("--probe=") {
            return Some(c.to_string());
        }
    }
    None
}
