//! Port traits: read and write capabilities are split so use cases can bound
//! on exactly the capability they need.

use crate::types::{Email, UserDto, UserId};
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

/// Write capability over auth users.
#[async_trait]
pub trait UserWriter: Send + Sync {
    /// Insert a new user. Returns:
    ///
    /// - `Ok(())` on success.
    /// - `AppError::Conflict` if `id` or `email` already exist.
    /// - `AppError::Unavailable` for transient backend errors.
    async fn insert(&self, ctx: &Context, user: &UserDto) -> Result<()>;

    /// Update an existing user. Returns:
    ///
    /// - `Ok(())` on success.
    /// - `AppError::NotFound` if `id` does not exist.
    /// - `AppError::Conflict` if the new email collides with another user.
    async fn update(&self, ctx: &Context, user: &UserDto) -> Result<()>;
}

/// Convenience supertrait for adapters supporting both halves.
///
/// Use cases that need both capabilities should bound on `R: UserReader +
/// UserWriter`; this trait is here mainly so test harnesses can ask for a
/// single object instead of two generic params.
pub trait UserRepository: UserReader + UserWriter {}
impl<T: UserReader + UserWriter> UserRepository for T {}
