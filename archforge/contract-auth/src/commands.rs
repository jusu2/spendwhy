//! Commands and queries that flow from callers (Application layer or
//! presentation layer) into Ports.

use crate::types::{DisplayName, Email, UserId};
use serde::{Deserialize, Serialize};

/// Create a new user with the given email and display name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateUserCmd {
    /// Email (must be unique).
    pub email: Email,
    /// Initial human-facing name.
    pub display_name: DisplayName,
}

/// Rename an existing user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameUserCmd {
    /// Identifier of the user to rename.
    pub id: UserId,
    /// New display name.
    pub display_name: DisplayName,
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
