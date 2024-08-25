/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::collections::HashMap;

use tracing::Metadata;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::{Context, Filter, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

use crate::config::logging::{Format, Options as LoggingOptions, Target};
use crate::Config;

pub fn setup(cfg: &Config) {
    match cfg.logging.format {
        Format::Json => {
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_filter(CfgFilter::from(&cfg.logging)),
                )
                .init();
        }
        Format::Ascii => {
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer().with_filter(CfgFilter::from(&cfg.logging)))
                .init();
        }
    }
}

struct CfgFilter {
    default_level: LevelFilter,
    target_overrides: Vec<(String, LevelFilter)>,
}

impl<S> Filter<S> for CfgFilter {
    fn enabled(&self, meta: &Metadata<'_>, _: &Context<'_, S>) -> bool {
        for (target, level) in &self.target_overrides {
            if meta.target().starts_with(target) {
                return *meta.level() <= *level;
            }
        }

        *meta.level() <= self.default_level
    }
}

impl CfgFilter {
    fn override_from_target(
        res: &mut Vec<(String, LevelFilter)>,
        prev: &String,
        target_map: &HashMap<String, Target>,
    ) {
        let mut buffer = Vec::new();
        for (target, value) in target_map {
            match value {
                Target::Level(level) => {
                    buffer.push((format!("{prev}{target}"), LevelFilter::from(*level)))
                }
                Target::Target(t_map) => {
                    Self::override_from_target(res, &format!("{prev}{target}::"), t_map)
                }
            }
        }

        res.append(&mut buffer);
    }
}

impl From<&LoggingOptions> for CfgFilter {
    fn from(opt: &LoggingOptions) -> Self {
        let mut target_overrides = Vec::new();

        Self::override_from_target(&mut target_overrides, &String::from(""), &opt.targets);

        CfgFilter {
            default_level: opt.level.into(),
            target_overrides,
        }
    }
}
