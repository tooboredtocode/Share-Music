use metronomos::lifecycle::{Lifecycle, LifecycleContext};
use tokio::task::JoinSet;
use tracing::{error, info, info_span};
use twilight_gateway::{EventTypeFlags, Shard, ShardState, StreamExt};
use twilight_model::gateway::CloseFrame;
use twilight_model::gateway::event::Event;

use crate::clients::discord::DiscordClient;
use crate::constants::{CLUSTER_COUNT, CLUSTER_ID};
use crate::event_handler::EventHandler;
use crate::util::EmptyResult;
use crate::util::error::ExpectErr;
use crate::util::panic_utils::panic_payload_as_str;

pub fn provide_shard_runners(
    lifecycle: Lifecycle,
    discord: DiscordClient,
    event_handler: EventHandler,
) {
    lifecycle
        .hook(move |ctx| {
            let discord = discord.clone();
            let event_handler = event_handler.clone();
            async move {
                if launch_shard_runners(&ctx, discord, event_handler)
                    .await
                    .is_err()
                {
                    // Error occurred during shard runner launch, we should terminate the lifecycle
                    ctx.notify_error();
                }
            }
        })
        .disable_timeout();
}

async fn launch_shard_runners(
    context: &LifecycleContext,
    discord: DiscordClient,
    event_handler: EventHandler,
) -> EmptyResult<()> {
    let shards = discord
        .create_shards(CLUSTER_ID, CLUSTER_COUNT)
        .await
        .map_err(|_| ExpectErr)?;

    let mut shard_tasks = JoinSet::new();
    let mut message_senders = Vec::with_capacity(shards.len());
    for shard in shards {
        message_senders.push(shard.sender());
        shard_tasks.spawn(shard_runner(context.clone(), shard, event_handler.clone()));
    }

    let ctx = context.clone();
    shard_tasks.spawn(async move {
        // Wait for the lifecycle to signal termination
        ctx.wait_for_shutdown().await;

        // Notify all shards to terminate
        for sender in message_senders {
            // The only error that can occur here is if the shard has already terminated, which is fine
            let _ = sender.close(CloseFrame::NORMAL);
        }
    });

    while let Some(res) = shard_tasks.join_next().await {
        let panic_info = match res {
            Ok(()) => continue, // Shard task completed successfully
            Err(err) if err.is_panic() => err.into_panic(),
            Err(_) => continue, // Shard task was cancelled, we can ignore this.
        };

        match panic_payload_as_str(&panic_info) {
            Some(msg) => error!("Shard task panicked with message: {}", msg),
            None => error!("Shard task panicked with unknown payload"),
        }
        context.notify_error(); // Notify the lifecycle of the error
    }

    info!("All shard tasks have been joined, hook is exiting.");
    Ok(())
}

macro_rules! poll_shard_stream {
    ($shard_stream:expr, $span:expr, $event:ident => {
        $($body:tt)*
    }) => {
        while let Some($event) = $shard_stream.next_event(EventTypeFlags::all()).await {
            match $event {
                Ok($event) => {
                    $($body)*
                }
                Err(err) => {
                    $span.in_scope(|| info!("A non fatal error occurred during shard polling: {}", err));
                }
            }
        }
    };
}

async fn shard_runner(context: LifecycleContext, shard: Shard, event_handler: EventHandler) {
    let span = info_span!("shard", id = %shard.id());
    let mut shard_stream = Box::pin(context.wrap_stream(shard));

    span.in_scope(|| info!("Shard is connecting..."));
    poll_shard_stream!(shard_stream, span, event => {
         // everything is wrapped in the handlers module
        event_handler.handle_event(shard_stream.inner(), event);
    });

    // Lifecycle termination has been signalled, continue polling until we receive a close frame or
    // the shard stream ends
    if shard_stream.inner().state() == ShardState::Active {
        // Note we need to poll the inner shard stream directly here, as the lifecycle wrapper will
        // return None once the termination future is triggered
        poll_shard_stream!(shard_stream.as_mut().inner_pin_mut(), span, event => {
            if let Event::GatewayClose(_) = event {
                // The shard has received a close frame, we can exit the loop and shut down
                return;
            }
            // continue processing events until we receive a close frame or the stream ends
            event_handler.handle_event(shard_stream.inner(), event);
        });
    }

    // If we get here, the termination future was triggered and we exited as expected
    span.in_scope(|| info!("Shard has shut down gracefully"));
}
