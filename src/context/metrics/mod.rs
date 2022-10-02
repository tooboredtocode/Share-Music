/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::{Body, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use prometheus::{Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry, TextEncoder};
use tracing::info;
use twilight_model::gateway::event::Event;

use crate::{Config, Context, TerminationFuture};
use crate::constants::NAME;
use crate::context::Ctx;
use crate::context::metrics::guild_store::GuildStore;
use crate::context::state::ClusterState;
use crate::util::error::Expectable;

mod guild_store;

#[derive(Debug)]
pub struct Metrics {
    pub registry: Registry,

    pub gateway_events: IntCounterVec,

    pub connected_guilds: IntGaugeVec,
    guild_store: GuildStore,

    pub shard_states: IntGaugeVec,
    pub cluster_state: IntGaugeVec,

    pub third_party_api: HistogramVec
}

macro_rules! prefixed {
    ($name:literal) => {
        concat!("discord_", $name)
    };
}

impl Metrics {
    pub fn new(cluster_id: u64) -> Self {
        let mut labels = HashMap::new();
        labels.insert("cluster".to_string(), cluster_id.to_string());
        labels.insert("bot".to_string(), NAME.to_string());
        let registry = Registry::new_custom(None, Some(labels)).unwrap();

        let gateway_events = IntCounterVec::new(
            Opts::new(prefixed!("gateway_events"), "Received gateway events"),
            &["shard", "event"],
        ).unwrap();
        registry.register(Box::new(gateway_events.clone())).unwrap();

        let connected_guilds = IntGaugeVec::new(
            Opts::new(prefixed!("guilds"), "Guilds Connected to the bot"),
            &["shard", "state"],
        ).unwrap();
        registry.register(Box::new(connected_guilds.clone())).unwrap();
        let guild_store = GuildStore::new();

        let shard_states = IntGaugeVec::new(
            Opts::new(prefixed!("shard_states"), "States of the shards"),
            &["shard", "state"]
        ).unwrap();
        registry.register(Box::new(shard_states.clone())).unwrap();

        let cluster_state = IntGaugeVec::new(
            Opts::new(prefixed!("cluster_state"), "Cluster state"),
            &["state"]
        ).unwrap();
        registry.register(Box::new(cluster_state.clone())).unwrap();

        let third_party_api = HistogramVec::new(
            HistogramOpts::new(prefixed!("3rd_party_api_request_duration_seconds"), "Response time for the various APIs used by the bots"),
            &["method", "url", "status"]
        ).unwrap();
        registry.register(Box::new(third_party_api.clone())).unwrap();

        cluster_state
            .get_metric_with_label_values(&[ClusterState::Starting.name()])
            .unwrap()
            .set(1);

        Self {
            registry,
            gateway_events,
            connected_guilds,
            guild_store,
            shard_states,
            cluster_state,
            third_party_api
        }
    }

    pub fn update_cluster_metrics(&self, shard_id: u64, event: &Event, ctx: &Ctx) {
        if let Some(name) = event.kind().name() {
            self.gateway_events
                .get_metric_with_label_values(&[&shard_id.to_string(), name])
                .unwrap()
                .inc();
        }

        self.guild_store.register(shard_id, event, ctx);

        match event {
            Event::ShardConnected(_)
            | Event::ShardConnecting(_)
            | Event::ShardDisconnected(_)
            | Event::ShardIdentifying(_)
            | Event::ShardReconnecting(_)
            | Event::ShardResuming(_) => {},
            _ => return
        }

        self.shard_states.reset();
        for (shard_id, info) in ctx.discord_cluster.info() {
            self.shard_states
                .get_metric_with_label_values(&[&shard_id.to_string(), &info.stage().to_string()])
                .unwrap()
                .inc();
        }
    }
}

impl Context {
    async fn metrics_handler(self: Arc<Self>) -> Result<Response<Body>, Infallible> {
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        let metric_families = self.metrics.registry.gather();
        encoder.encode(&metric_families, &mut buffer).unwrap();

        Ok(Response::new(Body::from(buffer)))
    }

    pub fn start_metrics_server(self: &Arc<Self>, config: &Config) {
        let context = self.clone();
        let make_svc = make_service_fn(move |_conn| {
            let ctx = context.clone();

            let service = service_fn(move |_| {
                ctx.clone().metrics_handler()
            });

            async move { Ok::<_, Infallible>(service) }
        });

        let addr: SocketAddr = ([0, 0, 0, 0], config.metrics.listen_port).into();
        let server = Server::bind(&addr).serve(make_svc);

        let fut = server.with_graceful_shutdown(TerminationFuture::new(self.create_state_listener()));

        info!("Starting Metrics Server");
        let ctx = self.clone();
        tokio::spawn(async move {
            fut.await.expect_with_state("Metrics server crashed", &ctx)
        });
    }
}