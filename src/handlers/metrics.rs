/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use twilight_model::gateway::event::Event;

use crate::context::Ctx;

pub fn update_cluster_metrics(shard_id: u64, event: &Event, context: &Ctx) {
    if let Some(name) = event.kind().name() {
        context.metrics
            .gateway_events
            .get_metric_with_label_values(&[&shard_id.to_string(), name])
            .unwrap()
            .inc();
    }

    let current_cache_stats = context.cache.stats();

    match event {
        Event::ShardConnected(_)
        | Event::ShardConnecting(_)
        | Event::ShardDisconnected(_)
        | Event::ShardIdentifying(_)
        | Event::ShardReconnecting(_)
        | Event::ShardResuming(_) => recalculate_shard_states(&context),
        Event::Ready(_)
        | Event::GuildCreate(_)
        | Event::GuildDelete(_) => {
            context.metrics
                .connected_guilds
                .get_metric_with_label_values(&[&shard_id.to_string(), "available"])
                .unwrap()
                .set(current_cache_stats.guilds() as i64);
            context.metrics
                .connected_guilds
                .get_metric_with_label_values(&[&shard_id.to_string(), "unavailable"])
                .unwrap()
                .set(current_cache_stats.unavailable_guilds() as i64)
        },
        _ => {}
    }
}

pub fn recalculate_shard_states(context: &Ctx) {
    context.metrics.shard_states.reset();
    for (shard_id, info) in context.discord_cluster.info() {
        context.metrics
            .shard_states
            .get_metric_with_label_values(&[&shard_id.to_string(), &info.stage().to_string()])
            .unwrap()
            .inc();
    }
}