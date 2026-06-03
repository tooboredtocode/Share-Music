/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use axum::http::Method;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum OdesliEndpoints {
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
