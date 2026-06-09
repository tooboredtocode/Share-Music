/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use axum::http::Method;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum OdesliEndpoints<'a> {
    Links {
        url: &'a str,
        // Return a song entity if the URL corresponds to a single
        song_if_single: bool,
    },
}

macro_rules! api {
    (base) => {
        "https://api.odesli.co/"
    };
    (version) => {
        "v1-alpha.1"
    };
    (full_base) => {
        concat!(api!(base), api!(version))
    };
}

const fn bool_to_str(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

impl<'a> OdesliEndpoints<'a> {
    pub fn links(url: &'a impl AsRef<str>) -> Self {
        Self::Links {
            url: url.as_ref(),
            song_if_single: true,
        }
    }

    #[allow(dead_code)]
    pub fn links_no_song_if_single(url: &'a impl AsRef<str>) -> Self {
        Self::Links {
            url: url.as_ref(),
            song_if_single: false,
        }
    }

    pub fn method(&self) -> Method {
        match self {
            OdesliEndpoints::Links { .. } => Method::GET,
        }
    }

    pub fn uri(&self) -> &'static str {
        match self {
            OdesliEndpoints::Links { .. } => concat!(api!(full_base), "/links"),
        }
    }

    pub fn query_parameters(&self) -> impl IntoIterator<Item = (&'static str, &'a str)> {
        match *self {
            OdesliEndpoints::Links {
                url,
                song_if_single,
            } => [("url", url), ("songIfSingle", bool_to_str(song_if_single))],
        }
    }
}
