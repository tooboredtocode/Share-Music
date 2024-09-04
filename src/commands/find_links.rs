/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use twilight_model::application::command::{Command, CommandType};
use twilight_util::builder::command::CommandBuilder;

pub const COMMAND_NAME: &str = "Find Links";

pub fn command() -> Command {
    CommandBuilder::new(COMMAND_NAME, "", CommandType::Message)
        .dm_permission(true)
        .build()
}
