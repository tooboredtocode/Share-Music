/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */
use reqwest::Client;
use std::time::Duration;

use crate::Context;
use crate::util::EmptyResult;
use crate::util::error::expect_err;

impl Context {
    pub(super) fn create_http_client() -> EmptyResult<Client> {
        Client::builder()
            .user_agent(crate::constants::USER_AGENT)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(expect_err!("Failed to create HTTP client"))
    }
}
