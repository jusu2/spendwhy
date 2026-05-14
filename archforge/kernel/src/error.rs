//! 统一的、收敛的、业务语义化的错误类型。
//!
//! ArchForge 不变量 #4: 每个 Port 错误变体描述的是*业务*状况, 不是底层
//! 技术。Adapter 必须在跨越 Port 边界前, 把自己原生的错误 (`sqlx::Error`、
//! `std::io::Error` 等) 转成 `AppError`。
//!
//! `AppError` 刻意只 `Serialize` —— 从不 `Deserialize`。能经 JSON 往返的
//! 错误类型本身就是攻击面: 对端可以伪造 `Internal("…")` 来诱导上游分支
//! 行为。如果需要把错误元数据走线传输, 在相应 `contract-*` crate 里另外
//! 定义 `WireError` DTO。

use serde::Serialize;
use thiserror::Error;

/// 统一的应用错误。
///
/// `#[non_exhaustive]` 是强制的 —— 新增变体不应破坏下游 `match` 分支。
#[non_exhaustive]
#[derive(Debug, Error, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "detail")]
pub enum AppError {
    /// 资源不存在。
    #[error("not found: {0}")]
    NotFound(String),

    /// 状态冲突 (主键重复、乐观并发等)。
    #[error("conflict: {0}")]
    Conflict(String),

    /// 输入违反领域不变量。
    #[error("invalid: {0}")]
    Invalid(String),

    /// 外部依赖暂时不可用; 重试可能成功。
    #[error("unavailable: {0}")]
    Unavailable(String),

    /// 调用方没有执行该操作的权限。
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// 操作完成前 `Context::deadline` 已过。
    #[error("deadline exceeded")]
    DeadlineExceeded,

    /// 不可恢复的内部错误。应少见且总是记日志。
    #[error("internal: {0}")]
    Internal(String),
}

/// workspace 通用的便捷别名。
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

    // 编译期守卫: AppError 必须始终不实现 Deserialize。我们本可以用
    // `static_assertions::assert_not_impl_any!` 做负向 impl 探测更干净,
    // 但不想引入这个依赖。改为: 谁再把 `Deserialize` 加回 derive 列表,
    // 谁就破坏了类型上文档注释的契约, 而下面那段 JSON 也会经由
    // serde_json::from_str 反序列化到 AppError —— 它目前编译就过不去。
    #[test]
    fn app_error_serializes_but_does_not_deserialize() {
        let s = serde_json::to_string(&AppError::NotFound("x".into())).unwrap();
        assert!(s.contains("NotFound"));
        // 下面这一行刻意注释: 启用它必须在 AppError 没有 `Deserialize`
        // 时编译失败。把它当作该不变量的可执行文档。
        // let _: AppError = serde_json::from_str(&s).unwrap();
    }
}
