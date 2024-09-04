/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use tracing::debug;
use twilight_model::application::command::{Command, CommandOptionType, CommandType};
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};
use twilight_util::builder::command::{CommandBuilder, StringBuilder};

use crate::commands::error::InvalidCommandInteraction;
use crate::util::discord_locales::DiscordLocale;

pub const URL_OPTION_NAME: &str = "url";

fn url_option() -> StringBuilder {
    StringBuilder::new(URL_OPTION_NAME, "The Link for the Song/Album")
        .description_localizations([(
            DiscordLocale::German.to_str(),
            "Der Link von dem Song/Album",
        )])
        .autocomplete(false)
        .required(true)
}

pub const COMMAND_NAME: &str = "share";

pub fn command() -> Command {
    CommandBuilder::new(
        COMMAND_NAME,
        "Share Music to all Platforms",
        CommandType::ChatInput,
    )
    .description_localizations([(
        DiscordLocale::German.to_str(),
        "Teile Musik von für alle Plattformen",
    )])
    .option(url_option())
    .dm_permission(true)
    .build()
}

pub struct ShareCommandData {
    pub url: String,
}

impl TryFrom<&CommandData> for ShareCommandData {
    type Error = InvalidCommandInteraction;

    fn try_from(data: &CommandData) -> Result<Self, Self::Error> {
        let mut url_option = None;

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
                    })
                }
                _ => {}
            }
        }

        let res = Self {
            url: url_option.ok_or(InvalidCommandInteraction::MissingOption {
                name: URL_OPTION_NAME,
            })?,
        };

        debug!(url = res.url, "Successfully parsed Share Command Data");

        Ok(res)
    }
}
