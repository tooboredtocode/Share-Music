/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use twilight_gateway::Shard;
use twilight_model::gateway::event::Event;

use crate::context::Ctx;

mod interactions;

pub fn handle(shard: &Shard, event: Event, context: &Ctx) {
    context
        .metrics
        .update_cluster_metrics(shard, &event, context);

    let ctx = context.clone();
    tokio::spawn(async move {
        #[allow(clippy::single_match)]
        match event {
            Event::InteractionCreate(event) => interactions::handle((*event).0, ctx).await,
            _ => {}
        }
    });
}
