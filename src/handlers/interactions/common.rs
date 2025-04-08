/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use std::future::IntoFuture;
use std::time::Duration;

use lazy_regex::{lazy_regex, Lazy};
use regex::Regex;
use tokio::time;
use tracing::{debug, debug_span, warn, Instrument};
use twilight_model::application::interaction::Interaction;
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource,
};

use crate::context::Ctx;
use crate::handlers::interactions::messages;
use crate::util::colour::{get_dominant_colour, RGBPixel};
use crate::util::odesli::{fetch_from_api, ApiErr, EntityData, OdesliResponse};
use crate::util::{create_termination_future, EmptyResult};
use crate::util::error::expect_warn;

// language=RegExp
pub static VALID_LINKS_REGEX: Lazy<Regex> = lazy_regex!(
    r#"(?x)
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
"#
);

/// The following enum contains all the possible ways a link can be invalid, but still pass the
/// above regex.
///
/// Using manual checks is easier and much easier to understand, than adjusting the regex to filter
/// out those cases
#[derive(Debug)]
pub enum InvalidLink {
    Playlist,
    Artist,
    YoutubeShort,
}

pub fn additional_link_validation(link: &str) -> Result<(), InvalidLink> {
    if link.contains("/playlist") {
        return Err(InvalidLink::Playlist);
    }

    if link.contains("/artist") {
        return Err(InvalidLink::Artist);
    }

    if link.contains("youtube.com/shorts") {
        return Err(InvalidLink::YoutubeShort);
    }

    Ok(())
}

pub async fn log_odesli_error(
    err: ApiErr,
    context: &Ctx,
    inter: &Interaction,
) {
    warn!(failed_with = %err, "Failed to get the data from the api");

    if let Ok(msg_resp) = context
        .interaction_client()
        .create_followup(inter.token.as_str())
        .content(messages::error((&inter.locale).into()))
        .into_future()
        .instrument(debug_span!("sending_error_message"))
        .await
        .map_err(expect_warn!("Failed to inform user of the error"))
    {
        let msg = match msg_resp.model().await {
            Ok(ok) => ok,
            Err(_) => return,
        };

        let ctx = context.clone();

        tokio::spawn(async move {
            let _ = time::timeout(
                Duration::from_secs(15),
                create_termination_future(&ctx.state),
            )
                .await;

            ctx.discord_client
                .delete_message(msg.channel_id, msg.id)
                .into_future()
                .instrument(debug_span!("deleting_error_message"))
                .await
                .map_err(expect_warn!("Failed to delete Error Message"))
        });
    }
}

pub fn build_embed(
    data: &OdesliResponse,
    entity: EntityData,
    colour: Option<RGBPixel>,
) -> EmbedBuilder {
    let mut embed = EmbedBuilder::new();
    let EntityData {
        title,
        artist_name,
        thumbnail_url,
    } = entity;

    if let Some(title) = title {
        embed = embed.title(title).url(&data.page_url)
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
        links
            .iter()
            .map(|(platform, link)| format!("[{}]({})", platform, link))
            .collect::<Vec<String>>()
            .join(" | "),
    );

    embed
}

// NOTE: This future is not instrumented as it should be instrumented in the calling function
pub async fn embed_routine(
    url: &String,
    context: &Ctx,
    inter: &Interaction,
) -> EmptyResult<EmbedBuilder> {
    debug!("Fetching information from API");
    let data = match fetch_from_api(url, context).await {
        Ok(data) => data,
        Err(err) => {
            log_odesli_error(err, context, inter).await;
            return Err(())
        }
    };

    let entity_data = data.get_data();
    debug!(
        "Got data from api: {} by {}",
        entity_data.title.as_ref().unwrap_or(&"<x>".into()),
        entity_data.artist_name.as_ref().unwrap_or(&"<x>".into())
    );

    let colour = match &entity_data.thumbnail_url {
        Some(url) => {
            debug!("Album/Song has a Thumbnail, getting dominant colour");
            get_dominant_colour(url, context, Default::default())
                .await
                .ok()
        }
        None => None,
    };

    Ok(build_embed(&data, entity_data, colour))
}
