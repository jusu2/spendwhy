//! Value objects and the [`UserDto`] transport shape.

use archforge_kernel::{arch_newtype, Timestamp};
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
    /// When the user was first created.
    pub created_at: Timestamp,
    /// When the user was last mutated.
    pub updated_at: Timestamp,
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
    fn user_dto_round_trips() {
        let dto = UserDto {
            id: UserId::new(),
            email: Email::new("a@b").unwrap(),
            display_name: DisplayName::new("Alice").unwrap(),
            created_at: Timestamp::from_ms(100),
            updated_at: Timestamp::from_ms(200),
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
            "schema_version": 1
        }"#;
        assert!(serde_json::from_str::<UserDto>(bad).is_err());
    }
}
