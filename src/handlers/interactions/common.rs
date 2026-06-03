/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use crate::context::Ctx;
use crate::handlers::interactions::show_player::build_select_menu;
use crate::util::colour::RGBPixel;
use crate::util::odesli::{ApiErr, EntityData, OdesliResponse, Platform};
use lazy_regex::{Lazy, lazy_regex};
use regex::Regex;
use tracing::{debug, instrument};
use twilight_model::channel::message::Component;
use twilight_model::channel::message::component::UnfurledMediaItem;
use twilight_util::builder::message::{
    ContainerBuilder, SectionBuilder, TextDisplayBuilder, ThumbnailBuilder,
};

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
pub async fn data_routine(
    url: &str,
    context: &Ctx,
) -> Result<(OdesliResponse, EntityData, Option<RGBPixel>), ApiErr> {
    debug!("Fetching information from API");
    let mut data = context.odesli_client.fetch(url).await?;
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
            context
                .image_client
                .get_dominant_colour_from_url(url, Default::default())
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
        let mut new_iter = new.split('?');

        let new = new_iter
            .next()
            .expect("A split should always return something");
        if let Some(query) = new_iter.next() {
            let song_id = query.split('&').find(|s| s.starts_with("i="));
            if let Some(song_id) = song_id {
                links.url = format!("{}?{}", new, song_id);
            } else {
                // Just return the album link
                links.url = new.to_string();
            }
        } else {
            links.url = new.to_string();
        }
    }
}

fn unfurled_media_item_from_url(url: String) -> UnfurledMediaItem {
    UnfurledMediaItem {
        url,
        width: None,
        height: None,
        proxy_url: None,
        content_type: None,
    }
}

pub fn build_components(
    data: &OdesliResponse,
    entity: EntityData,
    colour: Option<RGBPixel>,
    idx: Option<u16>,
) -> [Component; 1] {
    use std::fmt::Write;

    let mut container = ContainerBuilder::new();
    if let Some(colour) = colour {
        container = container.accent_color(Some(colour.to_hex()));
    }

    let EntityData {
        title,
        artist_name,
        thumbnail_url,
        ..
    } = entity;

    let artist_details = artist_name.as_ref().map(|artist| format!("**{}**", artist));

    let mut details = String::new();
    if let Some(title) = title {
        writeln!(details, "## [{}]({})", title, data.page_url)
            .expect("Writing to string should not fail");
    }

    let mut links = data.links();
    links.sort_by_key(|a| a.0.to_lowercase());

    for (i, (platform, link)) in links.iter().enumerate() {
        if i > 0 {
            details.push_str("  \u{2022}  "); // 2 tabs with a bullet in the middle
        }

        write!(details, "[{}]({})", platform, link).expect("Writing to string should not fail");
    }

    if let Some(thumbnail_url) = thumbnail_url {
        let thumbnail = ThumbnailBuilder::new(unfurled_media_item_from_url(thumbnail_url)).build();
        let mut section_builder = SectionBuilder::new(thumbnail);

        if let Some(artist_details) = artist_details {
            section_builder =
                section_builder.component(TextDisplayBuilder::new(artist_details).build());
        }
        section_builder = section_builder.component(TextDisplayBuilder::new(details).build());

        container = container.component(section_builder.build());
    } else {
        if let Some(artist_details) = artist_details {
            container = container.component(TextDisplayBuilder::new(artist_details).build());
        }
        container = container.component(TextDisplayBuilder::new(details).build());
    };

    if let Some(show_platform_players) = build_select_menu(data, idx) {
        container = container.component(show_platform_players);
    }

    container = container.component(TextDisplayBuilder::new("-# Powered by odesli.co").build());

    [container.build().into()]
}
