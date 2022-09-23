/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */


use tracing::trace;
use twilight_model::gateway::event::Event;

use crate::context::Ctx;
use crate::handlers::metrics::update_cluster_metrics;

mod metrics;
mod interactions;

pub fn handle(shard_id: u64, event: Event, context: Ctx) {
    // spawn a new tokio thread so we dont clog up the gateway listener
    tokio::spawn(inner_handle(shard_id, event, context));
}

async fn inner_handle(shard_id: u64, event: Event, context: Ctx) {
    trace!("Shard: {}, Event: {:?}", shard_id, event.kind());

    context.cache.update(&event);

    update_cluster_metrics(shard_id, &event, &context);

    match event {
        Event::InteractionCreate(event) => interactions::handle((*event).0, context).await,
        _ => {}
    }
}