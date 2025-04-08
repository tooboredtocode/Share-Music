/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use std::future::IntoFuture;
use tracing::{debug, debug_span, instrument, Instrument};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;

use crate::commands::share::ShareCommandData;
use crate::context::Ctx;
use crate::handlers::interactions::common::{
    additional_link_validation, InvalidLink, VALID_LINKS_REGEX,
};
use crate::handlers::interactions::{common, messages};
use crate::util::interaction::{defer, get_options, respond_with};
use crate::util::EmptyResult;
use crate::util::error::expect_warn;

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
        .map_err(expect_warn!("Failed to join the defer future"))??;

    let r = context
        .interaction_client()
        .create_followup(inter.token.as_str())
        .embeds(&[embed.build()])
        .into_future()
        .instrument(debug_span!("sending_response"))
        .await
        .map_err(expect_warn!("Failed to send the response to the user"));

    if r.is_ok() {
        debug!("Successfully sent Response");
    }

    Ok(())
}

pub async fn validate_url(
    options: &ShareCommandData,
    inter: &Interaction,
    context: &Ctx,
) -> EmptyResult<()> {
    let mat = match VALID_LINKS_REGEX.find(options.url.as_str()) {
        Some(mat) if mat.len() == options.url.len() => mat,
        _ => {
            debug!("URL is not valid, informing user");
            respond_with(
                inter,
                context,
                messages::invalid_url((&inter.locale).into()),
            )
            .await;
            return Err(());
        }
    };

    if let Err(reason) = additional_link_validation(mat.as_str()) {
        match reason {
            InvalidLink::Playlist => {
                debug!("URL is a playlist, informing user");
                respond_with(
                    inter,
                    context,
                    messages::playlist_not_supported((&inter.locale).into()),
                )
                .await;
            }
            InvalidLink::Artist => {
                debug!("URL is an artist, informing user");
                respond_with(
                    inter,
                    context,
                    messages::artist_not_supported((&inter.locale).into()),
                )
                .await;
            }
            InvalidLink::YoutubeShort => {
                debug!("URL is a shorts video, informing user");
                respond_with(
                    inter,
                    context,
                    messages::youtube_shorts_not_supported((&inter.locale).into()),
                )
                .await;
            }
        }
        return Err(());
    }

    Ok(())
}
