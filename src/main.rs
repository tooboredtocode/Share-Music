/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use std::time::Duration;

use futures_util::stream::StreamExt;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::context::ClusterState;
use crate::context::Context;
use crate::util::event_poller::EventStreamPoller;
use crate::util::{setup_logger, ShareResult};

mod commands;
mod config;
mod constants;
mod context;
mod handlers;
mod util;

fn main() {
    let cfg = match Config::load() {
        Ok(ok) => ok,
        Err(err) => {
            eprintln!("Could not read config: {}", err);
            return;
        }
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name(format!("{}Pool", constants::NAME_SHORT))
        .build()
        .expect("Failed to build tokio runtime");
    let _ = runtime.block_on(async_main(cfg));

    info!("Main loop exited gracefully, giving the last tasks 30 seconds to finish cleaning up");
    runtime.shutdown_timeout(Duration::from_secs(30));

    info!("Shutdown complete!");
}

async fn async_main(cfg: Config) -> ShareResult<()> {
    setup_logger::setup(&cfg);
    info!("{} v{} initializing!", constants::NAME, constants::VERSION);

    let (context, mut shards) = Context::new(&cfg).await?;
    context.start_metrics_server(&cfg).await?;
    commands::sync_commands(&context).await?;

    info!("Cluster connecting to discord...");
    let mut events = EventStreamPoller::new(shards.iter_mut(), &context.state);
    context.state.set(ClusterState::Running);

    while let Some((shard_id, event)) = events.next().await {
        match event {
            Ok(event) => {
                // everything is wrapped in the handlers module
                handlers::handle(shard_id, event, &context);
            }
            Err(err) => {
                if err.is_fatal() {
                    context.state.set(ClusterState::Crashing);
                    error!("Fatal error occurred, shutting down: {}", err);
                    break;
                } else {
                    warn!("Non-fatal error occurred: {}", err);
                }
            }
        }
    }

    Ok(())
}
