/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use tracing::{info, warn};
use twilight_interactions::command::CreateCommand;
use twilight_model::id::Id;

use crate::interactions::InteractionsHandler;
use crate::interactions::commands::find_links::FindLinksCommand;
use crate::interactions::commands::share::ShareCommand;
use crate::interactions::commands::test_colour_consts::TestColorConstsCommand;
use crate::util::EmptyResult;
use crate::util::error::expect_err;
use crate::util::message_command::MessageCommand;

pub mod find_links;
pub mod share;
pub mod test_colour_consts;

impl InteractionsHandler {
    pub async fn sync_commands(&self) -> EmptyResult<()> {
        info!("Syncing commands");
        self.sync().await?;
        info!("Successfully synced all commands");

        Ok(())
    }

    #[cfg(debug_assertions)]
    async fn sync(&self) -> EmptyResult<()> {
        if self.args().debug_server.is_empty() {
            warn!("No Debug Servers were configured")
        }

        for debug_server in &self.args().debug_server {
            self.discord()
                .interaction_client()
                .set_guild_commands(
                    Id::new(*debug_server),
                    &[
                        ShareCommand::create_command().into(),
                        FindLinksCommand::command(),
                        TestColorConstsCommand::create_command().into(),
                    ],
                )
                .await
                .map_err(expect_err!("Failed to Synchronize Commands"))?;
        }

        Ok(())
    }

    #[cfg(not(debug_assertions))]
    async fn sync(&self) -> EmptyResult<()> {
        self.discord()
            .interaction_client()
            .set_global_commands(&[
                ShareCommand::create_command().into(),
                FindLinksCommand::command(),
            ])
            .await
            .map_err(expect_err!("Failed to Synchronize Commands"))?;

        for debug_server in &self.args().debug_server {
            self.discord()
                .interaction_client()
                .set_guild_commands(
                    Id::new(*debug_server),
                    &[TestColorConstsCommand::create_command().into()],
                )
                .await
                .map_err(expect_err!("Failed to Synchronize Commands"))?;
        }

        Ok(())
    }
}
