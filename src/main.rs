/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::time::Duration;

use futures_util::stream::StreamExt;
use tokio::sync::broadcast;
use tracing::info;

use crate::config::Config;
use crate::constants::state_consts;
use crate::context::Context;
use crate::util::{setup_logger, ShareResult, StateListener, TerminationFuture};
use crate::util::event_poller::EventPoller;
use crate::util::signal::start_signal_listener;

mod constants;
mod config;
mod util;
mod context;
mod handlers;
mod commands;

fn main() {
    let cfg = match Config::load() {
        Ok(ok) => ok,
        Err(err) => {
            eprintln!("Could not read config: {}", err);
            return;
        }
    };

    setup_logger::setup(&cfg);

    info!("{} v{} initializing!", constants::NAME, constants::VERSION);
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
    let (sender, _) = broadcast::channel(state_consts::QUEUE_LEN);

    start_signal_listener(sender.clone());

    let (mut events, context) = Context::new(&cfg, sender).await?;
    context.start_metrics_server(&cfg);
    commands::sync_commands(&context).await?;
    context.start_cluster();

    info!("Starting Gateway Event Listener");
    while let Some((shard_id, event)) = events.next().await {
        // everything is wrapped in the handlers module
        handlers::handle(shard_id, event, context.clone());
    }

    Ok(())
}
