/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

fn usize_now() -> usize {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as usize
}

/// Lock free token bucket implementation for rate limiting.
///
/// ### Functionality
/// We use an atomic usize to store our state, which is initialized with
/// ```
/// state = now_ms / (refill_rate * capacity)
/// ```
/// i.e. the number of total tokens that could have been used since epoch.
///
pub struct TokenBucket {
    capacity: usize,
    time_per_token: usize,    // in milliseconds
    tokens_used: AtomicUsize, // since epoch
}

impl TokenBucket {
    pub fn new(capacity: usize, time_per_token: Duration) -> Self {
        let time_per_token_ms = time_per_token.as_millis() as usize;
        Self {
            capacity,
            time_per_token: time_per_token_ms,
            tokens_used: AtomicUsize::new(usize_now() / time_per_token_ms),
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
        let tokens_used = self.tokens_used.fetch_max(
            max_tokens_used.saturating_sub(self.capacity),
            Ordering::Relaxed,
        );

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
