/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::{Body, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::registry::Registry;
use tracing::info;
use twilight_model::gateway::event::Event;

use crate::{Config, Context, TerminationFuture};
use crate::constants::{GIT_BRANCH, GIT_REVISION, NAME, RUST_VERSION, VERSION};
use crate::context::Ctx;
use crate::context::metrics::guild_store::{GuildState, GuildStore};
use crate::context::state::ClusterState;
use crate::util::error::Expectable;

mod guild_store;

#[derive(Debug)]
pub struct Metrics {
    pub registry: Registry,

    pub gateway_events: Family<EventLabels, Counter>,

    pub connected_guilds: Family<GuildLabels, Gauge>,
    guild_store: GuildStore,

    pub shard_states: Family<ShardLabels, Gauge>,
    pub cluster_state: Family<ClusterLabels, Gauge>,

    pub third_party_api: Family<ThirdPartyLabels, Histogram>
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct VersionLabels {
    pub branch: String,
    pub revision: String,
    pub rustc_version: String,
    pub version: String
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct EventLabels {
    pub shard: u64,
    pub event: Cow<'static, str>
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ShardLabels {
    pub shard: u64,
    pub state: String
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct GuildLabels {
    pub shard: u64,
    pub state: GuildState
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ClusterLabels {
    pub state: ClusterState
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ThirdPartyLabels {
    pub method: Method,
    pub url: Cow<'static, str>,
    pub status: u16,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
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
            _ => panic!("Unknown method")
        }
    }
}

impl Metrics {
    pub fn new(cluster_id: u64) -> Self {
        let mut registry = Registry::with_prefix("discord");
        let mut r = registry.sub_registry_with_label((
            Cow::from("cluster"),
            Cow::from(cluster_id.to_string())
        ));
        r = r.sub_registry_with_label((
            Cow::from("bot"),
            Cow::from(NAME)
        ));

        let version = Family::<VersionLabels, Gauge>::default();
        version.get_or_create(&VersionLabels {
            branch: GIT_BRANCH.to_string(),
            revision: GIT_REVISION.to_string(),
            rustc_version: RUST_VERSION.to_string(),
            version: VERSION.to_string()
        }).set(1);
        r.register(
            "bot_info",
            "Information about the bot",
            version
        );

        let gateway_events = Family::<EventLabels, Counter>::default();
        r.register(
            "gateway_events",
            "Received gateway events",
            gateway_events.clone()
        );

        let shard_states = Family::<ShardLabels, Gauge>::default();
        r.register(
            "shard_states",
            "States of the shards",
            shard_states.clone()
        );

        let connected_guilds = Family::<GuildLabels, Gauge>::default();
        r.register(
            "guilds",
            "Guilds Connected to the bot",
            connected_guilds.clone()
        );
        let guild_store = GuildStore::new();

        let cluster_state = Family::<ClusterLabels, Gauge>::default();
        r.register(
            "cluster_state",
            "Cluster state",
            cluster_state.clone()
        );

        let third_party_api = Family::<ThirdPartyLabels, Histogram>::new_with_constructor(
            || Histogram::new([0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0].into_iter())
        );
        r.register(
            "3rd_party_api_request_duration_seconds",
            "Response time for the various APIs used by the bots",
            third_party_api.clone()
        );

        cluster_state.get_or_create(&ClusterLabels {
            state: ClusterState::Starting
        }).set(1);

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
                .get_or_create(&EventLabels {
                    shard: shard_id,
                    event: Cow::from(name)
                }).inc();
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

        self.shard_states.clear();
        for (shard_id, info) in ctx.discord_cluster.info() {
            self.shard_states
                .get_or_create(&ShardLabels {
                    shard: shard_id,
                    state: info.stage().to_string()
                }).inc();
        }
    }
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