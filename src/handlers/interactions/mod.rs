/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::ops::Deref;

use tracing::{debug, instrument, warn};
use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};

use crate::commands;
use crate::context::Ctx;

mod share;
mod test_colour_consts;

#[instrument(
    name = "interaction_handler",
    level = "debug",
    skip_all,
    fields(
        inter_id = inter.application_id.get(),
        user_id = inter.author_id().map(|id| id.get()),
        channel_id = inter.channel_id.map(|id| id.get()),
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

            match data.name.as_str() {
                commands::share::COMMAND_NAME => share::handle(&inter, data, context).await,
                commands::test_colour_consts::COMMAND_NAME => test_colour_consts::handle(&inter, data, context).await,
                _ => {}
            }
        }
        _ => {}
    }
}
