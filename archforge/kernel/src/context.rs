//! 沿每次 Port 调用传播的请求级上下文。
//!
//! `Context` 在各层间携带身份、locale、deadline 和幂等性信息, 不依赖
//! thread-local 或 task-local 的全局变量。Adapter 应把 `trace_id`
//! 附加到对外调用 (HTTP header、log span 等)。

use crate::{AppError, Result, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 分布式 trace 关联 id。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TraceId(Uuid);

impl TraceId {
    /// 全新的随机 v4 trace id。
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// 从已有 uuid 构造 (适用于进程边界)。
    pub const fn from_uuid(u: Uuid) -> Self {
        Self(u)
    }

    /// 内部 uuid。
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Display for TraceId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.0, f)
    }
}

// ---------------------------------------------------------------------------
// 带校验的 newtype (替换之前的 `pub String` 大包大揽)。
// ---------------------------------------------------------------------------
//
// 这三个原本都是公开 `String` 字段。这意味着控制字符、空串、兆字节量级的
// 字符串可以悄悄通过 `Context` 进入系统。现在改为在构造时校验。

const ACTOR_ID_MAX: usize = 256;
const IDEMPOTENCY_KEY_MIN: usize = 8;
const IDEMPOTENCY_KEY_MAX: usize = 256;
const LOCALE_MAX: usize = 35; // BCP-47 语法允许的 tag 长度远低于此。

fn is_printable_no_ws(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| !c.is_control() && !c.is_whitespace())
}

/// 不透明的 actor (用户/服务) 标识符。
///
/// 已修剪、非空、≤ 256 字节、不含控制字符或空白。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ActorId(String);

impl ActorId {
    /// 校验后构造。
    pub fn new<S: Into<String>>(value: S) -> Result<Self> {
        let v = value.into();
        if v.len() > ACTOR_ID_MAX {
            return Err(AppError::Invalid(format!(
                "ActorId: length {} exceeds {}",
                v.len(),
                ACTOR_ID_MAX
            )));
        }
        if !is_printable_no_ws(&v) {
            return Err(AppError::Invalid(
                "ActorId: must be non-empty, no whitespace, no control chars".into(),
            ));
        }
        Ok(Self(v))
    }

    /// 借用内部字符串。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ActorId {
    type Error = AppError;
    fn try_from(v: String) -> Result<Self> {
        Self::new(v)
    }
}

impl From<ActorId> for String {
    fn from(v: ActorId) -> String {
        v.0
    }
}

impl core::fmt::Display for ActorId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// 不透明的幂等键。
///
/// 支持幂等写入的 adapter 用它来去重重试请求。我们要求至少 8 个
/// 非空白、非控制字符的内容 —— 熵足够大, 两个不相关的调用方不会偶然
/// 碰撞。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// 校验后构造。
    pub fn new<S: Into<String>>(value: S) -> Result<Self> {
        let v = value.into();
        if v.len() < IDEMPOTENCY_KEY_MIN || v.len() > IDEMPOTENCY_KEY_MAX {
            return Err(AppError::Invalid(format!(
                "IdempotencyKey: length {} not in [{},{}]",
                v.len(),
                IDEMPOTENCY_KEY_MIN,
                IDEMPOTENCY_KEY_MAX
            )));
        }
        if !is_printable_no_ws(&v) {
            return Err(AppError::Invalid(
                "IdempotencyKey: no whitespace, no control chars".into(),
            ));
        }
        Ok(Self(v))
    }

    /// 借用内部字符串。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for IdempotencyKey {
    type Error = AppError;
    fn try_from(v: String) -> Result<Self> {
        Self::new(v)
    }
}

impl From<IdempotencyKey> for String {
    fn from(v: IdempotencyKey) -> String {
        v.0
    }
}

impl core::fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// BCP-47 locale 标签 (如 `"en-US"`、`"zh-CN"`)。
///
/// 我们只做语法检查 (1..=35 个 ASCII 字母数字 / `-`), 不做完整 BCP-47
/// 解析。需要严格语义的生产代码应在其上叠加 `unic-langid`。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Locale(String);

impl Locale {
    /// 校验后构造。
    pub fn new<S: Into<String>>(value: S) -> Result<Self> {
        let v = value.into();
        if v.is_empty() || v.len() > LOCALE_MAX {
            return Err(AppError::Invalid(format!(
                "Locale: length {} not in [1,{}]",
                v.len(),
                LOCALE_MAX
            )));
        }
        if !v.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(AppError::Invalid(
                "Locale: only ASCII alphanumerics and '-' allowed".into(),
            ));
        }
        Ok(Self(v))
    }

    /// `en-US`。tag 是常量, 不会失败。
    pub fn en_us() -> Self {
        Self("en-US".to_string())
    }

    /// `zh-CN`。tag 是常量, 不会失败。
    pub fn zh_cn() -> Self {
        Self("zh-CN".to_string())
    }

    /// 借用 BCP-47 tag。
    pub fn tag(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Locale {
    type Error = AppError;
    fn try_from(v: String) -> Result<Self> {
        Self::new(v)
    }
}

impl From<Locale> for String {
    fn from(v: Locale) -> String {
        v.0
    }
}

impl core::fmt::Display for Locale {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Default for Locale {
    fn default() -> Self {
        Self::en_us()
    }
}

/// 沿每次 Port 调用传播的请求级上下文。
#[derive(Debug, Clone)]
pub struct Context {
    /// 分布式 trace 关联 id。
    pub trace_id: TraceId,
    /// 已知时的执行身份。
    pub actor: Option<ActorId>,
    /// 错误消息、格式化等所用的 locale。
    pub locale: Locale,
    /// 绝对 deadline; `None` 表示无 deadline。
    pub deadline: Option<Timestamp>,
    /// 可选的幂等键, 用于重试安全的写入。
    pub idempotency_key: Option<IdempotencyKey>,
}

impl Context {
    /// 全新的 context, 带新 `TraceId` 和默认 locale。
    pub fn new() -> Self {
        Self {
            trace_id: TraceId::new(),
            actor: None,
            locale: Locale::default(),
            deadline: None,
            idempotency_key: None,
        }
    }

    /// 测试用便捷构造器。
    pub fn test() -> Self {
        Self::new()
    }

    /// 附加一个已校验的 actor。
    #[must_use]
    pub fn with_actor(mut self, actor: ActorId) -> Self {
        self.actor = Some(actor);
        self
    }

    /// 替换 locale。
    #[must_use]
    pub fn with_locale(mut self, locale: Locale) -> Self {
        self.locale = locale;
        self
    }

    /// 设置绝对 deadline。
    #[must_use]
    pub fn with_deadline(mut self, deadline: Timestamp) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// 附加一个已校验的幂等键。
    #[must_use]
    pub fn with_idempotency(mut self, key: IdempotencyKey) -> Self {
        self.idempotency_key = Some(key);
        self
    }

    /// 当且仅当 `deadline` 在 `now` 时已过期时为 `true`。
    pub fn is_expired_at(&self, now: Timestamp) -> bool {
        match self.deadline {
            Some(d) => now.as_ms() >= d.as_ms(),
            None => false,
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_chains() {
        let ctx = Context::new()
            .with_actor(ActorId::new("alice").unwrap())
            .with_locale(Locale::zh_cn())
            .with_idempotency(IdempotencyKey::new("abc-12345").unwrap());

        assert_eq!(ctx.actor.as_ref().unwrap().as_str(), "alice");
        assert_eq!(ctx.locale.tag(), "zh-CN");
        assert_eq!(ctx.idempotency_key.as_ref().unwrap().as_str(), "abc-12345");
    }

    #[test]
    fn deadline_expiry() {
        let ctx = Context::new().with_deadline(Timestamp::from_ms(1_000));
        assert!(!ctx.is_expired_at(Timestamp::from_ms(999)));
        assert!(ctx.is_expired_at(Timestamp::from_ms(1_000)));
        assert!(ctx.is_expired_at(Timestamp::from_ms(2_000)));
    }

    #[test]
    fn no_deadline_never_expires() {
        let ctx = Context::new();
        assert!(!ctx.is_expired_at(Timestamp::from_ms(i64::MAX)));
    }

    #[test]
    fn actor_id_rejects_garbage() {
        assert!(ActorId::new("").is_err());
        assert!(ActorId::new("has space").is_err());
        assert!(ActorId::new("\nctrl").is_err());
        assert!(ActorId::new("ok-actor-1").is_ok());
        assert!(ActorId::new("x".repeat(257)).is_err());
    }

    #[test]
    fn idempotency_key_rejects_short_or_garbled() {
        assert!(IdempotencyKey::new("short").is_err());
        assert!(IdempotencyKey::new("with space-here").is_err());
        assert!(IdempotencyKey::new("abcd-efgh").is_ok());
        assert!(IdempotencyKey::new("x".repeat(257)).is_err());
    }

    #[test]
    fn locale_validates_syntactically() {
        assert!(Locale::new("").is_err());
        assert!(Locale::new("en_US").is_err()); // 不允许下划线
        assert!(Locale::new("en-US").is_ok());
        assert!(Locale::new("zh-Hant-CN").is_ok());
        assert!(Locale::new("a".repeat(36)).is_err());
    }

    #[test]
    fn newtypes_round_trip_via_serde() {
        let a = ActorId::new("svc-001").unwrap();
        let s = serde_json::to_string(&a).unwrap();
        assert_eq!(s, "\"svc-001\"");
        let back: ActorId = serde_json::from_str(&s).unwrap();
        assert_eq!(a, back);

        let bad: Result<ActorId> =
            serde_json::from_str("\"\"").map_err(|e| AppError::Invalid(e.to_string()));
        assert!(bad.is_err());
    }
}
