/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use futures_util::future::try_join_all;
use itertools::Itertools;
use std::future::IntoFuture;
use tracing::{debug, debug_span, instrument, span, Instrument, Level};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;

use crate::context::Ctx;
use crate::handlers::interactions::common::{embed_routine, VALID_LINKS_REGEX};
use crate::handlers::interactions::messages::no_links_found;
use crate::util::error::Expectable;
use crate::util::interaction::{defer, get_message, respond_with};
use crate::util::EmptyResult;

pub async fn handle(inter: &Interaction, data: &CommandData, context: Ctx) {
    // use an inner function to make splitting the code easier
    let _ = handle_inner(inter, data, context).await;
}

#[instrument(name = "find_links_command_handler", level = "debug", skip_all)]
async fn handle_inner(inter: &Interaction, data: &CommandData, context: Ctx) -> EmptyResult<()> {
    debug!("Received Find Links Command Interaction");

    let msg = get_message(data)?;

    let links: Vec<String> = VALID_LINKS_REGEX
        .find_iter(msg.content.as_str())
        .take(10)
        .map(|mat| mat.as_str().to_string())
        .unique()
        .take(5)
        .collect();

    if links.len() == 0 {
        debug!("Could not find any links, informing user");

        respond_with(inter, &context, no_links_found((&inter.locale).into())).await;
        return Err(());
    }

    debug!(
        links = ?links,
        "Found links in message, deferring Response"
    );
    let defer_future = defer(inter, &context);

    debug!("Starting Routine for each link");
    let embeds = try_join_all(links.iter().map(|link| {
        embed_routine(link, &context, inter).instrument(span!(
            Level::DEBUG,
            "embed_routine",
            link = link
        ))
    }))
    .await?
    .into_iter()
    .map(|e| e.build())
    .collect_vec();

    defer_future
        .await
        .warn_with("Failed to join the defer future")
        .ok_or(())??;

    let r = context
        .interaction_client()
        .create_followup(inter.token.as_str())
        .embeds(embeds.as_slice())
        .expect("Somehow more than 10 embeds were created, this should never happen")
        .into_future()
        .instrument(debug_span!("sending_response"))
        .await
        .warn_with("Failed to send the response to the user");

    if r.is_some() {
        debug!("Successfully sent Response");
    }

    Ok(())
}
