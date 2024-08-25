/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use tokio::task::JoinHandle;
use tracing::{debug_span, warn, Instrument};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::Message;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::commands::sync_commands;
use crate::context::{ClusterState, Ctx};
use crate::util::error::Expectable;
use crate::util::EmptyResult;

pub async fn get_options<'a, T>(data: &'a CommandData, context: &Ctx) -> EmptyResult<T>
where
    T: TryFrom<&'a CommandData>,
    Result<T, T::Error>: Expectable<T>,
{
    let res: T = match data
        .try_into()
        .warn_with("Received Invalid Interaction data, re-syncing commands")
    {
        Some(s) => s,
        None => {
            if sync_commands(context).await.is_err() {
                context.state.set(ClusterState::Crashing);
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
            return Err(());
        }
        Some(r) => r,
    };

    match resolved.messages.iter().next() {
        None => {
            warn!("Received Message Application Command Interaction without message");
            Err(())
        }
        Some((_, msg)) => Ok(msg),
    }
}

pub fn defer(inter: &Interaction, context: &Ctx) -> JoinHandle<EmptyResult<()>> {
    let inter_id = inter.id;
    let inter_token = inter.token.clone();
    let ctx = context.clone();

    tokio::spawn(
        async move {
            if let Err(e) = ctx
                .interaction_client()
                .create_response(
                    inter_id,
                    inter_token.as_str(),
                    &InteractionResponse {
                        kind: InteractionResponseType::DeferredChannelMessageWithSource,
                        data: None,
                    },
                )
                .await
            {
                warn!("Failed to defer Response, aborting handler: {}", e);
                return Err(());
            }

            Ok(())
        }
        .instrument(debug_span!("deferring_response")),
    )
}

pub async fn respond_with(inter: &Interaction, context: &Ctx, msg: &str) {
    context
        .interaction_client()
        .create_response(
            inter.id,
            inter.token.as_str(),
            &InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: InteractionResponseDataBuilder::new()
                    .content(msg)
                    .flags(MessageFlags::EPHEMERAL)
                    .build()
                    .into(),
            },
        )
        .await
        .warn_with("Failed to respond to the Interaction");
}
