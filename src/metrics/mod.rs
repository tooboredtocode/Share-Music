/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use std::borrow::Cow;
use std::sync::Arc;

use axum::extract::State as AxumState;
use axum::http::StatusCode;
use axum::routing::MethodRouter;
use metronomos::builder::RuntimeBuilder;
use metronomos_pulse::builder::ProvideError;
use metronomos_pulse::value::ValueGroupEntry;
use prometheus_client::registry::Registry;
use tracing::warn;
use twilight_gateway::Shard;
use twilight_model::gateway::event::Event;

use crate::constants::CLUSTER_ID;
use crate::http_server::HttpServeRoute;
use crate::metrics::labels::{EventLabels, ShardLatencyLabels};

mod guild_metrics;
pub mod labels;
mod shard_states;
mod store;

pub use store::MetricsStore;

impl MetricsStore {
    pub fn update_cluster_metrics(&self, shard: &Shard, event: &Event) {
        if let Some(name) = event.kind().name() {
            self.gateway_events()
                .get_or_create(&EventLabels {
                    shard: shard.id().number(),
                    event: Cow::from(name),
                })
                .inc();
        }

        self.connected_guilds().register(shard.id().number(), event);

        match event {
            Event::GatewayHello(_)
            | Event::GatewayReconnect
            | Event::Ready(_)
            | Event::Resumed
            | Event::GatewayInvalidateSession(_)
            | Event::GatewayClose(_) => {}
            Event::GatewayHeartbeatAck => {
                self.shard_latencies()
                    .get_or_create(&ShardLatencyLabels {
                        shard: shard.id().number(),
                    })
                    .set(shard.latency().recent()[0].as_secs_f64());

                return;
            }
            _ => return,
        }

        self.shard_states()
            .update_shard_state(shard.id().number(), shard.state());
    }
}

async fn metrics_handler(AxumState(registry): AxumState<Arc<Registry>>) -> (StatusCode, String) {
    use prometheus_client::encoding::text::encode;

    let mut buffer = String::new();
    match encode(&mut buffer, &registry) {
        Ok(()) => (StatusCode::OK, buffer),
        Err(e) => {
            warn!("Failed to encode metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to encode metrics: {}", e),
            )
        }
    }
}

fn init_metrics_route(metrics: MetricsStore) -> ValueGroupEntry<HttpServeRoute> {
    let registry = metrics.registry(CLUSTER_ID);

    let router = MethodRouter::new()
        .get(metrics_handler)
        .with_state(Arc::new(registry));

    ValueGroupEntry(HttpServeRoute {
        path: "/metrics",
        router,
    })
}

pub fn provide_metrics(b: &mut RuntimeBuilder) -> Result<(), ProvideError> {
    b.provide_value(MetricsStore::new())?;
    b.provide(init_metrics_route)?;

    Ok(())
}
