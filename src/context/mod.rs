/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use std::sync::Arc;

use prometheus_client::encoding::EncodeLabelValue;
use reqwest::Client;
use this_state::State as ThisState;
use tracing::info;
use twilight_gateway::{create_recommended, Shard, ConfigBuilder as ShardConfigBuilder};
use twilight_http::Client as TwilightClient;
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;

use crate::config::colour::Options as ColourOptions;
use crate::config::Config;
use crate::constants::cluster_consts;
use crate::context::metrics::Metrics;
use crate::util::EmptyResult;
use crate::util::signal::start_signal_listener;
use crate::util::error::expect_err;

mod discord_client;
mod http_client;
pub mod metrics;

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
    pub colour: ColourOptions,
}

pub type Ctx = Arc<Context>;

impl Context {
    pub async fn new(config: &Config) -> EmptyResult<(Arc<Self>, impl ExactSizeIterator<Item = Shard>)> {
        info!("Creating Cluster");

        let metrics = Metrics::new(0);

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

        let (discord_client, bot_id) = Self::discord_client_from_config(config).await?;
        let discord_shards = Self::create_shards(&discord_client, config).await?;

        let http_client = Self::create_http_client()?;

        let ctx: Arc<_> = Context {
            discord_client,
            bot_id,
            http_client,
            cfg: SavedConfig {
                debug_server: config.discord.debug_server.clone(),
                colour: config.colour,
            },
            metrics,
            state,
        }
        .into();

        Ok((ctx, discord_shards))
    }

    async fn create_shards(
        client: &TwilightClient,
        config: &Config,
    ) -> EmptyResult<impl ExactSizeIterator<Item = Shard>> {
        let shard_config = ShardConfigBuilder::new(
            config.discord.token.clone(),
            cluster_consts::GATEWAY_INTENTS,
        )
            .presence(cluster_consts::presence())
            .build();

        let shards = create_recommended(client, shard_config, |_, builder| builder.build())
            .await
            .map_err(expect_err!("Failed to create recommended number of shards"))?;

        Ok(shards)
    }
}
