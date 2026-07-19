/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */
use metronomos_pulse::error::BuildDependencyError;
use metronomos_pulse::value::{ArcValue, PulseValue};
use migration::MigratorTrait;
use sea_orm::{DatabaseConnection, EntityTrait, Insert};
use tracing::log::LevelFilter;
use tracing::{debug, trace, warn};

use crate::args::Args;
use crate::util::error::expect_err;

mod entity;
mod guild_meta;
mod usage_data;
mod user_meta;
mod util;

pub use guild_meta::GuildMetadata;
pub use usage_data::UsageData;
pub use user_meta::UserMetadata;

#[derive(Debug, Clone, PulseValue)]
pub struct Database {
    connection: Option<DatabaseConnection>,
}

pub trait DbSavable {
    const TYPE_INFO: &'static str;
    type Entity: EntityTrait + 'static;

    fn into_active_model(self) -> <Self::Entity as EntityTrait>::ActiveModel;

    fn insert(
        model: <Self::Entity as EntityTrait>::ActiveModel,
    ) -> Insert<<Self::Entity as EntityTrait>::ActiveModel> {
        Self::Entity::insert(model)
    }

    fn insert_many(
        models: Vec<<Self::Entity as EntityTrait>::ActiveModel>,
    ) -> Insert<<Self::Entity as EntityTrait>::ActiveModel> {
        Self::Entity::insert_many(models)
    }
}

impl Database {
    pub async fn init(args: ArcValue<Args>) -> Result<Self, BuildDependencyError> {
        let connection = if let Some(db_url) = &args.database_url {
            let mut connection_opts = sea_orm::ConnectOptions::new(db_url);
            connection_opts.sqlx_logging_level(LevelFilter::Debug);

            let db_connection = sea_orm::Database::connect(connection_opts)
                .await
                .map_err(expect_err!("Failed to connect to the database"))?;

            migration::Migrator::up(&db_connection, None)
                .await
                .map_err(expect_err!("Failed to run database migrations"))?;

            Some(db_connection)
        } else {
            None
        };

        Ok(Self { connection })
    }

    pub fn spawn_save_to_db<T: DbSavable + Send + 'static>(&self, data: T) {
        if self.connection.is_none() {
            trace!("Db Url not provided, skipping saving {}", T::TYPE_INFO);
            return;
        }

        let db_clone = self.clone();
        tokio::spawn(async move {
            db_clone.save_to_db(data).await;
        });
    }

    pub async fn save_to_db<T: DbSavable>(&self, data: T) {
        let Some(conn) = &self.connection else {
            trace!("Db Url not provided, skipping saving {}", T::TYPE_INFO);
            return;
        };

        let active_model = data.into_active_model();

        match T::insert(active_model).exec(conn).await {
            Ok(_) => {
                debug!("Successfully saved {} to the database", T::TYPE_INFO);
            }
            Err(e) => {
                warn!("Failed to save {} to the database: {}", T::TYPE_INFO, e);
            }
        }
    }

    pub fn spawn_save_multi_to_db<T: DbSavable + Send + 'static>(&self, data: Vec<T>) {
        if self.connection.is_none() {
            trace!(
                "Db Url not provided, skipping saving multiple {}",
                T::TYPE_INFO
            );
            return;
        }

        let db_clone = self.clone();
        tokio::spawn(async move {
            db_clone.save_multi_to_db(data).await;
        });
    }

    pub async fn save_multi_to_db<T: DbSavable>(&self, data: Vec<T>) {
        let Some(conn) = &self.connection else {
            trace!(
                "Db Url not provided, skipping saving multiple {}",
                T::TYPE_INFO
            );
            return;
        };

        let active_models: Vec<<T::Entity as EntityTrait>::ActiveModel> =
            data.into_iter().map(|d| d.into_active_model()).collect();

        match T::insert_many(active_models).exec(conn).await {
            Ok(_) => {
                debug!(
                    "Successfully saved multiple {} to the database",
                    T::TYPE_INFO
                );
            }
            Err(e) => {
                warn!(
                    "Failed to save multiple {} to the database: {:?}",
                    T::TYPE_INFO,
                    e
                );
            }
        }
    }
}
