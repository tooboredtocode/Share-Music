/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use core::fmt;
use prometheus_client::encoding::{EncodeMetric, MetricEncoder};
use prometheus_client::metrics::{MetricType, TypedMetric};
use prometheus_client::registry;
use std::fmt::Error;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::debug;

pub(super) struct OdesliRateLimiter {
    semaphore: Arc<Semaphore>,
    jh: JoinHandle<()>,
}

impl OdesliRateLimiter {
    pub fn new(hourly_limit: usize) -> Self {
        Self::new_with_initial_tokens(hourly_limit, 0)
    }

    pub fn new_with_initial_tokens(hourly_limit: usize, initial_tokens: usize) -> Self {
        let semaphore = Arc::new(Semaphore::new(initial_tokens));
        let jh = tokio::spawn(Self::bucket_fill_task(semaphore.clone(), hourly_limit));

        Self { semaphore, jh }
    }

    async fn bucket_fill_task(semaphore: Arc<Semaphore>, hourly_limit: usize) {
        let duration = Duration::from_secs_f64(3600.0 / hourly_limit as f64);
        let mut interval = interval(duration);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Initial tick returns immediately, so we poll it first before entering the loop
        interval.tick().await;

        loop {
            interval.tick().await;

            if semaphore.available_permits() < hourly_limit {
                semaphore.add_permits(1);
            }
        }
    }

    pub fn register_metric(&self, registry: &mut registry::Registry) {
        registry.register(
            "odesli_rate_limit_tokens",
            "Number of tokens currently available in the Odesli rate limiter",
            SemaphoreGaugeMetric::new(&self.semaphore),
        );
    }

    /// Acquires a token from the bucket, waiting if necessary until one is available.
    pub async fn acquire(&self) {
        let permit = self
            .semaphore
            .acquire()
            .await
            .expect("Semaphore should never be closed");
        // Don't release the permit back to the semaphore
        permit.forget();

        debug!("Odesli rate limit token acquired, proceeding with request.");
    }
}

impl Drop for OdesliRateLimiter {
    fn drop(&mut self) {
        // Prevent the background task from running indefinitely
        self.jh.abort();
    }
}

/// Wrapper type to implement `Metric' for `Arc<Semaphore>`
struct SemaphoreGaugeMetric(Arc<Semaphore>);

impl SemaphoreGaugeMetric {
    fn new(semaphore: &Arc<Semaphore>) -> Self {
        Self(semaphore.clone())
    }
}

impl fmt::Debug for SemaphoreGaugeMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Match the debug output of a standard gauge metric for consistency
        f.debug_struct("Gauge")
            .field("value", &self.0.available_permits())
            .field("phantom", &PhantomData::<u64>)
            .finish()
    }
}

impl TypedMetric for SemaphoreGaugeMetric {
    const TYPE: MetricType = MetricType::Gauge;
}

impl EncodeMetric for SemaphoreGaugeMetric {
    fn encode(&self, mut encoder: MetricEncoder) -> Result<(), Error> {
        encoder.encode_gauge(&self.0.available_permits())
    }

    fn metric_type(&self) -> MetricType {
        Self::TYPE
    }
}
