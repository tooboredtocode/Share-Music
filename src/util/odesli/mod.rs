/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use hyper::{Body, Method};
use std::fmt::{Display, Formatter};
use std::time::Instant;

use crate::commands::share::ShareCommandData;
use crate::context::Ctx;
use crate::util::parser;

pub use api_type::*;
pub use error::ApiErr;

mod api_type;
mod error;

#[derive(Clone, Debug, Eq, PartialEq)]
enum OdesliEndpoints {
    Links {
        url: String
    }
}

impl OdesliEndpoints {
    const BASE: &'static str = "https://api.song.link";
    const API_VERSION: &'static str = "v1-alpha.1";

    pub fn links(url: impl Into<String>) -> Self {
        Self::Links {
            url: url.into()
        }
    }

    pub fn method(&self) -> Method {
        match self {
            OdesliEndpoints::Links { .. } => Method::GET
        }
    }

    pub fn uri(&self) -> String {
        self.to_string()
    }

    pub fn metrics_uri(&self) -> String {
        match self {
            OdesliEndpoints::Links { .. } => format!("{}/{}/links", Self::BASE, Self::API_VERSION)
        }
    }
}

impl Display for OdesliEndpoints {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", Self::BASE, Self::API_VERSION)?;

        match self {
            OdesliEndpoints::Links {
                url
            } => write!(f, "/links?url={}", url)
        }
    }
}

pub async fn fetch_from_api(options: &ShareCommandData, context: &Ctx) -> Result<OdesliResponse, ApiErr> {
    let req_data = OdesliEndpoints::links(&options.url);

    let req = hyper::Request::builder()
        .method(req_data.method())
        .uri(req_data.uri())
        .body(Body::empty())?;

    let now = Instant::now();
    let resp = context.http_client.request(req).await?;
    let diff = now.elapsed();

    context.metrics.third_party_api
        .get_metric_with_label_values(&[
            req_data.method().as_str(),
            req_data.metrics_uri().as_str(),
            resp.status().as_str()
        ])
        .unwrap()
        .observe(diff.as_secs_f64());

    if resp.status() != 200 {
        return Err(ApiErr::Non200Response(resp.status()));
    }

    Ok(parser::parse(resp).await?)
}
