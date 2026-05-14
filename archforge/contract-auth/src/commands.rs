//! 从调用方 (应用层或表现层) 流入 Port 的 Command 与 Query。

use crate::types::{DisplayName, Email, UserId};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};

/// 明文密码 wrapper。只在 command 形状内部存在, 以保证:
///
/// - `Debug` 会脱敏。
/// - drop 时通过 `secrecy` 触发 `zeroize::Zeroize`。
/// - 它不能被 `Serialize` 出去 (此 struct 不派生 Serialize)。
#[derive(Clone)]
pub struct PlainPassword(pub SecretString);

impl PlainPassword {
    /// 包装一段明文密码。
    pub fn new(s: impl Into<String>) -> Self {
        Self(SecretString::new(s.into().into()))
    }
}

impl core::fmt::Debug for PlainPassword {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("PlainPassword(<redacted>)")
    }
}

/// 用给定的邮箱和展示名创建新用户。
#[derive(Debug, Clone)]
pub struct CreateUserCmd {
    /// 邮箱 (必须唯一)。
    pub email: Email,
    /// 初始展示名。
    pub display_name: DisplayName,
    /// 初始密码 (迁移期内可选)。
    pub password: Option<PlainPassword>,
}

/// 重命名已有用户。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameUserCmd {
    /// 要重命名的用户标识符。
    pub id: UserId,
    /// 新的展示名。
    pub display_name: DisplayName,
}

/// 设置或轮换用户密码。
#[derive(Debug, Clone)]
pub struct SetPasswordCmd {
    /// 标识符。
    pub id: UserId,
    /// 新密码 (明文 —— 在 use case 内部做 hash)。
    pub password: PlainPassword,
}

/// 验证用户密码。
#[derive(Debug, Clone)]
pub struct VerifyPasswordCmd {
    /// 尝试认证的用户邮箱。
    pub email: Email,
    /// 提交的密码。
    pub password: PlainPassword,
}

/// 读侧查询。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "kind", content = "value")]
pub enum UserQuery {
    /// 按主键 id 查找用户。
    ById(UserId),
    /// 按邮箱查找用户。
    ByEmail(Email),
}
