//! Request-scoped context propagated through every Port call.
//!
//! `Context` carries identity, locale, deadline, and idempotency information
//! across layers without resorting to thread-local or task-local globals.
//! Adapters are expected to attach the `trace_id` to outbound calls (HTTP
//! headers, log spans, etc.).

use crate::Timestamp;
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

/// Opaque actor (user/service) identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActorId(pub String);

/// Opaque idempotency key. Adapters that support idempotent writes use this
/// to de-duplicate retried requests.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IdempotencyKey(pub String);

/// BCP-47 locale tag (e.g. `"en-US"`, `"zh-CN"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Locale {
    /// The raw BCP-47 tag.
    pub tag: String,
}

impl Locale {
    /// Build from a raw BCP-47 tag.
    pub fn new(tag: impl Into<String>) -> Self {
        Self { tag: tag.into() }
    }

    /// `en-US`.
    pub fn en_us() -> Self {
        Self::new("en-US")
    }

    /// `zh-CN`.
    pub fn zh_cn() -> Self {
        Self::new("zh-CN")
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

    /// Convenience constructor used by tests. Identical to [`Self::new`] today
    /// but provides a single grep target if test contexts need extra defaults
    /// in the future.
    pub fn test() -> Self {
        Self::new()
    }

    /// Attach an actor.
    #[must_use]
    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(ActorId(actor.into()));
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

    /// Attach an idempotency key.
    #[must_use]
    pub fn with_idempotency(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(IdempotencyKey(key.into()));
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
            .with_actor("alice")
            .with_locale(Locale::zh_cn())
            .with_idempotency("abc-123");

        assert_eq!(ctx.actor.as_ref().unwrap().0, "alice");
        assert_eq!(ctx.locale.tag, "zh-CN");
        assert_eq!(ctx.idempotency_key.as_ref().unwrap().0, "abc-123");
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
}
