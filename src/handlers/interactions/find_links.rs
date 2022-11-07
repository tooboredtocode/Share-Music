/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use futures_util::future::join_all;
use itertools::Itertools;
use tracing::{debug, instrument};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;

use crate::context::Ctx;
use crate::handlers::interactions::common;
use crate::handlers::interactions::common::{map_odesli_response, VALID_LINKS_REGEX};
use crate::handlers::interactions::messages::no_links_found;
use crate::util::colour::get_dominant_colour;
use crate::util::EmptyResult;
use crate::util::error::Expectable;
use crate::util::interaction::{defer, get_message, respond_with};
use crate::util::odesli::fetch_from_api;

pub async fn handle(inter: &Interaction, data: &CommandData, context: Ctx) {
    // use an inner function to make splitting the code easier
    let _ = handle_inner(inter, data, context).await;
}

#[instrument(
    name = "find_links_command_handler",
    level = "debug",
    skip_all
)]
async fn handle_inner(inter: &Interaction, data: &CommandData, context: Ctx) -> EmptyResult<()> {
    debug!("Received Find Links Command Interaction");

    let msg = get_message(data)?;

    let links: Vec<String> = VALID_LINKS_REGEX.find_iter(msg.content.as_str())
        .take(10)
        .map(|mat| mat.as_str().to_string())
        .unique()
        .take(5)
        .collect();

    if links.len() == 0 {
        debug!("Could not find any links, informing user");

        respond_with(inter, &context, no_links_found((&inter.locale).into())).await;
        return Err(())
    }

    debug!(
        links = ?links,
        "Found links in message, deferring Response"
    );
    defer(inter, &context).await?;

    let response = join_all(
        links.iter()
            .map(|link| fetch_from_api(link, &context))
    ).await;

    let mut data = Vec::with_capacity(response.len());
    for resp in response {
        data.push(map_odesli_response(resp, &context, inter).await?);
    }

    let embeds = join_all(
        data.iter()
            .map(|d| async {
                let entity_data = d.get_data();

                let colour = match &entity_data.thumbnail_url {
                    Some(url) => {
                        debug!("Album/Song has a Thumbnail, getting dominant colour");
                        get_dominant_colour(url, &context, Default::default()).await
                    },
                    None => None
                };

                common::build_embed(d, entity_data, colour).build()
            })
    ).await;

    let r = context.interaction_client()
        .create_followup(inter.token.as_str())
        .embeds(embeds.as_slice())
        .unwrap()
        .exec()
        .await
        .warn_with("Failed to send the response to the user");

    if r.is_some() {
        debug!("Successfully sent Response");
    }

    Ok(())
}

