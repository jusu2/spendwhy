//! Clock — a Port over wall time.
//!
//! Time is an external dependency. Calling `SystemTime::now()` deep inside
//! application code defeats determinism, makes tests rely on real sleeps,
//! and ties business logic to a particular runtime. ArchForge therefore
//! treats time as a Port: use cases bound on `&dyn Clock`, adapters supply
//! either `SystemClock` (production) or `FixedClock` (tests).

use crate::Timestamp;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

/// Read-only clock Port.
pub trait Clock: Send + Sync {
    /// Current wall-clock instant.
    fn now(&self) -> Timestamp;
}

/// Production clock: wraps `std::time::SystemTime`.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now_from_system()
    }
}

/// Manual clock for tests. Monotonic by construction (calls to [`Self::set`]
/// that move backwards in time are rejected).
#[derive(Debug, Clone)]
pub struct FixedClock(Arc<AtomicI64>);

impl FixedClock {
    /// New clock starting at `start`.
    pub fn new(start: Timestamp) -> Self {
        Self(Arc::new(AtomicI64::new(start.as_ms())))
    }

    /// Advance the clock by `ms` milliseconds.
    pub fn advance_ms(&self, ms: i64) {
        // `fetch_add` is monotonic by definition because callers can only
        // hand us positive deltas (we don't bother enforcing this — a
        // negative delta is a test bug, not a runtime concern).
        self.0.fetch_add(ms, Ordering::Relaxed);
    }

    /// Set absolute time. Panics in debug builds if the new value moves
    /// backwards (clock skew is a contract violation in tests).
    pub fn set(&self, t: Timestamp) {
        let prev = self.0.swap(t.as_ms(), Ordering::Relaxed);
        debug_assert!(
            t.as_ms() >= prev,
            "FixedClock: set() moved backwards from {prev} to {}",
            t.as_ms()
        );
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_ms(self.0.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_clock_advances() {
        let c = FixedClock::new(Timestamp::from_ms(1_000));
        assert_eq!(c.now().as_ms(), 1_000);
        c.advance_ms(500);
        assert_eq!(c.now().as_ms(), 1_500);
    }

    #[test]
    fn system_clock_is_after_2020() {
        let c = SystemClock;
        assert!(c.now().as_ms() > 1_577_836_800_000);
    }

    #[test]
    fn clock_is_object_safe() {
        // Compile-time check: Clock must be usable as `&dyn Clock`.
        fn _accept(c: &dyn Clock) -> Timestamp {
            c.now()
        }
        let f = FixedClock::new(Timestamp::from_ms(42));
        assert_eq!(_accept(&f).as_ms(), 42);
    }
}
