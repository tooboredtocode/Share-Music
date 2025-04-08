/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use std::time::Duration;
use tokio::task::JoinSet;
use tracing::{error, info, info_span};
use twilight_gateway::Shard;
use crate::config::Config;
use crate::context::{ClusterState, Ctx};
use crate::context::Context;
use crate::util::{setup_logger, EmptyResult};
use crate::util::shard_poller::ShardPoller;

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

async fn async_main(cfg: Config) -> EmptyResult<()> {
    setup_logger::setup(&cfg);
    info!("{} v{} initializing!", constants::NAME, constants::VERSION);

    let (context, shards) = Context::new(&cfg).await?;
    context.start_metrics_server(&cfg).await?;
    commands::sync_commands(&context).await?;

    info!("Cluster connecting to discord...");
    context.state.set(ClusterState::Running);

    let mut shard_tasks = JoinSet::new();
    for shard in shards {
        let context = context.clone();
        shard_tasks.spawn(shard_main(shard, context));
    }

    while let Some(res) = shard_tasks.join_next().await {
        match res {
            Ok(Ok(())) => continue,
            Ok(Err(())) => return Err(()),
            Err(err) => {
                error!("Shard task panicked: {}", err);
                context.state.set(ClusterState::Crashing);
            }
        }
    }

    info!("All shard tasks have exited");
    Ok(())
}

async fn shard_main(mut shard: Shard, context: Ctx) -> EmptyResult<()> {
    let mut shard_poller = ShardPoller::new_from_context(&context);
    let span = info_span!("shard", id = %shard.id());

    span.in_scope(|| info!("Shard is connecting..."));
    while let Some(event) = shard_poller
        .poll(&mut shard)
        .await
        .map_err(|_| {
            span.in_scope(|| error!("Shard has been fatally closed"));
            context.state.set(ClusterState::Crashing);
        })?
    {
        match event {
            Ok(event) => {
                // everything is wrapped in the handlers module
                handlers::handle(&shard, event, &context);
            }
            Err(err) => {
                span.in_scope(|| info!("A non fatal error occurred during shard polling: {}", err));
            }
        }
    }

    // If we get here, the termination future was triggered and we exited as expected
    span.in_scope(|| info!("Shard has shut down gracefully"));
    Ok(())
}
