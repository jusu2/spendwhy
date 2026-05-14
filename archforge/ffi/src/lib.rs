//! # ArchForge FFI 边界守门员
//!
//! 任何 FFI 边界上的硬约束是: **Rust 的 panic 绝对不能 unwind 进外部调用方** ——
//! 那是未定义行为。本 crate 提供每个公开入口点都需要的三件套:
//!
//! 1. [`guard_sync`] / [`guard_async`] — 包住任意闭包或 future, 捕获 panic,
//!    返回一个域内可识别的 [`AppError::Internal`]。
//! 2. [`WireError`] — **单独定义**的 DTO, 同时实现 `Serialize + Deserialize`。
//!    kernel 的 [`AppError`] 按设计是 Serialize-only (ArchForge 不变量 #4 ——
//!    否则对端可以伪造 `Internal` 分支)。`WireError` 是它的 wire-safe 表亲:
//!    单向有损映射 (`AppError -> WireError`), Rust 内部永远不做反向。
//! 3. [`PanicReporter`] — 一个全局可装的 trait, 让捕到的 panic 在被回给调用方
//!    之前先流过你的 tracing / Sentry / 日志管线。
//!
//! ## 为什么单独开一个 crate?
//!
//! ArchForge 早期版本把 panic 安全藏在 transport 模式库
//! (`pattern_t_panic_safety`) 里。作为教学样例 OK, 但守门员被锚定到
//! `TransportError`, 把 transport 语义渗到 auth、billing、sync……每个 bounded
//! context 都得各自再造一遍同样的轮子。
//!
//! `archforge-ffi` 把守门员上提到**内核错误**这一层, 一个 `guard_async(...)`
//! 对每个切片的每个 use case 都通用。
//!
//! ## 层级位置
//!
//! ```text
//!   bridge-* (FRB / cbindgen)
//!       ↓ 依赖
//!   archforge-ffi   ← 你在这儿
//!       ↓ 依赖
//!   archforge-kernel
//! ```
//!
//! `archforge-ffi` 故意**不**依赖任何 `contract-*` 或 `domain-*` crate。它是
//! "任何可能返回 `AppError`" 与 "任何跨 ABI 的入口" 之间最窄的一段适配。
//!
//! ## 示例
//!
//! ```
//! use archforge_ffi::{guard_sync, WireError};
//! use archforge_kernel::AppError;
//!
//! // 普通 use case 错误直通, 不会被改写。
//! let business: Result<i32, AppError> = guard_sync(|| Err(AppError::NotFound("u/1".into())));
//! assert!(matches!(business, Err(AppError::NotFound(_))));
//!
//! // panic 被转成域内 Internal —— 进程不会崩。
//! let panicked: Result<i32, AppError> = guard_sync(|| panic!("boom"));
//! assert!(matches!(panicked, Err(AppError::Internal(_))));
//!
//! // 任意结果都可以再转成 WireError 后序列化给宿主。
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
