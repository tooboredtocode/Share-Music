/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use crate::context::Ctx;
use crate::handlers::interactions::messages;
use crate::util::EmptyResult;
use crate::util::interaction::respond_with;
use crate::util::odesli::{OdesliResponse, Platform};
use tracing::{debug, instrument, warn};
use twilight_model::application::interaction::Interaction;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::channel::message::Component;
use twilight_model::channel::message::component::SelectMenuType;
use twilight_util::builder::message::{
    ActionRowBuilder, SelectMenuBuilder, SelectMenuOptionBuilder,
};

pub const SELECT_ID: &str = "odesli_select";

pub const EMBEDDABLE_PLATFORMS: &[Platform] = &[
    Platform::AppleMusic,
    Platform::Spotify,
    Platform::AmazonMusic,
    Platform::YouTube,
];

pub fn build_select_menu(data: &OdesliResponse, idx: Option<u16>) -> Option<Component> {
    let custom_id = idx
        .map(|i| format!("{}_{}", SELECT_ID, i))
        .unwrap_or_else(|| SELECT_ID.to_string());

    let mut select_menu =
        SelectMenuBuilder::new(custom_id, SelectMenuType::Text).placeholder("Show Embedded Player");

    let mut has_options = false;
    for (platform, links) in data
        .links_by_platform
        .iter()
        .filter(|(platform, _)| EMBEDDABLE_PLATFORMS.contains(platform))
    {
        let value = if links.url.len() <= 100 {
            links.url.clone()
        } else {
            debug!(
                link = links.url,
                "Link for platform {} is too long to embed, skipping", platform
            );
            continue;
        };

        select_menu =
            select_menu.option(SelectMenuOptionBuilder::new(platform.to_string(), value).build());
        has_options = true;
    }

    if !has_options {
        debug!("No embeddable platforms found, not sending select menu");
        return None;
    }

    Some(Component::ActionRow(
        ActionRowBuilder::new()
            .component(select_menu.build())
            .build(),
    ))
}

pub async fn handle(inter: Interaction, data: MessageComponentInteractionData, context: Ctx) {
    // use an inner function to make splitting the code easier
    let _ = handle_inner(inter, data, context).await;
}

#[instrument(name = "select_show_player_handler", level = "debug", skip_all)]
async fn handle_inner(
    inter: Interaction,
    data: MessageComponentInteractionData,
    context: Ctx,
) -> EmptyResult<()> {
    debug!("Received Show Player Select Menu Interaction");

    let Some(selected) = data.values.first() else {
        warn!("No values selected in Select Menu");
        respond_with(&inter, &context, messages::error((&inter.locale).into())).await;
        return Err(());
    };

    if selected.starts_with("lookup_") {
        warn!("Selected value is a depreciated lookup link, cannot show embedded player");
        respond_with(
            &inter,
            &context,
            messages::select_menu_with_depreciated_lookup_link((&inter.locale).into()),
        )
        .await;
        return Err(());
    }

    debug!("Sending link to embed the player");
    respond_with(&inter, &context, selected).await;

    Ok(())
}
