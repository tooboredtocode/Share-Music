/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use tracing::{debug, instrument};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};

use crate::commands::test_colour_consts::TestConstsCommandData;
use crate::context::Ctx;
use crate::util::colour::{get_dominant_colour, RGBPixel};
use crate::util::EmptyResult;
use crate::util::error::Expectable;
use crate::util::interaction::{defer, get_options};

pub async fn handle(inter: &Interaction, data: &CommandData, context: Ctx) {
    // use an inner function to make splitting the code easier
    let _ = handle_inner(inter, data, context).await;
}

#[instrument(
    name = "test_colour_consts_command_handler",
    level = "debug",
    skip_all
)]
async fn handle_inner(inter: &Interaction, data: &CommandData, context: Ctx) -> EmptyResult<()> {
    debug!("Received Test Colour Const Command Interaction");

    let options: TestConstsCommandData = get_options(data, &context).await?;

    debug!("Deferring Response");
    defer(inter, &context).await?;

    debug!("Fetching Dominant Colour of Image");
    let colour = get_dominant_colour(&options.url, &context, (&options).into()).await;

    let embed = build_embed(&options.url, colour);

    let r = context.interaction_client()
        .create_followup(inter.token.as_str())
        .embeds(&[embed.build()])
        .expect("Somehow we built an invalid embed, this should never happen")
        .await
        .warn_with("Failed to send the response to the user");

    if r.is_some() {
        debug!("Successfully sent Response");
    }

    Ok(())
}

fn build_embed(url: &String, colour: Option<RGBPixel>) -> EmbedBuilder {
    let mut embed = EmbedBuilder::new();

    if let Ok(src) = ImageSource::url(url) {
        embed = embed.image(src)
    }

    if let Some(colour) = colour {
        embed = embed.color(colour.to_hex());
    }

    embed
}