//! Reconnect backoff of the clients: exponential, capped, with "equal jitter" so a hub restart
//! does not get every admin reconnecting on the same millisecond.

/// Backoff policy. Delay of attempt `n` (0-based) before jitter: `min(cap, base * 2^n)`;
/// the jitter keeps it in `[delay / 2, delay)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Backoff {
    pub base_ms: u64,
    pub cap_ms: u64,
    /// From this many consecutive failures on, the client reports itself offline (it keeps
    /// retrying at the capped delay).
    pub offline_after: u32,
}

impl Default for Backoff {
    fn default() -> Self {
        Self { base_ms: 500, cap_ms: 30_000, offline_after: 6 }
    }
}

impl Backoff {
    /// Capped exponential delay before jitter.
    pub fn ceiling(&self, attempt: u32) -> u64 {
        let factor = 1u64.checked_shl(attempt.min(32)).unwrap_or(u64::MAX);
        self.base_ms.saturating_mul(factor).min(self.cap_ms)
    }

    /// Delay for `attempt` with `jitter` in `[0, 1)` (any value is clamped into it).
    pub fn delay(&self, attempt: u32, jitter: f64) -> u64 {
        let ceiling = self.ceiling(attempt);
        let j = if jitter.is_finite() {
            jitter.clamp(0.0, 0.999_999)
        } else {
            0.0
        };
        let half = ceiling / 2;
        half + ((ceiling - half) as f64 * j) as u64
    }

    pub fn is_offline(&self, attempt: u32) -> bool {
        attempt >= self.offline_after
    }
}

/// Tiny deterministic generator for the jitter (no `rand` in the wasm bundle).
#[derive(Clone, Copy, Debug)]
pub struct Jitter(u64);

impl Jitter {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    /// Next value in `[0, 1)` (xorshift64*).
    pub fn next_unit(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grows_exponentially_then_caps() {
        let b = Backoff::default();
        assert_eq!(b.ceiling(0), 500);
        assert_eq!(b.ceiling(1), 1_000);
        assert_eq!(b.ceiling(4), 8_000);
        assert_eq!(b.ceiling(6), 30_000);
        assert_eq!(b.ceiling(200), 30_000, "no overflow on long outages");
    }

    #[test]
    fn jitter_stays_in_upper_half() {
        let b = Backoff::default();
        for attempt in 0..10 {
            let c = b.ceiling(attempt);
            assert_eq!(b.delay(attempt, 0.0), c / 2);
            assert!(b.delay(attempt, 0.999) < c);
            assert!(b.delay(attempt, 0.5) >= c / 2);
        }
        assert_eq!(b.delay(3, f64::NAN), b.ceiling(3) / 2);
        assert!(b.delay(3, 7.0) < b.ceiling(3));
    }

    #[test]
    fn offline_after_threshold() {
        let b = Backoff::default();
        assert!(!b.is_offline(5));
        assert!(b.is_offline(6));
    }

    #[test]
    fn jitter_generator_is_unit_interval() {
        let mut j = Jitter::new(42);
        for _ in 0..1000 {
            let v = j.next_unit();
            assert!((0.0..1.0).contains(&v));
        }
    }
}
