/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use serde::Deserialize;

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct Options {
    #[serde(default = "default_metrics_port")]
    pub listen_port: u16,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            listen_port: default_metrics_port(),
        }
    }
}

const fn default_metrics_port() -> u16 {
    8481
}
