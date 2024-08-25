/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::future::IntoFuture;
use tracing::{debug, debug_span, instrument, Instrument};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;

use crate::commands::share::ShareCommandData;
use crate::context::Ctx;
use crate::handlers::interactions::common::VALID_LINKS_REGEX;
use crate::handlers::interactions::{common, messages};
use crate::util::error::Expectable;
use crate::util::interaction::{defer, get_options, respond_with};
use crate::util::EmptyResult;

pub async fn handle(inter: &Interaction, data: &CommandData, context: Ctx) {
    // use an inner function to make splitting the code easier
    let _ = handle_inner(inter, data, context).await;
}

#[instrument(name = "share_command_handler", level = "debug", skip_all)]
async fn handle_inner(inter: &Interaction, data: &CommandData, context: Ctx) -> EmptyResult<()> {
    debug!("Received Share Command Interaction");

    let options = get_options(data, &context).await?;
    validate_url(&options, inter, &context).await?;

    debug!("User passed valid arguments, deferring Response");
    let defer_future = defer(inter, &context);

    let embed = common::embed_routine(&options.url, &context, inter)
        .instrument(debug_span!("embed_routine"))
        .await?;

    defer_future
        .await
        .warn_with("Failed to join the defer future")
        .ok_or(())??;

    let r = context
        .interaction_client()
        .create_followup(inter.token.as_str())
        .embeds(&[embed.build()])
        .expect("Somehow we built an invalid embed, this should never happen")
        .into_future()
        .instrument(debug_span!("sending_response"))
        .await
        .warn_with("Failed to send the response to the user");

    if r.is_some() {
        debug!("Successfully sent Response");
    }

    Ok(())
}

pub async fn validate_url(
    options: &ShareCommandData,
    inter: &Interaction,
    context: &Ctx,
) -> EmptyResult<()> {
    match VALID_LINKS_REGEX.find(options.url.as_str()) {
        Some(mat) if mat.len() == options.url.len() => {
            Ok(())
        }
        _ => {
            debug!("URL is not valid, informing user");
            respond_with(
                inter,
                context,
                messages::invalid_url((&inter.locale).into()),
            )
            .await;
            Err(())
        }
    }
}
