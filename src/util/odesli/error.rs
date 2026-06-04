/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use std::error::Error;
use std::fmt::{Display, Formatter};

use reqwest::StatusCode;

#[derive(Debug)]
pub enum ApiErr {
    Reqwest(reqwest::Error),
    RateLimitExceeded,
    UnexpectedResponseStatus(StatusCode),
}

impl From<reqwest::Error> for ApiErr {
    fn from(err: reqwest::Error) -> Self {
        ApiErr::Reqwest(err)
    }
}

impl Display for ApiErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiErr::Reqwest(err) => write!(f, "Reqwest error: {}", err),
            ApiErr::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            ApiErr::UnexpectedResponseStatus(status_code) => {
                write!(f, "Unexpected response status: {}", status_code)
            }
        }
    }
}

impl Error for ApiErr {}
