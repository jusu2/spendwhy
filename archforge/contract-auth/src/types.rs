//! 值对象与 [`UserDto`] 传输形状。

use archforge_kernel::{arch_newtype, AppError, Result, Timestamp};
use serde::{Deserialize, Serialize};

arch_newtype! {
    /// 用户标识符。随机 v4 uuid, auth 上下文外部不透明。
    pub struct UserId(Uuid);
}

arch_newtype! {
    /// RFC-5321 简化版邮箱。校验器刻意收紧 —— "看起来像邮箱"而非"是
    /// 可投递的邮箱"。生产系统在信任地址前仍应调可达性服务再验一次。
    pub struct Email(String) where |s|
        s.len() >= 3
        && s.len() <= 254
        && s.contains('@')
        && !s.chars().any(char::is_whitespace);
}

arch_newtype! {
    /// 修剪后 1..=128 个 Unicode code point。空或纯空白的名字会被拒绝。
    pub struct DisplayName(String) where |s| {
        let trimmed = s.trim();
        !trimmed.is_empty() && trimmed.chars().count() <= 128
    };
}

/// 聚合版本, 用于乐观并发控制 (CAS)。
///
/// 每次成功写入都会单调递增。`update` 操作必须把读到的版本回传给 Port,
/// 这样 adapter 才能用 [`AppError::Conflict`] 拒绝过时写入。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Version(u64);

impl Version {
    /// 新建聚合的初始版本。
    pub const INITIAL: Self = Self(1);

    /// 从原始 `u64` 构造 (adapter 再水化时使用)。
    pub const fn from_u64(v: u64) -> Self {
        Self(v)
    }

    /// 内部值。
    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    /// 递增, 在 `u64::MAX` 处饱和。饱和是一种 fail-safe; 实际中每聚合
    /// 2^64 个版本不可能触达。
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::INITIAL
    }
}

impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

/// PHC 字符串格式的 Argon2id 密码 hash。
///
/// 封装 `argon2::PasswordHasher::hash_password` 的 verifier 输出。内部
/// 字符串包含全部参数 (salt、m、t、p), 因此 verifier 不需要任何旁路信息。
///
/// `Display` 会对 hash 做脱敏, 避免它意外出现在日志里。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PasswordHash(String);

impl PasswordHash {
    /// 包装一个预先算好的 PHC 字符串。前缀不像可识别的 PHC hash 时返回
    /// `Invalid`。
    pub fn from_phc(phc: impl Into<String>) -> Result<Self> {
        let s = phc.into();
        if !s.starts_with("$argon2") {
            return Err(AppError::Invalid(
                "PasswordHash: only argon2 PHC strings are accepted".into(),
            ));
        }
        if s.len() > 512 {
            return Err(AppError::Invalid(
                "PasswordHash: encoded length exceeds 512".into(),
            ));
        }
        Ok(Self(s))
    }

    /// 借用 PHC 字符串。
    pub fn as_phc(&self) -> &str {
        &self.0
    }
}

// `Display` 刻意脱敏。要查看真正的 PHC 字符串, 调用方应使用 `as_phc()` ——
// 这是可 grep、可 PR review 的。
impl core::fmt::Display for PasswordHash {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("<redacted-password-hash>")
    }
}

/// 用户的 auth 侧投影。**唯一允许跨 Port 边界的形状** ——
/// `domain-auth::User` 留在它自己的 crate 内。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserDto {
    /// 唯一标识符。
    pub id: UserId,
    /// 邮箱 (作为天然次键, 必须唯一)。
    pub email: Email,
    /// 面向人的展示名。
    pub display_name: DisplayName,
    /// Argon2id 密码 hash。在密码支持上线前注册的用户为 `None`
    /// (一次性迁移路径)。
    #[serde(default)]
    pub password_hash: Option<PasswordHash>,
    /// 用户首次创建时间。
    pub created_at: Timestamp,
    /// 用户最近一次变更时间。
    pub updated_at: Timestamp,
    /// 乐观并发用的聚合版本。
    #[serde(default)]
    pub version: Version,
    /// Schema 版本。Adapter 对当前布局必须输出 `1`。未来破坏性变更
    /// 引入新的 DTO 类型, 而不是新的变体值。
    #[serde(default = "default_schema_v1")]
    pub schema_version: u16,
}

const fn default_schema_v1() -> u16 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_validator_rejects_obvious_garbage() {
        assert!(Email::new("").is_err());
        assert!(Email::new("noatsign").is_err());
        assert!(Email::new("has space@x.y").is_err());
        assert!(Email::new("a@b").is_ok());
    }

    #[test]
    fn display_name_trims_and_limits() {
        assert!(DisplayName::new("   ").is_err());
        assert!(DisplayName::new("x").is_ok());
        let too_long: String = "x".repeat(129);
        assert!(DisplayName::new(too_long).is_err());
        let ok_long: String = "x".repeat(128);
        assert!(DisplayName::new(ok_long).is_ok());
    }

    #[test]
    fn version_monotonic() {
        let a = Version::INITIAL;
        let b = a.next();
        assert!(b > a);
        assert_eq!(b.as_u64(), 2);
    }

    #[test]
    fn password_hash_rejects_non_argon2() {
        assert!(PasswordHash::from_phc("$bcrypt$xyz").is_err());
        assert!(PasswordHash::from_phc("plain").is_err());
        assert!(PasswordHash::from_phc(format!(
            "$argon2id$v=19$m=19456,t=2,p=1${}${}",
            "AAAAAAAAAAAAAAAA", "BBBBBBBBBBBBBBBB"
        ))
        .is_ok());
    }

    #[test]
    fn password_hash_display_is_redacted() {
        let h = PasswordHash::from_phc(
            "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAA$BBBBBBBBBBBBBBBB",
        )
        .unwrap();
        assert_eq!(h.to_string(), "<redacted-password-hash>");
        assert!(h.as_phc().contains("argon2id"));
    }

    #[test]
    fn user_dto_round_trips() {
        let dto = UserDto {
            id: UserId::new(),
            email: Email::new("a@b").unwrap(),
            display_name: DisplayName::new("Alice").unwrap(),
            password_hash: None,
            created_at: Timestamp::from_ms(100),
            updated_at: Timestamp::from_ms(200),
            version: Version::INITIAL,
            schema_version: 1,
        };
        let json = serde_json::to_string(&dto).unwrap();
        let back: UserDto = serde_json::from_str(&json).unwrap();
        assert_eq!(dto, back);
    }

    #[test]
    fn user_dto_rejects_invalid_email_on_deserialize() {
        let bad = r#"{
            "id": "00000000-0000-0000-0000-000000000001",
            "email": "no-at-sign",
            "display_name": "x",
            "created_at": 0,
            "updated_at": 0,
            "version": 1,
            "schema_version": 1
        }"#;
        assert!(serde_json::from_str::<UserDto>(bad).is_err());
    }

    #[test]
    fn user_dto_back_compat_default_version() {
        // 旧 DTO 没有 `version`/`password_hash` 字段仍能用合理的默认值
        // 解析 —— 这正是 `#[serde(default)]` 的意义。
        let json = r#"{
            "id": "00000000-0000-0000-0000-000000000001",
            "email": "a@b",
            "display_name": "x",
            "created_at": 0,
            "updated_at": 0,
            "schema_version": 1
        }"#;
        let dto: UserDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.version, Version::INITIAL);
        assert!(dto.password_hash.is_none());
    }
}
