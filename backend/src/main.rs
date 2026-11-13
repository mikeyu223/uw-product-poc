use anyhow::{Context, Result};
use risk_watcher::api::{self, AppState};
use risk_watcher::cache::BookCache;
use risk_watcher::config::Config;
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

    // `cargo run -- --probe SPY` — verify API key + endpoints without starting the server.
    if let Some(ticker) = parse_probe_ticker(&args) {
        info!(%ticker, "probing Unusual Whales");
        match uw.probe(&ticker).await {
            Ok(report) => {
                println!("{report}");
                return Ok(());
            }
            Err(err) => {
                error!(error = %err, "probe failed");
                anyhow::bail!(err);
            }
        }
    }

    let state = AppState {
        config: config.clone(),
        uw,
        book_cache: BookCache::new(),
    };

    let app = api::router(state);
    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .with_context(|| format!("bind {}", config.bind_addr))?;

    info!(addr = %config.bind_addr, "risk-watcher listening");
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

fn parse_probe_ticker(args: &[String]) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--probe" {
            return iter.next().cloned().or(Some("SPY".to_string()));
        }
        if let Some(ticker) = arg.strip_prefix("--probe=") {
            return Some(ticker.to_string());
        }
    }
    None
}
