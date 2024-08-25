/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use tokio::sync::broadcast;

pub use termination_future::TerminationFuture;

use crate::context::state::ClusterState;

pub mod colour;
pub mod discord_locales;
pub mod error;
pub mod event_poller;
pub mod interaction;
pub mod odesli;
pub mod parser;
pub mod setup_logger;
pub mod signal;
mod termination_future;

pub type ShareResult<T> = Result<T, error::ShutDown>;
pub type EmptyResult<T> = Result<T, ()>;

pub type StateUpdater = broadcast::Sender<(ClusterState, bool)>;
pub type StateListener = broadcast::Receiver<(ClusterState, bool)>;
