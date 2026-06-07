/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use sea_orm_migration::{prelude::*, schema::*};

fn discord_snowflake<T: IntoIden>(name: T) -> ColumnDef {
    big_integer(name).take()
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DiscordUser::Table)
                    .if_not_exists()
                    .col(discord_snowflake(DiscordUser::Id).primary_key())
                    .col(string(DiscordUser::Username).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(DiscordGuild::Table)
                    .if_not_exists()
                    .col(discord_snowflake(DiscordGuild::Id).primary_key())
                    .col(string(DiscordGuild::Name).not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(DiscordGuild::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(DiscordUser::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum DiscordUser {
    Table,
    Id,
    Username,
}

#[derive(DeriveIden)]
enum DiscordGuild {
    Table,
    Id,
    Name,
}
