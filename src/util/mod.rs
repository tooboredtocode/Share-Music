/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use this_state::{State as ThisState, StateFuture};

use crate::context::ClusterState;

pub mod colour;
pub mod discord_locales;
pub mod error;
pub mod event_poller;
pub mod interaction;
pub mod odesli;
pub mod setup_logger;
pub mod signal;

pub type ShareResult<T> = Result<T, error::ShutDown>;
pub type EmptyResult<T> = Result<T, ()>;

pub type TerminationFuture = StateFuture<ClusterState, fn(&ClusterState) -> bool>;

pub fn create_termination_future(state: &ThisState<ClusterState>) -> TerminationFuture {
    state.wait_for(ClusterState::is_terminating)
}
