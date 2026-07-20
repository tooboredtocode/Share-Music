/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use std::future::IntoFuture;

use tracing::{Instrument, debug, debug_span, instrument, warn};
use twilight_model::application::interaction::Interaction;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::channel::message::MessageFlags;
use url::Url;

use crate::clients::odesli::ApiErr;
use crate::db::UsageData;
use crate::interactions::InteractionsHandler;
use crate::interactions::commands::share::ShareCommand;
use crate::interactions::handlers::common::{
    InvalidLink, VALID_DOMAINS_REGEX, additional_link_validation, build_components,
};
use crate::interactions::handlers::messages;
use crate::util::EmptyResult;
use crate::util::error::expect_warn;

impl InteractionsHandler {
    pub(super) async fn handle_share(&self, inter: Interaction, data: CommandData) {
        // use an inner function to make splitting the code easier
        let _ = handle_inner(self, inter, data).await;
    }
}

#[instrument(name = "share_command_handler", level = "debug", skip_all)]
async fn handle_inner(
    this: &InteractionsHandler,
    inter: Interaction,
    data: CommandData,
) -> EmptyResult<()> {
    debug!("Received Share Command Interaction");

    let command = this.parse_command(data)?;
    let url = validate_url(this, &inter, &command).await?;

    debug!("User passed valid arguments, deferring Response");
    let defer_future = this.defer(&inter);

    let (data, entity, color) = match this.data_routine(&url).await {
        Ok(data) => data,
        Err(e) => {
            let message = match e {
                ApiErr::ClientError(err) => {
                    debug!(
                        "Odesli API returned a client error, informing user: {}",
                        err
                    );
                    messages::api_client_error_message(err, (&inter.locale).into())
                }
                _ => {
                    warn!("Odesli API request failed, informing user: {}", e);
                    messages::error((&inter.locale).into())
                }
            };
            this.update_defer_with_error(&inter, message).await;
            return Err(());
        }
    };

    let usage_data =
        UsageData::from_share_command(&inter, url, &data.page_url, &entity, data.is_cached);

    // No need to pass an index since we only have one link, and thus one component
    let components = build_components(&data, entity, color, None);

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
    this.db().spawn_save_to_db(usage_data);

    Ok(())
}

async fn validate_url(
    this: &InteractionsHandler,
    inter: &Interaction,
    cmd: &ShareCommand,
) -> EmptyResult<Url> {
    let url = match Url::parse(cmd.url.as_str()) {
        Ok(url) => url,
        Err(_) => {
            debug!("URL is not valid, informing user");
            this.respond_with(inter, messages::invalid_url((&inter.locale).into()))
                .await;
            return Err(());
        }
    };

    match url.domain() {
        Some(domain) if VALID_DOMAINS_REGEX.is_match(domain) => (),
        _ => {
            debug!("URL domain is not supported, informing user");
            this.respond_with(inter, messages::invalid_url((&inter.locale).into()))
                .await;
            return Err(());
        }
    }

    if let Err(reason) = additional_link_validation(&url) {
        match reason {
            InvalidLink::Playlist => {
                debug!("URL is a playlist, informing user");
                this.respond_with(
                    inter,
                    messages::playlist_not_supported((&inter.locale).into()),
                )
                .await;
            }
            InvalidLink::Artist => {
                debug!("URL is an artist, informing user");
                this.respond_with(
                    inter,
                    messages::artist_not_supported((&inter.locale).into()),
                )
                .await;
            }
            InvalidLink::YoutubeShort => {
                debug!("URL is a shorts video, informing user");
                this.respond_with(
                    inter,
                    messages::youtube_shorts_not_supported((&inter.locale).into()),
                )
                .await;
            }
        }
        return Err(());
    }

    debug!(url = %url, "Successfully validated URL, proceeding to fetch data from Odesli API");
    Ok(url)
}
