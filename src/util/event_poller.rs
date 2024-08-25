/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use twilight_gateway::error::ReceiveMessageError;
use twilight_gateway::stream::{ShardEventStream, ShardRef};
use twilight_gateway::Shard;
use twilight_model::gateway::event::Event;

use crate::{StateListener, TerminationFuture};

pub struct EventStreamPoller<'a> {
    event_stream: ShardEventStream<'a>,
    term: TerminationFuture,
}

impl<'a> EventStreamPoller<'a> {
    pub fn new(shards: &'a mut Vec<Shard>, listener: StateListener) -> Self {
        Self {
            event_stream: ShardEventStream::new(shards.iter_mut()),
            term: TerminationFuture::new(listener),
        }
    }
}

impl<'a> Stream for EventStreamPoller<'a> {
    type Item = (ShardRef<'a>, Result<Event, ReceiveMessageError>);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Poll::Ready(_) = Pin::new(&mut self.term).poll(cx) {
            return Poll::Ready(None);
        }

        Pin::new(&mut self.event_stream).poll_next(cx)
    }
}
