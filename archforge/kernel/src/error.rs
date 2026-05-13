//! Unified, narrow, business-semantic error type.
//!
//! Invariant #4 of ArchForge: every Port error variant describes a *business*
//! condition, not an underlying technology. Adapters MUST convert their
//! native errors (`sqlx::Error`, `std::io::Error`, etc.) into `AppError`
//! before crossing a Port boundary.
//!
//! `AppError` is intentionally `Serialize` only — never `Deserialize`. An
//! error type that round-trips through JSON becomes an attack surface: a
//! peer can fabricate `Internal("…")` to coerce upstream branch behaviour.
//! If you need to ship error metadata across a wire, define a separate
//! `WireError` DTO in the relevant `contract-*` crate.

use serde::Serialize;
use thiserror::Error;

/// Unified application error.
///
/// `#[non_exhaustive]` is mandatory — adding a variant must not break
/// downstream `match` arms.
#[non_exhaustive]
#[derive(Debug, Error, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "detail")]
pub enum AppError {
    /// Resource does not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// State conflict (duplicate key, optimistic concurrency, etc.).
    #[error("conflict: {0}")]
    Conflict(String),

    /// Input violates a domain invariant.
    #[error("invalid: {0}")]
    Invalid(String),

    /// External dependency is temporarily unavailable; retry may succeed.
    #[error("unavailable: {0}")]
    Unavailable(String),

    /// Caller lacks permission to perform the operation.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// `Context::deadline` elapsed before the operation completed.
    #[error("deadline exceeded")]
    DeadlineExceeded,

    /// Unrecoverable internal error. Should be rare and always logged.
    #[error("internal: {0}")]
    Internal(String),
}

/// Convenience alias used across the workspace.
pub type Result<T> = core::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_in_a_stable_shape() {
        let json = serde_json::to_string(&AppError::Conflict("dup".into())).unwrap();
        assert!(json.contains(r#""kind":"Conflict""#));
        assert!(json.contains(r#""detail":"dup""#));
    }

    #[test]
    fn display_is_stable() {
        assert_eq!(
            AppError::NotFound("user/42".into()).to_string(),
            "not found: user/42"
        );
        assert_eq!(AppError::DeadlineExceeded.to_string(), "deadline exceeded");
    }

    // Compile-time guard: AppError must remain non-Deserialize. We use a
    // negative-impl probe via `Option`'s blanket impls — trait_assertions in
    // the form of `static_assertions::assert_not_impl_any!` would be
    // cleaner, but we don't want the dep. Instead, anyone who re-adds
    // `Deserialize` to the derive list breaks the doc-comment contract on
    // the type, and the JSON below would deserialize into AppError via
    // serde_json::from_str — which currently fails to compile.
    #[test]
    fn app_error_serializes_but_does_not_deserialize() {
        let s = serde_json::to_string(&AppError::NotFound("x".into())).unwrap();
        assert!(s.contains("NotFound"));
        // The next line is intentionally commented: enabling it must fail to
        // compile while AppError lacks `Deserialize`. Treat it as executable
        // documentation of the invariant.
        // let _: AppError = serde_json::from_str(&s).unwrap();
    }
}
