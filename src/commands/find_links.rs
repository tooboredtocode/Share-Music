/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::InteractionContextType;
use twilight_model::oauth::ApplicationIntegrationType;
use twilight_util::builder::command::CommandBuilder;

pub const COMMAND_NAME: &str = "Find Links";

pub fn command() -> Command {
    CommandBuilder::new(COMMAND_NAME, "", CommandType::Message)
        .integration_types([
            ApplicationIntegrationType::GuildInstall,
            ApplicationIntegrationType::UserInstall
        ])
        .contexts([
            InteractionContextType::Guild,
            InteractionContextType::BotDm,
            InteractionContextType::PrivateChannel
        ])
        .build()
}
