/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use std::future::IntoFuture;

use tracing::{Instrument, debug, debug_span, instrument};
use twilight_model::application::interaction::Interaction;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};

use crate::clients::colour::RGBPixel;
use crate::interactions::InteractionsHandler;
use crate::interactions::commands::test_colour_consts::TestColorConstsCommand;
use crate::util::EmptyResult;
use crate::util::error::expect_warn;

impl InteractionsHandler {
    pub(super) async fn handle_test_colour_consts(&self, inter: Interaction, data: CommandData) {
        // use an inner function to make splitting the code easier
        let _ = handle_inner(self, inter, data).await;
    }
}

#[instrument(name = "test_colour_consts_command_handler", level = "debug", skip_all)]
async fn handle_inner(
    this: &InteractionsHandler,
    inter: Interaction,
    data: CommandData,
) -> EmptyResult<()> {
    debug!("Received Test Colour Const Command Interaction");

    let command = this.parse_command::<TestColorConstsCommand>(data)?;

    let image_source = match ImageSource::url(&command.url) {
        Ok(source) => source,
        Err(_) => {
            debug!("URL is not valid, informing user");
            this.respond_with(&inter, "Please provide a valid image url!")
                .await;
            return Ok(());
        }
    };

    debug!("Deferring Response");
    let defer_future = this.defer(&inter);

    debug!("Fetching Dominant Colour of Image");
    let colour = this
        .image()
        .get_dominant_colour_from_url(&command.url, (&command).into())
        .await
        .ok();

    defer_future
        .await
        .map_err(expect_warn!("Failed to join the defer future"))?;

    let embed = build_embed(image_source, colour);

    let r = this
        .discord()
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
