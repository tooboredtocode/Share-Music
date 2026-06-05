/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use super::{
    AmazonMusicId, AnghamiId, AppleMusicId, BoomPlayId, DeezerId, NapsterId, PandoraId, ProviderId,
    SpotifyId, TidalId, YandexId, YouTubeId,
};
use std::fmt;
use url::Url;

#[derive(Debug)]
pub struct InvalidProviderUrl {
    pub invalid_url: Url,
    pub reason: InvalidProviderIdReason,
    _p: (),
}

#[derive(Debug)]
pub enum InvalidProviderIdReason {
    MissingDomain,
    UnknownDomain(String),
    MalformedUrl,
}

impl fmt::Display for InvalidProviderUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.reason {
            InvalidProviderIdReason::MissingDomain => {
                write!(
                    f,
                    "Invalid provider ID: missing domain in URL '{}'",
                    self.invalid_url
                )
            }
            InvalidProviderIdReason::UnknownDomain(domain) => {
                write!(f, "Invalid provider ID: unknown domain '{}'", domain)
            }
            InvalidProviderIdReason::MalformedUrl => {
                write!(
                    f,
                    "Invalid provider ID: malformed URL: '{}'",
                    self.invalid_url
                )
            }
        }
    }
}

impl std::error::Error for InvalidProviderUrl {}

macro_rules! impl_parse_track_album_providers {
    ($provider:ty) => {
        impl_parse_track_album_providers!($provider, "track", "album");
    };
    ($provider:ty, $track:literal, $album:literal) => {
        impl_parse_track_album_providers!($provider, (
            $track => Track,
            $album => Album
        ));
    };
    ($provider:ty, (
        $($ty:literal => $variant:ident),+
    )) => {
        impl $provider {
            fn from_url(url: &Url) -> Option<Self> {
                let mut path_segments = url.path_segments()?;

                let content_type = path_segments.next()?;
                let id = path_segments.next()?.parse().ok()?;

                if path_segments.next().is_some() {
                    return None;
                }

                match content_type {
                    $( $ty => Some(Self::$variant(id)), )+
                    _ => None,
                }
            }
        }
    };
}

impl AmazonMusicId {
    fn from_url(url: &Url) -> Option<Self> {
        let mut path_segments = url.path_segments()?;

        if path_segments.next() != Some("albums") {
            return None;
        }

        let album_id = path_segments.next()?.to_string();

        if path_segments.next().is_some() {
            return None;
        }

        let track_id = url.query_pairs().find_map(|(key, value)| {
            if key == "trackAsin" {
                Some(value.to_string())
            } else {
                None
            }
        });

        match track_id {
            Some(track_id) => Some(Self::Track { album_id, track_id }),
            None => Some(Self::Album(album_id)),
        }
    }
}

impl_parse_track_album_providers!(AnghamiId, "song", "album");

impl AppleMusicId {
    fn from_url(url: &Url) -> Option<Self> {
        let mut path_segments = url.path_segments()?;

        let _country_code = path_segments.next()?;
        let content_type = path_segments.next()?;
        let _item_name = path_segments.next()?;
        let id = path_segments.next()?.parse().ok()?;

        // If it's a song, the id is in the path and we can return immediately.
        if content_type == "song" {
            return Some(Self::Track(id));
        }

        // If it's an album, we need to check for a track ID in the query parameters.
        // If there is one, it's a track URL, otherwise it's an album URL.
        if content_type != "album" {
            return None;
        }

        let track_id = url.query_pairs().find_map(|(key, value)| {
            if key == "i" {
                Some(value.parse().ok()?)
            } else {
                None
            }
        });

        match track_id {
            Some(track_id) => Some(Self::Track(track_id)),
            None => Some(Self::Album(id)),
        }
    }
}

impl_parse_track_album_providers!(BoomPlayId, "songs", "albums");
impl_parse_track_album_providers!(DeezerId);

impl NapsterId {
    fn from_url(url: &Url) -> Option<Self> {
        let mut path_segments = url.path_segments()?;

        let content_type = path_segments.next()?;
        let id = path_segments.next()?;

        if path_segments.next().is_some() {
            return None;
        }

        match content_type {
            "track" => {
                if !id.starts_with("tra.") {
                    return None;
                }
                let track_id = id.strip_prefix("tra.")?.parse().ok()?;
                Some(Self::Track(track_id))
            }
            "album" => {
                if !id.starts_with("alb.") {
                    return None;
                }
                let album_id = id.strip_prefix("alb.")?.parse().ok()?;
                Some(Self::Album(album_id))
            }
            _ => None,
        }
    }
}

impl PandoraId {
    fn from_url(url: &Url) -> Option<Self> {
        let mut path_segments = url.path_segments()?;

        let raw_id = path_segments.next()?;

        if path_segments.next().is_some() {
            return None;
        }

        let mut id_parts = raw_id.split(':');
        let content_type = id_parts.next()?;
        let id = id_parts.next()?.parse().ok()?;

        match content_type {
            "TR" => Some(Self::Track(id)),
            "AL" => Some(Self::Album(id)),
            _ => None,
        }
    }
}

impl_parse_track_album_providers!(SpotifyId);
impl_parse_track_album_providers!(TidalId);
impl_parse_track_album_providers!(YandexId);

impl YouTubeId {
    fn from_url(url: &Url) -> Option<Self> {
        let mut path_segments = url.path_segments()?;

        if path_segments.next()? != "watch" {
            return None;
        }
        if path_segments.next().is_some() {
            return None;
        }

        let video_id = url.query_pairs().find_map(|(key, value)| {
            if key == "v" {
                Some(value.to_string())
            } else {
                None
            }
        })?;

        Some(Self(video_id))
    }
}

impl ProviderId {
    pub fn parse_url(url: &Url) -> Result<Self, InvalidProviderUrl> {
        let Some(domain) = url.domain() else {
            return Err(InvalidProviderUrl {
                invalid_url: url.clone(),
                reason: InvalidProviderIdReason::MissingDomain,
                _p: (),
            });
        };

        let provider_id = match domain {
            "music.amazon.com" => AmazonMusicId::from_url(url).map(Self::AmazonMusic),
            "play.anghami.com" => AnghamiId::from_url(url).map(Self::Anghami),
            "music.apple.com" | "geo.music.apple.com" => {
                AppleMusicId::from_url(url).map(Self::AppleMusic)
            }
            "www.boomplay.com" => BoomPlayId::from_url(url).map(Self::BoomPlay),
            "www.deezer.com" => DeezerId::from_url(url).map(Self::Deezer),
            "play.napster.com" => NapsterId::from_url(url).map(Self::Napster),
            "www.pandora.com" => PandoraId::from_url(url).map(Self::Pandora),
            "open.spotify.com" => SpotifyId::from_url(url).map(Self::Spotify),
            "listen.tidal.com" => TidalId::from_url(url).map(Self::Tidal),
            "music.yandex.ru" => YandexId::from_url(url).map(Self::Yandex),
            "www.youtube.com" | "music.youtube.com" => YouTubeId::from_url(url).map(Self::YouTube),
            other_domain => {
                return Err(InvalidProviderUrl {
                    invalid_url: url.clone(),
                    reason: InvalidProviderIdReason::UnknownDomain(other_domain.to_string()),
                    _p: (),
                });
            }
        };

        provider_id.ok_or_else(|| InvalidProviderUrl {
            invalid_url: url.clone(),
            reason: InvalidProviderIdReason::MalformedUrl,
            _p: (),
        })
    }
}
