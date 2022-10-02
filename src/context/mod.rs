/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::sync::Arc;

use hyper::Client;
use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnector;
use tracing::info;
use twilight_gateway::Cluster;
use twilight_http::Client as TwilightClient;
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;

use crate::{Config, EventPoller, ShareResult};
use crate::config::colour::Options as ColourOptions;
use crate::context::metrics::Metrics;
use crate::context::state::State;
use crate::util::StateUpdater;

mod discord_client;
mod discord_cluster;
pub mod state;
pub mod metrics;
mod http_client;

#[derive(Debug)]
pub struct Context {
    pub discord_client: TwilightClient,
    pub discord_cluster: Cluster,
    bot_id: Id<ApplicationMarker>,

    pub http_client: Client<HttpsConnector<HttpConnector>>,

    pub cfg: SavedConfig,

    pub metrics: Metrics,
    // TODO: add database for command invocation metrics

    state: State
}

#[derive(Debug)]
pub struct SavedConfig {
    pub debug_server: Vec<u64>,
    pub colour: ColourOptions
}

pub type Ctx = Arc<Context>;

impl Context {
    pub async fn new(config: &Config, snd: StateUpdater) -> ShareResult<(EventPoller, Arc<Self>)> {
        info!("Creating Cluster");

        let (discord_client, bot_id) = Self::discord_client_from_config(&config).await?;
        let (discord_cluster, events) = Self::cluster_from_config(&config).await?;

        let http_client = Self::create_http_client();

        let metrics = Metrics::new(0);

        let ctx: Arc<Self> = Context {
            discord_client,
            discord_cluster,
            bot_id,
            http_client,
            cfg: SavedConfig {
                debug_server: config.discord.debug_server.clone(),
                colour: config.colour
            },
            metrics,
            state: State::new(snd)
        }.into();

        ctx.start_state_listener();
        let events_poller = EventPoller::new(events, ctx.create_state_listener());

        Ok((events_poller, ctx))
    }
}