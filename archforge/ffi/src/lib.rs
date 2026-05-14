//! # ArchForge FFI Boundary Guards
//!
//! The hard rule at any FFI boundary is **a Rust panic must not unwind into a
//! foreign caller** — doing so is undefined behaviour. This crate provides the
//! three primitives every public entry point needs:
//!
//! 1. [`guard_sync`] / [`guard_async`] — wrap any closure or future, catch
//!    panics, and return a domain-typed [`AppError::Internal`].
//! 2. [`WireError`] — a *separately defined* DTO that is `Serialize +
//!    Deserialize`. The kernel's [`AppError`] is `Serialize`-only by design
//!    (ArchForge invariant #4 — otherwise a peer could fabricate `Internal`
//!    arms). `WireError` is the wire-safe cousin: lossy in one direction
//!    (`AppError -> WireError`), never the reverse inside Rust.
//! 3. [`PanicReporter`] — a globally installable trait so that captured
//!    panics flow into your tracing / Sentry / log pipeline before being
//!    redacted for the caller.
//!
//! ## Why a dedicated crate?
//!
//! Earlier ArchForge versions parked panic safety inside the transport
//! pattern library (`pattern_t_panic_safety`). That was fine for tutorial
//! purposes but bound the guard to `TransportError`, leaking transport
//! semantics into auth, billing, sync, etc. — every bounded context would
//! have re-implemented the same wheel against its own narrow error.
//!
//! `archforge-ffi` lifts the guard up to the *kernel error*, so a single
//! `guard_async(...)` works for every use case in every slice.
//!
//! ## Layering
//!
//! ```text
//!   bridge-* (FRB / cbindgen)
//!       ↓ depends on
//!   archforge-ffi   ← you are here
//!       ↓ depends on
//!   archforge-kernel
//! ```
//!
//! `archforge-ffi` deliberately does **not** depend on any `contract-*` or
//! `domain-*` crate. It is the narrowest possible adapter between "anything
//! that can return `AppError`" and "anything that crosses an ABI."
//!
//! ## Example
//!
//! ```
//! use archforge_ffi::{guard_sync, WireError};
//! use archforge_kernel::AppError;
//!
//! // A normal use-case error flows through unchanged.
//! let business: Result<i32, AppError> = guard_sync(|| Err(AppError::NotFound("u/1".into())));
//! assert!(matches!(business, Err(AppError::NotFound(_))));
//!
//! // A panic becomes a domain-typed Internal — never a crash.
//! let panicked: Result<i32, AppError> = guard_sync(|| panic!("boom"));
//! assert!(matches!(panicked, Err(AppError::Internal(_))));
//!
//! // Convert either result into a WireError before serialising to the host.
//! let wire_err = WireError::from_result(panicked).unwrap_err();
//! assert!(wire_err.is_panic);
//! ```

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

mod guard;
mod reporter;
mod wire;

pub use guard::{guard_async, guard_sync, PANIC_INTERNAL_TAG};
pub use reporter::{install_panic_reporter, NoopReporter, PanicEvent, PanicReporter};
pub use wire::{WireError, WireErrorKind};
