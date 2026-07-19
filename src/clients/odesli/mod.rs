/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use metronomos_pulse::value::{ArcValue, PulseValue};
use reqwest::{RequestBuilder, StatusCode};
use tokio::time::MissedTickBehavior;
use tracing::{Instrument, debug, field, instrument};
use url::Url;

use crate::args::Args;
use crate::clients::odesli::cache::OdesliCache;
use crate::clients::odesli::endpoints::OdesliEndpoints;
use crate::clients::odesli::provider_id::ProviderId;
use crate::clients::odesli::ratelimiter::OdesliRateLimiter;
use crate::clients::odesli::shared_queue::SharedQueue;
use crate::metrics::MetricsStore;
use crate::metrics::labels::{ThirdPartyLabels, ThirdPartyRateLimitLabels};
use crate::util::metric_utils::{HasHistogramFamilyExt, TimeFutureExt, UnpackErr};

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
use metronomos::lifecycle::{Lifecycle, LifecycleContext};

#[derive(Clone, PulseValue)]
pub struct OdesliClient {
    inner: Arc<OdesliClientInner>,
}

struct OdesliClientInner {
    client: reqwest::Client,
    api_key: Option<Box<str>>,
    ratelimiter: OdesliRateLimiter,
    shared_queue: SharedQueue<ProviderId, OdesliClientResponse>,
    cache: OdesliCache,
    metrics: MetricsStore,
}

pub struct OdesliClientBuilder {
    client: reqwest::Client,
    metrics: MetricsStore,
    api_key: Option<Box<str>>,
    hourly_limit: Option<u32>,
}

impl fmt::Debug for OdesliClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OdesliClient")
            .field("client", &self.inner.client)
            .field("api_key", &self.inner.api_key.as_ref().map(|_| "****"))
            .finish()
    }
}

impl OdesliClientBuilder {
    fn new(client: reqwest::Client, metrics: MetricsStore) -> Self {
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
        let ratelimiter = OdesliRateLimiter::new(
            self.hourly_limit.unwrap_or(60) as usize,
            self.metrics.odesli_rate_limit_tokens().clone(),
        );

        let inner = OdesliClientInner {
            client: self.client,
            api_key: self.api_key,
            ratelimiter,
            shared_queue: SharedQueue::new(),
            cache: OdesliCache::new(),
            metrics: self.metrics,
        };

        OdesliClient {
            inner: Arc::new(inner),
        }
    }
}

impl OdesliClient {
    pub fn init(
        lifecycle: Lifecycle,
        client: reqwest::Client,
        args: ArcValue<Args>,
        metrics: MetricsStore,
    ) -> Self {
        let res = Self::builder(client, metrics)
            .with_api_key(args.odesli_api_key.as_deref())
            .with_hourly_limit(args.odesli_hourly_limit)
            .build();

        let cloned = res.clone();
        lifecycle.hook(move |ctx| cloned.clone().cache_cleanup_task(ctx));

        res
    }

    pub fn builder(client: reqwest::Client, metrics: MetricsStore) -> OdesliClientBuilder {
        OdesliClientBuilder::new(client, metrics)
    }

    pub fn clear_expired_cache_entries(&self, max_age: Duration) {
        self.inner.cache.clear_expired(max_age);
    }

    pub async fn cache_cleanup_task(self, ctx: LifecycleContext) {
        let mut interval = ctx.interval(Duration::from_mins(15));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let max_age = Duration::from_hours(3);

        loop {
            if interval.tick().await.is_none() {
                return; // Lifecycle ended
            }

            debug!("Running Odesli cache cleanup task");
            self.clear_expired_cache_entries(max_age);
        }
    }

    fn request(&self, endpoint: &OdesliEndpoints<'_>) -> RequestBuilder {
        let mut req = self.inner.client.request(endpoint.method(), endpoint.uri());
        for (key, value) in endpoint.query_parameters() {
            req = req.query(&[(key, value)]);
        }
        if let Some(api_key) = &self.inner.api_key {
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

        if let Some(cached) = self.inner.cache.get_response(&provider_id) {
            debug!("Cache hit for provider");
            return Ok(cached);
        }
        debug!("Cache miss for provider, fetching from API");

        self.inner
            .shared_queue
            .run_shared(
                provider_id.clone(),
                || async {
                    // Check the cache again in case another request has already fetched the data
                    if let Some(cached) = self.inner.cache.get_response(&provider_id) {
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
        let ((), diff) = self.inner.ratelimiter.acquire().time().await;
        self.inner.metrics.observe_duration(
            ThirdPartyRateLimitLabels {
                method: req_data.method().into(),
                url: Cow::from(req_data.uri()),
            },
            diff,
        );

        let req = self.request(&req_data).build()?;

        let (resp, diff) = self
            .inner
            .client
            .execute(req)
            .instrument(tracing::info_span!("http_request"))
            .time()
            .await
            .unpack_err()?;
        self.inner.metrics.observe_duration(
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
        let client_response = self.inner.cache.store_response(api_response);

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
