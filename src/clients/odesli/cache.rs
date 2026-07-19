/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use core::fmt;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use tracing::warn;
use url::Url;

use crate::clients::odesli::OdesliResponse;
use crate::clients::odesli::provider_id::ProviderId;

pub(super) struct DataCacheEntry {
    response: OdesliResponse,
    last_access: AtomicU64,
}

pub(super) struct OdesliCache {
    cache: DashMap<ProviderId, Arc<DataCacheEntry>>,
}

pub struct OdesliClientResponse {
    /// Indicates whether the response was retrieved from the cache or not. This is useful for logging and metrics purposes.
    pub is_cached: bool,
    inner: Arc<DataCacheEntry>,
}

impl OdesliCache {
    pub fn new() -> Self {
        OdesliCache {
            cache: DashMap::new(),
        }
    }

    pub fn store_response(&self, response: OdesliResponse) -> OdesliClientResponse {
        let entry = Arc::new(DataCacheEntry {
            response,
            last_access: AtomicU64::new(Self::current_timestamp()),
        });

        let provider_ids = entry
            .response
            .links_by_platform
            .iter()
            .filter_map(|(platform, links)| {
                if platform.is_enabled() {
                    Some(links)
                } else {
                    None
                }
            })
            .filter_map(|links| {
                let url = match Url::parse(&links.url) {
                    Ok(u) => u,
                    Err(e) => {
                        warn!("Failed to parse URL {}: {}", links.url, e);
                        return None;
                    }
                };

                match ProviderId::parse_url(&url) {
                    Ok(pid) => Some(pid),
                    Err(e) => {
                        warn!("Failed to extract provider ID from URL: {}", e);
                        None
                    }
                }
            });

        for pid in provider_ids {
            self.cache.insert(pid, entry.clone());
        }

        OdesliClientResponse {
            is_cached: false,
            inner: entry,
        }
    }

    pub fn get_response(&self, provider_id: &ProviderId) -> Option<OdesliClientResponse> {
        if let Some(entry) = self.cache.get(provider_id) {
            entry.last_access.store(
                Self::current_timestamp(),
                std::sync::atomic::Ordering::Relaxed,
            );
            Some(OdesliClientResponse {
                is_cached: true,
                inner: entry.clone(),
            })
        } else {
            None
        }
    }

    pub fn clear_expired(&self, max_age: Duration) {
        let max_last_access = Self::current_timestamp() - max_age.as_secs();

        self.cache.retain(|_, entry| {
            max_last_access <= entry.last_access.load(std::sync::atomic::Ordering::Relaxed)
        });
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

impl OdesliClientResponse {
    pub fn duplicate(&self) -> Self {
        OdesliClientResponse {
            // Duplicating the response dictates that it is cached.
            is_cached: true,
            inner: self.inner.clone(),
        }
    }
}

impl Deref for OdesliClientResponse {
    type Target = OdesliResponse;

    fn deref(&self) -> &Self::Target {
        &self.inner.response
    }
}

impl fmt::Debug for OdesliClientResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.deref(), f)
    }
}
