/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::sync::Arc;

use hyper::client::HttpConnector;
use hyper::Client;
use hyper_rustls::HttpsConnector;
use tracing::info;
use twilight_gateway::{stream, Config as ShardConfig, Shard};
use twilight_http::Client as TwilightClient;
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;

use crate::config::colour::Options as ColourOptions;
use crate::constants::cluster_consts;
use crate::context::metrics::Metrics;
use crate::context::state::State;
use crate::util::error::Expectable;
use crate::util::StateUpdater;
use crate::{Config, ShareResult};

mod discord_client;
mod http_client;
pub mod metrics;
pub mod state;

#[derive(Debug)]
pub struct Context {
    pub discord_client: TwilightClient,
    bot_id: Id<ApplicationMarker>,

    pub http_client: Client<HttpsConnector<HttpConnector>>,

    pub cfg: SavedConfig,

    pub metrics: Metrics,
    // TODO: add database for command invocation metrics
    state: State,
}

#[derive(Debug)]
pub struct SavedConfig {
    pub debug_server: Vec<u64>,
    pub colour: ColourOptions,
}

pub type Ctx = Arc<Context>;

impl Context {
    pub async fn new(config: &Config, snd: StateUpdater) -> ShareResult<(Arc<Self>, Vec<Shard>)> {
        info!("Creating Cluster");

        let (discord_client, bot_id) = Self::discord_client_from_config(&config).await?;
        let discord_shards = stream::create_recommended(
            &discord_client,
            ShardConfig::builder(
                config.discord.token.clone(),
                cluster_consts::GATEWAY_INTENTS,
            )
            .presence(cluster_consts::presence())
            .build(),
            |_, builder| builder.build(),
        )
        .await
        .expect_with("Failed to create recommended shards")?
        .collect();

        let http_client = Self::create_http_client();

        let metrics = Metrics::new(0);

        let ctx: Arc<Self> = Context {
            discord_client,
            bot_id,
            http_client,
            cfg: SavedConfig {
                debug_server: config.discord.debug_server.clone(),
                colour: config.colour,
            },
            metrics,
            state: State::new(snd),
        }
        .into();

        ctx.start_state_listener();
        Ok((ctx, discord_shards))
    }
}
