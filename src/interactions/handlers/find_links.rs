/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use std::fmt;
use std::fmt::Write;
use std::future::IntoFuture;

use futures_util::future::try_join_all;
use tracing::{Instrument, debug, debug_span, instrument, warn};
use twilight_model::application::interaction::Interaction;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::channel::message::MessageFlags;
use url::Url;

use crate::clients::odesli::ApiErr;
use crate::db::UsageData;
use crate::interactions::InteractionsHandler;
use crate::interactions::handlers::common::{
    VALID_DOMAINS_REGEX, additional_link_validation, build_components,
};
use crate::interactions::handlers::messages;
use crate::util::EmptyResult;
use crate::util::error::expect_warn;
use crate::util::message_command::get_message;

impl InteractionsHandler {
    pub(super) async fn handle_find_links(&self, inter: Interaction, data: CommandData) {
        // use an inner function to make splitting the code easier
        let _ = handle_inner(self, inter, data).await;
    }
}

#[instrument(name = "find_links_command_handler", level = "debug", skip_all)]
async fn handle_inner(
    this: &InteractionsHandler,
    inter: Interaction,
    data: CommandData,
) -> EmptyResult<()> {
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

        this.respond_with(&inter, messages::no_links_found((&inter.locale).into()))
            .await;
        return Err(());
    }

    debug!(
        links = %LoggerLinks(&links),
        "Found links in message, deferring Response"
    );
    let defer_future = this.defer(&inter);

    debug!("Starting Routine for each link");
    let futures = links
        .into_iter()
        .map(async |link| match this.data_routine(&link).await {
            Ok((data, entity, colour)) => Ok(Some((link.clone(), data, entity, colour))),
            Err(ApiErr::ClientError(e)) => {
                debug!(
                    "Odesli API returned a client error for link {}, skipping it: {}",
                    link, e
                );
                Ok(None)
            }
            Err(e) => Err(e),
        });

    let data = match try_join_all(futures).await {
        Ok(data) => data,
        Err(e) => {
            warn!("Odesli API request failed, informing user: {}", e);
            this.update_defer_with_error(&inter, messages::error((&inter.locale).into()))
                .await;
            return Err(());
        }
    };

    let mut usage_data = Vec::with_capacity(data.len());
    let mut components = Vec::with_capacity(data.len());

    for (idx, (link, data, entity, color)) in data.into_iter().flatten().enumerate() {
        usage_data.push(UsageData::from_find_links_command(
            &inter,
            link,
            &data.page_url,
            &entity,
            data.is_cached,
        ));
        components.extend(build_components(&data, entity, color, Some(idx as u16)));
    }

    defer_future
        .await
        .map_err(expect_warn!("Failed to join the defer future"))?;

    this.discord()
        .interaction_client()
        .update_response(inter.token.as_str())
        .flags(MessageFlags::IS_COMPONENTS_V2)
        .components(Some(&components))
        .into_future()
        .instrument(debug_span!("sending_response"))
        .await
        .map_err(expect_warn!("Failed to send the response to the user"))?;

    debug!("Successfully sent Response, spawning task to save command usage data to the database");
    this.db().spawn_save_multi_to_db(usage_data);

    Ok(())
}

struct LoggerLinks<'a>(&'a [Url]);

impl<'a> fmt::Display for LoggerLinks<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('[')?;
        let mut iter = self.0.iter();
        if let Some(first) = iter.next() {
            f.write_str(first.as_str())?;
        }
        for url in iter {
            f.write_str(", ")?;
            f.write_str(url.as_str())?;
        }
        f.write_char(']')
    }
}
