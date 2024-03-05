/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::ops::Deref;

use tracing::{debug, instrument, warn};
use twilight_model::application::command::CommandType;
use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};

use crate::commands::{
    find_links::COMMAND_NAME as FIND_COMMAND_NAME,
    share::COMMAND_NAME as SHARE_COMMAND_NAME,
    test_colour_consts::COMMAND_NAME as TEST_COMMAND_NAME
};
use crate::context::Ctx;

mod share;
mod find_links;
mod test_colour_consts;
mod common;
mod messages;

#[instrument(
    name = "interaction_handler",
    level = "debug",
    skip_all,
    fields(
        inter_id = inter.id.get(),
        user_id = inter.author_id().map(|id| id.get()),
        channel_id = inter.channel.as_ref().map(|channel| channel.id.get()),
        guild_id = inter.guild_id.map(|id| id.get())
    )
)]
pub async fn handle(inter: Interaction, context: Ctx) {
    match inter.kind {
        InteractionType::ApplicationCommand => handle_application_commands(inter, context).await,
        _ => {}
    }
}

async fn handle_application_commands(inter: Interaction, context: Ctx) {
    let data = match &inter.data {
        Some(d) => {
            debug!("Received Application Command Interaction");
            d
        },
        None => {
            warn!("Received Application Command Interaction without data");
            return;
        }
    };

    match data {
        InteractionData::ApplicationCommand(data) => {
            let data = data.deref();

            match (data.kind, data.name.as_str()) {
                (CommandType::ChatInput, SHARE_COMMAND_NAME) =>
                    share::handle(&inter, data, context).await,
                (CommandType::ChatInput, TEST_COMMAND_NAME) =>
                    test_colour_consts::handle(&inter, data, context).await,
                (CommandType::Message, FIND_COMMAND_NAME) =>
                    find_links::handle(&inter, data, context).await,
                (kind, name) => debug!(
                    "Unknown {} Application Command Interaction: {}",
                    kind.kind(), name
                )
            }
        }
        _ => {}
    }
}
