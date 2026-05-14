//! # Auth use cases
//!
//! 通用的、技术中立的编排层。Use case 仅绑定其需要的具体
//! Port 能力（[`UserReader`]、[`UserWriter`]、
//! [`OutboxSink`]、[`Clock`]）—— 类型系统会拒绝
//! 与不能满足该需求的适配器的错误装配。
//!
//! 所有发出的 [`UserEvent`] 在 use case 返回前都会追加到给定的
//! [`OutboxSink`]。时间通过 [`Clock`] 提供，便于测试
//! 用 [`FixedClock`] 取得确定性。

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

mod password;

pub use password::PasswordHasher;

use archforge_contract_auth::{
    CreateUserCmd, Email, RenameUserCmd, SetPasswordCmd, UserDto, UserId, UserReader, UserWriter,
    VerifyPasswordCmd, Version,
};
use archforge_domain_auth::User;
use archforge_kernel::{AppError, BulkLoadable, Clock, Context, OutboxSink, Result};
use subtle::ConstantTimeEq;

/// 成功认证的结果：通过验证的用户加上 use case 观察到的
/// 版本（便于在不增加额外往返的情况下链入后续写操作）。
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// 验证时刻的用户 DTO。
    pub user: UserDto,
    /// 验证时刻观察到的版本。
    pub version: Version,
}

/// 新建用户，强制 email 唯一，可选设置初始密码。
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
    // Outbox 在写成功 *之后* 才追加。更完善的实现会
    // 将其放入同一事务中（UoW）。
    outbox.append(ctx, &event).await?;
    Ok(dto)
}

/// 用乐观并发重命名已存在的用户。
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

/// 轮换（或设置）用户密码。
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

/// 验证用户密码。
///
/// 对“用户是否存在”以及“密码是否匹配”均保持 **常数时间**：
/// 两条路径都会做一次 argon2 验证（用户不存在时
/// 对一个 dummy 哈希做验证）。失败消息不区分两种情况 ——
/// 均返回相同的 `Forbidden`。
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
    // 即使用户不存在或无密码，也总执行一次哈希验证，
    // 以避免“用户存在性”的计时旁路。
    let dummy = hasher.dummy_hash();
    let target = user
        .as_ref()
        .and_then(|u| u.password_hash.as_ref())
        .unwrap_or(&dummy);
    let ok = hasher.verify(&cmd.password, target);

    // 对“布尔结果”与“用户存在性”两个布尔值做 ConstantTimeEq ——
    // 将所有微小分支收敛为一次比较。
    let user_present: u8 = if user.is_some() { 1 } else { 0 };
    let combined: u8 = (ok as u8) & user_present;
    if combined.ct_eq(&1u8).unwrap_u8() == 0 {
        return Err(AppError::Forbidden("invalid credentials".into()));
    }

    // 此处 unwrap 安全：combined == 1 蕴含 user.is_some()。
    let dto = user.expect("user_present is 1");
    let version = dto.version;
    let domain = User::rehydrate(dto.clone())?;
    let event = domain.record_authenticated(clock.now());
    outbox.append(ctx, &event).await?;
    Ok(AuthenticatedUser { user: dto, version })
}

/// 按 id 查询。
pub async fn find_user_by_id<R>(repo: &R, ctx: &Context, id: UserId) -> Result<Option<UserDto>>
where
    R: UserReader + ?Sized,
{
    repo.find_by_id(ctx, &id).await
}

/// 按 email 查询。
pub async fn find_user_by_email<R>(repo: &R, ctx: &Context, email: Email) -> Result<Option<UserDto>>
where
    R: UserReader + ?Sized,
{
    repo.find_by_email(ctx, &email).await
}

/// 批量导入用户。
///
/// **能力受限（capability-bounded）**：要求 `R: UserWriter + BulkLoadable`。
/// 未实现 `BulkLoadable` 的适配器无法在此装配 ——
/// 类型系统会拒绝错误装配。此处是
/// Capability Marker 不变式的活样本。
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
        let dto = create_user(
            &repo,
            &outbox,
            &clock,
            &hasher,
            &ctx,
            create_cmd("a@b", "Alice", None),
        )
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
        create_user(
            &repo,
            &outbox,
            &clock,
            &hasher,
            &ctx,
            create_cmd("a@b", "Alice", None),
        )
        .await
        .unwrap();
        let err = create_user(
            &repo,
            &outbox,
            &clock,
            &hasher,
            &ctx,
            create_cmd("a@b", "Bob", None),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
        // 冲突的那次尝试必须没有追加事件。
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
        let dto = create_user(
            &repo,
            &outbox,
            &clock,
            &hasher,
            &ctx,
            create_cmd("a@b", "Alice", None),
        )
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
        // 我们刻意将“用户不存在”收敛为 Forbidden，
        // 避免 API 泄露用户存在性。
        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn import_users_requires_bulkloadable() {
        // InMemoryUserRepo 实现了 BulkLoadable，因此可编译。
        let (repo, outbox, clock, _hasher, ctx) = fixtures();
        let n = import_users(
            &repo,
            &outbox,
            &clock,
            &ctx,
            vec![
                (Email::new("a@b").unwrap(), DisplayName::new("A").unwrap()),
                (Email::new("c@d").unwrap(), DisplayName::new("C").unwrap()),
            ],
        )
        .await
        .unwrap();
        assert_eq!(n, 2);
        assert_eq!(outbox.snapshot().len(), 2);
    }
}
