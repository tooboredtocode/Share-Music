/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

#![allow(clippy::uninlined_format_args)]

use crate::args::Args;
use crate::color_config::ColorConfig;
use crate::context::Context;
use crate::context::{ClusterState, Ctx};
use crate::util::shard_poller::ShardPoller;
use crate::util::{EmptyResult, setup_logger};
use std::time::Duration;
use tokio::task::JoinSet;
use tracing::{error, info, info_span};
use twilight_gateway::Shard;

mod args;
mod color_config;
mod commands;
mod constants;
mod context;
mod handlers;
mod util;

fn main() {
    let args = Args::parse();
    let color_config = args
        .color_config
        .as_ref()
        .map(|path| ColorConfig::from_file(path))
        .unwrap_or_default();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name(format!("{}Pool", constants::NAME_SHORT))
        .build()
        .expect("Failed to build tokio runtime");
    let _ = runtime.block_on(async_main(args, color_config));

    info!("Main loop exited gracefully, giving the last tasks 30 seconds to finish cleaning up");
    runtime.shutdown_timeout(Duration::from_secs(30));

    info!("Shutdown complete!");
}

async fn async_main(args: Args, color_config: ColorConfig) -> EmptyResult<()> {
    setup_logger::setup(&args.log, args.log_format);
    info!("{} v{} initializing!", constants::NAME, constants::VERSION);

    let (context, shards) = Context::new(&args.token, &args.debug_server, color_config).await?;
    context.start_status_server(args.metrics_port).await?;
    commands::sync_commands(&context).await?;

    info!("Cluster connecting to discord...");
    context.state.set(ClusterState::Running);

    let mut shard_tasks = JoinSet::new();
    for shard in shards {
        let context = context.clone();
        shard_tasks.spawn(shard_main(shard, context));
    }

    while let Some(res) = shard_tasks.join_next().await {
        if let Err(err) = res {
            error!("Shard task panicked: {}", err);
            context.state.set(ClusterState::Crashing);
        }
    }

    info!("All shard tasks have been joined, exiting main loop");
    Ok(())
}

async fn shard_main(mut shard: Shard, context: Ctx) -> EmptyResult<()> {
    let mut shard_poller = ShardPoller::new_from_context(&context);
    let span = info_span!("shard", id = %shard.id());

    span.in_scope(|| info!("Shard is connecting..."));
    while let Some(event) = shard_poller.poll(&mut shard).await.map_err(|_| {
        span.in_scope(|| error!("Shard has been fatally closed"));
        context.state.set(ClusterState::Crashing);
    })? {
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
