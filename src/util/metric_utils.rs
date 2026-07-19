/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use std::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::histogram::Histogram;

pub trait TimeFutureExt: Future {
    /// Wraps the future in a `TimeFuture` that measures the time taken to complete the future.
    fn time(self) -> TimeFuture<Self>
    where
        Self: Sized,
    {
        TimeFuture {
            start: Instant::now(),
            future: self,
        }
    }
}

impl<F: Future> TimeFutureExt for F {}

pub struct TimeFuture<F> {
    start: Instant,
    future: F,
}

impl<F: Future> Future for TimeFuture<F> {
    type Output = (F::Output, Duration);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = unsafe {
            // SAFETY: We never move the future, we only access it mutably through this reference.
            self.as_mut().map_unchecked_mut(|s| &mut s.future)
        };
        match inner.poll(cx) {
            Poll::Ready(output) => Poll::Ready((output, self.start.elapsed())),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub trait UnpackErr<T, E> {
    /// Unpacks the `Result` from the first element of the tuple, returning an error if it is `Err`.
    fn unpack_err(self) -> Result<T, E>;
}

macro_rules! impl_unpack_err_for_tuple {
    ($t1:ident, $($t:ident),*) => {
        impl<$t1, $($t,)* E> UnpackErr<($t1, $($t,)*), E> for (Result<$t1, E>, $($t,)*)
        {
            #[allow(non_snake_case)]
            #[inline(always)]
            fn unpack_err(self) -> Result<($t1, $($t,)*), E> {
                match self {
                    (Ok($t1), $($t,)*) => Ok(($t1, $($t,)*)),
                    #[allow(unused_variables)]
                    (Err(e), $($t,)*) => Err(e),
                }
            }
        }
    };
}

impl_unpack_err_for_tuple!(T1, T2);
impl_unpack_err_for_tuple!(T1, T2, T3);
impl_unpack_err_for_tuple!(T1, T2, T3, T4);
impl_unpack_err_for_tuple!(T1, T2, T3, T4, T5);
impl_unpack_err_for_tuple!(T1, T2, T3, T4, T5, T6);
impl_unpack_err_for_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_unpack_err_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_unpack_err_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_unpack_err_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);

/// A trait for types that have a histogram family with a specific label type.
pub trait HasHistogramFamily<L: Clone + Hash + Eq> {
    fn family_with_label(&self) -> &Family<L, Histogram>;
}

pub trait HasHistogramFamilyExt<L: Clone + Hash + Eq>: HasHistogramFamily<L> {
    /// Observes the given duration in the histogram family with the given label.
    fn observe_duration(&self, label: L, duration: Duration) {
        self.family_with_label()
            .get_or_create(&label)
            .observe(duration.as_secs_f64());
    }
}

impl<L: Clone + Hash + Eq, T: HasHistogramFamily<L>> HasHistogramFamilyExt<L> for T {}
