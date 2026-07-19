/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use std::error::Error;
use std::fmt;

use reqwest::{Response, StatusCode};
use serde::Deserialize;
use tracing::debug;

#[derive(Debug)]
pub enum ApiErr {
    Reqwest(reqwest::Error),
    ClientError(ApiClientErr),
    RateLimitExceeded,
    UnexpectedClientError(String),
    UnexpectedResponseStatus {
        status_code: StatusCode,
        text: String,
    },
}

#[derive(Debug)]
pub enum ApiClientErr {
    InvalidEntityType,
    UnknownEntity,
    UnknownCode(String),
}

impl From<reqwest::Error> for ApiErr {
    fn from(err: reqwest::Error) -> Self {
        ApiErr::Reqwest(err)
    }
}

impl fmt::Display for ApiErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiErr::Reqwest(err) => write!(f, "Reqwest error: {}", err),
            ApiErr::ClientError(client_err) => write!(f, "Client error: {}", client_err),
            ApiErr::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            ApiErr::UnexpectedClientError(details) => {
                write!(f, "Unexpected client error response: {}", details)
            }
            ApiErr::UnexpectedResponseStatus { status_code, text } => write!(
                f,
                "Unexpected response status {}, with body: \"{}\"",
                status_code, text
            ),
        }
    }
}

impl Error for ApiErr {}

impl ApiClientErr {
    fn from_code(code: &str) -> Self {
        match code {
            "invalid_entity_type" => ApiClientErr::InvalidEntityType,
            "unknown_entity" => ApiClientErr::UnknownEntity,
            other => ApiClientErr::UnknownCode(other.to_string()),
        }
    }

    /// Parses a 400 Bad Request response from the API to determine the specific client error.
    pub async fn from_response(response: Response) -> Result<Self, ApiErr> {
        let bytes = response.bytes().await?;
        match serde_json::from_slice::<ApiClientErr>(&bytes) {
            Ok(client_err) => return Ok(client_err),
            Err(err) => debug!("Parsing client error response failed: {}", err),
        }
        let details = String::from_utf8_lossy(&bytes);
        Err(ApiErr::UnexpectedClientError(details.into_owned()))
    }
}

impl fmt::Display for ApiClientErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiClientErr::InvalidEntityType => f.write_str("The provided entity type is invalid."),
            ApiClientErr::UnknownEntity => f.write_str("The specified entity could not be found."),
            ApiClientErr::UnknownCode(code) => {
                write!(f, "API returned an unknown client error: {}", code)
            }
        }
    }
}

impl<'de> Deserialize<'de> for ApiClientErr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ApiClientErrResponse<'a> {
            #[serde(rename = "statusCode")]
            status_code: u16,
            code: &'a str,
        }

        let err_response = ApiClientErrResponse::deserialize(deserializer)?;
        if err_response.status_code != 400 {
            return Err(serde::de::Error::custom(format!(
                "Expected status code 400, got {}",
                err_response.status_code
            )));
        }

        Ok(ApiClientErr::from_code(err_response.code))
    }
}
