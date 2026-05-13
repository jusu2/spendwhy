//! Opaque, serialisable timestamp.
//!
//! ArchForge deliberately does **not** leak `std::time::SystemTime` or
//! `chrono`/`time` types across Port boundaries. This keeps contracts stable
//! when the underlying time library changes, and lets DTOs round-trip through
//! JSON/protobuf without ad-hoc encoders.
//!
//! `Timestamp::now_from_system()` is the *only* place in the kernel that
//! touches `std::time`. Application code should depend on
//! [`crate::Clock`] instead.

use serde::{Deserialize, Serialize};

/// Milliseconds since the Unix epoch (1970-01-01T00:00:00Z).
///
/// Wraps `i64`; supports negative values for historical timestamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(i64);

impl Timestamp {
    /// **Internal** — used by [`crate::SystemClock`]. Application code must
    /// route through `&dyn Clock` instead so that tests can inject a
    /// [`crate::FixedClock`].
    pub(crate) fn now_from_system() -> Self {
        let ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
            .unwrap_or(0);
        Self(ms)
    }

    /// Build from explicit millis. `const`-friendly for tests and fixtures.
    pub const fn from_ms(ms: i64) -> Self {
        Self(ms)
    }

    /// Inner millis.
    pub const fn as_ms(&self) -> i64 {
        self.0
    }

    /// Saturating addition in milliseconds.
    pub const fn saturating_add_ms(self, ms: i64) -> Self {
        Self(self.0.saturating_add(ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_from_system_is_after_2020() {
        let now = Timestamp::now_from_system();
        assert!(now.as_ms() > 1_577_836_800_000);
    }

    #[test]
    fn ord_is_natural() {
        let a = Timestamp::from_ms(100);
        let b = Timestamp::from_ms(200);
        assert!(a < b);
    }

    #[test]
    fn serde_is_transparent() {
        let t = Timestamp::from_ms(42);
        let s = serde_json::to_string(&t).unwrap();
        assert_eq!(s, "42");
        let back: Timestamp = serde_json::from_str("42").unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn saturating_add_does_not_overflow() {
        let t = Timestamp::from_ms(i64::MAX - 1);
        assert_eq!(t.saturating_add_ms(100).as_ms(), i64::MAX);
    }
}
