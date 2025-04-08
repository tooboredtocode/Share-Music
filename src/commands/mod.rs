/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use tracing::info;
#[cfg(debug_assertions)]
use tracing::warn;
use twilight_model::id::Id;

use crate::context::Ctx;
use crate::util::EmptyResult;
use crate::util::error::expect_err;

pub mod error;
pub mod find_links;
pub mod share;
pub mod test_colour_consts;

pub async fn sync_commands(ctx: &Ctx) -> EmptyResult<()> {
    info!("Syncing commands");
    sync(ctx).await?;
    info!("Successfully synced all commands");

    Ok(())
}

#[cfg(debug_assertions)]
async fn sync(ctx: &Ctx) -> EmptyResult<()> {
    if ctx.cfg.debug_server.is_empty() {
        warn!("No Debug Servers were configured")
    }

    for debug_server in &ctx.cfg.debug_server {
        ctx.interaction_client()
            .set_guild_commands(
                Id::new(*debug_server),
                &[
                    share::command(),
                    find_links::command(),
                    test_colour_consts::command(),
                ],
            )
            .await
            .map_err(expect_err!("Failed to Synchronize Commands"))?;
    }

    Ok(())
}

#[cfg(not(debug_assertions))]
async fn sync(ctx: &Ctx) -> EmptyResult<()> {
    ctx.interaction_client()
        .set_global_commands(&[share::command(), find_links::command()])
        .await
        .map_err(expect_err!("Failed to Synchronize Commands"))?;

    for debug_server in &ctx.cfg.debug_server {
        ctx.interaction_client()
            .set_guild_commands(Id::new(*debug_server), &[test_colour_consts::command()])
            .await
            .map_err(expect_err!("Failed to Synchronize Commands"))?;
    }

    Ok(())
}
