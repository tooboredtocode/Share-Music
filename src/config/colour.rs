/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use serde::Deserialize;

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct Options {
    #[serde(default = "default_brightest_percent")]
    pub brightest_percent: f32,

    #[serde(default = "default_percent_factor")]
    pub percent_factor: f32,
    #[serde(default = "default_saturation_factor")]
    pub saturation_factor: f32,
    #[serde(default = "default_luminosity_factor")]
    pub luminosity_factor: f32,
}

impl Default for Options {
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
