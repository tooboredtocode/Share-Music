/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use this_state::State as ThisState;
use tokio::signal;
use tracing::{error, info};

use crate::context::ClusterState;
use crate::util::create_termination_future;

async fn signal_listener(state: ThisState<ClusterState>) {
    info!("Starting Signal Listener");

    tokio::select! {
        _ = create_termination_future(&state) => {},
        signal_res = signal::ctrl_c() => {
            match signal_res {
                Ok(()) => {
                    info!("Received shutdown signal, terminating cluster!");
                    state.set(ClusterState::Terminating);
                },
                Err(err) => {
                    error!(failed_with = err.to_string(), "An Exception occurred while waiting for the shutdown signal, shutting down!");
                    state.set(ClusterState::Crashing);
                }
            }
        }
    }
}

pub fn start_signal_listener(state: ThisState<ClusterState>) {
    tokio::spawn(signal_listener(state));
}
