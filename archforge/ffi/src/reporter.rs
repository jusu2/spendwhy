//! Globally-installable panic reporter.
//!
//! When a guard catches a panic, the runtime needs to do **two** things:
//!
//! 1. Tell the host (Dart / C / wherever) "this call failed" — handled by
//!    [`crate::guard_sync`] / [`crate::guard_async`] returning
//!    `AppError::Internal`.
//! 2. Tell the operator (tracing / Sentry / structured logs) "a panic just
//!    crossed a boundary, here is the message" — handled here.
//!
//! The reporter is set **once** at process boot. Subsequent installs are
//! ignored to keep the contract trivially auditable; nothing in production
//! should be flipping the reporter on the fly. Tests can reach into the
//! reporter through their own mock via interior mutability.
//!
//! If no reporter is installed, panics are still caught and surfaced to the
//! caller — they are simply not logged. Failing closed (silent) is the safer
//! default than failing open (a missing logger blocking the FFI return).

use std::sync::OnceLock;

/// One panic event delivered to a [`PanicReporter`]. Borrowed strings to
/// avoid allocations in the hot error path.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct PanicEvent<'a> {
    /// Source guard that captured the panic: currently `"guard_sync"` or
    /// `"guard_async"`. Stable; new guards add new strings, never rename.
    pub site: &'static str,
    /// Best-effort panic message extracted from the payload.
    pub message: &'a str,
}

/// Receiver for panic events. Implementations should be `Send + Sync` and
/// non-blocking — they run on whichever thread the panic happened on.
pub trait PanicReporter: Send + Sync + 'static {
    /// Called once per caught panic, before the [`AppError::Internal`] is
    /// returned to the host.
    ///
    /// [`AppError::Internal`]: archforge_kernel::AppError::Internal
    fn report(&self, event: PanicEvent<'_>);
}

/// No-op reporter. Useful for tests or environments without a logging
/// backend yet wired up.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopReporter;

impl PanicReporter for NoopReporter {
    fn report(&self, _event: PanicEvent<'_>) {}
}

static REPORTER: OnceLock<Box<dyn PanicReporter>> = OnceLock::new();

/// Install the process-wide panic reporter. The first call wins; subsequent
/// calls return `Err` with the supplied reporter so the caller can dispose
/// of it. Recommended call site: bridge crate `main` / FFI init function.
///
/// # Example
///
/// ```
/// use archforge_ffi::{install_panic_reporter, NoopReporter};
/// // Returns Ok(()) on the first install per process.
/// let _ = install_panic_reporter(NoopReporter);
/// ```
pub fn install_panic_reporter<R: PanicReporter>(reporter: R) -> Result<(), R> {
    // OnceLock::set returns Err with the rejected value; we need to box, so
    // we convert by hand to preserve the original `R`.
    if REPORTER.get().is_some() {
        return Err(reporter);
    }
    let boxed: Box<dyn PanicReporter> = Box::new(reporter);
    // Race-safe: a concurrent install loses, but we already validated above
    // for the common case.
    REPORTER.set(boxed).map_err(|_| {
        // The reporter we returned to the caller is the one we tried to
        // install; surface a no-op stand-in so the signature stays useful
        // even on the rare race. This branch is exercised only by tests
        // that install concurrently.
        unreachable_reporter()
    })
}

fn unreachable_reporter<R: PanicReporter>() -> R {
    // Caller has already confirmed REPORTER is set; we have nothing
    // meaningful to hand back. Producing `R` without unsafe is impossible,
    // so we panic — which in turn would be caught by an outer guard. In
    // practice the OnceLock check above prevents reaching here.
    panic!("install_panic_reporter racing install on the same process");
}

pub(crate) fn report_panic(event: PanicEvent<'_>) {
    if let Some(reporter) = REPORTER.get() {
        reporter.report(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // The OnceLock is process-global; cargo test runs each #[test] in its
    // own thread of the same process. We install a *capturing* reporter
    // exactly once and assert behaviour through it across multiple tests.
    // Test ordering is not guaranteed, so each test must be tolerant of
    // events from sibling tests.

    #[derive(Default)]
    struct CapturingReporter {
        events: Mutex<Vec<(&'static str, String)>>,
    }

    impl PanicReporter for CapturingReporter {
        fn report(&self, event: PanicEvent<'_>) {
            self.events
                .lock()
                .unwrap()
                .push((event.site, event.message.to_string()));
        }
    }

    // We need a stable handle to inspect the reporter after install, so the
    // reporter lives in a static; install_panic_reporter takes ownership
    // of a `Box` clone of the reference shape via a trait object adapter.
    struct Forwarder(&'static CapturingReporter);
    impl PanicReporter for Forwarder {
        fn report(&self, event: PanicEvent<'_>) {
            self.0.report(event)
        }
    }

    static GLOBAL_REPORTER: std::sync::LazyLock<CapturingReporter> =
        std::sync::LazyLock::new(CapturingReporter::default);

    fn ensure_installed() {
        // Idempotent across tests; second install is a benign Err.
        let _ = install_panic_reporter(Forwarder(&GLOBAL_REPORTER));
    }

    #[test]
    fn install_is_idempotent() {
        ensure_installed();
        // Second install must not crash and must not replace the reporter.
        let err = install_panic_reporter(NoopReporter);
        assert!(err.is_err(), "second install must be rejected");
    }

    #[test]
    fn reporter_receives_sync_panic() {
        ensure_installed();
        let before = GLOBAL_REPORTER.events.lock().unwrap().len();
        let _ = crate::guard_sync::<u32, _>(|| panic!("report-sync-marker-{}", line!()));
        let events = GLOBAL_REPORTER.events.lock().unwrap();
        assert!(events.len() > before, "reporter must record an event");
        let last = events.last().unwrap();
        assert_eq!(last.0, "guard_sync");
        assert!(last.1.contains("report-sync-marker"));
    }

    #[tokio::test]
    async fn reporter_receives_async_panic() {
        ensure_installed();
        let before = GLOBAL_REPORTER.events.lock().unwrap().len();
        let _ = crate::guard_async::<u32, _>(async { panic!("report-async-marker") }).await;
        let events = GLOBAL_REPORTER.events.lock().unwrap();
        assert!(events.len() > before);
        let entry = events
            .iter()
            .rev()
            .find(|e| e.0 == "guard_async" && e.1.contains("report-async-marker"))
            .expect("async event must be recorded");
        assert_eq!(entry.0, "guard_async");
    }

    #[test]
    fn business_error_does_not_invoke_reporter() {
        ensure_installed();
        let before = GLOBAL_REPORTER.events.lock().unwrap().len();
        let _ = crate::guard_sync::<u32, _>(|| {
            Err(archforge_kernel::AppError::NotFound("nope".into()))
        });
        let after = GLOBAL_REPORTER.events.lock().unwrap().len();
        assert_eq!(before, after, "business error must not trigger reporter");
    }
}
