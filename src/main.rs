use std::time::Duration;

use metronomos::Runtime;
use tracing::{error, info};

use crate::args::Args;
use crate::color_config::ColorConfig;
use crate::http_server::provide_http_server;
use crate::util::EmptyResult;
use crate::util::setup_logger::setup_logger;

mod args;
mod clients;
mod color_config;
mod constants;
mod db;
mod event_handler;
mod http_server;
mod interactions;
mod metrics;
mod shard_runners;
mod util;

fn main() {
    let args = Args::parse();
    let color_config = args
        .color_config
        .as_ref()
        .map(|path| ColorConfig::from_file(path))
        .unwrap_or_default();

    setup_logger(&args.log, args.log_format);

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
    info!("{} v{} initializing!", constants::NAME, constants::VERSION);
    let mut runtime = Runtime::new_with(|b| {
        b.provide_arc_value(args)?;
        b.provide_arc_value(color_config)?;

        b.provide_async(db::Database::init)?;

        b.provide_with(clients::provide_clients)?;
        b.provide_with(metrics::provide_metrics)?;

        b.provide_async(interactions::InteractionsHandler::init)?;
        b.provide(event_handler::EventHandler::init)?;

        b.provide(provide_http_server)?;
        b.provide(shard_runners::provide_shard_runners)?;

        Ok(())
    })
    .await
    .map_err(|e| {
        error!("Failed to build runtime: {}", e);
    })?;

    info!("Runtime built successfully, starting main loop...");
    runtime.run().await;

    Ok(())
}
