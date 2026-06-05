/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

mod parse;

macro_rules! create_provider_id {
    ($name:ident, $ty:ty) => {
        #[derive(Debug, PartialEq, Eq, Hash)]
        pub enum $name {
            Album($ty),
            Track($ty),
        }
    };
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum AmazonMusicId {
    Album(String),
    Track { album_id: String, track_id: String },
}

create_provider_id!(AnghamiId, u64);
create_provider_id!(AppleMusicId, u64);
create_provider_id!(BoomPlayId, u64);
create_provider_id!(DeezerId, u64);
create_provider_id!(NapsterId, u64);
create_provider_id!(PandoraId, u64);
create_provider_id!(SpotifyId, String);
create_provider_id!(TidalId, u64);
create_provider_id!(YandexId, u64);

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct YouTubeId(String);

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ProviderId {
    AmazonMusic(AmazonMusicId),
    Anghami(AnghamiId),
    AppleMusic(AppleMusicId),
    BoomPlay(BoomPlayId),
    Deezer(DeezerId),
    Napster(NapsterId),
    Pandora(PandoraId),
    Spotify(SpotifyId),
    Tidal(TidalId),
    Yandex(YandexId),
    YouTube(YouTubeId),
}
