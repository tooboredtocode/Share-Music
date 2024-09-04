/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use this_state::State as ThisState;
use twilight_gateway::error::ReceiveMessageError;
use twilight_gateway::stream::{ShardEventStream, ShardRef};
use twilight_gateway::Shard;
use twilight_model::gateway::event::Event;

use crate::context::ClusterState;
use crate::util::{create_termination_future, TerminationFuture};

pub struct EventStreamPoller<'a> {
    event_stream: ShardEventStream<'a>,
    term: Pin<Box<TerminationFuture>>,
}

impl<'a> EventStreamPoller<'a> {
    pub fn new(
        shards: impl Iterator<Item = &'a mut Shard>,
        state: &ThisState<ClusterState>,
    ) -> Self {
        Self {
            event_stream: ShardEventStream::new(shards),
            term: Box::pin(create_termination_future(state)),
        }
    }
}

impl<'a> Stream for EventStreamPoller<'a> {
    type Item = (ShardRef<'a>, Result<Event, ReceiveMessageError>);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if Pin::new(&mut self.term).poll(cx).is_ready() {
            return Poll::Ready(None);
        }

        Pin::new(&mut self.event_stream).poll_next(cx)
    }
}
