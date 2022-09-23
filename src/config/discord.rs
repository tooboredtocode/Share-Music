/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Options {
    pub token: String,
    #[serde(default)]
    pub debug_server: Vec<u64>
}
