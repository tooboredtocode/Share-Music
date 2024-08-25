/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt;
use std::net::SocketAddr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Response, Server};
use parking_lot::Mutex;
use prometheus_client::encoding::{text::encode, EncodeLabelSet, EncodeLabelValue};
use prometheus_client::metrics::{
    counter::Counter, family::Family, gauge::Gauge, histogram::Histogram,
};
use prometheus_client::registry::{Registry, Unit};
use tracing::info;
use twilight_gateway::stream::ShardRef;
use twilight_gateway::ConnectionStatus;
use twilight_model::gateway::event::Event;

use crate::constants::{GIT_BRANCH, GIT_REVISION, NAME, RUST_VERSION, VERSION};
use crate::context::metrics::guild_store::{GuildState, GuildStore};
use crate::context::{ClusterState, Ctx};
use crate::util::create_termination_future;
use crate::util::error::Expectable;
use crate::{Config, Context};

mod guild_store;

#[derive(Debug)]
pub struct Metrics {
    pub registry: Registry,

    pub gateway_events: Family<EventLabels, Counter>,

    pub connected_guilds: Family<GuildLabels, Gauge>,
    guild_store: GuildStore,

    pub shard_states: Family<ShardStateLabels, Gauge>,
    current_states: Mutex<HashMap<u64, String>>,
    pub shard_latencies: Family<ShardLatencyLabels, Gauge<f64, AtomicU64>>,
    pub cluster_state: Family<ClusterLabels, Gauge>,

    pub third_party_api: Family<ThirdPartyLabels, Histogram>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct VersionLabels {
    pub branch: String,
    pub revision: String,
    pub rustc_version: String,
    pub version: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct EventLabels {
    pub shard: u64,
    pub event: Cow<'static, str>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ShardStateLabels {
    pub shard: u64,
    pub state: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ShardLatencyLabels {
    pub shard: u64,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct GuildLabels {
    pub shard: u64,
    pub state: GuildState,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ClusterLabels {
    pub state: ClusterState,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ThirdPartyLabels {
    pub method: Method,
    pub url: Cow<'static, str>,
    pub status: u16,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
#[allow(clippy::upper_case_acronyms)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    CONNECT,
    PATCH,
    TRACE,
}

impl From<hyper::Method> for Method {
    fn from(value: hyper::Method) -> Self {
        match value {
            hyper::Method::GET => Self::GET,
            hyper::Method::POST => Self::POST,
            hyper::Method::PUT => Self::PUT,
            hyper::Method::DELETE => Self::DELETE,
            hyper::Method::HEAD => Self::HEAD,
            hyper::Method::OPTIONS => Self::OPTIONS,
            hyper::Method::CONNECT => Self::CONNECT,
            hyper::Method::PATCH => Self::PATCH,
            hyper::Method::TRACE => Self::TRACE,
            _ => panic!("Unknown method"),
        }
    }
}

impl Metrics {
    pub fn new(cluster_id: u64) -> Self {
        let mut registry = Registry::with_prefix("discord");
        let mut r = registry
            .sub_registry_with_label((Cow::from("cluster"), Cow::from(cluster_id.to_string())));
        r = r.sub_registry_with_label((Cow::from("bot"), Cow::from(NAME)));

        let version = Family::<VersionLabels, Gauge>::default();
        version
            .get_or_create(&VersionLabels {
                branch: GIT_BRANCH.to_string(),
                revision: GIT_REVISION.to_string(),
                rustc_version: RUST_VERSION.to_string(),
                version: VERSION.to_string(),
            })
            .set(1);
        r.register("bot_info", "Information about the bot", version);

        let gateway_events = Family::<EventLabels, Counter>::default();
        r.register(
            "gateway_events",
            "Received gateway events",
            gateway_events.clone(),
        );

        let shard_states = Family::<ShardStateLabels, Gauge>::default();
        r.register("shard_states", "States of the shards", shard_states.clone());

        let shard_latencies = Family::<ShardLatencyLabels, Gauge<f64, AtomicU64>>::default();
        r.register_with_unit(
            "shard_latencies",
            "Latencies of the shards",
            Unit::Seconds,
            shard_latencies.clone(),
        );

        let connected_guilds = Family::<GuildLabels, Gauge>::default();
        r.register(
            "guilds",
            "Guilds Connected to the bot",
            connected_guilds.clone(),
        );
        let guild_store = GuildStore::new();

        let cluster_state = Family::<ClusterLabels, Gauge>::default();
        r.register("cluster_state", "Cluster state", cluster_state.clone());

        let third_party_api = Family::<ThirdPartyLabels, Histogram>::new_with_constructor(|| {
            Histogram::new(
                [
                    0.1, 0.15, 0.2, 0.3, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0, 7.5, 10.0, 15.0, 20.0,
                ]
                .into_iter(),
            )
        });
        r.register(
            "3rd_party_api_request_duration_seconds",
            "Response time for the various APIs used by the bots",
            third_party_api.clone(),
        );

        cluster_state
            .get_or_create(&ClusterLabels {
                state: ClusterState::Starting,
            })
            .set(1);

        Self {
            registry,
            gateway_events,
            connected_guilds,
            guild_store,
            shard_states,
            current_states: Mutex::new(HashMap::new()),
            shard_latencies,
            cluster_state,
            third_party_api,
        }
    }

    pub fn update_cluster_metrics(&self, shard: ShardRef, event: &Event, ctx: &Ctx) {
        if let Some(name) = event.kind().name() {
            self.gateway_events
                .get_or_create(&EventLabels {
                    shard: shard.id().number(),
                    event: Cow::from(name),
                })
                .inc();
        }

        self.guild_store.register(shard.id().number(), event, ctx);

        match event {
            Event::GatewayHello(_)
            | Event::GatewayReconnect
            | Event::Ready(_)
            | Event::Resumed
            | Event::GatewayInvalidateSession(_)
            | Event::GatewayClose(_) => {}
            Event::GatewayHeartbeatAck => {
                self.shard_latencies
                    .get_or_create(&ShardLatencyLabels {
                        shard: shard.id().number(),
                    })
                    .set(shard.latency().recent()[0].as_secs_f64());

                return;
            }
            _ => return,
        }

        let mut lock = self.current_states.lock();
        self.shard_states.clear();

        lock.insert(shard.id().number(), shard_status_to_string(shard.status()));
        for (shard, state) in lock.iter() {
            self.shard_states
                .get_or_create(&ShardStateLabels {
                    shard: *shard,
                    state: state.clone(),
                })
                .inc();
        }
    }
}

fn shard_status_to_string(status: &ConnectionStatus) -> String {
    use ConnectionStatus::*;

    match status {
        Connected => "Connected",
        Disconnected { .. } => "Disconnected",
        FatallyClosed { .. } => "FatallyClosed",
        Identifying => "Identifying",
        Resuming => "Resuming",
    }
    .to_string()
}

impl Context {
    async fn metrics_handler(self: Arc<Self>) -> Result<Response<Body>, fmt::Error> {
        let mut buffer = String::new();
        encode(&mut buffer, &self.metrics.registry)?;
        Ok(Response::new(Body::from(buffer)))
    }

    pub fn start_metrics_server(self: &Arc<Self>, config: &Config) {
        let context = self.clone();
        let make_svc = make_service_fn(move |_conn| {
            let ctx = context.clone();

            let service = service_fn(move |_| ctx.clone().metrics_handler());

            async move { Ok::<_, Infallible>(service) }
        });

        let addr: SocketAddr = ([0, 0, 0, 0], config.metrics.listen_port).into();
        let server = Server::bind(&addr).serve(make_svc);

        let fut = server.with_graceful_shutdown(create_termination_future(&self.state));

        info!("Starting Metrics Server");
        let ctx = self.clone();
        tokio::spawn(async move { fut.await.expect_with_state("Metrics server crashed", &ctx) });
    }
}
