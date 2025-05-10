/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */
use crate::commands::{
    find_links::COMMAND_NAME as FIND_COMMAND_NAME, share::COMMAND_NAME as SHARE_COMMAND_NAME,
    test_colour_consts::COMMAND_NAME as TEST_COMMAND_NAME,
};
use crate::context::Ctx;
use std::mem;
use tracing::{debug, instrument, warn};
use twilight_model::application::command::CommandType;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};
use twilight_model::channel::message::component::ComponentType;

mod common;
mod find_links;
mod messages;
mod share;
mod show_player;
mod test_colour_consts;

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
pub async fn handle(mut inter: Interaction, context: Ctx) {
    match mem::take(&mut inter.data) {
        Some(InteractionData::ApplicationCommand(command_data)) => {
            if inter.kind != InteractionType::ApplicationCommand {
                warn!(
                    "Received Interaction with type {} but data is ApplicationCommand",
                    inter.kind.kind()
                );
                return;
            }
            handle_application_commands(inter, *command_data, context).await;
        }
        Some(InteractionData::MessageComponent(component_data)) => {
            if inter.kind != InteractionType::MessageComponent {
                warn!(
                    "Received Interaction with type {} but data is MessageComponent",
                    inter.kind.kind()
                );
                return;
            }
            handle_message_components(inter, *component_data, context).await;
        }
        _ => {
            debug!("Received Unexpected {} Interaction", inter.kind.kind());
        }
    }
}

async fn handle_application_commands(inter: Interaction, command_data: CommandData, context: Ctx) {
    debug!("Received Application Command Interaction");

    match (command_data.kind, command_data.name.as_str()) {
        (CommandType::ChatInput, SHARE_COMMAND_NAME) => {
            share::handle(inter, command_data, context).await
        }
        (CommandType::ChatInput, TEST_COMMAND_NAME) => {
            test_colour_consts::handle(inter, command_data, context).await
        }
        (CommandType::Message, FIND_COMMAND_NAME) => {
            find_links::handle(inter, command_data, context).await
        }
        (kind, name) => debug!(
            "Unknown {} Application Command Interaction: {}",
            kind.kind(),
            name
        ),
    }
}

async fn handle_message_components(
    inter: Interaction,
    component_data: MessageComponentInteractionData,
    context: Ctx,
) {
    debug!("Received Message Component Interaction");

    match (
        component_data.component_type,
        component_data.custom_id.as_str(),
    ) {
        (ComponentType::TextSelectMenu, show_player::SELECT_ID) => {
            show_player::handle(inter, component_data, context).await
        }
        (kind, name) => debug!(
            "Unknown {} Application Command Interaction: {}",
            kind.name(),
            name
        ),
    }
}
