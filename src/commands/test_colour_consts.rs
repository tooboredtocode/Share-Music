/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use tracing::debug;
use twilight_model::application::command::{Command, CommandOptionType, CommandType};
use twilight_model::application::interaction::InteractionContextType;
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};
use twilight_model::oauth::ApplicationIntegrationType;
use twilight_util::builder::command::{CommandBuilder, NumberBuilder, StringBuilder};

use crate::commands::error::InvalidCommandInteraction;
use crate::util::colour;

pub const URL_OPTION_NAME: &str = "url";

fn url_option() -> StringBuilder {
    StringBuilder::new(URL_OPTION_NAME, "Cover link")
        .autocomplete(false)
        .required(true)
}

pub const BRIGHTEST_PERCENT_OPTION_NAME: &str = "brightest_percent";

fn brightest_percent_option() -> NumberBuilder {
    NumberBuilder::new(BRIGHTEST_PERCENT_OPTION_NAME, "Brightest Percent")
        .min_value(0.0)
        .max_value(1.0)
}

pub const PERCENT_FACTOR_OPTION_NAME: &str = "percent_factor";

fn percent_factor_option() -> NumberBuilder {
    NumberBuilder::new(PERCENT_FACTOR_OPTION_NAME, "Percent Factor")
}

pub const SATURATION_FACTOR_OPTION_NAME: &str = "saturation_factor";

fn saturation_factor_option() -> NumberBuilder {
    NumberBuilder::new(SATURATION_FACTOR_OPTION_NAME, "Saturation Factor")
}

pub const LUMINOSITY_FACTOR_OPTION_NAME: &str = "luminosity_factor";

fn luminosity_factor_option() -> NumberBuilder {
    NumberBuilder::new(LUMINOSITY_FACTOR_OPTION_NAME, "Luminosity Factor")
}

pub const COMMAND_NAME: &str = "test_colour_consts";

pub fn command() -> Command {
    CommandBuilder::new(COMMAND_NAME, "Test Colour Consts", CommandType::ChatInput)
        .option(url_option())
        .option(brightest_percent_option())
        .option(percent_factor_option())
        .option(saturation_factor_option())
        .option(luminosity_factor_option())
        .integration_types([ApplicationIntegrationType::GuildInstall])
        .contexts([
            InteractionContextType::Guild,
            InteractionContextType::BotDm,
            InteractionContextType::PrivateChannel,
        ])
        .build()
}

#[derive(Clone, Debug)]
pub struct TestConstsCommandData {
    pub url: String,
    pub brightest_percent: Option<f32>,
    pub percent_factor: Option<f32>,
    pub saturation_factor: Option<f32>,
    pub luminosity_factor: Option<f32>,
}

impl From<&TestConstsCommandData> for colour::Options {
    fn from(data: &TestConstsCommandData) -> Self {
        Self {
            brightest_percent: data.brightest_percent,
            percent_factor: data.percent_factor,
            saturation_factor: data.saturation_factor,
            luminosity_factor: data.luminosity_factor,
        }
    }
}

impl TryFrom<&CommandData> for TestConstsCommandData {
    type Error = InvalidCommandInteraction;

    fn try_from(data: &CommandData) -> Result<Self, Self::Error> {
        let mut url_option = None;
        let mut brightest_percent_option = None;
        let mut percent_factor_option = None;
        let mut saturation_factor_option = None;
        let mut luminosity_factor_option = None;

        for option in &data.options {
            match (option.name.as_str(), option.value.clone()) {
                (URL_OPTION_NAME, CommandOptionValue::String(url)) => {
                    url_option = Some(url);
                }
                (URL_OPTION_NAME, other_type) => {
                    return Err(InvalidCommandInteraction::InvalidOptionType {
                        name: URL_OPTION_NAME,
                        expected: CommandOptionType::String,
                        got: other_type,
                    });
                }
                (BRIGHTEST_PERCENT_OPTION_NAME, CommandOptionValue::Number(val)) => {
                    brightest_percent_option = Some(val)
                }
                (BRIGHTEST_PERCENT_OPTION_NAME, other_type) => {
                    return Err(InvalidCommandInteraction::InvalidOptionType {
                        name: BRIGHTEST_PERCENT_OPTION_NAME,
                        expected: CommandOptionType::Number,
                        got: other_type,
                    });
                }
                (PERCENT_FACTOR_OPTION_NAME, CommandOptionValue::Number(val)) => {
                    percent_factor_option = Some(val)
                }
                (PERCENT_FACTOR_OPTION_NAME, other_type) => {
                    return Err(InvalidCommandInteraction::InvalidOptionType {
                        name: PERCENT_FACTOR_OPTION_NAME,
                        expected: CommandOptionType::Number,
                        got: other_type,
                    });
                }
                (SATURATION_FACTOR_OPTION_NAME, CommandOptionValue::Number(val)) => {
                    saturation_factor_option = Some(val)
                }
                (SATURATION_FACTOR_OPTION_NAME, other_type) => {
                    return Err(InvalidCommandInteraction::InvalidOptionType {
                        name: SATURATION_FACTOR_OPTION_NAME,
                        expected: CommandOptionType::Number,
                        got: other_type,
                    });
                }
                (LUMINOSITY_FACTOR_OPTION_NAME, CommandOptionValue::Number(val)) => {
                    luminosity_factor_option = Some(val)
                }
                (LUMINOSITY_FACTOR_OPTION_NAME, other_type) => {
                    return Err(InvalidCommandInteraction::InvalidOptionType {
                        name: LUMINOSITY_FACTOR_OPTION_NAME,
                        expected: CommandOptionType::Number,
                        got: other_type,
                    });
                }
                _ => {}
            }
        }

        let res = Self {
            url: url_option.ok_or(InvalidCommandInteraction::MissingOption {
                name: URL_OPTION_NAME,
            })?,
            brightest_percent: brightest_percent_option.map(|f| f as f32),
            percent_factor: percent_factor_option.map(|f| f as f32),
            saturation_factor: saturation_factor_option.map(|f| f as f32),
            luminosity_factor: luminosity_factor_option.map(|f| f as f32),
        };

        debug!(
            url = res.url,
            brightest_percent = res.brightest_percent,
            percent_factor = res.percent_factor,
            saturation_factor = res.saturation_factor,
            luminosity_factor = res.luminosity_factor,
            "Successfully parsed Test Colour Const Command Data"
        );

        Ok(res)
    }
}
