/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use crate::context::metrics::{Metrics as CtxMetrics, ThirdPartyLabels};
use crate::util::odesli::cache::OdesliCache;
use crate::util::odesli::endpoints::OdesliEndpoints;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::histogram::Histogram;
use reqwest::StatusCode;
use std::borrow::Cow;
use std::fmt;
use std::time::{Duration, Instant};
use tracing::{Instrument, debug, instrument};
use url::Url;

mod api_type;
mod cache;
mod endpoints;
mod error;
mod provider_id;

use crate::util::odesli::provider_id::ProviderId;
pub use api_type::*;
pub use error::ApiErr;

pub struct OdesliClient {
    client: reqwest::Client,
    cache: OdesliCache,
    metrics: OdesliMetrics,
}

struct OdesliMetrics {
    third_party_api: Family<ThirdPartyLabels, Histogram>,
}

impl fmt::Debug for OdesliClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OdesliClient")
            .field("client", &self.client)
            .finish()
    }
}

impl OdesliClient {
    pub fn new(client: reqwest::Client, metrics: &CtxMetrics) -> Self {
        Self {
            client,
            cache: OdesliCache::new(),
            metrics: OdesliMetrics {
                third_party_api: metrics.third_party_api.clone(),
            },
        }
    }

    pub fn clear_expired_cache_entries(&self, max_age: Duration) {
        self.cache.clear_expired(max_age);
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn fetch(&self, url: &Url) -> Result<OdesliResponse, ApiErr> {
        match ProviderId::parse_url(url) {
            Ok(provider_id) => {
                if let Some(cached) = self.cache.get_response(&provider_id) {
                    debug!("Cache hit for provider {:?}", provider_id);
                    return Ok(cached);
                }
                debug!(
                    "Cache miss for provider {:?}, fetching from API",
                    provider_id
                );
            }
            Err(e) => {
                debug!("Failed to parse provider ID from URL: {}", e);
            }
        }

        let req_data = OdesliEndpoints::links(url.as_str());

        let req = self
            .client
            .request(req_data.method(), req_data.uri())
            .build()?;

        let now = Instant::now();
        let resp = self
            .client
            .execute(req)
            .instrument(tracing::info_span!("http_request"))
            .await?;
        let diff = now.elapsed();

        self.metrics
            .third_party_api
            .get_or_create(&ThirdPartyLabels {
                method: req_data.method().into(),
                url: Cow::from(req_data.metrics_uri()),
                status: resp.status().into(),
            })
            .observe(diff.as_secs_f64());

        match resp.status() {
            StatusCode::OK => {}
            StatusCode::TOO_MANY_REQUESTS => {
                return Err(ApiErr::RateLimitExceeded);
            }
            _ => {
                let status = resp.status();
                debug!(
                    "API request returned unexpected status {}, trying to read error response body",
                    status
                );
                match resp.text().await {
                    Ok(text) => debug!(body = %text, "Successfully read API error response body"),
                    Err(e) => debug!(failed_with = %e, "Failed to read API error response body"),
                }
                return Err(ApiErr::UnexpectedResponseStatus(status));
            }
        }

        let res = resp.json::<OdesliResponse>().await?;
        self.cache.store_response(&res);

        Ok(res)
    }
}
