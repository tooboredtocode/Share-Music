/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use crate::util::discord_locales::DiscordLocale;

#[inline]
pub const fn invalid_url(locale: DiscordLocale) -> &'static str {
    match locale {
        DiscordLocale::GERMAN => {
            "Bitte sende mir einen validen Link, \
            ich kann nur mit Links von den folgenden Plattformen arbeiten:\n\
            Spotify, iTunes, Apple Music, YouTube, YouTube Music, Pandora, Deezer, Tidal, \
            Amazon Music, SoundCloud and Yandex"
        }
        _ => {
            "Please send a valid link, I can only work with links from the following platforms:\n\
            Spotify, iTunes, Apple Music, YouTube, YouTube Music, Pandora, Deezer, Tidal, \
            Amazon Music, SoundCloud and Yandex"
        }
    }
}

#[inline]
pub const fn no_links_found(locale: DiscordLocale) -> &'static str {
    match locale {
        DiscordLocale::GERMAN => {
            "Es konnten keine uns bekannten links in der Nachricht gefunden werden"
        }
        _ => "Couldn't find any links in the message",
    }
}

#[inline]
pub const fn error(locale: DiscordLocale) -> &'static str {
    match locale {
        DiscordLocale::GERMAN => {
            "Ein unerwarteter Fehler ist passiert, die Developer wurden benachrichtigt"
        }
        _ => "An unexpected error has occurred, the dev team has been alerted",
    }
}
