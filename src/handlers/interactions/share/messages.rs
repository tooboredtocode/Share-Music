/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use crate::util::discord_locales::DiscordLocale;

#[inline]
pub const fn invalid_url(locale: DiscordLocale) -> &'static str {
    match locale {
        DiscordLocale::GERMAN => {
            "Bitte sende mir eine valide URL, \
            ich kann nur mit URLs von den folgenden Platformen arbeiten:\n\
            Spotify, iTunes, Apple Music, YouTube, YouTube Music, Pandora, Deezer, Tidal, \
            Amazon Music, SoundCloud and Yandex"
        }
        _ => {
            "Please send a valid URL, I can only work with links from the following platforms:\n\
            Spotify, iTunes, Apple Music, YouTube, YouTube Music, Pandora, Deezer, Tidal, \
            Amazon Music, SoundCloud and Yandex"
        }
    }
}

#[inline]
pub const fn error(locale: DiscordLocale) -> &'static str {
    match locale {
        DiscordLocale::GERMAN => {
            "Ein unerwarteter Fehler ist passiert, die Developer wurden benachrichtigt"
        }
        _ => {
            "An unexpected error has occurred, the dev team has been alerted"
        }
    }
}