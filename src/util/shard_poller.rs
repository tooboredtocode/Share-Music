/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */
use futures_util::{ready, Stream};
use std::fmt;
use std::fmt::Formatter;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use this_state::State as ThisState;
use twilight_gateway::error::ReceiveMessageError;
use twilight_gateway::{parse, EventTypeFlags, Message, Shard};
use twilight_model::gateway::event::Event;

use crate::context::{ClusterState, Ctx};
use crate::util::{create_termination_future, TerminationFuture};

/// A struct that polls a shard for events and handles termination.
pub struct ShardPoller {
    term: Pin<Box<TerminationFuture>>,
}

#[must_use = "futures do nothing unless polled"]
pub struct ShardPollerFuture<'a> {
    shard: &'a mut Shard,
    poller: &'a mut ShardPoller,
}

/// A struct that represents a fatal error when closing a shard.
#[derive(Debug)]
pub struct FatallyClosedShard;

impl ShardPoller {
    pub fn new(state: &ThisState<ClusterState>) -> Self {
        Self {
            term: Box::pin(create_termination_future(state)),
        }
    }

    pub fn new_from_context(context: &Ctx) -> Self {
        Self::new(&context.state)
    }

    pub fn poll<'a>(
        &'a mut self,
        shard: &'a mut Shard,
    ) -> ShardPollerFuture<'a> {
        ShardPollerFuture {
            shard,
            poller: self,
        }
    }
}

impl Future for ShardPollerFuture<'_> {
    type Output = Result<Option<Result<Event, ReceiveMessageError>>, FatallyClosedShard>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            if Pin::new(&mut self.poller.term).poll(cx).is_ready() {
                return Poll::Ready(Ok(None));
            }

            let Some(next) = ready!(Pin::new(&mut self.shard).poll_next(cx)) else {
                return Poll::Ready(Ok(None));
            };

            if let Some(res) = next.and_then(|message| match message {
                Message::Text(json) => parse(json, EventTypeFlags::INTERACTION_CREATE).map(|opt| opt.map(Into::into)),
                Message::Close(frame) => Ok(Some(Event::GatewayClose(frame))),
            }).transpose() {
                return Poll::Ready(Ok(Some(res)));
            }
        }
    }
}

impl fmt::Display for FatallyClosedShard {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Shard has been fatally closed")
    }
}

impl std::error::Error for FatallyClosedShard {}
