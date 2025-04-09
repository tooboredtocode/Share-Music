/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use futures_util::future::try_join_all;
use itertools::Itertools;
use std::future::IntoFuture;
use tracing::{debug, debug_span, instrument, warn, Instrument};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;

use crate::context::Ctx;
use crate::handlers::interactions::common::{additional_link_validation, build_embed, data_routine, VALID_LINKS_REGEX};
use crate::handlers::interactions::messages;
use crate::handlers::interactions::messages::no_links_found;
use crate::util::interaction::{defer, get_message, respond_with, update_defer_with_error};
use crate::util::EmptyResult;
use crate::util::error::expect_warn;

pub async fn handle(inter: Interaction, data: CommandData, context: Ctx) {
    // use an inner function to make splitting the code easier
    let _ = handle_inner(inter, data, context).await;
}

#[instrument(name = "find_links_command_handler", level = "debug", skip_all)]
async fn handle_inner(inter: Interaction, data: CommandData, context: Ctx) -> EmptyResult<()> {
    debug!("Received Find Links Command Interaction");

    let msg = get_message(&data)?;

    let links: Vec<String> = VALID_LINKS_REGEX
        .find_iter(msg.content.as_str())
        .take(10)
        .map(|m| m.as_str().to_string())
        .unique()
        .filter(|s| additional_link_validation(s).is_ok())
        .take(5)
        .collect();

    if links.is_empty() {
        debug!("Could not find any links, informing user");

        respond_with(&inter, &context, no_links_found((&inter.locale).into())).await;
        return Err(());
    }

    debug!(
        links = ?links,
        "Found links in message, deferring Response"
    );
    let defer_future = defer(&inter, &context);

    debug!("Starting Routine for each link");
    let data = match try_join_all(links.iter().map(|link| data_routine(link, &context))).await {
        Ok(data) => data,
        Err(e) => {
            warn!(failed_with = %e, "Failed to fetch data for some of the links");
            update_defer_with_error(
                &inter,
                &context,
                messages::error((&inter.locale).into())
            ).await;
            return Err(());
        }
    };

    let embeds = data.into_iter().map(|(data, entity, color)|
        build_embed(&data, entity, color)
    )
        .map(|e| e.build())
        .collect_vec();

    defer_future
        .await
        .map_err(expect_warn!("Failed to join the defer future"))??;

    let r = context
        .interaction_client()
        .create_followup(inter.token.as_str())
        .embeds(embeds.as_slice())
        .into_future()
        .instrument(debug_span!("sending_response"))
        .await
        .map_err(expect_warn!("Failed to send the response to the user"));

    if r.is_ok() {
        debug!("Successfully sent Response");
    }

    Ok(())
}
