/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::time::Instant;

use hyper::{Body, Method};
use tracing::{instrument, Instrument};

pub use api_type::*;
pub use error::ApiErr;

use crate::context::metrics::ThirdPartyLabels;
use crate::context::Ctx;
use crate::util::parser;

mod api_type;
mod error;

#[derive(Clone, Debug, Eq, PartialEq)]
enum OdesliEndpoints {
    Links { url: String },
}

impl OdesliEndpoints {
    const BASE: &'static str = "https://api.song.link";
    const API_VERSION: &'static str = "v1-alpha.1";

    pub fn links(url: impl Into<String>) -> Self {
        Self::Links { url: url.into() }
    }

    pub fn method(&self) -> Method {
        match self {
            OdesliEndpoints::Links { .. } => Method::GET,
        }
    }

    pub fn uri(&self) -> String {
        self.to_string()
    }

    pub fn metrics_uri(&self) -> String {
        match self {
            OdesliEndpoints::Links { .. } => format!("{}/{}/links", Self::BASE, Self::API_VERSION),
        }
    }
}

impl Display for OdesliEndpoints {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", Self::BASE, Self::API_VERSION)?;

        match self {
            OdesliEndpoints::Links { url } => write!(f, "/links?url={}", url),
        }
    }
}

#[instrument(level = "debug", skip_all)]
pub async fn fetch_from_api(url: &String, context: &Ctx) -> Result<OdesliResponse, ApiErr> {
    let req_data = OdesliEndpoints::links(url);

    let req = hyper::Request::builder()
        .method(req_data.method())
        .uri(req_data.uri())
        .body(Body::empty())?;

    let now = Instant::now();
    let resp = context
        .http_client
        .request(req)
        .instrument(tracing::info_span!("http_request"))
        .await?;
    let diff = now.elapsed();

    context
        .metrics
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

    Ok(parser::parse(resp).await?)
}
