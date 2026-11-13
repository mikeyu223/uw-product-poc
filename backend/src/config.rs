use anyhow::{Context, Result, bail};
use std::env;
use std::path::PathBuf;

/// Runtime configuration loaded from environment / `.env`.
#[derive(Debug, Clone)]
pub struct Config {
    pub uw_api_key: String,
    pub uw_base_url: String,
    pub bind_addr: String,
}

impl Config {
    /// Load dotenv from the repo root (parent of `backend/`) or CWD, then read env vars.
    pub fn from_env() -> Result<Self> {
        load_dotenv();

        let uw_api_key = env::var("UNUSUAL_WHALES_API_KEY")
            .context("UNUSUAL_WHALES_API_KEY is required (see .env.example)")?;

        if uw_api_key.trim().is_empty() || uw_api_key.contains("your-api-key") {
            bail!("UNUSUAL_WHALES_API_KEY looks unset — put a real UUID token in .env");
        }

        let uw_base_url = env::var("UW_BASE_URL")
            .unwrap_or_else(|_| "https://api.unusualwhales.com".to_string());

        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());

        Ok(Self {
            uw_api_key,
            uw_base_url,
            bind_addr,
        })
    }
}

fn load_dotenv() {
    // Prefer repo-root `.env` when running from `backend/`
    let candidates = [
        PathBuf::from("../.env"),
        PathBuf::from(".env"),
        PathBuf::from("backend/../.env"),
    ];

    for path in candidates {
        if path.exists() {
            let _ = dotenvy::from_path(&path);
            return;
        }
    }

    let _ = dotenvy::dotenv();
}
