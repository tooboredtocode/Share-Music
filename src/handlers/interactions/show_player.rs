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

pub fn build_select_menu(data: &OdesliResponse) -> Option<Component> {
    let mut select_menu = SelectMenuBuilder::new(SELECT_ID.to_string(), SelectMenuType::Text)
        .placeholder("Show Embedded Player");

    let mut has_options = false;
    for (platform, links) in data
        .links_by_platform
        .iter()
        .filter(|(platform, _)| EMBEDDABLE_PLATFORMS.contains(platform))
    {
        let value = if links.url.len() <= 100 {
            links.url.clone()
        } else {
            format!("lookup_{:?}", platform)
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

    let link = match selected.as_str() {
        "lookup_AppleMusic" => find_link_for_platform(&inter, Platform::AppleMusic)?,
        "lookup_Spotify" => find_link_for_platform(&inter, Platform::Spotify)?,
        "lookup_AmazonMusic" => find_link_for_platform(&inter, Platform::AmazonMusic)?,
        "lookup_YouTube" => find_link_for_platform(&inter, Platform::YouTube)?,
        s => s,
    };

    debug!("Sending link to embed the player");
    respond_with(&inter, &context, link).await;

    Ok(())
}

fn find_link_for_platform(inter: &Interaction, platform: Platform) -> EmptyResult<&str> {
    let Some(message) = &inter.message else {
        warn!("Received Message Component Interaction without a message");
        return Err(());
    };

    let Some(embed) = message.embeds.first() else {
        warn!("Message from Select Player Interaction has no embeds");
        return Err(());
    };

    let Some(description) = &embed.description else {
        warn!("Embed from Select Player Interaction has no description");
        return Err(());
    };

    let Some(link) = description
        .split(" | ")
        .filter_map(platform_and_link_from_link)
        .find_map(
            |(plat, link)| {
                if plat == platform { Some(link) } else { None }
            },
        )
    else {
        warn!(
            "No link found for platform {:?} in embed description",
            platform
        );
        return Err(());
    };

    Ok(link)
}

fn platform_and_link_from_link(link: &str) -> Option<(Platform, &str)> {
    let mut split_iter = link.split(&['[', ']', '(', ')']).filter(|s| !s.is_empty());

    let platform = split_iter.next()?;
    let link = split_iter.next()?;

    match platform {
        "Apple Music" => Some((Platform::AppleMusic, link)),
        "Spotify" => Some((Platform::Spotify, link)),
        "Amazon Music" => Some((Platform::AmazonMusic, link)),
        "YouTube" => Some((Platform::YouTube, link)),
        _ => None,
    }
}
