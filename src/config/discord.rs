/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Options {
    pub token: String,
    #[serde(default)]
    pub cluster_id: u64,
    #[serde(default = "default_cluster_count")]
    pub cluster_count: u64,
    #[serde(default)]
    pub debug_server: Vec<u64>,
}

fn default_cluster_count() -> u64 {
    1
}
