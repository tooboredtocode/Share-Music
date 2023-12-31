/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use tracing::info;
#[cfg(debug_assertions)]
use tracing::warn;
use twilight_model::id::Id;

use crate::context::Ctx;
use crate::ShareResult;
use crate::util::error::Expectable;

pub mod test_colour_consts;
pub mod find_links;
pub mod share;
pub mod error;

pub async fn sync_commands(ctx: &Ctx) -> ShareResult<()> {
    info!("Syncing commands");
    sync(ctx).await?;
    info!("Successfully synced all commands");

    Ok(())
}

#[cfg(debug_assertions)]
async fn sync(ctx: &Ctx) -> ShareResult<()> {
    if ctx.cfg.debug_server.len() == 0 {
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
                ]
            )
            .await
            .expect_with("Failed to Synchronize Commands")?;
    }

    Ok(())
}

#[cfg(not(debug_assertions))]
async fn sync(ctx: &Ctx) -> ShareResult<()> {
    ctx.interaction_client()
        .set_global_commands(&[
            share::command(),
            find_links::command()
        ])
        .await
        .expect_with("Failed to Synchronize Commands")?;

    for debug_server in &ctx.cfg.debug_server {
        ctx.interaction_client()
            .set_guild_commands(
                Id::new(*debug_server),
                &[
                    test_colour_consts::command()
                ]
            )
            .await
            .expect_with("Failed to Synchronize Commands")?;
    }

    Ok(())
}