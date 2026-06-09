/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use reqwest::{RequestBuilder, StatusCode};
use std::borrow::Cow;
use std::fmt;
use std::time::{Duration, Instant};
use tracing::{Instrument, debug, field, instrument};
use url::Url;

use crate::context::metrics::{
    Metrics as CtxMetrics, RequestWaitingState, ThirdPartyLabels, ThirdPartyWaitingLabels,
};
use crate::util::odesli::cache::OdesliCache;
use crate::util::odesli::endpoints::OdesliEndpoints;
use crate::util::odesli::provider_id::ProviderId;
use crate::util::odesli::ratelimiter::OdesliRateLimiter;

mod api_type;
mod cache;
mod endpoints;
mod error;
pub mod provider_id;
mod ratelimiter;
mod shared_queue;

use crate::util::odesli::shared_queue::SharedQueue;
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

struct OdesliMetrics {
    waiting_requests: Family<ThirdPartyWaitingLabels, Gauge>,
    third_party_api: Family<ThirdPartyLabels, Histogram>,
}

impl fmt::Debug for OdesliClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OdesliClient")
            .field("client", &self.client)
            .field("api_key", &self.api_key.as_ref().map(|_| "****"))
            .finish()
    }
}

impl OdesliClient {
    pub fn new(client: reqwest::Client, metrics: &CtxMetrics) -> Self {
        Self {
            client,
            api_key: None,
            ratelimiter: OdesliRateLimiter::new(60),
            shared_queue: SharedQueue::new(),
            cache: OdesliCache::new(),
            metrics: OdesliMetrics {
                waiting_requests: metrics.third_party_api_waiting.clone(),
                third_party_api: metrics.third_party_api.clone(),
            },
        }
    }

    pub fn with_api_key(mut self, api_key: impl AsRef<str>) -> Self {
        self.api_key = Some(Box::from(api_key.as_ref()));
        self
    }

    pub fn with_hourly_limit(mut self, hourly_limit: u32) -> Self {
        self.ratelimiter = OdesliRateLimiter::new(hourly_limit);
        self
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

    fn start_metrics_phase(&self, req_data: &OdesliEndpoints<'_>, state: RequestWaitingState) {
        self.metrics
            .waiting_requests
            .get_or_create(&ThirdPartyWaitingLabels {
                method: req_data.method().into(),
                url: Cow::from(req_data.uri()),
                state,
            })
            .inc();
    }

    fn end_metrics_phase(&self, req_data: &OdesliEndpoints<'_>, state: RequestWaitingState) {
        self.metrics
            .waiting_requests
            .get_or_create(&ThirdPartyWaitingLabels {
                method: req_data.method().into(),
                url: Cow::from(req_data.uri()),
                state,
            })
            .dec();
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

                // Wait for the rate limiter to allow us to make the request
                self.ratelimiter.acquire().await;

                // Make the API request without caching, since we don't have a provider ID to cache with
                return self.fetch_inner(&OdesliEndpoints::links(url)).await;
            }
        };

        tracing::Span::current().record("provider_id", field::debug(&provider_id));

        self.shared_queue
            .run_shared(
                provider_id.clone(),
                || async {
                    if let Some(cached) = self.cache.get_response(&provider_id) {
                        debug!("Cache hit for provider");
                        return Ok(cached);
                    }
                    debug!("Cache miss for provider, fetching from API");

                    let req_data = OdesliEndpoints::links(url);

                    self.start_metrics_phase(&req_data, RequestWaitingState::Ratelimited);
                    // Wait for the rate limiter to allow us to make the request
                    self.ratelimiter.acquire().await;
                    self.end_metrics_phase(&req_data, RequestWaitingState::Ratelimited);

                    self.start_metrics_phase(&req_data, RequestWaitingState::WaitingForResponse);
                    // Make the API request and cache the response
                    let res = self.fetch_inner(&req_data).await;
                    self.end_metrics_phase(&req_data, RequestWaitingState::WaitingForResponse);

                    res
                },
                |result| result.duplicate(),
            )
            .await
    }

    /// fetch without rate limiting or caching, used internally by fetch
    async fn fetch_inner(
        &self,
        req_data: &OdesliEndpoints<'_>,
    ) -> Result<OdesliClientResponse, ApiErr> {
        let req = self.request(req_data).build()?;

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
                url: Cow::from(req_data.uri()),
                status: resp.status().into(),
            })
            .observe(diff.as_secs_f64());

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
