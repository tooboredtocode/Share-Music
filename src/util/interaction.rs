/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use tracing::warn;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};

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
        .exec()
        .await
    {
        warn!("Failed to defer Response, aborting handler");
        return Err(());
    }

    Ok(())
}