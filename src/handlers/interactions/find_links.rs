/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use futures_util::future::try_join_all;
use std::future::IntoFuture;
use tracing::{Instrument, debug, debug_span, instrument, warn};
use twilight_model::application::interaction::Interaction;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::channel::message::MessageFlags;
use url::Url;

use crate::context::Ctx;
use crate::db::UsageData;
use crate::handlers::interactions::common::{
    VALID_DOMAINS_REGEX, additional_link_validation, build_components, data_routine,
};
use crate::handlers::interactions::messages;
use crate::handlers::interactions::messages::no_links_found;
use crate::util::EmptyResult;
use crate::util::error::expect_warn;
use crate::util::interaction::{defer, get_message, respond_with, update_defer_with_error};
use crate::util::odesli::ApiErr;

pub async fn handle(inter: Interaction, data: CommandData, context: Ctx) {
    // use an inner function to make splitting the code easier
    let _ = handle_inner(inter, data, context).await;
}

#[instrument(name = "find_links_command_handler", level = "debug", skip_all)]
async fn handle_inner(inter: Interaction, data: CommandData, context: Ctx) -> EmptyResult<()> {
    debug!("Received Find Links Command Interaction");

    let msg = get_message(&data)?;

    let links: Vec<Url> = msg
        .content
        .as_str()
        .split_whitespace()
        .filter(|s| s.starts_with("http://") || s.starts_with("https://"))
        .filter_map(|s| Url::parse(s).ok())
        .filter(|url| VALID_DOMAINS_REGEX.is_match(url.domain().unwrap_or_default()))
        .filter(|url| additional_link_validation(url).is_ok())
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
    let futures = links.into_iter().map(async |link| {
        let (data, entity, colour) = data_routine(&link, &context).await?;
        Ok::<_, ApiErr>((link.clone(), data, entity, colour))
    });

    let data = match try_join_all(futures).await {
        Ok(data) => data,
        Err(e) => {
            warn!(failed_with = %e, "Failed to fetch data for some of the links");
            update_defer_with_error(&inter, &context, messages::error((&inter.locale).into()))
                .await;
            return Err(());
        }
    };

    let mut usage_data = Vec::with_capacity(data.len());
    let mut components = Vec::with_capacity(data.len());

    for (idx, (link, data, entity, color)) in data.into_iter().enumerate() {
        usage_data.push(UsageData::from_find_links_command(
            &inter,
            link,
            &data.page_url,
            &entity,
            data.is_cached,
        ));
        components.extend(build_components(&data, entity, color, Some(idx as u16)));
    }

    let db_future = tokio::spawn(UsageData::save_multi_to_db(usage_data, context.clone()));

    defer_future
        .await
        .map_err(expect_warn!("Failed to join the defer future"))??;

    context
        .interaction_client()
        .update_response(inter.token.as_str())
        .flags(MessageFlags::IS_COMPONENTS_V2)
        .components(Some(&components))
        .into_future()
        .instrument(debug_span!("sending_response"))
        .await
        .map_err(expect_warn!("Failed to send the response to the user"))?;

    debug!("Successfully sent Response");

    db_future
        .await
        .map_err(expect_warn!("Failed to join the database future"))?;

    Ok(())
}
