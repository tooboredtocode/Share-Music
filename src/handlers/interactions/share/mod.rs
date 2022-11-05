/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use lazy_regex::regex;
use regex::Regex;
use tracing::{debug, instrument};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::MessageFlags;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::embed::{EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource};
use twilight_util::builder::InteractionResponseDataBuilder;
use tokio::time;
use std::time::Duration;

use crate::commands::share::ShareCommandData;
use crate::context::Ctx;
use crate::util::odesli::{EntityData, OdesliResponse};
use crate::TerminationFuture;
use crate::util::colour::{get_dominant_colour, RGBPixel};
use crate::util::{EmptyResult, odesli};
use crate::util::error::Expectable;
use crate::util::interaction::{defer, get_options};

mod messages;

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

    let data = fetch_data_from_api(&options, &context, inter).await?;

    let entity_data = data.get_data();
    let colour = match &entity_data.thumbnail_url {
        Some(url) => {
            debug!("Album/Song has a Thumbnail, getting dominant colour");
            get_dominant_colour(url, &context, Default::default()).await
        },
        None => None
    };

    let embed = build_embed(data, entity_data, colour);

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

async fn validate_url(options: &ShareCommandData, inter: &Interaction, context: &Ctx) -> EmptyResult<()> {
	    let regex: &Regex = regex!(
        r#"https://(?:.*amazon\.com|.*deezer\.com|.*deezer\.page\.link|.*music\.apple\.com|.*pandora.*\.com|soundcloud\.com|.*spotify\.com|.*tidal\.com|.*music\.yandex\..{1,3}|.*youtu(?:\.be|be\.com))"#
    );

    if !regex.is_match(options.url.as_str()) {
        debug!("URL is not valid, informing user");

        context.interaction_client()
            .create_response(
                inter.id,
                inter.token.as_str(),
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: InteractionResponseDataBuilder::new()
                        .content(messages::invalid_url((&inter.locale).into()))
                        .flags(MessageFlags::EPHEMERAL)
                        .build()
                        .into()
                }
            )
            .exec()
            .await
            .warn_with("Failed to respond to the Interaction");

        return Err(());
    }

    Ok(())
}

fn build_embed(data: OdesliResponse, entity: EntityData, colour: Option<RGBPixel>) -> EmbedBuilder {
    let mut embed = EmbedBuilder::new();
    let EntityData {
        title,
        artist_name,
        thumbnail_url,
    } = entity;

    if let Some(title) = title {
        embed = embed
            .title(title)
            .url(&data.page_url)
    }

    if let Some(artist_name) = artist_name {
        embed = embed.author(EmbedAuthorBuilder::new(artist_name))
    }

    if let Some(thumbnail_url) = thumbnail_url {
        if let Ok(src) = ImageSource::url(thumbnail_url) {
            embed = embed.thumbnail(src)
        }
    }

    if let Some(colour) = colour {
        embed = embed.color(colour.to_hex());
    }

    embed = embed.footer(EmbedFooterBuilder::new("Powered by odesli.co"));

    let mut links = data.links();
    links.sort_by(|a, b| a.0.cmp(&b.0));

    embed = embed.description(
        links.iter()
            .map(|(platform, link)| format!("[{}]({})", platform, link))
            .collect::<Vec<String>>()
            .join(" | ")
    );

    embed
}

pub async fn fetch_data_from_api(options: &ShareCommandData, context: &Ctx, inter: &Interaction) -> Result<OdesliResponse, ()> {
    debug!("Fetching Data from Odesli");
    match odesli::fetch_from_api(&options, &context)
        .await
        .warn_with("Failed to get the data from the api")
    {
        Some(s) => Ok(s),
        None => {
            match context.interaction_client()
                .create_followup(inter.token.as_str())
                .content(messages::error((&inter.locale).into()))
                .unwrap() // this is safe as we use static strings that are below the max size
                .exec()
                .await
                .warn_with("Failed to inform user of the error")
            {
                Some(msg_resp) => {
                    let msg = match msg_resp.model().await {
                        Ok(ok) => ok,
                        Err(_) => return Err(())
                    };

                    let ctx = context.clone();

                    tokio::spawn(async move {
                        let _ = time::timeout(
                            Duration::from_secs(15),
                            TerminationFuture::new(ctx.create_state_listener())
                        ).await;

                        ctx.discord_client
                            .delete_message(msg.channel_id, msg.id)
                            .exec()
                            .await
                            .warn_with("Failed to delete Error Message")
                    });
                },
                None => {}
            }

            return Err(());
        }
    }
}
