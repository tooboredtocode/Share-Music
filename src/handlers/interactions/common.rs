/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use lazy_regex::{lazy_regex, Lazy};
use regex::Regex;
use tracing::{debug, instrument};
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource,
};
use crate::context::Ctx;
use crate::util::colour::{get_dominant_colour, RGBPixel};
use crate::util::odesli::{fetch_from_api, ApiErr, EntityData, OdesliResponse, Platform};

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

#[instrument(level = "debug", skip_all, fields(link = url))]
pub async fn data_routine(url: &str, context: &Ctx) -> Result<(OdesliResponse, EntityData, Option<RGBPixel>), ApiErr> {
    debug!("Fetching information from API");
    let mut data = fetch_from_api(url, context).await?;
    fix_platform_links(&mut data);

    let entity_data = data.get_data();
    debug!(
        "Got data from api: {} by {}",
        entity_data.title.as_ref().unwrap_or(&"<x>".into()),
        entity_data.artist_name.as_ref().unwrap_or(&"<x>".into())
    );

    let color = match &entity_data.thumbnail_url {
        Some(url) => {
            debug!("Album/Song has a Thumbnail, getting dominant colour");
            get_dominant_colour(url, context, Default::default())
                .await
                .ok()
        }
        None => None,
    };

    Ok((data, entity_data, color))
}

// Fixes the links for some platforms, so they work properly
fn fix_platform_links(resp: &mut OdesliResponse) {
    if let Some(links) = resp.links_by_platform.get_mut(&Platform::AppleMusic) {
        let new = links.url.replace("geo.music.apple.com", "music.apple.com");
        let mut new_it = new.split('?');

        let new = new_it.next().expect("A split should always return something");
        if let Some(query) = new_it.next() {
            let song_id = query.split('&').find(|s| s.starts_with("i="));
            if let Some(song_id) = song_id {
                links.url = format!("{}?{}", new, song_id);
            } else {
                links.url = format!("{}?{}", new, query);
            }
        } else {
            links.url = new.to_string();
        }
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
    links.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    embed = embed.description(
        links
            .iter()
            .map(|(platform, link)| format!("[{}]({})", platform, link))
            .collect::<Vec<String>>()
            .join(" | "),
    );

    embed
}
