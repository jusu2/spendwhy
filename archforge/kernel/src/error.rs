//! Unified, narrow, business-semantic error type.
//!
//! Invariant #4 of ArchForge: every Port error variant describes a *business*
//! condition, not an underlying technology. Adapters MUST convert their
//! native errors (`sqlx::Error`, `std::io::Error`, etc.) into `AppError`
//! before crossing a Port boundary.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Unified application error.
///
/// `#[non_exhaustive]` is mandatory — adding a variant must not break
/// downstream `match` arms.
#[non_exhaustive]
#[derive(Debug, Error, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    fn round_trips_through_serde_json() {
        let e = AppError::Conflict("dup".into());
        let s = serde_json::to_string(&e).unwrap();
        let back: AppError = serde_json::from_str(&s).unwrap();
        assert_eq!(e, back);
    }

    #[test]
    fn display_is_stable() {
        assert_eq!(
            AppError::NotFound("user/42".into()).to_string(),
            "not found: user/42"
        );
        assert_eq!(AppError::DeadlineExceeded.to_string(), "deadline exceeded");
    }
}
