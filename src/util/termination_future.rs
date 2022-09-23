/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::sync::broadcast::error::RecvError;

use crate::StateListener;

// ever successfully tried to make a self referencing struct? i didnt, which is why this exists
async fn wrapper(mut listener: StateListener) {
    loop {
        let res = listener.recv().await;

        match res {
            Ok((state, _)) if state.is_terminating() => return,
            Err(RecvError::Closed) => return,
            _ => {}
        }
    }
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct TerminationFuture {
    wrapper: Pin<Box<dyn Future<Output=()> + Send>>
}

impl TerminationFuture {
    pub fn new(listener: StateListener) -> Self {
        Self {
            wrapper: Box::pin(wrapper(listener))
        }
    }
}

impl Future for TerminationFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.wrapper).poll(cx)
    }
}