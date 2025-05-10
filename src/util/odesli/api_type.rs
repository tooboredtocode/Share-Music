/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Display, Formatter};
use tracing::warn;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OdesliResponse {
    /// The unique ID for the input entity that was supplied in the request. The
    /// data for this entity, such as title, artistName, etc. will be found in
    /// an object at `nodesByUniqueId[entityUniqueId]`
    pub entity_unique_id: String,

    /// The userCountry query param that was supplied in the request. It signals
    /// the country/availability we use to query the streaming platforms. Defaults
    /// to 'US' if no userCountry supplied in the request.
    ///
    /// NOTE: As a fallback, our service may respond with matches that were found
    /// in a locale other than the userCountry supplied
    pub user_country: String,

    /// A URL that will render the Songlink page for this entity
    pub page_url: String,

    /// A collection of objects. Each key is a platform, and each value is an
    ///  object that contains data for linking to the match
    ///
    /// Each key in `linksByPlatform` is a Platform. A Platform will exist here
    /// only if there is a match found. E.g. if there is no YouTube match found,
    /// then neither `youtube` or `youtubeMusic` properties will exist here
    pub links_by_platform: HashMap<Platform, Links>,

    /// A collection of objects. Each key is a unique identifier for a streaming
    /// entity, and each value is an object that contains data for that entity,
    /// such as `title`, `artistName`, `thumbnailUrl`, etc.
    pub entities_by_unique_id: HashMap<String, Entity>,
}

pub struct EntityData {
    pub title: Option<String>,
    pub artist_name: Option<String>,
    pub thumbnail_url: Option<String>,
}

impl OdesliResponse {
    pub fn links(&self) -> Vec<(String, String)> {
        self.links_by_platform
            .iter()
            .filter(|(platform, _)| !matches!(platform, Platform::Other(_)))
            .map(|(p, l)| (p.to_string(), l.url.clone()))
            .collect()
    }

    pub fn get_data(&self) -> EntityData {
        let mut res = self
            .entities_by_unique_id
            .get(&self.entity_unique_id)
            .map(|e| EntityData {
                title: e.title.clone(),
                artist_name: e.artist_name.clone(),
                thumbnail_url: e.thumbnail_url.clone(),
            })
            .unwrap_or_else(|| {
                warn!(
                    "API returned response without data for original entity: {}",
                    self.entity_unique_id
                );
                // Maybe some prioritized entity has data
                EntityData {
                    title: None,
                    artist_name: None,
                    thumbnail_url: None,
                }
            });

        let mut curr_max = APIProvider::min_prio();
        let max_prio = self
            .entities_by_unique_id
            .iter()
            .map(|(_, e)| e.api_provider.prio())
            .min()
            .unwrap_or(APIProvider::max_prio());

        for entity in self.entities_by_unique_id.values() {
            let prio = entity.api_provider.prio();
            if prio <= curr_max {
                continue;
            }

            let Entity {
                title,
                artist_name,
                thumbnail_url,
                ..
            } = entity;

            if [title, artist_name, thumbnail_url]
                .iter()
                .any(|i| i.is_none())
            {
                continue;
            }

            res = EntityData {
                title: title.clone(),
                artist_name: artist_name.clone(),
                thumbnail_url: thumbnail_url.clone(),
            };

            curr_max = prio;
            if curr_max == max_prio {
                break;
            }
        }

        res
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Links {
    /// The unique ID for this entity. Use it to look up data about this entity
    /// at `entitiesByUniqueId[entityUniqueId]`
    pub entity_unique_id: String,

    /// The URL for this match
    pub url: String,

    /// The native app URI that can be used on mobile devices to open this
    /// entity directly in the native app
    pub native_app_uri_mobile: Option<String>,

    /// The native app URI that can be used on desktop devices to open this
    /// entity directly in the native app
    pub native_app_uri_desktop: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    /// This is the unique identifier on the streaming platform/API provider
    #[serde(deserialize_with = "deserialize_potential_int_to_string")]
    pub id: String,

    #[serde(rename = "type")]
    pub kind: String,

    pub title: Option<String>,
    pub artist_name: Option<String>,
    pub thumbnail_url: Option<String>,
    pub thumbnail_width: Option<u16>,
    pub thumbnail_height: Option<u16>,

    /// The API provider that powered this match. Useful if you'd like to use
    /// this entity's data to query the API directly
    pub api_provider: APIProvider,

    /// An array of platforms that are "powered" by this entity. E.g. an entity
    /// from Apple Music will generally have a `platforms` array of
    /// `["appleMusic", "itunes"]` since both those platforms/links are derived
    /// from this single entity
    pub platforms: Vec<Platform>,
}

// For some reason song_link returns bandcamp links as ids so aaaaaaaaa
fn deserialize_potential_int_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Number(n) => Ok(n.to_string()),
        Value::String(s) => Ok(s),
        _ => Err(serde::de::Error::custom("expected number or string")),
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    Song,
    Album,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Platform {
    Spotify,
    #[allow(non_camel_case_types)]
    #[serde(rename = "itunes")]
    iTunes,
    AppleMusic,
    #[serde(rename = "youtube")]
    YouTube,
    #[serde(rename = "youtubeMusic")]
    YouTubeMusic,
    Google,
    GoogleStore,
    Pandora,
    Deezer,
    Tidal,
    AmazonStore,
    AmazonMusic,
    Soundcloud,
    Napster,
    Spinrilla,
    Audius,
    Audiomack,
    Anghami,
    Yandex,
    #[serde(rename = "boomplay")]
    BoomPlay,
    #[serde(untagged)]
    Other(String),
}

impl Display for Platform {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppleMusic => write!(f, "Apple Music"),
            Self::YouTubeMusic => write!(f, "YouTube Music"),
            Self::GoogleStore => write!(f, "Google Store"),
            Self::AmazonStore => write!(f, "Amazon Store"),
            Self::AmazonMusic => write!(f, "Amazon Music"),
            Self::Other(str) => write!(f, "{}", str),
            other => write!(f, "{:?}", other),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum APIProvider {
    Spotify,
    #[allow(non_camel_case_types)]
    iTunes,
    YouTube,
    Google,
    Pandora,
    Deezer,
    Tidal,
    Amazon,
    Soundcloud,
    Napster,
    Yandex,
    Spinrilla,
    Audius,
    Audiomack,
    BoomPlay,
    Anghami,
    #[serde(untagged)]
    Other(String),
}

impl APIProvider {
    #[inline]
    const fn max_prio() -> u8 {
        u8::MAX
    }

    #[inline]
    const fn min_prio() -> u8 {
        0
    }

    const fn prio(&self) -> u8 {
        match self {
            APIProvider::iTunes => Self::max_prio() - 0,
            APIProvider::Spotify => Self::max_prio() - 1,
            APIProvider::Tidal => Self::max_prio() - 2,
            APIProvider::Amazon => Self::max_prio() - 3,
            APIProvider::Deezer => Self::max_prio() - 4,
            APIProvider::Google => Self::max_prio() - 5,
            _ => Self::min_prio(),
        }
    }
}

impl Display for APIProvider {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Other(str) => write!(f, "{}", str),
            other => write!(f, "{:?}", other),
        }
    }
}
