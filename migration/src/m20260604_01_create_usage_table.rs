/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use crate::extension::postgres::Type;
use sea_orm_migration::sea_orm::DbBackend;
use sea_orm_migration::{prelude::*, schema::*};

fn discord_snowflake<T: IntoIden>(name: T) -> ColumnDef {
    big_integer(name).take()
}

fn discord_snowflake_null<T: IntoIden>(name: T) -> ColumnDef {
    big_integer_null(name).take()
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db_backend = manager.get_connection().get_database_backend();

        if db_backend == DbBackend::Postgres {
            manager
                .create_type(
                    Type::create()
                        .as_enum(CommandSource::Enum)
                        .values([CommandSource::Share, CommandSource::FindLinks])
                        .to_owned(),
                )
                .await?;
        }

        manager
            .create_table(
                Table::create()
                    .table(CommandUsage::Table)
                    .if_not_exists()
                    .col(discord_snowflake(CommandUsage::InteractionId))
                    .col(string(CommandUsage::OriginalUrl))
                    .primary_key(
                        Index::create()
                            .col(CommandUsage::InteractionId)
                            .col(CommandUsage::OriginalUrl),
                    )
                    .col(enumeration(
                        CommandUsage::CommandSource,
                        CommandSource::Enum,
                        [CommandSource::Share, CommandSource::FindLinks],
                    ))
                    .col(discord_snowflake_null(CommandUsage::GuildId))
                    .col(discord_snowflake_null(CommandUsage::ChannelId))
                    .col(discord_snowflake_null(CommandUsage::UserId))
                    .col(string(CommandUsage::SongLinkUrl))
                    .col(boolean(CommandUsage::DataCached))
                    .col(string_null(CommandUsage::Kind))
                    .col(string_null(CommandUsage::Artist))
                    .col(string_null(CommandUsage::Title))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CommandUsage::Table).to_owned())
            .await?;

        let db_backend = manager.get_connection().get_database_backend();
        if db_backend == DbBackend::Postgres {
            manager
                .drop_type(Type::drop().name(CommandSource::Enum).to_owned())
                .await?;
        }

        Ok(())
    }
}

#[derive(DeriveIden)]
enum CommandSource {
    #[sea_orm(iden = "command_source")]
    Enum,
    Share,
    FindLinks,
}

#[derive(DeriveIden)]
enum CommandUsage {
    Table,
    InteractionId,
    OriginalUrl,
    CommandSource,
    GuildId,
    ChannelId,
    UserId,
    SongLinkUrl,
    DataCached,
    Kind,
    Artist,
    Title,
}
