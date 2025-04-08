/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use tracing::Level;
use tracing_subscriber::EnvFilter;
use crate::args::LogFormat;

pub fn setup(env_filter: &str, log_format: LogFormat) {
    let filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .parse_lossy(env_filter);

    let sub = tracing_subscriber::fmt().with_env_filter(filter);

    match log_format {
        LogFormat::Logfmt => sub.with_ansi(true).init(),
        LogFormat::LogfmtPlain => sub.with_ansi(false).init(),
        LogFormat::Json => sub.json().init(),
    }
}
