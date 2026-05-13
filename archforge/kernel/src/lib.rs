//! # ArchForge Kernel
//!
//! The 6 immutable primitives at the root of the ArchForge dependency graph:
//!
//! 1. [`AppError`] — narrow, business-semantic, `#[non_exhaustive]`.
//! 2. [`Context`] — trace_id / actor / locale / deadline / idempotency.
//! 3. [`Timestamp`] — opaque ms-since-epoch (no `std::time` leak).
//! 4. [`arch_newtype!`] — macro to declare validated value-object types.
//! 5. Capability markers — [`ReadOnly`], [`Writable`], [`Transactional`],
//!    [`BulkLoadable`], [`Streamable`].
//! 6. [`DomainEvent`] + [`OutboxSink`] — event-sourcing primitives.
//!
//! This crate has **zero internal dependencies**. Every other ArchForge crate
//! depends on it. Its public API is frozen for 12 months after the 1.0 release;
//! see `archforge/ARCHITECTURE_INVARIANTS.md`.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

mod capability;
mod context;
mod error;
mod event;
#[macro_use]
mod newtype;
mod time;

pub use capability::{BulkLoadable, ReadOnly, Streamable, Transactional, Writable};
pub use context::{ActorId, Context, IdempotencyKey, Locale, TraceId};
pub use error::{AppError, Result};
pub use event::{DomainEvent, OutboxSink};
pub use time::Timestamp;
