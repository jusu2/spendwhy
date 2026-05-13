//! Opaque, serialisable timestamp.
//!
//! ArchForge deliberately does **not** leak `std::time::SystemTime` or
//! `chrono`/`time` types across Port boundaries. This keeps contracts stable
//! when the underlying time library changes, and lets DTOs round-trip through
//! JSON/protobuf without ad-hoc encoders.

use serde::{Deserialize, Serialize};

/// Milliseconds since the Unix epoch (1970-01-01T00:00:00Z).
///
/// Wraps `i64`; supports negative values for historical timestamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Now, derived from `SystemTime`. Saturates to `i64::MAX` if the clock is
    /// somehow > 292 million years past the epoch.
    pub fn now() -> Self {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_is_after_2020() {
        let now = Timestamp::now();
        // 2020-01-01T00:00:00Z in ms.
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
}
