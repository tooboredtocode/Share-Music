/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */
use std::path::Path;
use std::process::exit;
use serde::Deserialize;

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct ColorConfig {
    #[serde(default = "default_brightest_percent")]
    pub brightest_percent: f32,

    #[serde(default = "default_percent_factor")]
    pub percent_factor: f32,
    #[serde(default = "default_saturation_factor")]
    pub saturation_factor: f32,
    #[serde(default = "default_luminosity_factor")]
    pub luminosity_factor: f32,
}

impl ColorConfig {
    pub fn from_file(file: &Path) -> Self {
        let file = match std::fs::File::open(file) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Failed to open color config file: {}", e);
                exit(1)
            }
        };

        serde_yaml::from_reader(file).unwrap_or_else(|err| {
            eprintln!("Failed to parse color config file: {}", err);
            exit(1)
        })
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            brightest_percent: default_brightest_percent(),
            percent_factor: default_percent_factor(),
            saturation_factor: default_saturation_factor(),
            luminosity_factor: default_luminosity_factor(),
        }
    }
}

const fn default_brightest_percent() -> f32 {
    0.4
}

const fn default_percent_factor() -> f32 {
    2.0
}

const fn default_saturation_factor() -> f32 {
    8.0
}

const fn default_luminosity_factor() -> f32 {
    4.0
}
