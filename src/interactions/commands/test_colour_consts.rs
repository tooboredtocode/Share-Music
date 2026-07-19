/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use crate::clients::colour;
use twilight_interactions::command::{CommandModel, CreateCommand};

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "test_colour_consts",
    integration_types = "guild_install",
    contexts = "guild bot_dm private_channel"
)]
/// Test Colour Consts
pub struct TestColorConstsCommand {
    /// Cover link
    pub url: String,
    /// Brightest Percent
    #[command(min_value = 0.0, max_value = 1.0)]
    pub brightest_percent: Option<f64>,
    /// Percent Factor
    pub percent_factor: Option<f64>,
    /// Saturation Factor
    pub saturation_factor: Option<f64>,
    /// Luminosity Factor
    pub luminosity_factor: Option<f64>,
}

impl From<&TestColorConstsCommand> for colour::OptionsOverride {
    fn from(data: &TestColorConstsCommand) -> Self {
        Self {
            brightest_percent: data.brightest_percent.map(|v| v as f32),
            percent_factor: data.percent_factor.map(|v| v as f32),
            saturation_factor: data.saturation_factor.map(|v| v as f32),
            luminosity_factor: data.luminosity_factor.map(|v| v as f32),
        }
    }
}
