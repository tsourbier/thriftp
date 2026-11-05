use std::time::{Duration, Instant};

use tokio::time::sleep;

/// Global token bucket shared by all active transfers.
#[derive(Debug, Clone)]
pub struct BandwidthLimiter {
    rate: u64,
    capacity: u64,
    inner: std::sync::Arc<tokio::sync::Mutex<Bucket>>,
}

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

impl BandwidthLimiter {
    /// Create a limiter in bytes per second, or `None` for unlimited.
    #[must_use]
    pub fn new(bytes_per_second: Option<u64>) -> Option<Self> {
        let rate = bytes_per_second?;
        if rate == 0 {
            return None;
        }
        Some(Self {
            rate,
            capacity: rate,
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(Bucket {
                tokens: rate as f64,
                last_refill: Instant::now(),
            })),
        })
    }

    #[must_use]
    pub fn rate(&self) -> u64 {
        self.rate
    }

    /// Wait until `bytes` may be sent under the configured rate.
    pub async fn acquire(&self, bytes: u64) {
        let bytes = bytes.min(self.capacity).max(1);
        loop {
            let wait = {
                let mut bucket = self.inner.lock().await;
                let now = Instant::now();
                let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
                bucket.tokens =
                    (bucket.tokens + elapsed * self.rate as f64).min(self.capacity as f64);
                bucket.last_refill = now;
                if bucket.tokens >= bytes as f64 {
                    bucket.tokens -= bytes as f64;
                    return;
                }
                let missing = bytes as f64 - bucket.tokens;
                Duration::from_secs_f64(missing / self.rate as f64)
            };
            sleep(wait).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unlimited_limiter_is_absent() {
        assert!(BandwidthLimiter::new(None).is_none());
        assert!(BandwidthLimiter::new(Some(0)).is_none());
    }

    #[tokio::test]
    async fn acquisition_consumes_tokens() {
        let limiter = BandwidthLimiter::new(Some(64 * 1024)).unwrap();
        let started = Instant::now();
        limiter.acquire(16 * 1024).await;
        // The bucket starts full, so the first chunk should not be delayed.
        assert!(started.elapsed() < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn large_requests_are_clamped_to_capacity() {
        let limiter = BandwidthLimiter::new(Some(1024)).unwrap();
        limiter.acquire(10 * 1024).await;
        assert_eq!(limiter.rate(), 1024);
    }
}
