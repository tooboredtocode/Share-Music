/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use tracing::warn;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::Message;
use twilight_model::channel::message::MessageFlags;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::commands::sync_commands;
use crate::context::Ctx;
use crate::context::state::ClusterState;
use crate::util::EmptyResult;
use crate::util::error::Expectable;

pub async fn get_options<'a, T>(data: &'a CommandData, context: &Ctx) -> EmptyResult<T>
where
    T: TryFrom<&'a CommandData>,
    Result<T, T::Error>: Expectable<T>
{
    let res: T = match data.try_into()
        .warn_with("Received Invalid Interaction data, resyncing commands")
    {
        Some(s) => s,
        None => {
            if let Err(_) = sync_commands(context).await {
                context.set_state(ClusterState::Crashing)
            }
            return Err(());
        }
    };

    Ok(res)
}

pub fn get_message(data: &CommandData) -> EmptyResult<&Message> {
    let resolved = match &data.resolved {
        None => {
            warn!("Received Message Application Command Interaction without resolved data");
            return Err(())
        }
        Some(r) => r
    };

    match resolved.messages.iter().next() {
        None => {
            warn!("Received Message Application Command Interaction without message");
            Err(())
        }
        Some((_, msg)) => {
            Ok(msg)
        }
    }
}

pub async fn defer(inter: &Interaction, context: &Ctx) -> EmptyResult<()> {
    if let Err(_) = context.interaction_client()
        .create_response(
            inter.id,
            inter.token.as_str(),
            &InteractionResponse {
                kind: InteractionResponseType::DeferredChannelMessageWithSource,
                data: None
            }
        )
        .await
    {
        warn!("Failed to defer Response, aborting handler");
        return Err(());
    }

    Ok(())
}

pub async fn respond_with(inter: &Interaction, context: &Ctx, msg: &str) {
    context.interaction_client()
        .create_response(
            inter.id,
            inter.token.as_str(),
            &InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: InteractionResponseDataBuilder::new()
                    .content(msg)
                    .flags(MessageFlags::EPHEMERAL)
                    .build()
                    .into()
            }
        )
        .await
        .warn_with("Failed to respond to the Interaction");
}