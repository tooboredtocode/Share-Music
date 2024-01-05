/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use figment::{Error, Figment};
use figment::providers::{Env, Format, Json, Yaml};
use serde::Deserialize;

use crate::constants::config_consts;

pub mod discord;
pub mod logging;
pub mod metrics;
pub mod colour;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub discord: discord::Options,
    #[serde(default)]
    pub metrics: metrics::Options,
    #[serde(default)]
    pub logging: logging::Options,
    #[serde(default)]
    pub colour: colour::Options
}

impl Config {
    pub fn load() -> Result<Self, Error> {
        Figment::new()
            .adjoin(Yaml::file(config_consts::YAML_FILE_PATH))
            .adjoin(Json::file(config_consts::JSON_FILE_PATH))
            .merge(Env::raw().map(|k| {
                // maps the first underscore in the key to a dot, nesting the key
                // e.g. "discord_token" -> "discord.token"
                match k.as_str().split_once("_") {
                    Some((r, l)) => format!("{}.{}", r, l).into(),
                    None => k.into()
                }
            }))
            .extract()
    }
}
