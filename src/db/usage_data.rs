/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use sea_orm::Set;
use twilight_model::application::interaction::Interaction;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, InteractionMarker, UserMarker};

use crate::clients::odesli::EntityData;
use crate::db::DbSavable;
use crate::db::entity::command_usage;
use crate::db::entity::sea_orm_active_enums::CommandSource as DbCommandSource;
use crate::db::util::snowflake_to_db;

#[derive(Debug, PartialEq, Eq)]
pub enum CommandSource {
    ShareCommand,
    FindLinksCommand,
}

#[derive(Debug)]
pub struct UsageData {
    pub interaction_id: Id<InteractionMarker>,
    pub original_url: String,
    pub command_source: CommandSource,
    pub guild_id: Option<Id<GuildMarker>>,
    pub channel_id: Option<Id<ChannelMarker>>,
    pub user_id: Option<Id<UserMarker>>,
    pub song_link_url: String,
    pub data_cached: bool,
    pub kind: Option<String>,
    pub artist: Option<String>,
    pub title: Option<String>,
}

impl CommandSource {
    fn to_db(&self) -> DbCommandSource {
        match self {
            CommandSource::ShareCommand => DbCommandSource::Share,
            CommandSource::FindLinksCommand => DbCommandSource::FindLinks,
        }
    }
}

impl UsageData {
    #[inline]
    pub fn from_share_command(
        inter: &Interaction,
        original_url: impl Into<String>,
        song_link_url: impl Into<String>,
        entity_data: &EntityData,
        is_data_cached: bool,
    ) -> Self {
        Self::from_data_and_source(
            inter,
            original_url,
            song_link_url,
            entity_data,
            is_data_cached,
            CommandSource::ShareCommand,
        )
    }

    #[inline]
    pub fn from_find_links_command(
        inter: &Interaction,
        original_url: impl Into<String>,
        song_link_url: impl Into<String>,
        entity_data: &EntityData,
        is_data_cached: bool,
    ) -> Self {
        Self::from_data_and_source(
            inter,
            original_url,
            song_link_url,
            entity_data,
            is_data_cached,
            CommandSource::FindLinksCommand,
        )
    }

    #[inline]
    fn from_data_and_source(
        inter: &Interaction,
        original_url: impl Into<String>,
        song_link_url: impl Into<String>,
        entity_data: &EntityData,
        is_data_cached: bool,
        source: CommandSource,
    ) -> Self {
        Self {
            interaction_id: inter.id,
            original_url: original_url.into(),
            command_source: source,
            guild_id: inter.guild_id,
            channel_id: inter.channel.as_ref().map(|c| c.id),
            user_id: inter.author_id(),
            song_link_url: song_link_url.into(),
            data_cached: is_data_cached,
            kind: entity_data.kind.clone(),
            artist: entity_data.artist_name.clone(),
            title: entity_data.title.clone(),
        }
    }
}

impl DbSavable for UsageData {
    const TYPE_INFO: &'static str = "command usage data";
    type Entity = command_usage::Entity;

    fn into_active_model(self) -> command_usage::ActiveModel {
        command_usage::ActiveModel {
            interaction_id: Set(snowflake_to_db(self.interaction_id)),
            original_url: Set(self.original_url),
            command_source: Set(self.command_source.to_db()),
            guild_id: Set(self.guild_id.map(snowflake_to_db)),
            channel_id: Set(self.channel_id.map(snowflake_to_db)),
            user_id: Set(self.user_id.map(snowflake_to_db)),
            song_link_url: Set(self.song_link_url),
            data_cached: Set(self.data_cached),
            kind: Set(self.kind),
            artist: Set(self.artist),
            title: Set(self.title),
        }
    }
}
