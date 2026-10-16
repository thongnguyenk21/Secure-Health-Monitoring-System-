//! A small in-memory sliding-window rate limiter (07 §3.6).
//!
//! One backend instance runs per deployment, so process memory is enough; the
//! limiter forgets everything on restart, which only makes it more lenient.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

/// Allows at most `max` events per key within `window`.
#[derive(Debug)]
pub struct RateLimiter<K> {
    max: usize,
    window: Duration,
    events: Mutex<HashMap<K, Vec<Instant>>>,
}

impl<K: Eq + Hash + Clone> RateLimiter<K> {
    /// Creates a limiter.
    ///
    /// # Arguments
    /// * `max` - events allowed per key within one window.
    /// * `window` - length of the sliding window.
    #[must_use]
    pub fn new(max: usize, window: Duration) -> Self {
        Self {
            max,
            window,
            events: Mutex::new(HashMap::new()),
        }
    }

    /// Records one event for `key` if the limit allows it.
    ///
    /// # Returns
    /// `Ok(())` when allowed, or `Err(seconds)` to wait before the next try.
    pub fn check(&self, key: &K, now: Instant) -> Result<(), u64> {
        let mut map = self.events.lock().unwrap_or_else(PoisonError::into_inner);
        if map.len() > 10_000 {
            // Bound memory under a flood of distinct keys.
            map.retain(|_, times| {
                times
                    .last()
                    .is_some_and(|t| now.duration_since(*t) < self.window)
            });
        }
        let times = map.entry(key.clone()).or_default();
        times.retain(|t| now.duration_since(*t) < self.window);
        if times.len() >= self.max {
            let oldest = times.first().copied().unwrap_or(now);
            let wait = self.window.saturating_sub(now.duration_since(oldest));
            return Err(wait.as_secs().max(1));
        }
        times.push(now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_limit_applies_per_key_and_slides() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        let t0 = Instant::now();
        assert!(limiter.check(&"a", t0).is_ok());
        assert!(limiter.check(&"a", t0).is_ok());
        assert!(limiter.check(&"a", t0).is_err());
        assert!(limiter.check(&"b", t0).is_ok());
        assert!(limiter.check(&"a", t0 + Duration::from_secs(61)).is_ok());
    }
}
