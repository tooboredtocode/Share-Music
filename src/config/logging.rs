/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::collections::HashMap;
use serde::Deserialize;
use tracing_subscriber::filter::LevelFilter;

#[derive(Clone, Debug, Deserialize, Default)]
pub struct Options {
    #[serde(default)]
    pub format: Format,
    #[serde(default)]
    pub level: Level,
    #[serde(default)]
    pub targets: HashMap<String, Target>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Json,
    Ascii
}

impl Default for Format {
    fn default() -> Self {
        Self::Json
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum Level {
    #[serde(alias="trace", alias="TRACE")]
    Trace,
    #[serde(alias="debug", alias="DEBUG")]
    Debug,
    #[serde(alias="info", alias="INFO")]
    Info,
    #[serde(alias="warn", alias="WARN")]
    Warn,
    #[serde(alias="error", alias="ERROR")]
    Error,
    #[serde(alias="off", alias="OFF")]
    Off
}

impl Default for Level {
    fn default() -> Self {
        Self::Trace
    }
}

impl From<Level> for LevelFilter {
    fn from(l: Level) -> Self {
        match l {
            Level::Trace => Self::TRACE,
            Level::Debug => Self::DEBUG,
            Level::Info => Self::INFO,
            Level::Warn => Self::WARN,
            Level::Error => Self::ERROR,
            Level::Off => Self::OFF
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Target {
    Level(Level),
    Target(HashMap<String, Self>)
}