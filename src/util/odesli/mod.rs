/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::histogram::Histogram;
use reqwest::{RequestBuilder, StatusCode};
use std::borrow::Cow;
use std::fmt;
use std::time::Duration;
use tracing::{Instrument, debug, field, instrument};
use url::Url;

use crate::context::metrics::{Metrics as CtxMetrics, ThirdPartyLabels, ThirdPartyRateLimitLabels};
use crate::util::metric_utils::{
    HasHistogramFamilyExt, TimeFutureExt, UnpackErr, has_histogram_families,
};
use crate::util::odesli::cache::OdesliCache;
use crate::util::odesli::endpoints::OdesliEndpoints;
use crate::util::odesli::provider_id::ProviderId;
use crate::util::odesli::ratelimiter::OdesliRateLimiter;
use crate::util::odesli::shared_queue::SharedQueue;

mod api_type;
mod cache;
mod endpoints;
mod error;
pub mod provider_id;
mod ratelimiter;
mod shared_queue;

pub use api_type::*;
pub use cache::OdesliClientResponse;
pub use error::{ApiClientErr, ApiErr};

pub struct OdesliClient {
    client: reqwest::Client,
    api_key: Option<Box<str>>,
    ratelimiter: OdesliRateLimiter,
    shared_queue: SharedQueue<ProviderId, OdesliClientResponse>,
    cache: OdesliCache,
    metrics: OdesliMetrics,
}

pub struct OdesliClientBuilder<'a> {
    client: reqwest::Client,
    metrics: &'a mut CtxMetrics,
    api_key: Option<Box<str>>,
    hourly_limit: Option<u32>,
}

struct OdesliMetrics {
    third_party_rate_limit: Family<ThirdPartyRateLimitLabels, Histogram>,
    third_party_api: Family<ThirdPartyLabels, Histogram>,
}

has_histogram_families!(OdesliMetrics, (
    third_party_rate_limit: ThirdPartyRateLimitLabels,
    third_party_api: ThirdPartyLabels
));

impl fmt::Debug for OdesliClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OdesliClient")
            .field("client", &self.client)
            .field("api_key", &self.api_key.as_ref().map(|_| "****"))
            .finish()
    }
}

impl<'a> OdesliClientBuilder<'a> {
    fn new(client: reqwest::Client, metrics: &'a mut CtxMetrics) -> Self {
        Self {
            client,
            metrics,
            api_key: None,
            hourly_limit: None,
        }
    }

    pub fn with_api_key(mut self, api_key: Option<impl AsRef<str>>) -> Self {
        self.api_key = api_key.map(|k| Box::from(k.as_ref()));
        self
    }

    pub fn with_hourly_limit(mut self, hourly_limit: Option<u32>) -> Self {
        self.hourly_limit = hourly_limit;
        self
    }

    pub fn build(self) -> OdesliClient {
        let ratelimiter = OdesliRateLimiter::new(self.hourly_limit.unwrap_or(60) as usize);
        ratelimiter.register_metric(&mut self.metrics.registry);

        let metrics = OdesliMetrics {
            third_party_rate_limit: self.metrics.third_party_rate_limit.clone(),
            third_party_api: self.metrics.third_party_api.clone(),
        };

        OdesliClient {
            client: self.client,
            api_key: self.api_key,
            ratelimiter,
            shared_queue: SharedQueue::new(),
            cache: OdesliCache::new(),
            metrics,
        }
    }
}

impl OdesliClient {
    pub fn builder(client: reqwest::Client, metrics: &mut CtxMetrics) -> OdesliClientBuilder<'_> {
        OdesliClientBuilder::new(client, metrics)
    }

    pub fn clear_expired_cache_entries(&self, max_age: Duration) {
        self.cache.clear_expired(max_age);
    }

    fn request(&self, endpoint: &OdesliEndpoints<'_>) -> RequestBuilder {
        let mut req = self.client.request(endpoint.method(), endpoint.uri());
        for (key, value) in endpoint.query_parameters() {
            req = req.query(&[(key, value)]);
        }
        if let Some(api_key) = &self.api_key {
            req = req.query(&[("key", api_key.as_ref())]);
        }
        req
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn fetch(&self, url: &Url) -> Result<OdesliClientResponse, ApiErr> {
        let provider_id = match ProviderId::parse_url(url) {
            Ok(provider_id) => provider_id,
            Err(e) => {
                debug!(
                    "Failed to parse provider ID from URL, falling back to uncached API request: {}",
                    e
                );

                return self.fetch_inner(url).await;
            }
        };

        tracing::Span::current().record("provider_id", field::debug(&provider_id));

        if let Some(cached) = self.cache.get_response(&provider_id) {
            debug!("Cache hit for provider");
            return Ok(cached);
        }
        debug!("Cache miss for provider, fetching from API");

        self.shared_queue
            .run_shared(
                provider_id.clone(),
                || async {
                    // Check the cache again in case another request has already fetched the data
                    if let Some(cached) = self.cache.get_response(&provider_id) {
                        return Ok(cached);
                    }
                    self.fetch_inner(url).await
                },
                |result| result.duplicate(),
            )
            .await
    }

    /// fetch without caching, used internally by fetch
    async fn fetch_inner(&self, url: &Url) -> Result<OdesliClientResponse, ApiErr> {
        let req_data = OdesliEndpoints::links(url);

        // Wait for the rate limiter to allow us to make the request
        let diff = self.ratelimiter.acquire().time().await.1;
        self.metrics.observe_duration(
            ThirdPartyRateLimitLabels {
                method: req_data.method().into(),
                url: Cow::from(req_data.uri()),
            },
            diff,
        );

        let req = self.request(&req_data).build()?;

        let (resp, diff) = self
            .client
            .execute(req)
            .instrument(tracing::info_span!("http_request"))
            .time()
            .await
            .unpack_err()?;
        self.metrics.observe_duration(
            ThirdPartyLabels {
                method: req_data.method().into(),
                url: Cow::from(req_data.uri()),
                status: resp.status().into(),
            },
            diff,
        );

        match resp.status() {
            StatusCode::OK => {}
            StatusCode::BAD_REQUEST => {
                return Err(ApiErr::ClientError(
                    ApiClientErr::from_response(resp).await?,
                ));
            }
            StatusCode::TOO_MANY_REQUESTS => {
                return Err(ApiErr::RateLimitExceeded);
            }
            _ => {
                let status = resp.status();
                let text = resp.text().await?;
                return Err(ApiErr::UnexpectedResponseStatus {
                    status_code: status,
                    text,
                });
            }
        }

        let mut api_response = resp.json::<OdesliResponse>().await?;
        fix_platform_links(&mut api_response);
        let client_response = self.cache.store_response(api_response);

        Ok(client_response)
    }
}

// Fixes the links for some platforms, so they work properly
fn fix_platform_links(resp: &mut OdesliResponse) {
    if let Some(links) = resp.links_by_platform.get_mut(&Platform::AppleMusic) {
        let new = links.url.replace("geo.music.apple.com", "music.apple.com");
        let mut new_iter = new.split('?');

        let new = new_iter
            .next()
            .expect("A split should always return something");
        if let Some(query) = new_iter.next() {
            let song_id = query.split('&').find(|s| s.starts_with("i="));
            if let Some(song_id) = song_id {
                links.url = format!("{}?{}", new, song_id);
            } else {
                // Just return the album link
                links.url = new.to_string();
            }
        } else {
            links.url = new.to_string();
        }
    }
}
