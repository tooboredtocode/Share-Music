/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use hyper::Client;
use hyper::client::HttpConnector;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};

use crate::Context;

impl Context {
    pub(super) fn create_http_client() -> Client<HttpsConnector<HttpConnector>> {
        let mut http_connector = HttpConnector::new();
        http_connector.enforce_http(false);

        Client::builder().build(
            HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_or_http()
                .enable_all_versions()
                .wrap_connector(http_connector)
        )
    }
}