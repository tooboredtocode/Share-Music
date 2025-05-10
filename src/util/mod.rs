/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use this_state::{State as ThisState, StateFuture};

use crate::context::ClusterState;

pub mod colour;
pub mod discord_locales;
pub mod error;
pub mod interaction;
pub mod odesli;
pub mod setup_logger;
pub mod shard_poller;
pub mod signal;

pub use error::EmptyResult;

pub type TerminationFuture = StateFuture<ClusterState, fn(&ClusterState) -> bool>;

pub fn create_termination_future(state: &ThisState<ClusterState>) -> TerminationFuture {
    state.wait_for(ClusterState::is_terminating)
}
