//! Wire-safe error DTO.
//!
//! ## Why a separate type?
//!
//! ArchForge invariant #4: [`AppError`] is `Serialize`-only — never
//! `Deserialize`. The reasoning is straightforward: if a peer (Dart code,
//! another process, a malicious wire participant) can fabricate an
//! `AppError::Internal("foo")` by sending crafted JSON, upstream branches
//! that switch on the variant become exploitable. The kernel error type
//! therefore has no `Deserialize` impl by design.
//!
//! But every FFI bridge needs to take the error on the wire and *parse* it
//! on the far side. Dart, Swift, Kotlin, C# — all of them want a stable
//! shape with a tag they can `switch` on. So we introduce [`WireError`]: a
//! cousin of `AppError` that
//!
//! - is `Serialize + Deserialize`,
//! - has a redundant `is_panic` boolean so consumers can distinguish a
//!   panic-derived `Internal` from an intentional `Internal` without parsing
//!   the message,
//! - carries a stable `kind` discriminator (lowercase snake_case strings,
//!   never the Rust enum name — that would couple every wire consumer to
//!   Rust's identifier renames).
//!
//! Conversion is **one-way**: `From<AppError> for WireError`. There is no
//! `From<WireError> for AppError`, and there should never be one — the
//! whole point of the asymmetry is to keep `AppError`'s arm set
//! untransferable.

use std::fmt;

use archforge_kernel::AppError;
use serde::{Deserialize, Serialize};

use crate::guard::PANIC_INTERNAL_TAG;

/// Stable wire discriminator for a [`WireError`].
///
/// String values are part of the public contract: cross-language consumers
/// must match on them. Adding a variant here is *backwards-compatible* iff
/// existing consumers handle "unknown kind" — which they should, because
/// the `#[serde(other)]` `Unknown` arm exists for exactly that.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireErrorKind {
    /// Resource does not exist (`AppError::NotFound`).
    NotFound,
    /// State conflict (`AppError::Conflict`).
    Conflict,
    /// Input violates a domain invariant (`AppError::Invalid`).
    Invalid,
    /// Dependency unavailable; retry may succeed (`AppError::Unavailable`).
    Unavailable,
    /// Caller lacks permission (`AppError::Forbidden`).
    Forbidden,
    /// Deadline expired (`AppError::DeadlineExceeded`).
    DeadlineExceeded,
    /// Unrecoverable internal failure (`AppError::Internal`).
    Internal,
    /// Wire received a kind this consumer does not understand. Treat as
    /// terminal but loggable.
    #[serde(other)]
    Unknown,
}

impl fmt::Display for WireErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::Invalid => "invalid",
            Self::Unavailable => "unavailable",
            Self::Forbidden => "forbidden",
            Self::DeadlineExceeded => "deadline_exceeded",
            Self::Internal => "internal",
            Self::Unknown => "unknown",
        };
        f.write_str(s)
    }
}

/// Wire-safe error DTO. See module docs for why this exists.
///
/// Field order is part of the contract; serialised shape is intentionally
/// shallow and JSON-friendly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireError {
    /// Stable wire discriminator.
    pub kind: WireErrorKind,
    /// Human-readable detail. Already redacted of internal-only context by
    /// the time it reaches a wire consumer.
    pub message: String,
    /// `true` iff the corresponding `AppError::Internal` was produced by a
    /// caught panic (its message starts with [`PANIC_INTERNAL_TAG`]).
    /// Consumers can use this to route reporting (Sentry / Crashlytics)
    /// without parsing strings.
    #[serde(default)]
    pub is_panic: bool,
}

impl WireError {
    /// Construct directly. Most callers should use [`From<AppError>`] or
    /// [`Self::from_result`].
    pub fn new(kind: WireErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            is_panic: false,
        }
    }

    /// Mark this error as panic-derived.
    pub fn with_panic_flag(mut self, is_panic: bool) -> Self {
        self.is_panic = is_panic;
        self
    }

    /// Map a `Result<T, AppError>` into a `Result<T, WireError>` for the
    /// outer FFI return type. Equivalent to `result.map_err(WireError::from)`
    /// but reads better at call sites.
    pub fn from_result<T>(result: Result<T, AppError>) -> Result<T, WireError> {
        result.map_err(WireError::from)
    }
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_panic {
            write!(f, "[{}|panic] {}", self.kind, self.message)
        } else {
            write!(f, "[{}] {}", self.kind, self.message)
        }
    }
}

impl std::error::Error for WireError {}

impl From<AppError> for WireError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::NotFound(m) => WireError::new(WireErrorKind::NotFound, m),
            AppError::Conflict(m) => WireError::new(WireErrorKind::Conflict, m),
            AppError::Invalid(m) => WireError::new(WireErrorKind::Invalid, m),
            AppError::Unavailable(m) => WireError::new(WireErrorKind::Unavailable, m),
            AppError::Forbidden(m) => WireError::new(WireErrorKind::Forbidden, m),
            AppError::DeadlineExceeded => {
                WireError::new(WireErrorKind::DeadlineExceeded, "deadline exceeded")
            }
            AppError::Internal(m) => {
                let is_panic = m.starts_with(PANIC_INTERNAL_TAG);
                WireError::new(WireErrorKind::Internal, m).with_panic_flag(is_panic)
            }
            // AppError is `#[non_exhaustive]`; future variants degrade to
            // Internal so callers always get a wire-safe payload. When a new
            // variant lands here, the cargo-public-api gate will surface it
            // for explicit mapping.
            other => WireError::new(WireErrorKind::Internal, other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_variants_map_to_their_kinds() {
        let cases = [
            (AppError::NotFound("x".into()), WireErrorKind::NotFound),
            (AppError::Conflict("y".into()), WireErrorKind::Conflict),
            (AppError::Invalid("z".into()), WireErrorKind::Invalid),
            (
                AppError::Unavailable("a".into()),
                WireErrorKind::Unavailable,
            ),
            (AppError::Forbidden("b".into()), WireErrorKind::Forbidden),
            (AppError::DeadlineExceeded, WireErrorKind::DeadlineExceeded),
        ];
        for (err, expected) in cases {
            let wire: WireError = err.into();
            assert_eq!(wire.kind, expected);
            assert!(!wire.is_panic);
        }
    }

    #[test]
    fn intentional_internal_does_not_flag_panic() {
        let wire: WireError = AppError::Internal("dbpool drained".into()).into();
        assert_eq!(wire.kind, WireErrorKind::Internal);
        assert!(!wire.is_panic);
        assert_eq!(wire.message, "dbpool drained");
    }

    #[test]
    fn panic_tagged_internal_flags_is_panic() {
        let raw = format!("{PANIC_INTERNAL_TAG}explosion in worker thread");
        let wire: WireError = AppError::Internal(raw.clone()).into();
        assert_eq!(wire.kind, WireErrorKind::Internal);
        assert!(wire.is_panic);
        assert_eq!(wire.message, raw);
    }

    #[test]
    fn json_round_trip_is_stable() {
        let wire = WireError::new(WireErrorKind::Conflict, "dup email").with_panic_flag(false);
        let json = serde_json::to_string(&wire).unwrap();
        // Stable shape — consumers depend on these key names.
        assert!(json.contains(r#""kind":"conflict""#), "got {json}");
        assert!(json.contains(r#""message":"dup email""#), "got {json}");
        assert!(json.contains(r#""is_panic":false"#), "got {json}");

        let back: WireError = serde_json::from_str(&json).unwrap();
        assert_eq!(back, wire);
    }

    #[test]
    fn unknown_kind_decays_to_unknown() {
        // Forward compatibility: an older consumer reading a future kind
        // must not crash.
        let json = r#"{"kind":"future_kind_we_havent_invented","message":"hi","is_panic":false}"#;
        let parsed: WireError = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.kind, WireErrorKind::Unknown);
        assert_eq!(parsed.message, "hi");
    }

    #[test]
    fn is_panic_defaults_to_false_if_absent() {
        // Older Rust producers that pre-date the field must still decode.
        let json = r#"{"kind":"internal","message":"older payload"}"#;
        let parsed: WireError = serde_json::from_str(json).unwrap();
        assert!(!parsed.is_panic);
    }

    #[test]
    fn from_result_passes_ok_through() {
        let r: Result<u32, AppError> = Ok(5);
        let mapped: Result<u32, WireError> = WireError::from_result(r);
        assert_eq!(mapped.unwrap(), 5);
    }

    #[test]
    fn from_result_maps_err() {
        let r: Result<u32, AppError> = Err(AppError::NotFound("u/1".into()));
        let mapped: Result<u32, WireError> = WireError::from_result(r);
        let err = mapped.unwrap_err();
        assert_eq!(err.kind, WireErrorKind::NotFound);
        assert_eq!(err.message, "u/1");
    }

    #[test]
    fn display_includes_panic_marker() {
        let wire = WireError::new(WireErrorKind::Internal, "boom").with_panic_flag(true);
        assert_eq!(wire.to_string(), "[internal|panic] boom");
    }
}
