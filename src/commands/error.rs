/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use std::error::Error;
use std::fmt::{Display, Formatter};

use twilight_model::application::command::CommandOptionType;
use twilight_model::application::interaction::application_command::CommandOptionValue;

#[derive(Debug)]
pub enum InvalidCommandInteraction {
    MissingOption {
        name: &'static str,
    },
    InvalidOptionType {
        name: &'static str,
        expected: CommandOptionType,
        got: CommandOptionValue,
    },
}

impl Display for InvalidCommandInteraction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid Interaction Data: ")?;

        match self {
            InvalidCommandInteraction::MissingOption { name } => {
                write!(f, "Missing option: {}", name)
            }
            InvalidCommandInteraction::InvalidOptionType {
                name,
                expected,
                got,
            } => write!(
                f,
                "Invalid Option Type for {}: expected {}, got {}",
                name,
                expected.kind(),
                got.kind().kind()
            ),
        }
    }
}

impl Error for InvalidCommandInteraction {}
