/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::error::Error;
use std::fmt::{Display, Formatter};

use hyper::http;

use crate::util::error::BlanketImpl;
use crate::util::parser::ParsingError;

#[derive(Debug)]
pub enum ApiErr {
    BuildingResponse(http::Error),
    RequestFailed(hyper::Error),
    Non200Response(hyper::StatusCode),
    InvalidResponse(ParsingError),
}

impl From<http::Error> for ApiErr {
    fn from(err: http::Error) -> Self {
        ApiErr::BuildingResponse(err)
    }
}

impl From<hyper::Error> for ApiErr {
    fn from(err: hyper::Error) -> Self {
        ApiErr::RequestFailed(err)
    }
}

impl From<ParsingError> for ApiErr {
    fn from(err: ParsingError) -> Self {
        ApiErr::InvalidResponse(err)
    }
}

impl Display for ApiErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiErr::BuildingResponse(err) => write!(f, "Failed to build request with: {}", err),
            ApiErr::RequestFailed(err) => write!(f, "Request failed with: {}", err),
            ApiErr::Non200Response(code) => write!(f, "API returned a non 200 response: {}", code),
            ApiErr::InvalidResponse(err) => write!(f, "API returned a faulty object: {}", err),
        }
    }
}

impl Error for ApiErr {}

impl BlanketImpl for ApiErr {}
