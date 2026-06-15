/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use migration::MigratorTrait;
use prometheus_client::encoding::EncodeLabelValue;
use reqwest::Client;
use sea_orm::DatabaseConnection;
use std::cmp::max;
use std::sync::Arc;
use std::time::Duration;
use this_state::State as ThisState;
use tracing::log::LevelFilter;
use tracing::{error, info, instrument};
use twilight_gateway::{ConfigBuilder as ShardConfigBuilder, Shard, create_iterator};
use twilight_http::Client as TwilightClient;
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;

use crate::args::Args;
use crate::color_config::ColorConfig;
use crate::constants::cluster_consts;
use crate::context::metrics::Metrics;
use crate::util::colour::ImageClient;
use crate::util::error::expect_err;
use crate::util::odesli::OdesliClient;
use crate::util::signal::start_signal_listener;
use crate::util::{EmptyResult, create_termination_future};

mod discord_client;
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
    pub odesli_client: OdesliClient,
    pub image_client: ImageClient,
    pub discord_client: TwilightClient,

    bot_id: Id<ApplicationMarker>,

    pub cfg: SavedConfig,

    pub db_connection: Option<DatabaseConnection>,
    pub metrics: Metrics,

    pub state: ThisState<ClusterState>,
}

#[derive(Debug)]
pub struct SavedConfig {
    pub debug_server: Vec<u64>,
}

pub type Ctx = Arc<Context>;

impl Context {
    pub async fn new(
        args: &Args,
        color_config: ColorConfig,
    ) -> EmptyResult<(Arc<Self>, impl ExactSizeIterator<Item = Shard>)> {
        info!("Creating Cluster");

        // TODO: add cluster_id and cluster_count to config and work out how to synchronize the total amount of shards
        let cluster_id = 0;
        let cluster_count = 1;

        let mut metrics = Metrics::new(cluster_id);

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

        let (discord_client, bot_id) = Self::discord_client_from_config(&args.token).await?;
        let discord_shards =
            Self::create_shards(&discord_client, &args.token, cluster_id, cluster_count).await?;

        let http_client = Client::builder()
            .user_agent(crate::constants::USER_AGENT)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(expect_err!("Failed to create HTTP client"))?;

        let db_connection = if let Some(db_url) = &args.database_url {
            let mut connection_opts = sea_orm::ConnectOptions::new(db_url);
            connection_opts.sqlx_logging_level(LevelFilter::Debug);

            let db_connection = sea_orm::Database::connect(connection_opts)
                .await
                .map_err(expect_err!("Failed to connect to the database"))?;

            migration::Migrator::up(&db_connection, None)
                .await
                .map_err(expect_err!("Failed to run database migrations"))?;

            Some(db_connection)
        } else {
            None
        };

        let odesli_client = OdesliClient::builder(http_client.clone(), &mut metrics)
            .with_api_key(args.odesli_api_key.as_ref())
            .with_hourly_limit(args.odesli_hourly_limit)
            .build();

        let ctx: Arc<_> = Context {
            image_client: ImageClient::new(http_client, color_config, &metrics),
            odesli_client,
            discord_client,
            bot_id,
            cfg: SavedConfig {
                debug_server: args.debug_server.clone(),
            },
            db_connection,
            metrics,
            state,
        }
        .into();

        ctx.start_odesli_cache_cleanup_task();

        Ok((ctx, discord_shards))
    }

    #[instrument(skip_all, name = "Context::cache_cleanup_task")]
    fn start_odesli_cache_cleanup_task(self: &Arc<Self>) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            let cleanup_interval = Duration::from_mins(15);
            let cache_entry_max_age = Duration::from_hours(3);

            loop {
                tokio::select! {
                    _ = create_termination_future(&this.state) => {
                        break;
                    },
                    _ = tokio::time::sleep(cleanup_interval) => {
                        info!("Running Odesli cache cleanup task");
                        this.odesli_client.clear_expired_cache_entries(cache_entry_max_age);
                    }
                }
            }
        });
    }

    async fn create_shards(
        client: &TwilightClient,
        token: &str,
        cluster_id: u16,
        cluster_count: u16,
    ) -> EmptyResult<impl ExactSizeIterator<Item = Shard> + use<>> {
        if cluster_id >= cluster_count {
            error!(
                "Cluster ID ({}) must be smaller than the number of clusters ({})",
                cluster_id, cluster_count
            );
            return Err(());
        }

        let request = client.gateway().authed();
        let response = request
            .await
            .map_err(expect_err!("Failed to get recommended number of shards"))?;
        let info = response
            .model()
            .await
            .map_err(expect_err!("Failed to get recommended number of shards"))?;

        let shard_config =
            ShardConfigBuilder::new(token.to_string(), cluster_consts::GATEWAY_INTENTS)
                .presence(cluster_consts::presence())
                .build();

        let cluster_id = cluster_id as u32;
        let cluster_count = cluster_count as u32;
        let total = max(info.shards, cluster_count);

        let iter = (cluster_id..total).step_by(cluster_count as usize);

        Ok(create_iterator(iter, total, shard_config, |_, builder| {
            builder.build()
        }))
    }
}
