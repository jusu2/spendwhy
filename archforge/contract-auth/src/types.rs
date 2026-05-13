//! Value objects and the [`UserDto`] transport shape.

use archforge_kernel::{arch_newtype, AppError, Result, Timestamp};
use serde::{Deserialize, Serialize};

arch_newtype! {
    /// User identifier. Random v4 uuid, opaque outside the auth context.
    pub struct UserId(Uuid);
}

arch_newtype! {
    /// RFC-5321 simplified email. The validator is intentionally narrow —
    /// "looks like an email" rather than "is a deliverable inbox". Production
    /// systems should still call out to a deliverability provider before
    /// trusting an address.
    pub struct Email(String) where |s|
        s.len() >= 3
        && s.len() <= 254
        && s.contains('@')
        && !s.chars().any(char::is_whitespace);
}

arch_newtype! {
    /// 1..=128 trimmed Unicode code points. Empty or whitespace-only names
    /// are rejected.
    pub struct DisplayName(String) where |s| {
        let trimmed = s.trim();
        !trimmed.is_empty() && trimmed.chars().count() <= 128
    };
}

/// Aggregate version, used for optimistic concurrency control (CAS).
///
/// Every successful write bumps this monotonically. `update` operations
/// must echo the version they read to the Port so the adapter can reject
/// stale writes with [`AppError::Conflict`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Version(u64);

impl Version {
    /// Initial version assigned to a freshly-created aggregate.
    pub const INITIAL: Self = Self(1);

    /// Construct from a raw `u64` (used by adapters when re-hydrating).
    pub const fn from_u64(v: u64) -> Self {
        Self(v)
    }

    /// Inner value.
    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    /// Increment, saturating at `u64::MAX`. The saturation is a fail-safe;
    /// in practice 2^64 versions per aggregate is unreachable.
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

/// Argon2id password hash in PHC string format.
///
/// Wraps the verifier output of `argon2::PasswordHasher::hash_password`. The
/// inner string contains all parameters (salt, m, t, p) so a verifier never
/// needs side-channel knowledge.
///
/// `Display` redacts the hash so it cannot accidentally appear in logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PasswordHash(String);

impl PasswordHash {
    /// Wrap a pre-computed PHC string. Returns `Invalid` if the prefix does
    /// not look like a recognised PHC hash.
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

    /// Borrow the PHC string.
    pub fn as_phc(&self) -> &str {
        &self.0
    }
}

// `Display` is intentionally redacted. To inspect the actual PHC string,
// callers use `as_phc()` — which is grep-able and PR-reviewable.
impl core::fmt::Display for PasswordHash {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("<redacted-password-hash>")
    }
}

/// Auth-side projection of a user. **The single shape allowed to cross a
/// Port boundary** — `domain-auth::User` stays in its own crate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserDto {
    /// Unique identifier.
    pub id: UserId,
    /// Email (acts as a natural secondary key, must be unique).
    pub email: Email,
    /// Human-facing name.
    pub display_name: DisplayName,
    /// Argon2id password hash. `None` for users registered before
    /// password support landed (a one-shot migration path).
    #[serde(default)]
    pub password_hash: Option<PasswordHash>,
    /// When the user was first created.
    pub created_at: Timestamp,
    /// When the user was last mutated.
    pub updated_at: Timestamp,
    /// Aggregate version for optimistic concurrency.
    #[serde(default)]
    pub version: Version,
    /// Schema version. Adapters MUST emit `1` for this layout. Future
    /// breaking changes introduce a new DTO type, not a new variant.
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
        // Older DTOs without `version`/`password_hash` still parse with
        // sensible defaults — that is the point of `#[serde(default)]`.
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
