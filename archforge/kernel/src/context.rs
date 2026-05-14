//! Request-scoped context propagated through every Port call.
//!
//! `Context` carries identity, locale, deadline, and idempotency information
//! across layers without resorting to thread-local or task-local globals.
//! Adapters are expected to attach the `trace_id` to outbound calls (HTTP
//! headers, log spans, etc.).

use crate::{AppError, Result, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Distributed trace correlation id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TraceId(Uuid);

impl TraceId {
    /// Fresh, random v4 trace id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Build from an existing uuid (useful at process boundaries).
    pub const fn from_uuid(u: Uuid) -> Self {
        Self(u)
    }

    /// Inner uuid.
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
// Validating newtypes (replaces the prior `pub String` bag).
// ---------------------------------------------------------------------------
//
// These three previously had public `String` fields. That meant control
// characters, empty strings, and megabyte-long payloads could quietly enter
// the system through `Context`. We now validate at construction.

const ACTOR_ID_MAX: usize = 256;
const IDEMPOTENCY_KEY_MIN: usize = 8;
const IDEMPOTENCY_KEY_MAX: usize = 256;
const LOCALE_MAX: usize = 35; // BCP-47 grammar caps tags well below this.

fn is_printable_no_ws(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| !c.is_control() && !c.is_whitespace())
}

/// Opaque actor (user/service) identifier.
///
/// Trimmed, non-empty, ≤ 256 bytes, no control characters or whitespace.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ActorId(String);

impl ActorId {
    /// Construct after validation.
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

    /// Borrow the inner string.
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

/// Opaque idempotency key.
///
/// Adapters that support idempotent writes use this to de-duplicate retried
/// requests. We require at least 8 characters of non-whitespace, non-control
/// content — enough entropy that two unrelated callers won't collide by
/// accident.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// Construct after validation.
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

    /// Borrow the inner string.
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

/// BCP-47 locale tag (e.g. `"en-US"`, `"zh-CN"`).
///
/// We do a syntactic check (1..=35 ASCII alphanumeric / `-`), not a full
/// BCP-47 parse. Production code that needs strict semantics should layer
/// `unic-langid` on top of this.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Locale(String);

impl Locale {
    /// Construct after validation.
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

    /// `en-US`. Infallible because the tag is a constant.
    pub fn en_us() -> Self {
        Self("en-US".to_string())
    }

    /// `zh-CN`. Infallible because the tag is a constant.
    pub fn zh_cn() -> Self {
        Self("zh-CN".to_string())
    }

    /// Borrow the BCP-47 tag.
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

/// Request-scoped context propagated through every Port call.
#[derive(Debug, Clone)]
pub struct Context {
    /// Distributed trace correlation id.
    pub trace_id: TraceId,
    /// Acting identity, if known.
    pub actor: Option<ActorId>,
    /// Locale for error messages, formatting, etc.
    pub locale: Locale,
    /// Absolute deadline; `None` means no deadline.
    pub deadline: Option<Timestamp>,
    /// Optional idempotency key for retry-safe writes.
    pub idempotency_key: Option<IdempotencyKey>,
}

impl Context {
    /// Fresh context with a new `TraceId` and default locale.
    pub fn new() -> Self {
        Self {
            trace_id: TraceId::new(),
            actor: None,
            locale: Locale::default(),
            deadline: None,
            idempotency_key: None,
        }
    }

    /// Convenience constructor used by tests.
    pub fn test() -> Self {
        Self::new()
    }

    /// Attach a pre-validated actor.
    #[must_use]
    pub fn with_actor(mut self, actor: ActorId) -> Self {
        self.actor = Some(actor);
        self
    }

    /// Replace locale.
    #[must_use]
    pub fn with_locale(mut self, locale: Locale) -> Self {
        self.locale = locale;
        self
    }

    /// Set an absolute deadline.
    #[must_use]
    pub fn with_deadline(mut self, deadline: Timestamp) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Attach a pre-validated idempotency key.
    #[must_use]
    pub fn with_idempotency(mut self, key: IdempotencyKey) -> Self {
        self.idempotency_key = Some(key);
        self
    }

    /// `true` iff `deadline` has elapsed at `now`.
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
        assert!(Locale::new("en_US").is_err()); // underscore not allowed
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
