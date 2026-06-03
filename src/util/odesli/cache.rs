/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use crate::util::odesli::OdesliResponse;
use crate::util::odesli::provider_id::ProviderId;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::warn;
use url::Url;

struct DataCacheEntry {
    response: OdesliResponse,
    last_access: AtomicU64,
}

pub struct OdesliCache {
    cache: DashMap<ProviderId, Arc<DataCacheEntry>>,
}

impl OdesliCache {
    pub fn new() -> Self {
        OdesliCache {
            cache: DashMap::new(),
        }
    }

    pub fn store_response(&self, response: &OdesliResponse) {
        let provider_ids = response
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

        let entry = Arc::new(DataCacheEntry {
            response: response.clone(),
            last_access: AtomicU64::new(Self::current_timestamp()),
        });
        for pid in provider_ids {
            self.cache.insert(pid, entry.clone());
        }
    }

    pub fn get_response(&self, provider_id: &ProviderId) -> Option<OdesliResponse> {
        if let Some(entry) = self.cache.get(provider_id) {
            entry.last_access.store(
                Self::current_timestamp(),
                std::sync::atomic::Ordering::Relaxed,
            );
            Some(entry.response.clone())
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
