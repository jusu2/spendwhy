//! # Auth use cases
//!
//! Generic, technology-agnostic orchestration. Use cases bound on the
//! specific Port capability they need ([`UserReader`], [`UserWriter`]) — the
//! type system rejects miswiring against an adapter that cannot fulfil the
//! requirement.
//!
//! TODO (Step 3): publish emitted [`UserEvent`]s through an `OutboxSink`
//! once the outbox crate exists. For the MVP we discard events; tests cover
//! the domain-level event emission separately.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

use archforge_contract_auth::{
    CreateUserCmd, Email, RenameUserCmd, UserDto, UserId, UserReader, UserWriter,
};
use archforge_domain_auth::User;
use archforge_kernel::{AppError, Context, Result, Timestamp};

/// Create a new user, enforcing email uniqueness.
pub async fn create_user<R>(repo: &R, ctx: &Context, cmd: CreateUserCmd) -> Result<UserDto>
where
    R: UserReader + UserWriter + ?Sized,
{
    if repo.find_by_email(ctx, &cmd.email).await?.is_some() {
        return Err(AppError::Conflict(format!(
            "email already exists: {}",
            cmd.email
        )));
    }
    let now = Timestamp::now();
    let (user, _event) = User::create(cmd.email, cmd.display_name, now);
    let dto = user.to_dto();
    repo.insert(ctx, &dto).await?;
    // TODO(step-3): outbox.append(ctx, &_event).await?;
    Ok(dto)
}

/// Rename an existing user. Returns the post-rename DTO.
pub async fn rename_user<R>(repo: &R, ctx: &Context, cmd: RenameUserCmd) -> Result<UserDto>
where
    R: UserReader + UserWriter + ?Sized,
{
    let existing = repo
        .find_by_id(ctx, &cmd.id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {}", cmd.id)))?;
    let mut user = User::rehydrate(existing)?;
    let _event = user.rename(cmd.display_name, Timestamp::now())?;
    let dto = user.to_dto();
    repo.update(ctx, &dto).await?;
    // TODO(step-3): outbox.append(ctx, &_event).await?;
    Ok(dto)
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

#[cfg(test)]
mod tests {
    use super::*;
    use archforge_contract_auth::{DisplayName, Email};
    use archforge_infra_auth_memory::InMemoryUserRepo;

    fn cmd(email: &str, name: &str) -> CreateUserCmd {
        CreateUserCmd {
            email: Email::new(email).unwrap(),
            display_name: DisplayName::new(name).unwrap(),
        }
    }

    #[tokio::test]
    async fn create_then_find_round_trips() {
        let repo = InMemoryUserRepo::new();
        let ctx = Context::test();
        let dto = create_user(&repo, &ctx, cmd("a@b", "Alice")).await.unwrap();
        let again = find_user_by_id(&repo, &ctx, dto.id).await.unwrap();
        assert_eq!(again.as_ref(), Some(&dto));
    }

    #[tokio::test]
    async fn duplicate_email_is_conflict() {
        let repo = InMemoryUserRepo::new();
        let ctx = Context::test();
        create_user(&repo, &ctx, cmd("a@b", "Alice")).await.unwrap();
        let err = create_user(&repo, &ctx, cmd("a@b", "Bob"))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn rename_missing_is_not_found() {
        let repo = InMemoryUserRepo::new();
        let ctx = Context::test();
        let err = rename_user(
            &repo,
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
    async fn rename_updates_display_name() {
        let repo = InMemoryUserRepo::new();
        let ctx = Context::test();
        let dto = create_user(&repo, &ctx, cmd("a@b", "Alice")).await.unwrap();
        // Sleep 1 ms so Timestamp::now() definitely advances past created_at on
        // platforms with coarse clocks.
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        let renamed = rename_user(
            &repo,
            &ctx,
            RenameUserCmd {
                id: dto.id,
                display_name: DisplayName::new("Alicia").unwrap(),
            },
        )
        .await
        .unwrap();
        assert_eq!(renamed.display_name.as_str(), "Alicia");
        assert!(renamed.updated_at >= dto.updated_at);
    }
}
