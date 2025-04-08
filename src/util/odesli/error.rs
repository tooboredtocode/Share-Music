/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use std::error::Error;
use std::fmt::{Display, Formatter};

use reqwest::StatusCode;

#[derive(Debug)]
pub enum ApiErr {
    Reqwest(reqwest::Error),
    Non200Response(StatusCode),
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
            ApiErr::Non200Response(code) => write!(f, "API returned a non 200 response: {}", code),
        }
    }
}

impl Error for ApiErr {}
