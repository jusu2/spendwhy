//! Panic-isolation guards.
//!
//! The two entry points — [`guard_sync`] and [`guard_async`] — share three
//! design choices that you should understand before using or extending them:
//!
//! 1. **Catch is final.** A caught panic is converted to
//!    [`AppError::Internal`] with the prefix [`PANIC_INTERNAL_TAG`]
//!    (`"panic: "`) so downstream code can distinguish "the use case
//!    intentionally signalled Internal" from "the runtime caught a panic"
//!    without parsing the message. The tag is part of the public contract.
//!
//! 2. **No double catch.** Wrapping a guarded call inside another guard is
//!    safe but redundant: nested guards return the inner result verbatim
//!    because the inner guard already converted the panic before the outer
//!    one could see it. This keeps reporting non-duplicated.
//!
//! 3. **`AssertUnwindSafe` is a promise the caller makes.** The closure
//!    being guarded must not leave invariants broken if it panics
//!    mid-execution (no half-written files held inside a `Mutex`, etc.).
//!    For shared state, prefer `parking_lot::Mutex` (no poisoning) or
//!    explicit rollback in `Drop`.

use std::panic::AssertUnwindSafe;

use archforge_kernel::AppError;
use futures::FutureExt;

use crate::reporter::{report_panic, PanicEvent};

/// The fixed prefix attached to every `AppError::Internal` produced by a
/// caught panic. Stable across versions; do not depend on the remainder of
/// the message.
pub const PANIC_INTERNAL_TAG: &str = "panic: ";

/// Synchronous panic guard.
///
/// Runs `f` to completion. On panic, the payload is extracted, fed to the
/// installed [`PanicReporter`], and the call returns
/// `Err(AppError::Internal("panic: <message>"))`. Business `Err` values are
/// passed through unchanged.
///
/// [`PanicReporter`]: crate::PanicReporter
///
/// # Example
///
/// ```
/// use archforge_ffi::guard_sync;
/// use archforge_kernel::AppError;
///
/// let r: Result<u32, AppError> = guard_sync(|| Ok(42));
/// assert_eq!(r.unwrap(), 42);
///
/// let r: Result<u32, AppError> = guard_sync(|| panic!("oops"));
/// assert!(matches!(r, Err(AppError::Internal(_))));
/// ```
pub fn guard_sync<T, F>(f: F) -> Result<T, AppError>
where
    F: FnOnce() -> Result<T, AppError>,
{
    match std::panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(ok) => ok,
        Err(payload) => Err(panic_to_app_error(payload, "guard_sync")),
    }
}

/// Asynchronous panic guard.
///
/// Awaits `fut` to completion. On panic at any `.await` boundary, the panic
/// is caught, reported, and converted to
/// `Err(AppError::Internal("panic: <message>"))`. Business `Err` values are
/// passed through unchanged.
///
/// # Example
///
/// ```
/// # use archforge_ffi::guard_async;
/// # use archforge_kernel::AppError;
/// # async fn run() {
/// let r: Result<&'static str, AppError> = guard_async(async { Ok("hi") }).await;
/// assert_eq!(r.unwrap(), "hi");
///
/// let r: Result<u8, AppError> = guard_async(async { panic!("nope") }).await;
/// assert!(matches!(r, Err(AppError::Internal(_))));
/// # }
/// # futures::executor::block_on(run());
/// ```
pub async fn guard_async<T, F>(fut: F) -> Result<T, AppError>
where
    F: std::future::Future<Output = Result<T, AppError>>,
{
    match AssertUnwindSafe(fut).catch_unwind().await {
        Ok(ok) => ok,
        Err(payload) => Err(panic_to_app_error(payload, "guard_async")),
    }
}

fn panic_to_app_error(payload: Box<dyn std::any::Any + Send>, site: &'static str) -> AppError {
    let message = extract_panic_message(&*payload);
    report_panic(PanicEvent {
        site,
        message: &message,
    });
    AppError::Internal(format!("{PANIC_INTERNAL_TAG}{message}"))
}

/// Best-effort extraction of a human-readable string from a panic payload.
/// Handles the two common payload types (`&'static str` and `String`); any
/// other type degrades to the sentinel below so callers can still log
/// something deterministic.
fn extract_panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_ok_passes_through() {
        let r: Result<u32, AppError> = guard_sync(|| Ok(7));
        assert_eq!(r.unwrap(), 7);
    }

    #[test]
    fn business_err_passes_through_unchanged() {
        let r: Result<u32, AppError> = guard_sync(|| Err(AppError::NotFound("x".into())));
        assert!(matches!(r, Err(AppError::NotFound(s)) if s == "x"));
    }

    #[test]
    fn static_str_panic_is_caught_with_tag() {
        let r: Result<u32, AppError> = guard_sync(|| panic!("static-str"));
        match r {
            Err(AppError::Internal(msg)) => {
                assert!(msg.starts_with(PANIC_INTERNAL_TAG));
                assert!(msg.contains("static-str"));
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn string_panic_is_caught_with_tag() {
        let r: Result<u32, AppError> = guard_sync(|| panic!("dynamic-{}", String::from("string")));
        match r {
            Err(AppError::Internal(msg)) => {
                assert!(msg.starts_with(PANIC_INTERNAL_TAG));
                assert!(msg.contains("dynamic-string"));
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn non_string_panic_payload_degrades_gracefully() {
        let r: Result<u32, AppError> = guard_sync(|| std::panic::panic_any(42_u32));
        match r {
            Err(AppError::Internal(msg)) => {
                assert!(msg.starts_with(PANIC_INTERNAL_TAG));
                assert!(msg.contains("<non-string panic payload>"));
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn nested_guards_do_not_double_wrap() {
        // The inner guard converts the panic; the outer guard sees the
        // Internal result and forwards it without re-tagging.
        let r: Result<u32, AppError> = guard_sync(|| guard_sync(|| panic!("once")));
        match r {
            Err(AppError::Internal(msg)) => {
                let occurrences = msg.matches(PANIC_INTERNAL_TAG).count();
                assert_eq!(occurrences, 1, "tag must not be doubled: {msg}");
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn async_business_ok_passes_through() {
        let r: Result<&'static str, AppError> = guard_async(async { Ok("hi") }).await;
        assert_eq!(r.unwrap(), "hi");
    }

    #[tokio::test]
    async fn async_panic_is_caught_with_tag() {
        let r: Result<u32, AppError> = guard_async(async { panic!("async-boom") }).await;
        match r {
            Err(AppError::Internal(msg)) => {
                assert!(msg.starts_with(PANIC_INTERNAL_TAG));
                assert!(msg.contains("async-boom"));
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn async_panic_across_await_is_caught() {
        // Ensures the catch survives a yield point.
        let r: Result<u32, AppError> = guard_async(async {
            tokio::task::yield_now().await;
            panic!("after-yield");
        })
        .await;
        assert!(matches!(r, Err(AppError::Internal(_))));
    }
}
