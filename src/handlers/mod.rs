/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use tracing::trace;
use twilight_gateway::stream::ShardRef;
use twilight_model::gateway::event::Event;

use crate::context::Ctx;

mod interactions;

pub fn handle(shard: ShardRef, event: Event, context: &Ctx) {
    trace!("Shard: {}, Event: {:?}", shard.id().number(), event.kind());

    context
        .metrics
        .update_cluster_metrics(shard, &event, &context);

    let ctx = context.clone();
    tokio::spawn(async move {
        match event {
            Event::InteractionCreate(event) => interactions::handle((*event).0, ctx).await,
            _ => {}
        }
    });
}
