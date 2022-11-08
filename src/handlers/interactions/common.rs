/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::time::Duration;

use lazy_regex::{Lazy, lazy_regex};
use regex::Regex;
use tokio::time;
use twilight_model::application::interaction::Interaction;
use twilight_util::builder::embed::{EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource};

use crate::context::Ctx;
use crate::handlers::interactions::messages;
use crate::util::{EmptyResult, TerminationFuture};
use crate::util::colour::RGBPixel;
use crate::util::error::Expectable;
use crate::util::odesli::{ApiErr, EntityData, OdesliResponse};

// language=RegExp
pub static VALID_LINKS_REGEX: Lazy<Regex> = lazy_regex!(r#"(?x)
    (?:http|https)://
    .* # match all potential subdomains cause shit sucks
    (?:
        music\.amazon\.com| # Amazon
        deezer\.(?:page\.link|com)| # Deezer
        music\.apple\.com| # Apple Music & iTunes
        pandora\.com| # Pandora Music
        soundcloud\.com| # Soundcloud
        spotify\.com| # Spotify
        tidal\.com| # Tidal
        music\.yandex\.com| # Yandex
        youtu(?:\.be|be\.com)| # YouTube (Music)
        play\.google\.com # Google Store
    )
    \S*
"#);

pub fn build_embed(data: &OdesliResponse, entity: EntityData, colour: Option<RGBPixel>) -> EmbedBuilder {
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

pub async fn map_odesli_response(
    resp: Result<OdesliResponse, ApiErr>,
    context: &Ctx,
    inter: &Interaction
) -> EmptyResult<OdesliResponse> {
    match resp.warn_with("Failed to get the data from the api") {
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

            Err(())
        }
    }
}