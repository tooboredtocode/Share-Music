/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */
use std::cmp::max;
use std::sync::Arc;

use prometheus_client::encoding::EncodeLabelValue;
use reqwest::Client;
use this_state::State as ThisState;
use tracing::{error, info};
use twilight_gateway::{Shard, ConfigBuilder as ShardConfigBuilder, create_iterator};
use twilight_http::Client as TwilightClient;
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use crate::color_config::ColorConfig;
use crate::constants::cluster_consts;
use crate::context::metrics::Metrics;
use crate::util::EmptyResult;
use crate::util::signal::start_signal_listener;
use crate::util::error::expect_err;

mod discord_client;
mod http_client;
pub mod metrics;
mod status_server;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
pub enum ClusterState {
    Starting,
    Running,
    Terminating,
    Crashing,
}

impl ClusterState {
    pub fn name(&self) -> &str {
        match self {
            Self::Starting => "Starting",
            Self::Running => "Running",
            Self::Terminating => "Terminating",
            Self::Crashing => "Crashing",
        }
    }

    pub fn is_terminating(&self) -> bool {
        matches!(self, Self::Terminating | Self::Crashing)
    }
}

#[derive(Debug)]
pub struct Context {
    pub discord_client: TwilightClient,
    bot_id: Id<ApplicationMarker>,

    pub http_client: Client,

    pub cfg: SavedConfig,

    pub metrics: Metrics,
    // TODO: add database for command invocation metrics
    pub state: ThisState<ClusterState>,
}

#[derive(Debug)]
pub struct SavedConfig {
    pub debug_server: Vec<u64>,
    pub color: ColorConfig,
}

pub type Ctx = Arc<Context>;

impl Context {
    pub async fn new(
        token: &str, debug_servers: &[u64], color_config: ColorConfig
    ) -> EmptyResult<(Arc<Self>, impl ExactSizeIterator<Item = Shard>)> {
        info!("Creating Cluster");

        // TODO: add cluster_id and cluster_count to config and work out how to synchronize the total amount of shards
        let cluster_id = 0;
        let cluster_count = 1;

        let metrics = Metrics::new(cluster_id);

        let cluster_state_metric = metrics.cluster_state.clone();
        let state = ThisState::new_with_on_change(ClusterState::Starting, move |old, new| {
            use metrics::ClusterLabels;

            info!("Cluster state change: {} -> {}", old.name(), new.name());
            cluster_state_metric.clear();
            cluster_state_metric
                .get_or_create(&ClusterLabels { state: *new })
                .inc();
        });

        start_signal_listener(state.clone());

        let (discord_client, bot_id) = Self::discord_client_from_config(token).await?;
        let discord_shards = Self::create_shards(
            &discord_client,
            token,
            cluster_id,
            cluster_count,
        ).await?;

        let http_client = Self::create_http_client()?;

        let ctx: Arc<_> = Context {
            discord_client,
            bot_id,
            http_client,
            cfg: SavedConfig {
                debug_server: debug_servers.to_vec(),
                color: color_config,
            },
            metrics,
            state,
        }
        .into();

        Ok((ctx, discord_shards))
    }

    async fn create_shards(
        client: &TwilightClient,
        token: &str,
        cluster_id: u16,
        cluster_count: u16,
    ) -> EmptyResult<impl ExactSizeIterator<Item = Shard> + use<>> {
        if cluster_id >= cluster_count {
            error!("Cluster ID ({}) must be smaller than the number of clusters ({})", cluster_id, cluster_count);
            return Err(())
        }

        let request = client.gateway().authed();
        let response = request.await
            .map_err(expect_err!("Failed to get recommended number of shards"))?;
        let info = response
            .model()
            .await
            .map_err(expect_err!("Failed to get recommended number of shards"))?;

        let shard_config = ShardConfigBuilder::new(
            token.to_string(),
            cluster_consts::GATEWAY_INTENTS,
        )
            .presence(cluster_consts::presence())
            .build();

        let cluster_id = cluster_id as u32;
        let cluster_count = cluster_count as u32;
        let total = max(info.shards, cluster_count);

        let iter = (cluster_id..total).step_by(cluster_count as usize);

        Ok(create_iterator(
            iter,
            total,
            shard_config,
            |_, builder| builder.build()
        ))
    }
}
