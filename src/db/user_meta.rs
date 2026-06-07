/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use sea_orm::{EntityTrait, Set, sea_query};
use tracing::{debug, trace, warn};
use twilight_model::gateway::event::Event;
use twilight_model::id::Id;
use twilight_model::id::marker::UserMarker;

use crate::context::Ctx;
use crate::db::entity::discord_user;
use crate::db::util::snowflake_to_db;

#[derive(Debug)]
pub struct UserMetadata {
    pub id: Id<UserMarker>,
    pub username: String,
}

impl UserMetadata {
    pub fn try_from_event(event: &Event) -> Option<Self> {
        match event {
            Event::InteractionCreate(event) => event.author().map(|author| Self {
                id: author.id,
                username: author.name.clone(),
            }),
            _ => None,
        }
    }

    fn into_active_model(self) -> discord_user::ActiveModel {
        discord_user::ActiveModel {
            id: Set(snowflake_to_db(self.id)),
            username: Set(self.username),
        }
    }

    pub async fn save_to_db(self, ctx: Ctx) {
        let Some(conn) = &ctx.db_connection else {
            trace!("Db Url not provided, skipping saving user metadata");
            return;
        };

        let active_model = self.into_active_model();

        match discord_user::Entity::insert(active_model)
            .on_conflict(
                sea_query::OnConflict::column(discord_user::Column::Id)
                    .update_column(discord_user::Column::Username)
                    .to_owned(),
            )
            .exec(conn)
            .await
        {
            Ok(_) => {
                debug!("Successfully saved user metadata to the database");
            }
            Err(e) => {
                warn!("Failed to save user metadata to the database: {}", e);
            }
        }
    }
}
