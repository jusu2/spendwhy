//! # ArchForge Kernel
//!
//! ArchForge 依赖图根部的 7 个不可变原语:
//!
//! 1. [`AppError`] — 收敛的、业务语义化的, `#[non_exhaustive]`,
//!    仅 `Serialize` (永不 `Deserialize`)。
//! 2. [`Context`] — trace_id / actor / locale / deadline / idempotency。
//! 3. [`Timestamp`] — 不透明的 ms-since-epoch (Port 间不泄漏 `std::time`)。
//! 4. [`Clock`] — wall time 的 Port; 提供 [`SystemClock`] /
//!    [`FixedClock`] 让 use case 在测试中保持确定性。
//! 5. [`arch_newtype!`] — 声明带校验的值对象类型的宏。
//! 6. Capability 标记 — [`ReadOnly`], [`Writable`], [`Transactional`],
//!    [`BulkLoadable`], [`Streamable`]。Use case bound 在它们之上; adapter
//!    实现它们。错配在编译期就失败。
//! 7. [`DomainEvent`] + [`OutboxSink`] — 事件溯源原语。
//!
//! 本 crate **零内部依赖**。其他所有 ArchForge crate 都依赖它。其公共 API
//! 在 1.0 发布后 12 个月内冻结; 见 `archforge/ARCHITECTURE_INVARIANTS.md`。

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

mod capability;
mod clock;
mod context;
mod error;
mod event;
#[macro_use]
mod newtype;
mod time;

pub use capability::{BulkLoadable, ReadOnly, Streamable, Transactional, Writable};
pub use clock::{Clock, FixedClock, SystemClock};
pub use context::{ActorId, Context, IdempotencyKey, Locale, TraceId};
pub use error::{AppError, Result};
pub use event::{DomainEvent, OutboxSink};
pub use time::Timestamp;
