/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */
use std::future::IntoFuture;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{debug_span, warn, Instrument};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::Message;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::commands::sync_commands;
use crate::context::{ClusterState, Ctx};
use crate::util::{create_termination_future, EmptyResult};
use crate::util::error::expect_warn;

pub async fn get_options<'a, T>(data: &'a CommandData, context: &Ctx) -> EmptyResult<T>
where
    T: TryFrom<&'a CommandData>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    let res: T = match data
        .try_into()
        .map_err(expect_warn!("Received Invalid Interaction data, re-syncing commands"))
    {
        Ok(s) => s,
        Err(()) => {
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
    let _ = context
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
        .map_err(expect_warn!("Failed to respond to the Interaction"));
}

pub async fn update_defer_with_error(inter: &Interaction, context: &Ctx, msg: &str) {
    if context
        .interaction_client()
        .update_response(inter.token.as_str())
        .content(Some(msg))
        .into_future()
        .instrument(debug_span!("sending_error_message"))
        .await
        .map_err(expect_warn!("Failed to inform user of the error"))
        .is_ok()
    {
        let ctx = context.clone();
        let inter_token = inter.token.clone();
        tokio::spawn(async move {
            let _ = time::timeout(
                Duration::from_secs(15),
                create_termination_future(&ctx.state),
            )
                .await;

            ctx.interaction_client()
                .delete_response(inter_token.as_str())
                .into_future()
                .instrument(debug_span!("deleting_error_message"))
                .await
                .map_err(expect_warn!("Failed to delete Error Message"))
        });
    }
}
