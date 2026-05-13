//! # Auth use cases
//!
//! Generic, technology-agnostic orchestration. Use cases bound on the
//! specific Port capability they need ([`UserReader`], [`UserWriter`],
//! [`OutboxSink`], [`Clock`]) — the type system rejects miswiring against an
//! adapter that cannot fulfil the requirement.
//!
//! All emitted [`UserEvent`]s are appended to the supplied [`OutboxSink`]
//! before the use case returns. Time is supplied through [`Clock`] so tests
//! can use [`FixedClock`] for determinism.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

mod password;

pub use password::PasswordHasher;

use archforge_contract_auth::{
    CreateUserCmd, Email, RenameUserCmd, SetPasswordCmd, UserDto, UserId, UserReader,
    UserWriter, VerifyPasswordCmd, Version,
};
use archforge_domain_auth::User;
use archforge_kernel::{AppError, BulkLoadable, Clock, Context, OutboxSink, Result};
use subtle::ConstantTimeEq;

/// Outcome of a successful authentication: the verified user plus the
/// version the use case observed (useful for chaining a follow-up write
/// without an extra round-trip).
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// The user DTO at the moment of verification.
    pub user: UserDto,
    /// Version observed at verification time.
    pub version: Version,
}

/// Create a new user, enforcing email uniqueness, optionally setting an
/// initial password.
pub async fn create_user<R>(
    repo: &R,
    outbox: &dyn OutboxSink,
    clock: &dyn Clock,
    hasher: &PasswordHasher,
    ctx: &Context,
    cmd: CreateUserCmd,
) -> Result<UserDto>
where
    R: UserReader + UserWriter + ?Sized,
{
    if repo.find_by_email(ctx, &cmd.email).await?.is_some() {
        return Err(AppError::Conflict(format!(
            "email already exists: {}",
            cmd.email
        )));
    }
    let now = clock.now();
    let hash = match cmd.password.as_ref() {
        Some(p) => Some(hasher.hash(p)?),
        None => None,
    };
    let (user, event) = User::create(cmd.email, cmd.display_name, hash, now);
    let dto = user.to_dto();
    repo.insert(ctx, &dto).await?;
    // Outbox is appended *after* the write succeeds. In a richer
    // implementation this would be inside the same transaction (UoW).
    outbox.append(ctx, &event).await?;
    Ok(dto)
}

/// Rename an existing user with optimistic concurrency.
pub async fn rename_user<R>(
    repo: &R,
    outbox: &dyn OutboxSink,
    clock: &dyn Clock,
    ctx: &Context,
    cmd: RenameUserCmd,
) -> Result<UserDto>
where
    R: UserReader + UserWriter + ?Sized,
{
    let existing = repo
        .find_by_id(ctx, &cmd.id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {}", cmd.id)))?;
    let expected_version = existing.version;
    let mut user = User::rehydrate(existing)?;
    let event = user.rename(cmd.display_name, clock.now())?;
    let dto = user.to_dto();
    repo.update(ctx, &dto, expected_version).await?;
    outbox.append(ctx, &event).await?;
    Ok(dto)
}

/// Rotate (or set) a user's password.
pub async fn set_password<R>(
    repo: &R,
    outbox: &dyn OutboxSink,
    clock: &dyn Clock,
    hasher: &PasswordHasher,
    ctx: &Context,
    cmd: SetPasswordCmd,
) -> Result<UserDto>
where
    R: UserReader + UserWriter + ?Sized,
{
    let existing = repo
        .find_by_id(ctx, &cmd.id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {}", cmd.id)))?;
    let expected_version = existing.version;
    let mut user = User::rehydrate(existing)?;
    let hash = hasher.hash(&cmd.password)?;
    let event = user.set_password(hash, clock.now())?;
    let dto = user.to_dto();
    repo.update(ctx, &dto, expected_version).await?;
    outbox.append(ctx, &event).await?;
    Ok(dto)
}

/// Verify a user's password.
///
/// **Constant-time** with respect to whether the user exists *or* whether
/// the password matches: both code paths run an argon2 verification (against
/// a dummy hash if the user is missing). Failure messages do not distinguish
/// between the two cases — both return the same `Forbidden`.
pub async fn verify_password<R>(
    repo: &R,
    outbox: &dyn OutboxSink,
    clock: &dyn Clock,
    hasher: &PasswordHasher,
    ctx: &Context,
    cmd: VerifyPasswordCmd,
) -> Result<AuthenticatedUser>
where
    R: UserReader + ?Sized,
{
    let user = repo.find_by_email(ctx, &cmd.email).await?;
    // Always do a hash verification, even if the user does not exist or has
    // no password, to avoid a timing oracle on user existence.
    let dummy = hasher.dummy_hash();
    let target = user
        .as_ref()
        .and_then(|u| u.password_hash.as_ref())
        .unwrap_or(&dummy);
    let ok = hasher.verify(&cmd.password, target);

    // ConstantTimeEq on the boolean outcome plus user-presence boolean —
    // collapses any micro-branching into one comparison.
    let user_present: u8 = if user.is_some() { 1 } else { 0 };
    let combined: u8 = (ok as u8) & user_present;
    if combined.ct_eq(&1u8).unwrap_u8() == 0 {
        return Err(AppError::Forbidden("invalid credentials".into()));
    }

    // Safe to unwrap: combined == 1 implies user.is_some().
    let dto = user.expect("user_present is 1");
    let version = dto.version;
    let domain = User::rehydrate(dto.clone())?;
    let event = domain.record_authenticated(clock.now());
    outbox.append(ctx, &event).await?;
    Ok(AuthenticatedUser { user: dto, version })
}

/// Lookup by id.
pub async fn find_user_by_id<R>(repo: &R, ctx: &Context, id: UserId) -> Result<Option<UserDto>>
where
    R: UserReader + ?Sized,
{
    repo.find_by_id(ctx, &id).await
}

/// Lookup by email.
pub async fn find_user_by_email<R>(repo: &R, ctx: &Context, email: Email) -> Result<Option<UserDto>>
where
    R: UserReader + ?Sized,
{
    repo.find_by_email(ctx, &email).await
}

/// Bulk-import users.
///
/// **Capability-bounded**: requires `R: UserWriter + BulkLoadable`. An
/// adapter that does not implement `BulkLoadable` cannot be wired here —
/// the type system rejects the miswiring. This is the live demonstration
/// of the Capability Marker invariant.
pub async fn import_users<R>(
    repo: &R,
    outbox: &dyn OutboxSink,
    clock: &dyn Clock,
    ctx: &Context,
    users: Vec<(Email, archforge_contract_auth::DisplayName)>,
) -> Result<usize>
where
    R: UserWriter + BulkLoadable + ?Sized,
{
    let mut imported = 0usize;
    for (email, display_name) in users {
        let now = clock.now();
        let (user, event) = User::create(email, display_name, None, now);
        let dto = user.to_dto();
        repo.insert(ctx, &dto).await?;
        outbox.append(ctx, &event).await?;
        imported += 1;
    }
    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use archforge_contract_auth::{DisplayName, Email, PlainPassword};
    use archforge_infra_auth_memory::{InMemoryOutbox, InMemoryUserRepo};
    use archforge_kernel::{FixedClock, Timestamp};

    fn create_cmd(email: &str, name: &str, password: Option<&str>) -> CreateUserCmd {
        CreateUserCmd {
            email: Email::new(email).unwrap(),
            display_name: DisplayName::new(name).unwrap(),
            password: password.map(PlainPassword::new),
        }
    }

    fn fixtures() -> (
        InMemoryUserRepo,
        InMemoryOutbox,
        FixedClock,
        PasswordHasher,
        Context,
    ) {
        (
            InMemoryUserRepo::new(),
            InMemoryOutbox::new(),
            FixedClock::new(Timestamp::from_ms(1_000_000)),
            PasswordHasher::test_fast(),
            Context::test(),
        )
    }

    #[tokio::test]
    async fn create_then_find_round_trips_and_emits_event() {
        let (repo, outbox, clock, hasher, ctx) = fixtures();
        let dto = create_user(&repo, &outbox, &clock, &hasher, &ctx, create_cmd("a@b", "Alice", None))
            .await
            .unwrap();
        let again = find_user_by_id(&repo, &ctx, dto.id).await.unwrap();
        assert_eq!(again.as_ref(), Some(&dto));
        let events = outbox.snapshot();
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn duplicate_email_is_conflict() {
        let (repo, outbox, clock, hasher, ctx) = fixtures();
        create_user(&repo, &outbox, &clock, &hasher, &ctx, create_cmd("a@b", "Alice", None))
            .await
            .unwrap();
        let err = create_user(&repo, &outbox, &clock, &hasher, &ctx, create_cmd("a@b", "Bob", None))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
        // The conflicting attempt must NOT have appended an event.
        assert_eq!(outbox.snapshot().len(), 1);
    }

    #[tokio::test]
    async fn rename_missing_is_not_found() {
        let (repo, outbox, clock, _hasher, ctx) = fixtures();
        let err = rename_user(
            &repo,
            &outbox,
            &clock,
            &ctx,
            RenameUserCmd {
                id: UserId::new(),
                display_name: DisplayName::new("X").unwrap(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn rename_advances_version_via_clock() {
        let (repo, outbox, clock, hasher, ctx) = fixtures();
        let dto = create_user(&repo, &outbox, &clock, &hasher, &ctx, create_cmd("a@b", "Alice", None))
            .await
            .unwrap();
        clock.advance_ms(10);
        let renamed = rename_user(
            &repo,
            &outbox,
            &clock,
            &ctx,
            RenameUserCmd {
                id: dto.id,
                display_name: DisplayName::new("Alicia").unwrap(),
            },
        )
        .await
        .unwrap();
        assert_eq!(renamed.display_name.as_str(), "Alicia");
        assert!(renamed.version > dto.version);
        assert!(renamed.updated_at > dto.updated_at);
    }

    #[tokio::test]
    async fn password_round_trip_succeeds() {
        let (repo, outbox, clock, hasher, ctx) = fixtures();
        let dto = create_user(
            &repo,
            &outbox,
            &clock,
            &hasher,
            &ctx,
            create_cmd("a@b", "Alice", Some("hunter2-strong")),
        )
        .await
        .unwrap();

        let auth = verify_password(
            &repo,
            &outbox,
            &clock,
            &hasher,
            &ctx,
            VerifyPasswordCmd {
                email: dto.email.clone(),
                password: PlainPassword::new("hunter2-strong"),
            },
        )
        .await
        .unwrap();
        assert_eq!(auth.user.id, dto.id);
    }

    #[tokio::test]
    async fn wrong_password_is_forbidden() {
        let (repo, outbox, clock, hasher, ctx) = fixtures();
        let dto = create_user(
            &repo,
            &outbox,
            &clock,
            &hasher,
            &ctx,
            create_cmd("a@b", "Alice", Some("hunter2-strong")),
        )
        .await
        .unwrap();
        let err = verify_password(
            &repo,
            &outbox,
            &clock,
            &hasher,
            &ctx,
            VerifyPasswordCmd {
                email: dto.email,
                password: PlainPassword::new("wrong-password"),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn unknown_user_is_forbidden_not_not_found() {
        let (repo, outbox, clock, hasher, ctx) = fixtures();
        let err = verify_password(
            &repo,
            &outbox,
            &clock,
            &hasher,
            &ctx,
            VerifyPasswordCmd {
                email: Email::new("nobody@example").unwrap(),
                password: PlainPassword::new("anything-strong"),
            },
        )
        .await
        .unwrap_err();
        // We deliberately collapse "no such user" into Forbidden so the API
        // does not leak existence.
        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn import_users_requires_bulkloadable() {
        // InMemoryUserRepo implements BulkLoadable, so this compiles.
        let (repo, outbox, clock, _hasher, ctx) = fixtures();
        let n = import_users(
            &repo,
            &outbox,
            &clock,
            &ctx,
            vec![
                (
                    Email::new("a@b").unwrap(),
                    DisplayName::new("A").unwrap(),
                ),
                (
                    Email::new("c@d").unwrap(),
                    DisplayName::new("C").unwrap(),
                ),
            ],
        )
        .await
        .unwrap();
        assert_eq!(n, 2);
        assert_eq!(outbox.snapshot().len(), 2);
    }
}
