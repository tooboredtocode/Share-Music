/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Duration;

use prometheus_client::metrics::gauge::Gauge;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::debug;

type RateLimitMetric = Gauge<u64, AtomicU64>;

pub(super) struct OdesliRateLimiter {
    semaphore: Arc<Semaphore>,
    jh: JoinHandle<()>,
    metric: RateLimitMetric,
}

impl OdesliRateLimiter {
    pub fn new(hourly_limit: usize, metric: RateLimitMetric) -> Self {
        Self::new_with_initial_tokens(hourly_limit, 0, metric)
    }

    pub fn new_with_initial_tokens(
        hourly_limit: usize,
        initial_tokens: usize,
        metric: RateLimitMetric,
    ) -> Self {
        let semaphore = Arc::new(Semaphore::new(initial_tokens));
        let jh = tokio::spawn(Self::bucket_fill_task(
            semaphore.clone(),
            metric.clone(),
            hourly_limit,
        ));

        Self {
            semaphore,
            jh,
            metric,
        }
    }

    async fn bucket_fill_task(
        semaphore: Arc<Semaphore>,
        metric: RateLimitMetric,
        hourly_limit: usize,
    ) {
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
            metric.set(semaphore.available_permits() as u64);
        }
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
        self.metric.dec();

        debug!("Odesli rate limit token acquired, proceeding with request.");
    }
}

impl Drop for OdesliRateLimiter {
    fn drop(&mut self) {
        // Prevent the background task from running indefinitely
        self.jh.abort();
    }
}
