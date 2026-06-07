/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use twilight_gateway::Shard;
use twilight_model::gateway::event::Event;

use crate::context::Ctx;
use crate::db::{GuildMetadata, UserMetadata};

mod interactions;

pub fn handle(shard: &Shard, event: Event, context: &Ctx) {
    context
        .metrics
        .update_cluster_metrics(shard, &event, context);

    persist_guild_metadata(&event, context);
    persist_user_metadata(&event, context);

    let ctx = context.clone();
    tokio::spawn(async move {
        #[allow(clippy::single_match)]
        match event {
            Event::InteractionCreate(event) => interactions::handle((*event).0, ctx).await,
            _ => {}
        }
    });
}

fn persist_guild_metadata(event: &Event, context: &Ctx) {
    let Some(metadata) = GuildMetadata::try_from_event(event) else {
        return; // Not a guild-related event, or guild is unavailable.
    };

    // Spawn a task to save the metadata to the database asynchronously.
    tokio::spawn(metadata.save_to_db(context.clone()));
}

fn persist_user_metadata(event: &Event, context: &Ctx) {
    let Some(metadata) = UserMetadata::try_from_event(event) else {
        return; // Not an event containing user information
    };

    // Spawn a task to save the metadata to the database asynchronously.
    tokio::spawn(metadata.save_to_db(context.clone()));
}
