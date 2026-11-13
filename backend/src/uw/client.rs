use super::error::{UwError, UwResult};
use super::models::{
    FlowAlert, GexLevels, GexSource, GreekExposure, MarketTidePoint, StockState, TickerInfo,
    UwEnvelope,
};
use reqwest::{Client, StatusCode};
use std::time::Duration;
use tracing::{debug, warn};
use url::Url;

/// Async Unusual Whales REST client (reqwest + Tokio).
#[derive(Debug, Clone)]
pub struct UwClient {
    http: Client,
    base_url: Url,
    api_key: String,
}

impl UwClient {
    pub fn new(base_url: &str, api_key: impl Into<String>) -> UwResult<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("risk-watcher-poc/0.1")
            .build()?;

        Ok(Self {
            http,
            base_url: Url::parse(base_url)?,
            api_key: api_key.into(),
        })
    }

    // -- Public endpoint helpers ------------------------------------------------

    /// `GET /api/stock/{ticker}/info`
    pub async fn ticker_info(&self, ticker: &str) -> UwResult<TickerInfo> {
        let path = format!("/api/stock/{}/info", normalize_ticker(ticker));
        let env: UwEnvelope<TickerInfo> = self.get_json(&path, &[]).await?;
        Ok(env.data)
    }

    /// `GET /api/stock/{ticker}/stock-state`
    pub async fn stock_state(&self, ticker: &str) -> UwResult<StockState> {
        let path = format!("/api/stock/{}/stock-state", normalize_ticker(ticker));
        let env: UwEnvelope<StockState> = self.get_json(&path, &[]).await?;
        Ok(env.data)
    }

    /// `GET /api/stock/{ticker}/gex-levels?source=vol|oi`
    pub async fn gex_levels(&self, ticker: &str, source: GexSource) -> UwResult<GexLevels> {
        let path = format!("/api/stock/{}/gex-levels", normalize_ticker(ticker));
        let env: UwEnvelope<GexLevels> = self
            .get_json(&path, &[("source", source.as_str())])
            .await?;
        Ok(env.data)
    }

    /// `GET /api/stock/{ticker}/greek-exposure`
    ///
    /// Returns a time series; newest entries are typically last.
    pub async fn greek_exposure(&self, ticker: &str) -> UwResult<Vec<GreekExposure>> {
        let path = format!("/api/stock/{}/greek-exposure", normalize_ticker(ticker));
        let env: UwEnvelope<Vec<GreekExposure>> = self.get_json(&path, &[]).await?;
        Ok(env.data)
    }

    /// Latest greek-exposure snapshot (last row of the series), if any.
    pub async fn latest_greek_exposure(&self, ticker: &str) -> UwResult<Option<GreekExposure>> {
        Ok(self.greek_exposure(ticker).await?.into_iter().next_back())
    }

    /// `GET /api/stock/{ticker}/flow-alerts`
    pub async fn flow_alerts(&self, ticker: &str, limit: u32) -> UwResult<Vec<FlowAlert>> {
        let path = format!("/api/stock/{}/flow-alerts", normalize_ticker(ticker));
        let limit_s = limit.clamp(1, 500).to_string();
        let env: UwEnvelope<Vec<FlowAlert>> = self
            .get_json(&path, &[("limit", limit_s.as_str())])
            .await?;
        Ok(env.data)
    }

    /// `GET /api/option-trades/flow-alerts` — market-wide flow alerts.
    pub async fn market_flow_alerts(&self, limit: u32) -> UwResult<Vec<FlowAlert>> {
        let limit_s = limit.clamp(1, 500).to_string();
        let env: UwEnvelope<Vec<FlowAlert>> = self
            .get_json("/api/option-trades/flow-alerts", &[("limit", limit_s.as_str())])
            .await?;
        Ok(env.data)
    }

    /// `GET /api/market/market-tide`
    pub async fn market_tide(&self) -> UwResult<Vec<MarketTidePoint>> {
        let env: UwEnvelope<Vec<MarketTidePoint>> =
            self.get_json("/api/market/market-tide", &[]).await?;
        Ok(env.data)
    }

    /// Lightweight connectivity check: fetch ticker info + GEX levels.
    pub async fn probe(&self, ticker: &str) -> UwResult<ProbeReport> {
        let info = self.ticker_info(ticker).await?;
        let gex = self.gex_levels(ticker, GexSource::Vol).await?;
        let greek = self.latest_greek_exposure(ticker).await?;
        let alerts = self.flow_alerts(ticker, 3).await?;

        Ok(ProbeReport {
            ticker: normalize_ticker(ticker),
            info,
            gex,
            latest_greek: greek,
            recent_alerts: alerts,
        })
    }

    // -- Internals --------------------------------------------------------------

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> UwResult<T> {
        let mut url = self.base_url.clone();
        url.set_path(path);
        url.set_query(None);
        {
            let mut pairs = url.query_pairs_mut();
            for (k, v) in query {
                pairs.append_pair(k, v);
            }
        }

        debug!(%url, "UW GET");

        let response = self
            .http
            .get(url.clone())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "application/json")
            .send()
            .await?;

        let status = response.status();

        // Respect Retry-After on 503 (documented for GEX endpoints).
        if status == StatusCode::SERVICE_UNAVAILABLE {
            if let Some(ra) = response.headers().get("retry-after") {
                warn!(retry_after = ?ra, "UW 503 — caller should retry");
            }
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(UwError::Api {
                status: status.as_u16(),
                body,
            });
        }

        let body = response.bytes().await?;
        Ok(serde_json::from_slice(&body)?)
    }
}

fn normalize_ticker(ticker: &str) -> String {
    ticker.trim().to_uppercase()
}

/// Bundle returned by [`UwClient::probe`].
#[derive(Debug, Clone)]
pub struct ProbeReport {
    pub ticker: String,
    pub info: TickerInfo,
    pub gex: GexLevels,
    pub latest_greek: Option<GreekExposure>,
    pub recent_alerts: Vec<FlowAlert>,
}

impl std::fmt::Display for ProbeReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Unusual Whales probe — {}", self.ticker)?;
        writeln!(
            f,
            "  name: {}",
            self.info
                .full_name
                .as_deref()
                .or(self.info.short_name.as_deref())
                .unwrap_or("(unknown)")
        )?;
        writeln!(
            f,
            "  GEX flip={} call_wall={} put_wall={} magnet={}",
            self.gex.gamma_flip.as_deref().unwrap_or("-"),
            self.gex.call_wall.as_deref().unwrap_or("-"),
            self.gex.put_wall.as_deref().unwrap_or("-"),
            self.gex.gamma_magnet.as_deref().unwrap_or("-"),
        )?;
        if let Some(g) = &self.latest_greek {
            writeln!(
                f,
                "  net_gamma={:?} positive_regime={:?}",
                g.net_gamma(),
                g.is_positive_gamma_regime()
            )?;
        }
        writeln!(f, "  recent_flow_alerts: {}", self.recent_alerts.len())?;
        for a in &self.recent_alerts {
            writeln!(
                f,
                "    - {} {} {} strike={} prem={} rule={}",
                a.created_at.as_deref().unwrap_or("?"),
                a.option_type.as_deref().unwrap_or("?"),
                a.expiry.as_deref().unwrap_or("?"),
                a.strike.as_deref().unwrap_or("?"),
                a.total_premium.as_deref().unwrap_or("?"),
                a.alert_rule.as_deref().unwrap_or("?"),
            )?;
        }
        Ok(())
    }
}
