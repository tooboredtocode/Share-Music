/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */
use reqwest::Client;
use std::time::Duration;

use crate::util::error::Expectable;
use crate::util::ShareResult;
use crate::Context;

impl Context {
    pub(super) fn create_http_client() -> ShareResult<Client> {
        Client::builder()
            .user_agent(crate::constants::USER_AGENT)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .expect_with("Failed to create HTTP client")
    }
}
