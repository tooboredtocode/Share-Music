/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use lazy_regex::{Lazy, lazy_regex};
use regex::Regex;
use tracing::{debug, instrument};
use twilight_model::channel::message::Component;
use twilight_model::channel::message::component::UnfurledMediaItem;
use twilight_util::builder::message::{
    ContainerBuilder, SectionBuilder, TextDisplayBuilder, ThumbnailBuilder,
};
use url::Url;

use crate::clients::colour::RGBPixel;
use crate::clients::odesli::{ApiErr, EntityData, OdesliClientResponse, OdesliResponse};
use crate::interactions::InteractionsHandler;
use crate::interactions::handlers::show_player::build_select_menu;

// language=RegExp
pub static VALID_DOMAINS_REGEX: Lazy<Regex> = lazy_regex!(
    r#"(?x)
    .* # match all potential subdomains cause shit sucks
    (?:
        music\.amazon\.com| # Amazon
        play\.anghami\.com| # Anghami
        deezer\.(?:page\.link|com)| # Deezer
        music\.apple\.com| # Apple Music & iTunes
        napster\.com| # Napster
        pandora\.com| # Pandora Music
        soundcloud\.com| # Soundcloud
        spotify\.com| # Spotify
        tidal\.com| # Tidal
        music\.yandex\.(?:ru|com)| # Yandex
        youtu(?:\.be|be\.com)| # YouTube (Music)
        play\.google\.com # Google Store
    )$
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

pub fn additional_link_validation(link: &Url) -> Result<(), InvalidLink> {
    if link.path().contains("/playlist") {
        return Err(InvalidLink::Playlist);
    }

    if link.path().contains("/artist") {
        return Err(InvalidLink::Artist);
    }

    if link.as_str().contains("youtube.com/shorts") {
        return Err(InvalidLink::YoutubeShort);
    }

    Ok(())
}

impl InteractionsHandler {
    #[instrument(level = "debug", skip_all, fields(link = %url))]
    pub(super) async fn data_routine(
        &self,
        url: &Url,
    ) -> Result<(OdesliClientResponse, EntityData, Option<RGBPixel>), ApiErr> {
        debug!("Fetching information from API");
        let data = self.odesli().fetch(url).await?;
        let entity_data = data.get_data();
        debug!(
            "Got data from api: {} by {}",
            entity_data.title.as_ref().unwrap_or(&"<x>".into()),
            entity_data.artist_name.as_ref().unwrap_or(&"<x>".into())
        );

        let color = match &entity_data.thumbnail_url {
            Some(url) => {
                debug!("Album/Song has a Thumbnail, getting dominant colour");
                self.image()
                    .get_dominant_colour_from_url(url, Default::default())
                    .await
                    .ok()
            }
            None => None,
        };

        Ok((data, entity_data, color))
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
