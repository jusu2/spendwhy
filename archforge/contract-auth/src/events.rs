//! Domain events emitted by the auth aggregate.
//!
//! Each event variant has a stable, versioned discriminator (`v` tag) that is
//! part of the wire contract. Renames or schema breaks introduce a new
//! variant rather than mutating an existing one — that is the only way to
//! keep old consumers working.

use crate::types::{DisplayName, Email, UserId};
use archforge_kernel::{DomainEvent, Timestamp};
use serde::{Deserialize, Serialize};

/// All domain events emitted by the auth aggregate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "v")]
pub enum UserEvent {
    /// `auth.user.created.v1`.
    #[serde(rename = "auth.user.created.v1")]
    Created(UserCreated),
    /// `auth.user.renamed.v1`.
    #[serde(rename = "auth.user.renamed.v1")]
    Renamed(UserRenamed),
    /// `auth.user.password_set.v1`.
    #[serde(rename = "auth.user.password_set.v1")]
    PasswordSet(UserPasswordSet),
    /// `auth.user.password_verified.v1`.
    ///
    /// Emitted on successful authentication. Failed attempts are NOT
    /// emitted as a domain event — they belong to a security-audit stream
    /// outside this aggregate.
    #[serde(rename = "auth.user.password_verified.v1")]
    PasswordVerified(UserPasswordVerified),
}

/// Payload of `auth.user.created.v1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserCreated {
    /// New user id.
    pub id: UserId,
    /// Email at creation time.
    pub email: Email,
    /// Display name at creation time.
    pub display_name: DisplayName,
    /// When it happened.
    pub at: Timestamp,
}

/// Payload of `auth.user.renamed.v1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserRenamed {
    /// Id of the user that was renamed.
    pub id: UserId,
    /// New display name (post-rename).
    pub display_name: DisplayName,
    /// When it happened.
    pub at: Timestamp,
}

/// Payload of `auth.user.password_set.v1`. Carries no hash — the audit log
/// only needs to know *that* a password was set, never the value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserPasswordSet {
    /// User whose password changed.
    pub id: UserId,
    /// When it happened.
    pub at: Timestamp,
}

/// Payload of `auth.user.password_verified.v1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserPasswordVerified {
    /// User who authenticated.
    pub id: UserId,
    /// When it happened.
    pub at: Timestamp,
}

impl DomainEvent for UserEvent {
    fn event_type(&self) -> &'static str {
        match self {
            Self::Created(_) => "auth.user.created.v1",
            Self::Renamed(_) => "auth.user.renamed.v1",
            Self::PasswordSet(_) => "auth.user.password_set.v1",
            Self::PasswordVerified(_) => "auth.user.password_verified.v1",
        }
    }

    fn aggregate_id(&self) -> String {
        match self {
            Self::Created(e) => e.id.to_string(),
            Self::Renamed(e) => e.id.to_string(),
            Self::PasswordSet(e) => e.id.to_string(),
            Self::PasswordVerified(e) => e.id.to_string(),
        }
    }

    fn occurred_at(&self) -> Timestamp {
        match self {
            Self::Created(e) => e.at,
            Self::Renamed(e) => e.at,
            Self::PasswordSet(e) => e.at,
            Self::PasswordVerified(e) => e.at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Email;

    #[test]
    fn created_event_round_trips() {
        let e = UserEvent::Created(UserCreated {
            id: UserId::new(),
            email: Email::new("a@b").unwrap(),
            display_name: DisplayName::new("A").unwrap(),
            at: Timestamp::from_ms(1),
        });
        let s = serde_json::to_string(&e).unwrap();
        assert!(s.contains("auth.user.created.v1"));
        let back: UserEvent = serde_json::from_str(&s).unwrap();
        assert_eq!(e, back);
    }

    #[test]
    fn event_type_string_is_stable() {
        let e = UserEvent::Renamed(UserRenamed {
            id: UserId::new(),
            display_name: DisplayName::new("B").unwrap(),
            at: Timestamp::from_ms(2),
        });
        assert_eq!(e.event_type(), "auth.user.renamed.v1");
    }

    #[test]
    fn password_set_event_round_trips() {
        let e = UserEvent::PasswordSet(UserPasswordSet {
            id: UserId::new(),
            at: Timestamp::from_ms(99),
        });
        assert_eq!(e.event_type(), "auth.user.password_set.v1");
        let s = serde_json::to_string(&e).unwrap();
        let back: UserEvent = serde_json::from_str(&s).unwrap();
        assert_eq!(e, back);
    }
}
