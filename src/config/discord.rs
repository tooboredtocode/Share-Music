/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Options {
    pub token: String,
    #[serde(default)]
    pub debug_server: Vec<u64>,
}
