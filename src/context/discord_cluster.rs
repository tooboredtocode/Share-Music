/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::sync::Arc;

use tracing::info;
use twilight_gateway::Cluster;
use twilight_gateway::cluster::Events;

use crate::{Config, Context, ShareResult};
use crate::constants::cluster_consts;
use crate::context::state::ClusterState;
use crate::util::error::Expectable;

impl Context {
    pub(super) async fn cluster_from_config(config: &Config) -> ShareResult<(Cluster, Events)> {
        let builder = Cluster::builder(
            config.discord.token.clone(),
            cluster_consts::GATEWAY_INTENTS
        )
            .presence(cluster_consts::presence());

        // TODO: Use cluster manager

        builder.build().await.expect_with("Failed to build Cluster from Config")
    }

    pub fn start_cluster(self: &Arc<Self>) {
        let ctx = self.clone();

        tokio::spawn(async move {
            info!("Cluster connecting to discord...");
            ctx.discord_cluster.up().await;
            ctx.set_state(ClusterState::Running);
            info!("All shards are up!")
        });
    }
}