/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::signal;
use tracing::{error, info};

use crate::context::state::ClusterState;
use crate::TerminationFuture;
use crate::util::StateUpdater;

struct SignalListener {
    snd: StateUpdater,
    fut: Pin<Box<dyn Future<Output=io::Result<()>> + Send>>,
    term: TerminationFuture
}

impl SignalListener {
    fn new(snd: StateUpdater) -> Self {
        Self {
            fut: Box::pin(signal::ctrl_c()),
            term: TerminationFuture::new(snd.subscribe()),
            snd
        }
    }
}

impl Future for SignalListener {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(signal_res) = Pin::new(&mut self.fut).poll(cx) {
            match signal_res {
                Ok(()) => {
                    info!("Received shutdown signal, terminating cluster!");
                    let _ = self.snd.send((ClusterState::Terminating, false));
                },
                Err(err) => {
                    error!(failed_with = err.to_string(), "An Exception occurred while waiting for the shutdown signal, shutting down!");
                    let _ = self.snd.send((ClusterState::Crashing, false));
                }
            }

            return Poll::Ready(())
        }

        Pin::new(&mut self.term).poll(cx)
    }
}

pub fn start_signal_listener(snd: StateUpdater) {
    info!("Starting Signal Listener");
    tokio::spawn(SignalListener::new(snd));
}