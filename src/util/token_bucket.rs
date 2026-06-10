/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[cfg(not(test))]
fn usize_now() -> usize {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as usize
}

#[cfg(test)]
mod now_mock {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    static MOCK_NOW: AtomicUsize = AtomicUsize::new(0);

    pub fn reset() {
        MOCK_NOW.store(0, Ordering::SeqCst);
    }

    pub fn advance_by(duration: Duration) {
        MOCK_NOW.fetch_add(duration.as_millis() as usize, Ordering::SeqCst);
    }

    pub fn now() -> usize {
        MOCK_NOW.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
use now_mock::now as usize_now;

/// Lock free token bucket implementation for rate limiting.
///
/// ### Functionality
/// We can avoid using locks by tracking the total number of tokens used since epoch.
/// All tokens that were generated when the bucket was full are considered used.
///
/// The number of tokens currently available can be calculated as:
/// ```text
/// tokens_available = max_tokens_used - tokens_used
/// ```
/// Where:
/// - `max_tokens_used` is the total number of tokens that could have been used since epoch,
///   calculated as `current_time / time_per_token`.
/// - `tokens_used` tracks the actual number of tokens consumed, with the following assumptions:
///   - Every token before the start of the program is assumed to be used (unless we grant initial tokens).
///   - Every token generated when the bucket was full is also considered used.
///
/// This approach allows us to determine the number of tokens available without needing to track the
/// last refill time or use locks, as we can rely on the total tokens used and the current time to
/// calculate availability.
pub struct TokenBucket {
    capacity: usize,
    time_per_token: usize,    // in milliseconds
    tokens_used: AtomicUsize, // since epoch
}

impl TokenBucket {
    /// Creates a new token bucket with the specified capacity and refill rate.
    pub fn new(capacity: usize, time_per_token: Duration) -> Self {
        Self::new_with_initial_tokens(capacity, time_per_token, 0)
    }

    /// Creates a new token bucket with the specified capacity and refill rate, granting
    /// an initial number of tokens immediately available.
    pub fn new_with_initial_tokens(
        capacity: usize,
        time_per_token: Duration,
        initial_tokens: usize,
    ) -> Self {
        let time_per_token_ms = time_per_token.as_millis() as usize;

        let initial_tokens = initial_tokens.min(capacity); // Ensure initial tokens do not exceed capacity
        let tokens_used = (usize_now() / time_per_token_ms).saturating_sub(initial_tokens);
        Self {
            capacity,
            time_per_token: time_per_token_ms,
            tokens_used: AtomicUsize::new(tokens_used),
        }
    }

    pub fn time_for_tokens(&self, tokens: usize) -> Duration {
        if tokens == 0 {
            return Duration::from_millis(0);
        }
        self.time_to_next_token()  // Time until the next token is generated
            + Duration::from_millis(((tokens - 1) * self.time_per_token) as u64) // Time for the remaining tokens after the next one
    }

    fn time_to_next_token(&self) -> Duration {
        let now = usize_now();
        let time_since_last_token = now % self.time_per_token;
        if time_since_last_token == 0 {
            Duration::from_millis(self.time_per_token as u64)
        } else {
            Duration::from_millis((self.time_per_token - time_since_last_token) as u64)
        }
    }

    /// Gets the number of tokens currently available in the bucket.
    #[inline]
    pub fn tokens_available(&self) -> usize {
        self.load_tokens_internal().0
    }

    /// Internal method to get both the number of tokens currently available and the total number
    /// of tokens used since epoch.
    fn load_tokens_internal(&self) -> (usize, usize) {
        // Get the total number of tokens that could have been used since epoch
        let max_tokens_used = usize_now() / self.time_per_token;

        // Get the current number of tokens used, with all unused tokens beyond the capacity
        // marked as used, so we don't exceed the capacity when calculating available tokens.
        let tokens_used = self
            .tokens_used
            .fetch_max(
                max_tokens_used.saturating_sub(self.capacity),
                Ordering::Relaxed,
            )
            .max(max_tokens_used.saturating_sub(self.capacity)); // Ensure tokens_used is at least max_tokens_used - capacity

        // Now we can calculate the number of tokens currently available
        let tokens_available = max_tokens_used.saturating_sub(tokens_used);

        (tokens_available, tokens_used)
    }

    /// Attempts to consume the specified number of tokens from the bucket.
    /// Returns Ok(()) if successful, or Err(missing_tokens) if there are not enough tokens available.
    pub fn consume(&self, tokens: usize) -> Result<(), usize> {
        loop {
            let (tokens_available, tokens_used) = self.load_tokens_internal();
            if tokens_available < tokens {
                return Err(tokens - tokens_available); // Not enough tokens available
            }
            // Attempt to consume the tokens by updating the tokens_used atomically
            let new_tokens_used = tokens_used + tokens;
            if self
                .tokens_used
                .compare_exchange(
                    tokens_used,
                    new_tokens_used,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return Ok(()); // Successfully consumed the tokens
            }
            // If we failed to update, it means another thread modified the state, so we need to retry
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        now_mock::reset();
        let bucket = TokenBucket::new(5, Duration::from_secs(1));

        // Initially, the bucket should have 0 tokens available
        assert_eq!(bucket.tokens_available(), 0);

        // Advance time by 1 second, which should generate 1 token
        now_mock::advance_by(Duration::from_secs(1));
        assert_eq!(bucket.tokens_available(), 1);

        // Advance time by 5 seconds, which should generate 5 tokens, but the bucket capacity is 5, so it should be full
        now_mock::advance_by(Duration::from_secs(5));
        assert_eq!(bucket.tokens_available(), 5);

        // Consume 3 tokens
        assert!(bucket.consume(3).is_ok());
        assert_eq!(bucket.tokens_available(), 2);

        // Advance time by 2 seconds, which should generate 2 tokens, so the bucket should hold 4 tokens now
        now_mock::advance_by(Duration::from_secs(2));
        assert_eq!(bucket.tokens_available(), 4);
    }
}
