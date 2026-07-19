/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */
use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
pub struct Args {
    /// The token to run the bot with
    #[clap(long, env = "DISCORD_TOKEN", hide_env_values = true)]
    pub token: String,
    /// The servers to register debug and testing commands with
    #[clap(long, env = "DISCORD_DEBUG_SERVER")]
    pub debug_server: Vec<u64>,

    /// The api key for the Odesli API
    #[clap(long = "api-key", env = "ODESLI_API_KEY", hide_env_values = true)]
    pub odesli_api_key: Option<String>,
    /// The hourly limit for the Odesli API
    #[clap(long = "odesli-hourly-limit", env = "ODESLI_HOURLY_LIMIT")]
    pub odesli_hourly_limit: Option<u32>,

    /// The port the metrics server will listen on
    #[clap(long, env = "METRICS_PORT", default_value_t = 8481)]
    pub metrics_port: u16,

    /// The database url to send the metrics to
    #[clap(long, env = "DATABASE_URL", hide_env_values = true)]
    pub database_url: Option<String>,

    /// The log filter configuration (e.g. "info,my_crate=debug").
    #[clap(short, long, default_value = "info", env = "BOT_LOG")]
    pub log: String,
    /// The log format configuration
    #[clap(long, default_value = "logfmt", env = "BOT_LOG_FORMAT")]
    pub log_format: LogFormat,

    /// The file with the (partial) color configuration for the bot in yaml format
    /// If no file is provided, the default color configuration will be used
    #[clap(long, env = "COLOR_CONFIG")]
    pub color_config: Option<PathBuf>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, ValueEnum)]
pub enum LogFormat {
    /// Logfmt with ANSI color codes.
    Logfmt,
    /// Logfmt without ANSI color codes.
    LogfmtPlain,
    /// JSON.
    Json,
}

impl Args {
    pub fn parse() -> Self {
        Parser::parse()
    }
}
