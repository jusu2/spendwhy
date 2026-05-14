//! Wire-safe 错误 DTO。
//!
//! ## 为什么单独搞一个类型?
//!
//! ArchForge 不变量 #4: [`AppError`] **只能** `Serialize`, **不能** `Deserialize`。
//! 道理直白: 如果一个对端 (Dart 代码、另一个进程、恶意的 wire 参与方) 能通过
//! 伪造 JSON 直接造一个 `AppError::Internal("foo")`, 那么上游凡是按 variant
//! 分支的代码都可能被攻击。所以内核错误类型设计上就没有 `Deserialize` impl。
//!
//! 但每条 FFI 桥还是得把 wire 上的错误**解析**回来给对端。Dart、Swift、
//! Kotlin、C# —— 它们都想要一个稳定的形状, 带一个能 `switch` 的 tag。
//! 所以我们引入 [`WireError`]: `AppError` 的表亲, 满足
//!
//! - `Serialize + Deserialize`,
//! - 有一个**冗余**的 `is_panic` 布尔, 让消费方不需要解析文案就能区分
//!   panic 来源的 `Internal` 和业务自己抛的 `Internal`,
//! - 带一个稳定的 `kind` 鉴别符 (小写 snake_case 字符串, 永远不直接用 Rust
//!   枚举名 —— 否则每个 wire 消费方都被绑死在 Rust 标识符的改名上)。
//!
//! 转换是**单向**的: `From<AppError> for WireError`。没有
//! `From<WireError> for AppError`, 也永远不该有 —— 这种不对称的全部意义就是
//! 让 `AppError` 的分支集对外不可转移。

use std::fmt;

use archforge_kernel::AppError;
use serde::{Deserialize, Serialize};

use crate::guard::PANIC_INTERNAL_TAG;

/// [`WireError`] 的稳定 wire 鉴别符。
///
/// 字符串值是公开契约的一部分: 跨语言消费方需要按它分支。在这里新增 variant
/// 是**向后兼容**的, 前提是现有消费方处理"未知 kind" —— 它们应该处理, 因为
/// `#[serde(other)]` 的 `Unknown` 分支正是为此存在的。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireErrorKind {
    /// 资源不存在 (`AppError::NotFound`)。
    NotFound,
    /// 状态冲突 (`AppError::Conflict`)。
    Conflict,
    /// 输入违反领域不变量 (`AppError::Invalid`)。
    Invalid,
    /// 依赖不可用; 重试可能成功 (`AppError::Unavailable`)。
    Unavailable,
    /// 调用方权限不足 (`AppError::Forbidden`)。
    Forbidden,
    /// 截止时间已过 (`AppError::DeadlineExceeded`)。
    DeadlineExceeded,
    /// 不可恢复的内部错误 (`AppError::Internal`)。
    Internal,
    /// wire 收到的 kind 本消费方不认识。当成终态错误处理, 但要落日志。
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

/// Wire-safe 错误 DTO。为什么单独造一个, 看模块文档。
///
/// 字段顺序是契约的一部分; 序列化形状故意保持**扁平**、JSON 友好。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireError {
    /// 稳定的 wire 鉴别符。
    pub kind: WireErrorKind,
    /// 人类可读的描述。到达 wire 消费方时, 仅内部用的上下文已经被裁过。
    pub message: String,
    /// 当且仅当对应的 `AppError::Internal` 来自捕获 panic 时为 `true`
    /// (消息以 [`PANIC_INTERNAL_TAG`] 开头)。消费方可以据此把上报路由到
    /// Sentry / Crashlytics, 而不需要解析字符串。
    #[serde(default)]
    pub is_panic: bool,
}

impl WireError {
    /// 直接构造。多数调用方应该用 [`From<AppError>`] 或 [`Self::from_result`]。
    pub fn new(kind: WireErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            is_panic: false,
        }
    }

    /// 把当前错误标成 panic 来源。
    pub fn with_panic_flag(mut self, is_panic: bool) -> Self {
        self.is_panic = is_panic;
        self
    }

    /// 把 `Result<T, AppError>` 转成 `Result<T, WireError>`, 供 FFI 入口出参
    /// 使用。等价于 `result.map_err(WireError::from)`, 但调用点更易读。
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
            // AppError 是 `#[non_exhaustive]`; 将来新增的 variant 默认退化到
            // Internal, 让调用方永远拿到一个 wire-safe 的载荷。新 variant
            // 一旦落到这里, cargo-public-api 闸门会把它显式标出来, 提醒补一
            // 个显式映射。
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
        // 形状稳定 —— 消费方依赖这些 key 名。
        assert!(json.contains(r#""kind":"conflict""#), "got {json}");
        assert!(json.contains(r#""message":"dup email""#), "got {json}");
        assert!(json.contains(r#""is_panic":false"#), "got {json}");

        let back: WireError = serde_json::from_str(&json).unwrap();
        assert_eq!(back, wire);
    }

    #[test]
    fn unknown_kind_decays_to_unknown() {
        // 向前兼容: 老消费方读到将来才有的 kind 不能崩。
        let json = r#"{"kind":"future_kind_we_havent_invented","message":"hi","is_panic":false}"#;
        let parsed: WireError = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.kind, WireErrorKind::Unknown);
        assert_eq!(parsed.message, "hi");
    }

    #[test]
    fn is_panic_defaults_to_false_if_absent() {
        // 早于该字段引入的 Rust 生产者发来的载荷, 也必须能解析。
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
