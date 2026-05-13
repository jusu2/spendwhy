//! 跨边界错误模型。
//!
//! 设计原则：
//! - 用 struct + `kind` 字符串而非 enum，避免 Dart 端引入 freezed。
//!   Dart 端通过 `AppErrorKind` 常量进行 switch，仍然可以做穷举。
//! - 错误的可机读类别在 `kind`，可读详情在 `message`。
//! - `field_or_code` 用于细分：InvalidInput 时是字段名，DomainRule 时是规则代号。

#[derive(Debug, Clone)]
pub struct AppError {
    pub kind: String,
    pub field_or_code: String,
    pub message: String,
}

impl AppError {
    pub const KIND_INVALID_INPUT: &'static str = "invalid_input";
    pub const KIND_DOMAIN_RULE: &'static str = "domain_rule";
    pub const KIND_INTERNAL: &'static str = "internal";

    pub fn invalid_input(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: Self::KIND_INVALID_INPUT.into(),
            field_or_code: field.into(),
            message: message.into(),
        }
    }

    pub fn domain_rule(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: Self::KIND_DOMAIN_RULE.into(),
            field_or_code: code.into(),
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: Self::KIND_INTERNAL.into(),
            field_or_code: String::new(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.field_or_code.is_empty() {
            write!(f, "{}: {}", self.kind, self.message)
        } else {
            write!(f, "{}[{}]: {}", self.kind, self.field_or_code, self.message)
        }
    }
}

impl std::error::Error for AppError {}

pub type AppResult<T> = std::result::Result<T, AppError>;
