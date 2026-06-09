use crate::util::atomic_queue::AtomicQueue;
use crate::util::token_bucket::TokenBucket;
use std::time::Duration;
use tracing::debug;

pub(super) struct OdesliRateLimiter {
    bucket: TokenBucket,
    queue: AtomicQueue,
}

impl OdesliRateLimiter {
    pub fn new(hourly_limit: u32) -> Self {
        Self {
            bucket: TokenBucket::new(
                hourly_limit as usize,
                Duration::from_millis(3_600_000 / hourly_limit as u64),
            ),
            queue: AtomicQueue::new(),
        }
    }

    pub async fn acquire(&self) {
        // Increment the queue and get the current position
        let queue_guard = self.queue.enter();

        loop {
            let queue_position = queue_guard.position();
            if queue_position == 0 && self.bucket.consume(1).is_ok() {
                debug!("Odesli rate limit token acquired, proceeding with request.");
                queue_guard.leave();
                return; // Successfully acquired a token, we can proceed with the request
            }

            // We are either not at the front of the queue or failed to acquire a token, so we need to wait.
            let wait_time = self.bucket.time_for_tokens(queue_position + 1);

            debug!(
                "Odesli rate limit reached. Queue position: {}, waiting for {:?} before retrying.",
                queue_position, wait_time
            );
            tokio::time::sleep(wait_time).await;
        }
    }
}
