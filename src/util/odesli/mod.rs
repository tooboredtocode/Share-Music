/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::histogram::Histogram;
use std::borrow::Cow;
use std::fmt;
use std::time::Instant;
use tracing::{Instrument, instrument};

use crate::context::metrics::{Metrics as CtxMetrics, ThirdPartyLabels};
use crate::util::odesli::endpoints::OdesliEndpoints;

mod api_type;
mod endpoints;
mod error;

pub use api_type::*;
pub use error::ApiErr;

pub struct OdesliClient {
    client: reqwest::Client,
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
            metrics: OdesliMetrics {
                third_party_api: metrics.third_party_api.clone(),
            },
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn fetch(&self, url: impl Into<String>) -> Result<OdesliResponse, ApiErr> {
        let req_data = OdesliEndpoints::links(url);

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

        if resp.status() != 200 {
            return Err(ApiErr::Non200Response(resp.status()));
        }

        Ok(resp.json().await?)
    }
}
