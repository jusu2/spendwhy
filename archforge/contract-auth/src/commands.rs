//! Commands and queries that flow from callers (Application layer or
//! presentation layer) into Ports.

use crate::types::{DisplayName, Email, UserId};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};

/// Plain-text password wrapper. Lives only inside command shapes so that:
///
/// - `Debug` redacts it.
/// - It is `zeroize::Zeroize`d on drop (via `secrecy`).
/// - It cannot be `Serialize`d back out (no derive on this struct).
#[derive(Clone)]
pub struct PlainPassword(pub SecretString);

impl PlainPassword {
    /// Wrap a plain-text password.
    pub fn new(s: impl Into<String>) -> Self {
        Self(SecretString::new(s.into().into()))
    }
}

impl core::fmt::Debug for PlainPassword {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("PlainPassword(<redacted>)")
    }
}

/// Create a new user with the given email and display name.
#[derive(Debug, Clone)]
pub struct CreateUserCmd {
    /// Email (must be unique).
    pub email: Email,
    /// Initial human-facing name.
    pub display_name: DisplayName,
    /// Initial password (optional during the migration window).
    pub password: Option<PlainPassword>,
}

/// Rename an existing user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameUserCmd {
    /// Identifier of the user to rename.
    pub id: UserId,
    /// New display name.
    pub display_name: DisplayName,
}

/// Set or rotate a user's password.
#[derive(Debug, Clone)]
pub struct SetPasswordCmd {
    /// Identifier.
    pub id: UserId,
    /// New password (plain text — hashed inside the use case).
    pub password: PlainPassword,
}

/// Verify a user's password.
#[derive(Debug, Clone)]
pub struct VerifyPasswordCmd {
    /// Email of the user attempting to authenticate.
    pub email: Email,
    /// Submitted password.
    pub password: PlainPassword,
}

/// Read-side query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "kind", content = "value")]
pub enum UserQuery {
    /// Find a user by primary id.
    ById(UserId),
    /// Find a user by email.
    ByEmail(Email),
}
