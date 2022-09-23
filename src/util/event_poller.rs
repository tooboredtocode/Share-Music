/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use twilight_gateway::cluster::Events;
use twilight_model::gateway::event::Event;

use crate::{StateListener, TerminationFuture};

pub struct EventPoller {
    events: Events,
    term: TerminationFuture
}

impl EventPoller {
    pub fn new(events: Events, listener: StateListener) -> Self {
        Self {
            events,
            term: TerminationFuture::new(listener)
        }
    }
}

impl Stream for EventPoller {
    type Item = (u64, Event);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Poll::Ready(_) = Pin::new(&mut self.term).poll(cx) {
            return Poll::Ready(None)
        }

        Pin::new(&mut self.events).poll_next(cx)
    }
}