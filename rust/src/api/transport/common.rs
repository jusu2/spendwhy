//! 共享边界类型: `TransportError`, `ProgressDto`, `CancelHandle`。
//!
//! 这里只放跨模式复用的"信号类型"，不放任何业务 DTO。
//! 与手册 §11.2 错误契约保持一致: 扁平 enum，不暴露 `anyhow` 原文。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use flutter_rust_bridge::frb;

/// 跨边界的标准化错误。Dart 侧据 `code` 字段映射到 UI 文案 / 重试策略。
///
/// 用扁平 struct + `code` 字符串 (而非带数据的 sum-type enum), 避免触发 FRB
/// 要求 `freezed` 依赖。`code` 枚举值定义见 `TransportErrorCode` 常量。
///
/// 业务系统应**包装**而不是**透传** `anyhow::Error`: 任何 Rust 内部错误
/// 在抵达 FFI 边界前都应转成此结构。
#[derive(Debug, Clone)]
pub struct TransportError {
    /// 错误码: `invalid_argument` / `not_found` / `conflict` / `canceled` / `timeout` / `internal`。
    pub code: String,
    /// 人类可读消息; 不含 PII、不含堆栈。
    pub message: String,
    /// 仅 `timeout` 用; 其余为 0。
    pub elapsed_ms: u64,
}

/// 错误码常量。在 Dart 侧也应有同名常量以保持契约一致。
#[frb(ignore)]
pub struct TransportErrorCode {}

#[allow(non_upper_case_globals)]
impl TransportErrorCode {
    pub const InvalidArgument: &'static str = "invalid_argument";
    pub const NotFound: &'static str = "not_found";
    pub const Conflict: &'static str = "conflict";
    pub const Canceled: &'static str = "canceled";
    pub const Timeout: &'static str = "timeout";
    pub const Internal: &'static str = "internal";
}

impl TransportError {
    #[frb(ignore)]
    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Self {
            code: TransportErrorCode::InvalidArgument.into(),
            message: msg.into(),
            elapsed_ms: 0,
        }
    }

    #[frb(ignore)]
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            code: TransportErrorCode::NotFound.into(),
            message: msg.into(),
            elapsed_ms: 0,
        }
    }

    #[frb(ignore)]
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self {
            code: TransportErrorCode::Conflict.into(),
            message: msg.into(),
            elapsed_ms: 0,
        }
    }

    #[frb(ignore)]
    pub fn canceled() -> Self {
        Self {
            code: TransportErrorCode::Canceled.into(),
            message: "canceled".into(),
            elapsed_ms: 0,
        }
    }

    #[frb(ignore)]
    pub fn timeout(elapsed_ms: u64) -> Self {
        Self {
            code: TransportErrorCode::Timeout.into(),
            message: format!("timeout after {elapsed_ms}ms"),
            elapsed_ms,
        }
    }

    #[frb(ignore)]
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: TransportErrorCode::Internal.into(),
            message: msg.into(),
            elapsed_ms: 0,
        }
    }
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for TransportError {}

/// 任务进度。`fraction` ∈ [0.0, 1.0]; `stage` 是人类可读阶段名。
#[derive(Debug, Clone)]
pub struct ProgressDto {
    pub fraction: f64,
    pub stage: String,
    pub message: Option<String>,
}

/// 取消句柄。Dart 持有 `RustOpaque<CancelHandle>`, 调 `cancel()` 触发 Rust 端中断。
///
/// 实现上仅是 `Arc<AtomicBool>`: 业务 Rust 代码在循环里轮询 `is_cancelled()`，
/// 或在 `tokio::select!` 里等待 `wait_cancelled()` future。
pub struct CancelHandle {
    flag: Arc<AtomicBool>,
}

impl CancelHandle {
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Dart 侧调用。幂等。
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    #[frb(sync)]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    /// Rust 内部使用: 克隆一个可在异步任务里轮询的 token。
    #[frb(ignore)]
    pub fn token(&self) -> CancelToken {
        CancelToken { flag: self.flag.clone() }
    }
}

impl Default for CancelHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// 仅 Rust 内部使用的轻量克隆。
#[frb(ignore)]
#[derive(Clone)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}

#[frb(ignore)]
impl CancelToken {
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    /// 在循环里调用; 若已取消, 返回 `Err(TransportError::Canceled)`。
    pub fn check(&self) -> Result<(), TransportError> {
        if self.is_cancelled() {
            Err(TransportError::canceled())
        } else {
            Ok(())
        }
    }
}

/// 工厂: Dart 侧 `await newCancelHandle()` 拿到 `RustOpaque<CancelHandle>`。
pub fn new_cancel_handle() -> CancelHandle {
    CancelHandle::new()
}
