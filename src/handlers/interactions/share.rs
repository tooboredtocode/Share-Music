/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use tracing::{debug, instrument};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;

use crate::commands::share::ShareCommandData;
use crate::context::Ctx;
use crate::handlers::interactions::{common, messages};
use crate::handlers::interactions::common::{map_odesli_response, VALID_LINKS_REGEX};
use crate::util::{EmptyResult, odesli};
use crate::util::colour::get_dominant_colour;
use crate::util::discord_locales::DiscordLocale;
use crate::util::error::Expectable;
use crate::util::interaction::{defer, get_options, respond_with};

pub async fn handle(inter: &Interaction, data: &CommandData, context: Ctx) {
    // use an inner function to make splitting the code easier
    let _ = handle_inner(inter, data, context).await;
}

#[instrument(
    name = "share_command_handler",
    level = "debug",
    skip_all
)]
async fn handle_inner(inter: &Interaction, data: &CommandData, context: Ctx) -> EmptyResult<()> {
    debug!("Received Share Command Interaction");

    let options = get_options(data, &context).await?;
    validate_url(&options, inter, &context).await?;

    debug!("User passed valid arguments, deferring Response");
    defer(inter, &context).await?;

    let data = map_odesli_response(
        odesli::fetch_from_api(&options.url, &context).await,
        &context,
        inter
    ).await?;

    let entity_data = data.get_data();
    let colour = match &entity_data.thumbnail_url {
        Some(url) => {
            debug!("Album/Song has a Thumbnail, getting dominant colour");
            get_dominant_colour(url, &context, Default::default()).await
        },
        None => None
    };

    let embed = common::build_embed(&data, entity_data, colour);

    let r = context.interaction_client()
        .create_followup(inter.token.as_str())
        .embeds(&[embed.build()])
        .unwrap()
        .exec()
        .await
        .warn_with("Failed to send the response to the user");

    if r.is_some() {
        debug!("Successfully sent Response");
    }

    Ok(())
}

pub async fn validate_url(options: &ShareCommandData, inter: &Interaction, context: &Ctx) -> EmptyResult<()> {
    if !VALID_LINKS_REGEX.is_match(options.url.as_str()) {
        debug!("URL is not valid, informing user");
        respond_with(inter, context, messages::invalid_url((&inter.locale).into())).await;
        Err(())
    } else {
        Ok(())
    }
}
