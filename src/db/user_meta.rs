/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use sea_orm::{EntityTrait, Insert, Set, sea_query};
use twilight_model::gateway::event::Event;
use twilight_model::id::Id;
use twilight_model::id::marker::UserMarker;

use crate::db::DbSavable;
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
}

impl DbSavable for UserMetadata {
    const TYPE_INFO: &'static str = "user metadata";
    type Entity = discord_user::Entity;

    fn into_active_model(self) -> discord_user::ActiveModel {
        discord_user::ActiveModel {
            id: Set(snowflake_to_db(self.id)),
            username: Set(self.username),
        }
    }

    fn insert(model: discord_user::ActiveModel) -> Insert<discord_user::ActiveModel> {
        discord_user::Entity::insert(model).on_conflict(
            sea_query::OnConflict::column(discord_user::Column::Id)
                .update_column(discord_user::Column::Username)
                .to_owned(),
        )
    }

    fn insert_many(models: Vec<discord_user::ActiveModel>) -> Insert<discord_user::ActiveModel> {
        discord_user::Entity::insert_many(models).on_conflict(
            sea_query::OnConflict::column(discord_user::Column::Id)
                .update_column(discord_user::Column::Username)
                .to_owned(),
        )
    }
}
