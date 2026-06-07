/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use sea_orm::{EntityTrait, Set, sea_query};
use tracing::{debug, trace, warn};
use twilight_model::gateway::event::Event;
use twilight_model::gateway::payload::incoming::{GuildCreate, GuildUpdate};
use twilight_model::id::Id;
use twilight_model::id::marker::GuildMarker;

use crate::context::Ctx;
use crate::db::entity::discord_guild;
use crate::db::util::snowflake_to_db;

#[derive(Debug)]
pub struct GuildMetadata {
    pub id: Id<GuildMarker>,
    pub name: String,
}

impl GuildMetadata {
    pub fn try_from_event(event: &Event) -> Option<Self> {
        match event {
            Event::GuildCreate(event) => Self::try_from_guild_create(event),
            Event::GuildUpdate(event) => Some(Self::from_guild_update(event)),
            _ => None,
        }
    }

    fn try_from_guild_create(guild: &GuildCreate) -> Option<Self> {
        match guild {
            GuildCreate::Available(guild) => Some(Self {
                id: guild.id,
                name: guild.name.clone(),
            }),
            GuildCreate::Unavailable(_) => None, // Unavailable guilds don't have metadata to persist.
        }
    }

    fn from_guild_update(guild: &GuildUpdate) -> Self {
        Self {
            id: guild.id,
            name: guild.name.clone(),
        }
    }

    fn into_active_model(self) -> discord_guild::ActiveModel {
        discord_guild::ActiveModel {
            id: Set(snowflake_to_db(self.id)),
            name: Set(self.name),
        }
    }

    pub async fn save_to_db(self, ctx: Ctx) {
        let Some(conn) = &ctx.db_connection else {
            trace!("Db Url not provided, skipping saving guild metadata");
            return;
        };

        let active_model = self.into_active_model();

        match discord_guild::Entity::insert(active_model)
            .on_conflict(
                sea_query::OnConflict::column(discord_guild::Column::Id)
                    .update_column(discord_guild::Column::Name)
                    .to_owned(),
            )
            .exec(conn)
            .await
        {
            Ok(_) => {
                debug!("Successfully saved guild metadata to the database");
            }
            Err(e) => {
                warn!("Failed to save guild metadata to the database: {}", e);
            }
        }
    }
}
