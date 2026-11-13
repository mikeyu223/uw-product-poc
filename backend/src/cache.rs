//! In-memory scored-book cache with TTL + singleflight.
//!
//! Without this, each dashboard load was calling Unusual Whales ~25–50 times
//! (portfolio + positions both re-scored the full book sequentially).

use crate::risk::{ScoredPosition, score_book};
use crate::uw::{UwClient, UwError};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Notify, RwLock};
use tracing::info;

const DEFAULT_TTL: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, Serialize)]
pub struct CachedBook {
    pub positions: Vec<ScoredPosition>,
    pub fetched_at: DateTime<Utc>,
    pub ttl_secs: u64,
    pub cache_hit: bool,
    pub age_secs: u64,
}

impl CachedBook {
    pub fn fresh(positions: Vec<ScoredPosition>, ttl: Duration) -> Self {
        Self {
            positions,
            fetched_at: Utc::now(),
            ttl_secs: ttl.as_secs(),
            cache_hit: false,
            age_secs: 0,
        }
    }

    pub fn with_hit_meta(mut self, now: DateTime<Utc>) -> Self {
        self.cache_hit = true;
        self.age_secs = (now - self.fetched_at).num_seconds().max(0) as u64;
        self
    }

    pub fn is_fresh(&self, now: DateTime<Utc>, ttl: Duration) -> bool {
        now.signed_duration_since(self.fetched_at)
            .to_std()
            .map(|age| age < ttl)
            .unwrap_or(false)
    }
}

struct CacheInner {
    book: Option<CachedBook>,
    refreshing: bool,
}

/// Shared cache for the scored sample book.
#[derive(Clone)]
pub struct BookCache {
    inner: Arc<RwLock<CacheInner>>,
    refresh_lock: Arc<Mutex<()>>,
    notify: Arc<Notify>,
    ttl: Duration,
}

impl BookCache {
    pub fn new() -> Self {
        Self::with_ttl(DEFAULT_TTL)
    }

    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            inner: Arc::new(RwLock::new(CacheInner {
                book: None,
                refreshing: false,
            })),
            refresh_lock: Arc::new(Mutex::new(())),
            notify: Arc::new(Notify::new()),
            ttl,
        }
    }

    /// Return cached book if fresh; otherwise refresh (singleflight).
    /// `force` bypasses TTL (Refresh button).
    pub async fn get(&self, uw: &UwClient, force: bool) -> Result<CachedBook, UwError> {
        let now = Utc::now();

        if !force {
            let guard = self.inner.read().await;
            if let Some(book) = &guard.book {
                if book.is_fresh(now, self.ttl) {
                    info!(age_secs = book.age_secs.max(0), "book cache hit");
                    return Ok(book.clone().with_hit_meta(now));
                }
            }
        }

        self.refresh(uw, force).await
    }

    async fn refresh(&self, uw: &UwClient, force: bool) -> Result<CachedBook, UwError> {
        // Singleflight: only one refresh at a time; waiters reuse the result.
        let _gate = self.refresh_lock.lock().await;

        // Another waiter may have filled the cache while we waited.
        if !force {
            let now = Utc::now();
            let guard = self.inner.read().await;
            if let Some(book) = &guard.book {
                if book.is_fresh(now, self.ttl) {
                    return Ok(book.clone().with_hit_meta(now));
                }
            }
        }

        {
            let mut guard = self.inner.write().await;
            guard.refreshing = true;
        }

        info!(force, "refreshing scored book from Unusual Whales");
        let result = score_book(uw).await;

        let mut guard = self.inner.write().await;
        guard.refreshing = false;

        match result {
            Ok(positions) => {
                let book = CachedBook::fresh(positions, self.ttl);
                guard.book = Some(book.clone());
                self.notify.notify_waiters();
                info!(
                    count = book.positions.len(),
                    ttl_secs = self.ttl.as_secs(),
                    "book cache updated"
                );
                Ok(book)
            }
            Err(err) => {
                // Serve stale cache on UW failure if we have anything.
                if let Some(book) = &guard.book {
                    info!("UW refresh failed — serving stale cache");
                    return Ok(book.clone().with_hit_meta(Utc::now()));
                }
                Err(err)
            }
        }
    }
}

impl Default for BookCache {
    fn default() -> Self {
        Self::new()
    }
}
