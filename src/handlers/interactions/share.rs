/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use std::future::IntoFuture;
use tracing::{debug, debug_span, instrument, warn, Instrument};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;

use crate::commands::share::ShareCommandData;
use crate::context::Ctx;
use crate::handlers::interactions::common::{additional_link_validation, build_embed, data_routine, InvalidLink, VALID_LINKS_REGEX};
use crate::handlers::interactions::messages;
use crate::handlers::interactions::show_player::build_components;
use crate::util::interaction::{defer, get_options, respond_with, update_defer_with_error};
use crate::util::EmptyResult;
use crate::util::error::expect_warn;

pub async fn handle(inter: Interaction, data: CommandData, context: Ctx) {
    // use an inner function to make splitting the code easier
    let _ = handle_inner(inter, data, context).await;
}

#[instrument(name = "share_command_handler", level = "debug", skip_all)]
async fn handle_inner(inter: Interaction, data: CommandData, context: Ctx) -> EmptyResult<()> {
    debug!("Received Share Command Interaction");

    let options = get_options(&data, &context).await?;
    validate_url(&options, &inter, &context).await?;

    debug!("User passed valid arguments, deferring Response");
    let defer_future = defer(&inter, &context);

    let (data, entity, color) = match data_routine(&options.url, &context).await {
        Ok(data) => data,
        Err(e) => {
            warn!(failed_with = %e, "Failed to get the data from the api");
            update_defer_with_error(
                &inter,
                &context,
                messages::error((&inter.locale).into())
            ).await;
            return Err(());
        }
    };

    defer_future
        .await
        .map_err(expect_warn!("Failed to join the defer future"))??;

    let embed = build_embed(&data, entity, color);
    let components = build_components(&data);

    context.interaction_client()
        .update_response(inter.token.as_str())
        .embeds(Some(&[embed.build()]))
        .components(components.as_ref().map(|c| c.as_ref()))
        .into_future()
        .instrument(debug_span!("sending_response"))
        .await
        .map_err(expect_warn!("Failed to send the response to the user"))?;

    debug!("Successfully sent Response");

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
