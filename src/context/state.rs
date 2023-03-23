/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::sync::Arc;

use parking_lot::RwLock;
use prometheus_client::encoding::EncodeLabelValue;
use tokio::sync::broadcast::error::RecvError;
use tracing::info;

use crate::Context;
use crate::context::metrics::ClusterLabels;
use crate::util::{StateListener, StateUpdater};

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
pub enum ClusterState {
    Starting,
    Running,
    Terminating,
    Crashing
}

impl ClusterState {
    pub fn name(&self) -> &str {
        match self {
            Self::Starting => "Starting",
            Self::Running => "Running",
            Self::Terminating => "Terminating",
            Self::Crashing => "Crashing"
        }
    }

    pub fn is_terminating(&self) -> bool {
        match self {
            Self::Terminating
            | Self::Crashing => true,
            _ => false
        }
    }
}

#[derive(Debug)]
pub(super) struct State {
    data: RwLock<ClusterState>,
    send: StateUpdater
}

impl State {
    pub(super) fn new(send: StateUpdater) -> State {
        Self {
            data: ClusterState::Starting.into(),
            send
        }
    }

    fn set_state(&self, new_state: ClusterState) {
        let mut state = self.data.write();

        info!("Cluster state change: {} -> {}", state.name(), new_state.name());

        *state = new_state.clone();
        let _ = self.send.send((new_state, true));
    }

    #[allow(dead_code)]
    fn match_state(&self) -> ClusterState {
        *self.data.read()
    }

    fn create_state_listener(&self) -> StateListener {
        self.send.subscribe()
    }
}

impl Context {
    pub(super) fn start_state_listener(self: &Arc<Self>) {
        let mut rcv = self.state.create_state_listener();
        let ctx = self.clone();

        tokio::spawn(async move {
            loop {
                match rcv.recv().await {
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => {},
                    Ok((state, handled)) => {
                        let ret = state.is_terminating();
                        if !handled {
                            ctx.set_state(state);
                        }
                        if ret {
                            break;
                        }
                    }
                }
            }
        });
    }

    /// Sets the state of the cluster and updates the metrics
    pub fn set_state(&self, new_state: ClusterState) {
        self.metrics.cluster_state.clear();
        self.metrics.cluster_state
            .get_or_create(&ClusterLabels {
                state: new_state,
            })
            .set(1);

        self.state.set_state(new_state)
    }

    /// Get a read view into the current state of the cluster
    #[allow(dead_code)]
    pub fn match_state(&self) -> ClusterState {
        self.state.match_state()
    }

    /// Returns a new state listener, this can be used to build a [`TerminationFuture`]
    ///
    /// [`TerminationFuture`]: crate::util::termination_future::TerminationFuture
    pub fn create_state_listener(&self) -> StateListener {
        self.state.create_state_listener()
    }
}
