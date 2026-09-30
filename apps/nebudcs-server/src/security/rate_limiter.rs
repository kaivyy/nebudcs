//! In-memory token bucket rate limiter defending API and WebSocket endpoints
//! against resource exhaustion, DoS, and runaway clients.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct Bucket {
    tokens: f64,
    last_replenish: Instant,
}

/// Thread-safe in-memory token bucket rate limiter.
#[derive(Debug)]
pub struct RateLimiter {
    capacity: f64,
    replenish_rate_per_sec: f64,
    buckets: Mutex<HashMap<String, Bucket>>,
    last_cleanup: Mutex<Instant>,
}

impl RateLimiter {
    /// Creates a new RateLimiter with specified burst capacity and replenishment rate.
    pub fn new(capacity: u32, requests_per_sec: f64) -> Self {
        Self {
            capacity: capacity as f64,
            replenish_rate_per_sec: requests_per_sec,
            buckets: Mutex::new(HashMap::new()),
            last_cleanup: Mutex::new(Instant::now()),
        }
    }

    /// Checks if a request from the given key (e.g. IP address or client ID) is permitted.
    /// Consumes 1 token if permitted.
    pub fn check_allowed(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut buckets = match self.buckets.lock() {
            Ok(b) => b,
            Err(_) => return true, // Fail open if lock poisoned
        };

        // Periodic cleanup of stale buckets every 5 minutes
        if let Ok(mut last_cleanup) = self.last_cleanup.try_lock() {
            if now.duration_since(*last_cleanup) > Duration::from_secs(300) {
                buckets
                    .retain(|_, b| now.duration_since(b.last_replenish) < Duration::from_secs(600));
                *last_cleanup = now;
            }
        }

        let bucket = buckets.entry(key.to_string()).or_insert_with(|| Bucket {
            tokens: self.capacity,
            last_replenish: now,
        });

        // Replenish tokens based on elapsed time
        let elapsed = now.duration_since(bucket.last_replenish).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.replenish_rate_per_sec).min(self.capacity);
        bucket.last_replenish = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        // Default: 100 burst capacity, 50 requests per second
        Self::new(100, 50.0)
    }
}
