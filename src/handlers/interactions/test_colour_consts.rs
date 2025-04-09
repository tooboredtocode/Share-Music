/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use std::future::IntoFuture;
use tracing::{debug, debug_span, instrument, Instrument};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};

use crate::commands::test_colour_consts::TestConstsCommandData;
use crate::context::Ctx;
use crate::util::colour::{get_dominant_colour, RGBPixel};
use crate::util::interaction::{defer, get_options, respond_with};
use crate::util::EmptyResult;
use crate::util::error::expect_warn;

pub async fn handle(inter: Interaction, data: CommandData, context: Ctx) {
    // use an inner function to make splitting the code easier
    let _ = handle_inner(inter, data, context).await;
}

#[instrument(name = "test_colour_consts_command_handler", level = "debug", skip_all)]
async fn handle_inner(inter: Interaction, data: CommandData, context: Ctx) -> EmptyResult<()> {
    debug!("Received Test Colour Const Command Interaction");

    let options: TestConstsCommandData = get_options(&data, &context).await?;

    let image_source = match ImageSource::url(&options.url) {
        Ok(source) => source,
        Err(_) => {
            debug!("URL is not valid, informing user");
            respond_with(&inter, &context, "Please provide a valid image url!").await;
            return Ok(());
        }
    };

    debug!("Deferring Response");
    let defer_future = defer(&inter, &context);

    debug!("Fetching Dominant Colour of Image");
    let colour = get_dominant_colour(&options.url, &context, (&options).into())
        .await
        .ok();

    defer_future
        .await
        .map_err(expect_warn!("Failed to join the defer future"))??;

    let embed = build_embed(image_source, colour);

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

fn build_embed(url: ImageSource, colour: Option<RGBPixel>) -> EmbedBuilder {
    let mut embed = EmbedBuilder::new().image(url);

    if let Some(colour) = colour {
        embed = embed.color(colour.to_hex());
    }

    embed
}
