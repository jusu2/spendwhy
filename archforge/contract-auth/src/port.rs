//! Port traits: read and write capabilities are split so use cases can bound
//! on exactly the capability they need.
//!
//! All write operations on `UserWriter` use **optimistic concurrency control**
//! via [`crate::Version`]: callers pass the version they expect to be current
//! and the adapter rejects stale writes with [`AppError::Conflict`].

use crate::types::{Email, PasswordHash, UserDto, UserId, Version};
use archforge_kernel::{Context, Result};
use async_trait::async_trait;

/// Read capability over auth users.
#[async_trait]
pub trait UserReader: Send + Sync {
    /// Find a user by primary id.
    ///
    /// `Ok(None)` is **not** an error — the user simply does not exist.
    /// Implementations must NOT return `AppError::NotFound` for missing rows.
    async fn find_by_id(&self, ctx: &Context, id: &UserId) -> Result<Option<UserDto>>;

    /// Find a user by email.
    ///
    /// Same `Ok(None)` discipline as [`find_by_id`].
    async fn find_by_email(&self, ctx: &Context, email: &Email) -> Result<Option<UserDto>>;
}

/// Write capability over auth users with optimistic concurrency.
#[async_trait]
pub trait UserWriter: Send + Sync {
    /// Insert a new user. The DTO's `version` must equal [`Version::INITIAL`].
    ///
    /// - `Ok(())` on success.
    /// - `AppError::Conflict` if `id` or `email` already exist.
    /// - `AppError::Invalid` if `version != Version::INITIAL`.
    /// - `AppError::Unavailable` for transient backend errors.
    ///
    /// Adapters honour `ctx.idempotency_key` when present: a retry with the
    /// same key returns `Ok(())` instead of `Conflict` for the *same* DTO,
    /// and `Conflict` for a *different* DTO.
    async fn insert(&self, ctx: &Context, user: &UserDto) -> Result<()>;

    /// Update an existing user. The caller passes the version they read
    /// (`expected_version`); the adapter applies the write only if the
    /// stored row's version matches.
    ///
    /// On success, the stored row's version becomes `user.version` (which
    /// the caller is expected to have set to `expected_version.next()`).
    ///
    /// - `Ok(())` on success.
    /// - `AppError::NotFound` if `id` does not exist.
    /// - `AppError::Conflict` if the stored version differs from
    ///   `expected_version` (lost update prevention).
    /// - `AppError::Conflict` if the new email collides with another user.
    async fn update(&self, ctx: &Context, user: &UserDto, expected_version: Version) -> Result<()>;

    /// Delete a user by id, with version check.
    ///
    /// - `Ok(())` on success (idempotent: deleting a missing id returns Ok).
    /// - `AppError::Conflict` if the stored version differs from
    ///   `expected_version`.
    async fn delete(&self, ctx: &Context, id: &UserId, expected_version: Version) -> Result<()>;
}

/// Convenience supertrait for adapters supporting both halves.
pub trait UserRepository: UserReader + UserWriter {}
impl<T: UserReader + UserWriter> UserRepository for T {}

/// Password storage Port.
///
/// Split from `UserWriter` so adapters can implement it independently —
/// e.g. a future read-only LDAP adapter can implement [`UserReader`] alone
/// without claiming to store passwords.
#[async_trait]
pub trait CredentialStore: Send + Sync {
    /// Persist (or replace) the password hash for a user.
    ///
    /// Returns `AppError::NotFound` if the user does not exist. Uses CAS
    /// against `expected_version` like [`UserWriter::update`].
    async fn set_password(
        &self,
        ctx: &Context,
        id: &UserId,
        hash: &PasswordHash,
        expected_version: Version,
    ) -> Result<()>;
}
